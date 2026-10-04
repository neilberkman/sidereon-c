//! rtk_rinex_smoke.c: sidereon-core's single-frequency static RINEX RTK
//! baseline, wide-lane-fixed baseline and static reference-station metadata
//! for the WTZR/WTZZ arcs, with the configurations that test sets, converted
//! as bindings/c/src/rtk.rs converts them.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::carrier_phase::CycleSlipOptions;
use sidereon_core::ephemeris::Sp3;
use sidereon_core::positioning::{
    solve_static_reference_station_rinex, StaticReferenceCarrierRinexOptions,
    StaticReferenceStationRinexOptions,
};
use sidereon_core::rinex::observations::RinexObs;
use sidereon_core::rtk::BaselineReferenceSelection;
use sidereon_core::rtk_filter::defaults::{
    AMBIGUITY_TOL_M, MAX_ITERATIONS, PARTIAL_MIN_AMBIGUITIES, POSITION_TOL_M, RATIO_THRESHOLD,
};
use sidereon_core::rtk_filter::{
    build_dual_frequency_rinex_rtk_arc, build_rinex_rtk_arc, solve_static_rtk_arc,
    solve_wide_lane_fixed_rtk_arc, CycleSlipPolicy, DynamicsModel, FixedSolveOpts, FloatSolveOpts,
    MeasModel, ResidualValidationOpts, RtkArcConfig, RtkArcPreprocessing, RtkDualCycleSlipConfig,
    RtkIonosphereFreeArcConfig, RtkRinexArcOptions, RtkRinexDualArcOptions, RtkStaticArcConfig,
    RtkWideLaneArcConfig, RtkWideLaneFixedArcConfig, RtkWideLaneFixedArcSolution,
    RtkWideLaneFixedArcSolveConfig, SearchOpts, StochasticModel, UpdateOpts,
    ValidatedFixedSolveOpts, WideLaneOptions,
};
use std::collections::BTreeMap;
use valgen::{core_fixtures, read, read_bytes};

/// WTZR marker, ECEF metres, as rtk_rinex_smoke.c states it.
const WTZR_MARKER_M: [f64; 3] = [4075580.3111, 931854.0543, 4801568.2808];

/// rtk_rinex_smoke.c's arp_position.
fn arp_position(marker: [f64; 3], obs: &RinexObs) -> [f64; 3] {
    let delta = obs.header().antenna_delta_hen_m.expect("antenna delta");
    let norm = (marker[0] * marker[0] + marker[1] * marker[1] + marker[2] * marker[2]).sqrt();
    let height = delta[0];
    [
        marker[0] + height * marker[0] / norm,
        marker[1] + height * marker[1] / norm,
        marker[2] + height * marker[2] / norm,
    ]
}

/// The WTZR model: code 2 m, phase 1 cm, Sagnac, simple elevation-weighted.
fn model() -> MeasModel {
    MeasModel {
        code_sigma_m: 2.0,
        phase_sigma_m: 0.01,
        sagnac: true,
        stochastic: StochasticModel::Simple {
            elevation_weighting: true,
        },
    }
}

/// default_rtk_arc_update_options_value.
fn update_opts() -> UpdateOpts {
    UpdateOpts {
        hold_sigma_m: AMBIGUITY_TOL_M,
        position_tol_m: POSITION_TOL_M,
        ambiguity_tol_m: AMBIGUITY_TOL_M,
        max_iterations: MAX_ITERATIONS,
        process_noise_baseline_sigma_m: 0.0,
        dynamics_model: DynamicsModel::ConstantPosition,
        float_only_systems: Vec::new(),
        report_residuals: false,
        receiver_antenna_corrections: None,
        ar_arming_sigma_m: None,
        search: SearchOpts {
            ratio_threshold: RATIO_THRESHOLD,
        },
    }
}

