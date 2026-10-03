//! robustness_smoke.c: sidereon-core's FDE over the ESBC broadcast epoch
//! (clean, and with one used satellite biased by 200 m), the robust and
//! coarse-search GRG SPP solves, and the fallback source choices, for the
//! inputs that test builds.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::positioning::{
    solve_with_fallback, solve_with_policy, Corrections, KlobucharCoeffs, Observation,
    PseudorangeCode, QzssClock, ReceiverSolution, RobustConfig, SolveInputs, SolvePolicy,
    SurfaceMet, TroposphereModel,
};
use sidereon_core::quality::{
    fde_spp, FdeError, FdeOptions, FdeSppOptions, FdeUnresolvedReason, RaimOptions, RaimWeights,
    SolutionValidationOptions,
};
use sidereon_core::staleness::StalenessPolicy;
use sidereon_core::GnssSatelliteId;
use std::collections::BTreeMap;
use valgen::{core_fixtures, read, read_bytes, tests_path};

fn inputs(
    sats: &[String],
    pseudoranges: &[f64],
    t_rx: f64,
    sod: f64,
    doy: f64,
    guess: [f64; 4],
    troposphere: bool,
    met: [f64; 3],
) -> SolveInputs {
    SolveInputs {
        observations: sats
            .iter()
            .zip(pseudoranges)
            .map(|(sat, pseudorange)| Observation {
                satellite_id: sat.parse::<GnssSatelliteId>().expect("satellite"),
                pseudorange_m: *pseudorange,
            })
            .collect(),
        t_rx_j2000_s: t_rx,
        t_rx_second_of_day_s: sod,
        day_of_year: doy,
        initial_guess: guess,
        corrections: Corrections {
            ionosphere: false,
            troposphere,
        },
        klobuchar: KlobucharCoeffs {
            alpha: [0.0; 4],
            beta: [0.0; 4],
        },
        beidou_klobuchar: None,
        galileo_nequick: None,
        sbas_iono: None,
        glonass_channels: BTreeMap::new(),
        met: SurfaceMet {
            pressure_hpa: met[0],
            temperature_k: met[1],
            relative_humidity: met[2],
        },
        robust: None,
        pseudorange_code: PseudorangeCode::SingleFrequency,
        troposphere_model: TroposphereModel::Rtklib,
        qzss_clock: QzssClock::Gps,
    }
}

fn used(solution: &ReceiverSolution) -> Vec<String> {
    solution.used_sats.iter().map(|s| s.to_string()).collect()
}

