use std::sync::Arc;
use std::sync::atomic::AtomicBool;


use indexmap::IndexMap;
use itertools::Itertools;
use shared::{Simple, Site, Vehicle};


use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use crate::fixed_size::brancher::{Brancher, SolveError};

use crate::{SiteArray, CG_EPSILON};

use crate::fixed_size::brancher::ResultPattern;

use rust_hawktracer::*;

use rand::prelude::{StdRng, SliceRandom};
use rand::{SeedableRng, Rng};
use gurobi::*;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use std::collections::HashSet;
use std::time::{Instant, Duration};
use std::sync::atomic::Ordering::Relaxed;
use std::io;
use std::io::BufRead;
use std::cmp::max;


struct ScenarioManager<'a> {
    pub branchers : Vec<Brancher<'a>>,
    pub active_sets : Vec<bool>,
    pub generation_set : Vec<bool>

}

impl <'a> ScenarioManager<'a> {

    pub fn new(branchers : Vec<Brancher<'a>>) -> Self {

        let active_sets = vec![false;branchers.len()];
        let generation_set = vec![false;branchers.len()];
        ScenarioManager{
            branchers,
            active_sets,
            generation_set
        }
    }

    pub fn add_brancher_and_activate(&mut self, brancher : Brancher<'a>) {
        self.branchers.push(brancher);
        self.active_sets.push(true);
        self.generation_set.push(true);
    }

    pub fn new_generation(&mut self) {
        self.generation_set = vec![false;self.branchers.len()];
    }


    pub fn get_all_branchers(&mut self) -> impl Iterator<Item =  (usize,&mut Brancher<'a>)>{
        self.branchers.iter_mut().enumerate()
    }

    pub fn get_active_branchers(&mut self) -> impl Iterator<Item = (usize,&mut Brancher<'a>)>{
        self.active_sets.iter().zip(self.branchers.iter_mut().enumerate())
            .filter_map(|(f, b) | {
                if *f == true {
                    Some(b)
                } else {
                    None
                }
            })
    }


    pub fn get_inactive_branchers(&mut self) -> impl Iterator<Item =  (usize,&mut Brancher<'a>)>{
        self.active_sets.iter().zip(self.branchers.iter_mut().enumerate())
            .filter_map(|(f, b) | {
                if *f == false {
                    Some(b)
                } else {
                    None
                }
            })

    }



    pub fn activate(&mut self, index : usize) {
        self.active_sets[index] = true;
        self.generation_set[index] =true;
    }

    pub fn num_active(&self) -> usize {
        self.active_sets.iter().filter(|x| **x).count()
    }

    pub fn deactivate(&mut self, index : usize) {
        self.active_sets[index] = false;
        self.generation_set[index] = true;
    }
}

#[derive(PartialEq)]
enum SubsetFeasibility {
    FEASIBLE,
    UNFEASIBLE,
    UNKNOWN
}

pub struct BendersRobust<'a> {
    scenarion_manager : ScenarioManager<'a>,
    vehicles_sets: &'a[ &'a [Vehicle<'a>]],
    sites: &'a IndexMap<u8, Site>,
    site_conf_factory : SiteConfFactory,
    rng : StdRng,
    site_array : Vec<Site>,
    allowed_infeasible : usize,
    quorum_accept_percent : u8,
    benevolent_accept_percent : u8,
    max_activate_per_generation : usize,
    activate_all : bool,
    iis_activate : bool,
    total_num_vehicles : i64,

    best_cost : u32,
    best_pattern : SiteConf,
    best_brancher_pattern : Option<ResultPattern<'a>>,
}

const MAX_FIXED_SIZE: u8 = 4;

impl<'a> BendersRobust<'a> {

