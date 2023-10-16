use std::ops::Index;
use petgraph::Graph;
use petgraph::prelude::EdgeRef;
use petgraph::stable_graph::NodeIndex;
use typed_arena::Arena;
use shared::{Period, Segment, Site, Vehicle};
use crate::CG_EPSILON;
use crate::dag_builder::{EdgeWeight, NodeWeight};
use crate::fixed_size::cg_model::{SegmentId, SiteIndex};
use crate::pattern_pool::Pattern;

#[derive(Clone, Debug)]
struct Label<'a> {
    current_node: NodeIndex,
    soc: f64,
    parent_label: Option<&'a Label<'a>>,
    collected_edge_duals: f64,
    // is none if there is no chance to create a forbidden
    // column anymore. Otherwise it indicates the last evaluated
    // index in the forbidden_columns array for that label
    forbidden_column_check_position : Vec<Option<usize>>
}






pub fn generate_patterns<'s, 'a>(vehicle : &'a Vehicle, dag : &'s Graph<NodeWeight<'a>, EdgeWeight, petgraph::Directed>, root : NodeIndex, destination : NodeIndex, fixed_dual_cost : f64, exit_early : bool, forbidden_columns : &[Pattern]) -> Result<Vec<(f64, f64, Vec<(SegmentId, SiteIndex, Period)>)>, &'static str>   {


    let arena : Arena<Label> = Arena::new();

    // apply resource constrained shortest path algorithm using labels
    let mut labels_at : Vec<Vec<&Label>> = vec![Vec::with_capacity(1024);dag.node_count()];

    // initialize first label with initial range and initial reduced costs of path
    let initial_label : &Label = arena.alloc(Label {
        current_node: root,
        soc: vehicle.battery_initial_soc(),
        parent_label: None,
        collected_edge_duals: 0.0,
        // initially_none of the columns are in forbidden state
        // and the check index is 0
        forbidden_column_check_position :  vec![Some(0); forbidden_columns.len()]
    });

    labels_at[root.index()].push(initial_label);

    let mut unprocessed_labels: Vec<&Label> = Vec::with_capacity(1024);
    unprocessed_labels.push(initial_label);


    'generateLabelsLoop: loop {

        // take unprocessed label from front of vec
        let process_opt = unprocessed_labels.pop();

        // generate all direct possible child labels that are feasible
        let potential_next_labels = match process_opt {
            Some(process) => generate_labels_from_parent(fixed_dual_cost, destination, &vehicle, process, &dag, forbidden_columns),
            None => break,
        };




        // process every possible label
        for new_label in potential_next_labels {

            // apply some dominance rules to invalidate label


            if labels_at[new_label.current_node.index()].iter().any(|existing_label| {
                (new_label.soc <= existing_label.soc) &&
                    (/* duals here are negative, thus smaller is worse for reduced costs */
                        new_label.collected_edge_duals <= existing_label.collected_edge_duals)
                 && /* check that we can only be dominated if we cant be a forbidden column */
                    new_label.forbidden_column_check_position.iter().all(|col| {
                        let could_never_lead_to_forbidden = col.is_none();
                        could_never_lead_to_forbidden
                    })
            }) {
                continue
            }


            // if the label is not dominated (no break/continue before) we add it to the pool
            let arena_label = arena.alloc(new_label);
            labels_at[arena_label.current_node.index()].push(arena_label);
            unprocessed_labels.push(arena_label);

            if exit_early && arena_label.current_node == destination {
                break 'generateLabelsLoop; // label is good -> exit warly
            }

        }


    }


    let mut atlast = &labels_at[destination.index()];



    if atlast.is_empty() {
        #[cfg(feature = "column_generation_debug")]
        println!("No feasible path in vehicle {}", vehicle.id);
        Err("no feasible path")
    } else {






        fn get_and_add_parent(current_label : &Label, nodes : &mut Vec<NodeIndex>)  {
            nodes.push(current_label.current_node);
            if let Some(ref parent) = current_label.parent_label {
                get_and_add_parent(parent,nodes);
            }
        }

        let labels_with_neg_reduced_cost = atlast.iter()
            // calculate the reduced costs for each column and filter for negatives
            .filter_map(| &label| {
                let rc =  0.0 /* cost of column (no range cost)*/ - label.collected_edge_duals - fixed_dual_cost;
                if rc < - CG_EPSILON {
                    // recursivly get the path of nodes taken by the last label
                    let mut nodes : Vec<NodeIndex> = Vec::new();
                    get_and_add_parent(label,&mut nodes);
                    nodes.reverse();

                    Some((label.collected_edge_duals, rc, nodes ))

                } else {
                    None
                }
            } );





        let mut results : Vec<(f64, f64, Vec<(SegmentId, SiteIndex,Period)>)>  = labels_with_neg_reduced_cost.map(|(collected_range, reduced_costs, nodes)| {
            (collected_range, reduced_costs, nodes.iter().filter_map(|node| {
                let nw : &NodeWeight = &dag[*node];
                if let Some(charge_period) = nw.charge_period {
                    Some((SegmentId::new(nw.segment.unwrap()), SiteIndex::new(nw.site.unwrap().site), charge_period))
                } else {
                    None
                }

            }).collect())
        }).collect();


        // sort first by charging cost, then by remaining range as tiebraker
        results.sort_unstable_by(|(xr,xc,_),(yr,yc,_)| xc.partial_cmp(&yc).unwrap().then(
            xr.partial_cmp(&yr).unwrap()
        ));


        // remove forbidden columns at the end!
        // this should never be a problem, as we made sure
        // that only columns that can never become forbidden can dominate others.
        if ! forbidden_columns.is_empty() {
            results.retain(|(collected, reduced, pattern)| {
                if forbidden_columns.contains(pattern) {
                    return  false;
                } else {
                    return true;
                }
            });

            if results.is_empty() {
                // need to double check that not only the generated column is left!
                #[cfg(feature = "column_generation_debug")]
                println!("No feasible path in vehicle {}", vehicle.id);
                return Err("no feasible path")
            }
        }


        // return results
        Ok(results)

    }



}