fn def_tokens(name: &str, tokens: &[String]) {
    def_size(&format!("{name}_COUNT"), tokens.len());
    println!(
        "static const char *const {name}[{}] = {{ {} }};",
        tokens.len().max(1),
        if tokens.is_empty() {
            "\"\"".to_string()
        } else {
            tokens
                .iter()
                .map(|t| valgen::c_string(t))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
}

fn main() {
    let guard = "SIDEREON_W6_ROBUSTNESS_PINS_H";
    valgen::header_start("w6_robustness", guard);

    // The broadcast epoch of broadcast_fixture.h (tests/fbgen).
    let header = tests_path("broadcast_fixture.h");
    let sats = header_strings(&header, "BC_SAT_IDS");
    let pseudoranges = header_bits(&header, "BC_PSEUDORANGE_BITS");
    let guess = header_bits(&header, "BC_INITIAL_GUESS_BITS");
    let bc = inputs(
        &sats,
        &pseudoranges,
        header_bits(&header, "BC_T_RX_J2000_S_BITS")[0],
        header_bits(&header, "BC_T_RX_SOD_S_BITS")[0],
        header_bits(&header, "BC_DOY_BITS")[0],
        [guess[0], guess[1], guess[2], guess[3]],
        true,
        [
            header_bits(&header, "BC_PRESSURE_HPA_BITS")[0],
            header_bits(&header, "BC_TEMPERATURE_K_BITS")[0],
            header_bits(&header, "BC_RELATIVE_HUMIDITY_BITS")[0],
        ],
    );
    let broadcast = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .expect("broadcast");

    // sidereon_fde_options_init with p_fa 1e-3, unit weights, max_exclusions
    // the observation count; no validation options.
    let mut raim = RaimOptions::default();
    raim.p_fa = 1.0e-3;
    raim.weights = RaimWeights::Unit;
    raim.n_systems = None;
    let options = FdeSppOptions::new(
        FdeOptions::new(raim, sats.len()),
        SolutionValidationOptions::default(),
    );
    let clean = fde_spp(&broadcast, &bc, true, &options).expect("clean FDE");
    comment("Clean FDE: iterations, excluded satellites, used satellites.");
    def_size("W6_ROB_CLEAN_ITERATIONS", clean.iterations);
    def_tokens("W6_ROB_CLEAN_EXCLUDED", &clean.excluded);
    let clean_used = used(&clean.solution);
    def_tokens("W6_ROB_CLEAN_USED", &clean_used);

    // The test's choice: the first fixture satellite the clean solve uses.
    let bad_index = sats
        .iter()
        .position(|sat| clean_used.contains(sat))
        .expect("a used satellite");
    let mut corrupt = bc.clone();
    corrupt.observations[bad_index].pseudorange_m += 200.0;
    let fixed = fde_spp(&broadcast, &corrupt, true, &options).expect("corrupted FDE");
    comment("FDE with the first used satellite biased by 200 m.");
    def_str("W6_ROB_BAD_SAT", &sats[bad_index]);
    def_size("W6_ROB_CORRUPT_ITERATIONS", fixed.iterations);
    def_tokens("W6_ROB_CORRUPT_EXCLUDED", &fixed.excluded);
    def_tokens("W6_ROB_CORRUPT_USED", &used(&fixed.solution));
    comment("The accepted corrupted-run solution's detection test.");
    def_bool(
        "W6_ROB_CORRUPT_RAIM_FAULT_DETECTED",
        fixed.raim.fault_detected,
    );
    def_bool("W6_ROB_CORRUPT_RAIM_TESTABLE", fixed.raim.testable);
    def_bits(
        "W6_ROB_CORRUPT_RAIM_TEST_STATISTIC_BITS",
        fixed.raim.test_statistic,
    );

    // No exclusion budget: the fault is detected and left unresolved, with the
    // last solution, no exclusions and its detection test.
    let mut no_budget = options.clone();
    no_budget.fde.max_exclusions = 0;
    let Err(FdeError::FaultUnresolved(unresolved)) =
        fde_spp(&broadcast, &corrupt, true, &no_budget)
    else {
        panic!("sidereon-core leaves the fault unresolved with no exclusion budget");
    };
    comment("FDE with no exclusion budget on the corrupted run: unresolved.");
    def(
        "W6_ROB_UNRESOLVED_REASON",
        match unresolved.reason {
            FdeUnresolvedReason::ExclusionBudgetExhausted => {
                "SIDEREON_FDE_UNRESOLVED_REASON_EXCLUSION_BUDGET_EXHAUSTED"
            }
            FdeUnresolvedReason::NoAdmissibleExclusion => {
                "SIDEREON_FDE_UNRESOLVED_REASON_NO_ADMISSIBLE_EXCLUSION"
            }
            _ => "SIDEREON_FDE_UNRESOLVED_REASON_UNKNOWN",
        },
    );
    def_tokens("W6_ROB_UNRESOLVED_EXCLUDED", &unresolved.excluded);
    def_bool(
        "W6_ROB_UNRESOLVED_FAULT_DETECTED",
        unresolved.raim.fault_detected,
    );
    def_bits(
        "W6_ROB_UNRESOLVED_TEST_STATISTIC_BITS",
        unresolved.raim.test_statistic,
    );
    def_tokens("W6_ROB_UNRESOLVED_USED", &used(&unresolved.solution));

    // The GRG L0_minimal inputs from sidereon-core's SPP fixture, as
    // fill_grg_inputs builds them: no ionosphere, no troposphere, zero
    // Klobuchar.
    let fixture: serde_json::Value = serde_json::from_str(&read(&format!(
        "{}/spp_trace_L0_minimal.json",
        core_fixtures()
    )))
    .expect("SPP fixture JSON");
    let fx = &fixture["fixture"];
    let inp = &fx["inputs"];
    let observations = inp["observations"].as_array().expect("observations");
    let grg_sats: Vec<String> = observations
        .iter()
        .map(|o| o["sat_id"].as_str().expect("sat_id").to_string())
        .collect();
    let grg_prs: Vec<f64> = observations
        .iter()
        .map(|o| fixture_f64(&o["p_meas_m"], "p_meas_m"))
        .collect();
    let x0: Vec<f64> = fx["frozen"]["initial_guess_x0"]
        .as_array()
        .expect("x0")
        .iter()
        .map(|v| fixture_f64(v, "x0 value"))
        .collect();
    let met = &inp["met"];
    let grg = |guess: [f64; 4]| {
        inputs(
            &grg_sats,
            &grg_prs,
            fixture_f64(&inp["t_rx_j2000_s"], "t_rx"),
            fixture_f64(&inp["t_rx_sod_s"], "sod"),
            fixture_f64(&inp["doy"], "doy"),
            guess,
            false,
            [
                fixture_f64(&met["pressure_hpa"], "pressure"),
                fixture_f64(&met["temperature_k"], "temperature"),
                fixture_f64(&met["relative_humidity"], "humidity"),
            ],
        )
    };
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3");
    let policy = |coarse: Option<usize>| SolvePolicy {
        validation: SolutionValidationOptions::default(),
        coarse_search_seeds: coarse,
    };

    let mut robust_inputs = grg([x0[0], x0[1], x0[2], x0[3]]);
    let mut robust = RobustConfig::default();
    robust.max_outer = 2;
    robust_inputs.robust = Some(robust);
    let robust_solution =
        solve_with_policy(&sp3, &robust_inputs, true, policy(None)).expect("robust solve");
    comment("Robust (max_outer 2) GRG solve metadata.");
    def_size(
        "W6_ROB_ROBUST_OUTER_ITERATIONS",
        robust_solution.metadata.outer_iterations,
    );
    def_bool(
        "W6_ROB_ROBUST_HAS_SCALE",
        robust_solution.metadata.final_robust_scale_m.is_some(),
    );
    def_bits(
        "W6_ROB_ROBUST_SCALE_BITS",
        robust_solution.metadata.final_robust_scale_m.unwrap_or(0.0),
    );

    let coarse_solution =
        solve_with_policy(&sp3, &grg([0.0; 4]), true, policy(Some(24))).expect("coarse solve");
    comment("Coarse-search (24 seeds) GRG solve from the geocenter.");
    def_bits_array(
        "W6_ROB_COARSE_POSITION_BITS",
        &[
            coarse_solution.position.x_m,
            coarse_solution.position.y_m,
            coarse_solution.position.z_m,
        ],
    );

    comment("Fallback source kinds: no precise product, then the 2026 wrong-epoch SP3.");
    let wrong = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3",
    )))
    .expect("wrong-epoch SP3");
    for (label, set) in [("EMPTY", Vec::new()), ("WRONG_EPOCH", vec![wrong])] {
        let sourced = solve_with_fallback(&set, &broadcast, &bc, StalenessPolicy::days(3.0), true)
            .expect("fallback solve");
        def(
            &format!("W6_ROB_FALLBACK_{label}_KIND"),
            if sourced.source.is_precise() {
                "SIDEREON_FIX_SOURCE_KIND_PRECISE"
            } else {
                "SIDEREON_FIX_SOURCE_KIND_BROADCAST"
            },
        );
    }
    valgen::header_end(guard);
}

/// A double the core SPP fixture stores as its hex bit pattern ("0x...").
fn fixture_f64(value: &serde_json::Value, what: &str) -> f64 {
    let text = value
        .as_str()
        .unwrap_or_else(|| panic!("{what}: hex bit pattern"));
    f64::from_bits(
        u64::from_str_radix(text.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("{what}: hex bit pattern")),
    )
}