    pub fn new(  sites: &'a IndexMap<u8, Site>,
                 vehicles_sets: &'a[ &'a [Vehicle<'a>]],
                 allowed_infeasible : usize,
                 quorum_accept_percent : u8,
                 benevolent_accept_percent : u8,
                 max_activate_per_generation : usize,
                 activate_all : bool,
                 iis_activate : bool,
                 total_num_vehicles : i64
                    ) -> Self {

        let mut rng: StdRng = StdRng::seed_from_u64(12345);


        let site_array : Vec<Site>  = sites.iter().map(|(_i,site)| site.clone()).collect();


        let site_conf_factory = SiteConfFactory {
            num_sites: site_array.len()
        };

        // create one brancher per vehicle in scenario manager
        let mut scenarion_manager = ScenarioManager::new(
            vehicles_sets.iter().map(|v| {
                Brancher::new(site_array.clone(),
                              v,
                              site_conf_factory.empty(),
                              allowed_infeasible,
                              Arc::new(AtomicBool::new(false))
                )
            }).collect()
        );
        scenarion_manager.new_generation();


        BendersRobust {
            scenarion_manager,
            vehicles_sets,
            sites,
            rng,
            site_array,
            allowed_infeasible,
            quorum_accept_percent,
            benevolent_accept_percent ,
            max_activate_per_generation,
            activate_all,
            iis_activate,
            total_num_vehicles,
            best_cost : u32::MAX,
            best_brancher_pattern : None,
            best_pattern : site_conf_factory.full(MAX_FIXED_SIZE),
            site_conf_factory,
        }

    }


    fn subset_is_feasible (&mut self, smaller_subset : &Vec<usize>) -> bool {
        let mut current_pattern: SiteConf = self.site_conf_factory.empty();
        for (idx, el) in current_pattern.iter_mut().enumerate() {
            if !smaller_subset.contains(&idx) {
                *el = u8::min(self.site_array[idx].capacity,MAX_FIXED_SIZE)
            }
        }


        #[cfg(feature = "pattern_generation_debug")]
        println!("CUT|{:?}", current_pattern);

        let num_active = self.scenarion_manager.num_active();
        let quorum_required =  (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;


        let results = self.scenarion_manager
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
                    u32::from(site.cost + site.charger_cost * size as u8)
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

    pub fn run(&mut self,
                   should_stop: Arc<AtomicBool>,
                   path_charge_process : &str,
                   cut_file_output: &str,
                   cut_file_input: &str,

                ) -> Simple {






        let start_benders = Instant::now();




        /*
            WARNUNG.
             Im Moment lade ich die cuts von scenario 1 als input cuts
             wenn das hier angepasst wird, müssen auch die input cuts
             im snakefile angepasst werden!

         */
        if self.activate_all {
            for s in 0..self.vehicles_sets.len() {
                self.scenarion_manager.activate(s);
            }
        } else {
            self.scenarion_manager.activate(0);
        }

        // erstelle pro vehicle set == scenario einen separaten brancher
        // Dann habe einen scenario manager 
        //  -> optimisation set
        //  -> optimize über diese
        // am ende, test welche in das scenario set kommen
        // quorum behalten, cuts behalten (wenn ich nie wieder einen rausnehme!)


        // initialize vehicle sets


        let mut env = Env::new("/tmp/gurobi_benders.log").unwrap();
        env.set(param::LogToConsole, 0).unwrap();
        env.set(param::Threads, 1).unwrap();
        env.set(param::Seed, 12345).unwrap();

        #[cfg(not(feature = "column_generation_debug"))]
            env.set(param::OutputFlag, 0).unwrap();

        // create an empty model which associated with `env`:
        let mut benders_master = Model::new("benders_master", &env).unwrap();
        benders_master.set(attr::ModelSense, ModelSense::Minimize.into()).unwrap();

        let mut close_sites: IndexMap<u8, Var> = IndexMap::default();

        for (idx, site) in self.sites {
            close_sites.insert(*idx,
                               benders_master.add_var(&format!("closeSite[{}]", idx), Binary, -1.0 * f64::from(site.cost + MAX_FIXED_SIZE * site.charger_cost), 0.0, 1.0, &[], &[]).unwrap()
            );
        }


        let mut tested_cuts: HashSet<Vec<usize>> = HashSet::new();
        let mut active_cuts: Vec<(Vec<usize>, Constr)> = Vec::with_capacity(1000);


        let mut num_cuts: usize = 0;

        { // copy cuts from first level

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

                        // format is site_idx:size_level,site_idx:size_level
                        // size level must be eq to fixed site size in this fixed size benders!
                        let sites_in_cut: Vec<usize> = str_line.split(",").map(|a| {
                            let mut item = a.split(":");
                            let index = item.next().unwrap();
                            let size = item.next().unwrap().parse::<u8>().unwrap();
                            assert_eq!(size, 0);
                            index.parse::<usize>().unwrap()
                        }).collect();

                        let constr = benders_master.add_constr(&format!("benderCut[{}]", num_cuts),
                                                               sites_in_cut.iter().map(|idx| close_sites.get_index(*idx).unwrap()).fold(LinExpr::new(), |a, b| a + b.1)
                                                               , Less, (sites_in_cut.len() - 1) as f64).unwrap();


                        active_cuts.push((sites_in_cut.clone(), constr));
                        tested_cuts.insert(sites_in_cut);


                        num_cuts += 1;
                    }
                }


                println!("Loaded {} cuts from external file", num_cuts);
            }
        }


        // jetzt loopen solange die aktuelle beste lösung ungültig auf dem 
        // vollen vehicle set is, ggf das optimisation set halt erweitern!
        
        // in diesem fall sogar tauschen!, weil ja unabhängig
        // wenn nur in der benders loop getestet wird, alle anderen
        // in der cut improvement loop müssen durch das ganze modell
        // evtl ist das aber schon ganz gut performance improvement!
        
        
        let mut best_cost : u32 = std::u32::MAX;
        let mut best_pattern :SiteConf = self.site_conf_factory.full(MAX_FIXED_SIZE);
        let mut best_brancher_pattern : ResultPattern = Vec::with_capacity(0);



        loop {

            // reset the pattern stats, as now our brancher setup might look different!
            best_cost = std::u32::MAX;
            best_pattern = self.site_conf_factory.full(MAX_FIXED_SIZE);
            best_brancher_pattern  = Vec::with_capacity(0);




            'patternLoop : loop {
                scoped_tracepoint!(_cut_loop);


                if should_stop.load(Relaxed) {
                    break;
                }
                // optimize model

                {
                    scoped_tracepoint!(_bd_master_optimize);
                    benders_master.optimize().unwrap();
                }

                
                if benders_master.status().unwrap() != Status::Optimal {
                    panic!("{}", "Error in solving benders master!");
                }



                let result : Vec<bool> = benders_master.get_values(attr::X, &close_sites.iter().map(|(_idx,var)| var).cloned().collect::<Vec<Var>>())
                    .unwrap().iter().map(|el| *el > CG_EPSILON).collect();




                let set_of_closed_sites = result.iter().enumerate().filter(|(_idx,val)| **val).map(|(idx,_val)| idx).collect::<Vec<usize>>();


                // create pattern
                let mut current_pattern :SiteConf = self.site_conf_factory.empty();
                for (((_idx,el),result_val),site) in current_pattern.iter_mut()
                        .enumerate()
                        .zip(result.iter())
                        .zip(self.sites.iter().map(|(_,site)| site)) {
                    if ! result_val { // wenn nicht geschlossen
                        *el = u8::min(site.capacity,MAX_FIXED_SIZE)
                    }
                }


                // test pattern
                let pattern_cost : u32 = current_pattern.iter().zip(self.sites.iter().map(|(_,site)| site)).map(|(&size, site)| {
                    if size == 0 {
                        0_u32
                    } else {
                        u32::from(site.cost + site.charger_cost * size as u8)
                    }
                }).sum();

                {
                    scoped_tracepoint!(_bd_test_configuration);

                    let num_active =  self.scenarion_manager.num_active();
                    let quorum_required =  (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;

                    let results = self.scenarion_manager
                        .get_active_branchers()
                        .map(|(_idx,br)| {
                            br.replace_site_sizes(current_pattern.clone());
                            br.solve(false, false)
                    });

                    let mut oracle_ok  = 0;
                    let mut oracle_denied = 0;

                    for result in results {
                        if result.is_ok() {
                            println!("Said OK");
                            oracle_ok += 1;
                        } else {
                            println!("Said FAIL");
                            oracle_denied += 1;
                        }


                        if oracle_ok >= quorum_required {

                            // if it is ok we are done!
                            #[cfg(feature = "benders_debug")] {
                                println!("BEND|{:?}|{}|{}|{}|{:?}", &current_pattern, &pattern_cost, &best_cost,start_benders.elapsed().as_secs(), true);
                                println!("DONE!");
                            }


                            best_cost = pattern_cost;
                            best_pattern = current_pattern.clone();
                            //TODO: SET BEST BRANCHER PATTERN
                            best_brancher_pattern = Vec::new();
                            break 'patternLoop; // we have a good pattern, exit the outer loop

                        } else if num_active - oracle_denied < quorum_required { // quorum is not reachable anymore
                            break; // is not feasible, no need to search anymore so "continue" with cut generation
                        }

                    }
                }

                if set_of_closed_sites.len() == 0{
                    panic!("{}", "Opened all sites!");
                }


                #[cfg(feature = "benders_debug")]
                println!("BEND|{:?}|{}|{}|{}|{:?}", &current_pattern, &pattern_cost, &best_cost,start_benders.elapsed().as_secs(), false);



                // since we are infeasible try to generate cuts

                let mut potential_cuts : Vec<Vec<usize>> = Vec::new();
                #[cfg(feature = "pattern_generation_improve_cuts")] {

                    // we will here try to improve the cuts using a heuristic
                    // the allowed time budget is the same as the time spend
                    // in the benders_master problem (the harder the problem gets
                    // the more time do we spend in the heuristics

                   let runtime_benders_master = Duration::from_secs(benders_master.get(gurobi::attr::Runtime).unwrap().round() as u64);
                   let runtime_cut_loop = max(Duration::from_secs(5),runtime_benders_master);


                    // try to generate smaller cut
                    scoped_tracepoint!(_bd_lift_cuts);
                    let start_cut_loop = Instant::now();
                    let mut has_found_improvement = false;
                    let max_duration_without_improvement = Duration::from_secs(60 * 30);

                    while start_cut_loop.elapsed() < runtime_cut_loop {


                        if ! has_found_improvement && start_cut_loop.elapsed() > max_duration_without_improvement {
                            break;
                        }

                        let count_closed_sites = set_of_closed_sites.len();



                        let smaller_subset: Vec<usize> = if count_closed_sites > 1 {
                            let count = if count_closed_sites == 2 {
                                1 /* gen range does not like min = max */
                            } else {
                                self.rng.gen_range(1..(count_closed_sites - 1))
                            };
                            set_of_closed_sites.choose_multiple(&mut self.rng, count).cloned().collect()
                        } else {
                            // otherwise just cut single one
                            set_of_closed_sites.clone()
                        };



                        // sample again if we already tested this subset!
                        // set insert returns false if element was already in set
                        if ! tested_cuts.insert(smaller_subset.clone()) {
                            continue;
                        }






                        if ! self.subset_is_feasible(&smaller_subset) {
                            let mut found_better_level2 = false;
                            // further strengthen cut
                            if smaller_subset.len() > 1 {
                                for _i in 0..2 {
                                    let mut test_set : Vec<usize> = smaller_subset.choose_multiple(&mut self.rng, max(1, smaller_subset.len() - 2)).cloned().collect();
                                    while ! self.subset_is_feasible(&test_set) && test_set.len() > 1 {
                                        potential_cuts.push(test_set.clone());
                                        found_better_level2 = true;
                                        test_set = test_set.choose_multiple(&mut self.rng, max(1, test_set.len() - 2)).cloned().collect();
                                    }
                                }
                            }

                            if ! found_better_level2 {
                                potential_cuts.push(smaller_subset);
                            }

                            has_found_improvement = true;
                        }



                        if count_closed_sites == 1 { // no need to sample multiple if size is already at 1
                            break
                        }
                    }
                }






                // add cuts to master
                if potential_cuts.is_empty() {
                    potential_cuts.push(set_of_closed_sites);
                }
                potential_cuts.sort_by(|a,b| a.len().cmp(&b.len()));
                'nextPotentialCut: for set_of_closed_sites in potential_cuts.iter().take(50) {

                    if set_of_closed_sites.is_empty() {
                        continue 'nextPotentialCut;
                    }


                    // do dominance management
                    let mut do_skip_this_cut = false;

                    active_cuts.retain(|(cut_pattern,cut_constr)| {
                        if cut_pattern.len() > set_of_closed_sites.len() {
                            // if the existing is larger, maybe new dominates old?
                            if set_of_closed_sites.iter().all(|v| cut_pattern.contains(v)) {
                                // new dominates old ! remove old
                                benders_master.remove(cut_constr.clone());
                                #[cfg(feature = "pattern_generation_debug")]
                                println!("Removed cut from master");
                                return false;
                            }

                        } else {
                            // if existing is smaller or equal, then maybe it dominates it?
                            // if equal, then must be different!

                            if cut_pattern.iter().all(|v| set_of_closed_sites.contains(v)) {
                                // old dominates new, do not add!
                                do_skip_this_cut = true;
                                return true
                            }
                        }
                        true
                    });

                    if do_skip_this_cut  {
                        continue 'nextPotentialCut;
                    }

                    num_cuts += 1;

                    #[cfg(feature = "pattern_generation_debug")]
                    println!("Adding cut that not all of {:?} can be set, total : {}",set_of_closed_sites, active_cuts.len());
                    let constr = benders_master.add_constr(&format!("benderCut[{}]", num_cuts),
                                            set_of_closed_sites.iter().map(|idx| close_sites.get_index(*idx).unwrap()).fold(LinExpr::new(), |a, b| a + b.1)
                                            , Less, (set_of_closed_sites.len() - 1) as f64).unwrap();

                    active_cuts.push((set_of_closed_sites.clone(),constr))

                };
            }


            println!("Best cost {} with pattern {:?}", &best_cost,&best_pattern);
            println!("# {} open sites", &best_pattern.iter().filter(|i| **i > 0).count());




            // test the solution for feasibility over the full data set, then
            // update the active brancher set, then continue the loop
            // if everything is ok, we can break.



            let mut infeasible_scenarios = Vec::new();


            for (bidx,b) in self.scenarion_manager.get_all_branchers() {
                b.replace_site_sizes(best_pattern.clone());
                match  b.solve(false, true) {
                    Ok(_) => {
                        println!("Feasible for {:?}",best_pattern.clone())
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

                            println!("Infeasible count = {} (+{})", infs.len(),external_infeasibility_penalty);
                            infeasible_scenarios.push((infs,b.get_vehicles().len(),bidx));
                        } else {
                            println!("Benevolent Feasible ( inf count = {})", infs.len());
                        }

                    },
                    Err(SolveError::Generic(msg)) =>  panic!("{}", msg),
                    Err(SolveError::StoppedByExternal) =>  panic!("{}", "InvalidError"),
                    Err(SolveError::NoQuickIntegerResult) => panic!("{}", "InvalidError"),
                    Err(SolveError::NoQuickResult) => panic!("{}", "InvalidError")
                }
            }

            let num_active =  self.scenarion_manager.num_active();
            let quorum_required =  (num_active as f32 * (self.quorum_accept_percent as f32 / 100.0)).round() as usize;
            let infeasible_allowed_through_quorum =  num_active - quorum_required;

            if infeasible_scenarios.is_empty() || infeasible_scenarios.len() <= infeasible_allowed_through_quorum {
                break;
            }

            infeasible_scenarios.sort_by_key(|(inf,_,_)| inf.len());

            self.scenarion_manager.new_generation();
            for (inf,num_vehicles,idx) in infeasible_scenarios.into_iter().take(self.max_activate_per_generation) {



                let num_vehicles_base_for_benevolent = if self.total_num_vehicles > 0 {
                    self.total_num_vehicles as usize
                } else {
                    num_vehicles
                };


                let external_infeasibility_penalty = if self.total_num_vehicles > 0 {
                    assert!(self.total_num_vehicles > num_vehicles  as i64);
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
                    let num_inf_above_level = (inf.len()+external_infeasibility_penalty)  - benevolent_accept_limit_count;
                    let reduced_inf_list : HashSet<usize> = inf.into_iter().take(num_inf_above_level).collect();

                    // get brancher
                    let brancher_vehicles = self.scenarion_manager.branchers[idx].get_vehicles();

                    let  active_map : Vec<bool> = brancher_vehicles.iter().map(|v| {
                        reduced_inf_list.contains(&v.index)
                    }).collect();

                    println!("Activating brancher with this list {:?}",&active_map);

                    let new_brancher = Brancher::new_with_active_map(
                        self.site_array.clone(),
                        brancher_vehicles,
                        active_map,
                        self.site_conf_factory.empty(),
                        self.allowed_infeasible,
                        should_stop.clone()
                    );


                    self.scenarion_manager.add_brancher_and_activate(new_brancher);

                } else {



                        println!("Activating {}", idx);
                        self.scenarion_manager.activate(idx);



                }
            }

        }


        let open_sites = self.sites.iter().map(|(_id, site)| {
            (best_pattern[site.index],site.index)
        } ).collect();


        // write charge processes to file
        {
            let write_file = File::create(path_charge_process).unwrap();
            let mut writer = BufWriter::new(&write_file);

            for (vehicle, patterns) in best_brancher_pattern {
                for (_segment, site, time) in patterns {
                    write!(&mut writer,"{},{},{}\n", vehicle.id, site.id, time).unwrap();
                }
            }
        }

        // write cuts to file
        {
            let write_file = File::create(cut_file_output).unwrap();
            let mut writer = BufWriter::new(&write_file);
            write!(&mut writer,"{}\n", best_cost).unwrap();
            for (cut, _) in active_cuts {
                    write!(&mut writer,"{}\n", cut.iter().map(|e| format!("{}:0",e)).join(",")).unwrap();
            }
        }

        Simple {

            cost : u64::from(best_cost),
            sites_open : open_sites,


        }
    }
}
