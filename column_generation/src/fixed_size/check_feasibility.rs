use indexmap::IndexMap;
use shared::{Site, Segment, Vehicle};
use crate::SiteArray;
use crate::fixed_size::brancher::{Brancher, SolveError};
use crate::fixed_size::site_conf::{SiteConfFactory};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct CheckFeasibility {

}

impl CheckFeasibility{

    pub fn has_feasibility_error<'a>(sites: &'a IndexMap<u8, Site>, _segments_: &'a IndexMap<u32, Segment<'a>>, vehicles: &'a [Vehicle<'a>], num_infeasible_allowed : usize) -> Option<SolveError> {

        let site_array: Vec<Site> = sites.into_iter().map(|(_i,site)| site.clone()).collect();
        let site_conf_factory = SiteConfFactory {
            num_sites : site_array.len()
        };


        // take the maximum sites from the site configuration array as configuration to test
        let mut site_conf = site_conf_factory.empty();
        for (site, conf) in site_array.iter().zip(site_conf.iter_mut()) {
            *conf = site.capacity;
        }

        let mut brancher = Brancher::new(
            site_array,
            vehicles,
            site_conf,
            num_infeasible_allowed,
            Arc::new(AtomicBool::new(false))
        );

        match brancher.solve(false, true) {
            Err(solve_error) => Some(solve_error),
            Ok(_) => None
        }

    }

    pub fn get_potentially_feasible<'a>(sites: &'a IndexMap<u8, Site>, _segments: &'a IndexMap<u32, Segment<'a>>, vehicles: &'a [Vehicle<'a>]) -> Vec<Vehicle<'a>> {
            let site_conf_factory = SiteConfFactory {
                num_sites : sites.len()
            };
            Brancher::get_vehicles_that_can_be_feasible(vehicles,site_conf_factory)
    }
}