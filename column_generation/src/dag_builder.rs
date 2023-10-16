
use typed_arena::Arena;




use std::io::Write;
use shared::{Segment, Site, Period, ReachableSite, Vehicle, MAX_PERIOD, MIN_PER_PERIOD, charge_time_to_capacity_charge_time};
use petgraph::graph::NodeIndex;
use std::cell::Cell;
use std::rc::Rc;
use core::{fmt, cmp};
use petgraph::Graph;
use std::fs::OpenOptions;
use petgraph::dot::Dot;
use ndarray::Array2;
use crate::fixed_size::site_conf::SiteConf;

use crate::CG_EPSILON;
use crate::branching_filter::BranchingFilter;
use petgraph::prelude::EdgeRef;


#[derive(Clone, Debug)]
struct Label<'a> {
    current_node: NodeIndex,
    soc: f64,
    parent_label: Option<&'a Label<'a>>,
    collected_edge_duals: f64
}

pub struct EdgeWeight {
    distance_m: u32, // distance that is driven
    charge_duration_minutes : u8,
    start_of_charge : bool,
    edge_dual_term: Rc<Cell<f64>>,
}

#[derive(Debug,Clone)]
pub struct NodeWeight<'a> {
    title: String,
    segment: Option<&'a Segment<'a>>,
    site: Option<&'a ReachableSite<'a>>,
    charge_period: Option<Period>,
}

impl<'a> NodeWeight<'a> {
    pub fn get_segment(&self) -> Option<&'a Segment<'a>> {
        self.segment
    }
    pub fn get_site(&self) -> Option<&'a ReachableSite<'a>> {
        self.site
    }
}


impl<'a> fmt::Display for EdgeWeight {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {


        if self.start_of_charge {
            fmt.write_str("⚡")?
        }
        fmt.write_str("€:")?;
        fmt.write_str(&self.edge_dual_term.get().to_string())?;
        fmt.write_str("m:")?;
        fmt.write_str(&self.distance_m.to_string())?;
        fmt.write_str("d:")?;
        fmt.write_str(&self.charge_duration_minutes.to_string())?;
        Ok(())
    }
}





impl<'a> fmt::Display for NodeWeight<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.title)?;
        Ok(())
    }
}


pub fn save_dag(name : &str, dag : &Graph<NodeWeight, EdgeWeight, petgraph::Directed>) {

    OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(format!("/tmp/{}.dot", name))
        .and_then(|mut f| f.write_all((Dot::with_config(&dag, &[/*petgraph::dot::Config::NodeIndexLabel,petgraph::dot::Config::EdgeIndexLabel*/]).to_string()).as_bytes()))
        .expect("Failed");

}




/**
    Build a DAG of the vehicles choices with reduced costs attached to the nodes
*/