/// default_rtk_float_options, default_rtk_fixed_options and
/// default_rtk_residual_options.
fn default_validated() -> ValidatedFixedSolveOpts {
    ValidatedFixedSolveOpts {
        float: FloatSolveOpts {
            position_tol_m: POSITION_TOL_M,
            ambiguity_tol_m: AMBIGUITY_TOL_M,
            max_iterations: MAX_ITERATIONS,
        },
        fixed: FixedSolveOpts {
            position_tol_m: POSITION_TOL_M,
            ambiguity_tol_m: AMBIGUITY_TOL_M,
            max_iterations: MAX_ITERATIONS,
            ratio_threshold: RATIO_THRESHOLD,
            partial_ambiguity_resolution: false,
            partial_min_ambiguities: PARTIAL_MIN_AMBIGUITIES,
        },
        residual: ResidualValidationOpts {
            threshold_sigma: None,
            max_exclusions: 0,
        },
    }
}

fn def_vec3(name: &str, v: &[f64; 3]) {
    def_bits_array(name, v);
}

fn main() {
    let guard = "SIDEREON_W6_RTK_RINEX_PINS_H";
    valgen::header_start("w6_rtk_rinex", guard);
    let core = core_fixtures();
    let sp3 = Sp3::parse(&read_bytes(&format!(
        "{core}/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3"
    )))
    .expect("RTK SP3");
    let base = RinexObs::parse(&read(&format!(
        "{core}/obs/WTZR00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"
    )))
    .expect("WTZR");
    let rover = RinexObs::parse(&read(&format!(
        "{core}/obs/WTZZ00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"
    )))
    .expect("WTZZ");
    let base_arp = arp_position(WTZR_MARKER_M, &base);

    // test_static_rinex_rtk: set_wtzr_single_frequency_config.
    let mut arc_options = RtkRinexArcOptions::gps_l1_c();
    arc_options.max_epochs = Some(120);
    arc_options.min_common_satellites = 4;
    arc_options.include_prediction_time = false;
    let arc = build_rinex_rtk_arc(&sp3, &base, &rover, &arc_options).expect("single-frequency arc");
    let config = RtkStaticArcConfig::new(
        RtkArcConfig::new(
            base_arp,
            BaselineReferenceSelection::Auto,
            model(),
            30.0,
            30.0,
            [0.0; 3],
            arc.wavelengths_m.clone(),
            arc.offsets_m.clone(),
            update_opts(),
            RtkArcPreprocessing {
                cycle_slip: Some(CycleSlipPolicy::SplitArc),
                hatch_window_cap: None,
                elevation_mask_deg: None,
            },
        ),
        default_validated(),
    );
    let solution = solve_static_rtk_arc(&arc.epochs, &config).expect("static RINEX RTK");
    comment("Single-frequency static RINEX RTK baseline.");
    def_vec3(
        "W6_RTKR_STATIC_FLOAT_BASELINE_BITS",
        &solution.float_solution.baseline_m,
    );
    def_vec3(
        "W6_RTKR_STATIC_FIXED_BASELINE_BITS",
        &solution.fixed_solution.fixed_solution.baseline_m,
    );
    def_size(
        "W6_RTKR_STATIC_AMBIGUITY_COUNT",
        solution.float_solution.ambiguities_m.len(),
    );
    def_size(
        "W6_RTKR_STATIC_N_OBSERVATIONS",
        solution.float_solution.n_observations,
    );
    let search = &solution.fixed_solution.fixed_solution.search;
    def(
        "W6_RTKR_STATIC_INTEGER_STATUS",
        c_enum("SIDEREON_RTK_INTEGER_STATUS", &search.integer_status),
    );
    def_bool("W6_RTKR_STATIC_HAS_RATIO", search.integer_ratio.is_some());
    def_bits(
        "W6_RTKR_STATIC_RATIO_BITS",
        search.integer_ratio.unwrap_or(0.0),
    );
    def_size(
        "W6_RTKR_STATIC_SPLIT_ARC_COUNT",
        solution.split_cycle_slip_arcs.len(),
    );
    def_size(
        "W6_RTKR_STATIC_AMBIGUITY_ID_COUNT",
        solution.ambiguity_ids.len(),
    );

    // test_static_reference_station: the carrier-only solve with 24 epochs.
    let mut ref_arc_options = RtkRinexArcOptions::gps_l1_c();
    ref_arc_options.max_epochs = Some(24);
    ref_arc_options.min_common_satellites = 4;
    ref_arc_options.include_prediction_time = false;
    let mut ref_validated = default_validated();
    ref_validated.float = FloatSolveOpts {
        position_tol_m: 1.0e-4,
        ambiguity_tol_m: 1.0e-4,
        max_iterations: 10,
    };
    ref_validated.fixed = FixedSolveOpts {
        position_tol_m: 1.0e-4,
        ambiguity_tol_m: 1.0e-4,
        max_iterations: 10,
        ratio_threshold: 3.0,
        partial_ambiguity_resolution: true,
        partial_min_ambiguities: 4,
    };
    let reference_options = StaticReferenceStationRinexOptions::new(
        None,
        Some(StaticReferenceCarrierRinexOptions::new(
            ref_arc_options,
            RtkStaticArcConfig::new(
                RtkArcConfig::new(
                    base_arp,
                    BaselineReferenceSelection::Auto,
                    model(),
                    30.0,
                    30.0,
                    [0.0; 3],
                    BTreeMap::new(),
                    BTreeMap::new(),
                    update_opts(),
                    RtkArcPreprocessing {
                        cycle_slip: Some(CycleSlipPolicy::SplitArc),
                        hatch_window_cap: None,
                        elevation_mask_deg: None,
                    },
                ),
                ref_validated,
            ),
        )),
        true,
    );
    let station =
        solve_static_reference_station_rinex(&sp3, &base, &rover, base_arp, &reference_options)
            .expect("static reference station");
    let carrier = station.carrier_solution.as_ref();
    comment("Static reference-station metadata, first diagnostic and mode report.");
    def(
        "W6_RTKR_REF_MODE",
        c_enum("SIDEREON_STATIC_REFERENCE_STATION_MODE", &station.mode),
    );
    def(
        "W6_RTKR_REF_FIX_STATUS",
        c_enum("SIDEREON_STATIC_REFERENCE_FIX_STATUS", &station.fix_status),
    );
    def_bool("W6_RTKR_REF_HAS_CARRIER", carrier.is_some());
    def_bool("W6_RTKR_REF_HAS_CODE", station.code_solution.is_some());
    def_bool("W6_RTKR_REF_HAS_GEODETIC", station.geodetic.is_some());
    def_size("W6_RTKR_REF_DIAGNOSTIC_COUNT", station.diagnostics.len());
    def_size(
        "W6_RTKR_REF_CARRIER_DIAGNOSTIC_COUNT",
        carrier.map_or(0, |c| c.diagnostics.len()),
    );
    def_size("W6_RTKR_REF_MODE_REPORT_COUNT", station.mode_reports.len());
    def(
        "W6_RTKR_REF_CARRIER_INTEGER_STATUS",
        carrier.map_or("SIDEREON_RTK_INTEGER_STATUS_NOT_FIXED".to_string(), |c| {
            c_enum("SIDEREON_RTK_INTEGER_STATUS", &c.integer_status)
        }),
    );
    def_bool(
        "W6_RTKR_REF_HAS_CARRIER_RATIO",
        carrier.and_then(|c| c.integer_ratio).is_some(),
    );
    let first = &station.diagnostics[0];
    def(
        "W6_RTKR_REF_DIAG0_MODE",
        c_enum("SIDEREON_STATIC_REFERENCE_STATION_MODE", &first.mode),
    );
    def_size("W6_RTKR_REF_DIAG0_USED_SATS", first.used_satellites.len());
    def_bool(
        "W6_RTKR_REF_DIAG0_HAS_CODE_RMS",
        first.code_residual_rms_m.is_some(),
    );
    def_bool(
        "W6_RTKR_REF_DIAG0_HAS_PHASE_RMS",
        first.phase_residual_rms_m.is_some(),
    );
    let report = &station.mode_reports[0];
    def(
        "W6_RTKR_REF_REPORT0_MODE",
        c_enum("SIDEREON_STATIC_REFERENCE_STATION_MODE", &report.mode),
    );
    def(
        "W6_RTKR_REF_REPORT0_STATUS",
        c_enum("SIDEREON_STATIC_REFERENCE_MODE_STATUS", &report.status),
    );
    def_size("W6_RTKR_REF_REPORT0_USED_EPOCHS", report.used_epochs);
    def_size("W6_RTKR_REF_REPORT0_SKIPPED_EPOCHS", report.skipped_epochs);
    def_size(
        "W6_RTKR_REF_REPORT0_USED_MEASUREMENTS",
        report.used_measurements,
    );
    def_bool("W6_RTKR_REF_REPORT0_HAS_ERROR", report.error.is_some());

    // test_wide_lane_rinex_rtk: set_wtzr_wide_lane_config, converted by
    // rtk_rinex_wide_lane_fixed_config_from_c.
    let mut dual_options = RtkRinexDualArcOptions::gps_l1_l2_cw();
    dual_options.max_epochs = Some(120);
    dual_options.min_common_satellites = 4;
    dual_options.include_prediction_time = false;
    let dual = build_dual_frequency_rinex_rtk_arc(&sp3, &base, &rover, &dual_options)
        .expect("dual-frequency arc");
    let mut wide_lane_opts = WideLaneOptions::new(2, 0.5);
    wide_lane_opts.skip_short_fragments = false;
    let wide_lane = RtkWideLaneArcConfig::new(
        base_arp,
        BaselineReferenceSelection::Auto,
        wide_lane_opts,
        Some(RtkDualCycleSlipConfig::new(
            CycleSlipPolicy::DropSatellite,
            CycleSlipOptions::default(),
        )),
    );
    let ionosphere_free =
        RtkIonosphereFreeArcConfig::new(base_arp, [0.0; 3], BaselineReferenceSelection::Auto, true);
    let static_config = RtkStaticArcConfig::new(
        RtkArcConfig::new(
            base_arp,
            BaselineReferenceSelection::Auto,
            model(),
            30.0,
            30.0,
            [0.0; 3],
            BTreeMap::new(),
            BTreeMap::new(),
            update_opts(),
            RtkArcPreprocessing::default(),
        ),
        default_validated(),
    );
    let config = RtkWideLaneFixedArcConfig::new(
        wide_lane,
        ionosphere_free,
        RtkWideLaneFixedArcSolveConfig::Static(static_config),
    );
    let solved = match solve_wide_lane_fixed_rtk_arc(&dual.epochs, &config).expect("wide lane") {
        RtkWideLaneFixedArcSolution::Static(inner) => inner,
        RtkWideLaneFixedArcSolution::Sequential(_) => panic!("expected a static solution"),
    };
    comment("Wide-lane-fixed static RINEX RTK baseline and metadata.");
    def_vec3(
        "W6_RTKR_WL_FLOAT_BASELINE_BITS",
        &solved.solution.float_solution.baseline_m,
    );
    def_vec3(
        "W6_RTKR_WL_FIXED_BASELINE_BITS",
        &solved.solution.fixed_solution.fixed_solution.baseline_m,
    );
    def_size(
        "W6_RTKR_WL_AMBIGUITY_COUNT",
        solved.solution.float_solution.ambiguities_m.len(),
    );
    def_size(
        "W6_RTKR_WL_N_OBSERVATIONS",
        solved.solution.float_solution.n_observations,
    );
    let search = &solved.solution.fixed_solution.fixed_solution.search;
    def(
        "W6_RTKR_WL_INTEGER_STATUS",
        c_enum("SIDEREON_RTK_INTEGER_STATUS", &search.integer_status),
    );
    def_bool("W6_RTKR_WL_HAS_RATIO", search.integer_ratio.is_some());
    def_bits("W6_RTKR_WL_RATIO_BITS", search.integer_ratio.unwrap_or(0.0));
    def_bool("W6_RTKR_WL_FIXED", solved.metadata.wide_lane_fixed);
    def_size(
        "W6_RTKR_WL_AMBIGUITY_CYCLE_COUNT",
        solved.metadata.wide_lane_ambiguities_cycles.len(),
    );
    valgen::header_end(guard);
}
