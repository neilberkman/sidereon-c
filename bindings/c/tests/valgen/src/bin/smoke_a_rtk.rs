// smoke.c exercise_rtk_surface: the solve metadata tests/rtkgen does not pin
// (convergence, solve status, geometry quality, integer status and presence)
// and the singular-geometry refusal. The inputs are read from
// tests/rtk_fixture.h, which the smoke consumes, and solved with the same
// sidereon-core calls and options as rtkgen.

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::rtk_filter::{
    solve_fixed_baseline_validated, solve_float_baseline, AmbiguityScale, AmbiguitySet, Epoch,
    FixedSolveOpts, FloatSolveOpts, MeasModel, ReceiverAntennaCalibration,
    ReceiverAntennaCorrections, ResidualValidationOpts, SatMeas, StochasticModel,
    ValidatedFixedSolveOpts,
};
use std::collections::BTreeMap;
use support::*;
use valgen::{header_end, header_start, read, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_RTK_PINS_H";
const SAT_IDS: [&str; 5] = ["G01", "G02", "G03", "G04", "G05"];

/// The doubles a `static const uint64_t NAME...` bit-pattern array of
/// rtk_fixture.h holds, in order.
fn header_doubles(header: &str, name: &str) -> Vec<f64> {
    let start = header
        .find(&format!(" {name}["))
        .or_else(|| header.find(&format!(" {name} ")))
        .unwrap_or_else(|| panic!("{name} in rtk_fixture.h"));
    let end = start + header[start..].find(';').expect("end of initializer");
    let body = &header[start..end];
    let body = &body[body.find('=').expect("initializer") + 1..];
    let mut values = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find("0x") {
        let digits: String = rest[at + 2..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        values.push(f64::from_bits(
            u64::from_str_radix(&digits, 16).expect("hex bits"),
        ));
        rest = &rest[at + 2 + digits.len()..];
    }
    values
}

fn zero_calibration() -> ReceiverAntennaCalibration {
    ReceiverAntennaCalibration {
        pco_neu_m: [0.0; 3],
        noazi_pcv_m: vec![(0.0, 0.0), (90.0, 0.0)],
        azi_pcv_m: Vec::new(),
    }
}

/// smoke.c fill_rtk_epoch: G01 is the reference, base phase equals base code.
fn epoch(
    base_code: &[f64],
    rover_code: &[f64],
    rover_phase: &[f64],
    positions: &[[f64; 3]],
) -> Epoch {
    let row = |i: usize| SatMeas {
        sat: SAT_IDS[i].to_string(),
        sd_ambiguity_id: SAT_IDS[i].to_string(),
        base_code_m: base_code[i],
        base_phase_m: base_code[i],
        rover_code_m: rover_code[i],
        rover_phase_m: rover_phase[i],
        base_tx_pos: positions[i],
        rover_tx_pos: positions[i],
        pos: positions[i],
    };
    Epoch {
        references: vec![row(0)],
        nonref: (1..5).map(row).collect(),
        velocity_mps: None,
        dt_s: 0.0,
    }
}

/// The binding's rtk_used_satellite_tokens count: distinct residual
/// satellites.
fn used_satellites<'a>(ids: impl Iterator<Item = &'a str>) -> usize {
    ids.collect::<std::collections::BTreeSet<_>>().len()
}

fn geometry_pins(prefix: &str, quality: &sidereon_core::geometry_quality::GeometryQuality) {
    def_variant(&format!("{prefix}_TIER"), &quality.tier);
    def_bits(
        &format!("{prefix}_CONDITION_NUMBER_BITS"),
        quality.condition_number,
    );
    def_bits(&format!("{prefix}_GDOP_BITS"), quality.gdop);
    def_bool(&format!("{prefix}_RAIM_CHECKABLE"), quality.raim_checkable);
    def_bool(
        &format!("{prefix}_COVARIANCE_VALIDATED"),
        quality.covariance_validated,
    );
}

fn main() {
    let header = read(&tests_path("rtk_fixture.h"));
    let base_ecef = header_doubles(&header, "CFIX_RTK_BASE_ECEF_M_BITS");
    let initial = header_doubles(&header, "CFIX_RTK_INITIAL_BASELINE_M_BITS");
    let positions_flat = header_doubles(&header, "CFIX_RTK_SAT_POS_M_BITS");
    let positions: Vec<[f64; 3]> = positions_flat
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    let base_code = header_doubles(&header, "CFIX_RTK_BASE_CODE_BITS");
    let rover_code = header_doubles(&header, "CFIX_RTK_ROVER_CODE_BITS");
    let float_phase = header_doubles(&header, "CFIX_RTK_FLOAT_ROVER_PHASE_BITS");
    let fixed_phase = header_doubles(&header, "CFIX_RTK_FIXED_ROVER_PHASE_BITS");
    let wavelength_m = header_doubles(&header, "CFIX_RTK_L1_WAVELENGTH_M_BITS")[0];
    let base = [base_ecef[0], base_ecef[1], base_ecef[2]];
    let initial = [initial[0], initial[1], initial[2]];

    // smoke.c configure_rtk_model and configure_rtk_float_options, the fixed
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
    let antenna = ReceiverAntennaCorrections {
        base: zero_calibration(),
        rover: zero_calibration(),
    };
    let ambiguity_ids: Vec<String> = SAT_IDS[1..].iter().map(|id| id.to_string()).collect();

    let float = solve_float_baseline(
        &[epoch(&base_code, &rover_code, &float_phase, &positions)],
        base,
        &ambiguity_ids,
        initial,
        &model,
        float_opts,
        Some(&antenna),
    )
    .expect("float RTK solve");

    // smoke.c's singular case: every non-reference row takes the first
    // non-reference satellite's positions.
    let mut singular_positions = positions.clone();
    for i in 2..5 {
        singular_positions[i] = positions[1];
    }
    let singular = solve_float_baseline(
        &[epoch(
            &base_code,
            &rover_code,
            &float_phase,
            &singular_positions,
        )],
        base,
        &ambiguity_ids,
        initial,
        &model,
        float_opts,
        Some(&antenna),
    );

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
        &[epoch(&base_code, &rover_code, &fixed_phase, &positions)],
        base,
        AmbiguitySet {
            ids: &ambiguity_ids,
            satellites: &satellites,
            scale: AmbiguityScale {
                wavelengths_m: &wavelengths,
                offsets_m: &offsets,
            },
            float_only_systems: &float_only_systems,
        },
        initial,
        &model,
        ValidatedFixedSolveOpts {
            float: float_opts,
            fixed: fixed_opts,
            residual: ResidualValidationOpts {
                threshold_sigma: None,
                max_exclusions: 0,
            },
        },
        Some(&antenna),
    )
    .expect("fixed RTK solve");
    let fixed = &validated.fixed_solution;

    header_start("smoke_a_rtk", GUARD);
    comment("sidereon_core::rtk_filter::solve_float_baseline of rtk_fixture.h's float inputs.");
    def_bool("SMOKE_A_RTK_FLOAT_CONVERGED", float.converged);
    def_variant("SMOKE_A_RTK_FLOAT_STATUS", &float.status);
    def_usize("SMOKE_A_RTK_FLOAT_RESIDUAL_COUNT", float.residuals.len());
    def_usize(
        "SMOKE_A_RTK_FLOAT_USED_SAT_COUNT",
        used_satellites(float.residuals.iter().map(|r| r.satellite_id.as_str())),
    );
    geometry_pins("SMOKE_A_RTK_FLOAT_GEOMETRY", &float.geometry_quality);

    comment("solve_fixed_baseline_validated of rtk_fixture.h's fixed inputs.");
    def_bool("SMOKE_A_RTK_FIXED_CONVERGED", fixed.converged);
    def_variant("SMOKE_A_RTK_FIXED_STATUS", &fixed.status);
    def_usize("SMOKE_A_RTK_FIXED_RESIDUAL_COUNT", fixed.residuals.len());
    def_usize(
        "SMOKE_A_RTK_FIXED_USED_SAT_COUNT",
        used_satellites(fixed.residuals.iter().map(|r| r.satellite_id.as_str())),
    );
    def_variant(
        "SMOKE_A_RTK_FIXED_INTEGER_STATUS",
        &fixed.search.integer_status,
    );
    def_bool(
        "SMOKE_A_RTK_FIXED_HAS_INTEGER_RATIO",
        fixed.search.integer_ratio.is_some(),
    );
    def_bool(
        "SMOKE_A_RTK_FIXED_HAS_INTEGER_BEST_SCORE",
        fixed.search.integer_best_score.is_some(),
    );
    def_bool(
        "SMOKE_A_RTK_FIXED_HAS_INTEGER_SECOND_BEST_SCORE",
        fixed.search.integer_second_best_score.is_some(),
    );
    geometry_pins(
        "SMOKE_A_RTK_FIXED_GEOMETRY",
        &validated.float_solution.geometry_quality,
    );

    comment("The singular-geometry float solve: the engine's refusal and its text.");
    match &singular {
        Ok(_) => {
            def_bool("SMOKE_A_RTK_SINGULAR_REFUSED", false);
            def_str("SMOKE_A_RTK_SINGULAR_ERROR_TEXT", "");
        }
        Err(err) => {
            def_bool("SMOKE_A_RTK_SINGULAR_REFUSED", true);
            def_str("SMOKE_A_RTK_SINGULAR_ERROR_TEXT", &err.to_string());
        }
    }
    header_end(GUARD);
}
