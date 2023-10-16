use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter::Sum;

use grb::constr::IneqExpr;
use grb::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use ndarray::Array2;
use petgraph::visit::Walker;
use shared::{charge_time_to_capacity_charge_time, CustomHashMap, CustomHashSet, CustomMultiHashMap, MAX_PERIOD, Period, Segment, Site, Vehicle};
use crate::branching_filter::{BranchingFilter, Dir};
use crate::CG_EPSILON;
use crate::fixed_size::brancher::{DUMMY_COST, SinglePattern};
use crate::fixed_size::site_conf::SiteConf;
use crate::pattern_pool::{Pattern, PatternEntry, PatternPool};

#[derive(Copy,Debug,Clone,Eq, Hash, PartialEq)]
pub struct VehicleIndex(usize);
impl VehicleIndex {
    pub fn new(v : &Vehicle) -> Self {
        VehicleIndex(v.index)
    }
    pub fn index(&self) -> usize {
        self.0
    }
}
#[derive(Copy,Clone,Eq, Hash, PartialEq)]
pub struct ColumnIndex(usize);
#[derive(Copy,Clone,Eq, Hash, PartialEq)]
pub struct SegmentId(u32);
impl SegmentId {
    pub fn index(&self) -> u32 {
        self.0
    }
    pub fn new(v : &Segment) -> Self {
        SegmentId(v.id)
    }
}


#[derive(Copy,Clone,Eq, Hash, PartialEq,Debug)]
pub struct SiteIndex(usize);
impl SiteIndex {
    pub fn index(&self) -> usize {
        self.0
    }
    pub fn new(v : &Site) -> Self {
        SiteIndex(v.index)
    }
}

pub struct CgModel {

    sites : Vec<Site>,

    current_site_sizes : Vec<u8>,

    gurobi_model : RefCell<Model>,

    // initialize hashmaps for model variables and patterns
    vehicle_patterns : CustomMultiHashMap<VehicleIndex,(ColumnIndex, Var,Pattern)>,
    vehicle_convexity: IndexMap<VehicleIndex, Constr>,
    dummy_vars: Vec<Var>,
    constr_max_capacity: Array2<Constr>,

    site_time_branch_constraint : CustomMultiHashMap<(SiteIndex,Period), (BranchingFilter,Constr)>,

    applied_filters : CustomHashSet<BranchingFilter>,
}

impl CgModel {


    pub fn add_column(&mut self, vehicle : VehicleIndex, new_column : &PatternEntry) {


        let id = ColumnIndex(new_column.id);

        let mut coef_vec: Vec<(Constr, f64)> = Vec::with_capacity(new_column.pattern.len()+1);
        // make column use one unit of convexity constraint
        coef_vec.push( (self.vehicle_convexity[vehicle.index()].clone(),1.0));



        // make column use one unit of capacity at every used site
        for (_, site, period) in new_column.pattern.iter() {
            coef_vec.push((self.constr_max_capacity[[site.index(), charge_time_to_capacity_charge_time(period)]].clone(),1.0));
            // register the site_time branch constraints!
            if let Some(entry) = self.site_time_branch_constraint.get_vec(&(*site,*period)) {
                for constr in entry {
                    coef_vec.push((constr.1.clone(),1.0));
                }
            }
        }

        let var_use_pattern = self.gurobi_model.get_mut().add_var(&format!("usePattern[{}]", new_column.id), Continuous, 0.0, 0.0, 1.0, coef_vec).unwrap();
        self.vehicle_patterns.insert(vehicle, (id,var_use_pattern, new_column.pattern.clone()));


    }

    pub fn get_active_columns(&self) -> usize {
        self.vehicle_patterns.iter_all().map(|(_,v)| v.len()).sum()
    }

