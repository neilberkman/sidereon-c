//! full_coverage_smoke.c: sidereon-core's force, Doppler and covariance
//! results, time metadata and GNSS week arithmetic, the ISS coverage grid, the
//! constellation helpers and catalog diff, and the piecewise reduced-orbit
//! fits, for the inputs that test passes.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::covariance::{positive_semidefinite, rtn_to_eci, symmetric};
use sidereon_core::astro::coverage::{
    access_counts, look_angles_batch, max_elevation, visible_mask,
};
use sidereon_core::astro::doppler::{doppler_shift, range_rate_and_ratio};
use sidereon_core::astro::forces::{ForceModel, J2Gravity, TwoBodyGravity};
use sidereon_core::astro::omm::parse_json_array;
use sidereon_core::astro::passes::{GroundStation, UtcInstant};
use sidereon_core::astro::propagator::api::PropagationContext;
use sidereon_core::astro::sgp4::{OpsMode, Satellite};
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::gnss::{
    seconds_of_week_from_calendar, week_and_seconds_of_week, week_epoch_julian_day_number,
    week_from_calendar,
};
use sidereon_core::astro::time::scales::{
    find_leap_seconds, julian_day_number, leap_second_table, ut1_coverage, TimeScales,
};
use sidereon_core::astro::time::{GnssWeekTow, TimeScale};
use sidereon_core::astro::tle::TlePolicy;
use sidereon_core::constellation::{
    changed, diff, from_celestrak_omm, galileo_prn_for_gsat, glonass_fdma_channel,
    glonass_slot_for_number, is_valid, merge_navcen, parse_navcen, validate_against_sp3,
    validate_against_sp3_ids_strict,
};
use sidereon_core::ephemeris::Sp3;
use sidereon_core::orbit::{
    drift_piecewise_reduced_orbit_source, fit_piecewise_reduced_orbit_source, CalendarEpoch, Model,
    PiecewiseOrbitSourceFitOptions, ReducedOrbitSource, ReducedOrbitSourceDriftOptions,
    ReducedOrbitSourceSampling,
};
use sidereon_core::GnssSystem;
use valgen::{read, read_bytes, tests_path};

const POSITION: [f64; 3] = [7000.0, -1210.0, 1300.0];
const VELOCITY: [f64; 3] = [0.2, 7.2, 1.0];

fn acceleration(force: &dyn ForceModel) -> [f64; 3] {
    let a = force
        .acceleration(
            &CartesianState::new(0.0, POSITION, VELOCITY),
            &PropagationContext::default(),
        )
        .expect("acceleration");
    [a.x, a.y, a.z]
}

fn prop_header() -> String {
    tests_path("prop_fixture.h")
}

fn sampling(t0: CalendarEpoch, t1: CalendarEpoch, cadence: f64) -> ReducedOrbitSourceSampling {
    ReducedOrbitSourceSampling::new(t0, t1, cadence)
}