pub fn build_dag <'s, 'a : 's>(vehicle : &'a Vehicle, no_dual : Rc<Cell<f64>>, site_period_duals : Rc<Array2<Rc<Cell<f64>>>>, charge_filters : &[BranchingFilter<'a>], site_sizes : SiteConf) ->  (NodeIndex<u32>, NodeIndex<u32>, Graph<NodeWeight<'s>, EdgeWeight, petgraph::Directed>) {

    let mut dag = Graph::<NodeWeight, EdgeWeight, petgraph::Directed>::new();

    // add dummy origin and destination nodes
    let root = dag.add_node(NodeWeight {
        title: "Origin".to_string(),
        site: None,
        segment: None,
        charge_period: None,
    });

    let destination = dag.add_node(NodeWeight {
        title: "Destination".to_string(),
        site: None,
        segment: None,
        charge_period: None,
    });


    let mut last_segment_end: Option<NodeIndex> = None;




    // iterate over tour of vehicle, adding nodes on the way.
    for segment in &vehicle.tour {

        // create state node for end of trip
        let trip_end = dag.add_node(NodeWeight {
            title: format!("{}_end", segment.id),
            site: None,
            segment: Some(segment),
            charge_period: None,
        });

        let trip_start;

        // check if we have an previous segment that we need to connect to
        match last_segment_end {
            None => {
                // if not, insert dummy start node referencing segment
                trip_start = dag.add_node(NodeWeight {
                    title: "Start".to_string(),
                    site: None,
                    segment: Some(segment),
                    charge_period: None,
                });

                // connect dummy start node to root node
                dag.add_edge(root, trip_start, EdgeWeight { edge_dual_term: no_dual.clone(), charge_duration_minutes : 0, distance_m: 0 , start_of_charge : false });
            }
            Some(node) => {
                trip_start = node;
            }
        }


        // if we do not have a customer in this segment we need to explore charging possibilities
        if segment.is_free {



            // remove filters that are for other segments
            let filters = charge_filters.iter().filter(|filter| {
                match filter {
                    BranchingFilter::ChargeSegmentSite(_, filter_segment, _, _) => {
                        filter_segment == segment
                    }
                    BranchingFilter::ChargeSegmentSiteTime(_, filter_segment, _, _, _) => {
                        filter_segment == segment
                    }
                    BranchingFilter::OpenSite(_filter_site,_typ) => true, // not segment specific
                    BranchingFilter::OpenSiteGroupMax(_,_) => true, // not segment specific
                    BranchingFilter::OpenSiteGroupMin(_,_) => true, // not segment specific
                }
            }).collect::<Vec<&BranchingFilter>>();

            let positive_charging_filters = filters.iter().filter(|filter| {
                match filter {
                    BranchingFilter::ChargeSegmentSite(_, _, _, typ) => {
                        *typ == true
                    }
                    BranchingFilter::ChargeSegmentSiteTime(_, _, _, _, typ) => {
                        *typ == true
                    },
                    BranchingFilter::OpenSite(_filter_site,_typ) => false, // a open or closed site is no charging filter
                    BranchingFilter::OpenSiteGroupMax(_,_) => false,  // a open or closed site is no charging filter
                    BranchingFilter::OpenSiteGroupMin(_,_) => false,  // a open or closed site is no charging filter
                }
            });





            let mut min_charge = MAX_PERIOD as Period;
            let mut max_charge = 0;


            let time_filters =  filters.iter().filter_map(|filter| {
                match filter {
                    BranchingFilter::ChargeSegmentSite(_, _, _, _) => None,
                    BranchingFilter::ChargeSegmentSiteTime(_, _, site, period, typ) => Some((*site, *period, *typ)),
                    BranchingFilter::OpenSite(_filter_site,_typ) => None, // cosed sites should be excluded from dag!
                    BranchingFilter::OpenSiteGroupMax(_,_) => None,  // a open or closed site is no charging filter
                    BranchingFilter::OpenSiteGroupMin(_,_) => None,  // a open or closed site is no charging filter
                }
            }).collect::<Vec<(&Site,Period,bool)>>();


            let mut time_filter_site = None;
            for (site,period,typ) in &time_filters {
                    if *typ {
                        let p = period;

                        if min_charge > *p {
                            min_charge = *p;
                        }
                        if max_charge < *p {
                            max_charge = *p;
                        }

                        time_filter_site = Some(site);
                    }
            }



            // Only add the arc that allows skipping the charging we we dont enforce a charge action here.
            if positive_charging_filters.count() == 0 {
                dag.add_edge(trip_start, trip_end, EdgeWeight { distance_m: segment.distance, charge_duration_minutes : 0, edge_dual_term: no_dual.clone(), start_of_charge : false });
            }



            // for every site that we can reach (from preprocessing)
            for site in segment.reachable_sites.iter().filter(|&site | {

                // if this site has no size we skip it
                if site_sizes[site.site.index] == 0 {
                    return false
                }

                // look at complete site filters
                // if we must charge somewhere else in this segment block it
                // if we have negative filter at site also block it
                if filters.iter().any(|filter| {
                    // return true if we want to exclude
                    match filter {
                        BranchingFilter::ChargeSegmentSite(_, _, filter_site, typ) => {
                            // if filter asks not to use and sites match or filter asks use and no match
                            if (!*typ && *filter_site == site.site) || (*typ && *filter_site != site.site)  {
                                return true;
                            }
                            false
                        }
                        BranchingFilter::ChargeSegmentSiteTime(_, _, filter_site, _, typ) => {
                            // if filter asks not to use and sites match or filter asks use and no match
                            // at any time
                            if (!*typ && *filter_site == site.site) || (*typ && *filter_site != site.site)  {
                                return true;
                            }
                            false
                        },
                        BranchingFilter::OpenSite(filter_site,typ) => {

                            if *typ == false && site.site == *filter_site {
                                // exlude sites where i have a negative charging filter
                                return true;
                            }
                            false
                        } // cosed sites should be excluded from dag!
                        BranchingFilter::OpenSiteGroupMax( sites,val) => {
                            if *val == 0.0 {
                                // only if we open zero sites in this group we will kill it
                                for filter_site in sites.iter() {
                                    if site.site == *filter_site {
                                        return true;
                                    }
                                }
                                false
                            } else {
                                false
                            }
                        }
                        BranchingFilter::OpenSiteGroupMin(_,_) => false,  // a open or closed site is no charging filter
                    }
                }) {
                    return false;
                }


                // otherwise ok
                true

            }) {






                // calculate arrival time at site based on distance required to drive there
                let arrival_period = site.arrival_time;
                let departure_period = site.departure_time;

                // check is needed as underflow would otherwise happen!
                if arrival_period >= departure_period {
                    continue;
                }

                let periods_availiable : Period = cmp::min(10,departure_period - arrival_period);


                // if we do not have time to reach this it is infeasible
                if periods_availiable /* <= but can't be neg */== 0 {
                    continue;
                }

                // 10 minute minimum for charging
                if periods_availiable <= 10 / MIN_PER_PERIOD as u16 {
                    continue;
                }

                // add an exit node for the site && connect via edge to trip end
                let site_exit_node = dag.add_node(
                    NodeWeight {
                        title: format!("site_{}[{}]_e", site.site.id, site.site.index),
                        site: Some(site),
                        segment: Some(segment),
                        charge_period: None,
                    }
                );

                dag.add_edge(site_exit_node, trip_end,
                             EdgeWeight {
                                 distance_m: site.distance_from,
                                 charge_duration_minutes : 0,
                                 edge_dual_term: no_dual.clone(),
                                 start_of_charge : false
                             },
                );


                let mut last_node = None;
                let mut last_site : Option<usize> = None;

                // for each possible charging period an charging node is created and connected to previous charging nodes
                // The graph thus contains any possible combinations of consecutive charging blocks within the time frame
                /*
                                +-------------stop after p 1------------+
                                |                                       |
                                |                                       |
                  +-------------+---start in p 2----+                   |
                  |             |                   v                   v
                +-------+     +-------------+     +-------------+     +-----------+     +-----+
                | Start | --> | site_14_a_1 | --> | site_14_a_2 | --> | site_14_e | --> | End |
                +-------+     +-------------+     +-------------+     +-----------+     +-----+
                  |                                                                       ^
                  +---------------------------[ Do not charge ]---------------------------+
                */
                for period in 0..=periods_availiable {
                    let charge_time = arrival_period + period;

                    // we must test here if we did not exclude this time point with a negative filter
                    // if we have a match, we must not add this time node; It is thus skipped.
                    // as we enforce continious charging a skip means breaking the chaining of nodes
                    // thus "last_node" is resetted and the next time point will be only directly connected
                    // to the root node again.

                    if time_filters.iter().any(|(s,p,typ)| !typ && *s == site.site && *p == charge_time ) {


                        // since we only allow consecutive charges a gap between two charges is infeasible
                        #[cfg(feature = "column_generation_debug")]
                            {
                                if charge_time < max_charge && charge_time > min_charge {
                                    panic!("{}", "Infeasible Filter Variable: No Charge enforcement between two required charges");
                                }
                            }


                        last_node = None;
                        continue;
                    }



                    let site_node = dag.add_node(
                        NodeWeight {
                            title: format!("site_{}_a_{}", site.site.id, charge_time),
                            site: Some(site),
                            segment: Some(segment),
                            charge_period: Some(charge_time),
                        }
                    );

                    // if there is no filter that did require we have a charge at an earlier time
                    // we must add a direct connection from the start to this charge node
                    // as the taxi could wait and then only charge later
                    // if we have a filter requesting an earlier charge this connection would not
                    // be possible
                    let mut add_edge_from_start = true;
                    let mut add_edge_to_exit = true;

                    if let Some(filter_site) = time_filter_site {

                        if *filter_site == site.site {
                            if charge_time > min_charge {
                                add_edge_from_start = false;
                            }

                            if charge_time < max_charge {
                                add_edge_to_exit = false;
                            }
                        }
                    }

                    match last_node {
                        None => {

                            if add_edge_from_start {
                                dag.add_edge(trip_start, site_node,
                                             EdgeWeight {
                                                 distance_m: site.distance_to,
                                                 charge_duration_minutes : 0,
                                                 edge_dual_term: no_dual.clone(),
                                                 start_of_charge: true
                                             },
                                );
                            }

                            if add_edge_to_exit {
                                dag.add_edge(site_node, site_exit_node,
                                             EdgeWeight {
                                                 distance_m: 0,
                                                 charge_duration_minutes : MIN_PER_PERIOD,
                                                 edge_dual_term: site_period_duals[[site.site.index, charge_time_to_capacity_charge_time(&charge_time)]].clone(),
                                                 start_of_charge: false
                                             });
                            }
                        }

                        Some(last) => {





                            if add_edge_from_start {
                                dag.add_edge(trip_start, site_node,
                                             EdgeWeight {
                                                 distance_m: site.distance_to,
                                                 charge_duration_minutes : 0,
                                                 edge_dual_term: no_dual.clone(),
                                                 start_of_charge: true
                                             },
                                );
                            }

                            if add_edge_to_exit {
                                dag.add_edge(site_node, site_exit_node, EdgeWeight {
                                    distance_m: 0,
                                    charge_duration_minutes : MIN_PER_PERIOD,
                                    edge_dual_term: site_period_duals[[site.site.index, charge_time_to_capacity_charge_time(&(arrival_period + period))]].clone(),
                                    start_of_charge: false
                                });
                            }

                            dag.add_edge(last, site_node, EdgeWeight {
                                distance_m: 0,
                                charge_duration_minutes : MIN_PER_PERIOD,
                                edge_dual_term: site_period_duals[[last_site.unwrap(),charge_time_to_capacity_charge_time(&(arrival_period + period - 1))]].clone(),
                                start_of_charge : false
                            });
                        }
                    }

                    last_node = Some(site_node);
                    last_site = Some(site.site.index);
                }
            }
        } else {
            // connect start_trip (last_trip) to end of trip (this trip);
            dag.add_edge(trip_start, trip_end, EdgeWeight { distance_m: segment.distance ,charge_duration_minutes : 0, edge_dual_term: no_dual.clone(), start_of_charge : false });
        }


        last_segment_end = Some(trip_end);
    }

    dag.add_edge(last_segment_end.unwrap(), destination, EdgeWeight {
        distance_m: 0,
        charge_duration_minutes : 0,
        edge_dual_term: no_dual,
        start_of_charge : false
    });

    // save_dag(&format!("{}_vehicle_{}", charge_filters.len(), vehicle.id), &dag);
    // save_dag(&format!("vehicle_{}",  vehicle.id), &dag);

    (root,destination,dag)
}

