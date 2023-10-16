use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufWriter};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use shared::{Segment, Site, Period, Vehicle, CustomHashSet};

#[cfg(feature = "perf_statistics")]
use crate::metrics::*;
use crate::fixed_size::site_conf::SiteConf;
use crate::branching_filter::{BranchingFilter, DataFloat, Dir};
use crate::branching_filter::BranchingFilter::MasterNumberOfCharges;
use crate::fixed_size::cg_model::{SegmentId, SiteIndex, VehicleIndex};
use rust_hawktracer::*;

pub type Pattern = Vec<(SegmentId, SiteIndex, Period)>;
static COLUMN_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct PatternEntry {
    pub id : usize,
    pub used : usize,
    pub cost : f64,
    pub pattern : Pattern
}

pub struct PatternPool{
    store : Vec<Vec<PatternEntry>>,
    entry_count : usize,
    attempts_at_same_column : usize
}

impl <'a> Clone for PatternPool {
    fn clone(&self) -> Self {
        PatternPool {
            store : self.store.clone(),
            entry_count : self.entry_count,
            attempts_at_same_column : self.attempts_at_same_column
        }
    }
}

impl <'b> PatternPool {


    pub fn new(num_vehicles : usize) -> Self{
      Self::new_with_store(vec![Vec::new(); num_vehicles])
    }

    pub fn new_with_store(store : Vec<Vec<PatternEntry>>) -> Self {
        PatternPool {
            store,
            entry_count : 0,
            attempts_at_same_column : 0
        }
    }


    pub fn store_at_index(&self, idx : usize) -> &Vec<PatternEntry> {
        &self.store[idx]
    }

    pub fn num_columns(&self) -> usize {
        self.entry_count
    }

    pub fn report_ok_pattern(&mut self, _vehicle : VehicleIndex, _pattern_id : usize){
        //self.store[vehicle.index][pattern_id].used += 1;
    }

    pub fn clean_patterns(&mut self) {


        if self.entry_count < 100_000 {
            return;
        }

        let mut items = self.store.iter().flat_map(|el| el.iter().map( |entry| entry.used)).collect::<Vec<usize>>();
        items.sort_unstable();
        let cutoff = items[items.len() / 50];



        let mut removed = 0;
        for el in self.store.iter_mut() {
            let count_before = el.len();
            el.retain(|item| item.used > cutoff );
            for item in el.iter_mut() {
                item.used = 0;
            }
            let delta = count_before - el.len();
            removed += delta;
        }

        println!("MAINT {} columns of {} removed",removed,self.entry_count);
        self.entry_count -= removed;


    }

    pub fn write_to_disk(&self, path : &PathBuf, vehicle_ordered_by_index : &[Vehicle]) {
        use std::io::Write;
        use itertools::Itertools;
        if path.to_str().unwrap() != "/dev/null" {
            let write_file = File::create(path).unwrap();
            let mut writer = BufWriter::new(&write_file);

            for (column, vehicle) in self.store.iter().zip(vehicle_ordered_by_index) {
                for entry in column {
                    let patterns = entry.pattern.iter().map(|(segment, site, time)| {
                        format!("{},{},{}", segment.index(), site.index(), time)
                    }).join(";");
                    write!(&mut writer, "{}|{}|{}\n", vehicle.original_id, entry.cost, patterns).unwrap();
                }
            }
        }
    }

    pub fn read_from_disk(&mut self,path : PathBuf, vehicle_ordered_by_index : Vec<Vehicle>, sites : &[Site] ) {
        if path.to_str().unwrap() != "/dev/null" {
            let file = File::open(path).unwrap();
            let mut lines = io::BufReader::new(file).lines();
            // first line is bound
            for line in lines {
                if let Ok(str_line) = line {

                    // format is vehicle_id|segment,site,time;segment,site,time...
                    let mut m = str_line.split("|");
                    let vehicle_id = m.next().unwrap().parse::<u32>().unwrap();
                    let distance_cost = m.next().unwrap().parse::<f64>().unwrap();
                    let remainder = m.next().unwrap();

                    let vehicle = vehicle_ordered_by_index.iter().filter(|v| v.original_id == vehicle_id).next().unwrap();

                    let entry: Pattern = remainder.split(";").map(|entry| {
                        let mut m = entry.split(",");
                        let segment_id = m.next().unwrap().parse::<u32>().unwrap();
                        let site_id = m.next().unwrap().parse::<u8>().unwrap();
                        let time = m.next().unwrap().parse::<Period>().unwrap();
                        let site = sites.iter().filter(|s| s.id == site_id).next().unwrap();
                        let segment = vehicle.tour.iter().filter(|s| s.id == segment_id).next().unwrap();
                        (SegmentId::new(segment), SiteIndex::new(site), time)
                    }).collect();

                    self.add_pattern(VehicleIndex::new(vehicle), distance_cost, entry);
                }
            }
        }
    }


    #[hawktracer(cg_add_pattern)]
    pub fn add_pattern(&mut self, vehicle : VehicleIndex, distance_cost : f64, pattern : Pattern) -> Option<&PatternEntry> {
        // println!("Adding pattern {:?} to pool", pattern);
        #[cfg(feature = "perf_statistics")]
            COLUMNS_GENERATED.mark();

        // test if patterm exists:
        // only enable in feasibility debug check mode
        #[cfg(feature = "column_generation_validate")]
            {
                'patternLoop: for entry in &self.store[vehicle.index()] {
                    if entry.pattern.len() !=pattern.len() {
                        continue;
                    }
                    for new_entry in &pattern {
                        if !entry.pattern.contains(&new_entry) {
                            continue 'patternLoop;
                        }
                    }

                    self.attempts_at_same_column += 1;
                    println!("IS SAME PATTERN! NOW AT {}", self.attempts_at_same_column);
                    panic!("SAME PATTERN");
                    break;
                }


            }

        {

            let new_entry : CustomHashSet<(SegmentId, SiteIndex, Period)>  = pattern.iter().cloned().collect();
            // test if i dominate any other / any other dominates me

            let mut accept_new_column = true;
            self.store[vehicle.index()].retain(|entry| {
                let existing_entry: CustomHashSet<(SegmentId, SiteIndex, Period)> = entry.pattern.iter().cloned().collect();
                if new_entry.is_subset(&existing_entry) {
                    return false;
                } else if existing_entry.is_subset(&new_entry) {
                    accept_new_column = false;
                    return true;
                }
                return true;
            });
            if accept_new_column == false {
                return  None;
            }
        }



        self.entry_count += 1;
        self.store[vehicle.index()].push(PatternEntry { id : COLUMN_ID_COUNTER.fetch_add(1, Ordering::SeqCst), used : 1, cost : distance_cost, pattern });
        Some(self.store[vehicle.index()].last().unwrap())

    }

    pub fn would_be_infeasible_when_removing_site(&self,vehicle :VehicleIndex,  existing_filters : Vec<BranchingFilter>, site_conf : SiteConf, branch_site : &SiteIndex) -> usize {
        // search get number of patterns under existing filters that would be infeasible when forcing site to zero
        // or to one.
       self.get_active_patterns(vehicle, &existing_filters, site_conf).filter(|entry| {
               entry.pattern.iter().any(|(_,site,_)| {
                   *site == *branch_site
               })
       }).count()

    }



    pub fn get_pattern_count_for_configuration(&self,site_conf : SiteConf) -> usize {
        self.store.iter().map(|p| p.iter().filter(|el  | {
            for (_, site, _) in el.pattern.iter() {
                if site_conf[site.index()] == 0 {
                    return false;
                }
            }
            return true;
        }).count()).sum()
    }

    pub fn get_active_patterns<'a>(&'a self, vehicle : VehicleIndex, pattern_filter : &'a [BranchingFilter], site_conf : SiteConf) -> impl Iterator<Item = &'a PatternEntry> + '_{

        self.store[vehicle.index()].iter().filter( move|entry| {

            // remove patterns that apply to non existent sites! (either through branching or through site config)
            if site_conf.iter().any(|el| *el == 0 ) {
                for (_, site, _) in entry.pattern.iter() {
                    if site_conf[site.index()] == 0 {
                        return false;
                    }
                }
            }

            for filter  in pattern_filter.iter() {
                match filter {
                    BranchingFilter::ChargeSegmentSiteTime(filter_vehicle, segment, site, time, filter_requires_usage) => {
                        if *filter_vehicle == vehicle {
                            // now that this applies to this vehicle, check if filter is contrary to pattern
                            if (*filter_requires_usage && !entry.pattern.contains(&(*segment, *site, *time))) ||
                                (!*filter_requires_usage && entry.pattern.contains(&(*segment, *site, *time))) {
                                return false;
                            }
                        }
                    },
                    BranchingFilter::ChargeSegmentSite(filter_vehicle, segment, site, filter_requires_use) => {
                        if *filter_vehicle == vehicle {
                            // now that this applies to this vehicle, check if filter is contrary to pattern
                            let pattern_uses = entry.pattern.iter().any(|(row_segment, row_site, _)| row_segment == segment && row_site == site);
                            if *filter_requires_use && !pattern_uses || !*filter_requires_use && pattern_uses {
                                return false;
                            }
                        }
                    },
                    BranchingFilter::OpenSite(site_filter,filter_says_is_open) => {
                        // if we forbid this site, skip all patterns that contain the site
                        if *filter_says_is_open == false {
                            for (_,site,_) in entry.pattern.iter() {
                                if *site_filter == *site {
                                    return false;
                                }
                            }
                        }
                    },
                    BranchingFilter::OpenSiteGroupMax(sites_filter,num) => {
                        if *num <= DataFloat::epsilon() {
                            // filter relevant since we are setting all sizes to zero
                            for (_, site, _) in entry.pattern.iter() {
                                for site_filter in sites_filter.iter() {
                                    if *site_filter == *site {
                                        return false;
                                    }
                                }
                            }
                        }
                    }

                    BranchingFilter::MasterNumberOfCharges(filter_site,filter_period,filter_dir,filter_val) => {
                        // kill those columns that charge at forbidden sites
                        if *filter_val == DataFloat::zero() && matches!(filter_dir,Dir::Less) {
                            // we disable charging at a site
                            for (_,site,time) in entry.pattern.iter() {
                                if *filter_site == *site && filter_period == time {
                                    return false;
                                }
                            }
                        }
                    }

                    BranchingFilter::MasterMustUseColumn(filter_vehicle,filter_pattern,filter_must_use) => {
                        // if our vehicle is affected
                        if vehicle == *filter_vehicle  {
                             // and the pattern matches
                            if entry.pattern == *filter_pattern  {
                                // and we must not use it => remove pattern from return
                                if *filter_must_use == false {
                                    return  false;
                                }
                            } else {
                                // if it doesn't match and we must use something else => remove pattern
                                if *filter_must_use == true {
                                    return  false;
                                }
                            }
                        }
                    },

                    BranchingFilter::OpenSiteGroupMin(_,_) => (),  // group min does not influence pattern directly


                }
            }

            true
        })
    }

}
