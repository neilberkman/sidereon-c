// Print tests/w3_merged_pins.h: sidereon-core's results for the inputs
// merged_smoke.c builds (a NeQuick-G ray, a range FDE design with one planted
// outlier, synthetic RTK arcs, the committed PPP arc and a captured RTCM 1046
// frame).

use sidereon_core::astro::time::ExactEpoch;
use sidereon_core::atmosphere::ionosphere::{
    nequick_g_delay_m, nequick_g_stec_tecu, NequickGRayEval,
};
use sidereon_core::ephemeris::{Sp3, Sp3InterpolationOptions};
use sidereon_core::ppp_corrections::CivilDateTime;
use sidereon_core::precise_positioning::defaults as ppp_defaults;
use sidereon_core::precise_positioning::{
    solve_ppp_auto_init_fixed, solve_ppp_auto_init_float,
    FixedAmbiguityOptions as PppFixedAmbiguityOptions, FixedSolveConfig as PppFixedSolveConfig,
    FloatEpoch as PppFloatEpoch, FloatObservation as PppFloatObservation,
    FloatSolveConfig as PppFloatSolveConfig, FloatSolveOptions as PppFloatSolveOptions,
    MeasurementWeights as PppMeasurementWeights, PppAutoInitOptions, RangeCorrections,
    TroposphereOptions as PppTroposphereOptions,
};
use sidereon_core::quality::{raim_fde_design, RangeFdeOptions, RangeFdeRow};
use sidereon_core::rinex::nav::GalileoNequickCoeffs;
use sidereon_core::rtk::BaselineReferenceSelection;
use sidereon_core::rtk_filter::defaults::{
    AMBIGUITY_TOL_M, CODE_SIGMA_M, MAX_ITERATIONS, PARTIAL_MIN_AMBIGUITIES, PHASE_SIGMA_M,
    POSITION_TOL_M, RATIO_THRESHOLD,
};
use sidereon_core::rtk_filter::{
    fix_wide_lane_rtk_arc, prepare_ionosphere_free_rtk_arc, solve_rtk_arc, solve_static_rtk_arc,
    CycleSlipPolicy, DynamicsModel, FixedSolveOpts, FloatSolveOpts, MeasModel,
    ResidualValidationOpts, RtkArcConfig, RtkArcEpoch, RtkArcObservation, RtkArcPreprocessing,
    RtkArcSolution, RtkDualFrequencyArcEpoch, RtkDualFrequencyObservation,
    RtkDualFrequencySatelliteObservation, RtkIonosphereFreeArcConfig, RtkStaticArcConfig,
    RtkWideLaneArcConfig, SearchOpts, StochasticModel, UpdateOpts, ValidatedFixedSolveOpts,
    WideLaneOptions,
};
use sidereon_core::GnssSatelliteId;
use std::collections::BTreeMap;
use std::str::FromStr;
use valgen::{bits, c_string, header_end, header_start, read, read_bytes, tests_path};

const P: &str = "W3M";

fn def(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}_{name} {value}");
}

fn def_bits(name: &str, value: f64) {
    def(name, bits(value));
}

fn def_str(name: &str, value: &str) {
    def(name, c_string(value));
}

fn def_bool(name: &str, value: bool) {
    def(name, if value { "true" } else { "false" });
}

fn def_count(name: &str, value: usize) {
    def(name, format!("((size_t)UINT64_C({value}))"));
}

