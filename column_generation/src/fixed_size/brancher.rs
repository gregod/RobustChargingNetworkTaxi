#![allow(dead_code,unused)]

pub const DUMMY_COST : f64 = 1.0;
#[cfg(feature = "profiling_enabled")]
use rust_hawktracer::*;

use grb::{Status, param, Model, Env, attr, INFINITY, Var, Constr, ConstrSense, Expr, c};

use std::io::{Read, BufWriter};
use std::rc::Rc;
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Write;
use std::iter::Sum;

macro_rules! pause {
    ($x:expr) => {
        println!($x);
        io::stdin().read(&mut [0u8]);
    };
}

use shared::{Segment, Vehicle, Site, Period, CustomMultiHashMap, CustomHashMap, MAX_PERIOD, charge_time_to_capacity_charge_time, ReachableSite, CustomHashSet};
use crate::{SiteArrayRef, CG_EPSILON, format_pattern};
use indexmap::IndexMap;
use itertools::assert_equal;
use crate::fixed_size::site_conf::{SiteConf, SiteConfFactory};
use crate::pattern_pool::{Pattern, PatternEntry, PatternPool};
use ndarray::Array2;
use crate::dag_builder::{build_dag, NodeWeight, EdgeWeight};
use crate::rcsp::generate_patterns;
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
use crate::branching_filter::{BranchingFilter, DataFloat, Dir};
use snowflake::ProcessUniqueId;
use petgraph::dot::Dot;
use petgraph::visit::Walker;
use rand::distributions::Distribution;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::hash::Hash;


pub type SinglePattern = Vec<(SegmentId, SiteIndex, Period)>;
pub type ResultPattern =  Vec<(VehicleIndex,SinglePattern)>;
type PatternSelected = f64;



#[repr(usize)]
#[derive(Clone,Debug)]
pub enum BranchPriority {
    Default = 1,
    Higher = 2
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


pub trait HasPriority {
    fn get_priority_when_no_bound(&self) -> u32;
    fn get_priority_when_existing_bound(&self) -> u32;
}
use binary_heap_plus::{BinaryHeap, FnComparator, KeyComparator};
use grb::attribute::ObjAttrGet;
use grb::constr::IneqExpr;
use grb::prelude::Continuous;
use grb::VarType::{Binary, Integer};
use crate::branching_filter::Dir::{Greater, Less};
use crate::fixed_size::cg_model::{CgModel, SegmentId, SiteIndex, VehicleIndex};


pub struct BranchQueue<T>
    where T : HasPriority
{
    queue: BinaryHeap<T,KeyComparator<fn(&T) -> u32>>,
    did_swap_priority : bool,
}

impl <T> BranchQueue<T>
    where T : HasPriority{
    fn new() -> Self {
        BranchQueue {
            queue : BinaryHeap::new_by_key(|f| f.get_priority_when_no_bound()),
            did_swap_priority : false
        }
    }

    pub fn push(&mut self, item : T) {
        self.queue.push(item);
    }
    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop()
    }


    pub fn now_has_bound(&mut self) {

        // only swap if not previously
        if ! self.did_swap_priority {
            self.did_swap_priority = false;
            self.queue.replace_cmp(KeyComparator(|f| f.get_priority_when_existing_bound()));
        }
    }
}

pub struct BranchNode {
    pub id : ProcessUniqueId,
    pub parent_id : ProcessUniqueId,
    pub parent_approx : bool,
    pub parent_objective : f64,
    pub branch_priority: BranchPriority,
    pub filters : Vec<BranchingFilter>,
    pub meta : BranchMeta
}






impl BranchNode {

