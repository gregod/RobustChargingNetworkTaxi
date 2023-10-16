extern crate dipstick;
use dipstick::*;

metrics! {
    pub COLUMNS_GENERATED : Marker  = "columns_generated";
    pub LPS_SOLVED : Marker = "master_lps_solved";
    pub CUT_IMPROVEMENT_FAILED : Marker = "cut_improvement_failed";
    pub CUT_IMPROVEMENT_HELPED : Marker = "cut_improvement_helped";
    pub TIME_IN_BRANCHER_SOLVE : Timer = "time_in_brancher_solve";
}

/*
Custom Timer Object, that records its time from creation until its dropped
*/

pub struct ConfigurationTimer {
    handle : dipstick::TimeHandle
}
impl ConfigurationTimer {
    pub fn new() -> Self {
        ConfigurationTimer {
            handle :TIME_IN_BRANCHER_SOLVE.start()
        }
    }
}
impl Drop for ConfigurationTimer {
    fn drop(&mut self) {
        TIME_IN_BRANCHER_SOLVE.stop(self.handle);
    }
}