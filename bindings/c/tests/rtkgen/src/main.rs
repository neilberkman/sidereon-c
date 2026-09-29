// Print tests/rtk_fixture.h: the synthetic RTK inputs smoke.c feeds the C
// float and fixed RTK solves, and sidereon-core's results for them. See
// Cargo.toml for how to run it.

use sidereon_core::constants::{C_M_S, F_L1_HZ};
use sidereon_core::rtk_filter::{
    solve_fixed_baseline_validated, solve_float_baseline, AmbiguityScale, AmbiguitySet, Epoch,
    FixedSolveOpts, FloatSolveOpts, MeasModel, ReceiverAntennaCalibration,
    ReceiverAntennaCorrections, ResidualValidationOpts, SatMeas, StochasticModel,
    ValidatedFixedSolveOpts,
};
use std::collections::BTreeMap;

/// The sidereon-core revision the goldens are written from, which this
/// crate's Cargo.toml pins.
const CORE_REVISION: &str = include_str!("../../CORE_REVISION");

const SAT_IDS: [&str; 5] = ["G01", "G02", "G03", "G04", "G05"];
const BASE_ECEF_M: [f64; 3] = [4_075_580.0, 931_854.0, 4_801_568.0];
/// The rover the synthetic observations are formed at, as rover minus base.
const TRUE_BASELINE_M: [f64; 3] = [1.2, -0.85, 0.91];
const INITIAL_BASELINE_M: [f64; 3] = [-30.0, 25.0, -10.0];
const SAT_POS_M: [[f64; 3]; 5] = [
    [15_000_000.0, 7_000_000.0, 21_000_000.0],
    [-12_000_000.0, 18_000_000.0, 19_000_000.0],
    [20_000_000.0, -10_000_000.0, 17_000_000.0],
    [-19_000_000.0, -13_000_000.0, 20_000_000.0],
    [9_000_000.0, 22_000_000.0, 16_000_000.0],
];
/// The float solve's rover phase offset from the code, metres, per satellite.
const FLOAT_PHASE_OFFSET_M: [f64; 5] = [0.0, 0.6, -1.4, 1.0, -0.3];
/// The fixed solve's rover phase offset from the code, whole L1 cycles.
const FIXED_PHASE_CYCLES: [i64; 5] = [0, 4, -7, 9, -3];

fn bits(value: f64) -> String {
    format!("UINT64_C({:#018x})", value.to_bits())
}

