use std::cell::Cell;
use std::cmp;
use std::iter::Sum;
use std::ops::Div;
use indexmap::IndexMap;
use shared::{Site, Segment, Vehicle, Period, MIN_PER_PERIOD, CustomMultiHashMap, MAX_PERIOD, charge_time_to_capacity_charge_time, ReachableSite};
use crate::{CG_EPSILON, SiteArray};
use crate::fixed_size::brancher::{Brancher, BranchNode, BranchQueue, DUMMY_COST, ResultPattern, SinglePattern, SolveError};
use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use chrono::Weekday::Sat;
use grb::{attr, c, Constr, Env, Model, param, Status};
use grb::prelude::Binary;
use ndarray::Array2;
use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};
use rand::distributions::Standard;
use crate::fixed_size::cg_model::{CgModel, SegmentId, SiteIndex, VehicleIndex};

use rand_distr::{Normal, Distribution};

use crate::pattern_pool::PatternPool;


pub struct SimulationFeasibility2<'a> {

    vehicle_states : Vec<Cell<VehicleState>>,
    vehicle_tour_position : Vec<Cell<usize>>,
    station_free: Vec<Cell<u8>>,

    sites: &'a IndexMap<u8, Site>,
    segments: &'a IndexMap<u32, Segment<'a>>,
    vehicles: Vec<Vehicle<'a>>
}

#[derive(Clone,Copy)]
struct VehicleState {
    start_time : Period,
    end_time : Period,
    action : VehicleAction,
    soc_at_end: f64
}

#[derive(Clone,Copy,Debug)]
enum VehicleAction {
    IdleUntil(Period),
    Done,
    Infeasible,
    InboundSite(WaitingInfo),
    WaitingSite(WaitingInfo),
}

#[derive(Clone,Copy,Debug)]
struct WaitingInfo {
    at : SiteIndex,
    arrival_at : Period,
    max_until : Period,
    distance_from : u32
}



enum  StateUpdate {
    UseCapacity(SiteIndex,Period),
    BecameInfeasible
}

impl StateUpdate {
    fn apply(&self, to_state : &State) -> State {
        let mut current_state = to_state.clone();
        match self {
            StateUpdate::UseCapacity(idx,period) => {
                #[cfg(feature = "simulation_debug")]
                println!("Record capacity at {} in {}", idx.index(), period);
                current_state.free_capacity[idx.index()][*period as usize] -= 1;
            }
            StateUpdate::BecameInfeasible => {}
        }
        current_state
    }
}

#[derive(Clone,Debug)]
struct  State {
    free_capacity : Vec<Vec<u8>>
}

impl State {
    fn new(site_sizes :  Vec<u8> ) -> Self {
        Self {
            free_capacity : site_sizes.into_iter().map(|s| vec![s; MAX_PERIOD * 2]).collect()
        }
    }
}

struct SimulatedVehicle<'a> {
    vehicle : Vehicle<'a>,

    _soc : Cell<f64>,
    _active_action : Cell<VehicleAction>,


    _current_segment_index : Cell<usize>,

    _handled_segments : Cell<usize>,

    low_tresh : f64, low_tresh_late : f64, high_tresh : f64
}




impl<'a> SimulatedVehicle<'a> {

    pub fn new(vehicle : Vehicle<'a>, low_tresh : f64, low_tresh_late : f64, high_tresh : f64) -> Self {
        Self {
            _soc : Cell::new(vehicle.battery.initial_charge),
            vehicle,
            _active_action : Cell::new(VehicleAction::IdleUntil(0)),
            _handled_segments : Cell::new(0),
            _current_segment_index : Cell::new(0),
            low_tresh , low_tresh_late , high_tresh
        }
    }


    fn set_active_action(&self, new : VehicleAction)  {
        self._active_action.set(new);
    }

    fn soc(&self) -> f64 {
        self._soc.get()
    }

    fn is_late_in_shift(&self, p : Period) -> bool {

        let start = self.vehicle.tour.first().unwrap().start_time;
        let end = self.vehicle.tour.last().unwrap().stop_time;

        let mostly_done_after = (start as f64 + ((end-start) as f64) * 0.75) as Period;


        p > mostly_done_after


    }

