// smoke.c exercise_propagation_surface: what prop_fixture.h does not carry.
// sidereon-core's reading of the smoke's TLE line pairs (the clean pair's
// checksum findings, the strict refusal of the bad-checksum pair, the refusal
// of two lines that are no TLE) and the passes sidereon-core finds for the
// smoke's station, window and options. The inputs are read from
// prop_fixture.h, which the smoke passes to the C routes.

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::astro::passes::{
    find_passes_for_satellite, GroundStation, PassFinderOptions, UtcInstant,
};
use sidereon_core::astro::sgp4::{OpsMode, Satellite};
use sidereon_core::astro::tle::{parse_with_policy, TlePolicy};
use support::*;
use valgen::{header_end, header_start, read, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_PROP_PINS_H";

/// The replacement text of `#define NAME ...` in prop_fixture.h.
fn define<'a>(header: &'a str, name: &str) -> &'a str {
    let marker = format!("#define {name} ");
    let start = header
        .find(&marker)
        .unwrap_or_else(|| panic!("{name} in prop_fixture.h"))
        + marker.len();
    header[start..].lines().next().expect("value").trim()
}

/// A C string literal define, read back to its text.
fn define_str(header: &str, name: &str) -> String {
    let value = define(header, name);
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or_else(|| panic!("{name} is a string literal"))
        .to_string()
}

/// A numeric define, read as the C compiler reads the literal: a decimal
/// double, or INT64_C(n).
fn define_f64(header: &str, name: &str) -> f64 {
    define(header, name).parse().expect("decimal literal")
}

fn define_i64(header: &str, name: &str) -> i64 {
    let value = define(header, name);
    value
        .strip_prefix("INT64_C(")
        .and_then(|v| v.strip_suffix(')'))
        .unwrap_or(value)
        .parse()
        .expect("integer literal")
}

fn main() {
    let header = read(&tests_path("prop_fixture.h"));
    let line1 = define_str(&header, "PROP_TLE_LINE1");
    let line2 = define_str(&header, "PROP_TLE_LINE2");
    let bad1 = define_str(&header, "PROP_TLE_BAD_CHECKSUM_LINE1");
    let bad2 = define_str(&header, "PROP_TLE_BAD_CHECKSUM_LINE2");
    // SIDEREON_TLE_OPS_MODE_AFSPC is 0 and IMPROVED is 1.
    let opsmode = match define_i64(&header, "PROP_TLE_OPSMODE") {
        0 => OpsMode::Afspc,
        1 => OpsMode::Improved,
        other => panic!("opsmode {other}"),
    };

    header_start("smoke_a_prop", GUARD);

    comment("tle::parse_with_policy under the strict policy sidereon_tle_load applies.");
    let clean = parse_with_policy(&line1, &line2, TlePolicy::Strict).expect("clean TLE");
    def_usize(
        "SMOKE_A_PROP_CLEAN_CHECKSUM_WARNING_COUNT",
        clean.checksum_warnings.len(),
    );
    let refusal = |name: &str, result: Result<String, String>| match result {
        Ok(_) => {
            def_bool(&format!("{name}_REFUSED"), false);
            def_str(&format!("{name}_ERROR_TEXT"), "");
        }
        Err(text) => {
            def_bool(&format!("{name}_REFUSED"), true);
            def_str(&format!("{name}_ERROR_TEXT"), &text);
        }
    };
    refusal(
        "SMOKE_A_PROP_BAD_CHECKSUM_STRICT",
        parse_with_policy(&bad1, &bad2, TlePolicy::Strict)
            .map(|_| String::new())
            .map_err(|err| err.to_string()),
    );
    refusal(
        "SMOKE_A_PROP_NOT_A_TLE",
        parse_with_policy("not a tle", "also not a tle", TlePolicy::Strict)
            .map(|_| String::new())
            .map_err(|err| err.to_string()),
    );

    comment("find_passes_for_satellite over the smoke's window with its options.");
    let (satellite, _) =
        Satellite::from_tle_with_policy(&line1, &line2, opsmode, TlePolicy::Strict)
            .expect("SGP4 satellite");
    let station = GroundStation {
        latitude_deg: define_f64(&header, "PROP_STATION_LATITUDE_DEG"),
        longitude_deg: define_f64(&header, "PROP_STATION_LONGITUDE_DEG"),
        altitude_m: define_f64(&header, "PROP_STATION_ALTITUDE_M"),
    };
    let mut options = PassFinderOptions::default();
    options.elevation_mask_deg = define_f64(&header, "PROP_PASS_ELEVATION_MASK_DEG");
    options.coarse_step_seconds = define_f64(&header, "PROP_PASS_STEP_SECONDS");
    options.time_tolerance_seconds = define_f64(&header, "PROP_PASS_TIME_TOLERANCE_S");
    let passes = find_passes_for_satellite(
        &satellite,
        station,
        UtcInstant::from_unix_microseconds(define_i64(&header, "PROP_PASS_START_UNIX_US")),
        UtcInstant::from_unix_microseconds(define_i64(&header, "PROP_PASS_END_UNIX_US")),
        options,
    )
    .expect("passes");
    def_usize("SMOKE_A_PROP_PASS_COUNT", passes.len());
    let max_elevation: Vec<f64> = passes.iter().map(|pass| pass.max_elevation_deg).collect();
    def_bits_array("SMOKE_A_PROP_PASS_MAX_ELEVATION_BITS", &max_elevation);
    header_end(GUARD);
}