    pub fn remove_column(&mut self, vehicle : VehicleIndex, column_id : ColumnIndex) {





        let var: Var = self.vehicle_patterns.get_vec(&vehicle).unwrap().iter().filter_map(|(iter_column_id, variable, _pattern)| {
            if iter_column_id.0 == column_id.0 {
                Some(variable.clone())
            } else {
                None
            }
        }).next().expect(&format!("Tried to remove a column {} on vehicle {} that was not added before", column_id.0, vehicle.0));


        self.vehicle_patterns.get_vec_mut(&vehicle).unwrap().retain(|(id,_,_)| if id.0 == column_id.0 { false} else { true });
        self.gurobi_model.get_mut().remove(var).unwrap();


    }


    pub fn update_active_columns(&mut self, vehicle : VehicleIndex, should_be_active: &[&PatternEntry]) {


        let mut added_columns = 0;

        let active_columns : HashSet<ColumnIndex> =  {
            if let Some(data) = self.vehicle_patterns.get_vec(&vehicle) {
                data.iter().map(|(id,_,_)| id.clone()).collect()
            } else {
                HashSet::default()
            }
        };

            for col in should_be_active {
                if !active_columns.contains(&ColumnIndex(col.id)) {
                    // add column that was not active before
                    self.add_column(vehicle, col);
                    added_columns+=1;
                }
            }



        let mut columns_to_remove = Vec::default();
        for active in active_columns {
            if should_be_active.iter().filter(|c| ColumnIndex(c.id) == active).next().is_none() {
                columns_to_remove.push(active.clone());
            }
        }


        //println!("Added {} and removed {} columns",added_columns, columns_to_remove.len());

        for col in columns_to_remove {
            self.remove_column(vehicle, col);
        }





    }


    pub fn update_site_capacities(&mut self, new_site_sizes: Vec<u8>){

        for ((old_size,new_size), constraints) in self.current_site_sizes.iter().zip(new_site_sizes.iter()).zip(self.constr_max_capacity.outer_iter()) {
            if old_size != new_size {
                self.gurobi_model.get_mut().set_obj_attr_batch(attr::RHS,
                                                               constraints.into_iter().cloned().zip(vec![*new_size as f64; constraints.len()])
                ).unwrap();
            }
        }

        self.current_site_sizes = new_site_sizes;

    }



    pub fn add_filter(&mut self, filter : &BranchingFilter) {

        // just initalize those that we have in the branching constraint
        if let BranchingFilter::MasterNumberOfCharges(site, period, direction, value) = filter {


            // find current patterns that are connected to current filter
            let mut active_capacities : Vec<Var> = Vec::default();
            self.vehicle_patterns.iter_all().for_each(|(vehicle,patterns)| {
                patterns.iter().for_each(|(_,var,pattern)| {
                    for (p_segment,p_site,p_period) in pattern {
                        if site == p_site && period == p_period {
                            active_capacities.push(var.clone());
                            break;
                        }
                    }
                });
            });



            self.site_time_branch_constraint.insert((*site, *period),
                                                    (filter.clone(),
                                                    self.gurobi_model.get_mut().add_constr(
                                                        &format!("branchCapacity[{},{},{}]", site.index(), period, value.float()),
                                                        IneqExpr{
                                                            lhs: Expr::sum(active_capacities.iter()),
                                                            sense: match direction {
                                                                Dir::Less => ConstrSense::Less,
                                                                Dir::Greater => ConstrSense::Greater,
                                                            },
                                                            rhs:  Expr::Constant(value.float())
                                                        }
                                                    ).unwrap()
                                                    )
            );
        }

        self.applied_filters.insert(filter.clone());
    }


    pub fn remove_filter(&mut self, filter : &BranchingFilter) {

        if let BranchingFilter::MasterNumberOfCharges(site, period, direction, value) = filter {

            let mut constrs = self.site_time_branch_constraint.get_vec_mut(&(*site, *period)).unwrap();

            {
                for (constr_filter, constr) in constrs.iter() {
                    if constr_filter == filter {
                        self.gurobi_model.get_mut().remove(constr.clone()).unwrap();
                        break;
                    }
                }
            }

            constrs.retain(|(constr_filter,_c)| constr_filter != filter);


        }



        self.applied_filters.remove(filter);
    }