    fn active_action(&self) -> VehicleAction {
        self._active_action.get()
    }


    fn get_current_segment(&self) -> Option<Segment> {
        self.vehicle.tour.get(self._current_segment_index.get()).map(|x| x.clone().clone())
    }

    fn mark_segment_done(&self) {
        self._current_segment_index.update(|x| x + 1);
    }



    fn drive(&self, distance_m : u32) {
        self._soc.set(
            self.vehicle.get_new_soc_after_distance(self.soc(), distance_m)
        );
    }



    fn make_decision_at_start_of_segment(&self, p : Period, segment : Segment, state : &State) -> Vec<StateUpdate> {

        self._handled_segments.update(|x| x + 1);

        let (updates,action,done) : (Vec<StateUpdate>, VehicleAction,bool) = {
            if segment.is_free {


                let tresh = if self.is_late_in_shift(p) { self.low_tresh_late } else { self.low_tresh };

                let potential_charge_sites = segment.reachable_sites.iter()
                    .filter(|rc| {
                        if rc.site.capacity <= 0 { // site does not exit
                            return false;
                        }


                        // make sure to only include those that are reachable
                        let soc_at_arrival = self.vehicle.battery.get_new_soc_after_distance(self.soc(), rc.distance_to);
                        if soc_at_arrival < self.vehicle.battery.min_charge {
                            return false;
                        }

                        // make sure that there is meaningful charging possible
                        let arrival_period = rc.arrival_time;
                        let departure_period = rc.departure_time;

                        // check is needed as underflow would otherwise happen!
                        if arrival_period >= departure_period {
                            return false;
                        }
                        let periods_available: Period = departure_period - arrival_period;
                        // if we do not have time to reach this it is infeasible
                        if periods_available /* <= but can't be neg */ == 0 {
                            return false;
                        }
                        // 10 minute minimum for charging
                        if periods_available < 2 {
                            return false;
                        }

                        // now the checks if we should generally charge
                        // those where false lead to an empty list of reachable
                        // sites -> a no charging decision!

                        // if we are below threshold,
                        if self.soc() < tresh {
                            // always consider
                            return  true
                        }


                        // otherwise -> if the charging station is very close by and has free slot right now -> go for it
                        if  rc.distance_to < 1500 && state.free_capacity[rc.site.index][p as usize] > 0 {
                            return  true
                        }


                        // default -> no charge allowed
                        return false;
                    }).take(3).collect::<Vec<&ReachableSite>>();

                let free_potential_site = potential_charge_sites.iter()
                    .filter(|s| state.free_capacity[s.site.index][p as usize] > 0 /* has capacity available right now */).next();


                // there exists a site that has capacity
                if let Some(site) = free_potential_site {
                    self.drive(site.distance_to);

                    #[cfg(feature = "simulation_debug")]
                    println!("Begin inbound to site where free now");

                    (vec![], VehicleAction::InboundSite(WaitingInfo {
                        at: SiteIndex::new(site.site),
                        max_until: site.departure_time,
                        arrival_at: site.arrival_time,
                        distance_from: site.distance_from
                    }), false)

                } else if let Some(site_with_waiting) = potential_charge_sites.iter().next() {
                    // we have a site close but without capacity where we can wait.
                    self.drive(site_with_waiting.distance_to);
                    #[cfg(feature = "simulation_debug")]
                    println!("Begin inbound to where wait anticipated");
                    (vec![], VehicleAction::InboundSite(WaitingInfo {
                        at: SiteIndex::new(site_with_waiting.site),
                        max_until: site_with_waiting.departure_time,
                        arrival_at: site_with_waiting.arrival_time,
                        distance_from: site_with_waiting.distance_from
                    }), false)
                } else {
                    #[cfg(feature = "simulation_debug")]
                    println!("No chance at charging");
                    self.drive(segment.distance);
                    // no chance at charging, just skip that charge for now
                    (vec![], VehicleAction::IdleUntil(segment.stop_time + 1), true)
                }
            } else {
                #[cfg(feature = "simulation_debug")]
                println!("skipping charging");
                // do not charge, just drive to next customer
                self.drive(segment.distance);

                (vec![], VehicleAction::IdleUntil(segment.stop_time + 1), true)
            }

        };


        self._active_action.set(
            action
        );

        // this marks that we have fully processed the segment
        // this is not the case if we have an inbound charging action
        if done {
            self.mark_segment_done();
        }


        updates

    }