    pub fn root(env : &Env, sites : Vec<Site>, site_sizes : Vec<u8>, vehicles : &[Vehicle]) -> Self {
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

    pub fn integer_from_parent( parent : &BranchNode) -> Self {
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



    pub fn from_parent(parent : &BranchNode, new_filter : BranchingFilter, branch_type : BranchPriority) -> Self {

        let mut filters = parent.filters.to_owned();
        filters.push(new_filter);

        BranchNode {
            id : ProcessUniqueId::new(),
            parent_id : parent.id,
            parent_approx : parent.parent_approx,
            parent_objective : parent.parent_objective,
            filters,
            branch_priority: branch_type,
            meta :  BranchMeta::Default
        }
    }


    pub fn from_parent_to_exact(parent : &BranchNode,  parent_objective : f64,parent_approx : bool) -> Self {
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

    pub fn from_parent_multifilters(parent : &BranchNode, parent_objective : f64,parent_approx : bool, new_filters : Vec<BranchingFilter>, branch_type : BranchPriority) -> Self {
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

pub struct SolvedCGResult {
    master_x : f64,
    patterns : Vec<(VehicleIndex, Vec<(SinglePattern, PatternSelected)>)>
}


impl HasPriority for BranchNode {
    fn get_priority_when_no_bound(&self) -> u32 {

        // higher is better


        if matches!(self.meta,BranchMeta::OnlyInteger) {
            return u32::MAX;
        }
        // approximate for depth. //smaller is always better so leave some room below 100


        return self.filters.len() as u32  + match self.branch_priority {BranchPriority::Default => {0}, BranchPriority::Higher => {1000}}


    }
    fn get_priority_when_existing_bound(&self) -> u32 {

        if matches!(self.meta,BranchMeta::OnlyInteger) {
            return   u32::MAX; // is important
        }

        self.parent_objective as u32
    }
}

pub struct Brancher<'a> {
    sites:  Vec<Site>,
    vehicles: Vec<Vehicle<'a>>,
    allowed_infeasible : usize,
    site_sizes:  SiteConf,
    current_upper_bound: Option<f64>,
    current_best_pattern: Option<ResultPattern>,
    pattern_pool: PatternPool,
    open_branches : BranchQueue<BranchNode>,
    pub env : &'a Env,
    pub env_integer : &'a Env,
    cg_model : CgModel,
    should_stop : Arc<AtomicBool>,
    invisibility_event_counter : usize,
    sort_many_columns_first : bool
}

#[derive(Debug,Clone)]
pub enum SolveError {
    NoQuickIntegerResult,
    NoQuickResult,
    VehiclesInfeasible(Vec<VehicleIndex>),
    Generic(&'static str),
    StoppedByExternal
}



impl<'a,'b : 'a> Brancher<'a> {



    pub fn load_columns(&'b mut self, path : PathBuf) {
        self.pattern_pool.read_from_disk(path, self.vehicles.clone(), &self.sites);
    }

    pub fn write_columns(&self, path : &PathBuf) {
        self.pattern_pool.write_to_disk(path, &self.get_vehicles())
    }

    pub fn get_pattern_pool(&self) -> &PatternPool {
        &self.pattern_pool
    }

    pub fn new(sites: Vec<Site>,
               vehicles: Vec<Vehicle<'a>>,
               site_sizes: SiteConf,
               env : &'a Env,
               env_integer : &'a Env,
               allowed_infeasible : usize,
                               sort_many_columns_first : bool,
               should_stop : Arc<AtomicBool>, pattern_pool : PatternPool) -> Brancher<'a> {







        Brancher {
            cg_model : CgModel::new(&env, sites.clone(), site_sizes.clone(), vehicles.clone()),
            pattern_pool,
            sites,
            vehicles,
            site_sizes,
            sort_many_columns_first,
            current_upper_bound: None,
            current_best_pattern: None,

            open_branches : BranchQueue::new(),
            env,
            env_integer,
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



    pub fn get_vehicles(&self) -> &[Vehicle<'a>] {
        &self.vehicles
    }

    pub fn process_branch_node(&mut self, node : &BranchNode,find_num_infeasible : bool) -> Result<SolvedCGResult,SolveError>  {



        match node.meta {
            BranchMeta::Default  => {
                self.solve_relaxed_problem(&node.filters, find_num_infeasible)
            },
            BranchMeta::Approx => {
                self.solve_relaxed_problem(&node.filters, find_num_infeasible)
            },
            BranchMeta::OnlyInteger => {
                self.solve_integer_problem(&node.filters,find_num_infeasible)
            }
        }
    }

    #[hawktracer(solve_pattern)]
    pub fn solve(&mut self, find_quick_result_or_exit: bool, find_num_infeasible : bool) -> Result<(f64, ResultPattern),SolveError> {

        #[cfg(feature = "level_print")]
        println!("- Solving Operational Problem");
        if self.should_stop.load(Relaxed) {
            return Err(SolveError::StoppedByExternal);
        }


        #[cfg(feature = "progress_icons")]
        print!("s");


        #[cfg(feature = "perf_statistics")]
        let timer = ConfigurationTimer::new();


        #[cfg(feature = "column_generation_debug")]
        println!("Solving first Problem ! (Has {} patterns;)", self.pattern_pool.num_columns());


        /* reset bounds */
        self.current_best_pattern = None;
        self.current_upper_bound = None;



        /* reset open branches */
        self.open_branches = BranchQueue::new();
        let root_node = BranchNode::root(&self.env, self.sites.clone(), self.site_sizes.clone(), &self.vehicles);

        #[cfg(feature = "column_generation_sometimes_integer")] {
            if self.pattern_pool.num_columns() > 0 {
                self.open_branches.push(BranchNode::integer_from_parent(&root_node))
            }
        }

        let mut next_integer_solve = 0;

        self.open_branches.push(root_node);
        let mut last_error = Generic("No Upper Bound FOUND!");
        loop {


            let onode = self.open_branches.pop();




            if let Some(node) = onode {

                scoped_tracepoint!(_branch_node);


                #[cfg(feature = "progress_icons")]
                print!("b(c:{})", self.get_num_colums());





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

                #[cfg(feature = "level_print")]
                println!("-- Evaluating Branch Node {}", if matches!(node.meta, BranchMeta::OnlyInteger) { "(integer)" } else { "" });

                match self.process_branch_node(&node,find_num_infeasible) {

                 Ok(result) =>  {

                    #[cfg(feature = "branching_debug")]
                    println!("NODE\t{}\t{}\t{}\t{}",node.id,node.parent_id,node.parent_objective,result.master_x);


                    // now check for possible branching points!
                    if let Some(branches) = self.check_result_for_branching_points(&node, &result) {


                        // pick branching point
                        // initially pick first!
                        for b in branches {
                            self.open_branches.push(b);
                        }


                        #[cfg(feature = "column_generation_sometimes_integer")] {
                            // integer solve after every X new columns
                            if self.pattern_pool.num_columns() > next_integer_solve {
                                next_integer_solve = self.pattern_pool.num_columns() + 1000;
                                self.open_branches.push(
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


                        // if we have enough feasible vehicles exit branching early
                        #[cfg(feature = "column_generation_exit_early")] {
                            if result.patterns.iter()
                                .map(|(_, patterns)| patterns.iter()
                                    .filter(|(_, value)| *value > 0.0)
                                    .count()
                                ).sum::<usize>() >= self.get_vehicles().len() - self.allowed_infeasible {
                                break
                            }
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
    fn solve_integer_problem(&mut self, charge_filters: &[BranchingFilter], find_num_infeasible : bool) -> Result<SolvedCGResult, SolveError> {

            if self.should_stop.load(Relaxed) {
                return Err(SolveError::StoppedByExternal);
            }

            #[cfg(feature = "perf_statistics")]
            LPS_SOLVED.mark();
            // integer problem is only solved as kind of heuristic on the root node, thus only give limited amount of time.


            // create an empty model which associated with `env`:
            let mut integer_master = Model::with_env("integer_env", self.env_integer).unwrap();

            // initialize hashmaps for model variables and patterns
            let mut vehicle_patterns = CustomMultiHashMap::default();
            let mut vehicle_convexity: IndexMap<&Vehicle, Constr> = IndexMap::default();



            let mut site_constraints = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
            for site in &self.sites {
                for p in 0..MAX_PERIOD {
                    site_constraints.push(
                        integer_master.add_constr(&format!("maxCapacity[{},{},{}]", site.id, site.index, p),
                                                  c!( 0 <= f64::from(self.site_sizes[site.index]))/*site.capacity.into()*/).unwrap());
                }
            }
            let constr_max_capacity: Array2<Constr> = Array2::from_shape_vec((self.sites.len(), MAX_PERIOD), site_constraints).unwrap();


            let mut site_time_branch_constraint : CustomMultiHashMap<(SiteIndex,Period), Constr> = CustomMultiHashMap::default();
            for branch in charge_filters {
                // just initalize those that we have in the branching constraint
                if let BranchingFilter::MasterNumberOfCharges(site,period,direction,value) = branch {
                    site_time_branch_constraint.insert((*site,*period),
                                                       integer_master.add_constr(
                                                           &format!("branchCapacity[{},{},{}]", site.index(), period, value.float()),
                                                           IneqExpr{
                                                               lhs: Expr::default(),
                                                               sense: match direction {
                                                                   Dir::Less => ConstrSense::Less,
                                                                   Dir::Greater => ConstrSense::Greater,
                                                               },
                                                               rhs:  Expr::Constant(value.float())
                                                           }
                                                        ).unwrap()
                    )
                }
            }



            let mut pattern_counter = 0;
            let mut all_dummy_vars = Vec::with_capacity(self.get_vehicles().len());
            for vehicle in self.get_vehicles() {
                let dummy_var = integer_master.add_var(&format!("dummy_column[{}]", vehicle.id), Binary, DUMMY_COST, 0.0, 1.0, []).unwrap();
                let constr_convexity = integer_master.add_constr(&format!("convexity[{}]", vehicle.id), c!(1.0 * dummy_var == 1.0)).unwrap();
                all_dummy_vars.push(dummy_var);

                vehicle_convexity.insert(vehicle, constr_convexity.clone());

                // add the existing patterns from the pool
                // copy paste from code in column generation below
                {
                    let patterns = self.pattern_pool.get_active_patterns(VehicleIndex::new(vehicle), &charge_filters, self.site_sizes.clone());

                    for entry in patterns {
                        let mut coef_vec: Vec<(Constr, f64)> = Vec::with_capacity(entry.pattern.len()+1);


                        // make column use one unit of convexity constraint
                        coef_vec.push((constr_convexity,1.0));



                        // make column use one unit of capacity at every used site
                        for (_, site, period) in entry.pattern.iter() {


                            coef_vec.push((constr_max_capacity[[site.index(), charge_time_to_capacity_charge_time(period)]].clone(),1.0));



                            // register the site_time branch constraints!
                            if let Some(entry) = site_time_branch_constraint.get_vec(&(*site,*period)) {
                                for constr in entry {
                                    coef_vec.push((constr.clone(), 1.0));

                                }
                            }

                        }



                        let var_use_pattern = integer_master.add_var(&format!("usePattern[{}]", pattern_counter), Binary, 0.0, 0.0, 1.0, coef_vec).unwrap();
                        vehicle_patterns.insert(vehicle, (var_use_pattern, entry.pattern.clone()));
                        pattern_counter += 1;
                    }
                }

            }

            if ! find_num_infeasible {
                integer_master.add_constr("limitInfeasible", c!(Expr::sum(all_dummy_vars.iter()) <=  self.allowed_infeasible as f64)).unwrap();
            }


            integer_master.update().unwrap();
            integer_master.optimize().unwrap();


            if integer_master.status().unwrap() != Status::Optimal {
                return Err(Generic("could not solve correctly"));
            }


        // do test if we are infeasible

        let has_dummy_values = integer_master.get_obj_attr_batch(attr::X, all_dummy_vars.clone()).unwrap().iter().filter(|&v| *v >= CG_EPSILON).count();
        if has_dummy_values > self.allowed_infeasible {

            let infeasible_vehicles = integer_master.get_obj_attr_batch(attr::X, all_dummy_vars.clone()).unwrap().iter().zip(self.get_vehicles()).filter_map(|(value,vehicle)| {
                if *value > CG_EPSILON {
                    Some(VehicleIndex::new(vehicle))
                } else {
                    None
                }
            }).collect::<Vec<VehicleIndex>>();

            #[cfg(feature = "column_generation_debug")]
            println!("has {} dummy vars set", has_dummy_values);
            return Err(SolveError::VehiclesInfeasible(infeasible_vehicles));
        }



        Ok(
            SolvedCGResult {
                master_x: integer_master.get_attr(attr::ObjVal).unwrap(),
                patterns: self.get_vehicles().iter().map(|vehicle| {
                    if let Some(patterns) = &vehicle_patterns.get_vec(vehicle) {
                        let solution_values = integer_master.get_obj_attr_batch(attr::X, patterns.iter().map(|(var, _)| var.clone()).collect::<Vec<Var>>()).unwrap();

                        (VehicleIndex::new(vehicle),
                         patterns.iter()
                             .map(|(_, pattern)| pattern.clone())
                             .zip(solution_values)
                             .filter(|(_, value)| *value > 0.0)
                             .collect::<Vec<(SinglePattern, f64)>>()
                        )
                    } else {
                        (VehicleIndex::new(vehicle), Vec::new())
                    }
                }).collect::<Vec<(VehicleIndex, Vec<(SinglePattern, PatternSelected)>)>>()
            }
        )

    }


    pub fn get_vehicles_that_can_be_feasible<'f>(vehicles: impl Iterator<Item=&'f Vehicle<'f>>, site_conf : SiteConf) -> Vec<&'f Vehicle<'f>> {
        // Following block is to initialize an ndarray with individual RC pointers; With shorthand methods the
        // RC gets cloned resulting in all cells pointing to the same orgin. We do not want that!
        let mut tmp_vec: Vec<Rc<Cell<f64>>> = Vec::with_capacity(site_conf.len() * MAX_PERIOD);
        for _ in 0..(site_conf.len() * MAX_PERIOD) {
            tmp_vec.push(Rc::new(Cell::new(0_f64)))
        }
        let site_period_duals = Array2::<Rc<Cell<f64>>>::from_shape_vec((site_conf.len(), MAX_PERIOD), tmp_vec).unwrap();


        let no_dual: Rc<Cell<f64>> = Rc::new(Cell::new(0_f64));
        let arc_site_period_duals = Rc::new(site_period_duals);


        vehicles
            .map(|vehicle| (vehicle,build_dag(vehicle, no_dual.clone(), arc_site_period_duals.clone(), &Vec::new(), site_conf.clone())))
            .filter(|(vehicle,(root, destination, dag))| {


                match generate_patterns(vehicle, dag, *root, *destination, 10000.0, false, &[]) {
                    Ok(e) => true,
                    Err(e) => {
                      //  eprintln!("v{:?} is infeasible because : {:?}",vehicle.id,e);
                        false
                    }
                }
            }).map(|e| {
            (e.0)
        }).collect()

    }


    fn pattern_similarity(a : &Pattern, b : &Pattern) -> i32 {

        let mut site_set_a = CustomHashSet::new();
        let mut site_set_b  = CustomHashSet::new();

        let mut time_set_a = CustomHashSet::new();
        let mut time_set_b  = CustomHashSet::new();

        for (segment,site,time) in a {
            site_set_a.insert(site);
            time_set_a.insert(time);
        }

        for (segment,site,time) in b {
            site_set_b.insert(site);
            time_set_b.insert(time);
        }


        let count_sites_that_are_not_in_both = site_set_a.symmetric_difference(&site_set_b).count() as i32;
        let count_times_that_are_not_in_both = time_set_a.symmetric_difference(&time_set_b).count() as i32;


        10 * count_sites_that_are_not_in_both + count_times_that_are_not_in_both
    }

    fn retain_diverse_columns_and_first(&self, patterns: &mut Vec<(f64, f64, Pattern)>, k : usize) {



        let num_patterns = patterns.len();
        if k > num_patterns {
            return;
        }

        use kmedoids::{fasterpam, first_k};

        let mut similarity_matrix: Array2<i32> = Array2::from_elem((num_patterns,num_patterns), 0);
        let mut max_similarity = 0;
        for (ia,a) in patterns.iter().enumerate() {
            for (ib,b) in patterns.iter().enumerate() {
                if a != b {

                    let similarity = Self::pattern_similarity(&a.2, &b.2);;
                    if similarity > max_similarity {
                        max_similarity = similarity;
                    }
                    similarity_matrix[[ia,ib]]  = similarity;
                }
            }
        }

        for i in 0..num_patterns {
            similarity_matrix[[i,i]] = max_similarity;
        }

        // turn similarity into distance -> the more similar the less distance
        let distance_matrix = max_similarity - similarity_matrix;


        let mut meds : Vec<usize> = kmedoids::first_k(k.min(num_patterns));
        let (_loss, _, _iter, _swaps) : (i32, _, _, _)  = kmedoids::fasterpam(&distance_matrix, &mut meds, 100);

        let mut index = 0;
        patterns.retain(|_| {
            index += 1;
            // keep first 10% and all cluster centers
            if (index - 1) <= (num_patterns / 10).max(5) || meds.contains(&(index - 1)) {
                true
            } else {
                false
            }
        })

    }

    #[hawktracer(solve_relaxed_problem)]
    fn solve_relaxed_problem(&mut self, charge_filters: &[BranchingFilter], find_num_infeasible : bool) -> Result<SolvedCGResult, SolveError> {

        #[cfg(feature = "level_print")]
        println!("--- Running Column Generation");

        if self.should_stop.load(Relaxed) {
            return Err(SolveError::StoppedByExternal);
        }

        #[cfg(feature = "perf_statistics")]
        LPS_SOLVED.mark();

        // #[cfg(feature = "column_generation_debug")] {
            #[cfg(feature = "progress_icons")] {
                print!("█");
                io::stdout().flush().ok().expect("Could not flush stdout");
            }
        //}





        // Following block is to initialize an ndarray with individual RC pointers; With shorthand methods the
        // RC gets cloned resulting in all cells pointing to the same orgin. We do not want that!
        let mut tmp_vec: Vec<Rc<Cell<f64>>> = Vec::with_capacity(self.sites.len() * MAX_PERIOD);
        for _ in 0..(self.sites.len() * MAX_PERIOD) {
            tmp_vec.push(Rc::new(Cell::new(0_f64)))
        }
        let site_period_duals = Array2::<Rc<Cell<f64>>>::from_shape_vec((self.sites.len(), MAX_PERIOD), tmp_vec).unwrap();


        let no_dual : Rc<Cell<f64>> = Rc::new(Cell::new(0_f64));
        let arc_site_period_duals = Rc::new(site_period_duals);


        // build dags vor all vehicles, not just active ones so that the index of the dag array matches.
        let vehicle_dags: Vec<(NodeIndex, NodeIndex, Graph<NodeWeight, EdgeWeight, Directed>)> = self.vehicles.iter()
            .map(|vehicle| build_dag(vehicle, no_dual.clone(), arc_site_period_duals.clone(), &charge_filters, self.site_sizes.clone())).collect();




        #[cfg(feature = "branching_debug")]
        println!("Active Filters: {:?}", &charge_filters);


        scoped_tracepoint!(_build_initial_rcmp);


        // get forced columns
        let vehicles_with_forced_column = charge_filters.iter().filter_map(|cf : &BranchingFilter| {
            match cf {
                BranchingFilter::MasterMustUseColumn(vehicle_index, filter_visits, true) => {
                    Some(vehicle_index)
                },
                _ => None
            }
        }).collect::<CustomHashSet<&VehicleIndex>>();




        self.cg_model.update(
            self.site_sizes.clone(),
            &self.pattern_pool,
            &self.vehicles,
            charge_filters
        );




        #[cfg(feature = "profiling_enabled")]
        drop(_build_initial_rcmp);

        /*
        |
        -->
         bis her haben wir einen dag für jedes taxi + eine constraint für jede zeit periode die ich an einer site nutzen kann.
            für jedes taxi ist eine covexity constraint angelegt
        */

        let mut last_convexity_dual_cost: IndexMap<&Vehicle, f64> = IndexMap::default();



        // TODO: column generation smoothing
        // let cg_stabilizer = CG_Stabilizer::new();





        loop {

                if self.should_stop.load(Relaxed) {
                    return Err(SolveError::StoppedByExternal);
                }

              //  #[cfg(feature = "column_generation_debug")] {
                #[cfg(feature = "progress_icons")]
                print!("░");

                    io::stdout().flush().ok().expect("Could not flush stdout");
               // }


                // integrate all of the variables into the model.

                #[cfg(feature = "level_print")]
                println!("---- CG Iteration");

                if   {
                    scoped_tracepoint!(_rcmp);
                    self.cg_model.solve()
                } != Status::Optimal {
                    return Err(SolveError::Generic("To Many Vehicles Infeasible"));
                }



                let max_capacity_constraint_duals = self.cg_model.get_capacity_const_duals();
                let vehicle_convexity_duals = self.cg_model.get_vehicle_convexity_const_duals();





                #[cfg(feature = "column_generation_debug")]
                println!("{:?}",&max_capacity_constraint_duals);

                // update the charging station capacity duals in the shared array which is mapped to the dag edges
                for (site, duals) in self.sites.iter().zip(max_capacity_constraint_duals.outer_iter()) {
                    for (period_idx, capacity_dual) in duals.iter().enumerate() {

                        let dual = capacity_dual +
                        // if we have additonal duals from the branching, add them here
                     self.cg_model.get_site_time_branch_const_duals(SiteIndex::new(site), period_idx as Period);

                        arc_site_period_duals[[site.index, period_idx]].set(dual);
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


                // update the vehicle convexity constraint duals

                for (vehicle, dual) in self.vehicles.iter().zip(vehicle_convexity_duals.iter()) {
                    #[cfg(feature = "column_generation_debug")]
                        println!("Updated convexity dual for vehicle {} to {}", vehicle.id, dual);
                    last_convexity_dual_cost.insert(vehicle, *dual);
                }


                let mut did_add_columns = false;
                {

                    scoped_tracepoint!(_inner_pricing_problem);
                    let mut infeasible_counter = 0;
                    for vehicle in &self.vehicles {
                        let (root, destination, ref dag) = vehicle_dags[vehicle.index];

                        let forbidden_columns = charge_filters.iter().filter_map(|cf: &BranchingFilter| {
                            match cf {
                                BranchingFilter::MasterMustUseColumn(filter_vehicle, filter_visits, filter_must_use) => {
                                    if *filter_vehicle != VehicleIndex::new(vehicle) {
                                        None
                                    } else {
                                        // only forbid if must_use = false (== must not use)
                                        if *filter_must_use == false {
                                            None
                                        } else {
                                            Some(filter_visits)
                                        }
                                    }
                                }

                                // all other that are not forbidden columns
                                BranchingFilter::ChargeSegmentSiteTime(_, _, _, _, _) => None,
                                BranchingFilter::ChargeSegmentSite(_, _, _, _) => None,
                                BranchingFilter::OpenSite(_, _) => None,
                                BranchingFilter::OpenSiteGroupMin(_, _) => None,
                                BranchingFilter::OpenSiteGroupMax(_, _) => None,
                                BranchingFilter::MasterNumberOfCharges(_, _, _, _) => None,
                            }
                        });

                        let has_forced_column = vehicles_with_forced_column.contains(&VehicleIndex::new(vehicle));

                        if !has_forced_column {
                            match generate_patterns(vehicle, &dag, root, destination, last_convexity_dual_cost[&vehicle], false, &[]) {
                                Ok(mut path) => {
                                    // loop over all received patterns

                                    if path.len() > 50 {
                                        self.retain_diverse_columns_and_first(&mut path, 50);
                                    }

                                    for (detour_cost, reduced_costs, pattern) in path
                                        .iter() {


                                        // add the pattern to the pool
                                        if let Some(entry) = self.pattern_pool.add_pattern(VehicleIndex::new(vehicle), *detour_cost, pattern.clone()) {
                                            self.cg_model.add_column(VehicleIndex::new(vehicle), entry);
                                            did_add_columns = true;
                                        }
                                    }
                                }
                                Err(e) => {
                                    // cant find single path for vehicle

                                    // if we also have not a single valid pattern configuration must be infeasible
                                    // thus exit early unless we try to find the number of infeasible taxis.
                                    if self.pattern_pool.get_active_patterns(VehicleIndex::new(vehicle), charge_filters, self.site_sizes.clone()).next().is_none() {
                                        infeasible_counter += 1;

                                        if !find_num_infeasible && infeasible_counter > self.allowed_infeasible {
                                            return Err(SolveError::Generic("Has Infeasible over Infeasible Counter"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !did_add_columns {
                    break
                }
            }
        #[cfg(feature = "column_generation_debug")]
        println!();


/*
        #[cfg(feature = "column_generation_debug")] {
            master.write("/tmp/problem_rust.sol").unwrap();
            master.write("/tmp/problem_rust.lp").unwrap();
        }
*/


        let dummy_vars_x = self.cg_model.get_dummy_vars_x();
        let dummy_var_is_set_count = dummy_vars_x.iter().filter(|v| **v > CG_EPSILON).count();
        if dummy_var_is_set_count > self.allowed_infeasible {
            let infeasible_vehicles = dummy_vars_x.iter().zip(self.get_vehicles()).filter_map(|(value,vehicle)| {
                if *value > CG_EPSILON {
                    Some(VehicleIndex::new(vehicle))
                } else {
                    None
                }
            }).collect::<Vec<VehicleIndex>>();

            #[cfg(feature = "column_generation_debug")]
            println!("has {} dummy vars set", dummy_var_is_set_count);
            return Err(SolveError::VehiclesInfeasible(infeasible_vehicles));
        }


        Ok(
            SolvedCGResult {
                master_x: self.cg_model.obj_value(),
                patterns: self.get_vehicles().iter().map(|vehicle| {

                    (VehicleIndex::new(vehicle), self.cg_model.get_chosen_vehicle_patterns(VehicleIndex::new(vehicle)))

                }).collect::<Vec<(VehicleIndex, Vec<(SinglePattern, PatternSelected)>)>>(),

            }
        )
    }

    fn should_branch_be_cut(&mut self, result : &SolvedCGResult) -> bool {



        if let Some(best_int) = self.current_upper_bound {


            #[cfg(feature = "column_generation_exit_early")] {
                // early exit ! if we have any non-fractional solution
                #[cfg(feature = "branching_debug")] {
                    println!("EARLY EXIT: bound {}",best_int);
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
                        println!("IS NEW BEST! {}", result.master_x);
                    }
                    self.current_upper_bound = Some(result.master_x);
                    self.current_best_pattern = Some(result.patterns.iter().map(|(vehicle, patterns)| {
                        (*vehicle, patterns.iter().find(|(_, value)| *value > 0.0).unwrap().0.clone())
                    }).collect());
                }
            } else {
                #[cfg(feature = "branching_debug")] {
                    println!("#####");
                    println!("IS FIRST BEST! {}", result.master_x);
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


    /*
        Take a set of patterns and tests if they violate feasibility requirements
        if all are selected: Must be at most one column per vehicle!
     */
    fn set_of_columns_is_feasible(selected : &[Pattern], site_conf : &SiteConf) -> bool  {
        let mut capacity_map: CustomHashMap<(SiteIndex,Period), u8> = CustomHashMap::new();
        for p in selected {
            for (segment,site,time) in p {
                let visit_counter = capacity_map.entry((*site,*time)).or_insert(0);
                *visit_counter += 1;
                if *visit_counter > site_conf[site.index()] {
                    return false
                }
            }
        }
        true
    }

    #[hawktracer(check_result_for_branching_points)]
    pub fn check_result_for_branching_points(&mut self, parent: &BranchNode, result: &SolvedCGResult) -> Option<Vec<BranchNode>>  {
        #[cfg(feature = "branching_debug")] {
            println!("Checking Result");
            println!("Has {} patterns in the global pool", self.pattern_pool.num_columns());
        }

        // # todo: Pick good ones!







        #[cfg(feature = "infeasibility_events")] {
            use std::fs;
            self.invisibility_event_counter += 1;
            let folder = Path::new("/tmp/infeasibility_events").join(self.invisibility_event_counter.to_string());
            fs::create_dir_all(&folder);
        }

        if self.should_branch_be_cut(result) {
            return None;
        }




        #[cfg(feature = "vistnum_branching")]
        {
            // test, find site level branch
            // calculate the used capacity at every site
            let mut site_period_value_adder: CustomHashMap<(&SiteIndex, Period), f64> = CustomHashMap::default();
            let mut site_period_counter: CustomHashMap<(&SiteIndex, Period), usize> = CustomHashMap::default();

            for (v, patterns) in &result.patterns {
                for (pattern, x) in patterns {
                    for (segment, site, time) in pattern {
                        *(site_period_value_adder.entry((site, *time)).or_insert(0.0)) += x;
                        *(site_period_counter.entry((site, *time)).or_insert(0)) += 1;
                    }
                }
            }

            // pick one with highest fractionality
            let mut min_fractionality = (f64::INFINITY, None);
            for (key, value) in site_period_value_adder
                    .iter().filter(|(_,value)| **value > 1.0) // only get those where were there is no 1/0 decision TODO: Evaluate if this is good!
            {
                let frac = (value.fract() - 0.5).abs();
                if frac < min_fractionality.0 {
                    min_fractionality.0 = frac;
                    min_fractionality.1 = Some(key);
                }
            }

            if  min_fractionality.0 != 0.5  {
                if let Some(chosen_node) = min_fractionality.1 {
                    let (&site, period) = chosen_node;
                    let counter = site_period_counter[chosen_node];
                    let val = site_period_value_adder[chosen_node];

                    println!("Chosen to visitnum branch with value {} (frac {}) consisting of {} patterns", val,(val.fract() - 0.5).abs(), counter);

                    return Some(
                        vec![
                        BranchNode::from_parent(parent,
                                                BranchingFilter::MasterNumberOfCharges(site, *period, Less, DataFloat::from(val.floor())), BranchPriority::Default),
                        BranchNode::from_parent(parent, BranchingFilter::MasterNumberOfCharges(site, *period, Greater, DataFloat::from(val.ceil())), BranchPriority::Default)
                        ])
                    // for now assume that this is a very good cut, therefore immediately return!
                }
            }
            // end find site level branch
        }




        // sort vehicles so that those with many patterns are first ->hopefully good for  diving heuristic
        // TODO: Test
        let mut ord_patterns = result.patterns.clone();
        ord_patterns.sort_unstable_by(|(_,a),(_,b)| {
            let a_count = a.iter().filter(|(p,v)| *v > 0.0).count();
            let b_count = b.iter().filter(|(p,v)| *v > 0.0).count();

            if self.sort_many_columns_first {
                b_count.cmp(&a_count)
            } else {
                a_count.cmp(&b_count)
            }

        });



        // column fixing
        {
            if self.sort_many_columns_first {
                let (vehicle, patterns) = &ord_patterns[0];

                let mut pattern_ord_with_highest_value: Vec<(SinglePattern, PatternSelected)> = patterns.clone();
                pattern_ord_with_highest_value.sort_unstable_by(|a: &(SinglePattern, PatternSelected), b: &(SinglePattern, PatternSelected)| {
                    b.1.partial_cmp(&a.1).unwrap_or_else(|| Ordering::Equal)
                });

                if pattern_ord_with_highest_value[0].1 > 0.0 && pattern_ord_with_highest_value[0].1 < 1.0  {
                    let most_used_pattern = pattern_ord_with_highest_value[0].0.clone();
                    return Some(vec![
                        BranchNode::from_parent(parent, BranchingFilter::MasterMustUseColumn(
                            *vehicle, most_used_pattern.clone(), true
                        ), BranchPriority::Higher),
                         BranchNode::from_parent(parent, BranchingFilter::MasterMustUseColumn(
                             *vehicle, most_used_pattern, false
                         ), BranchPriority::Default
                         )
                        ])

                }
            }
        }



        // look at every vehicle seperately
        for (vehicle, patterns) in &ord_patterns {


// we want to branch on different sites used in a segment first.
            let mut site_segment_combo_count : CustomHashMap<(SegmentId, SiteIndex),u8> = CustomHashMap::default();

            // over all the patterns of the vehicle
            let mut active_patterns_count = 0;

            // now try to identify the location of the fractionality
            for (pattern, value) in patterns {
                // looking only on the activated...
                if *value > 0.0 {
                    // use this for deduplication of segment site in this pattern (ignore time)
                    let mut site_segment_combo: HashSet<(SegmentId, SiteIndex)> = HashSet::default();

                    active_patterns_count += 1;
                    // record that we use a certain site-segment combination in the pattern
                    // (deduplicated, so only count once)
                    for (segment, site, time) in pattern {
                        site_segment_combo.insert((*segment, *site));
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

                return Some(vec![

                    BranchNode::from_parent(parent,
                                            BranchingFilter::ChargeSegmentSite(*vehicle, *chosen_segment, *chosen_site, true),
                                            BranchPriority::Higher),
                    BranchNode::from_parent(parent,
                                            BranchingFilter::ChargeSegmentSite(*vehicle, *chosen_segment, *chosen_site, false),
                                            BranchPriority::Default
                    )
                ]);



                #[cfg(feature = "branching_debug")]
                println!("Vehicle {} has fractional result with {}!", vehicle.index(), value);
                // find branch on the result
                // select a random pattern to branch on


            }
            else
            {
                // check if the fractionality comes from time incompatibilites
                // for every pattern; Record start time of charge at site


                let mut earliest_charge: CustomHashMap<(SegmentId, SiteIndex), Period> = CustomHashMap::default();
                for (pattern, value) in patterns.iter() {
                    if *value > 0.0 {
                        for (segment, site, time) in pattern {
                            let entry = earliest_charge.entry((*segment,*site)).or_insert(*time);
                            if *entry > *time {
                                *entry = *time;
                            }
                        }
                    }
                }

                let mut branch_on : Option<(SegmentId,SiteIndex, Period)> = None;

                'search_loop: for (pattern, value) in patterns {
                    if *value > 0.0 {

                        let mut pattern_earliest_charge: CustomHashMap<(SegmentId, SiteIndex), Period> = CustomHashMap::default();
                        for (segment, site, time) in pattern {
                            let entry = pattern_earliest_charge.entry((*segment,*site)).or_insert(*time);
                            if *entry > *time {
                                *entry = *time;
                            }
                        }


                        for (segment, site, time) in pattern {
                            let entry = earliest_charge[&(*segment,*site)];
                            let pattern_charge = pattern_earliest_charge[&(*segment,*site)];

                            if entry != pattern_charge {
                                branch_on = Some((*segment,*site,entry));
                                break 'search_loop;
                            }
                        }


                    }
                }

                if let Some((segment,site,period)) = branch_on {
                    return Some(vec![
                        BranchNode::from_parent(parent,
                                                BranchingFilter::ChargeSegmentSiteTime(*vehicle, segment, site, period, true),
                                                BranchPriority::Higher),
                        BranchNode::from_parent(parent,
                                                BranchingFilter::ChargeSegmentSiteTime(*vehicle, segment, site, period, false),
                                                BranchPriority::Default
                        )
                    ]);
                }
            }






        }



        return None
    }
}
