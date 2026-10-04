//! smoke.c exercise_timescale_surface: sidereon-core's inter-scale offsets
//! for each (from, to) pair the test queries, and whether it refuses the
//! query.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::time::{timescale_offset_at_s, timescale_offset_s, TimeScale};
use valgen::{header_end, header_start};

const BIN: &str = "smoke_b_timescale";
const GUARD: &str = "SIDEREON_SMOKE_B_TIMESCALE_PINS_H";
const P: &str = "SMOKE_B_TIMESCALE";

/// 2017-01-01 00:00:00 UTC, the epoch of the leap-aware queries.
const JD_2017: f64 = 2457754.5;

fn main() {
    use TimeScale::*;
    // (name, from, to, UTC Julian date for the leap-aware form)
    let cases: [(&str, TimeScale, TimeScale, Option<f64>); 14] = [
        ("GPST_BDT", Gpst, Bdt, None),
        ("BDT_GPST", Bdt, Gpst, None),
        ("TAI_TT", Tai, Tt, None),
        ("GPST_TT", Gpst, Tt, None),
        ("GPST_TAI", Gpst, Tai, None),
        ("GPST_GST", Gpst, Gst, None),
        ("GPST_QZSST", Gpst, Qzsst, None),
        ("AT_UTC_GPST", Utc, Gpst, Some(JD_2017)),
        ("AT_GPST_UTC", Gpst, Utc, Some(JD_2017)),
        ("AT_GPST_GLONASST", Gpst, Glonasst, Some(JD_2017)),
        ("GPST_UTC", Gpst, Utc, None),
        ("GLONASST_GPST", Glonasst, Gpst, None),
        ("GPST_TDB", Gpst, Tdb, None),
        ("AT_GPST_UTC_NAN", Gpst, Utc, Some(f64::NAN)),
    ];
    header_start(BIN, GUARD);
    define_bits(&format!("{P}_JD_2017_BITS"), JD_2017);
    for (name, from, to, jd) in cases {
        let result = match jd {
            None => timescale_offset_s(from, to),
            Some(jd) => timescale_offset_at_s(from, to, jd),
        };
        // An engine refusal reaches C as SIDEREON_STATUS_INVALID_ARGUMENT
        // (src/time.rs time_offset_error_to_status) with the output zeroed.
        define_bool(&format!("{P}_{name}_OK"), result.is_ok());
        define_bits(&format!("{P}_{name}_BITS"), result.unwrap_or(0.0));
    }
    header_end(GUARD);
}