    pub fn tick(&mut self, p : Period, state : &State) -> Vec<StateUpdate> {

        if ! matches!(self._active_action.get(), VehicleAction::Infeasible) && self.soc() <= self.vehicle.battery.min_charge {
            self._active_action.set(VehicleAction::Infeasible);
            return  return vec![];
        }

        let curr_active = self._active_action.get();


        let updates = match curr_active {
            VehicleAction::IdleUntil(period) => {
                    if period <= p {
                        #[cfg(feature = "simulation_debug")]
                        println!("Wakeup at {p}");
                        if let Some(segment) =  self.get_current_segment() {
                            if p == segment.start_time {
                                #[cfg(feature = "simulation_debug")]
                                println!("Segment {}", segment.id);
                                return self.make_decision_at_start_of_segment(p, segment, state)
                            } else {
                                #[cfg(feature = "simulation_debug")]
                                println!("TO early");
                            }
                        } else {
                            if  self._handled_segments.get() == self.vehicle.tour.len() {
                                self._active_action.set(VehicleAction::Done);
                            } else {
                                dbg!(period,p,self._handled_segments.get(),self.vehicle.tour.len());
                                dbg!(&self.vehicle.tour.iter().map(|s| s.start_time).collect::<Vec<Period>>());
                                panic!()
                            }
                        }
                    } else {
                        #[cfg(feature = "simulation_debug")]
                        println!("Waiting for {period} (cur={p})");
                    }

                // no state update
                vec![]
            }
            VehicleAction::Infeasible => { return  vec![] }
            VehicleAction::Done => { return  vec![] }

            VehicleAction::InboundSite(inbound_info) => {
                // wakeup and test if we can charge or must wait.
                // we can't reserve a spot -> only known at arrival which
                // option can be made
                if p >= inbound_info.arrival_at  {
                    // has arrived
                    if state.free_capacity[inbound_info.at.index()][p as usize] > 0 {

                        // new slot possible
                        #[cfg(feature = "simulation_debug")]
                        println!("started charging at arrival");

                        let arrival_period = p;
                        let latest_departure_period = inbound_info.max_until;
                        let max_periods_available: Period = cmp::min(10, latest_departure_period - arrival_period + 1);


                        let mut actual_departure_period = latest_departure_period;
                        let mut updates = Vec::default();
                        for charge_period in arrival_period..=arrival_period + max_periods_available {
                            // charge for that period
                            if state.free_capacity[inbound_info.at.index()][charge_period as usize] > 0 {
                                updates.push(StateUpdate::UseCapacity(inbound_info.at, charge_period));
                                self._soc.set(
                                    self.vehicle.get_new_soc_after_charging(self.soc(), MIN_PER_PERIOD)
                                );
                                // test if we have charged enough
                                if self.soc() > self.high_tresh {
                                    actual_departure_period = charge_period;
                                    break
                                }
                            }  else {
                                panic!("Denied {p} {arrival_period} {charge_period}");
                            }
                        }
                        // charging is over, either by next customer request, or charge threshold.
                        // now must drive return trip
                        self.drive(inbound_info.distance_from);

                        self._active_action.set(VehicleAction::IdleUntil(
                            if let Some(segment) = self.get_current_segment() {
                                segment.stop_time + 1
                            } else {
                                MAX_PERIOD  as Period
                            }
                        ));
                        self.mark_segment_done();
                        return  updates;

                    } else {

                        self._active_action.set(VehicleAction::WaitingSite(WaitingInfo {
                            at: inbound_info.at,
                            max_until: inbound_info.max_until,
                            arrival_at: inbound_info.arrival_at,
                            distance_from: inbound_info.distance_from
                        }))


                    }
                }

                return vec![];
            }
            VehicleAction::WaitingSite(waiting_info) => {



                if p >= waiting_info.max_until  {
                    #[cfg(feature = "simulation_debug")]
                    println!("aborted waiting");
                    // have waited without success
                    self.drive(waiting_info.distance_from);
                    self._active_action.set(VehicleAction::IdleUntil(
                        if let Some(segment) = self.get_current_segment() {
                            segment.stop_time + 1
                        } else {
                            MAX_PERIOD as Period
                        }
                    ));
                    self.mark_segment_done();

                } else if  p < waiting_info.max_until && state.free_capacity[waiting_info.at.index()][p as usize] > 0 {

                    // new slot possible
                    #[cfg(feature = "simulation_debug")]
                    println!("started charging after waiting");

                    let arrival_period = p;
                    let latest_departure_period = waiting_info.max_until;
                    let max_periods_available: Period = cmp::min(10, latest_departure_period - arrival_period + 1);


                    let mut actual_departure_period = latest_departure_period;
                    let mut updates = Vec::default();
                    for charge_period in arrival_period..=arrival_period + max_periods_available {
                        // charge for that period
                        if state.free_capacity[waiting_info.at.index()][charge_period as usize] > 0 {
                            updates.push(StateUpdate::UseCapacity(waiting_info.at, charge_period));
                            self._soc.set(
                                self.vehicle.get_new_soc_after_charging(self.soc(), MIN_PER_PERIOD)
                            );
                            // test if we have charged enough
                            if self.soc() > self.high_tresh {
                                actual_departure_period = charge_period;
                                break
                            }
                        }  else {
                            panic!("Denied {p} {arrival_period} {charge_period}");
                        }
                    }
                    // charging is over, either by next customer request, or charge threshold.
                    // now must drive return trip
                    self.drive(waiting_info.distance_from);

                    self._active_action.set(VehicleAction::IdleUntil(
                        if let Some(segment) = self.get_current_segment() {
                            segment.stop_time + 1
                        } else {
                            MAX_PERIOD  as Period
                        }
                    ));
                    self.mark_segment_done();
                    return  updates;
                }


                return  vec![]}
        };



        return  updates;
    }
}