fn  generate_labels_from_parent <'a> (fixed_dual_costs : f64, destination: NodeIndex, vehicle: &'a Vehicle, parent_label: &'a Label<'a>,
                                      dag: &'a Graph<NodeWeight, EdgeWeight, petgraph::Directed, u32>, forbidden_columns : &'a [Pattern]) -> impl Iterator<Item=Label<'a>> {

    let labels = dag.edges_directed(parent_label.current_node, petgraph::Direction::Outgoing).into_iter().filter_map(move |edge| {



        let current : &EdgeWeight = edge.weight();




        let new_collected_edge_duals = parent_label.collected_edge_duals + edge.weight().edge_dual_term.get();

        /*
                       reduced costs of a column are,  (0.0 /* cost of column (no range cost)*/ - label.collected_edge_duals - fixed_dual_cost) which should be < 0
                       where label.collected_edge_duals are negative and fixed dual costs always positive,
                       thus: if -collected_duals > fixed_duals -> can never be negative -> can only get worse later -> never valid
                       */

        if - new_collected_edge_duals  > fixed_dual_costs {
            return None
        }

        // we can charge at most the maximum battery capacity,
        // as the charging is normally discrete and the range cost is fractional
        // this min formulation allows to partially use an charge block
        let new_soc = if current.charge_duration_minutes > 0 {
            debug_assert!(current.distance_m == 0);
            vehicle.get_new_soc_after_charging(parent_label.soc, current.charge_duration_minutes)
        } else {
            debug_assert!(current.charge_duration_minutes == 0);
            vehicle.get_new_soc_after_distance(parent_label.soc, current.distance_m)
        };


        if  // have min battery constraint
        new_soc <  vehicle.battery_min_soc()
            // have maximum battery constraint
            || (new_soc >  vehicle.battery_max_soc())

        {
            return None
            //println!("Failing max charge");
        } else if edge.target() == destination {
            if new_soc < vehicle.battery_min_final_soc() {
                return None
            }
        }


        let mut new_forbidden_column_check = parent_label.forbidden_column_check_position.clone();

        let target = &dag[destination];

        // test if we need to update the forbidden_column_status
        for (visits, can_visit) in forbidden_columns
            .iter().zip(new_forbidden_column_check.iter_mut())
            .filter(|(_,can_visit)| can_visit.is_some()) // we only need to evaluate those that can still be visited.
        {

            let last_check = can_visit.expect("Must be set");



            /*
            take the first forbidden element that we haven't checked yet

            two actions to do:
                if this label is in the segment:
                    see if we make a decision that is != the decision of the forbidden column
                        if so, mark as None since we can't create a forbidden column that way
                    if we make a decision that is == the decision of the forbidden column
                        increment the check counter by one

                if the label is not in the segment,
                    see if our current label time is larger than current forbidden head.
                    if so, mark as None as we have somehow driven around one.
             */

            let (segment,site,period) = visits[last_check];

            if target.time_period > period {
                // we somehow have ended up skipping a charge node
                *can_visit = None;
                continue
            }

            if  let Some(cur_segment) = target.segment
            {
                if let Some(cur_site) = target.site {
                    if let Some(cur_charge) = target.charge_period {
                        if cur_charge == period && SegmentId::new(cur_segment) == segment && SiteIndex::new(cur_site.site) == site {
                            // we have found a visit that is exactly as desired
                            // increment counter +1 (we need to continue checking
                            // with the next potential forbidden charge location)
                            *can_visit = Some(last_check + 1);
                            continue
                        } else {
                            // we are at a charge that is not at a current charge period
                            // => we cant recreate the column
                            *can_visit = None;
                            continue
                        }
                    }
                }
            }
        }




        // TODO: pull down dominance requirements to here to prevent allocating new label
        // Early examination: The read log is expensive but likely nessesary
        // Investigate: Since read and write phases do not overlap maybe cheaper data type possibe

        Some(Label {
            current_node: edge.target(),
            soc: new_soc,
            parent_label: Some(parent_label),
            collected_edge_duals: new_collected_edge_duals,
            forbidden_column_check_position : new_forbidden_column_check
        })

    });

    labels
}
