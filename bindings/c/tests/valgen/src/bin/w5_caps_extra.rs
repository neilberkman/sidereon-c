//! tests/w5_caps_extra_pins.h: sidereon-core's results for the inputs
//! caps_extra_smoke.c passes (error ellipse, DOP conventions, residual
//! statistics, batch prediction, leap seconds, EGM96, Sun/Moon geometry).
//! The trust-region solves go through trust_region_least_squares and the
//! Jacobian covariance helpers through sidereon-core, as the binding routes
//! them (src/solve.rs, src/estimation.rs, src/covariance.rs).

#[path = "w5_support/inputs.rs"]
mod inputs;
#[path = "w5_support/pins.rs"]
mod pins;

use pins::Pins;
use sidereon_core::astro::bodies::{
    find_moon_elevation_crossings, find_moon_transits, moon_az_el, moon_illumination, sun_az_el,
    MoonElevationOptions,
};
use sidereon_core::astro::frames::transforms::GeodeticStationKm;
use sidereon_core::astro::passes::UtcInstant;
use sidereon_core::astro::time::scales::{gps_utc_offset_s, tai_utc_offset_s};
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::geoid::{
    egm96_ellipsoidal_height_m, egm96_orthometric_height_m, egm96_undulation,
};
use sidereon_core::geometry::{
    dop_with_convention, error_ellipse_2x2, line_of_sight_from_az_el_deg, EnuConvention,
    LineOfSight,
};
use sidereon_core::observables::{predict_batch, PredictOptions, PredictRequest};
use sidereon_core::quality::normality::{jarque_bera, kurtosis, moments, shapiro_wilk, skewness};
use trust_region_least_squares::batch::solve_data_problem_drop_one;
use trust_region_least_squares::data::{
    solve_data_problem, solve_data_problem_with, BuiltinResidual, DataProblem,
};
use trust_region_least_squares::hostlapack::LapackSvd;
use trust_region_least_squares::loss::Loss;
use trust_region_least_squares::trf::XScale;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

/// A data problem with the defaults sidereon_data_problem_init writes
/// (src/trls.rs) and the given residual and start point.
fn data_problem(kind: BuiltinResidual, x0: Vec<f64>) -> DataProblem {
    DataProblem {
        kind,
        x0,
        loss: Loss::Linear,
        f_scale: 1.0,
        x_scale: XScale::Unit,
        max_nfev: None,
        ftol: 1e-8,
        xtol: 1e-8,
        gtol: 1e-10,
    }
}

const GUARD: &str = "SIDEREON_W5_CAPS_EXTRA_PINS_H";

fn batch(
    p: &Pins,
    name: &str,
    results: &[Result<
        sidereon_core::observables::PredictedObservables,
        sidereon_core::observables::ObservablesError,
    >],
) {
    let ok: Vec<i128> = results.iter().map(|r| i128::from(r.is_ok())).collect();
    let range: Vec<f64> = results
        .iter()
        .map(|r| r.as_ref().map_or(0.0, |o| o.geometric_range_m))
        .collect();
    let elevation: Vec<f64> = results
        .iter()
        .map(|r| r.as_ref().map_or(0.0, |o| o.elevation_deg))
        .collect();
    p.ints(&format!("{name}_OK"), "bool", &ok);
    p.f64s(&format!("{name}_RANGE_M_BITS"), &range);
    p.f64s(&format!("{name}_ELEVATION_DEG_BITS"), &elevation);
}