pub struct SimulationFeasibility {

}


impl SimulationFeasibility {


    pub fn run<'a>(sites: &'a IndexMap<u8, Site>, segments: &'a IndexMap<u32, Segment<'a>>, vehicles: Vec<Vehicle<'a>>, rng : &mut StdRng) -> usize {


        let site_sizes  =  sites.iter().map(|(_i,site)| site.capacity).collect();
        let mut state = State::new(site_sizes);


        let low_dist =  Normal::new(0.30, 0.05).unwrap();
        let low_late_dist =  Normal::new(0.60, 0.05).unwrap();
        let high_dist =  Normal::new(0.90, 0.05).unwrap();

        let mut simulated_vehicles : Vec<SimulatedVehicle> = vehicles.into_iter().map(|v|

            SimulatedVehicle::new(v,
            rng.sample(low_dist),
            rng.sample(low_late_dist),
            rng.sample(high_dist))
        ).collect();





        for t in (0 as Period)..(MAX_PERIOD as Period)*2 {
            for vehicle in simulated_vehicles.iter_mut() {
                for update in vehicle.tick(t,&state) { state = update.apply(&state); }
            }
        }


        let infeasible : Vec<bool> = simulated_vehicles.iter().map(|v| {
            let unhand =     v.vehicle.tour.len() - v._handled_segments.get();
            if unhand > 0 && !matches!(v._active_action.get(), VehicleAction::Infeasible) {
                dbg!(unhand,v._active_action.get(),v.vehicle.index);
                assert!(matches!(v._active_action.get(), VehicleAction::Infeasible))
            }
            v.soc() <= v.vehicle.battery.min_final_charge || unhand > 0 || matches!(v._active_action.get(), VehicleAction::Infeasible)
        }).collect();



        infeasible.iter().filter(|x| **x).count()
    }



}