fn main() {
    let guard = "SIDEREON_W6_FULL_COVERAGE_PINS_H";
    valgen::header_start("w6_full_coverage", guard);

    comment("Two-body and J2 accelerations at the test state.");
    def_bits_array(
        "W6_FCOV_TWOBODY_BITS",
        &acceleration(&TwoBodyGravity::default()),
    );
    def_bits_array("W6_FCOV_J2_BITS", &acceleration(&J2Gravity::default()));

    let ts = TimeScales::from_utc(2020, 6, 24, 0, 0, 0.0).expect("time scales");
    let (range_rate, ratio) =
        range_rate_and_ratio(POSITION, VELOCITY, 51.5, -0.1, 0.08, &ts).expect("range rate");
    comment("Doppler range rate and ratio, and the L1 shift.");
    def_bits("W6_FCOV_RANGE_RATE_BITS", range_rate);
    def_bits("W6_FCOV_RATIO_BITS", ratio);
    let shift =
        doppler_shift(POSITION, VELOCITY, 51.5, -0.1, 0.08, &ts, 1575.42e6).expect("Doppler shift");
    def_bits("W6_FCOV_SHIFT_RANGE_RATE_BITS", shift.range_rate_km_s);
    def_bits("W6_FCOV_SHIFT_HZ_BITS", shift.doppler_hz);
    def_bits("W6_FCOV_SHIFT_RATIO_BITS", shift.doppler_ratio);

    let cov = [[1.0, 0.1, 0.0], [0.1, 2.0, 0.2], [0.0, 0.2, 3.0]];
    let eci = rtn_to_eci(&cov, POSITION, VELOCITY).expect("RTN to ECI");
    comment("RTN-to-ECI covariance (row major) and the input's symmetry and PSD verdicts.");
    def_bits_array(
        "W6_FCOV_ECI_COV_BITS",
        &eci.iter().flatten().copied().collect::<Vec<_>>(),
    );
    def_bool("W6_FCOV_SYMMETRIC", symmetric(&cov));
    def_bool("W6_FCOV_PSD", positive_semidefinite(&cov));

    comment("Time metadata.");
    def_str("W6_FCOV_GPST_ABBREV", TimeScale::Gpst.abbrev());
    def_bits(
        "W6_FCOV_LEAP_2020_BITS",
        find_leap_seconds(julian_day_number(2020, 1, 1) as f64 - 0.5),
    );
    let table = leap_second_table();
    def_size("W6_FCOV_LEAP_ENTRIES", table.entries);
    def_size("W6_FCOV_LEAP_SOURCE_LEN", table.source.len());
    let ut1 = ut1_coverage();
    def_size("W6_FCOV_UT1_ENTRIES", ut1.entries);
    def_size("W6_FCOV_UT1_SOURCE_LEN", ut1.source.len());
    def_bits("W6_FCOV_UT1_FIRST_JD_TT_BITS", ut1.first_jd_tt);
    def_bits("W6_FCOV_UT1_LAST_JD_TT_BITS", ut1.last_jd_tt);
    def_bool(
        "W6_FCOV_UT1_COVERS_MIDPOINT",
        ut1.covers_jd_tt(0.5 * (ut1.first_jd_tt + ut1.last_jd_tt)),
    );

    comment("GNSS week arithmetic.");
    let normalized = GnssWeekTow::new(TimeScale::Gpst, 100, 604805.0)
        .expect("week tow")
        .normalized()
        .expect("normalized");
    def("W6_FCOV_NORMALIZED_WEEK", normalized.week);
    def_bits("W6_FCOV_NORMALIZED_TOW_BITS", normalized.tow_s);
    def(
        "W6_FCOV_UNROLLED_WEEK",
        normalized.unrolled_week(2).expect("unrolled"),
    );
    let jdn = week_epoch_julian_day_number(TimeScale::Gpst);
    def_bool("W6_FCOV_WEEK_EPOCH_PRESENT", jdn.is_some());
    def(
        "W6_FCOV_WEEK_EPOCH_JDN",
        format!("INT64_C({})", jdn.unwrap_or(0)),
    );
    let week = week_from_calendar(TimeScale::Gpst, 2020, 6, 24);
    def_bool("W6_FCOV_WEEK_2020_06_24_PRESENT", week.is_some());
    def("W6_FCOV_WEEK_2020_06_24", week.unwrap_or(0));
    let sow = seconds_of_week_from_calendar(2020, 6, 24, 1, 2, 3);
    def_bool("W6_FCOV_SOW_PRESENT", sow.is_some());
    def_bits("W6_FCOV_SOW_BITS", sow.unwrap_or(f64::NAN));
    def_bool(
        "W6_FCOV_SOW_MONTH13_REFUSED",
        seconds_of_week_from_calendar(2020, 13, 1, 0, 0, 0).is_none(),
    );
    def_bool(
        "W6_FCOV_SOW_FEB29_2021_REFUSED",
        seconds_of_week_from_calendar(2021, 2, 29, 0, 0, 0).is_none(),
    );
    def_bool(
        "W6_FCOV_SOW_HOUR24_REFUSED",
        seconds_of_week_from_calendar(2020, 6, 24, 24, 0, 0).is_none(),
    );
    def_bool(
        "W6_FCOV_WEEK_FEB30_PRESENT",
        week_from_calendar(TimeScale::Gpst, 2020, 2, 30).is_some(),
    );
    let (split_week, split_sow) = week_and_seconds_of_week(604800.0 * 3.0 + 42.0);
    def_bits("W6_FCOV_SPLIT_WEEK_BITS", split_week);
    def_bits("W6_FCOV_SPLIT_SOW_BITS", split_sow);

    // Coverage grid: the committed ISS TLE, AFSPC mode, strict checksums.
    let header = prop_header();
    let line1 = header_define_string(&header, "PROP_TLE_LINE1");
    let line2 = header_define_string(&header, "PROP_TLE_LINE2");
    let (satellite, _) =
        Satellite::from_tle_with_policy(&line1, &line2, OpsMode::Afspc, TlePolicy::Strict)
            .expect("ISS TLE");
    let stations = [
        GroundStation {
            latitude_deg: header_define_f64(&header, "PROP_STATION_LATITUDE_DEG"),
            longitude_deg: header_define_f64(&header, "PROP_STATION_LONGITUDE_DEG"),
            altitude_m: header_define_f64(&header, "PROP_STATION_ALTITUDE_M"),
        },
        GroundStation {
            latitude_deg: 40.7128,
            longitude_deg: -74.0060,
            altitude_m: 10.0,
        },
    ];
    let epoch_us = header_int64s(&header, "PROP_EPOCHS_UNIX_US")[0];
    let grid = look_angles_batch(
        &[satellite.clone()],
        &stations,
        UtcInstant::from_unix_microseconds(epoch_us),
    );
    comment(
        "Coverage grid at the first epoch: cell (0, 0), mask, counts, max elevation at -90 deg.",
    );
    let cell = grid[0][0].as_ref();
    def_bool("W6_FCOV_CELL_OK", cell.is_ok());
    let look = cell.expect("cell look angle");
    def_bits("W6_FCOV_CELL_AZ_BITS", look.azimuth_deg);
    def_bits("W6_FCOV_CELL_EL_BITS", look.elevation_deg);
    def_bits("W6_FCOV_CELL_RANGE_BITS", look.range_km);
    let mask: Vec<bool> = visible_mask(&grid, -90.0).into_iter().flatten().collect();
    println!(
        "static const bool W6_FCOV_MASK[{}] = {{ {} }};",
        mask.len(),
        mask.iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let counts = access_counts(&grid, -90.0);
    println!(
        "static const size_t W6_FCOV_COUNTS[{}] = {{ {} }};",
        counts.len(),
        counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let max_el: Vec<f64> = max_elevation(&grid)
        .into_iter()
        .map(|v| v.unwrap_or(f64::NAN))
        .collect();
    def_bits_array("W6_FCOV_MAX_EL_BITS", &max_el);

    comment("Constellation helpers.");
    let gsat = galileo_prn_for_gsat(210);
    def_bool("W6_FCOV_GSAT210_PRESENT", gsat.is_some());
    def("W6_FCOV_GSAT210_PRN", gsat.unwrap_or(0));
    let slot = glonass_slot_for_number(730);
    def_bool("W6_FCOV_GLONASS730_PRESENT", slot.is_some());
    def("W6_FCOV_GLONASS730_SLOT", slot.unwrap_or(0));
    let channel = glonass_fdma_channel(1);
    def_bool("W6_FCOV_FDMA1_PRESENT", channel.is_some());
    def("W6_FCOV_FDMA1_CHANNEL", channel.unwrap_or(0));

    let json = read(&tests_path("fixtures/constellation/gps_ops_sample.json"));
    let navcen = read_bytes(&tests_path("fixtures/constellation/navcen_gps_sample.html"));
    let omms = parse_json_array(&json).expect("gps-ops JSON").omms;
    let base = from_celestrak_omm(GnssSystem::Gps, &omms).expect("base catalog");
    let with_status = merge_navcen(&base, &parse_navcen(&navcen).expect("NAVCEN"));
    comment("Strict SP3-id validation of G03 G05 G13 G19, and the base vs NAVCEN-merged diff.");
    def_bool(
        "W6_FCOV_STRICT_IDS_OK",
        validate_against_sp3_ids_strict(&base, &["G03", "G05", "G13", "G19"]).is_ok(),
    );
    let d = diff(&base, &with_status);
    def_bool("W6_FCOV_DIFF_CHANGED", changed(&d));
    def_size("W6_FCOV_DIFF_ADDED", d.added.len());
    def_size("W6_FCOV_DIFF_REMOVED", d.removed.len());
    def_size("W6_FCOV_DIFF_NORAD_REASSIGNED", d.norad_reassigned.len());
    def_size("W6_FCOV_DIFF_SP3_ID_CHANGED", d.sp3_id_changed.len());
    def_size("W6_FCOV_DIFF_SVN_CHANGED", d.svn_changed.len());
    def_size("W6_FCOV_DIFF_FDMA_CHANGED", d.fdma_channel_changed.len());
    def_size("W6_FCOV_DIFF_ACTIVITY_CHANGED", d.activity_changed.len());
    def_size("W6_FCOV_DIFF_USABILITY_CHANGED", d.usability_changed.len());
    def_bool("W6_FCOV_SAME_DIFF_CHANGED", changed(&diff(&base, &base)));

    // sidereon_constellation_validate_against_sp3 on the NAVCEN-merged catalog
    // and the SP3 the test loads (its argv[1], run_smoke.sh's GRG product).
    let grg = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3");
    let report = validate_against_sp3(&with_status, &grg);
    comment("validate_against_sp3 of the merged catalog against the GRG SP3.");
    def_bool("W6_FCOV_SP3_VALID", is_valid(&report));
    let prns: Vec<String> = report
        .inactive_unusable_prns
        .iter()
        .map(|(_, prn)| prn.to_string())
        .collect();
    let systems: Vec<String> = report
        .inactive_unusable_prns
        .iter()
        .map(|(system, _)| c_enum("SIDEREON_GNSS_SYSTEM", system))
        .collect();
    def_size("W6_FCOV_SP3_INACTIVE_COUNT", prns.len());
    c_array("unsigned short", "W6_FCOV_SP3_INACTIVE_PRNS", &prns, "0");
    c_array(
        "unsigned int",
        "W6_FCOV_SP3_INACTIVE_SYSTEMS",
        &systems,
        "0",
    );
    def_size(
        "W6_FCOV_SP3_DUPLICATE_PRN_COUNT",
        report.duplicate_prns.len(),
    );
    def_size(
        "W6_FCOV_SP3_DUPLICATE_NORAD_COUNT",
        report.duplicate_norad_ids.len(),
    );
    let quoted = |ids: &[String]| {
        ids.iter()
            .map(|id| valgen::c_string(id))
            .collect::<Vec<_>>()
    };
    def_size("W6_FCOV_SP3_MISSING_COUNT", report.missing_sp3_ids.len());
    c_array(
        "char *const",
        "W6_FCOV_SP3_MISSING_IDS",
        &quoted(&report.missing_sp3_ids),
        "\"\"",
    );
    def_size("W6_FCOV_SP3_EXTRA_COUNT", report.extra_sp3_ids.len());
    c_array(
        "char *const",
        "W6_FCOV_SP3_EXTRA_IDS",
        &quoted(&report.extra_sp3_ids),
        "\"\"",
    );

    // Piecewise reduced orbits from the GRG SP3 (G01) and the ISS TLE.
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3");
    let g01 = "G01".parse().expect("G01");
    let fit = fit_piecewise_reduced_orbit_source(
        ReducedOrbitSource::Sp3 {
            product: &sp3,
            satellite: g01,
        },
        PiecewiseOrbitSourceFitOptions::new(
            sampling(
                CalendarEpoch::new(2020, 6, 24, 0, 0, 0.0),
                CalendarEpoch::new(2020, 6, 24, 3, 0, 0.0),
                900.0,
            ),
            Model::CircularSecular,
            3600.0,
        ),
    )
    .expect("piecewise SP3 fit");
    comment("Piecewise SP3 fit (G01, 3600 s segments) and its drift to 04:00.");
    def_size("W6_FCOV_PW_SP3_REQUESTED", fit.requested_samples);
    def_size(
        "W6_FCOV_PW_SP3_USED",
        fit.orbit
            .segments
            .iter()
            .map(|s| s.orbit.stats.n_samples)
            .sum::<usize>(),
    );
    def_size("W6_FCOV_PW_SP3_SEGMENTS", fit.orbit.segments.len());
    let drift = drift_piecewise_reduced_orbit_source(
        &fit.orbit,
        ReducedOrbitSource::Sp3 {
            product: &sp3,
            satellite: g01,
        },
        ReducedOrbitSourceDriftOptions::new(
            sampling(
                CalendarEpoch::new(2020, 6, 24, 0, 0, 0.0),
                CalendarEpoch::new(2020, 6, 24, 4, 0, 0.0),
                900.0,
            ),
            1.0e9,
        ),
    )
    .expect("piecewise SP3 drift");
    def_bits("W6_FCOV_PW_SP3_DRIFT_MAX_BITS", drift.report.max_m);
    def_bits("W6_FCOV_PW_SP3_DRIFT_RMS_BITS", drift.report.rms_m);

    let tle_window = || {
        sampling(
            CalendarEpoch::new(2018, 7, 3, 19, 30, 0.0),
            CalendarEpoch::new(2018, 7, 3, 22, 30, 0.0),
            600.0,
        )
    };
    let tle_fit = fit_piecewise_reduced_orbit_source(
        ReducedOrbitSource::Sgp4 {
            satellite: &satellite,
        },
        PiecewiseOrbitSourceFitOptions::new(tle_window(), Model::CircularSecular, 3600.0),
    )
    .expect("piecewise TLE fit");
    comment("Piecewise TLE fit (2018-07-03 19:30-22:30, 600 s, 3600 s segments).");
    def_size("W6_FCOV_PW_TLE_REQUESTED", tle_fit.requested_samples);
    def_size(
        "W6_FCOV_PW_TLE_USED",
        tle_fit
            .orbit
            .segments
            .iter()
            .map(|s| s.orbit.stats.n_samples)
            .sum::<usize>(),
    );
    def_bool(
        "W6_FCOV_PW_TLE_DRIFT_OK",
        drift_piecewise_reduced_orbit_source(
            &tle_fit.orbit,
            ReducedOrbitSource::Sgp4 {
                satellite: &satellite,
            },
            ReducedOrbitSourceDriftOptions::new(tle_window(), 1.0e9),
        )
        .is_ok(),
    );
    valgen::header_end(guard);
}

/// `static const <ty> name[] = { ... };`, with `empty` as the one element of
/// an empty list (C has no empty initializer; the count macro is zero).
fn c_array(ty: &str, name: &str, values: &[String], empty: &str) {
    let body = if values.is_empty() {
        empty.to_string()
    } else {
        values.join(", ")
    };
    println!("static const {ty} {name}[] = {{ {body} }};");
}
