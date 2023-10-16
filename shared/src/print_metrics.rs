use std::time::Duration;
extern crate dipstick;
use dipstick::*;

pub fn setup_metrics_printer() {
    let metrics = AtomicBucket::new();

    metrics.flush_every(Duration::from_secs(10));
    metrics.drain(Stream::write_to_stdout());
    metrics.stats(dipstick::stats_all);

    dipstick::Proxy::default_target(metrics);
}