    pub fn sync_filters(&mut self, should_filters: &[BranchingFilter]) {


        // add filters that are new
        for filter in should_filters {
            if !self.applied_filters.contains(filter) {
                self.add_filter(filter);
            }
        }


        // remove filters that are old
        let filters_to_remove : Vec<BranchingFilter> = self.applied_filters.iter().filter(|filter| !should_filters.contains(filter)).cloned().collect();
        for filter in filters_to_remove {
                self.remove_filter(&filter);
        }

    }



    pub fn update<'a>(&mut self, site_sizes : SiteConf, pattern_pool : &PatternPool,  vehicles : &'a [Vehicle], charge_filters : &[BranchingFilter]) {

        self.model_update();

        for vehicle in vehicles {
            // add the existing patterns from the pool
            {
                let patterns = pattern_pool.get_active_patterns(VehicleIndex::new(vehicle), charge_filters, site_sizes.clone());
                self.update_active_columns(VehicleIndex::new(vehicle), &patterns.collect::<Vec<&PatternEntry>>());
            }
        }

        self.update_site_capacities(site_sizes);
        self.sync_filters(charge_filters);




    }

    pub fn model_update(&mut self)  {
        self.gurobi_model.get_mut().update().unwrap();
    }
    pub fn solve(&mut self)  -> Status {
        self.gurobi_model.get_mut().optimize().unwrap();
        return self.gurobi_model.get_mut().status().unwrap();
    }

    pub fn get_capacity_const_duals(&self) -> Array2<f64> {
        // collect the duals of the convexity from gurobi c api
        Array2::from_shape_vec((self.sites.len(), MAX_PERIOD),
                               self.gurobi_model.borrow().get_obj_attr_batch(attr::Pi, self.constr_max_capacity.clone()
                               ).unwrap()).unwrap()
    }

    pub fn get_site_time_branch_const_duals(&self, site : SiteIndex, period : Period) -> f64 {
        if let Some(entry) = self.site_time_branch_constraint.get_vec(&(site, period)) {
            self.gurobi_model.borrow().get_obj_attr_batch(attr::Pi, entry.iter().map(|(_,constr)| constr.clone())).unwrap().into_iter().sum()
        } else { 0.0 }
    }

    pub fn get_vehicle_convexity_const_duals(&self)  -> Vec<f64> {
        self.gurobi_model.borrow().get_obj_attr_batch(attr::Pi, self.vehicle_convexity.values().cloned().collect::<Vec<Constr>>()).unwrap()
    }


    pub fn obj_value(&self) -> f64 {
        self.gurobi_model.borrow().get_attr(attr::ObjVal).unwrap()
    }
    pub fn get_dummy_vars_x(&self) -> Vec<f64> {
        self.gurobi_model.borrow().get_obj_attr_batch(attr::X, self.dummy_vars.clone()).unwrap()
    }


    pub fn get_chosen_vehicle_patterns(&self, vehicle : VehicleIndex) -> Vec<(SinglePattern,f64)> {


        if let Some(patterns) = self.vehicle_patterns.get_vec(&vehicle) {
            let solution_values = self.gurobi_model.borrow().get_obj_attr_batch(attr::X, patterns.iter().map(|(id,var, _)| var.clone()).collect::<Vec<Var>>()).unwrap();

             patterns.iter()
                 .map(|(id,_, pattern)| pattern.clone())
                 .zip(solution_values)
                 .filter(|(_, value)| *value > 0.0)
                 .collect::<Vec<(SinglePattern, f64)>>()

        } else {
            // no patterns
            Vec::new()
        }

    }


    pub fn new(env : &Env, sites : Vec<Site>, site_sizes : Vec<u8>, vehicles : Vec<Vehicle>) -> Self {

        let mut model = Model::with_env("master", env).unwrap();

        // setup site constaints
        let mut site_constraints = Vec::with_capacity(sites.len() * MAX_PERIOD);
        for site in &sites {
            for p in 0..MAX_PERIOD {
                site_constraints.push(
                    model.add_constr(
                        &format!("maxCapacity[{},{},{}]", site.id, site.index, p),c!(Expr::default() <= f64::from(site_sizes[site.index]))
                    ).unwrap());
            }
        }
        let constr_max_capacity: Array2<Constr> = Array2::from_shape_vec((sites.len(), MAX_PERIOD), site_constraints).unwrap();


        // setup vehicles
        let mut dummy_vars = Vec::default();
        let mut vehicle_convexity = IndexMap::default();
        for vehicle in &vehicles {
            let dummy_var = model.add_var(&format!("dummy_column[{}]", vehicle.id), Continuous, DUMMY_COST, 0.0, 1.0, []).unwrap();
            let constr_convexity = model.add_constr(&format!("convexity[{}]", vehicle.id), c!(1.0 * dummy_var == 1.0)).unwrap();
            dummy_vars.push(dummy_var);
            vehicle_convexity.insert(VehicleIndex::new(vehicle), constr_convexity.clone());
        }




        CgModel {
            sites,
            gurobi_model : RefCell::new(model),
            vehicle_patterns : CustomMultiHashMap::default(),
            vehicle_convexity,
            dummy_vars,
            constr_max_capacity : constr_max_capacity,
            site_time_branch_constraint : CustomMultiHashMap::default(),
            applied_filters : CustomHashSet::default(),
            current_site_sizes : site_sizes
        }
    }

    pub fn copy(&self) -> Self {

        let mut model = self.gurobi_model.borrow_mut();
        model.update().unwrap();

        let mut cloned_model = model.try_clone().unwrap();
        cloned_model.update().unwrap();


        let mut constr_map : CustomHashMap<Constr,Constr> = CustomHashMap::default();

        let new_constr = cloned_model.get_constrs().unwrap().to_vec();
        let old_constr = model.get_constrs().unwrap();


        assert_eq!(
            cloned_model.get_obj_attr(attr::ConstrName, &new_constr[0]).unwrap(),
            model.get_obj_attr(attr::ConstrName, &old_constr[0]).unwrap()
        );

        old_constr.into_iter().zip(new_constr.into_iter()).for_each(|(old,new)| {
           constr_map.insert(*old,new);
        });




        let new_vars = cloned_model.get_vars().unwrap().to_vec();
        let old_vars = model.get_vars().unwrap();
        let mut var_map : CustomHashMap<Var,Var> = CustomHashMap::default();


        assert_eq!(
            cloned_model.get_obj_attr(attr::VarName, &new_vars[0]).unwrap(),
            model.get_obj_attr(attr::VarName, &old_vars[0]).unwrap()
        );

        old_vars.into_iter().zip(new_vars.into_iter()).for_each(|(old,new)| {
            var_map.insert(*old,new);
        });

        let mut new_constr_max_capacity = self.constr_max_capacity.clone();
        new_constr_max_capacity.iter_mut().for_each(|e| *e = constr_map[e]);

        let mut new_vehicle_convexity = self.vehicle_convexity.clone();
        new_vehicle_convexity.iter_mut().for_each(|(_,c)| *c = constr_map[c] );



        let mut new_site_time_branch_constraint = self.site_time_branch_constraint.clone();
        new_site_time_branch_constraint.iter_all_mut().for_each(|(_k,cv)| cv.iter_mut().for_each(|c| (*c).1 = constr_map[&c.1]));




        let new_dummy_vars : Vec<Var> = self.dummy_vars.iter().map(|v| var_map[v]).collect();

        let mut new_vehicle_patterns = self.vehicle_patterns.clone();
        new_vehicle_patterns.iter_all_mut().for_each(|(_,arr)| arr.iter_mut().for_each(|(_,var,_)| *var = var_map[var]));


        CgModel {
            sites: self.sites.clone(),
            current_site_sizes: self.current_site_sizes.clone(),
            gurobi_model : RefCell::new(cloned_model),

            applied_filters: self.applied_filters.clone(),


            // vars
            vehicle_patterns : new_vehicle_patterns,
            dummy_vars : new_dummy_vars,

            // constr
            vehicle_convexity : new_vehicle_convexity,
            constr_max_capacity : new_constr_max_capacity,
            site_time_branch_constraint: new_site_time_branch_constraint,

        }
    }







}


