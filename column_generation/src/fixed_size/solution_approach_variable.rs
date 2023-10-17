use std::sync::Arc;
use std::sync::atomic::AtomicBool;


use indexmap::IndexMap;
use itertools::Itertools;
use shared::{Segment, Simple, Site, Vehicle};
use std::iter::Sum;

use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use crate::fixed_size::brancher::{Brancher, SolveError};

use crate::{SiteArray, CG_EPSILON, SiteIndex};

use crate::fixed_size::brancher::ResultPattern;

use rust_hawktracer::*;

use rand::prelude::{StdRng, SliceRandom};
use rand::{SeedableRng, Rng};
use grb::prelude::*;
use grb::expr::LinExpr;
use std::path::PathBuf;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use std::collections::HashSet;
use std::time::{Instant, Duration};
use std::sync::atomic::Ordering::Relaxed;
use std::io;
use std::io::BufRead;
use std::cmp::max;

use grb::{attr, Var};
use petgraph::visit::Walker;
use crate::fixed_size::cg_model::VehicleIndex;
use crate::fixed_size::scenario_manager::ScenarioManager;
use crate::pattern_pool::{PatternEntry, PatternPool};

pub struct SolutionApproachVariable<'a> {
    rng : StdRng,
    min_num_sites: usize,
    site_array : Vec<Site>,
    site_conf_factory : SiteConfFactory,
    best_cost : u32,
    best_pattern : SiteConf,
    best_brancher_pattern : Option<ResultPattern>,
    tested_cuts : HashSet<Cut>,
    scenario_manager : ScenarioManager<'a>,
    allowed_infeasible : usize,
    quorum_accept_percent : u8,
    benevolent_accept_percent : u8,
    max_activate_per_generation : usize,
    activate_all : bool,
    iis_activate : bool,
    total_num_vehicles : i64,
    should_stop : Arc<AtomicBool>,
    gurobi_threads : i32
}


const MAX_FIXED_SIZE : u8 = 4;



const SITE_LOW_LEVEL: u8 = 2;
const SITE_HIGH_LEVEL: u8 = 4;


#[derive(PartialEq)]
enum SubsetFeasibility {
    FEASIBLE,
    UNFEASIBLE,
    UNKNOWN
}

#[derive(PartialEq,Hash,Eq,Clone,Debug)]
struct Cut {
    items : Vec<CutItem>
}

impl Cut {
    fn new(items : Vec<CutItem>) -> Self {
        Self { items}
    }


    fn forbid_site_size_array(size_of_site : &[u8], fixed_size_level : Option<u8>) -> Self {
        // list all level shifts that can be feasible
        let mut cut_items : Vec<CutItem> = Vec::new();
        for (idx,size) in size_of_site.iter().enumerate() {

            if let Some(size_level) = fixed_size_level {
                match *size {
                    0 => {
                        cut_items.push(CutItem::new(idx, size_level));
                    },
                    size_level => { /* cant open anymore */}
                    _ => unreachable!()
                }

            } else {
                match *size {
                    0 => {

                        // can/must open either low or high to change
                        cut_items.push(CutItem::new(idx, SITE_LOW_LEVEL));
                        cut_items.push(CutItem::new(idx, SITE_HIGH_LEVEL));
                    },
                    SITE_LOW_LEVEL => {
                        // can/must open high to change
                        cut_items.push(CutItem::new(idx, SITE_HIGH_LEVEL));
                    }
                    SITE_HIGH_LEVEL => {
                        // cant open anymore
                    }
                    _ => unreachable!()
                }
            }
        }
        return  Cut::new(cut_items);
    }
}

#[derive(PartialEq,Hash,Eq,Clone,Debug)]
struct CutItem {
    site_index : usize,
    open_level : u8
}

impl CutItem {
    fn new(site_index : usize, open_level : u8) -> Self {
        Self {
            site_index, open_level
        }
    }
}

impl<'a> SolutionApproachVariable<'a> {