fn main() {
    header_start("w5_caps_extra", GUARD);
    let p = Pins::new("W5_CAPS_EXTRA");

    // Trust-region solves.
    let linear = data_problem(
        BuiltinResidual::Linear {
            a: vec![1.0, 0.0, 1.0, 1.0, 1.0, 2.0],
            b: vec![1.0, 3.0, 5.0],
            m: 3,
            n: 2,
        },
        vec![0.0, 0.0],
    );
    let solved = solve_data_problem(&linear).expect("linear solve");
    p.comment("trust_region_least_squares on y = 1 + 2x at (0, 1), (1, 3), (2, 5): the\nlinear and degree-1 polynomial solves, leave-one-out, and the host-LAPACK\nbackend where the environment names one.");
    p.bool("TRLS_SUCCESS", solved.success());
    p.f64s("TRLS_X_BITS", &solved.x);
    p.f64("TRLS_COST_BITS", solved.cost);
    p.f64s("TRLS_RESIDUALS_BITS", &solved.fun);
    let poly = solve_data_problem(&data_problem(
        BuiltinResidual::Polynomial {
            degree: 1,
            t: vec![0.0, 1.0, 2.0],
            y: vec![1.0, 3.0, 5.0],
        },
        vec![0.0, 0.0],
    ))
    .expect("polynomial solve");
    p.f64s("TRLS_POLY_X_BITS", &poly.x);
    let drop_one = solve_data_problem_drop_one(&linear).expect("drop-one");
    p.int("TRLS_DROP_COUNT", drop_one.drops.len() as i128);
    p.bool("TRLS_DROP_BASE_SUCCESS", drop_one.base.success());
    p.f64s("TRLS_DROP_COST_DELTA_BITS", &drop_one.cost_delta);
    let drop_x: Vec<f64> = drop_one.drops.iter().flat_map(|d| d.x.clone()).collect();
    p.f64s("TRLS_DROP_X_BITS", &drop_x);
    let host = if std::env::var_os("TRUST_REGION_LEAST_SQUARES_LAPACK_PATH").is_some() {
        solve_data_problem_with(&linear, &LapackSvd::from_env()).ok()
    } else {
        None
    };
    p.bool("TRLS_HOST_LAPACK_SOLVED", host.is_some());
    p.f64s(
        "TRLS_HOST_LAPACK_X_BITS",
        &host.map_or(vec![0.0, 0.0], |h| h.x),
    );

    // Jacobian covariance helpers on [[1, 0], [1, 1], [1, 2]].
    use sidereon_core::astro::math::least_squares::{
        covariance_from_jacobian, hessian_trace, normal_covariance,
    };
    let jac = nalgebra::DMatrix::from_row_slice(3, 2, &[1.0, 0.0, 1.0, 1.0, 1.0, 2.0]);
    let row_major = |m: nalgebra::DMatrix<f64>| -> Vec<f64> {
        m.row_iter()
            .flat_map(|r| r.iter().copied().collect::<Vec<_>>())
            .collect()
    };
    p.comment("sidereon_core::astro::math::least_squares covariance helpers on the\ndesign [[1, 0], [1, 1], [1, 2]], row-major as the binding copies them.");
    p.f64s(
        "NORMAL_COVARIANCE_BITS",
        &row_major(normal_covariance(&jac, 1.0).expect("normal covariance")),
    );
    p.f64("HESSIAN_TRACE_BITS", hessian_trace(&jac));
    p.f64s(
        "JACOBIAN_COVARIANCE_BITS",
        &row_major(covariance_from_jacobian(&jac, 3.0).expect("Jacobian covariance")),
    );

    // Error ellipse: diag(4, 1) at confidence 1 - exp(-0.5).
    let e = error_ellipse_2x2([[4.0, 0.0], [0.0, 1.0]], 1.0 - (-0.5f64).exp()).expect("ellipse");
    p.comment("sidereon_core::geometry::error_ellipse_2x2(diag(4, 1), 1 - exp(-0.5)).");
    p.f64("ELLIPSE_CHI_SQUARE_SCALE_BITS", e.chi_square_scale);
    p.f64("ELLIPSE_SEMI_MAJOR_BITS", e.semi_major);
    p.f64("ELLIPSE_SEMI_MINOR_BITS", e.semi_minor);

    // DOP under both ENU conventions.
    let receiver = Wgs84Geodetic::new(0.6, 0.1, 100.0).expect("receiver");
    let az = [0.0, 90.0, 180.0, 270.0];
    let el = [15.0, 30.0, 45.0, 70.0];
    let los: Vec<LineOfSight> = (0..4)
        .map(|i| {
            let l = line_of_sight_from_az_el_deg(az[i], el[i], receiver).expect("los");
            LineOfSight::new(l.e_x, l.e_y, l.e_z)
        })
        .collect();
    let weights = [1.0; 4];
    p.comment("sidereon_core::geometry::dop_with_convention, both conventions.");
    for (name, convention) in [
        ("GEODETIC", EnuConvention::GeodeticNormal),
        ("GEOCENTRIC", EnuConvention::GeocentricRadial),
    ] {
        let d = dop_with_convention(&los, &weights, receiver, convention).expect("dop");
        p.f64(&format!("DOP_{name}_GDOP_BITS"), d.gdop);
        p.f64(&format!("DOP_{name}_PDOP_BITS"), d.pdop);
        p.f64(&format!("DOP_{name}_HDOP_BITS"), d.hdop);
        p.f64(&format!("DOP_{name}_VDOP_BITS"), d.vdop);
        p.f64(&format!("DOP_{name}_TDOP_BITS"), d.tdop);
    }

    // Residual statistics.
    let x = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let m = moments(&x, true, false).expect("moments");
    p.comment("sidereon_core::quality::normality on {2, 4, 4, 4, 5, 5, 7, 9}.");
    p.f64("MOMENTS_MEAN_BITS", m.mean);
    p.f64("MOMENTS_VARIANCE_BITS", m.variance);
    p.f64("SKEWNESS_BITS", skewness(&x, true).expect("skewness"));
    p.f64(
        "KURTOSIS_BITS",
        kurtosis(&x, true, false).expect("kurtosis"),
    );
    let jb = jarque_bera(&x).expect("jarque-bera");
    p.f64("JARQUE_BERA_STATISTIC_BITS", jb.statistic);
    p.f64("JARQUE_BERA_P_VALUE_BITS", jb.p_value);
    let sw = shapiro_wilk(&x).expect("shapiro-wilk");
    p.f64("SHAPIRO_WILK_W_BITS", sw.w);
    p.f64("SHAPIRO_WILK_P_VALUE_BITS", sw.p_value);
    p.outcome("SHAPIRO_WILK_TWO_SAMPLES", &shapiro_wilk(&[1.0, 2.0]));

    // Batch prediction, SP3 (first six SPP satellites) and broadcast.
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3")
    .with_interpolation_options(Default::default());
    let spp = inputs::spp();
    let requests: Vec<PredictRequest> = spp.sat_ids[..6]
        .iter()
        .map(|s| (s.parse().expect("sat"), spp.position_m, spp.t_rx_j2000_s))
        .collect();
    p.comment("sidereon_core::observables::predict_batch from the GRG SP3 at the SPP\nreceiver and epoch, and from the ESBC broadcast NAV at the broadcast\nfixture's receiver and epoch; default options.");
    batch(
        &p,
        "SP3_BATCH",
        &predict_batch(&sp3, &requests, PredictOptions::default()),
    );
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .expect("broadcast NAV");
    let bc = inputs::broadcast();
    let requests: Vec<PredictRequest> = bc
        .sat_ids
        .iter()
        .map(|s| (s.parse().expect("sat"), bc.receiver_m, bc.t_rx_j2000_s))
        .collect();
    batch(
        &p,
        "BROADCAST_BATCH",
        &predict_batch(&nav, &requests, PredictOptions::default()),
    );

    // Leap seconds.
    p.comment("Leap-second table at JD(UTC) 2459025.5 (2020-06-25).");
    p.f64("GPS_UTC_S_BITS", gps_utc_offset_s(2459025.5));
    p.f64("TAI_UTC_S_BITS", tai_utc_offset_s(2459025.5));

    // EGM96.
    let lat = 40.0 * std::f64::consts::PI / 180.0;
    let lon = -105.0 * std::f64::consts::PI / 180.0;
    let ortho = egm96_orthometric_height_m(1600.0, lat, lon);
    p.comment("Embedded EGM96 at 40 N, 105 W.");
    p.f64("EGM96_UNDULATION_M_BITS", egm96_undulation(lat, lon));
    p.f64("EGM96_ORTHOMETRIC_M_BITS", ortho);
    p.f64(
        "EGM96_ELLIPSOIDAL_M_BITS",
        egm96_ellipsoidal_height_m(ortho, lat, lon),
    );

    // Sun/Moon from 40 N, 105 W, 1.6 km at 2024-01-01T00:00:00Z.
    let station = GeodeticStationKm {
        latitude_deg: 40.0,
        longitude_deg: -105.0,
        altitude_km: 1.6,
    };
    let start_us = 1704067200i64 * 1_000_000;
    let end_us = start_us + 48 * 3600 * 1_000_000;
    let start = UtcInstant::from_unix_microseconds(start_us);
    let end = UtcInstant::from_unix_microseconds(end_us);
    let sun = sun_az_el(&station, start).expect("sun az-el");
    let moon = moon_az_el(&station, start).expect("moon az-el");
    let illum = moon_illumination(&station, start).expect("illumination");
    p.comment("sidereon_core::astro::bodies from 40 N, 105 W, 1.6 km at\n2024-01-01T00:00:00Z, and over the following 48 hours.");
    p.f64("SUN_AZIMUTH_DEG_BITS", sun.azimuth_deg);
    p.f64("SUN_ELEVATION_DEG_BITS", sun.elevation_deg);
    p.f64("SUN_RANGE_KM_BITS", sun.range_km);
    p.f64("MOON_ELEVATION_DEG_BITS", moon.elevation_deg);
    p.f64("MOON_RANGE_KM_BITS", moon.range_km);
    p.f64("MOON_ILLUMINATED_FRACTION_BITS", illum.illuminated_fraction);
    p.f64("MOON_PHASE_ANGLE_DEG_BITS", illum.phase_angle_deg);
    let bad = GeodeticStationKm {
        latitude_deg: 200.0,
        longitude_deg: -105.0,
        altitude_km: 1.6,
    };
    p.outcome("MOON_BAD_STATION", &moon_az_el(&bad, start));
    let crossings =
        find_moon_elevation_crossings(&station, start, end, MoonElevationOptions::default())
            .expect("crossings");
    let times: Vec<i128> = crossings
        .iter()
        .map(|c| i128::from(c.time.unix_microseconds()))
        .collect();
    p.ints("MOON_CROSSING_TIMES_US", "int64_t", &times);
    let transits = find_moon_transits(&station, start, end, 300.0, 1.0).expect("transits");
    let times: Vec<i128> = transits
        .iter()
        .map(|t| i128::from(t.time.unix_microseconds()))
        .collect();
    p.ints("MOON_TRANSIT_TIMES_US", "int64_t", &times);

    header_end(GUARD);
}
