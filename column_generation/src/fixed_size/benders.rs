use std::sync::Arc;
use std::sync::atomic::AtomicBool;


use indexmap::IndexMap;
use itertools::Itertools;
use shared::{Segment, Simple, Site, Vehicle};


use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use crate::fixed_size::brancher::{Brancher, SolveError};

use crate::{SiteArray, CG_EPSILON};

use crate::fixed_size::brancher::ResultPattern;

use rust_hawktracer::*;

use rand::prelude::{StdRng, SliceRandom};
use rand::{SeedableRng, Rng};
use gurobi;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use std::collections::HashSet;
use std::time::{Instant, Duration};
use std::sync::atomic::Ordering::Relaxed;
use std::io;
use std::io::BufRead;
use std::cmp::max;

pub struct Benders <'a> {
    rng : StdRng,
    min_num_sites: usize,
    site_array : Vec<Site>,
    site_conf_factory : SiteConfFactory,
    best_cost : u32,
    best_pattern : SiteConf,
    best_brancher_pattern : Option<ResultPattern<'a>>,
    tested_cuts : HashSet<Vec<usize>>,
    operational_problem : Brancher<'a>,
    should_stop : Arc<AtomicBool>
}


const MAX_FIXED_SIZE : u8 = 4;

#[derive(PartialEq)]
enum SubsetFeasibility {
    FEASIBLE,
    UNFEASIBLE,
    UNKNOWN
}


impl<'a> Benders<'a> {

