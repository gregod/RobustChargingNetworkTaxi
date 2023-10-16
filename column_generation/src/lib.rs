#![feature(int_roundings)]
#![feature(cell_update)]
#![deny(clippy::all)]
#![allow(clippy::bool_comparison)]
#![allow(clippy::type_complexity)]
#![allow(clippy::block_in_if_condition_stmt)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unreachable_patterns)]
#![allow(unreachable_code)]

pub mod fixed_size;

mod pattern_pool;
mod dag_builder;

mod branching_filter;
#[cfg(feature = "perf_statistics")]
mod metrics;
mod rcsp;


extern crate shared;
extern crate typed_arena;
extern crate ndarray;
extern crate petgraph;
extern crate grb;
extern crate chrono;
extern crate rand;
extern crate crossbeam;
extern crate crossbeam_utils;
#[macro_use] extern crate rust_hawktracer;
extern crate core;


use shared::{Site, Segment, Period};
use crate::fixed_size::cg_model::{SegmentId, SiteIndex};


const CG_EPSILON : f64 = 1e-6_f64;




pub type SiteArray<'a> = Vec<&'a Site>;
pub type SiteArrayRef<'a> = &'a [&'a Site];



fn format_pattern (pattern : &[(SegmentId,SiteIndex,Period)]) -> String {
    pattern.iter().map(|(_,site,time) | format!("s{}@{}",site.index(),time)).fold(String::new(), |acc, l| acc + "->" + &l)
}
