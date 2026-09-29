//! tests/w5_cap015_pins.h: sidereon-core's results for the inputs
//! cap015_smoke.c passes (error metrics, sidereal repeat and filter, MIDAS,
//! clock power-law noise, sparse orbit fit, w-test constants, reliability
//! design, SBAS protection levels).

#[path = "w5_support/pins.rs"]
mod pins;

use pins::{variant_name, Pins};
use sidereon_core::araim::{AraimGeometry, AraimRow};
use sidereon_core::astro::frames::transforms as ft;
use sidereon_core::astro::propagator::api::{IntegratorOptions, PropagationContext};
use sidereon_core::astro::propagator::{ForceModelKind, IntegratorKind, StatePropagator};
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::scales::TimeScales;
use sidereon_core::astro::time::{Instant, JulianDateSplit, TimeScale};
use sidereon_core::clock_stability::{
    allan_deviation_power_law_slope, allan_variance_power_law_tau_exponent, fit_power_law_noise,
    modified_allan_deviation_power_law_slope, AllanResult, PowerLawNoiseOptions, PowerLawNoiseType,
    PowerLawOctaveDominance,
};
use sidereon_core::ephemeris::{OrbitFitCovariance, OrbitFitOptions, PreciseEphemerisSample};
use sidereon_core::error_metrics::{
    error_ellipse_from_enu_m2, horizontal_radius_at, metrics_from_ecef_covariance_m2,
    metrics_from_enu_covariance_m2, metrics_from_kinematic_solution,
    metrics_from_position_covariance, spherical_radius_at, vertical_radius_at,
    PositionErrorMetrics,
};
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::geometry::{line_of_sight_from_az_el_deg, LineOfSight};
use sidereon_core::sbas_pl::{
    sbas_protection_levels, SbasErrorModel, SbasKMultipliers, SbasSisError,
};
use sidereon_core::GnssSystem;
use std::collections::BTreeMap;
use valgen::{header_end, header_start};

const GUARD: &str = "SIDEREON_W5_CAP015_PINS_H";

fn metrics(p: &Pins, name: &str, m: &PositionErrorMetrics) {
    p.f64(&format!("{name}_CEP_M_BITS"), m.cep_m.radius_m);
    p.f64(&format!("{name}_R95_M_BITS"), m.r95_m.radius_m);
    p.bool(&format!("{name}_R95_APPROX_VALID"), m.r95_m.approx_valid);
    p.f64(&format!("{name}_SEP_M_BITS"), m.sep_m.radius_m);
    p.f64(&format!("{name}_DRMS_M_BITS"), m.drms_m);
}

