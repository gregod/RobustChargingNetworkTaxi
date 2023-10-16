use std::cmp;
use std::iter::Sum;
use std::ops::Div;
use indexmap::IndexMap;
use shared::{Site, Segment, Vehicle, Period, MIN_PER_PERIOD, CustomMultiHashMap, MAX_PERIOD, charge_time_to_capacity_charge_time};
use crate::{CG_EPSILON, SiteArray};
use crate::fixed_size::brancher::{Brancher, BranchNode, BranchQueue, DUMMY_COST, ResultPattern, SinglePattern, SolveError};
use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use grb::{attr, c, Constr, Env, Model, param, Status};
use grb::prelude::Binary;
use ndarray::Array2;
use rand::prelude::{IteratorRandom, StdRng};
use rand::{Rng, SeedableRng};
use crate::fixed_size::cg_model::{CgModel, SegmentId, SiteIndex, VehicleIndex};
use crate::fixed_size::policy_feasibility::VehicleAction::Idle;
use crate::pattern_pool::PatternPool;


pub struct PolicyFeasibility {

}

#[derive(Copy, Clone)]
struct VehicleState {
    start_time : Period,
    end_time : Period,
    action : VehicleAction,
    soc : f64
}

#[derive(Copy, Clone)]
enum VehicleAction {
    Idle,
    Charging(SiteIndex), // @charging Station
    ServingTour(usize) // index in tour
}

impl PolicyFeasibility{





    pub fn has_feasibility_error<'a>(sites: &'a IndexMap<u8, Site>, _segments_: &'a IndexMap<u32, Segment<'a>>, vehicles: Vec<Vehicle<'a>>) -> Option<SolveError> {

        /* high level:
        for every vehicle generate graph. Try to find path trough network
        similar to normal but under "behaviour" policy.
        RCSPP difficult as dominance criteria is hard.

        -> Therefore "fuzzy sampling according to policy"

        start with 50\%, once <30% - RND() -> RND() pick one of 5 closest & reachable charging stations.
            charge up until X + RND(),  or next customer.
         */


        let outer_loops = 100;
        let fuzzer_seed = 12345;
        let fuzzer_inner_loops = 3;
        let choice_top_closest = 3;

        let mut rng =  StdRng::seed_from_u64(fuzzer_seed);


        let site_array : Vec<Site> = sites.iter().map(|(_i,site)| site.clone()).collect();
        let site_conf_factory = SiteConfFactory {
            num_sites: site_array.len()
        };
        let site_conf = site_conf_factory.full(4);

        let mut inf_results = Vec::default();
        for l in 0..outer_loops {
            let mut inf_counter = 0;
            let mut vehicle_patterns = Vec::default();

            for vehicle in &vehicles {

                // track charge plans that are feasible
                let mut patterns: Vec<SinglePattern> = Vec::default();

                // run fuzzer loops to generate feasible columns
                for i in 0..fuzzer_inner_loops {
                    if let Some(pattern) = Self::gen_fuzzy_charge_pattern(vehicle, &mut rng, &site_conf, choice_top_closest) {
                        patterns.push(pattern)
                    }
                }
                vehicle_patterns.push(patterns);
            }


            let mut env_integer = Env::new("/tmp/fuzzer_gurobi.log").unwrap();

            env_integer.set(grb::param::LogToConsole, 1).unwrap();
            env_integer.set(grb::param::Threads, 1).unwrap();
            env_integer.set(grb::param::Seed, 12345).unwrap();


            let inf = Self::solve_assignment_problem(&env_integer, site_array.clone(), &site_conf, &vehicles, vehicle_patterns);

            inf_results.push(inf);
        }


        println!("{:?}",inf_results);

