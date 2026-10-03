//! exact_cache_single_flight_surface_smoke.c: the engine's default
//! single-flight options, which sidereon_exact_cache_single_flight_options_init
//! reports in milliseconds.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::exact_cache::ExactCacheSingleFlightOptions;

fn main() {
    let guard = "SIDEREON_W6_EXACT_CACHE_PINS_H";
    valgen::header_start("w6_exact_cache", guard);
    let defaults = ExactCacheSingleFlightOptions::default();
    comment("ExactCacheSingleFlightOptions::default(), in milliseconds.");
    def(
        "W6_EXACT_CACHE_POLL_INTERVAL_MS",
        format!("UINT64_C({})", defaults.poll_interval.as_millis()),
    );
    def(
        "W6_EXACT_CACHE_HEARTBEAT_INTERVAL_MS",
        format!("UINT64_C({})", defaults.heartbeat_interval.as_millis()),
    );
    def(
        "W6_EXACT_CACHE_LIVENESS_TIMEOUT_MS",
        format!("UINT64_C({})", defaults.liveness_timeout.as_millis()),
    );
    def(
        "W6_EXACT_CACHE_WAIT_TIMEOUT_MS",
        format!("UINT64_C({})", defaults.wait_timeout.as_millis()),
    );
    valgen::header_end(guard);
}