fn def_bits_array(name: &str, values: &[f64]) {
    println!(
        "static const uint64_t {P}_{name}[{}] = {{ {} }};",
        values.len().max(1),
        if values.is_empty() {
            "0".to_string()
        } else {
            values
                .iter()
                .map(|v| bits(*v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
}

fn def_str_array(name: &str, values: &[String]) {
    println!(
        "static const char *const {P}_{name}[{}] = {{ {} }};",
        values.len().max(1),
        if values.is_empty() {
            "\"\"".to_string()
        } else {
            values
                .iter()
                .map(|v| c_string(v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
}

fn variant(debug: &str) -> String {
    let cut = debug.find(['{', ' ', '(']).unwrap_or(debug.len());
    debug[..cut].trim().to_string()
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

/// The C constant the binding writes for an engine enum value: the binding's
/// constants are the engine variant names under the C enum's prefix.
fn c_const(prefix: &str, value: &impl std::fmt::Debug) -> String {
    format!("{prefix}_{}", snake(&variant(&format!("{value:?}"))))
}

fn emit_geometry(name: &str, geometry: &sidereon_core::geometry_quality::GeometryQuality) {
    def(
        &format!("{name}_TIER"),
        c_const("SIDEREON_OBSERVABILITY_TIER", &geometry.tier),
    );
    def(&format!("{name}_REDUNDANCY"), geometry.redundancy);
    def_count(&format!("{name}_RANK"), geometry.rank);
    def_bits(
        &format!("{name}_CONDITION_NUMBER"),
        geometry.condition_number,
    );
    def_bits(&format!("{name}_GDOP"), geometry.gdop);
    def_bool(&format!("{name}_RAIM_CHECKABLE"), geometry.raim_checkable);
    def_bool(
        &format!("{name}_COVARIANCE_VALIDATED"),
        geometry.covariance_validated,
    );
}

// ------------------------------------------------------------------ NeQuick

fn nequick_section() {
    // test_nequick_slant's ray and E1 carrier.
    let ray = NequickGRayEval {
        month: 6,
        utc_hours: 12.0,
        station_lon_deg: 9.0,
        station_lat_deg: 45.0,
        station_height_m: 0.0,
        satellite_lon_deg: 12.0,
        satellite_lat_deg: 40.0,
        satellite_height_m: 20000000.0,
    };
    let f_e1 = 1.57542e9;
    let zero = GalileoNequickCoeffs {
        ai0: 0.0,
        ai1: 0.0,
        ai2: 0.0,
    };
    let broadcast = GalileoNequickCoeffs {
        ai0: 80.0,
        ai1: 0.1,
        ai2: 0.05,
    };
    println!("/* test_nequick_slant: sidereon_core NeQuick-G slant TEC and delay. */");
    def_bits(
        "NEQUICK_STEC_DEFAULT",
        nequick_g_stec_tecu(&zero, &ray).expect("NeQuick-G STEC"),
    );
    def_bits(
        "NEQUICK_DELAY_DEFAULT",
        nequick_g_delay_m(&zero, &ray, f_e1).expect("NeQuick-G delay"),
    );
    def_bits(
        "NEQUICK_STEC_BROADCAST",
        nequick_g_stec_tecu(&broadcast, &ray).expect("NeQuick-G broadcast STEC"),
    );
    println!();
}

// ------------------------------------------------------------------ RAIM/FDE

const RAIM_DX_TRUE: [f64; 4] = [1.0, 2.0, 3.0, 4.0];
const RAIM_DESIGN: [[f64; 4]; 6] = [
    [0.10, 0.20, 0.97, 1.0],
    [0.90, 0.10, 0.42, 1.0],
    [-0.50, 0.60, 0.62, 1.0],
    [0.30, -0.80, 0.52, 1.0],
    [-0.70, -0.30, 0.65, 1.0],
    [0.20, 0.50, 0.84, 1.0],
];
const RAIM_IDS: [&str; 6] = ["G01", "G02", "G03", "G04", "G05", "G06"];
const RAIM_OUTLIER: usize = 2;
const RAIM_OUTLIER_M: f64 = 50.0;

fn raim_section() {
    println!("/* test_raim_fde: the design, the clean state and the planted outlier, and");
    println!(" * sidereon_core::quality::raim_fde_design of them. */");
    println!(
        "static const double {P}_RAIM_DX_TRUE[4] = {{ {:?}, {:?}, {:?}, {:?} }};",
        RAIM_DX_TRUE[0], RAIM_DX_TRUE[1], RAIM_DX_TRUE[2], RAIM_DX_TRUE[3]
    );
    println!("static const double {P}_RAIM_DESIGN[6][4] = {{");
    for row in RAIM_DESIGN {
        println!(
            "    {{ {:?}, {:?}, {:?}, {:?} }},",
            row[0], row[1], row[2], row[3]
        );
    }
    println!("}};");
    def("RAIM_OUTLIER", RAIM_OUTLIER);
    println!(
        "static const double {P}_RAIM_OUTLIER_M = {:?};",
        RAIM_OUTLIER_M
    );
    let rows: Vec<RangeFdeRow> = (0..6)
        .map(|i| {
            // The C test sums the row against the clean state in this order.
            let mut residual = 0.0;
            for k in 0..4 {
                residual += RAIM_DESIGN[i][k] * RAIM_DX_TRUE[k];
            }
            if i == RAIM_OUTLIER {
                residual += RAIM_OUTLIER_M;
            }
            RangeFdeRow {
                id: RAIM_IDS[i].to_string(),
                residual_m: residual,
                design_row: RAIM_DESIGN[i].to_vec(),
                weight: 1.0,
            }
        })
        .collect();
    let defaults = RangeFdeOptions::default();
    def_bits("RAIM_OPTIONS_P_FA", defaults.p_fa);
    def_count("RAIM_OPTIONS_MAX_EXCLUSIONS", defaults.max_exclusions);
    def_count("RAIM_OPTIONS_MIN_REDUNDANCY", defaults.min_redundancy);
    def_bits(
        "RAIM_OPTIONS_MAX_EXCLUSION_RMS_M",
        defaults.max_exclusion_rms_m,
    );
    let result = raim_fde_design(&rows, &defaults).expect("range FDE");
    def_count("RAIM_STATE_DIM", result.state_correction.len());
    def_bits_array("RAIM_STATE_CORRECTION", &result.state_correction);
    let covariance: Vec<f64> = result
        .state_covariance
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect();
    def_count("RAIM_COVARIANCE_LEN", covariance.len());
    def_bits_array("RAIM_COVARIANCE", &covariance);
    let test = &result.global_test;
    def_bool("RAIM_TEST_TESTABLE", test.testable);
    def_bool("RAIM_TEST_FAULT_DETECTED", test.fault_detected);
    def_bool("RAIM_TEST_HAS_THRESHOLD", test.threshold.is_some());
    def_bits("RAIM_TEST_THRESHOLD", test.threshold.unwrap_or(0.0));
    def_bits("RAIM_TEST_WEIGHTED_SUM_SQUARES", test.weighted_sum_squares);
    def("RAIM_TEST_DOF", test.dof);
    def_count("RAIM_ITERATIONS", result.iterations);
    def_count("RAIM_EXCLUDED_COUNT", result.excluded.len());
    def_str_array("RAIM_EXCLUDED", &result.excluded);
    def_count("RAIM_DIAGNOSTIC_COUNT", result.diagnostics.len());
    println!(
        "static const bool {P}_RAIM_DIAGNOSTIC_EXCLUDED[{}] = {{ {} }};",
        result.diagnostics.len(),
        result
            .diagnostics
            .iter()
            .map(|d| d.excluded.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let post: Vec<f64> = result
        .diagnostics
        .iter()
        .map(|d| d.post_fit_residual_m)
        .collect();
    let normalized: Vec<f64> = result
        .diagnostics
        .iter()
        .map(|d| d.normalized_residual)
        .collect();
    def_bits_array("RAIM_DIAGNOSTIC_POST_FIT", &post);
    def_bits_array("RAIM_DIAGNOSTIC_NORMALIZED", &normalized);
    println!();
}

// -------------------------------------------------------------------- RTK arc

struct ArcSat {
    id: &'static str,
    pos: [f64; 3],
    cycles: i64,
}

const ARC_SATS: [ArcSat; 5] = [
    ArcSat {
        id: "G01",
        pos: [15000000.0, 7000000.0, 21000000.0],
        cycles: 0,
    },
    ArcSat {
        id: "G02",
        pos: [-12000000.0, 18000000.0, 19000000.0],
        cycles: 4,
    },
    ArcSat {
        id: "G03",
        pos: [20000000.0, -10000000.0, 17000000.0],
        cycles: -7,
    },
    ArcSat {
        id: "G04",
        pos: [-19000000.0, -13000000.0, 20000000.0],
        cycles: 9,
    },
    ArcSat {
        id: "G05",
        pos: [9000000.0, 22000000.0, 16000000.0],
        cycles: -3,
    },
];
const ARC_BASE: [f64; 3] = [-2700000.0, -4300000.0, 3850000.0];
const ARC_BASELINE: [f64; 3] = [12.0, -7.0, 5.0];

fn arc_lambda() -> f64 {
    let c = 299792458.0;
    let f_l1 = 1575420000.0;
    c / f_l1
}

fn range(sat: [f64; 3], receiver: [f64; 3]) -> f64 {
    let mut sum = 0.0;
    for k in 0..3 {
        sum += (sat[k] - receiver[k]) * (sat[k] - receiver[k]);
    }
    sum.sqrt()
}

/// The base and rover observations of test_rtk_arc, formed as the C test forms
/// them; the rover G02 observation of `lli_epoch` carries LLI 1 when given.
fn arc_epochs(lli_epoch: Option<usize>) -> Vec<RtkArcEpoch> {
    let rover = [
        ARC_BASE[0] + ARC_BASELINE[0],
        ARC_BASE[1] + ARC_BASELINE[1],
        ARC_BASE[2] + ARC_BASELINE[2],
    ];
    let lambda = arc_lambda();
    let positions: BTreeMap<String, [f64; 3]> = ARC_SATS
        .iter()
        .map(|sat| (sat.id.to_string(), sat.pos))
        .collect();
    (0..2)
        .map(|epoch| {
            let base: Vec<RtkArcObservation> = ARC_SATS
                .iter()
                .map(|sat| {
                    let db = range(sat.pos, ARC_BASE);
                    RtkArcObservation {
                        satellite_id: sat.id.to_string(),
                        ambiguity_id: sat.id.to_string(),
                        code_m: db,
                        phase_m: db,
                        lli: None,
                    }
                })
                .collect();
            let rover: Vec<RtkArcObservation> = ARC_SATS
                .iter()
                .enumerate()
                .map(|(i, sat)| {
                    let dr = range(sat.pos, rover);
                    RtkArcObservation {
                        satellite_id: sat.id.to_string(),
                        ambiguity_id: sat.id.to_string(),
                        code_m: dr,
                        phase_m: dr + sat.cycles as f64 * lambda,
                        lli: (lli_epoch == Some(epoch) && i == 1).then_some(1),
                    }
                })
                .collect();
            RtkArcEpoch {
                base,
                rover,
                satellite_positions_m: positions.clone(),
                base_satellite_positions_m: BTreeMap::new(),
                rover_satellite_positions_m: BTreeMap::new(),
                velocity_mps: None,
                prediction_time_s: None,
                prediction_epoch: None,
            }
        })
        .collect()
}

/// sidereon_rtk_measurement_model_init.
fn default_model() -> MeasModel {
    MeasModel {
        code_sigma_m: CODE_SIGMA_M,
        phase_sigma_m: PHASE_SIGMA_M,
        sagnac: true,
        stochastic: StochasticModel::Simple {
            elevation_weighting: false,
        },
    }
}

/// sidereon_rtk_arc_update_options_init with report_residuals set.
fn arc_update_opts() -> UpdateOpts {
    UpdateOpts {
        hold_sigma_m: AMBIGUITY_TOL_M,
        position_tol_m: POSITION_TOL_M,
        ambiguity_tol_m: AMBIGUITY_TOL_M,
        max_iterations: MAX_ITERATIONS,
        process_noise_baseline_sigma_m: 0.0,
        dynamics_model: DynamicsModel::ConstantPosition,
        float_only_systems: Vec::new(),
        report_residuals: true,
        receiver_antenna_corrections: None,
        ar_arming_sigma_m: None,
        search: SearchOpts {
            ratio_threshold: RATIO_THRESHOLD,
        },
    }
}

fn arc_config(with_split_ids: bool, preprocessing: RtkArcPreprocessing) -> RtkArcConfig {
    let lambda = arc_lambda();
    let mut wavelengths: BTreeMap<String, f64> = ARC_SATS
        .iter()
        .map(|sat| (sat.id.to_string(), lambda))
        .collect();
    let mut offsets: BTreeMap<String, f64> = ARC_SATS
        .iter()
        .map(|sat| (sat.id.to_string(), 0.0))
        .collect();
    if with_split_ids {
        for id in ["G02@rover#1", "G02@rover#2"] {
            wavelengths.insert(id.to_string(), lambda);
            offsets.insert(id.to_string(), 0.0);
        }
    }
    RtkArcConfig::new(
        ARC_BASE,
        BaselineReferenceSelection::Auto,
        default_model(),
        100.0,
        100.0,
        [0.0; 3],
        wavelengths,
        offsets,
        arc_update_opts(),
        preprocessing,
    )
}

fn no_preprocessing() -> RtkArcPreprocessing {
    RtkArcPreprocessing {
        cycle_slip: None,
        hatch_window_cap: None,
        elevation_mask_deg: None,
    }
}

fn arc_section() {
    println!("/* test_rtk_arc: sidereon_core::rtk_filter arc drivers of the synthetic arc. */");
    let solution: RtkArcSolution =
        solve_rtk_arc(&arc_epochs(None), &arc_config(false, no_preprocessing())).expect("RTK arc");
    def_count("ARC_EPOCH_COUNT", solution.epochs.len());
    let epoch = &solution.epochs[1];
    def_count(
        "ARC_E1_USED_SATELLITE_COUNT",
        epoch.used_satellite_ids.len(),
    );
    def_count("ARC_E1_SD_AMBIGUITY_COUNT", epoch.sd_ambiguities_m.len());
    def_count("ARC_E1_FIXED_ID_COUNT", epoch.fixed_ids.len());
    def_bool("ARC_E1_INTEGER_FIXED", epoch.integer_fixed);
    def_count("ARC_E1_RESIDUAL_COUNT", epoch.residuals.len());
    def_bits_array("ARC_E1_REPORTED_BASELINE", &epoch.reported_baseline_m);
    def_str_array("ARC_E1_USED_SATELLITES", &epoch.used_satellite_ids);
    let sd: Vec<f64> = epoch.sd_ambiguities_m.iter().map(|(_, v)| *v).collect();
    let sd_ids: Vec<String> = epoch
        .sd_ambiguities_m
        .iter()
        .map(|(id, _)| id.clone())
        .collect();
    def_bits_array("ARC_E1_SD_AMBIGUITIES", &sd);
    def_str_array("ARC_E1_SD_AMBIGUITY_IDS", &sd_ids);
    def_str_array("ARC_E1_FIXED_IDS", &epoch.fixed_ids);
    def_count("ARC_REFERENCE_COUNT", solution.references.len());
    let (system, reference) = solution.references.iter().next().expect("one reference");
    def_str("ARC_REFERENCE0_SYSTEM", system);
    def_str("ARC_REFERENCE0_ID", reference);
    def_bits_array("ARC_FINAL_BASELINE", &solution.final_state.baseline_m);
    def_count("ARC_FINAL_EPOCH_COUNT", solution.final_state.epoch_count);

    let static_config = RtkStaticArcConfig::new(
        arc_config(false, no_preprocessing()),
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
        },
    );
    let static_solution =
        solve_static_rtk_arc(&arc_epochs(None), &static_config).expect("static RTK arc");
    def_bits_array(
        "STATIC_FLOAT_BASELINE",
        &static_solution.float_solution.baseline_m,
    );
    let fixed = &static_solution.fixed_solution;
    def_bits_array("STATIC_FIXED_BASELINE", &fixed.fixed_solution.baseline_m);
    def_count(
        "STATIC_FIXED_AMBIGUITY_COUNT",
        fixed.fixed_solution.fixed_ambiguities_cycles.len(),
    );
    def(
        "STATIC_INTEGER_STATUS",
        c_const(
            "SIDEREON_RTK_INTEGER_STATUS",
            &fixed.fixed_solution.search.integer_status,
        ),
    );
    emit_geometry(
        "STATIC_FIXED_GEOMETRY",
        &fixed.float_solution.geometry_quality,
    );
    emit_geometry("STATIC_GEOMETRY", &static_solution.geometry_quality);
    def_count(
        "STATIC_AMBIGUITY_SATELLITE_COUNT",
        static_solution.ambiguity_satellites.len(),
    );

    let split = solve_rtk_arc(
        &arc_epochs(Some(1)),
        &arc_config(
            true,
            RtkArcPreprocessing {
                cycle_slip: Some(CycleSlipPolicy::SplitArc),
                hatch_window_cap: Some(8),
                elevation_mask_deg: Some(-20.0),
            },
        ),
    )
    .expect("RTK arc, split cycle slip");
    def_count("SPLIT_ARC_COUNT", split.split_cycle_slip_arcs.len());
    let arcs = &split.split_cycle_slip_arcs;
    let n = arcs.len().max(1);
    let join = |values: Vec<String>| {
        if values.is_empty() {
            "0".to_string()
        } else {
            values.join(", ")
        }
    };
    println!(
        "static const uint32_t {P}_SPLIT_ARC_RECEIVERS[{n}] = {{ {} }};",
        join(
            arcs.iter()
                .map(|a| c_const("SIDEREON_RTK_CYCLE_SLIP_RECEIVER", &a.receiver))
                .collect()
        )
    );
    def_str_array(
        "SPLIT_ARC_SATELLITES",
        &arcs
            .iter()
            .map(|a| a.satellite_id.clone())
            .collect::<Vec<_>>(),
    );
    def_str_array(
        "SPLIT_ARC_AMBIGUITIES",
        &arcs
            .iter()
            .map(|a| a.ambiguity_id.clone())
            .collect::<Vec<_>>(),
    );
    for (name, values) in [
        (
            "SPLIT_ARC_STARTS",
            arcs.iter().map(|a| a.start_epoch_index).collect::<Vec<_>>(),
        ),
        (
            "SPLIT_ARC_ENDS",
            arcs.iter().map(|a| a.end_epoch_index).collect::<Vec<_>>(),
        ),
        (
            "SPLIT_ARC_N_EPOCHS",
            arcs.iter().map(|a| a.n_epochs).collect::<Vec<_>>(),
        ),
    ] {
        println!(
            "static const size_t {P}_{name}[{n}] = {{ {} }};",
            join(values.iter().map(|v| v.to_string()).collect())
        );
    }
    def_count("SPLIT_MASKED_COUNT", split.elevation_masked_sats.len());
    def_str_array("SPLIT_MASKED", &split.elevation_masked_sats);
    def_count("SPLIT_COVARIANCE_LEN", split.measurement_covariance.len());
    def_bits_array("SPLIT_COVARIANCE", &split.measurement_covariance);

    let drop = solve_rtk_arc(
        &arc_epochs(Some(1)),
        &arc_config(
            false,
            RtkArcPreprocessing {
                cycle_slip: Some(CycleSlipPolicy::DropSatellite),
                hatch_window_cap: None,
                elevation_mask_deg: None,
            },
        ),
    )
    .expect("RTK arc, drop satellite");
    def_count("DROP_COUNT", drop.dropped_sats.len());
    def_str_array("DROPPED", &drop.dropped_sats);
    println!();
}

// ------------------------------------------------------ dual-frequency arcs

fn dual_obs(id: &str, p1_m: f64, p2_m: f64, phi1_cycles: f64) -> RtkDualFrequencyObservation {
    RtkDualFrequencyObservation {
        ambiguity_id: id.to_string(),
        p1_m,
        p2_m,
        phi1_cycles,
        phi2_cycles: 0.0,
        f1_hz: 1575420000.0,
        f2_hz: 1227600000.0,
        lli1: None,
        lli2: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn dual_sat(
    id: &str,
    base_p1: f64,
    base_p2: f64,
    base_phi1: f64,
    rover_p1: f64,
    rover_p2: f64,
    rover_phi1: f64,
) -> RtkDualFrequencySatelliteObservation {
    RtkDualFrequencySatelliteObservation {
        satellite_id: id.to_string(),
        base: dual_obs(id, base_p1, base_p2, base_phi1),
        rover: dual_obs(id, rover_p1, rover_p2, rover_phi1),
    }
}

fn dual_epochs() -> Vec<RtkDualFrequencyArcEpoch> {
    let positions: BTreeMap<String, [f64; 3]> = [
        ("G01", [14350000.0, 3190000.0, 21440000.0]),
        ("G02", [20000000.0, 3000000.0, 18000000.0]),
        ("G03", [9000000.0, 9000000.0, 22000000.0]),
        ("G04", [16000000.0, -4000000.0, 21000000.0]),
    ]
    .into_iter()
    .map(|(id, pos)| (id.to_string(), pos))
    .collect();
    (0..3)
        .map(|e| RtkDualFrequencyArcEpoch {
            jd_whole: 2460100.5,
            jd_fraction: 0.25,
            epoch_sort_key: Some(format!("{e:03}")),
            gap_time_s: Some(e as f64),
            gap_epoch: Some(
                ExactEpoch::from_j2000_seconds(e as f64)
                    .expect("finite whole-second dual-frequency gap epoch"),
            ),
            observations: vec![
                dual_sat(
                    "G01", 20000020.0, 20000022.0, 2.0, 20000050.0, 20000052.5, 5.0,
                ),
                dual_sat(
                    "G02", 20000010.0, 20000012.0, 1.0, 20000042.0, 20000044.5, 7.0,
                ),
                dual_sat(
                    "G03", 19999980.0, 19999982.0, -2.0, 20000005.0, 20000007.5, 0.0,
                ),
                dual_sat(
                    "G04", 20000040.0, 20000042.0, 4.0, 20000073.0, 20000075.5, 8.0,
                ),
            ],
            satellite_positions_m: positions.clone(),
            base_satellite_positions_m: BTreeMap::new(),
            rover_satellite_positions_m: BTreeMap::new(),
            velocity_mps: None,
            prediction_time_s: None,
            prediction_epoch: None,
        })
        .collect()
}

fn dual_section() {
    println!("/* test_rtk_dual_arc_drivers: the wide-lane fix and ionosphere-free");
    println!(" * preparation of the synthetic dual-frequency arc. */");
    let base = [3512900.0, 780500.0, 5248700.0];
    let mut options = WideLaneOptions::new(2, 0.5);
    options.skip_short_fragments = false;
    let epochs = dual_epochs();
    let wide_lane = fix_wide_lane_rtk_arc(
        &epochs,
        &RtkWideLaneArcConfig::new(base, BaselineReferenceSelection::Auto, options, None),
    )
    .expect("wide-lane arc");
    def_count("WL_EPOCH_COUNT", wide_lane.epochs.len());
    emit_geometry("WL_GEOMETRY", &wide_lane.geometry_quality);
    def_count("WL_CYCLE_COUNT", wide_lane.wide_lane_cycles.len());
    let ids: Vec<String> = wide_lane.wide_lane_cycles.keys().cloned().collect();
    def_str_array("WL_CYCLE_IDS", &ids);
    println!(
        "static const int64_t {P}_WL_CYCLES[{}] = {{ {} }};",
        wide_lane.wide_lane_cycles.len().max(1),
        wide_lane
            .wide_lane_cycles
            .values()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    def_count("WL_REFERENCE_COUNT", wide_lane.references.len());

    let iono_free = prepare_ionosphere_free_rtk_arc(
        &epochs,
        &wide_lane.wide_lane_cycles,
        &RtkIonosphereFreeArcConfig::new(base, [0.0; 3], BaselineReferenceSelection::Auto, false),
    )
    .expect("ionosphere-free arc");
    def_count("IF_EPOCH_COUNT", iono_free.epochs.len());
    def_count("IF_WAVELENGTH_COUNT", iono_free.wavelengths_m.len());
    let wavelengths: Vec<f64> = iono_free.wavelengths_m.values().copied().collect();
    def_bits_array("IF_WAVELENGTHS", &wavelengths);
    def_count("IF_OFFSET_COUNT", iono_free.offsets_m.len());
    let offsets: Vec<f64> = iono_free.offsets_m.values().copied().collect();
    def_bits_array("IF_OFFSETS", &offsets);
    let first = &iono_free.epochs[0];
    def_count("IF_E0_BASE_COUNT", first.base.len());
    def_count("IF_E0_ROVER_COUNT", first.rover.len());
    def_count("IF_E0_POSITION_COUNT", first.satellite_positions_m.len());
    def_str("IF_E0_BASE0_SAT", &first.base[0].satellite_id);
    println!();
}

// --------------------------------------------------------------- PPP auto-init

fn ppp_section() {
    let json: serde_json::Value =
        serde_json::from_str(&read(&tests_path("fixtures/ppp_esbc.json"))).expect("PPP JSON");
    let sp3_name = json["sp3_file"].as_str().expect("sp3_file");
    // sidereon_sp3_load: default interpolation options.
    let sp3 = Sp3::parse(&read_bytes(&tests_path(&format!(
        "fixtures/sp3/{sp3_name}"
    ))))
    .expect("PPP SP3")
    .with_interpolation_options(Sp3InterpolationOptions::default());
    let f = |v: &serde_json::Value| v.as_f64().expect("number");
    let epochs: Vec<PppFloatEpoch> = json["epochs"]
        .as_array()
        .expect("epochs")
        .iter()
        .map(|epoch| {
            let civil = &epoch["civil"];
            PppFloatEpoch {
                epoch: CivilDateTime {
                    year: civil["year"].as_i64().expect("year") as i32,
                    month: civil["month"].as_i64().expect("month") as u8,
                    day: civil["day"].as_i64().expect("day") as u8,
                    hour: civil["hour"].as_i64().expect("hour") as u8,
                    minute: civil["minute"].as_i64().expect("minute") as u8,
                    second: f(&civil["second"]),
                },
                jd_whole: f(&epoch["jd_whole"]),
                jd_fraction: f(&epoch["jd_fraction"]),
                t_rx_j2000_s: f(&epoch["t_rx_j2000_s"]),
                observations: epoch["observations"]
                    .as_array()
                    .expect("observations")
                    .iter()
                    .map(|obs| {
                        let sat = GnssSatelliteId::from_str(
                            obs["satellite_id"].as_str().expect("satellite"),
                        )
                        .expect("satellite token");
                        PppFloatObservation {
                            sat,
                            satellite_id: sat.to_string(),
                            ambiguity_id: obs["ambiguity_id"]
                                .as_str()
                                .expect("ambiguity")
                                .to_string(),
                            code_m: f(&obs["code_m"]),
                            phase_m: f(&obs["phase_m"]),
                            freq1_hz: f(&obs["freq1_hz"]),
                            freq2_hz: f(&obs["freq2_hz"]),
                            glonass_channel: None,
                            signals: None,
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    // The configs test_ppp_auto_init builds: the fixture's code and phase
    // weights over the init defaults, disabled troposphere, no corrections,
    // default options, no elevation cutoff and no residual screen.
    let weights = &json["config"]["weights"];
    let measurement_weights = || PppMeasurementWeights {
        code: f(&weights["code"]),
        phase: f(&weights["phase"]),
        elevation_weighting: false,
    };
    let corrections = || RangeCorrections {
        receiver_antenna: None,
        sat_clock_relativity: false,
        satellite_clock: None,
        ppp: Default::default(),
    };
    let options = || {
        let mut o = PppFloatSolveOptions::default();
        o.max_iterations = ppp_defaults::MAX_ITERATIONS;
        o.position_tolerance_m = ppp_defaults::POSITION_TOLERANCE_M;
        o.clock_tolerance_m = ppp_defaults::CLOCK_TOLERANCE_M;
        o.ambiguity_tolerance_m = ppp_defaults::AMBIGUITY_TOLERANCE_M;
        o.ztd_tolerance_m = ppp_defaults::ZTD_TOLERANCE_M;
        o
    };
    let float_config = || {
        PppFloatSolveConfig::new(
            measurement_weights(),
            PppTroposphereOptions::disabled(),
            corrections(),
            options(),
            None,
            false,
            false,
        )
    };
    let ambiguity_json = &json["fixed_config"]["ambiguity"];
    let map = |value: &serde_json::Value| -> BTreeMap<String, f64> {
        value
            .as_object()
            .expect("map")
            .iter()
            .map(|(k, v)| (k.clone(), f(v)))
            .collect()
    };
    let ratio = f(&ambiguity_json["ratio_threshold"]);
    let mut ambiguity = PppFixedAmbiguityOptions::new(ratio);
    ambiguity.wavelengths_m = map(&ambiguity_json["wavelengths_m"]);
    ambiguity.offsets_m = map(&ambiguity_json["offsets_m"]);
    ambiguity.ratio_threshold = ratio;
    let fixed_config = PppFixedSolveConfig::new(
        measurement_weights(),
        PppTroposphereOptions::disabled(),
        corrections(),
        options(),
        None,
        ambiguity,
        false,
    );
    let auto = PppAutoInitOptions::default();
    println!("/* test_ppp_auto_init: sidereon_core::precise_positioning auto-initialised");
    println!(" * PPP of the committed ESBC arc. */");
    def_bool("PPP_AUTO_HAS_INITIAL_GUESS", auto.initial_guess.is_some());
    def_bits("PPP_AUTO_SPP_PRESSURE_HPA", auto.spp_met.pressure_hpa);
    let float =
        solve_ppp_auto_init_float(&sp3, &epochs, PppAutoInitOptions::default(), float_config())
            .expect("PPP auto-init float");
    def_bits_array("PPP_AUTO_FLOAT_POSITION", &float.position_m);
    let fixed = solve_ppp_auto_init_fixed(
        &sp3,
        &epochs,
        PppAutoInitOptions::default(),
        float_config(),
        fixed_config,
    )
    .expect("PPP auto-init fixed");
    def_bits_array("PPP_AUTO_FIXED_POSITION", &fixed.position_m);
    println!();
}

// ------------------------------------------------------------- RTCM decode

/// The captured RTCM 1046 frame test_rtcm_construct decodes.
const REAL_1046: [u8; 69] = [
    0xd3, 0x00, 0x3f, 0x41, 0x60, 0xd5, 0xe8, 0x07, 0x6b, 0x06, 0xc9, 0x41, 0xe0, 0x3f, 0xfe, 0xd3,
    0xff, 0xe3, 0x39, 0x17, 0xf3, 0xa4, 0x90, 0xe9, 0x84, 0xd2, 0x08, 0x9b, 0xf4, 0xf4, 0x01, 0x10,
    0x30, 0xb0, 0x34, 0x3a, 0xa8, 0x13, 0xab, 0x5d, 0x41, 0xef, 0xff, 0xb7, 0xe4, 0x4f, 0xe8, 0xcf,
    0xff, 0x52, 0x77, 0xd0, 0xb0, 0x11, 0xa2, 0x41, 0x63, 0x97, 0xff, 0xff, 0xfc, 0x22, 0x80, 0x14,
    0x07, 0x00, 0x80, 0x0a, 0x8e,
];

fn rtcm_section() {
    println!("/* test_rtcm_construct: the captured 1046 frame and sidereon_core's decode. */");
    println!(
        "static const uint8_t {P}_REAL_1046[{}] = {{ {} }};",
        REAL_1046.len(),
        REAL_1046
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let messages = sidereon_core::rtcm::decode_messages(&REAL_1046).expect("decode 1046");
    def_count("REAL_1046_MESSAGE_COUNT", messages.len());
    let message = &messages[0];
    def(
        "REAL_1046_KIND",
        c_const("SIDEREON_RTCM_MESSAGE_KIND", message),
    );
    def("REAL_1046_MESSAGE_NUMBER", message.message_number());
    match message {
        sidereon_core::rtcm::Message::GalileoInavEphemeris(eph) => {
            def("REAL_1046_SATELLITE_ID", eph.satellite_id);
            def("REAL_1046_WEEK_NUMBER", eph.week_number);
            def("REAL_1046_IOD_NAV", eph.iod_nav);
            def("REAL_1046_SQRT_A", format!("UINT64_C({})", eph.sqrt_a));
            def(
                "REAL_1046_ECCENTRICITY",
                format!("UINT64_C({})", eph.eccentricity),
            );
        }
        other => panic!("the 1046 frame decodes as {other:?}"),
    }
    println!();
}

fn main() {
    let guard = "SIDEREON_W3_MERGED_PINS_H";
    header_start("w3_merged", guard);
    nequick_section();
    raim_section();
    arc_section();
    dual_section();
    rtcm_section();
    ppp_section();
    header_end(guard);
}
