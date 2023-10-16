use crate::{Segment, Simple, Site, Vehicle};
use indexmap::IndexMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait SolutionMethod {
    fn run<'a>(
        &self,
        sites: &'a IndexMap<u8, Site>,
        segments: &'a IndexMap<u32, Segment<'a>>,
        vehicles: &'a [Vehicle<'a>],
        should_stop: Arc<AtomicBool>,
    ) -> Simple;
}
