use crate::site::Site;
use crate::Period;

#[derive(Debug,Clone,Copy)]
pub struct ReachableSite<'a> {
    pub site: &'a Site,
    pub arrival_time: Period,
    pub departure_time: Period,
    pub distance_to: u32,
    pub distance_from: u32,
}