pub fn generate_patterns<'s, 'a>(vehicle : &'a Vehicle, dag : &'s Graph<NodeWeight<'a>, EdgeWeight, petgraph::Directed>, root : NodeIndex, destination : NodeIndex , fixed_dual_cost : f64, exit_early : bool) -> Result<Vec<(f64, f64, Vec<(&'a Segment<'a>, &'a Site,Period)>)>, &'static str>   {


    let arena : Arena<Label> = Arena::new();

    // apply resource constrained shortest path algorithm using labels
    let mut labels_at : Vec<Vec<&Label>> = vec![Vec::with_capacity(1024);dag.node_count()];

    // initialize first label with initial range and initial reduced costs of path
    let initial_label : &Label = arena.alloc(Label {
        current_node: root,
        soc: vehicle.battery_initial_soc(),
        parent_label: None,
        collected_edge_duals: 0.0,
    });

    labels_at[root.index()].push(initial_label);

    let mut unprocessed_labels: Vec<&Label> = Vec::with_capacity(1024);
    unprocessed_labels.push(initial_label);


    'generateLabelsLoop: loop {

        // take unprocessed label from front of vec
        let process_opt = unprocessed_labels.pop();

        // generate all direct possible child labels that are feasible
        let potential_next_labels = match process_opt {
            Some(process) => generate_labels_from_parent(fixed_dual_cost, destination, &vehicle, process, &dag),
            None => break,
        };




        // process every possible label
        for new_label in potential_next_labels {

            // apply some dominance rules to invalidate label


            if labels_at[new_label.current_node.index()].iter().any(|existing_label| {
                (new_label.soc <= existing_label.soc) &&
                    (/* duals here are negative, thus smaller is worse for reduced costs */ new_label.collected_edge_duals <= existing_label.collected_edge_duals)
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


    let atlast = &labels_at[destination.index()];

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
            };
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





            let mut results : Vec<(f64, f64, Vec<(&'a Segment<'a>, &'a Site,Period)>)>  = labels_with_neg_reduced_cost.map(|(collected_range, reduced_costs, nodes)| {
                (collected_range, reduced_costs, nodes.iter().filter_map(|node| {
                    let nw : &NodeWeight = &dag[*node];
                    if let Some(charge_period) = nw.charge_period {
                        Some((nw.segment.unwrap(), nw.site.unwrap().site,charge_period))
                    } else {
                        None
                    }

                }).collect())
            }).collect();


            // sort first by charging cost, then by remaining range as tiebraker
            results.sort_unstable_by(|(xr,xc,_),(yr,yc,_)| xc.partial_cmp(&yc).unwrap().then(
                xr.partial_cmp(&yr).unwrap()
            ));




            // return results
            Ok(results)

    }



}

fn  generate_labels_from_parent <'a> (fixed_dual_costs : f64, destination: NodeIndex, vehicle: &'a Vehicle, parent_label: &'a Label<'a>,
                                      dag: &'a Graph<NodeWeight, EdgeWeight, petgraph::Directed, u32>) -> impl Iterator<Item=Label<'a>> {

    let labels = dag.edges_directed(parent_label.current_node, petgraph::Direction::Outgoing).into_iter().filter_map(move |edge| {



        let current = edge.weight();

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




        // TODO: pull down dominance requirements to here to prevent allocating new label
        // Early examination: The read log is expensive but likely nessesary
        // Investigate: Since read and write phases do not overlap maybe cheaper data type possibe

        Some(Label {
            current_node: edge.target(),
            soc: new_soc,
            parent_label: Some(parent_label),
            collected_edge_duals: new_collected_edge_duals,
        })

    });

    labels
}
