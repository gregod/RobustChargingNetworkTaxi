#![warn(clippy::all)]

mod battery;
pub use battery::Battery;

mod location;
pub use location::Location;

mod reachable_site;
pub use reachable_site::ReachableSite;

mod segment;
pub use segment::Segment;

mod site;
pub use site::Site;
mod vehicle;

pub use vehicle::Vehicle;

mod custom_hashmap;
pub use custom_hashmap::CustomHashMap;
pub use custom_hashmap::CustomHashSet;
pub use custom_hashmap::CustomMultiHashMap;

mod solution_method;
pub use solution_method::SolutionMethod;

mod solution;
pub use solution::Simple;

#[cfg(feature = "perf_statistics")]
mod print_metrics;
use flate2::read::GzDecoder;
#[cfg(feature = "perf_statistics")]
pub use print_metrics::setup_metrics_printer;
use std::fs::File;
use std::io::Read;

extern crate csv;
extern crate indexmap;
extern crate regex;

pub type Period = u16;
pub const MAX_PERIOD: usize = 288;
pub const MIN_PER_PERIOD: u8 = 5;

pub fn charge_time_to_capacity_charge_time(charge_time: &Period) -> usize {
    usize::from(charge_time % (24 * 60 / MIN_PER_PERIOD as u16))
}

pub fn get_reader(path: &str) -> Box<dyn Read> {
    if path.ends_with(".gz") {
        Box::new(GzDecoder::new(File::open(path).unwrap()))
    } else {
        Box::new(File::open(path).unwrap())
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
            println!($($arg)*);
    };
}
