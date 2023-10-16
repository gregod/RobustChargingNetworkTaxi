#![allow(dead_code,unused)]

pub const DUMMY_COST : f64 = 1.0;
use rust_hawktracer::*;

use gurobi::{Status,param, Model, Env, attr,Integer, Continuous,INFINITY, LinExpr, Var, Constr, Less, Greater, Equal};

use std::io::{Read, BufWriter};
use std::rc::Rc;
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Write;

macro_rules! pause {
    ($x:expr) => {
        println!($x);
        io::stdin().read(&mut [0u8]);
    };
}


use shared::{Segment, Vehicle, Site, Period, CustomMultiHashMap, CustomHashMap, MAX_PERIOD, charge_time_to_capacity_charge_time, ReachableSite};
use crate::{SiteArrayRef, CG_EPSILON, format_pattern};
use indexmap::IndexMap;
use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use crate::pattern_pool::PatternPool;
use ndarray::Array2;
use crate::dag_builder::{build_dag, generate_patterns, NodeWeight, EdgeWeight};
use std::io;
use petgraph::graph::NodeIndex;
use petgraph::{Graph, Directed};

#[cfg(feature = "perf_statistics")]
use crate::metrics::*;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::error::Error;
use crate::fixed_size::brancher::SolveError::{Generic, NoQuickResult, VehiclesInfeasible};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::cmp::Ordering;
use std::sync::atomic::Ordering::Relaxed;
use crate::branching_filter::BranchingFilter;
use snowflake::ProcessUniqueId;
use petgraph::dot::Dot;
use std::path::Path;
use std::fs::File;


type SinglePattern<'a> = Vec<(&'a Segment<'a>, &'a Site, Period)>;
pub type ResultPattern<'a> =  Vec<(&'a Vehicle<'a>, SinglePattern<'a>)>;
type PatternSelected = f64;



#[repr(usize)]
#[derive(Clone,Debug)]
pub enum BranchPriority {
    Default = 1,
}

#[derive(Clone,PartialEq)]
pub enum BranchMeta {
    Default,
    Approx,
    OnlyInteger
}

impl<'a> Debug for BranchMeta {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            BranchMeta::Default => {
                f.write_str(&"Default")
            },
            BranchMeta::Approx => {
                f.write_str(&"Approx")
            }
            BranchMeta::OnlyInteger => {
                f.write_str(&"OnlyInteger")
            }

        }
    }
}


#[derive(Debug, Clone)]
pub struct BranchNode<'a> {
    pub id : ProcessUniqueId,
    pub parent_id : ProcessUniqueId,
    pub parent_approx : bool,
    pub parent_objective : f64,
    pub branch_priority: BranchPriority,
    pub filters : Vec<BranchingFilter<'a>>,
    pub meta : BranchMeta
}



impl Eq for BranchNode<'_> {}
impl PartialEq for BranchNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.parent_objective == other.parent_objective
    }
}

impl PartialOrd for BranchNode<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.parent_objective.partial_cmp(&self.parent_objective)
    }
}

impl Ord for BranchNode<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}


impl <'a> BranchNode<'a> {

    pub fn root() -> Self {
        let root_id = ProcessUniqueId::new();
        BranchNode {
            id : root_id,
            parent_id : root_id,
            filters : Vec::new(),
            parent_objective : 0.0,
            parent_approx : false,
            branch_priority: BranchPriority::Default,
            meta :  BranchMeta::Default
        }
    }

    pub fn integer_from_parent( parent : &BranchNode<'a>) -> Self {
        BranchNode {
            id : ProcessUniqueId::new(),
            parent_objective : parent.parent_objective ,
            parent_approx : parent.parent_approx,
            parent_id : parent.id,
            filters : parent.filters.clone(),
            branch_priority: parent.branch_priority.clone(),
            meta : BranchMeta::OnlyInteger
        }
    }



    pub fn from_parent(parent : &BranchNode<'a>, parent_objective : f64, parent_approx : bool, new_filter : BranchingFilter<'a>, branch_type : BranchPriority) -> Self {
        let mut filters = parent.filters.to_owned();
        filters.push(new_filter);
        BranchNode {
            id : ProcessUniqueId::new(),
            parent_id : parent.id,
            parent_approx,
            parent_objective,
            filters,
            branch_priority: branch_type,
            meta :  BranchMeta::Default
        }
    }


    pub fn from_parent_to_exact(parent : &BranchNode<'a>,parent_objective : f64,parent_approx : bool) -> Self {
        let mut filters = parent.filters.to_owned();
        BranchNode {
            id : ProcessUniqueId::new(),
            parent_id : parent.id,
            parent_objective,
            parent_approx,
            filters,
            branch_priority: BranchPriority::Default,
            meta : BranchMeta::Default
        }
    }

    pub fn from_parent_multifilters(parent : &BranchNode<'a>, parent_objective : f64,parent_approx : bool, new_filters : Vec<BranchingFilter<'a>>, branch_type : BranchPriority) -> Self {
        let mut filters = parent.filters.to_owned();
        filters.extend(new_filters);

        BranchNode {
            id : ProcessUniqueId::new(),
            parent_id : parent.id,
            parent_objective,
            parent_approx,
            filters,
            branch_priority: branch_type,
            meta :  BranchMeta::Default
        }
    }
}

pub struct SolvedCGResult<'a> {
    master_x : f64,
    patterns : Vec<(&'a Vehicle<'a>, Vec<(SinglePattern<'a>, PatternSelected)>)>
}



pub struct Brancher<'a> {
    sites:  Vec<Site>,
    vehicles: &'a [Vehicle<'a>],
    active_map : Vec<bool>,
    allowed_infeasible : usize,
    site_sizes:  SiteConf,
    current_upper_bound: Option<f64>,
    current_best_pattern: Option<ResultPattern<'a>>,
    pattern_pool: PatternPool<'a>,
    open_branches : VecDeque<BranchNode<'a>>,
    env : Env,
    should_stop : Arc<AtomicBool>,
    invisibility_event_counter : usize
}

