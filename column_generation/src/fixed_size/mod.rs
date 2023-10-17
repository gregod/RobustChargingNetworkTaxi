
pub const UPPER_BOUND_COST : u32 = 9_999_999;

pub mod brancher;

pub mod check_feasibility;
pub mod site_conf;
//pub mod solution_approach;
//pub mod solution_approach_robust; Disabled: "Old" functionality is now included in _variable
pub mod solution_approach_variable;
pub mod cg_model;
mod scenario_manager;
pub mod policy_feasibility;
pub mod simulation_feasibility;