
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

use crate::{CG_EPSILON, SegmentId, SiteIndex};
use crate::branching_filter::{BranchingFilter, DataFloat};
use petgraph::prelude::EdgeRef;



pub struct EdgeWeight {
    pub(crate) distance_m: u32, // distance that is driven
    pub(crate) charge_duration_minutes : u8,
    start_of_charge : bool,
    pub(crate) edge_dual_term: Rc<Cell<f64>>,
}

#[derive(Debug,Clone)]
pub struct NodeWeight<'a> {
    title: String,
    pub(crate) segment: Option<&'a Segment<'a>>,
    pub(crate) site: Option<&'a ReachableSite<'a>>,
    pub(crate) charge_period: Option<Period>,
    pub(crate) time_period : Period
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

pub fn build_dag <'s, 'a : 's>(vehicle : &'a Vehicle, no_dual : Rc<Cell<f64>>, site_period_duals : Rc<Array2<Rc<Cell<f64>>>>, charge_filters : &[BranchingFilter], site_sizes : SiteConf) ->  (NodeIndex<u32>, NodeIndex<u32>, Graph<NodeWeight<'s>, EdgeWeight, petgraph::Directed>) {

    let mut dag = Graph::<NodeWeight, EdgeWeight, petgraph::Directed>::new();

    // add dummy origin and destination nodes
    let root = dag.add_node(NodeWeight {
        title: "Origin".to_string(),
        site: None,
        segment: None,
        charge_period: None,
        time_period : 0,
    });

    let destination = dag.add_node(NodeWeight {
        title: "Destination".to_string(),
        site: None,
        segment: None,
        charge_period: None,
        time_period : MAX_PERIOD as Period,
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
            time_period : segment.stop_time
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
                    time_period : segment.start_time
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
                        *filter_segment == SegmentId::new(segment)
                    }
                    BranchingFilter::ChargeSegmentSiteTime(_, filter_segment, _, _, _) => {
                        *filter_segment == SegmentId::new(segment)
                    }
                    BranchingFilter::OpenSite(_filter_site,_typ) => true, // not segment specific
                    BranchingFilter::OpenSiteGroupMax(_,_) => true, // not segment specific
                    BranchingFilter::OpenSiteGroupMin(_,_) => true, // not segment specific
                    BranchingFilter::MasterNumberOfCharges(_,_,_,_) => true, // master number of charges does not influence single patterns,
                    BranchingFilter::MasterMustUseColumn(_, _, _) => true // master must use pattern does not influence single patterns
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
                    BranchingFilter::MasterNumberOfCharges(_,_,_,_) => false, // master number of charges does not influence single patterns,
                    BranchingFilter::MasterMustUseColumn(_, _, _) => false // master must use pattern does not influence single patterns
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
                    BranchingFilter::MasterNumberOfCharges(_,_,_,_) => None, // master number of charges does not influence single patterns
                    BranchingFilter::MasterMustUseColumn(_, _, _) => None // master must use pattern does not influence single patterns
                }
            }).collect::<Vec<(SiteIndex,Period,bool)>>();


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
                            if (!*typ && *filter_site == SiteIndex::new(site.site) || (*typ && *filter_site != SiteIndex::new(site.site)))  {
                                return true;
                            }
                            false
                        }
                        BranchingFilter::ChargeSegmentSiteTime(_, _, filter_site, _, typ) => {
                            // if filter asks not to use and sites match or filter asks use and no match
                            // at any time
                            if (!*typ && *filter_site == SiteIndex::new(site.site)) || (*typ && *filter_site != SiteIndex::new(site.site))  {
                                return true;
                            }
                            false
                        },
                        BranchingFilter::OpenSite(filter_site,typ) => {

                            if *typ == false && SiteIndex::new(site.site) == *filter_site {
                                // exlude sites where i have a negative charging filter
                                return true;
                            }
                            false
                        } // cosed sites should be excluded from dag!
                        BranchingFilter::OpenSiteGroupMax( sites,val) => {
                            if *val == DataFloat::zero() {
                                // only if we open zero sites in this group we will kill it
                                for filter_site in sites.iter() {
                                    if SiteIndex::new(site.site) == *filter_site {
                                        return true;
                                    }
                                }
                                false
                            } else {
                                false
                            }
                        }
                        BranchingFilter::OpenSiteGroupMin(_,_) => false,  // a open or closed site is no charging filter
                        BranchingFilter::MasterNumberOfCharges(_,_,_,_) => false, // master number of charges does not influence single patterns
                        BranchingFilter::MasterMustUseColumn(_, _, _) => false, // master must use pattern does not influence single patterns
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
                        time_period : segment.stop_time
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

                    if time_filters.iter().any(|(s,p,typ)| !typ && *s == SiteIndex::new(site.site) && *p == charge_time ) {


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
                            time_period : charge_time
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

                        if *filter_site == SiteIndex::new(site.site) {
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
