
use std::sync::atomic::{AtomicUsize, Ordering};
use shared::{Segment, Site, Period, Vehicle };

#[cfg(feature = "perf_statistics")]
use crate::metrics::*;
use crate::fixed_size::site_conf::SiteConf;
use crate::branching_filter::BranchingFilter;


type Pattern<'a> = Vec<(&'a Segment<'a>,&'a Site, Period)>;
static COLUMN_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct PatternEntry<'a> {
    pub id : usize,
    pub used : usize,
    pub cost : f64,
    pub pattern : Pattern<'a>
}

pub struct PatternPool<'a> {
    store : Vec<Vec<PatternEntry<'a>>>,
    entry_count : usize,
    attempts_at_same_column : usize
}

impl <'a> Clone for PatternPool<'a> {
    fn clone(&self) -> Self {
        PatternPool {
            store : self.store.clone(),
            entry_count : self.entry_count,
            attempts_at_same_column : self.attempts_at_same_column
        }
    }
}

impl <'a> PatternPool<'a> {


    pub fn new(num_vehicles : usize) -> PatternPool<'a>{
        PatternPool {
            store : vec![Vec::new(); num_vehicles],
            entry_count : 0,
            attempts_at_same_column : 0
        }
    }

    pub fn num_columns(&self) -> usize {
        self.entry_count
    }

    pub fn report_ok_pattern(&mut self, _vehicle : &'a Vehicle<'a>, _pattern_id : usize){
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


    pub fn add_pattern(&mut self, vehicle : &'a Vehicle<'a>, distance_cost : f64, pattern : Pattern<'a>) -> &PatternEntry {
        // println!("Adding pattern {:?} to pool", pattern);
        #[cfg(feature = "perf_statistics")]
            COLUMNS_GENERATED.mark();

        // test if patterm exists:
        // only enable in feasibility debug check mode
        #[cfg(feature = "column_generation_validate")]
            {
                'patternLoop: for entry in &self.store[vehicle.index] {
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
                    break;
                }


            }

        self.entry_count += 1;
        self.store[vehicle.index].push(PatternEntry { id : COLUMN_ID_COUNTER.fetch_add(1, Ordering::SeqCst), used : 1, cost : distance_cost, pattern });
        self.store[vehicle.index].last().unwrap()

    }

    pub fn would_be_infeasible_when_removing_site(&self,vehicle : &'a Vehicle<'a>,  existing_filters : Vec<BranchingFilter<'a>>, site_conf : SiteConf, branch_site : &Site) -> usize {
        // search get number of patterns under existing filters that would be infeasible when forcing site to zero
        // or to one.
       self.get_active_patterns(vehicle, existing_filters, site_conf).filter(|entry| {
               entry.pattern.iter().any(|(_,site,_)| {
                   *site == branch_site
               })
       }).count()

    }

    pub fn get_patterns(&self, vehicle : &'a Vehicle<'a>, pattern_filter : Vec<BranchingFilter<'a>>, site_conf : SiteConf) -> impl Iterator<Item = (&PatternEntry<'a>,bool)>{


        self.store[vehicle.index].iter().map(move |entry| {

            // remove patterns that apply to non existant sites! (either through branching or through site config)
            if site_conf.iter().any(|el| *el == 0 ) {
                for (_, site, _) in entry.pattern.iter() {
                    if site_conf[site.index] == 0 {
                        return (entry,false);
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
                                return (entry,false);
                            }
                        }
                    },
                    BranchingFilter::ChargeSegmentSite(filter_vehicle, segment, site, filter_requires_use) => {
                        if *filter_vehicle == vehicle {
                            // now that this applies to this vehicle, check if filter is contrary to pattern
                            let pattern_uses = entry.pattern.iter().any(|(row_segment, row_site, _)| row_segment == segment && row_site == site);
                            if *filter_requires_use && !pattern_uses || !*filter_requires_use && pattern_uses {
                                return (entry,false);
                            }
                        }
                    },
                    BranchingFilter::OpenSite(site_filter,filter_says_is_open) => {
                        // if we forbid this site, skip all patterns that contain the site
                        if *filter_says_is_open == false {
                            for (_,site,_) in entry.pattern.iter() {
                                if **site_filter == **site {
                                    return (entry,false);
                                }
                            }
                        }
                    },
                    BranchingFilter::OpenSiteGroupMax(sites_filter,num) => {
                        if *num <= std::f64::EPSILON {
                            // filter relevant since we are setting all sizes to zero
                            for (_, site, _) in entry.pattern.iter() {
                                for site_filter in sites_filter.iter() {
                                    if **site_filter == **site {
                                        return (entry,false);
                                    }
                                }
                            }
                        }
                    }


                    BranchingFilter::OpenSiteGroupMin(_,_) => ()  // group min does not influence pattern directly
                }
            }

            (entry,true)
        })
    }



    pub fn get_active_patterns(&self, vehicle : &'a Vehicle<'a>, pattern_filter : Vec<BranchingFilter<'a>>, site_conf : SiteConf) -> impl Iterator<Item = &PatternEntry<'a>> + '_{

        self.store[vehicle.index].iter().filter( move|entry| {

            // remove patterns that apply to non existant sites! (either through branching or through site config)
            if site_conf.iter().any(|el| *el == 0 ) {
                for (_, site, _) in entry.pattern.iter() {
                    if site_conf[site.index] == 0 {
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
                                if **site_filter == **site {
                                    return false;
                                }
                            }
                        }
                    },
                    BranchingFilter::OpenSiteGroupMax(sites_filter,num) => {
                        if *num <= std::f64::EPSILON {
                            // filter relevant since we are setting all sizes to zero
                            for (_, site, _) in entry.pattern.iter() {
                                for site_filter in sites_filter.iter() {
                                    if **site_filter == **site {
                                        return false;
                                    }
                                }
                            }
                        }
                    }


                    BranchingFilter::OpenSiteGroupMin(_,_) => ()  // group min does not influence pattern directly
                }
            }

            true
        })
    }

}