fn range_m(sat: [f64; 3], receiver: [f64; 3]) -> f64 {
    let d = [
        sat[0] - receiver[0],
        sat[1] - receiver[1],
        sat[2] - receiver[2],
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// One epoch as smoke.c's fill_rtk_epoch builds it from these inputs: G01 is
/// the reference, base phase equals base code, and every position is the
/// satellite's.
fn epoch(base_code: &[f64; 5], rover_code: &[f64; 5], rover_phase: &[f64; 5]) -> Epoch {
    let row = |i: usize| SatMeas {
        sat: SAT_IDS[i].to_string(),
        sd_ambiguity_id: SAT_IDS[i].to_string(),
        base_code_m: base_code[i],
        base_phase_m: base_code[i],
        rover_code_m: rover_code[i],
        rover_phase_m: rover_phase[i],
        base_tx_pos: SAT_POS_M[i],
        rover_tx_pos: SAT_POS_M[i],
        pos: SAT_POS_M[i],
    };
    Epoch {
        references: vec![row(0)],
        nonref: (1..5).map(row).collect(),
        velocity_mps: None,
        dt_s: 0.0,
    }
}

fn zero_calibration() -> ReceiverAntennaCalibration {
    ReceiverAntennaCalibration {
        pco_neu_m: [0.0; 3],
        noazi_pcv_m: vec![(0.0, 0.0), (90.0, 0.0)],
        azi_pcv_m: Vec::new(),
    }
}

fn print_bits_array(name: &str, count: &str, values: &[f64]) {
    println!("static const uint64_t {name}[{count}] = {{");
    for value in values {
        println!("    {},", bits(*value));
    }
    println!("}};");
}

fn main() {
    let rover = [
        BASE_ECEF_M[0] + TRUE_BASELINE_M[0],
        BASE_ECEF_M[1] + TRUE_BASELINE_M[1],
        BASE_ECEF_M[2] + TRUE_BASELINE_M[2],
    ];
    let wavelength_m = C_M_S / F_L1_HZ;
    let mut base_code = [0.0; 5];
    let mut rover_code = [0.0; 5];
    let mut float_phase = [0.0; 5];
    let mut fixed_phase = [0.0; 5];
    for i in 0..5 {
        base_code[i] = range_m(SAT_POS_M[i], BASE_ECEF_M);
        rover_code[i] = range_m(SAT_POS_M[i], rover);
        float_phase[i] = rover_code[i] + FLOAT_PHASE_OFFSET_M[i];
        fixed_phase[i] = rover_code[i] + FIXED_PHASE_CYCLES[i] as f64 * wavelength_m;
    }

    // smoke.c's configure_rtk_model, configure_rtk_float_options, the fixed
    // options it sets and sidereon_rtk_residual_validation_options_init.
    let model = MeasModel {
        code_sigma_m: 0.3,
        phase_sigma_m: 0.003,
        sagnac: false,
        stochastic: StochasticModel::Simple {
            elevation_weighting: false,
        },
    };
    let float_opts = FloatSolveOpts {
        position_tol_m: 1.0e-3,
        ambiguity_tol_m: 1.0e-6,
        max_iterations: 10,
    };
    let fixed_opts = FixedSolveOpts {
        position_tol_m: 1.0e-3,
        ambiguity_tol_m: 1.0e-6,
        max_iterations: 10,
        ratio_threshold: 3.0,
        partial_ambiguity_resolution: false,
        partial_min_ambiguities: 4,
    };
    let residual_opts = ResidualValidationOpts {
        threshold_sigma: None,
        max_exclusions: 0,
    };
    let antenna = ReceiverAntennaCorrections {
        base: zero_calibration(),
        rover: zero_calibration(),
    };
    let ambiguity_ids: Vec<String> = SAT_IDS[1..].iter().map(|id| id.to_string()).collect();

    let float = solve_float_baseline(
        &[epoch(&base_code, &rover_code, &float_phase)],
        BASE_ECEF_M,
        &ambiguity_ids,
        INITIAL_BASELINE_M,
        &model,
        float_opts,
        Some(&antenna),
    )
    .expect("float RTK solve");

    let satellites: BTreeMap<String, String> = ambiguity_ids
        .iter()
        .map(|id| (id.clone(), id.clone()))
        .collect();
    let wavelengths: BTreeMap<String, f64> = ambiguity_ids
        .iter()
        .map(|id| (id.clone(), wavelength_m))
        .collect();
    let offsets: BTreeMap<String, f64> = ambiguity_ids.iter().map(|id| (id.clone(), 0.0)).collect();
    let float_only_systems: Vec<String> = Vec::new();
    let validated = solve_fixed_baseline_validated(
        &[epoch(&base_code, &rover_code, &fixed_phase)],
        BASE_ECEF_M,
        AmbiguitySet {
            ids: &ambiguity_ids,
            satellites: &satellites,
            scale: AmbiguityScale {
                wavelengths_m: &wavelengths,
                offsets_m: &offsets,
            },
            float_only_systems: &float_only_systems,
        },
        INITIAL_BASELINE_M,
        &model,
        ValidatedFixedSolveOpts {
            float: float_opts,
            fixed: fixed_opts,
            residual: residual_opts,
        },
        Some(&antenna),
    )
    .expect("fixed RTK solve");
    let fixed = &validated.fixed_solution;

    let float_ids: Vec<&str> = float
        .ambiguities_m
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(float_ids, SAT_IDS[1..], "float ambiguity order");
    let fixed_ids: Vec<&str> = fixed
        .fixed_ambiguities_cycles
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(fixed_ids, SAT_IDS[1..], "fixed ambiguity order");
    let fixed_m: BTreeMap<&str, f64> = fixed
        .fixed_ambiguities_m
        .iter()
        .map(|(id, value)| (id.as_str(), *value))
        .collect();
    let search = &fixed.search;

    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("/* Generated by tests/rtkgen from sidereon-core. The inputs are the");
    println!(" * synthetic observations rtkgen forms; the results are sidereon-core's");
    println!(" * float and validated fixed baseline solves of them under smoke.c's");
    println!(" * configuration. Every double is an exact IEEE-754 bit pattern. Do not");
    println!(" * edit by hand; rerun the generator. */");
    println!("#ifndef SIDEREON_C_TESTS_CFIX_RTK_FIXTURE_H");
    println!("#define SIDEREON_C_TESTS_CFIX_RTK_FIXTURE_H");
    println!();
    println!("#include <stdint.h>");
    println!();
    println!("#define CFIX_RTK_SAT_COUNT 5");
    println!("#define CFIX_RTK_NONREF_COUNT 4");
    println!();
    let quoted = |ids: &[&str]| {
        ids.iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!(
        "static const char *const CFIX_RTK_SAT_IDS[CFIX_RTK_SAT_COUNT] = {{{}}};",
        quoted(&SAT_IDS)
    );
    println!(
        "static const char *const CFIX_RTK_AMBIGUITY_IDS[CFIX_RTK_NONREF_COUNT] = {{{}}};",
        quoted(&SAT_IDS[1..])
    );
    println!();
    print_bits_array("CFIX_RTK_BASE_ECEF_M_BITS", "3", &BASE_ECEF_M);
    print_bits_array("CFIX_RTK_INITIAL_BASELINE_M_BITS", "3", &INITIAL_BASELINE_M);
    println!("static const uint64_t CFIX_RTK_SAT_POS_M_BITS[CFIX_RTK_SAT_COUNT][3] = {{");
    for pos in SAT_POS_M {
        println!(
            "    {{{}, {}, {}}},",
            bits(pos[0]),
            bits(pos[1]),
            bits(pos[2])
        );
    }
    println!("}};");
    println!();
    print_bits_array("CFIX_RTK_BASE_CODE_BITS", "CFIX_RTK_SAT_COUNT", &base_code);
    print_bits_array(
        "CFIX_RTK_ROVER_CODE_BITS",
        "CFIX_RTK_SAT_COUNT",
        &rover_code,
    );
    print_bits_array(
        "CFIX_RTK_FLOAT_ROVER_PHASE_BITS",
        "CFIX_RTK_SAT_COUNT",
        &float_phase,
    );
    print_bits_array(
        "CFIX_RTK_FIXED_ROVER_PHASE_BITS",
        "CFIX_RTK_SAT_COUNT",
        &fixed_phase,
    );
    println!();
    println!(
        "static const uint64_t CFIX_RTK_L1_WAVELENGTH_M_BITS = {};",
        bits(wavelength_m)
    );
    println!();
    println!("/* sidereon_core::rtk_filter::solve_float_baseline. */");
    print_bits_array("CFIX_RTK_FLOAT_BASELINE_BITS", "3", &float.baseline_m);
    let float_ambiguities: Vec<f64> = float.ambiguities_m.iter().map(|(_, v)| *v).collect();
    print_bits_array(
        "CFIX_RTK_FLOAT_AMBIGUITY_BITS",
        "CFIX_RTK_NONREF_COUNT",
        &float_ambiguities,
    );
    println!(
        "static const uint64_t CFIX_RTK_FLOAT_CODE_RMS_BITS = {};",
        bits(float.code_rms_m)
    );
    println!(
        "static const uint64_t CFIX_RTK_FLOAT_PHASE_RMS_BITS = {};",
        bits(float.phase_rms_m)
    );
    println!(
        "static const uint64_t CFIX_RTK_FLOAT_WEIGHTED_RMS_BITS = {};",
        bits(float.weighted_rms_m)
    );
    println!("#define CFIX_RTK_FLOAT_ITERATIONS {}", float.iterations);
    println!(
        "#define CFIX_RTK_FLOAT_N_OBSERVATIONS {}",
        float.n_observations
    );
    println!(
        "#define CFIX_RTK_FLOAT_GEOMETRY_REDUNDANCY {}",
        float.geometry_quality.redundancy
    );
    println!(
        "#define CFIX_RTK_FLOAT_GEOMETRY_RANK {}",
        float.geometry_quality.rank
    );
    println!();
    println!("/* sidereon_core::rtk_filter::solve_fixed_baseline_validated. */");
    print_bits_array("CFIX_RTK_FIXED_BASELINE_BITS", "3", &fixed.baseline_m);
    let cycles: Vec<String> = fixed
        .fixed_ambiguities_cycles
        .iter()
        .map(|(_, c)| c.to_string())
        .collect();
    println!(
        "static const int64_t CFIX_RTK_FIXED_AMBIGUITY_CYCLES[CFIX_RTK_NONREF_COUNT] = {{{}}};",
        cycles.join(", ")
    );
    let fixed_meters: Vec<f64> = fixed
        .fixed_ambiguities_cycles
        .iter()
        .map(|(id, _)| fixed_m[id.as_str()])
        .collect();
    print_bits_array(
        "CFIX_RTK_FIXED_AMBIGUITY_M_BITS",
        "CFIX_RTK_NONREF_COUNT",
        &fixed_meters,
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_CODE_RMS_BITS = {};",
        bits(fixed.code_rms_m)
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_PHASE_RMS_BITS = {};",
        bits(fixed.phase_rms_m)
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_WEIGHTED_RMS_BITS = {};",
        bits(fixed.weighted_rms_m)
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_RATIO_BITS = {};",
        bits(search.integer_ratio.expect("integer ratio"))
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_BEST_SCORE_BITS = {};",
        bits(search.integer_best_score.expect("best score"))
    );
    println!(
        "static const uint64_t CFIX_RTK_FIXED_SECOND_BEST_SCORE_BITS = {};",
        bits(search.integer_second_best_score.expect("second best score"))
    );
    println!("#define CFIX_RTK_FIXED_ITERATIONS {}", fixed.iterations);
    println!(
        "#define CFIX_RTK_FIXED_N_OBSERVATIONS {}",
        fixed.n_observations
    );
    // The fixed route reports the geometry of its own float solve.
    println!(
        "#define CFIX_RTK_FIXED_GEOMETRY_REDUNDANCY {}",
        validated.float_solution.geometry_quality.redundancy
    );
    println!(
        "#define CFIX_RTK_FIXED_GEOMETRY_RANK {}",
        validated.float_solution.geometry_quality.rank
    );
    println!(
        "#define CFIX_RTK_FIXED_FREE_AMBIGUITY_COUNT {}",
        fixed.free_ambiguities_m.len()
    );
    println!(
        "#define CFIX_RTK_FIXED_INTEGER_CANDIDATES {}",
        search.integer_candidates
    );
    println!();
    println!("#endif");
}
