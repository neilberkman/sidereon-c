//! smoke.c exercise_dop_surface, exercise_antex_surface and
//! exercise_velocity_surface: the values those checks compare against that
//! the committed golden headers do not already carry, as sidereon-core
//! computes them.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::antex::Antex;
use sidereon_core::ephemeris::Sp3;
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::geometry::{dop, line_of_sight_from_az_el_deg};
use sidereon_core::velocity::{
    solve, VelocityObservable, VelocityObservation, VelocitySolveOptions,
};
use sidereon_core::GnssSatelliteId;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const BIN: &str = "smoke_b_geometry";
const GUARD: &str = "SIDEREON_SMOKE_B_GEOMETRY_PINS_H";
const P: &str = "SMOKE_B_GEOMETRY";

/// The az/el directions and the site the DOP check builds lines of sight
/// from.
const AZ_DEG: [f64; 4] = [0.0, 90.0, 180.0, 270.0];
const EL_DEG: [f64; 4] = [80.0, 30.0, 30.0, 30.0];
const SITE: [f64; 3] = [0.6, 0.1, 0.0];

fn dop_section() {
    let site = Wgs84Geodetic::new(SITE[0], SITE[1], SITE[2]).expect("site");
    define_bits_array(&format!("{P}_LOS_AZ_DEG_BITS"), &AZ_DEG);
    define_bits_array(&format!("{P}_LOS_EL_DEG_BITS"), &EL_DEG);
    define_bits_array(&format!("{P}_SITE_BITS"), &SITE);
    let mut rows = Vec::new();
    let mut flat = Vec::new();
    for i in 0..4 {
        let los = line_of_sight_from_az_el_deg(AZ_DEG[i], EL_DEG[i], site).expect("line of sight");
        flat.extend([los.e_x, los.e_y, los.e_z]);
        rows.push(los);
    }
    define_bits_array(&format!("{P}_LOS_XYZ_BITS"), &flat);
    let d = dop(&rows, &[1.0; 4], site).expect("DOP of the built geometry");
    define_bits_array(
        &format!("{P}_BUILT_DOP_BITS"),
        &[d.gdop, d.pdop, d.hdop, d.vdop, d.tdop],
    );
    // An elevation of 91 degrees; an engine refusal reaches C as
    // SIDEREON_STATUS_INVALID_ARGUMENT (src/lib.rs map_dop_error).
    let refused = line_of_sight_from_az_el_deg(0.0, 91.0, site);
    define_bool(&format!("{P}_LOS_EL91_OK"), refused.is_ok());
    if let Err(err) = &refused {
        define_text(&format!("{P}_LOS_EL91_ERROR"), &variant_name(err));
    }
    println!();
}

fn antex_section() {
    let text = read(&tests_path("fixtures/antex/igs20_wettzell_trim.atx"));
    let antex = Antex::parse(&text).expect("ANTEX fixture");
    define_bool(
        &format!("{P}_ANTEX_NO_SUCH_ANTENNA_FOUND"),
        antex.antenna("NO SUCH ANTENNA").is_some(),
    );
    let first_id = header_strings("antex_fixture.h", "ANTEX_PCO_CASES")
        .into_iter()
        .next()
        .expect("first PCO case antenna id");
    let antenna = antex.antenna(&first_id).expect("first PCO case antenna");
    // An engine ANTEX refusal reaches C as SIDEREON_STATUS_INVALID_ARGUMENT
    // (src/lib.rs map_antex_error).
    let unknown = antenna.pco("ZZ9");
    define_bool(&format!("{P}_ANTEX_PCO_ZZ9_OK"), unknown.is_ok());
    if let Err(err) = &unknown {
        define_text(&format!("{P}_ANTEX_PCO_ZZ9_ERROR"), &variant_name(err));
    }
    println!();
}

fn velocity_section() {
    let defaults = VelocitySolveOptions::default();
    define_bool(
        &format!("{P}_VELOCITY_DEFAULT_RANGE_RATE"),
        matches!(defaults.observable, VelocityObservable::RangeRate),
    );
    define_bool(
        &format!("{P}_VELOCITY_DEFAULT_LIGHT_TIME"),
        defaults.light_time,
    );
    define_bool(&format!("{P}_VELOCITY_DEFAULT_SAGNAC"), defaults.sagnac);

    // The first three range-rate observations of velocity_fixture.h, solved
    // as the test solves them.
    let sp3_file = header_define_string("velocity_fixture.h", "VEL_SP3_FILE");
    let sp3 = Sp3::parse(&read_bytes(&tests_path(&format!(
        "fixtures/sp3/{sp3_file}"
    ))))
    .expect("velocity SP3");
    let ids = header_strings("velocity_fixture.h", "VEL_SAT_IDS");
    let values = header_f64s("velocity_fixture.h", "VEL_RANGE_RATE_BITS");
    let carrier = header_f64s("velocity_fixture.h", "VEL_F_L1_HZ_BITS")[0];
    let receiver = header_f64s("velocity_fixture.h", "VEL_RECEIVER_BITS");
    let t_rx = header_f64s("velocity_fixture.h", "VEL_T_RX_J2000_S_BITS")[0];
    let observations: Vec<VelocityObservation> = ids
        .iter()
        .zip(&values)
        .take(3)
        .map(|(id, value)| VelocityObservation {
            satellite_id: id.parse::<GnssSatelliteId>().expect("satellite id"),
            value: *value,
            carrier_hz: carrier,
            sat_clock_drift_s_s: 0.0,
        })
        .collect();
    let thin = solve(
        &sp3,
        &observations,
        [receiver[0], receiver[1], receiver[2]],
        t_rx,
        defaults,
    );
    // An engine refusal reaches C as SIDEREON_STATUS_SOLVE (src/velocity.rs
    // guard).
    define_bool(&format!("{P}_VELOCITY_THREE_SATS_OK"), thin.is_ok());
    if let Err(err) = &thin {
        define_text(
            &format!("{P}_VELOCITY_THREE_SATS_ERROR"),
            &variant_name(err),
        );
    }
}

fn main() {
    header_start(BIN, GUARD);
    dop_section();
    antex_section();
    velocity_section();
    header_end(GUARD);
}