    pub fn new(min_num_sites: usize, allowed_infeasible : usize, sites: IndexMap<u8, Site>, vehicles : &'a [Vehicle<'a>]) -> Self {



        let site_array : Vec<Site> = sites.iter().map(|(_i,site)| site.clone()).collect();

        let site_conf_factory = SiteConfFactory {
            num_sites: site_array.len()
        };

        let should_stop = Arc::new(AtomicBool::new(false));
        let operational_problem = Brancher::new(
            site_array.clone(),
            &vehicles,
            site_conf_factory.empty(),
            allowed_infeasible,
            should_stop.clone()

        );


        Benders {
            min_num_sites,
            site_array,
            best_pattern : site_conf_factory.full(MAX_FIXED_SIZE),
            site_conf_factory : site_conf_factory,
            best_cost : u32::MAX,
            best_brancher_pattern : None,
            rng : StdRng::seed_from_u64(12345),
            tested_cuts : HashSet::new(),
            operational_problem,
            should_stop
        }
    }

    pub fn run(&mut self, should_stop: Arc<AtomicBool>, path_charge_process : &str, cut_file_output: &str, cut_file_input: &str) -> Simple {


        let start_benders = Instant::now();


        let mut env = gurobi::Env::new("/tmp/gurobi_benders.log").unwrap();
        env.set(gurobi::param::LogToConsole, 0).unwrap();
        env.set(gurobi::param::Threads, 1).unwrap();
        env.set(gurobi::param::Seed, 12345).unwrap();

        #[cfg(not(feature = "column_generation_debug"))]
            env.set(gurobi::param::OutputFlag, 0).unwrap();

        // create an empty model which associated with `env`:
        let mut benders_master = gurobi::Model::new("benders_master", &env).unwrap();
        benders_master.set(gurobi::attr::ModelSense,gurobi::ModelSense::Minimize.into()).unwrap();

        let mut close_sites : IndexMap<usize,gurobi::Var> = IndexMap::default();

        for site in &self.site_array {
            close_sites.insert(site.index,
                benders_master.add_var(&format!("closeSite[{}]", site.index), gurobi::Binary, -1.0 * f64::from(site.cost + MAX_FIXED_SIZE * site.charger_cost), 0.0, 1.0, &[], &[]).unwrap()
            );
        }
        //benders_master.add_constr(&"fixedNumSites",close_sites.iter().map(|(_idx,site)| site).fold(LinExpr::new(),|a,b| a + b), Less, (sites.len() - self.num_sites) as f64).unwrap();


        let mut active_cuts : Vec<(Vec<usize>,gurobi::Constr)> = Vec::with_capacity(1000);


        let mut num_cuts : usize = 0;


        // Load cuts from existing cut file
        {
            // copy cuts from first level
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
                                                               sites_in_cut.iter().map(|idx| close_sites.get_index(*idx).unwrap()).fold(gurobi::LinExpr::new(), |a, b| a + b.1)
                                                               , gurobi::Less, (sites_in_cut.len() - 1) as f64).unwrap();

                        active_cuts.push((sites_in_cut.clone(), constr));
                        self.tested_cuts.insert(sites_in_cut);
                        num_cuts += 1;
                    }
                }
                println!("Loaded {} cuts from external file", num_cuts);
            }
        }


        loop {
            scoped_tracepoint!(_cut_loop);

            if should_stop.load(Relaxed) {
                break;
            }

            {
                scoped_tracepoint!(_bd_master_optimize);
                benders_master.optimize().unwrap();
            }

            
            if benders_master.status().unwrap() != gurobi::Status::Optimal {
                panic!("{}", "Error in solving benders master!");
            }


            let result_site_vars: Vec<bool> = benders_master.get_values(gurobi::attr::X, &close_sites.iter().map(|(_idx,var)| var).cloned().collect::<Vec<gurobi::Var>>())
                .unwrap().iter().map(|el| *el > CG_EPSILON).collect();


          
            let set_of_closed_sites = result_site_vars.iter().enumerate().filter(|(_idx,val)| **val).map(|(idx,_val)| idx).collect::<Vec<usize>>();

            #[cfg(feature = "benders_lifting_debug")]
            println!("LFT|BEND_OPT_COVER_CANDIDATE|{:?}",set_of_closed_sites);
            


            // create pattern
            let mut current_pattern :SiteConf = self.site_conf_factory.empty();
            for (((_idx,el),result_val),site) in current_pattern.iter_mut().enumerate().zip(result_site_vars.iter()).zip(self.site_array.iter()) {
                if ! result_val { // wenn nicht geschlossen
                    *el = u8::min(site.capacity,MAX_FIXED_SIZE)
                }
            }
            // calculate pattern cost

            let pattern_cost = self.get_pattern_cost(&current_pattern);
            {
                scoped_tracepoint!(_bd_test_configuration);
                if let Ok((_local_travel_cost, brancher_patterns)) = self.evaluate_configuration(&current_pattern, false,false) {
                    // if it is ok we are done!
                    #[cfg(feature = "benders_debug")] {
                        println!("BEND|{:?}|{}|{}|{}|{:?}", current_pattern, pattern_cost, self.best_cost,start_benders.elapsed().as_secs(), true);
                        println!("DONE!");
                    }

                    #[cfg(feature = "benders_lifting_debug")]
                    println!("LFT|BEND_OPT_NOT_COVER");
                    self.record_feasible_solution(current_pattern, brancher_patterns);
                    break;

                } else {
                    #[cfg(feature = "benders_lifting_debug")]
                    println!("LFT|BEND_OPT_IS_COVER");
                }
            }

            if set_of_closed_sites.len() == 0{
                panic!("{}", "Opened all sites!");
            }


            #[cfg(feature = "benders_debug")]
            println!("BEND|{:?}|{}|{}|{}|{:?}", current_pattern, pattern_cost, self.best_cost,start_benders.elapsed().as_secs(), false);


            // since we are infeasible try to generate cuts
            let mut potential_cuts = self.improve_cuts(&set_of_closed_sites, Duration::from_secs(benders_master.get(gurobi::attr::Runtime).unwrap().round() as u64));


            // add cuts to master
            if potential_cuts.is_empty() {
                potential_cuts.push(set_of_closed_sites);
            }
            potential_cuts.sort_by(|a,b| a.len().cmp(&b.len()));

            'nextPotentialCut: for set_of_closed_sites in potential_cuts.iter() {

                if set_of_closed_sites.is_empty() {
                    continue 'nextPotentialCut;
                }
                // do dominance management on cuts
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

                #[cfg(feature = "benders_lifting_debug")]
                println!("LFT|IMPROVED_COVER|{:?}",set_of_closed_sites);

                num_cuts += 1;

                #[cfg(feature = "pattern_generation_debug")]
                println!("Adding cut that not all of {:?} can be set, total : {}",set_of_closed_sites, active_cuts.len());
                let constr = benders_master.add_constr(&format!("benderCut[{}]", num_cuts),
                                          set_of_closed_sites.iter().map(|idx| close_sites.get_index(*idx).unwrap()).fold(gurobi::LinExpr::new(), |a, b| a + b.1)
                                          , gurobi::Less, (set_of_closed_sites.len() - 1) as f64).unwrap();
                active_cuts.push((set_of_closed_sites.clone(),constr));


                /* // NO_GOOD_CUT, Eine der variablen muss sich ändern. Oberer cut aber stärker
                let set_of_open_sites = site_array.iter().filter_map(|s| {
                    if set_of_closed_sites.contains(&s.index) {
                        Some(s.index)
                    }  else {
                        None
                    }
                });

                benders_master.add_constr(&format!("benderCut2[{}]", num_cuts),
                                                       set_of_closed_sites.iter().map(|idx| close_sites.get_index(*idx).unwrap()).fold(LinExpr::new(), |a, b| a + (1.0-b.1))
                                                       + set_of_open_sites.map(|idx| close_sites.get_index(idx).unwrap()).fold(LinExpr::new(), |a, b| a + b.1)
                                                       , Greater, 1.0).unwrap();

                */


            };
        }



        println!("Best cost {} with pattern {:?}", self.best_cost,self.best_pattern);
        println!("# {} open sites", self.best_pattern.iter().filter(|i| **i > 0).count());



        let open_sites = self.site_array.iter().map(|site| {
            (self.best_pattern[site.index],site.index)
        } ).collect();


        // write charge processes to file
        {
            let write_file = File::create(path_charge_process).unwrap();
            let mut writer = BufWriter::new(&write_file);

            for (vehicle, patterns) in self.best_brancher_pattern.as_ref().unwrap() {
                for (_segment, site, time) in patterns {
                    write!(&mut writer,"{},{},{}\n", vehicle.id, site.id, time).unwrap();
                }
            }
        }

        // write cuts to file
        {
            let write_file = File::create(cut_file_output).unwrap();
            let mut writer = BufWriter::new(&write_file);
            write!(&mut writer,"{}\n", self.best_cost).unwrap();
            for (cut, _) in active_cuts {
                    write!(&mut writer,"{}\n", cut.iter().map(|e| format!("{}:0",e)).join(",")).unwrap();
            }
        }

        println!("Total Number of Columns: {}", self.operational_problem.get_num_colums());


        Simple {

            cost : u64::from(self.best_cost),
            sites_open : open_sites,


        }
    }


    fn get_pattern_cost(&self, pattern : &SiteConf) -> u32 {
        pattern.iter().zip(self.site_array.iter()).map(|(&size,site)| {
            if size == 0 {
                0_u32
            } else {
                u32::from(site.cost + site.charger_cost * size as u8)
            }
        }).sum()
    }

    fn record_feasible_solution(&mut self, current_pattern : SiteConf, brancher_pattern : ResultPattern<'a>) {
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


    fn evaluate_configuration(&mut self, config : &SiteConf, find_quick_result_or_exit : bool, find_num_infeasible : bool) -> Result<(f64, ResultPattern<'a>),SolveError> {

        #[cfg(feature = "pattern_generation_debug")]
        println!("CUT|{:?}", current_pattern);
        self.operational_problem.replace_site_sizes(config.clone());
        self.operational_problem.solve(find_quick_result_or_exit, find_num_infeasible)
    }

    fn subset_is_feasible(&mut self, smaller_subset : &[usize]) -> SubsetFeasibility {
            let mut current_pattern: SiteConf = self.site_conf_factory.empty();
            for (idx, el) in current_pattern.iter_mut().enumerate() {
                if !smaller_subset.contains(&idx) {
                    *el = u8::min(self.site_array[idx].capacity,MAX_FIXED_SIZE)
                }
            }

            let res = match self.evaluate_configuration( &current_pattern,true, false) {
                Ok((_local_travel_cost, brancher_patterns)) => {
                    self.record_feasible_solution(current_pattern, brancher_patterns);
                    SubsetFeasibility::FEASIBLE
                },
                Err(SolveError::VehiclesInfeasible(_)) => SubsetFeasibility::UNFEASIBLE,
                Err(SolveError::Generic(_)) => SubsetFeasibility::UNFEASIBLE,
                Err(SolveError::StoppedByExternal) => SubsetFeasibility::UNKNOWN,
                Err(SolveError::NoQuickIntegerResult) => SubsetFeasibility::UNKNOWN,
                Err(SolveError::NoQuickResult) => SubsetFeasibility::UNKNOWN
            };


            #[cfg(feature = "benders_lifting_debug")]
                {
                    match res {
                        SubsetFeasibility::FEASIBLE => println!("LFT|TEST_RES_SUBSET|{:?}|{}",smaller_subset,"FEASIBLE"),
                        SubsetFeasibility::UNFEASIBLE => println!("LFT|TEST_RES_SUBSET|{:?}|{}",smaller_subset,"INFEASIBLE"),
                        SubsetFeasibility::UNKNOWN => println!("LFT|TEST_RES_SUBSET|{:?}|{}",smaller_subset,"UNKNOWN")
                    }
                }


            return res;
    }

    fn improve_cuts(&mut self,
                    set_of_closed_sites : &[usize],
                    runtime_benders_master : Duration
    ) -> Vec<Vec<usize>> {

        let mut potential_cuts : Vec<Vec<usize>> = Vec::new();

        #[cfg(feature = "pattern_generation_improve_cuts")] {

            // we will here try to improve the cuts using a heuristic
            // the allowed time budget is the same as the time spend
            // in the benders_master problem (the harder the problem gets
            // the more time do we spend in the heuristics

           // let runtime_benders_master = Duration::from_secs(benders_master.get(gurobi::attr::Runtime).unwrap().round() as u64);
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

                let mut smaller_subset: Vec<usize> = Vec::from(set_of_closed_sites.clone());
                while smaller_subset.len() > 1 {


                    let count = if smaller_subset.len() == 2 {
                        1 /* gen range does not like min = max */
                    } else {
                        self.rng.gen_range(1..(smaller_subset.len() - 1))
                    };
                    smaller_subset = smaller_subset.choose_multiple(&mut self.rng, count).cloned().collect();

                    // sample again if we already tested this subset!
                    // set insert returns false if element was already in set
                    if ! self.tested_cuts.insert(smaller_subset.clone()) {
                        continue;
                    }


                    match self.subset_is_feasible(&smaller_subset) {
                        SubsetFeasibility::UNFEASIBLE => {
                            has_found_improvement = true;
                            potential_cuts.push(smaller_subset.clone());
                        }
                        SubsetFeasibility::FEASIBLE => {
                            break
                        }
                        SubsetFeasibility::UNKNOWN => {}
                    }
                }
            }
        }


        return  potential_cuts;
    }
}