    pub fn new(
        min_num_sites: usize,
        allowed_infeasible : usize,
        sites: IndexMap<u8, Site>,
        scenario_vehicle_sets : &'a [ &'a [Vehicle<'a>]],
        gurobi_threads : i32,
        sort_many_columns_first : bool,
        quorum_accept_percent : u8,
        benevolent_accept_percent : u8,
        max_activate_per_generation : usize,
        activate_all : bool,
        iis_activate : bool,
        total_num_vehicles : i64,
        env : &'a Env,
        env_integer : &'a Env
    ) -> Self {



        let site_array : Vec<Site> = sites.iter().map(|(_i,site)| site.clone()).collect();

        let site_conf_factory = SiteConfFactory {
            num_sites: site_array.len()
        };





        // create one brancher per vehicle_set item in scenario manager
        let mut scenario_manager = ScenarioManager::new(
            scenario_vehicle_sets.iter().map(|v| {
                Brancher::new(site_array.clone(),
                              v.to_vec(),
                              site_conf_factory.empty(),
                              &env,
                              &env_integer,
                              allowed_infeasible,
                              true,
                              Arc::new(AtomicBool::new(false)),
                              PatternPool::new(v.len())

                )
            }).collect()
        );
        scenario_manager.new_generation();


        let should_stop = Arc::new(AtomicBool::new(false));



        SolutionApproachVariable {
            min_num_sites,
            site_array,
            best_pattern : site_conf_factory.full(MAX_FIXED_SIZE),
            site_conf_factory : site_conf_factory,
            best_cost : u32::MAX,
            allowed_infeasible,
            best_brancher_pattern : None,
            rng : StdRng::seed_from_u64(12345),
            tested_cuts : HashSet::new(),
            scenario_manager,
            should_stop,
            quorum_accept_percent,
            benevolent_accept_percent,
            gurobi_threads,
            max_activate_per_generation,
            activate_all,
            iis_activate,
            total_num_vehicles
        }
    }

    fn load_cuts(&mut self, cut_file_input: &str, model: &mut Model,
                 mut site_open_vars: &IndexMap<usize,Var>, active_cuts: &mut Vec<(Cut, Constr)>, num_cuts: &mut usize, fixed_size_level : Option<u8>) {
        {
            // copy cuts from first level
            if cut_file_input != "/dev/null" {
                let file = File::open(cut_file_input).unwrap();
                let mut lines = io::BufReader::new(file).lines();
                // first line is bound
                if let Some(first_line) = lines.next() {
                    // throw away first line!
                    drop(first_line);
                    // we are unsure if we can update the bound here!
                    // as the cuts could come from an cross check
                    // where the cuts are valid but the bound is to optimistic


                    // rest of the lines are cuts by site id
                    for line in lines {
                        if let Ok(str_line) = line {

                            // format is site_idx@size_level,site_idx@size_level
                            let sites_in_cut: Vec<CutItem> = str_line.split(",").map(|a| {
                                let mut item = a.split("@");
                                let index = item.next().unwrap().parse::<usize>().unwrap();
                                let size = item.next().unwrap().parse::<u8>().unwrap();
                                CutItem::new(index, size)
                            }).collect();


                            // if we are at a size level only take those that are exclusive for
                            // that level
                            if let Some(size_level) = fixed_size_level {
                                    if sites_in_cut.iter().any(|c| c.open_level != size_level) {
                                        continue
                                    }
                            }

                            let cut = Cut::new(sites_in_cut);

                            let constr = model.add_constr(&format!("benderCut[{}]", num_cuts),
                                                          c!(
                                                          Expr::sum(cut.items.iter().map(|c| site_open_vars.get_index(c.site_index).unwrap().1)) >= 1
                                                          )).unwrap();

                            active_cuts.push((cut.clone(), constr));
                            self.tested_cuts.insert(cut);
                            *num_cuts += 1;
                        }
                    }
                    println!("Loaded {} cuts from external file", num_cuts);
                }
            }
        }
    }

    pub fn run(&mut self, should_stop: Arc<AtomicBool>, path_charge_process : &str, cut_file_output: &str, cut_file_input: &str,
               do_low_level : bool,
               do_low_high_swap : bool,
               do_variable_sizing : bool
    ) -> Simple {


        let start_cutting_plane = Instant::now();
        let mut active_cuts : Vec<(Cut,Constr)> = Vec::with_capacity(1000);



        let mut env = Env::new("/tmp/gurobi_cutting_plane_variable.log").unwrap();

        env.set(grb::param::LogToConsole, 0).unwrap();
        env.set(grb::param::Threads, self.gurobi_threads).unwrap();
        env.set(grb::param::Seed, 12345).unwrap();

        #[cfg(not(feature = "column_generation_debug"))]
            env.set(grb::param::OutputFlag, 0).unwrap();


        let mut num_cuts : usize = 0;




        // first do a normal static sized cutting plane here
        // contains only the level_high items


        // create an empty model which associated with `env`:
        let mut cutting_plane_master = Model::with_env("cutting_plane_master", &env).unwrap();
        cutting_plane_master.set_attr(attr::ModelSense, grb::ModelSense::Minimize).unwrap();

        let mut open_site_at_level_high: IndexMap<usize, Var> = IndexMap::default();
        for site in &self.site_array {
            open_site_at_level_high.insert(site.index,
                                           cutting_plane_master.add_var(&format!("openSiteLevelHigh[{}]", site.index), Binary, (f64::from(site.cost_4)), 0.0, 1.0, []).unwrap()
            );
        }


        fn get_trueish_vars(model : &Model, vars : &IndexMap<usize,Var>) -> Vec<bool> {
            model.get_obj_attr_batch(attr::X, vars.iter().map(|(_idx, var)| var).cloned().collect::<Vec<Var>>())
                .unwrap().iter().map(|el| *el > CG_EPSILON).collect()
        }

        // always start at 4, then decrease to target_static_station size




        // can do either low high swap or variable sizing
        assert!( !(do_low_high_swap == true && do_variable_sizing == true));


        let target_static_station_size;
        let mut current_station_size;

        // set up static loop based on input settings
        match (do_low_high_swap,do_variable_sizing){
            (false, false) => {
                target_static_station_size =
                    if do_low_level {SITE_LOW_LEVEL} else {SITE_HIGH_LEVEL};
                current_station_size = target_static_station_size;

            }, // if neither high or low swap, set sizing level to requested
            (true, false) => {
                target_static_station_size = SITE_LOW_LEVEL;
                current_station_size = SITE_HIGH_LEVEL;
            }, // if we do high low swap, target low lwevel
            (false, true) => {
                target_static_station_size = SITE_HIGH_LEVEL;
                current_station_size = SITE_HIGH_LEVEL;
            }, // if we do variable sizing: target high level first, low level is dealt with later,
            _ => unreachable!("Invalid combination")
        };

        println!("Target: {}, Start:{}", target_static_station_size, current_station_size);

        if self.activate_all{
            for s in 0..self.scenario_manager.branchers.len() {
                self.scenario_manager.activate(s);
            }
        } else {
            self.scenario_manager.activate(0);
        }

        {
            self.load_cuts(cut_file_input, &mut cutting_plane_master, &mut open_site_at_level_high, &mut active_cuts, &mut num_cuts, Some(current_station_size));
        }

        while current_station_size >= target_static_station_size {
            scoped_tracepoint!(_station_size_loop);

            // reset best_information
            // the best_pattern might have been from a previous, larger target level
            // therefore not a feasible upper bound at the lower level
            // we are still dealing with fixed station sizes not variable
            // otherwise bound would work!
            self.best_pattern = self.site_conf_factory.full(current_station_size);
            self.best_cost = u32::MAX;
            self.best_brancher_pattern = None;
            self.tested_cuts = HashSet::new();




            // update objective function contribution for closing stations based on current station size
            for site in &self.site_array {
                let v: &Var = &open_site_at_level_high[site.index];
                cutting_plane_master.set_obj_attr(grb::attr::Obj, v, f64::from(
                    if current_station_size == SITE_LOW_LEVEL { site.cost_2}
                         else if current_station_size == SITE_HIGH_LEVEL { site.cost_4}
                         else { unreachable!()}
                )
                ).unwrap();
            }

            let mut last_pattern_cost = u32::MAX;

            'scenarioLoop: loop {
                // reset the pattern stats, as now our brancher setup might look different!
                self.best_cost = std::u32::MAX;
                self.best_pattern = self.site_conf_factory.full(MAX_FIXED_SIZE);

                'patternLoop: loop {
                    scoped_tracepoint!(cut_loop);

                    if should_stop.load(Relaxed) {
                        break;
                    }

                    {
                        scoped_tracepoint!(bd_master_optimize);
                        cutting_plane_master.optimize().unwrap();
                    }


                    if cutting_plane_master.status().unwrap() != Status::Optimal {
                        panic!("{}", "Error in solving cutting plane master!");
                    }


                    let opened_site_levels: Vec<bool> = get_trueish_vars(&cutting_plane_master, &open_site_at_level_high);
                    let mut current_pattern: SiteConf = self.site_conf_factory.empty();
                    for (pattern_val, is_open) in current_pattern.iter_mut().zip(&opened_site_levels) {
                        *pattern_val = if *is_open { current_station_size } else { 0 }
                    }
                    let pattern_cost = self.get_pattern_cost(&current_pattern);


                    {
                        scoped_tracepoint!(bd_test_configuration);

                        let num_active = self.scenario_manager.num_active();
                        let quorum_required = (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;


                        let results = self.scenario_manager
                            .get_active_branchers()
                            .map(|(_idx, br)| {
                                br.replace_site_sizes(current_pattern.clone());
                                br.solve(false, false)
                            });

                        let mut oracle_ok = 0;
                        let mut oracle_denied = 0;

                        for result in results {
                            if result.is_ok() {
                                oracle_ok += 1;

                            } else {
                                oracle_denied += 1;
                            }

                            if oracle_ok >= quorum_required {

                                // if it is ok we are done!
                                #[cfg(feature = "cutting_plane_debug")] {
                                    println!("BEND|{:?}|{}|{}|{}|{:?}", &current_pattern, &pattern_cost, &self.best_cost, start_cutting_plane.elapsed().as_secs(), true);
                                    println!("DONE!");
                                }


                                self.best_cost = pattern_cost;
                                self.best_pattern = current_pattern.clone();
                                //TODO: SET BEST BRANCHER PATTERN
                                //best_brancher_pattern = Vec::new();
                                break 'patternLoop; // we have a good pattern, exit the outer loop
                            } else if num_active - oracle_denied < quorum_required { // quorum is not reachable anymore
                                break; // is not feasible, no need to search anymore so "continue" with cut generation
                            }
                        }
                    }


                    if opened_site_levels.iter().all(|f| *f) {
                        panic!("{}", "Opened all sites!");
                    }


                    #[cfg(feature = "cutting_plane_debug")]
                    println!("BEND|{:?}|{}|{}|cols:{}|{}|{:?}", current_pattern, pattern_cost, self.best_cost, self.scenario_manager.branchers.iter().map(|b| b.get_num_colums()).sum::<usize>(), start_cutting_plane.elapsed().as_secs(), false);


                    let delta_pattern_cost = pattern_cost - last_pattern_cost;
                    last_pattern_cost = pattern_cost;

                    // since we are infeasible try to generate cuts
                    let mut potential_cuts = self.improve_cuts(&current_pattern,
                                                               Duration::from_secs(cutting_plane_master.get_attr(grb::attr::Runtime).unwrap().round() as u64),
                                                               delta_pattern_cost,
                                                               Some(current_station_size)
                    );



                    potential_cuts.sort_by(|a, b| a.items.len().cmp(&b.items.len()));

                    'nextPotentialCut: for cut in potential_cuts.iter() {


                        // do dominance management on cuts
                        let mut do_skip_this_cut = false;
/*
                        active_cuts.retain(|(cut_pattern, cut_constr)| {
                            if cut_pattern.items.len() > cut.items.len() {
                                // if the existing is larger, maybe new dominates old?
                                if cut.items.iter().all(|v| cut_pattern.items.contains(v)) {
                                    // new dominates old ! remove old
                                    cutting_plane_master.remove(cut_constr.clone()).unwrap();
                                    #[cfg(feature = "pattern_generation_debug")]
                                    println!("Removed cut from master");
                                    return false;
                                }
                            } else {
                                // if existing is smaller or equal, then maybe it dominates it?
                                // if equal, then must be different!
                                if cut_pattern.items.iter().all(|v| cut.items.contains(v)) {
                                    // old dominates new, do not add!
                                    do_skip_this_cut = true;
                                    return true
                                }
                            }
                            true
                        });

                        if do_skip_this_cut {
                            continue 'nextPotentialCut;
                        }
*/

                        let constr = cutting_plane_master.add_constr(&format!("benderCut[{}]", num_cuts),
                                                                     c!(
                                                           Expr::sum(
                                                                   cut.items.iter().map(|item| open_site_at_level_high.get_index(item.site_index).unwrap().1))
                                                              >= 1
                                                               )
                        ).unwrap();
                        active_cuts.push((cut.clone(), constr));
                    };
                }

                if self.evaluate_all_scenarios_and_update_active(&should_stop) {
                    break 'scenarioLoop;
                }
            }
            // decrease station size by 2, only usefull if we did start with value != target size
            current_station_size -= 2;
        }


        // DONE: With all static loops.


        if do_variable_sizing {
            // if we are variable, then proceed with variable sizing model,
            // but we keep the cuts from high bit

            let mut open_site_at_level_low: IndexMap<usize, Var> = IndexMap::default();
            for site in &self.site_array {

                // for all cuts that require a site to be opened at the old (high) level,
                // there is now the possibility to open the site at the low level instead
                // we collect the constraints for those cuts and include them in the col
                // vector of the variable with a coeef of 1
                let high_level_cuts_where_site_is_included: Vec<(Constr, f64)> = active_cuts.iter().filter_map(|(cut, constr)| {
                    let cut: &Cut = cut;
                    for i in &cut.items {
                        if i.site_index == site.index {
                            return Some((constr.clone(), 1.0))
                        }
                    }

                    return None
                }).collect();

                open_site_at_level_low.insert(site.index,
                                              cutting_plane_master.add_var(&format!("openSiteLevelLow\
                                 [{}]", site.index), Binary, f64::from(site.cost_2), 0.0, 1.0, high_level_cuts_where_site_is_included).unwrap()
                );

                // add convexity constraint: Cant open at both levels.
                cutting_plane_master.add_constr(&format!("closeSizeConv[{}]", site.index), c!(open_site_at_level_low[site.index] + open_site_at_level_high[site.index] <= 1.0)).unwrap();
            }


            let mut last_pattern_cost = 0;

            'scenarioLoop: loop {
                self.best_cost = std::u32::MAX;
                self.best_pattern = self.site_conf_factory.full(MAX_FIXED_SIZE);

                'patternLoop: loop {
                    scoped_tracepoint!(_cut_loop);

                    if should_stop.load(Relaxed) {
                        break;
                    }

                    {
                        scoped_tracepoint!(_bd_master_optimize);
                        cutting_plane_master.optimize().unwrap();
                    }


                    if cutting_plane_master.status().unwrap() != Status::Optimal {
                        cutting_plane_master.write("/tmp/lp.lp").unwrap();
                        panic!("{}: {:?}", "Error in solving cutting plane master!", cutting_plane_master.status().unwrap());
                    }


                    let result_open_high: Vec<bool> = get_trueish_vars(&cutting_plane_master, &open_site_at_level_high);
                    let result_open_low: Vec<bool> = get_trueish_vars(&cutting_plane_master, &open_site_at_level_low);

                    let size_of_site: Vec<u8> = self.site_array.iter().map(|site| {
                        // must not both be true
                        assert!(!(result_open_high[site.index] && result_open_low[site.index]));
                        if result_open_high[site.index] {
                            SITE_HIGH_LEVEL
                        } else if result_open_low[site.index] {
                            SITE_LOW_LEVEL
                        } else {
                            0
                        }
                    }).collect();


                    // create pattern
                    let mut current_pattern: SiteConf = self.site_conf_factory.empty();
                    for (pattern_val, site_size) in current_pattern.iter_mut().zip(&size_of_site) {
                        *pattern_val = *site_size
                    }
                    // calculate pattern cost

                    let pattern_cost = self.get_pattern_cost(&current_pattern);
                    let delta_pattern_cost = pattern_cost - last_pattern_cost;
                    last_pattern_cost = pattern_cost;

                    {
                        scoped_tracepoint!(bd_test_configuration);

                        let num_active = self.scenario_manager.num_active();
                        let quorum_required = (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;

                        let results = self.scenario_manager
                            .get_active_branchers()
                            .map(|(_idx, br)| {
                                br.replace_site_sizes(current_pattern.clone());
                                br.solve(false, false)
                            });

                        let mut oracle_ok = 0;
                        let mut oracle_denied = 0;

                        for result in results {
                            if result.is_ok() {
                                oracle_ok += 1;
                            } else {
                                oracle_denied += 1;
                            }
                            if oracle_ok >= quorum_required {
                                // if it is ok we are done!
                                #[cfg(feature = "cutting_plane_debug")] {
                                    println!("BEND|{:?}|{}|{}|{}|{:?}", &current_pattern, &pattern_cost, &self.best_cost, start_cutting_plane.elapsed().as_secs(), true);
                                    println!("DONE!");
                                }
                                self.best_cost = pattern_cost;
                                self.best_pattern = current_pattern.clone();
                                //TODO: SET BEST BRANCHER PATTERN
                                //best_brancher_pattern = Vec::new();
                                break 'patternLoop; // we have a good pattern, exit the outer loop
                            } else if num_active - oracle_denied < quorum_required { // quorum is not reachable anymore
                                break; // is not feasible, no need to search anymore so "continue" with cut generation
                            }
                        }
                    }

                    let all_open = (size_of_site.len() * SITE_HIGH_LEVEL as usize) as u16;
                    if size_of_site.iter().fold(0_u16, |a, b| a + (*b as u16)) == all_open {
                        panic!("{}", "Opened all sites!");
                    }

                    #[cfg(feature = "cutting_plane_debug")]
                    println!("BEND|{:?}|{}|{}|{}|{:?}", current_pattern, pattern_cost, self.best_cost, start_cutting_plane.elapsed().as_secs(), false);


                    // since we are infeasible try to generate cuts
                    let mut potential_cuts = self.improve_cuts(&size_of_site, Duration::from_secs(cutting_plane_master.get_attr(attr::Runtime).unwrap().round() as u64), delta_pattern_cost, None);

                    potential_cuts.sort_unstable_by(|a, b| a.items.len().cmp(&b.items.len()));


                    'nextPotentialCut: for cut in potential_cuts.iter() {
                        if cut.items.is_empty() {
                            continue 'nextPotentialCut;
                        }

                        // do dominance management on cuts
                        let mut do_skip_this_cut = false;
/*
                        active_cuts.retain(|(existing_cut_pattern, cut_constr)| {
                            if existing_cut_pattern.items.len() > cut.items.len() {
                                // if the existing is larger, maybe new dominates old?
                                if cut.items.iter().all(|v| existing_cut_pattern.items.contains(v)) {
                                    // new dominates old ! remove old
                                    cutting_plane_master.remove(cut_constr.clone()).unwrap();
                                    #[cfg(feature = "pattern_generation_debug")]
                                    println!("Removed cut from master");
                                    return false;
                                }
                            } else {
                                // if existing is smaller or equal, then maybe it dominates it?
                                // if equal, then must be different!
                                if existing_cut_pattern.items.iter().all(|v| cut.items.contains(v)) {
                                    // old dominates new, do not add!
                                    do_skip_this_cut = true;
                                    return true
                                }
                            }
                            true
                        });

                        if do_skip_this_cut {
                            #[cfg(feature = "cutting_plane_lifting_debug")]
                            println!("Skipping cut");
                            
                            continue 'nextPotentialCut;
                        }

*/
                        #[cfg(feature = "cutting_plane_lifting_debug")]
                        println!("LFT|IMPROVED_COVER|{:?}", cut);

                        num_cuts += 1;


                        let mut open_set: Vec<Var> = cut.items.iter().filter_map(|ci| {
                            match ci.open_level {
                                SITE_HIGH_LEVEL => Some(open_site_at_level_high[&ci.site_index].clone()),
                                SITE_LOW_LEVEL => Some(open_site_at_level_low[&ci.site_index].clone()),
                                0 => None,
                                _ => unreachable!()
                            }
                        }
                        ).collect();



                        #[cfg(feature = "pattern_generation_debug")]
                        println!("Adding cut that not all of {:?} can be set, total : {}", cut, active_cuts.len());


                        let constr = cutting_plane_master.add_constr(&format!("planeCut[{}]", num_cuts),
                                                                     c!( Expr::sum(open_set.iter())
                                                            >= 1.0)).unwrap();


                        active_cuts.push((cut.clone(),constr));




                    };
                }

                if self.scenario_manager.branchers.len() == 0 /* = deterministic case */ || self.evaluate_all_scenarios_and_update_active(&should_stop) {
                    break 'scenarioLoop;
                }

            }
        }


        println!("Best cost {} with pattern {:?}", self.best_cost,self.best_pattern);
        println!("# {} open sites", self.best_pattern.iter().filter(|i| **i > 0).count());



        let open_sites = self.site_array.iter().map(|site| {
            (self.best_pattern[site.index],site.index)
        } ).collect();


        // write charge processes to file
        {
            if path_charge_process != "/dev/null" {
                let write_file = File::create(path_charge_process).unwrap();
                let mut writer = BufWriter::new(&write_file);

                for (vehicle, patterns) in self.best_brancher_pattern.as_ref().unwrap() {
                    for (_segment, site, time) in patterns {
                        write!(&mut writer, "{},{},{}\n", vehicle.index(), site.index(), time).unwrap();
                    }
                }
            }
        }

        // write cuts to file
        {
            if cut_file_output != "/dev/null" {
                let write_file = File::create(cut_file_output).unwrap();
                let mut writer = BufWriter::new(&write_file);
                write!(&mut writer, "{}\n", self.best_cost).unwrap();
                for (cut, _) in active_cuts {
                    write!(&mut writer, "{}\n", cut.items.iter().map(|e| format!("{}@{}", e.site_index, e.open_level)).join(",")).unwrap();
                }
            }
        }

        println!("Total Number of Columns: {}", self.scenario_manager.branchers.iter().map(|b| b.get_num_colums()).sum::<usize>());


        Simple {

            cost : u64::from(self.best_cost),
            sites_open : open_sites,


        }
    }

    fn evaluate_all_scenarios_and_update_active(&mut self, should_stop: &Arc<AtomicBool>)  -> bool{
        let mut infeasible_scenarios = Vec::new();

        for (bidx, b) in self.scenario_manager.get_all_branchers() {
            b.replace_site_sizes(self.best_pattern.clone());
            match b.solve(false, true) {
                Ok(_) => {
                    println!("Feasible for {:?}", &self.best_pattern)
                },

                Err(SolveError::VehiclesInfeasible(infs)) => {
                    let count_infs = infs.len();


                    let num_vehicles_base_for_benevolent = if self.total_num_vehicles > 0 {
                        self.total_num_vehicles as usize
                    } else {
                        b.get_vehicles().len()
                    };


                    let external_infeasibility_penalty = if self.total_num_vehicles > 0 {
                        assert!(self.total_num_vehicles > b.get_vehicles().len() as i64);
                        (self.total_num_vehicles as usize) - b.get_vehicles().len()
                    } else {
                        0
                    };

                    let benevolent_accept_limit_count = (
                        (self.benevolent_accept_percent as f32 / 100.0) * num_vehicles_base_for_benevolent as f32).floor() as usize;


                    if count_infs + external_infeasibility_penalty > benevolent_accept_limit_count {
                        println!("Infeasible count = {} (+{})", infs.len(), external_infeasibility_penalty);
                        infeasible_scenarios.push((infs, b.get_vehicles().len(), bidx));
                    } else {
                        println!("Benevolent Feasible ( inf count = {})", infs.len());
                    }
                },
                Err(SolveError::Generic(msg)) => panic!("{}", msg),
                Err(SolveError::StoppedByExternal) => panic!("{}", "InvalidError"),
                Err(SolveError::NoQuickIntegerResult) => panic!("{}", "InvalidError"),
                Err(SolveError::NoQuickResult) => panic!("{}", "InvalidError")
            }
        }

        let num_active = self.scenario_manager.num_active();
        let quorum_required = (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;
        let infeasible_allowed_through_quorum = num_active - quorum_required;

        if infeasible_scenarios.is_empty() || infeasible_scenarios.len() <= infeasible_allowed_through_quorum {
            return true;
        }

        infeasible_scenarios.sort_by_key(|(inf, _, _)| inf.len());

        self.scenario_manager.new_generation();

        for (inf, num_vehicles, idx) in infeasible_scenarios.into_iter().take(self.max_activate_per_generation) {
            let num_vehicles_base_for_benevolent = if self.total_num_vehicles > 0 {
                self.total_num_vehicles as usize
            } else {
                num_vehicles
            };

            let external_infeasibility_penalty = if self.total_num_vehicles > 0 {
                assert!(self.total_num_vehicles > num_vehicles as i64);
                (self.total_num_vehicles as usize) - num_vehicles
            } else {
                0
            };

            let benevolent_accept_limit_count = (
                (self.benevolent_accept_percent as f32 / 100.0) * num_vehicles_base_for_benevolent as f32).floor() as usize;

            if self.iis_activate {
                // create virtual scenario with only the infeasible vehicles!

                // take only the ones above the benevolent inf level
                assert!(inf.len() + external_infeasibility_penalty > benevolent_accept_limit_count);
                let num_inf_above_level = (inf.len() + external_infeasibility_penalty) - benevolent_accept_limit_count;
                let reduced_inf_list: HashSet<VehicleIndex> = inf.into_iter().take(num_inf_above_level).collect();

                // get brancher
                let from_vehicles = self.scenario_manager.branchers[idx].get_vehicles();

                let mut brancher_vehicles : Vec<Vehicle> = Vec::with_capacity(from_vehicles.len());
                let mut columns : Vec<Vec<PatternEntry>> = Vec::with_capacity(from_vehicles.len());


                // this is needed to remap the vehicles to the columns
                for (ix,fv) in from_vehicles.into_iter().enumerate() {
                    if reduced_inf_list.contains(&VehicleIndex::new(fv)) {

                        // add vehicle to new list
                        let mut new_vehicle = fv.clone();
                        new_vehicle.index = brancher_vehicles.len();
                        brancher_vehicles.push(new_vehicle);
                        // extract and add columns
                        columns.push(self.scenario_manager.branchers[idx].get_pattern_pool().store_at_index(ix).clone());


                    }
                }




                // then we must map
                let new_brancher = Brancher::new(
                    self.site_array.clone(),
                    brancher_vehicles,
                    self.site_conf_factory.empty(),
                    self.scenario_manager.branchers[0].env,
                    self.scenario_manager.branchers[0].env_integer,
                    self.allowed_infeasible,
                    true,
                    should_stop.clone(),
                    PatternPool::new_with_store(columns)
                );


                println!("Activated {} with {} of {} vehicles", idx, new_brancher.get_vehicles().len(),from_vehicles.len());
                self.scenario_manager.add_brancher_and_activate(new_brancher);



            } else {
                println!("Activating {}", idx);
                self.scenario_manager.activate(idx);
            }
        }

        return false;
    }


    fn get_pattern_cost(&self, pattern : &SiteConf) -> u32 {
        pattern.iter().zip(self.site_array.iter()).map(|(&size,site)| {
            if size == 0 {
                0_u32
            } else {
                u32::from(

                    if size == SITE_LOW_LEVEL { site.cost_2}
                    else if size == SITE_HIGH_LEVEL { site.cost_4}
                    else { unreachable!()}

                )
            }
        }).sum()
    }

    fn record_feasible_solution(&mut self, current_pattern : SiteConf, brancher_pattern : ResultPattern) {
        // test pattern
        let pattern_cost =  self.get_pattern_cost(&current_pattern);

        #[cfg(feature = "pattern_generation_debug")]
        println!("FOUND|Accidentially found new feasible!");
        if self.best_cost > pattern_cost {
            self.best_cost = pattern_cost ;
            self.best_pattern = current_pattern;
            self.best_brancher_pattern = Some(brancher_pattern);
        }
    }




    fn subset_is_feasible(&mut self, site_sizes: &[u8]) -> bool {
            let mut current_pattern: SiteConf = self.site_conf_factory.empty();
            for (idx, el) in current_pattern.iter_mut().enumerate() {
                *el = site_sizes[idx];
            }


         let num_active = self.scenario_manager.num_active();
         let quorum_required =  (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;


        let results = self.scenario_manager
            .get_active_branchers()
            .map(|(_idx,br)| {
                br.replace_site_sizes(current_pattern.clone());
                br.solve(false, false)
            });

        let mut oracle_ok  = 0;
        let mut oracle_denied = 0;

        for result in results {

            let testing_result = match result {
                Ok(_) => SubsetFeasibility::FEASIBLE,
                Err(SolveError::VehiclesInfeasible(_)) =>  SubsetFeasibility::UNFEASIBLE,
                Err(SolveError::Generic(_)) =>  SubsetFeasibility::UNFEASIBLE,
                Err(SolveError::StoppedByExternal) => panic!("{}", "Should not be unknown"),
                Err(SolveError::NoQuickIntegerResult) => panic!("{}", "Should not be unknown"),
                Err(SolveError::NoQuickResult) => panic!("{}", "Should not be unknown")
            };



            if testing_result == SubsetFeasibility::FEASIBLE {
                oracle_ok += 1;
            } else if testing_result == SubsetFeasibility::UNFEASIBLE {
                oracle_denied += 1;
            }




            if oracle_ok >= quorum_required {
                let pattern_cost: u32 = current_pattern.iter().zip(self.site_array.iter()).map(|(&size, site)| {
                    if size == 0 {
                        0_u32
                    } else {
                        u32::from(
                            if size == SITE_LOW_LEVEL { site.cost_2}
                            else if size == SITE_HIGH_LEVEL { site.cost_4}
                            else { unreachable!()}
                        )
                    }
                }).sum();


                #[cfg(feature = "pattern_generation_debug")]
                println!("FOUND|Accidentially found new feasible");
                if self.best_cost > pattern_cost {
                    self.best_cost = pattern_cost;
                    self.best_pattern = current_pattern;
                    // TODO FIX BEST BRANCHER PATTERN
                    self.best_brancher_pattern = Some(Vec::new());

                    #[cfg(feature = "pattern_generation_debug")]{
                        println!("FOUND|NEW BEST\t{:?}\t{:?}",best_pattern, best_cost );
                    }

                }
                return true;
            } else if num_active - oracle_denied < quorum_required { // quorum is not reachable anymore {
                return false;
            }

        }

        return false;

    }

    fn improve_cuts(&mut self,
                    size_sizes : &[u8],
                    runtime_cutting_plane_master: Duration,
                    delta_pattern_cost : u32,
                    has_fixed_level : Option<u8>
    ) -> Vec<Cut> {

        let original_cut = Cut::forbid_site_size_array(size_sizes, has_fixed_level);







        let mut potential_cuts : Vec<Cut> = Vec::new();

        //#[cfg(feature = "pattern_generation_improve_cuts")]
        {

            // we will here try to improve the cuts using a heuristic
            // the allowed time budget is the same as the time spend
            // in the cutting_plane_master problem (the harder the problem gets
            // the more time do we spend in the heuristics

            let runtime_cut_loop = max(Duration::from_secs(5), runtime_cutting_plane_master);

            // try to generate smaller cut
            scoped_tracepoint!(_bd_lift_cuts);
            let start_cut_loop = Instant::now();
            let mut has_found_improvement = false;
            let max_duration_without_improvement = Duration::from_secs(60 * 30);


            while start_cut_loop.elapsed() < runtime_cut_loop {

                if ! has_found_improvement && start_cut_loop.elapsed() > max_duration_without_improvement {
                    break;
                }

                let mut working_size_set: Vec<u8> = Vec::from(size_sizes.clone());


                let mut indexes_still_openable : Vec<usize> = working_size_set.iter().enumerate().filter_map(|(idx,size)| {
                    if (*size < if let Some(to_level) = has_fixed_level { to_level} else { SITE_HIGH_LEVEL  } ) {
                        Some(idx)
                    } else {
                        None
                    }
                }).collect();

                if indexes_still_openable.is_empty() {
                    break
                }


                while indexes_still_openable.len() > 1 {



                    // was macht der alte code:




                    let count = if indexes_still_openable.len() == 2 {
                        1 /* gen range does not like min = max */
                    } else if delta_pattern_cost <= 2 {
                        // if we are in a phase where there is minimal improvement
                        // restrict ourselves to smaller changes
                        self.rng.gen_range(1..(indexes_still_openable.len() - 1).min(3))
                    } else {
                        self.rng.gen_range((indexes_still_openable.len().div_floor(2)).min(indexes_still_openable.len() - 2)..indexes_still_openable.len() - 1)
                    };

                    let sites_to_open : Vec<usize>  = indexes_still_openable.choose_multiple(&mut self.rng, count).cloned().collect();
                    let before_update =   working_size_set.clone();
                    for (site_idx, size) in working_size_set.iter_mut().enumerate() {

                        if sites_to_open.contains(&site_idx) {

                            // if we are still in fixed size mode,
                            // we just flip from 0 to the size level
                            // no other move will be suggested
                            if let Some(to_level) = has_fixed_level {
                                *size = match *size {
                                    0 => to_level,
                                    _ => unreachable!("size was {} not fixed {}", size, to_level)
                                }
                            }  else {
                                // otherwise we will apply the increase from 0 to low, low to high
                                *size = match *size {
                                    0 =>  SITE_LOW_LEVEL,
                                    SITE_LOW_LEVEL => SITE_HIGH_LEVEL,
                                    _ => unreachable!()
                                }
                            }
                        }
                    }



                    indexes_still_openable= working_size_set.iter().enumerate().filter_map(|(idx,size)| {
                        if (*size < if let Some(to_level) = has_fixed_level { to_level} else { SITE_HIGH_LEVEL  } ) {
                            Some(idx)
                        } else {
                            None
                        }
                    }).collect();


                    let cut_perspective = Cut::forbid_site_size_array(&working_size_set, has_fixed_level);




                    // sample again if we already tested this subset!
                    // set insert returns false if element was already in set
                    if ! self.tested_cuts.insert(cut_perspective.clone()) {
                        continue;
                    }


                    if ! self.subset_is_feasible(&working_size_set) {

                            has_found_improvement = true;
                            potential_cuts.push(cut_perspective.clone());
                        } else {

                            break
                    }


                }
            }
        }


        if potential_cuts.is_empty() {
            potential_cuts.push(original_cut);
        }


        return  potential_cuts;
    }
}
