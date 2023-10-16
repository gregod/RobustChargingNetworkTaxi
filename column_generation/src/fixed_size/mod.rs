
pub const UPPER_BOUND_COST : u32 = 9_999_999;

pub mod brancher;
//pub mod benders;
pub mod check_feasibility;
pub mod site_conf;
//pub mod benders_robust; Disabled: "Old robust" functionality is now included in benders_variable
pub mod benders_variable;
pub mod cg_model;
mod scenario_manager;
pub mod policy_feasibility;
pub mod simulation_feasibility;