#[derive(Debug,Clone)]
pub enum SolveError {
    NoQuickIntegerResult,
    NoQuickResult,
    VehiclesInfeasible(Vec<usize>),
    Generic(&'static str),
    StoppedByExternal
}



impl<'a> Brancher<'a> {

    pub fn new(sites:Vec<Site>,
               vehicles: &'a [Vehicle<'a>],
               site_sizes: SiteConf,
               allowed_infeasible : usize,
               should_stop : Arc<AtomicBool>) -> Brancher<'a> {

        let active_map = vec![true;vehicles.len()];

        Brancher::new_with_active_map(
            sites,vehicles,active_map,site_sizes,allowed_infeasible,should_stop
        )

    }

    pub fn get_inactive_count(&self) -> usize {
        self.active_map.iter().filter(|v| **v == false).count()
    }

    pub fn new_with_active_map(sites: Vec<Site>,
               vehicles: &'a [Vehicle<'a>],
               active_map : Vec<bool>,
               site_sizes: SiteConf,
               allowed_infeasible : usize,
               should_stop : Arc<AtomicBool>) -> Brancher<'a> {

        let mut env = Env::new("/tmp/gurobi.log").unwrap();

        #[cfg(not(feature = "column_generation_debug"))]
            env.set(param::LogToConsole, 0).unwrap();

        #[cfg(not(feature = "column_generation_debug"))]
            env.set(param::OutputFlag, 0).unwrap();

        env.set(param::Threads, 1).unwrap();
        env.set(param::Seed, 12345).unwrap();
        // set low time limit; we mainly want the integer solution by branching this function is only for quick wins;
        //env.set(param::TimeLimit, 20.0).unwrap();

        Brancher {
            sites,
            vehicles,
            active_map,
            site_sizes,
            current_upper_bound: None,
            current_best_pattern: None,
            pattern_pool: PatternPool::new(vehicles.len()),
            open_branches : VecDeque::new(),
            env,
            allowed_infeasible,
            should_stop,
            invisibility_event_counter : 0
        }
    }

    pub fn get_num_colums(&self) -> usize {
        self.pattern_pool.num_columns()
    }

    pub fn replace_site_sizes(&mut self, site_sizes : SiteConf) {
        self.site_sizes = site_sizes;
    }

    pub fn get_site_sizes(&self)  -> SiteConf {
        self.site_sizes.clone()
    }

    pub fn get_site_size(&self, site : &Site) -> u8 {
        self.site_sizes[site.index]
    }
    pub fn update_site(&mut self, site : &Site, new_size : u8 ) {
        self.site_sizes[site.index] = new_size;
    }

    pub fn get_vehicles(&self) -> &'a [Vehicle<'a>] {
        self.vehicles
    }

    fn get_active_vehicles(&self) ->  impl Iterator<Item = &'a Vehicle<'a>> {
        self.vehicles.iter().zip(self.active_map.clone()).filter_map( |(v,a)| {
            if a {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn process_branch_node(&mut self, node : &BranchNode<'a>,find_num_infeasible : bool) -> Result<SolvedCGResult<'a>,SolveError>  {
        match node.meta {
            BranchMeta::Default  => {
                self.solve_relaxed_problem(&node.filters,find_num_infeasible)
            },
            BranchMeta::Approx => {
                self.solve_relaxed_problem(&node.filters,find_num_infeasible)
            },
            BranchMeta::OnlyInteger => {
                self.solve_integer_problem(&node.filters,find_num_infeasible)
            }
        }
    }

    #[hawktracer(solve_pattern)]
    pub fn solve(&mut self, find_quick_result_or_exit: bool, find_num_infeasible : bool) -> Result<(f64, ResultPattern<'a>),SolveError> {
        if self.should_stop.load(Relaxed) {
            return Err(SolveError::StoppedByExternal);
        }


        #[cfg(feature = "perf_statistics")]
        let timer = ConfigurationTimer::new();


        #[cfg(feature = "column_generation_debug")]
        println!("Solving first Problem ! (Has {} patterns;)", self.pattern_pool.num_columns());


        /* reset bounds */
        self.current_best_pattern = None;
        self.current_upper_bound = None;

        /* reset open branches */
        self.open_branches = VecDeque::new();
        let root_node = BranchNode::root();

        #[cfg(feature = "column_generation_sometimes_integer")] {
            if self.pattern_pool.num_columns() > 0 {
                self.open_branches.push_back(BranchNode::integer_from_parent(&root_node))
            }
        }

        self.open_branches.push_front(root_node);
        let mut last_error = Generic("No Upper Bound FOUND!");
        loop {


            let onode = self.open_branches.pop_back();
            if let Some(node) = onode {

                if find_quick_result_or_exit && node.filters.len() > 25 {
                    return Err(NoQuickResult)
                }


                #[cfg(feature = "branching_debug")]
                println!("CHILD\t{}\t{}\t",node.id,node.parent_id);

                // is the node outperformed by bound?
                if let Some(bound) = self.current_upper_bound {
                    if !node.parent_approx && node.parent_objective > bound {
                        continue
                    }
                }
                let is_approx = node.meta == BranchMeta::Approx;
                // process the node and get result
                match self.process_branch_node(&node,find_num_infeasible) {

                 Ok(result) =>  {

                    #[cfg(feature = "branching_debug")]
                    println!("NODE\t{}\t{}\t{}\t{}",node.id,node.parent_id,node.parent_objective,result.master_x);

                    // now check for possible branching points!
                    if let Some(branches) = self.check_result_for_branching_points(&node.filters, &result) {
                        // pick branching point
                        // initially pick first!

                        let (a_branch,b_branch) = &branches[0];

                        let a_node = BranchNode::from_parent(&node, result.master_x.clone(), is_approx, a_branch.clone(), BranchPriority::Default);
                        let b_node = BranchNode::from_parent(&node, result.master_x.clone(), is_approx, b_branch.clone(), BranchPriority::Default);

                        self.open_branches.push_back(a_node);
                        self.open_branches.push_back(b_node);

                        #[cfg(feature = "column_generation_sometimes_integer")] {
                            // every 10 constraints (and first) attempt to solve using integer heuristic
                            if node.filters.len() % 10 == 0 {
                                self.open_branches.push_back(
                                    BranchNode::integer_from_parent(&node)
                                );
                            }
                        }

                        continue;

                    } else {
                        // no branching points => is feasible
                        if let Some(ub) = self.current_upper_bound {
                            if result.master_x <= ub {
                                self.current_upper_bound = Some(result.master_x);
                                self.current_best_pattern = Some(result.patterns.iter().map(|(vehicle, patterns)| {

                                    debug_assert!(
                                        {
                                            // have either zero or one patterns selected
                                            let tmp = patterns.iter().filter(|(_, value)| *value > 0.0).count();
                                            tmp == 1 || tmp == 0
                                        }
                                    );

                                    if let Some(active_patterns) = patterns.iter().find(|(_, value)| *value > 0.0) {
                                        (*vehicle, active_patterns.0.clone())
                                    } else {
                                        (*vehicle,Vec::with_capacity(0))
                                    }

                                }).collect());

                                // if we have enough feasible vehicles exit branching early
                                #[cfg(feature = "column_generation_exit_early")] {
                                    if result.patterns.iter()
                                        .map(|(_, patterns)| patterns.iter()
                                            .filter(|(_, value)| *value > 0.0)
                                            .count()
                                        ).sum::<usize>() >= self.get_active_vehicles().count() - self.allowed_infeasible {
                                        break
                                    }
                                }


                            }



                        }  else {
                            self.current_upper_bound = Some(result.master_x);
                            self.current_best_pattern = Some(result.patterns.iter().map(|(vehicle, patterns)| {
                                if let Some(active_patterns) = patterns.iter().find(|(_, value)| *value > 0.0) {
                                    (*vehicle, active_patterns.0.clone())
                                } else {
                                    (*vehicle,Vec::with_capacity(0))
                                }
                            }).collect());

                        }

                        continue

                    }
                }
                    Err(e) => {
                        if node.meta == BranchMeta::Default {
                            last_error = e;
                        }
                        continue;
                    }
                }

            } else {
                break
            }
        }





        if let Some(upper_bound) = self.current_upper_bound {
            let best_pattern = self.current_best_pattern.clone().unwrap().clone();
            Ok((upper_bound, best_pattern))
        } else {
            Err(last_error)
        }
    }

    #[hawktracer(solve_integer_problem)]
    fn solve_integer_problem(&mut self, charge_filters: &[BranchingFilter<'a>], find_num_infeasible : bool) -> Result<SolvedCGResult<'a>, SolveError> {

            if self.should_stop.load(Relaxed) {
                return Err(SolveError::StoppedByExternal);
            }

            #[cfg(feature = "perf_statistics")]
            LPS_SOLVED.mark();
            // integer problem is only solved as kind of heuristic on the root node, thus only give limited amount of time.
            self.env.set(param::TimeLimit, 60.0).unwrap();

            // create an empty model which associated with `env`:
            let mut integer_master = Model::new("master", &self.env).unwrap();

            // initialize hashmaps for model variables and patterns
            let mut vehicle_patterns = CustomMultiHashMap::default();
            let mut vehicle_convexity: IndexMap<&Vehicle, Constr> = IndexMap::default();



            let mut site_constraints = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
            for site in &self.sites {
                for p in 0..MAX_PERIOD {
                    site_constraints.push(integer_master.add_constr(&format!("maxCapacity[{},{},{}]", site.id, site.index, p), LinExpr::new(), Less, f64::from(self.site_sizes[site.index])/*site.capacity.into()*/).unwrap());
                }
            }
            let constr_max_capacity: Array2<Constr> = Array2::from_shape_vec((self.sites.len(), MAX_PERIOD), site_constraints).unwrap();






            let mut pattern_counter = 0;
            let mut all_dummy_vars = Vec::with_capacity(self.get_active_vehicles().count());
            for vehicle in self.get_active_vehicles() {
                let dummy_var = integer_master.add_var(&format!("dummy_column[{}]", vehicle.id), Continuous, DUMMY_COST, 0.0, 1.0, &[], &[]).unwrap();
                let constr_convexity = integer_master.add_constr(&format!("convexity[{}]", vehicle.id), 1.0 * &dummy_var, Equal, 1.0).unwrap();
                all_dummy_vars.push(dummy_var);

                vehicle_convexity.insert(vehicle, constr_convexity.clone());

                // add the existing patterns from the pool
                // copy paste from code in column generation below
                {
                    let patterns = self.pattern_pool.get_active_patterns(vehicle, charge_filters.to_vec(), self.site_sizes.clone());

                    for entry in patterns {
                        let mut const_vec: Vec<Constr> = Vec::with_capacity(entry.pattern.len()+1);
                        let mut val_vec= Vec::with_capacity(entry.pattern.len()+1);

                        // make column use one unit of convexity constraint
                        const_vec.push(constr_convexity.clone());
                        val_vec.push(1.0);


                        // make column use one unit of capacity at every used site
                        for (_, site, period) in entry.pattern.iter() {
                            const_vec.push(constr_max_capacity[[site.index, charge_time_to_capacity_charge_time(period)]].clone());
                            val_vec.push(1.0);
                        }



                        let var_use_pattern = integer_master.add_var(&format!("usePattern[{}]", pattern_counter), Integer, 0.0, 0.0, 1.0, &const_vec, &val_vec).unwrap();
                        vehicle_patterns.insert(vehicle, (var_use_pattern, entry.pattern.clone()));
                        pattern_counter += 1;
                    }
                }

            }

            if ! find_num_infeasible {
                integer_master.add_constr("limitInfeasible", all_dummy_vars.iter().fold(LinExpr::new(), |a, b| a + b), Less, self.allowed_infeasible as f64).unwrap();
            }


            integer_master.update().unwrap();
            integer_master.optimize().unwrap();


            if integer_master.status().unwrap() != Status::Optimal {
                return Err(Generic("could not solve correctly"));
            }


        // do test if we are infeasible

        let has_dummy_values = integer_master.get_values(attr::X, all_dummy_vars.as_slice()).unwrap().iter().filter(|&v| *v >= CG_EPSILON).count();
        if has_dummy_values > self.allowed_infeasible {

            let infeasible_vehicles = integer_master.get_values(attr::X, all_dummy_vars.as_slice()).unwrap().iter().zip(self.get_active_vehicles()).filter_map(|(value,vehicle)| {
                if *value > CG_EPSILON {
                    Some(vehicle.index)
                } else {
                    None
                }
            }).collect::<Vec<usize>>();

            #[cfg(feature = "column_generation_debug")]
            println!("has {} dummy vars set", has_dummy_values);
            return Err(SolveError::VehiclesInfeasible(infeasible_vehicles));
        }



        Ok(
            SolvedCGResult {
                master_x: integer_master.get(attr::ObjVal).unwrap(),
                patterns: self.get_active_vehicles().map(|vehicle| {
                    if let Some(patterns) = &vehicle_patterns.get_vec(vehicle) {
                        let solution_values = integer_master.get_values(attr::X, patterns.iter().map(|(var, _)| var.clone()).collect::<Vec<Var>>().as_slice()).unwrap();

                        (vehicle,
                         patterns.iter()
                             .map(|(_, pattern)| pattern.clone())
                             .zip(solution_values)
                             .filter(|(_, value)| *value > 0.0)
                             .collect::<Vec<(SinglePattern<'a>, f64)>>()
                        )
                    } else {
                        (vehicle, Vec::new())
                    }
                }).collect::<Vec<(&'a Vehicle<'a>, Vec<(SinglePattern<'a>, PatternSelected)>)>>()
            }
        )

    }


    pub fn get_vehicles_that_can_be_feasible(vehicles: &'a [Vehicle<'a>], site_conf_builder : SiteConfFactory) -> Vec<Vehicle> {
        // Following block is to initialize an ndarray with individual RC pointers; With shorthand methods the
        // RC gets cloned resulting in all cells pointing to the same orgin. We do not want that!
        let mut tmp_vec: Vec<Rc<Cell<f64>>> = Vec::with_capacity(site_conf_builder.num_sites * MAX_PERIOD);
        for _ in 0..(site_conf_builder.num_sites * MAX_PERIOD) {
            tmp_vec.push(Rc::new(Cell::new(0_f64)))
        }
        let site_period_duals = Array2::<Rc<Cell<f64>>>::from_shape_vec((site_conf_builder.num_sites, MAX_PERIOD), tmp_vec).unwrap();


        let no_dual: Rc<Cell<f64>> = Rc::new(Cell::new(0_f64));
        let arc_site_period_duals = Rc::new(site_period_duals);


        vehicles.iter()
            .map(|vehicle| (vehicle,build_dag(vehicle, no_dual.clone(), arc_site_period_duals.clone(), &Vec::new(), site_conf_builder.full(1))))
            .filter(|(vehicle,(root, destination, dag))| {

                match generate_patterns(vehicle, dag, *root, *destination, 10000.0, false) {
                    Ok(e) => true,
                    Err(e) => {
                        eprintln!("v{:?} is infeasible because : {:?}",vehicle.id,e);
                        false
                    }
                }
            }).enumerate().map(|(idx,(vehicle,_))| {
                // clone and fix index
                let mut copy_vehicle = vehicle.clone();
                copy_vehicle.index = idx;
                copy_vehicle
            }).collect()

    }

    #[hawktracer(solve_relaxed_problem)]
    fn solve_relaxed_problem(&mut self, charge_filters: &[BranchingFilter<'a>], find_num_infeasible : bool) -> Result<SolvedCGResult<'a>, SolveError> {

        if self.should_stop.load(Relaxed) {
            return Err(SolveError::StoppedByExternal);
        }

        #[cfg(feature = "perf_statistics")]
        LPS_SOLVED.mark();

        #[cfg(feature = "column_generation_debug")] {
            print!("█");
            io::stdout().flush().ok().expect("Could not flush stdout");
        }

        // disable timelimit set by integer problem
        self.env.set(param::TimeLimit, INFINITY).unwrap();

        // Following block is to initialize an ndarray with individual RC pointers; With shorthand methods the
        // RC gets cloned resulting in all cells pointing to the same orgin. We do not want that!
        let mut tmp_vec: Vec<Rc<Cell<f64>>> = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
        for _ in 0..(self.sites.len() * MAX_PERIOD) {
            tmp_vec.push(Rc::new(Cell::new(0_f64)))
        }
        let site_period_duals = Array2::<Rc<Cell<f64>>>::from_shape_vec((self.sites.len(), MAX_PERIOD), tmp_vec).unwrap();


        let no_dual : Rc<Cell<f64>> = Rc::new(Cell::new(0_f64));
        let arc_site_period_duals = Rc::new(site_period_duals);


        // build dags vor all vehciles, not just active ones so that the index of the dag array matches.
        let vehicle_dags: Vec<(NodeIndex, NodeIndex, Graph<NodeWeight, EdgeWeight, Directed>)> = self.vehicles.iter()
            .map(|vehicle| build_dag(vehicle, no_dual.clone(), arc_site_period_duals.clone(), &charge_filters, self.site_sizes.clone())).collect();




        #[cfg(feature = "branching_debug")]
        println!("Active Filters: {:?}", &charge_filters);


        scoped_tracepoint!(_build_initial_rcmp);


        // create an empty model which associated with `env`:
        let mut master = Model::new("master", &self.env).unwrap();

        // initialize hashmaps for model variables and patterns
        let mut vehicle_patterns = CustomMultiHashMap::default();
        let mut vehicle_convexity: IndexMap<&Vehicle, Constr> = IndexMap::default();

        //let mut site_patterns = Vec::new();
        //let mut site_pattern_vars = Vec::new();


        let mut dummy_vars: Vec<Var> = Vec::with_capacity(self.get_active_vehicles().count());


        let mut site_constraints = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
        for site in &self.sites {
            #[cfg(feature = "column_generation_debug")]
                println!("Site index {}", site.index);





            for p in 0..MAX_PERIOD {
                site_constraints.push(
                    master.add_constr(
                        &format!("maxCapacity[{},{},{}]", site.id, site.index, p),
                        LinExpr::new(), Less, f64::from(self.site_sizes[site.index])
                    ).unwrap());
            }
        }

        let constr_max_capacity: Array2<Constr> = Array2::from_shape_vec((self.sites.len(), MAX_PERIOD), site_constraints).unwrap();







        let mut pattern_counter = 0;
        for vehicle in self.get_active_vehicles() {
            let dummy_var = master.add_var(&format!("dummy_column[{}]", vehicle.id), Continuous, DUMMY_COST, 0.0, 1.0, &[], &[]).unwrap();
            let constr_convexity = master.add_constr(&format!("convexity[{}]", vehicle.id), 1.0 * &dummy_var,  Equal, 1.0).unwrap();
            dummy_vars.push(dummy_var);
            vehicle_convexity.insert(vehicle, constr_convexity.clone());

            // add the existing patterns from the pool
            // copy paste from code in column generation below
            {
                let patterns = self.pattern_pool.get_active_patterns(vehicle, charge_filters.to_owned(), self.site_sizes.clone());

                for entry in patterns {


                    let mut const_vec: Vec<Constr> = Vec::with_capacity(entry.pattern.len()+1);
                    let mut val_vec = Vec::with_capacity(entry.pattern.len()+1);

                    // make column use one unit of convexity constraint
                    const_vec.push(constr_convexity.clone());
                    val_vec.push(1.0);

                    /*
                    #[cfg(feature = "column_generation_debug")]
                        println!("The pattern is {:?}", pattern);
                    */
                    // make column use one unit of capacity at every used site
                    let mut blacklisted_site = false;
                    for (_, site, period) in entry.pattern.iter() {
                        const_vec.push(constr_max_capacity[[site.index, charge_time_to_capacity_charge_time(period)]].clone());
                        val_vec.push(1.0);

                        if self.site_sizes[site.index]== 0 {
                            blacklisted_site = true;
                            break;
                        }

                    }

                    if !blacklisted_site {
                        let var_use_pattern = master.add_var(&format!("usePattern[{}]", pattern_counter), Continuous, 0.0, 0.0, gurobi::INFINITY, &const_vec, &val_vec).unwrap();
                        vehicle_patterns.insert(vehicle, (var_use_pattern, entry.pattern.clone()));
                        pattern_counter += 1;
                    }
                }


            }

        }

        if  ! find_num_infeasible {
            // if all vehicles have at least one pattern (theoretically do not need the dummy column)
            // then we can add a constraint limiting the amount of infeasibility to the allowed infeasibility
            if self.get_active_vehicles().all(|v| if let Some(p) = vehicle_patterns.get_vec(v) {
                !p.is_empty()
            } else { false }) {
                // limit my infeasibility
                master.add_constr("limitInf", dummy_vars.iter().fold(LinExpr::new(), |a, b| a + b), Less, self.allowed_infeasible as f64).unwrap();
            }
        }

        #[cfg(feature = "profiling_enabled")]
        drop(_build_initial_rcmp);

        /*
        |
        -->
         bis her haben wir einen dag für jedes taxi + eine constraint für jede zeit periode die ich an einer site nutzen kann.
            für jedes taxi ist eine covexity constraint angelegt
        */

        let mut last_convexity_dual_cost: IndexMap<&Vehicle, f64> = IndexMap::default();

            loop {

                if self.should_stop.load(Relaxed) {
                    return Err(SolveError::StoppedByExternal);
                }

                #[cfg(feature = "column_generation_debug")] {
                    print!("░");
                    io::stdout().flush().ok().expect("Could not flush stdout");
                }


                // integrate all of the variables into the model.
                {
                    scoped_tracepoint!(_rcmp);
                    master.update().unwrap();
                    master.optimize().unwrap();
                }

                if master.status().unwrap() != Status::Optimal {
                    return Err(SolveError::Generic("To Many Vehicles Infeasible"));
                }



                // collect the duals of the convexity from gurobi c api
                let max_capacity_constraints: Vec<Constr> = constr_max_capacity
                    .iter()
                    .cloned()
                    .collect();
                let max_capacity_constraint_duals = Array2::from_shape_vec((self.sites.len(), MAX_PERIOD), master.get_values(attr::Pi, &max_capacity_constraints).unwrap()).unwrap();

                #[cfg(feature = "column_generation_debug")]
                println!("{:?}",&max_capacity_constraint_duals);

                // update the charging station capacity duals in the shared array which is mapped to the dag edges
                for (site, duals) in self.sites.iter().zip(max_capacity_constraint_duals.outer_iter()) {
                    for (period_idx, dual) in duals.iter().enumerate() {
                        arc_site_period_duals[[site.index, period_idx]].set(*dual);
                    }
                }



                #[cfg(feature = "dag_output")] {

                    master.write("/tmp/problem_rust.sol").unwrap();
                    master.write("/tmp/problem_rust.lp").unwrap();

                    if arc_site_period_duals.iter().filter(|i| i.get() != 0.0).count() > 100 {
                        for (idx, (_, _, dag)) in vehicle_dags.iter().enumerate() {
                            crate::dag_builder::save_dag(&format!("problem_rust_graph_{}", idx), dag);
                        }


                        pause!("Pause at Loop; Has Written LP Files");
                    }
                }


                // collect the vehicle convexity constraint duals
                let vehicle_convexity_duals = master.get_values(attr::Pi, &vehicle_convexity.values().cloned().collect::<Vec<Constr>>()).unwrap();
                for (vehicle, dual) in self.get_active_vehicles().zip(vehicle_convexity_duals.iter()) {
                    #[cfg(feature = "column_generation_debug")]
                        println!("Updated convexity dual for vehicle {} to {}", vehicle.id, dual);
                    last_convexity_dual_cost.insert(vehicle, *dual);
                }


                let mut did_add_columns = false;
                {

                    scoped_tracepoint!(_inner_pricing_problem);
                    let mut infeasible_counter = 0;
                    for vehicle in self.get_active_vehicles() {
                        let (root, destination, ref dag) = vehicle_dags[vehicle.index];
                        match generate_patterns(vehicle, &dag, root, destination, last_convexity_dual_cost[&vehicle], false) {
                            Ok(path) => {
                            // get current vehicle convexity dual
                                let constr_convexity = vehicle_convexity.get(vehicle).unwrap();

                                // loop over all received patterns
                                for (detour_cost, reduced_costs, pattern) in path.iter() {
                                    let mut const_vec: Vec<Constr> = Vec::with_capacity(pattern.len()+1);
                                    let mut val_vec = Vec::with_capacity(pattern.len()+1);

                                    // make column use one unit of convexity constraint
                                    const_vec.push(constr_convexity.clone());
                                    val_vec.push(1.0);

                                    /* #[cfg(feature = "column_generation_debug")]
                                println!("The pattern is {:?}", pattern);
    */
                                    // make column use one unit of capacity at every used site
                                    for (_, site, period) in pattern.iter() {
                                        const_vec.push(constr_max_capacity[[site.index, charge_time_to_capacity_charge_time(period )]].clone());
                                        val_vec.push(1.0);
                                    }


                                    #[cfg(feature = "column_generation_debug")] {
                                        println!("Adding column {} to vehicle {}({}) with detour cost of {} and reduced costs of {}", pattern_counter, vehicle.id, vehicle.index, detour_cost, reduced_costs);
                                        println!("{:?}", format_pattern(&pattern));
                                    }


                                    let var_use_pattern = master.add_var(&format!("usePattern[{}]", pattern_counter), Continuous, 0.0, 0.0, gurobi::INFINITY, &const_vec, &val_vec).unwrap();
                                    vehicle_patterns.insert(vehicle, (var_use_pattern, pattern.clone()));


                                    // add the pattern to the pool

                                    self.pattern_pool.add_pattern(vehicle, *detour_cost, pattern.clone());


                                    did_add_columns = true;
                                    pattern_counter += 1;
                                }
                            }
                            Err(e) => {
                                // cant find single path for vehicle

                                // if we also have not a single valid pattern configuration must be infeasible
                                // thus exit early unless we try to find the number of infeasible taxis.
                                if  self.pattern_pool.get_active_patterns(vehicle, charge_filters.to_owned(), self.site_sizes.clone()).next().is_none() {
                                    infeasible_counter += 1;

                                    if ! find_num_infeasible && infeasible_counter > self.allowed_infeasible  {
                                        return Err(SolveError::Generic("Has Infeasible over Infeasible Counter"));
                                    }
                                }

                            }
                        }

                    }

                    if !did_add_columns {
                        break
                    }
                }
            }
        #[cfg(feature = "column_generation_debug")]
        println!();

        // recalculate final solution
        master.update().unwrap();
        master.optimize().unwrap();

        #[cfg(feature = "column_generation_debug")] {
            master.write("/tmp/problem_rust.sol").unwrap();
            master.write("/tmp/problem_rust.lp").unwrap();
        }


        if master.status().unwrap() != Status::Optimal {
            return Err(SolveError::NoQuickIntegerResult);
        }


        let has_dummy_values = master.get_values(attr::X, dummy_vars.as_slice()).unwrap().iter().filter(|&v| *v >= CG_EPSILON).count();
        if has_dummy_values > self.allowed_infeasible {

            let infeasible_vehicles = master.get_values(attr::X, dummy_vars.as_slice()).unwrap().iter().zip(self.get_active_vehicles()).filter_map(|(value,vehicle)| {
                if *value > CG_EPSILON {
                    Some(vehicle.index)
                } else {
                    None
                }
            }).collect::<Vec<usize>>();

            #[cfg(feature = "column_generation_debug")]
            println!("has {} dummy vars set", has_dummy_values);
            return Err(SolveError::VehiclesInfeasible(infeasible_vehicles));
        }



        Ok(
            SolvedCGResult {
                master_x: master.get(attr::ObjVal).unwrap(),
                patterns: self.get_active_vehicles().map(|vehicle| {
                    if let Some(patterns) = &vehicle_patterns.get_vec(vehicle) {
                    let solution_values = master.get_values(attr::X, patterns.iter().map(|(var, _)| var.clone()).collect::<Vec<Var>>().as_slice()).unwrap();

                    (vehicle,
                     patterns.iter()
                         .map(|(_, pattern)| pattern.clone())
                         .zip(solution_values)
                         .filter(|(_, value)| *value > 0.0)
                         .collect::<Vec<(SinglePattern<'a>, f64)>>()
                    )
                    } else {
                            // no patterns
                            (vehicle, Vec::new())
                    } }).collect::<Vec<(&'a Vehicle<'a>, Vec<(SinglePattern<'a>, PatternSelected)>)>>()

            }
        )
    }

    fn should_branch_be_cut(&mut self, result : &SolvedCGResult<'a>) -> bool {



        if let Some(best_int) = self.current_upper_bound {


            #[cfg(feature = "column_generation_exit_early")] {
                // early exit ! if we have any non-fractional solution
                #[cfg(feature = "branching_debug")] {
                    println!("EARLY EXIT");
                }
                return true;
            }


            #[cfg(feature = "branching_debug")] {
                let gap = (1.0 - best_int / result.master_x).abs();
                println!("Current cost of {}, best is {}, gap to fractional : {:.4}%", result.master_x, best_int, gap * 100.0);
            }

            if result.master_x >= best_int {
                #[cfg(feature = "branching_debug")] {
                    println!("#####");
                    println!("CUTTING BRANCH SINCE WORSE THAN BEST BOUND");
                    println!("#####");
                }
                return true;
            }
        }

        let mut seen_vehicles : HashSet<Vehicle> = HashSet::new();
        let mut has_multiple_patterns = false;
        let mut count_feasible_vehicles = 0;

        for (vehicle,patterns) in  &result.patterns {

            let active_patterns = patterns.iter().filter(|(_,x)| {
                *x > CG_EPSILON
            }).count();


            if active_patterns > 1 {
                has_multiple_patterns = true;
            } else if  active_patterns == 1 {
                count_feasible_vehicles += 1;
            }

        }

        if !has_multiple_patterns {
            #[cfg(feature = "branching_debug")] {
                println!("#####");
                println!("HAS A NON FRACTIONAL RESULT");
            }





            if let Some(best_int) = self.current_upper_bound {
                if result.master_x < best_int {

                    #[cfg(feature = "branching_debug")] {
                        println!("#####");
                        println!("IS NEW BEST!");
                    }
                    self.current_upper_bound = Some(result.master_x);
                    self.current_best_pattern = Some(result.patterns.iter().map(|(vehicle, patterns)| {
                        (*vehicle, patterns.iter().find(|(_, value)| *value > 0.0).unwrap().0.clone())
                    }).collect());
                }
            } else {
                #[cfg(feature = "branching_debug")] {
                    println!("#####");
                    println!("IS FIRST BEST!");
                }

                self.current_upper_bound = Some(result.master_x);
                self.current_best_pattern = Some(result.patterns.iter().map(|(vehicle, patterns)| {
                    (*vehicle,
                        if let Some(pattern) = patterns.iter().find(|(_, value)| *value > 0.0) {
                            pattern.0.clone()
                        } else {
                            Vec::new()
                        })
                }).collect());
            }

            #[cfg(feature = "branching_debug")]
                println!("#####");

        }

        false
    }


    #[hawktracer(check_result_for_branching_points)]
    pub fn check_result_for_branching_points(&mut self, charge_filters: &[BranchingFilter<'a>], result: &SolvedCGResult<'a>) -> Option<Vec<(BranchingFilter<'a>, BranchingFilter<'a>)>>  {
        #[cfg(feature = "branching_debug")] {
            println!("Checking Result");
            println!("Has {} patterns in the global pool", self.pattern_pool.num_columns());
        }




        use std::fs;

        #[cfg(feature = "infeasibility_events")] {
            self.invisibility_event_counter += 1;
            let folder = Path::new("/tmp/infeasibility_events").join(self.invisibility_event_counter.to_string());
            fs::create_dir_all(&folder);
        }

        if self.should_branch_be_cut(result) {
            return None;
        }

        let mut possible_branches = Vec::new();


        // look at every vehicle seperately
        for (vehicle, patterns) in &result.patterns {




            // now try to identify the location of the fractionality

            // we want to branch on different sites used in a segment first.
            let mut site_segment_combo_count : CustomHashMap<(&Segment, &Site),u8> = CustomHashMap::default();
            // over all the patterns of the vehicle
            let mut active_patterns_count = 0;
            for (pattern, value) in patterns {
                // looking only on the activated...
                if *value > 0.0 {


                    // use this for deduplication of segment site in this pattern (ignore time)
                    let mut site_segment_combo: HashSet<(&Segment, &Site)> = HashSet::default();

                    active_patterns_count += 1;
                    // record that we use a certain site-segment combination in the pattern
                    // (deduplicated, so only count once)
                    for (segment, site, time) in pattern {
                        site_segment_combo.insert((segment, site));
                    }
                    // collect unique uses of site<->segment uses in this pattern
                    // & increase global counter for vehicle of used site-segment combos from this pattern
                    for element in site_segment_combo {
                        let entry = site_segment_combo_count.entry(element).or_insert(0);
                        *entry += 1;
                    }
                }
            }



            // now detect the patterns where we use more than one site at a segment
            // Anytime where the number of usages of a site segment tuple is not equal to the total number of active patterns (paths/columns)
            // there must be at least one point (involving the given pattern) where one pattern does not use the segment / site combo
            // eg. if i use a site after a segment all active patterns must use it. Otherwise there is a divergence in the path.
            let  multiple_sites_in_segment = site_segment_combo_count.iter()
                    .filter(|(_,&combo_counts)| usize::from(combo_counts) != active_patterns_count)
                    .collect::<Vec<_>>() ;


            #[cfg(feature = "infeasibility_events")] {
                if multiple_sites_in_segment.len() > 0 {
                    println!("Infeasibility event {} with vehicle {}",self.invisibility_event_counter, vehicle.id );
                    // debug output these things!
                    // ###########
                    use crate::petgraph::visit::EdgeRef;
                    use crate::petgraph::visit::NodeIndexable;
                    use crate::petgraph::visit::IntoNodeReferences;
                    use crate::petgraph::visit::NodeRef;
                    let mut tmp_vec: Vec<Rc<Cell<f64>>> = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
                    for _ in 0..(self.sites.len() * MAX_PERIOD) {
                        tmp_vec.push(Rc::new(Cell::new(0_f64)))
                    }
                    let site_period_duals = Array2::<Rc<Cell<f64>>>::from_shape_vec((self.sites.len(), MAX_PERIOD), tmp_vec).unwrap();
                    let no_dual: Rc<Cell<f64>> = Rc::new(Cell::new(0_f64));
                    let arc_site_period_duals = Rc::new(site_period_duals);


                    let (start, end, dag) = build_dag(vehicle, no_dual.clone(), arc_site_period_duals.clone(), &Vec::new(), self.site_sizes.clone());


                    let folder = Path::new("/tmp/infeasibility_events").join(self.invisibility_event_counter.to_string());
                    let mut f = File::create(folder.join(format!("vehicle_{}.dot", vehicle.id))).unwrap();

                    writeln!(f, "digraph {{");


                    // output all labels
                    for node in dag.node_references() {
                        let weight = (node.weight() as &NodeWeight);

                        let node_style = match (weight.get_site(), weight.get_segment()) {
                            (Some(ref site), Some(ref segment)) => {
                                if multiple_sites_in_segment.iter().filter_map(|((si, seg), _)| {
                                    if si.id == segment.id && site.site.id == seg.id {
                                        Some({})
                                    } else {
                                        None
                                    }
                                }).count() > 0 {
                                    "fillcolor = red,style=filled"
                                } else {
                                    ""
                                }
                            },

                            _ => "",
                        };

                        write!(f, "{}{} [ ", "     ", dag.to_index(node.id()), );
                        write!(f, "label = \"");
                        write!(f, "{}", node.weight());


                        write!(f, "\" ");
                        writeln!(f, ",{}]", node_style);
                    }
                    // output all edges
                    for (i, edge) in dag.edge_references().enumerate() {
                        write!(
                            f,
                            "{}{} {} {} [ ",
                            "    ",
                            dag.to_index(edge.source()),
                            "->",
                            dag.to_index(edge.target()),
                        );

                        write!(f, "label = \"");
                        write!(f, "{}", i);

                        //edge_fmt(edge.weight(), &mut |d| Escaped(d).fmt(f))?;

                        write!(f, "\" ");
                        writeln!(f, "]");
                    }

                    writeln!(f, "}}");


                    // ###########
                }
            }


            // if there is actually two different segments used (not two patterns using the same segment at different times)
            // we branch on this first
            if multiple_sites_in_segment.len() > 1 {

                let ((chosen_segment, chosen_site), value) = multiple_sites_in_segment.first().expect("Must work since we checked count before");

                possible_branches.push((
                    BranchingFilter::ChargeSegmentSite(vehicle, *chosen_segment, *chosen_site, true),
                    BranchingFilter::ChargeSegmentSite(vehicle, *chosen_segment, *chosen_site, false))
                );



                #[cfg(feature = "branching_debug")]
                println!("Vehicle {} has fractional result with {}!", vehicle.id, value);
                // find branch on the result
                // select a random pattern to branch on


            } else {
                // check if the fractionality comes from time incompatibilites
                // for every pattern; Record start time of charge at site


                let mut earliest_charge: CustomHashMap<(&Segment, &Site), Period> = CustomHashMap::default();
                for (pattern, value) in patterns {
                    if *value > 0.0 {
                        for (segment, site, time) in pattern {
                            let entry = earliest_charge.entry((segment,site)).or_insert(*time);
                            if *entry > *time {
                                *entry = *time;
                            }
                        }
                    }
                }

               let mut branch_on : Option<(&Segment,&Site, Period)> = None;

               'search_loop: for (pattern, value) in patterns {
                    if *value > 0.0 {

                        let mut pattern_earliest_charge: CustomHashMap<(&Segment, &Site), Period> = CustomHashMap::default();
                        for (segment, site, time) in pattern {
                            let entry = pattern_earliest_charge.entry((segment,site)).or_insert(*time);
                            if *entry > *time {
                                *entry = *time;
                            }
                        }


                        for (segment, site, time) in pattern {
                            let entry = earliest_charge[&(*segment,*site)];
                            let pattern_charge = pattern_earliest_charge[&(*segment,*site)];

                            if entry != pattern_charge {
                                branch_on = Some((segment,site,entry));
                                break 'search_loop;
                            }
                        }


                    }
                }

                if let Some((segment,site,period)) = branch_on {
                    possible_branches.push((
                        BranchingFilter::ChargeSegmentSiteTime(vehicle, segment, site, period, true),
                        BranchingFilter::ChargeSegmentSiteTime(vehicle, segment, site, period, false)
                    ));
                }
            }
        }

        if possible_branches.is_empty() {
            return None;
        } else {
            return Some(possible_branches);
        }
    }
}