fn main() {
    header_start("w5_cap015", GUARD);
    let p = Pins::new("W5_CAP015");

    // Error metrics.
    let sigma = 3.0_f64;
    let iso = [
        [sigma * sigma, 0.0, 0.0],
        [0.0, sigma * sigma, 0.0],
        [0.0, 0.0, sigma * sigma],
    ];
    p.comment("sidereon_core::error_metrics on the isotropic, elongated and rotated\ncovariances cap015_smoke.c builds.");
    let m = metrics_from_enu_covariance_m2(iso).expect("isotropic metrics");
    metrics(&p, "ISO", &m);
    let e = error_ellipse_from_enu_m2(iso).expect("isotropic ellipse");
    p.f64("ISO_ELLIPSE_SEMI_MAJOR_M_BITS", e.semi_major_m);
    p.f64("ISO_ELLIPSE_SEMI_MINOR_M_BITS", e.semi_minor_m);
    p.f64("ISO_ELLIPSE_ORIENTATION_RAD_BITS", e.orientation_rad);
    let h = horizontal_radius_at(iso, 0.95).expect("horizontal radius");
    p.f64("ISO_HORIZONTAL_R95_M_BITS", h.radius_m);
    p.bool("ISO_HORIZONTAL_R95_APPROX_VALID", h.approx_valid);
    let s = spherical_radius_at(iso, 0.5).expect("spherical radius");
    p.f64("ISO_SPHERICAL_R50_M_BITS", s.radius_m);
    p.f64(
        "ISO_VERTICAL_R50_M_BITS",
        vertical_radius_at(sigma * sigma, 0.5).expect("vertical radius"),
    );
    let pc = sidereon_core::geometry::PositionCovariance {
        ecef_m2: [
            [2.0 * sigma * sigma, 0.0, 0.0],
            [0.0, 2.0 * sigma * sigma, 0.0],
            [0.0, 0.0, 2.0 * sigma * sigma],
        ],
        enu_m2: iso,
    };
    metrics(
        &p,
        "POSITION_COV",
        &metrics_from_position_covariance(&pc).expect("position covariance metrics"),
    );
    let elongated = [[9.0, 2.0, 0.0], [2.0, 4.0, 0.0], [0.0, 0.0, 1.44]];
    let e = error_ellipse_from_enu_m2(elongated).expect("elongated ellipse");
    p.f64("ELONGATED_SEMI_MAJOR_M_BITS", e.semi_major_m);
    p.f64("ELONGATED_SEMI_MINOR_M_BITS", e.semi_minor_m);
    p.f64("ELONGATED_ORIENTATION_RAD_BITS", e.orientation_rad);
    let enu2 = [[5.0, 0.25, 0.1], [0.25, 2.0, -0.2], [0.1, -0.2, 1.25]];
    let ecef2 = [[1.25, 0.1, -0.2], [0.1, 5.0, 0.25], [-0.2, 0.25, 2.0]];
    metrics(
        &p,
        "ROTATED_ENU",
        &metrics_from_enu_covariance_m2(enu2).expect("rotated ENU metrics"),
    );
    metrics(
        &p,
        "ROTATED_ECEF",
        &metrics_from_ecef_covariance_m2(ecef2, Wgs84Geodetic::new(0.0, 0.0, 0.0).expect("rx"))
            .expect("rotated ECEF metrics"),
    );
    let kinematic = sidereon_core::precise_positioning::KinematicEpochSolution {
        position_m: [6378137.0, 0.0, 0.0],
        clock_m: 0.0,
        ztd_residual_m: 0.0,
        ambiguities_m: BTreeMap::new(),
        position_covariance_m2: ecef2,
        used_sats: Vec::new(),
        innovation_rms_m: 0.0,
        status: sidereon_core::precise_positioning::KinematicEpochStatus::Updated,
        ssr_bias_exclusions: Vec::new(),
        unplaced_observations: Vec::new(),
    };
    metrics(
        &p,
        "KINEMATIC",
        &metrics_from_kinematic_solution(&kinematic).expect("kinematic metrics"),
    );
    let non_psd = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]];
    let refused = metrics_from_enu_covariance_m2(non_psd);
    p.outcome("NON_PSD", &refused);
    p.variant_named(
        "NON_PSD_KIND",
        "SIDEREON_ERROR_METRICS_ERROR_KIND_",
        &refused.as_ref().err().map(variant_name).unwrap_or_default(),
    );

    // Sidereal repeat period and filter.
    p.comment("sidereon_core::sidereal: GPS repeat period and the two-sample filter.");
    p.f64(
        "SIDEREAL_GPS_PERIOD_S_BITS",
        sidereon_core::sidereal::repeat_period(GnssSystem::Gps).as_seconds(),
    );
    let mut options = sidereon_core::sidereal::SiderealFilterOptions::default();
    options.sample_interval =
        sidereon_core::astro::time::Duration::from_seconds(1.0).expect("interval");
    options.prior_periods = 1;
    options.min_coverage = 2;
    options.template_method = sidereon_core::sidereal::SiderealTemplateMethod::Mean;
    let filtered = sidereon_core::sidereal::sidereal_filter(
        &[10.0, 20.0],
        sidereon_core::astro::time::Duration::from_seconds(2.0).expect("period"),
        options,
    )
    .expect("sidereal filter");
    let under: Vec<i128> = filtered
        .under_covered
        .iter()
        .map(|flag| i128::from(*flag))
        .collect();
    p.ints("SIDEREAL_UNDER_COVERED", "bool", &under);

    // MIDAS on the synthetic linear ENU series.
    let rate = [0.01, -0.02, 0.005];
    let samples: Vec<_> = (0..5)
        .map(|i| {
            let dt = i as f64;
            sidereon_core::geodetic_time_series::PositionSample {
                epoch_year: 2020.0 + dt,
                position_m: [rate[0] * dt, rate[1] * dt, rate[2] * dt],
                covariance_m2: None,
            }
        })
        .collect();
    let series = sidereon_core::geodetic_time_series::PositionSeries {
        frame: sidereon_core::geodetic_time_series::PositionFrame::Enu,
        samples: &samples,
    };
    let velocity = sidereon_core::geodetic_time_series::velocity_midas(
        &series,
        sidereon_core::geodetic_time_series::MidasOptions::default(),
    )
    .expect("MIDAS");
    p.comment("sidereon_core::geodetic_time_series::velocity_midas, default options.");
    p.f64s("MIDAS_RATE_ENU_M_PER_YR_BITS", &velocity.rate_enu_m_per_yr);

    // Clock power-law noise.
    p.comment("sidereon_core::clock_stability power-law slopes and the one-point fit.");
    p.f64(
        "WHITE_FM_ADEV_SLOPE_BITS",
        allan_deviation_power_law_slope(PowerLawNoiseType::WhiteFM),
    );
    p.f64(
        "WHITE_FM_MDEV_SLOPE_BITS",
        modified_allan_deviation_power_law_slope(PowerLawNoiseType::WhiteFM),
    );
    p.int(
        "WHITE_FM_VARIANCE_TAU_EXPONENT",
        i128::from(allan_variance_power_law_tau_exponent(
            PowerLawNoiseType::WhiteFM,
        )),
    );
    let curve = AllanResult {
        tau_s: vec![1.0],
        deviation: vec![1.0],
        n: vec![1],
    };
    let mut o = PowerLawNoiseOptions::new(1.0, 0.5);
    o.slope_tolerance = 1.0e-12;
    o.scatter_tolerance = 1.0e-12;
    let fit = fit_power_law_noise(&curve, &curve, o).expect("power-law fit");
    p.int(
        "POWER_LAW_OCTAVE_COUNT",
        fit.dominant_per_octave.len() as i128,
    );
    let octave = &fit.dominant_per_octave[0];
    p.variant(
        "POWER_LAW_OCTAVE_DOMINANCE_KIND",
        "SIDEREON_POWER_LAW_OCTAVE_DOMINANCE_KIND_",
        &octave.dominance,
    );
    if let PowerLawOctaveDominance::Flagged(flag) = octave.dominance {
        p.variant(
            "POWER_LAW_OCTAVE_FLAG",
            "SIDEREON_POWER_LAW_OCTAVE_FLAG_",
            &flag,
        );
    } else {
        p.variant_named(
            "POWER_LAW_OCTAVE_FLAG",
            "SIDEREON_POWER_LAW_OCTAVE_FLAG_",
            "UnderSampled",
        );
    }

    // Sparse orbit fit: the two-epoch two-body truth, rotated to ITRS.
    let start = sidereon_core::astro::time::civil::j2000_seconds(2026, 6, 1, 0, 0, 0.0);
    let epochs = [start, start + 600.0];
    let mut integrator_options = IntegratorOptions::default();
    integrator_options.initial_step = 10.0;
    integrator_options.max_step = 60.0;
    integrator_options.dense_output = false;
    let propagator = StatePropagator {
        initial: CartesianState::new(start, [7078.0, 0.0, 820.0], [0.15, 7.35, 1.00]),
        force_model: ForceModelKind::TwoBody {
            mu_km3_s2: sidereon_core::astro::constants::MU_EARTH,
        },
        integrator: IntegratorKind::Dp54,
        options: integrator_options,
        drag: None,
        space_weather: None,
    };
    let states = propagator
        .ephemeris_with_context(&epochs, &PropagationContext::default())
        .expect("truth propagation");
    let scales = [
        TimeScales::from_utc(2026, 6, 1, 0, 0, 0.0).expect("scales 0"),
        TimeScales::from_utc(2026, 6, 1, 0, 10, 0.0).expect("scales 1"),
    ];
    let samples: Vec<PreciseEphemerisSample> = (0..2)
        .map(|i| {
            let r = &states[i].position_km;
            let (x, y, z) =
                ft::gcrs_to_itrs_compute(r[0], r[1], r[2], &scales[i], false).expect("ITRS");
            PreciseEphemerisSample {
                sat: "G11".parse().expect("G11"),
                epoch: instant_from_j2000_seconds(TimeScale::Utc, epochs[i]),
                position_ecef_m: [x * 1000.0, y * 1000.0, z * 1000.0],
                clock_s: None,
                clock_event: false,
            }
        })
        .collect();
    let mut fit_options = OrbitFitOptions::default();
    fit_options.force_model = ForceModelKind::TwoBody {
        mu_km3_s2: sidereon_core::astro::constants::MU_EARTH,
    };
    fit_options.integrator_options.initial_step = 10.0;
    fit_options.integrator_options.max_step = 60.0;
    fit_options.integrator_options.dense_output = false;
    fit_options.solver_options.gtol = 1.0e-15;
    fit_options.solver_options.ftol = 1.0e-15;
    fit_options.solver_options.xtol = 1.0e-15;
    fit_options.solver_options.max_nfev = 1200;
    let report = sidereon_core::ephemeris::fit_precise_ephemeris_sample_orbit(
        &samples,
        "G11".parse().expect("G11"),
        &fit_options,
    )
    .expect("sparse orbit fit");
    let fit = report.fits.values().next().expect("fit");
    p.comment("sidereon_core::ephemeris::fit_precise_ephemeris_sample_orbit on the two\nsamples cap015_smoke.c forms.");
    p.int("SPARSE_FIT_COUNT", report.fits.len() as i128);
    p.variant_named(
        "SPARSE_FIT_COVARIANCE_KIND",
        "SIDEREON_ORBIT_FIT_COVARIANCE_KIND_",
        match fit.covariance {
            OrbitFitCovariance::Estimated { .. } => "Estimated",
            OrbitFitCovariance::Unbounded => "Unbounded",
        },
    );
    p.variant(
        "SPARSE_FIT_TIER",
        "SIDEREON_OBSERVABILITY_TIER_",
        &fit.geometry_quality.tier,
    );
    p.int("SPARSE_LEDGER_COUNT", report.ledger.per_sat.len() as i128);
    let stats = report.ledger.per_sat.values().next().expect("ledger row");
    p.int("SPARSE_LEDGER_N", stats.n as i128);
    p.bool("SPARSE_LEDGER_LOW_SAMPLE_COUNT", stats.low_sample_count);

    // W-test constants.
    let w = sidereon_core::quality::wtest_noncentrality_components(0.001, 0.20).expect("w-test");
    p.comment("sidereon_core::quality::wtest_noncentrality_components(0.001, 0.20).");
    p.f64("WTEST_DELTA0_BITS", w.delta0);
    p.f64("WTEST_LAMBDA0_BITS", w.lambda0);

    // Reliability design.
    let rows = vec![
        sidereon_core::quality::RangeReliabilityRow {
            id: "rx_clock".to_string(),
            design_row: vec![1.0, 0.0],
            sigma_m: 1.0,
        },
        sidereon_core::quality::RangeReliabilityRow {
            id: "range_a".to_string(),
            design_row: vec![0.0, 1.0],
            sigma_m: 1.0,
        },
        sidereon_core::quality::RangeReliabilityRow {
            id: "range_b".to_string(),
            design_row: vec![0.0, 1.0],
            sigma_m: 1.0,
        },
    ];
    let report = sidereon_core::quality::reliability_design(
        &rows,
        &sidereon_core::quality::ReliabilityOptions::default(),
    )
    .expect("reliability design");
    p.comment("sidereon_core::quality::reliability_design, default options.");
    p.int(
        "RELIABILITY_OBS_COUNT",
        report.per_observation.len() as i128,
    );
    p.int("RELIABILITY_DOF", report.summary.dof as i128);
    p.int(
        "RELIABILITY_N_UNCHECKABLE",
        report.summary.n_uncheckable as i128,
    );
    p.f64(
        "RELIABILITY_SUM_REDUNDANCY_BITS",
        report.summary.sum_redundancy,
    );
    let redundancy: Vec<f64> = report
        .per_observation
        .iter()
        .map(|o| o.redundancy)
        .collect();
    p.f64s("RELIABILITY_REDUNDANCY_BITS", &redundancy);
    let flags = |f: &dyn Fn(&sidereon_core::quality::ObservationReliability) -> bool| -> Vec<i128> {
        report
            .per_observation
            .iter()
            .map(|o| i128::from(f(o)))
            .collect()
    };
    p.ints(
        "RELIABILITY_UNCHECKABLE",
        "bool",
        &flags(&|o| o.uncheckable),
    );
    p.ints(
        "RELIABILITY_HAS_MDB",
        "bool",
        &flags(&|o| o.mdb_m.is_some()),
    );
    p.ints(
        "RELIABILITY_HAS_EXTERNAL_ENU",
        "bool",
        &flags(&|o| o.external_enu_m.is_some()),
    );
    p.ints(
        "RELIABILITY_HAS_BIAS_TO_NOISE",
        "bool",
        &flags(&|o| o.bias_to_noise.is_some()),
    );

    // SBAS protection levels.
    let precision = SbasKMultipliers::PRECISION_APPROACH;
    let enroute = SbasKMultipliers::EN_ROUTE_NPA;
    p.comment("sidereon_core::sbas_pl K multipliers and protection levels for the five\nrows cap015_smoke.c builds.");
    p.f64("SBAS_PRECISION_K_H_BITS", precision.k_h);
    p.f64("SBAS_PRECISION_K_V_BITS", precision.k_v);
    p.f64("SBAS_ENROUTE_K_H_BITS", enroute.k_h);
    p.f64("SBAS_ENROUTE_K_V_BITS", enroute.k_v);
    let deg_to_rad = 0.017453292519943295;
    let receiver = Wgs84Geodetic::new(0.0, 0.0, 0.0).expect("receiver");
    let az_el = [
        [15.0, 15.0],
        [80.0, 70.0],
        [155.0, 25.0],
        [230.0, 55.0],
        [310.0, 35.0],
    ];
    let ids = ["G01", "G02", "G03", "G04", "G05"];
    let sigmas = [2.0, 1.0, 1.5, 1.2, 1.8];
    let geometry = AraimGeometry {
        rows: (0..5)
            .map(|i| {
                let los =
                    line_of_sight_from_az_el_deg(az_el[i][0], az_el[i][1], receiver).expect("los");
                AraimRow {
                    id: ids[i].parse().expect("sat"),
                    line_of_sight: LineOfSight::new(los.e_x, los.e_y, los.e_z),
                    system: GnssSystem::Gps,
                    elevation_rad: az_el[i][1] * deg_to_rad,
                }
            })
            .collect(),
        receiver,
        clock_systems: vec![GnssSystem::Gps],
        ut1_degraded: None,
    };
    let model = SbasErrorModel::new(
        (0..5)
            .map(|i| SbasSisError {
                id: ids[i].parse().expect("sat"),
                sigma_flt_m: sigmas[i],
                sigma_uire_m: 0.0,
                sigma_air_m: 0.0,
                sigma_tropo_m: 0.0,
            })
            .collect(),
    );
    let pl = sbas_protection_levels(&geometry, &model, precision).expect("SBAS PL");
    p.f64("SBAS_HPL_M_BITS", pl.hpl_m);
    p.f64("SBAS_VPL_M_BITS", pl.vpl_m);

    header_end(GUARD);
}

/// The binding's conversion of a J2000-seconds epoch to an instant
/// (src/lib.rs, instant_from_j2000_seconds).
fn instant_from_j2000_seconds(scale: TimeScale, j2000_s: f64) -> Instant {
    let whole_s = j2000_s.floor();
    let (jd_whole, day_fraction) =
        sidereon_core::astro::time::civil::split_julian_date_from_j2000_seconds(whole_s as i64);
    let fraction = day_fraction + (j2000_s - whole_s) / sidereon_core::constants::SECONDS_PER_DAY;
    Instant::from_julian_date(
        scale,
        JulianDateSplit::new(jd_whole, fraction).expect("split date"),
    )
}
