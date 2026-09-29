//! smoke.c exercise_tle_file_surface: sidereon-core's reading of the
//! three-record TLE text the test assembles, and the look angle of its first
//! satellite.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::passes::{look_angle_arc, GroundStation, UtcInstant};
use sidereon_core::astro::sgp4::{parse_tle_file_with_policy, OpsMode, TleRecordIssue};
use sidereon_core::astro::tle::TlePolicy;
use valgen::{header_end, header_start};

const BIN: &str = "smoke_b_tle";
const GUARD: &str = "SIDEREON_SMOKE_B_TLE_PINS_H";
const P: &str = "SMOKE_B_TLE";

const ISS_L1: &str = "1 25544U 98067A   18184.80969102  .00001614  00000-0  31745-4 0  9993";
const ISS_L2: &str = "2 25544  51.6414 295.8524 0003435 262.6267 204.2868 15.54005638121106";
/// The instant the test evaluates the look angle at.
const LOOK_UNIX_US: i64 = 1_530_645_960_000_000;

fn main() {
    // The text smoke.c formats: a named ISS set with CRLF line ends, a blank
    // line, the same set without a name, and a pair that does not initialize.
    let text = format!(
        "ISS (ZARYA)\r\n{ISS_L1}\r\n{ISS_L2}\r\n\r\n{ISS_L1}\n{ISS_L2}\n\
         1 00000U 00000XYZ BADDATA\n2 00000 BADDATA\n"
    );
    let file = parse_tle_file_with_policy(&text, OpsMode::Improved, TlePolicy::Strict);

    header_start(BIN, GUARD);
    define_text(&format!("{P}_ISS_L1"), ISS_L1);
    define_text(&format!("{P}_ISS_L2"), ISS_L2);
    define_typed(&format!("{P}_COUNT"), "size_t", file.satellites.len());
    define_typed(&format!("{P}_SKIPPED"), "size_t", file.skipped());
    let rejected = &file.rejected[0];
    define_typed(
        &format!("{P}_REJECTED0_LINE"),
        "size_t",
        rejected.line_number,
    );
    let issue = match rejected.issue {
        TleRecordIssue::Invalid(_) => "SIDEREON_TLE_RECORD_ISSUE_INVALID",
        TleRecordIssue::MissingLine2 => "SIDEREON_TLE_RECORD_ISSUE_MISSING_LINE2",
        TleRecordIssue::OrphanLine2 => "SIDEREON_TLE_RECORD_ISSUE_ORPHAN_LINE2",
        TleRecordIssue::OrphanName => "SIDEREON_TLE_RECORD_ISSUE_ORPHAN_NAME",
    };
    println!("#define {P}_REJECTED0_ISSUE {issue}");
    let error = match &rejected.issue {
        TleRecordIssue::Invalid(err) => err.to_string(),
        _ => String::new(),
    };
    define_text(&format!("{P}_REJECTED0_ERROR"), &error);
    define_typed(
        &format!("{P}_RECORD0_LINE"),
        "size_t",
        file.satellites[0].line_number,
    );
    define_text(&format!("{P}_RECORD0_NAME"), &file.satellites[0].name);
    define_text(&format!("{P}_RECORD1_NAME"), &file.satellites[1].name);

    let station = GroundStation {
        latitude_deg: 40.0,
        longitude_deg: -75.0,
        altitude_m: 0.0,
    };
    let look = look_angle_arc(
        &file.satellites[0].satellite,
        station,
        &[UtcInstant::from_unix_microseconds(LOOK_UNIX_US)],
    )
    .expect("look angle");
    define_i64(&format!("{P}_LOOK_UNIX_US"), LOOK_UNIX_US);
    define_typed(&format!("{P}_LOOK_COUNT"), "size_t", look.len());
    define_bits_array(
        &format!("{P}_LOOK_BITS"),
        &[look[0].azimuth_deg, look[0].elevation_deg, look[0].range_km],
    );
    header_end(GUARD);
}