        return None;

    }

    fn gen_fuzzy_charge_pattern(vehicle : &Vehicle, mut rng: &mut StdRng, site_sizes : &SiteConf, take_top_closest : usize) -> Option<SinglePattern> {


        let mut charge_pattern : SinglePattern = Vec::default();



        let mut soc = vehicle.battery.initial_charge;


        fn get_low_tresh(rng: &mut StdRng, is_endphase: bool) -> f64 {
            if is_endphase {
                rng.gen_range(0.45..0.55)
            } else {
                rng.gen_range(0.10..0.45)
            }
        }

        fn get_high_tresh(rng: &mut StdRng) -> f64 {
            rng.gen_range(0.75..0.95)
        }

        let first_period = vehicle.tour.first().unwrap().start_time;
        let last_period = vehicle.tour.last().unwrap().stop_time;
        let dur = last_period - first_period;

        // after 75% of the shift we are in "end_phase" setting where we charge earlier
        // to achieve 50% at end.
        let end_phase_threshold = first_period + (0.70 * dur as f64) as Period;

       // println!();
        for segment in &vehicle.tour {
         //   print!("{soc} -> ");


            let is_endphase = segment.start_time > end_phase_threshold;

            if segment.is_free {
                // we could charge
                let mut did_charge = false;

                if soc < get_low_tresh(&mut rng, is_endphase) {
                    // we are below our given threshold

                    let potential_charge_site = segment.reachable_sites.iter()
                        .filter(|rc| {

                            let capacity = rc.site.capacity.min(site_sizes[rc.site.index]);
                            if capacity <= 0 {
                                return  false;
                            }

                            // make sure to only include those that are reachable
                            let soc_at_arrival = vehicle.battery.get_new_soc_after_distance(soc, rc.distance_to);
                            if soc_at_arrival < vehicle.battery.min_charge {
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
                            if periods_available <= 10 / MIN_PER_PERIOD as u16 {
                                return false;
                            }

                            return true;
                        })
                        // chose one of the first N (they are sorted min dist)
                        .take(take_top_closest).choose(&mut rng);


                    // there exists a site that we can charge at.
                    if let Some(site) = potential_charge_site {
                        did_charge = true;
                        // now we need to determine on how long to charge.
                        // this is copied to match dag_builder.rs
                        // TODO: Refactor into single module + test


                        soc = vehicle.get_new_soc_after_distance(soc, site.distance_to);


                        let arrival_period = site.arrival_time;
                        let latest_departure_period = site.departure_time;
                        let max_periods_available: Period = cmp::min(10, latest_departure_period - arrival_period);


                        let charge_threshold = get_high_tresh(&mut rng);
                        let mut actual_departure_period = latest_departure_period;
                        for charge_period in arrival_period..=arrival_period + max_periods_available {
                            // charge for that period

                            charge_pattern.push((SegmentId::new(segment), SiteIndex::new(site.site), charge_period));
                            soc = vehicle.get_new_soc_after_charging(soc, MIN_PER_PERIOD);
                            // test if we have charged enough
                            if soc >= charge_threshold {
                                actual_departure_period = charge_period;
                                break
                            }
                        }
                        // charging is over, either by next customer request, or charge threshold.

                        // now must drive return trip

                        soc = vehicle.get_new_soc_after_distance(soc, site.distance_from);
                    }
                }

                if !did_charge {

                    // if we dont charge, must drive direct distance
                    soc = vehicle.get_new_soc_after_distance(soc, segment.distance);
                }
            } else {
                // we must serve customer
                soc = vehicle.get_new_soc_after_distance(soc, segment.distance);
            }

            if soc < vehicle.battery.min_charge {
                return  None;
            }

        }

        if soc < vehicle.battery.min_final_charge {
                   return  None;
        }



        return  Some(charge_pattern);
    }


    fn solve_assignment_problem(env_integer : &Env, sites : Vec<Site>, site_sizes : &[u8], vehicles : &[Vehicle], patterns : Vec<Vec<SinglePattern>>) -> usize {


            // integer problem is only solved as kind of heuristic on the root node, thus only give limited amount of time.


            // create an empty model which associated with `env`:
            let mut integer_master = Model::with_env("integer_env", env_integer).unwrap();

            // initialize hashmaps for model variables and patterns
            let mut vehicle_patterns = CustomMultiHashMap::default();
            let mut vehicle_convexity: IndexMap<&Vehicle, Constr> = IndexMap::default();



            let mut site_constraints = Vec::with_capacity(sites.len() * MAX_PERIOD);
            for site in &sites {
                for p in 0..MAX_PERIOD {
                    site_constraints.push(
                        integer_master.add_constr(&format!("maxCapacity[{},{},{}]", site.id, site.index, p),
                                                  c!( 0 <= f64::from( site_sizes[site.index].min(site.capacity)))/*site.capacity.into()*/).unwrap());
                }
            }
            let constr_max_capacity: Array2<Constr> = Array2::from_shape_vec((sites.len(), MAX_PERIOD), site_constraints).unwrap();





            let mut pattern_counter = 0;
            let mut all_dummy_vars = Vec::with_capacity(vehicles.len());


            for vehicle in vehicles {
                let dummy_var = integer_master.add_var(&format!("dummy_column[{}]", vehicle.id), Binary, DUMMY_COST, 0.0, 1.0, []).unwrap();
                let constr_convexity = integer_master.add_constr(&format!("convexity[{}]", vehicle.id), c!(1.0 * dummy_var == 1.0)).unwrap();
                all_dummy_vars.push(dummy_var);

                vehicle_convexity.insert(vehicle, constr_convexity.clone());

                // add the existing patterns from the pool
                // copy paste from code in column generation below
                {
                    let patterns = &patterns[vehicle.index];

                    for entry in patterns {
                        let mut coef_vec: Vec<(Constr, f64)> = Vec::with_capacity(entry.len()+1);


                        // make column use one unit of convexity constraint
                        coef_vec.push((constr_convexity,1.0));



                        // make column use one unit of capacity at every used site
                        for (_, site, period) in entry.iter() {
                            coef_vec.push((constr_max_capacity[[site.index(), charge_time_to_capacity_charge_time(period)]].clone(),1.0));
                        }



                        let var_use_pattern = integer_master.add_var(&format!("usePattern[{}]", pattern_counter), Binary, 0.0, 0.0, 1.0, coef_vec).unwrap();
                        vehicle_patterns.insert(vehicle, (var_use_pattern, entry.clone()));
                        pattern_counter += 1;
                    }
                }

            }



            // integer_master.add_constr("limitInfeasible", c!(Expr::sum(all_dummy_vars.iter()) <=  self.allowed_infeasible as f64)).unwrap();



            integer_master.update().unwrap();
            integer_master.optimize().unwrap();


            if integer_master.status().unwrap() != Status::Optimal {
                panic!("INFEASIBLE");
            }


            // do test if we are infeasible

            let has_dummy_values = integer_master.get_obj_attr_batch(attr::X, all_dummy_vars.clone()).unwrap().iter().filter(|&v| *v >= CG_EPSILON).count();
            if has_dummy_values > 0 {

                let infeasible_vehicles = integer_master.get_obj_attr_batch(attr::X, all_dummy_vars.clone()).unwrap().iter().zip(vehicles).filter_map(|(value,vehicle)| {
                    if *value > CG_EPSILON {
                        Some(VehicleIndex::new(vehicle))
                    } else {
                        None
                    }
                }).collect::<Vec<VehicleIndex>>();

                //println!("has {} dummy vars set", has_dummy_values);
                //return Err(SolveError::VehiclesInfeasible(infeasible_vehicles));
            }

        return  has_dummy_values;
    }


}