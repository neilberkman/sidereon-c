//! smoke.c exercise_rinex_spp_surface, exercise_broadcast_fallback_surface
//! and the SPP checks in main: sidereon-core's solutions for the same inputs
//! and products, with the provenance and metadata the checks compare.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::positioning::{
    solve_broadcast, solve_spp_from_rinex_obs, solve_with_fallback, solve_with_policy,
    spp_inputs_from_rinex_obs, BroadcastReason, Corrections, FixSource, KlobucharCoeffs,
    Observation, PseudorangeCode, QzssClock, ReceiverSolution, RinexSppOptions, RobustConfig,
    SolveInputs, SolvePolicy, SourcedSolution, SurfaceMet, TroposphereModel,
};
use sidereon_core::quality::SolutionValidationOptions;
use sidereon_core::rinex::observations::ObservationFile;
use sidereon_core::staleness::{StalenessMetadata, StalenessPolicy};
use sidereon_core::GnssSatelliteId;
use std::collections::BTreeMap;
use valgen::{bits, header_end, header_start, read, read_bytes, tests_path};

const BIN: &str = "smoke_b_positioning";
const GUARD: &str = "SIDEREON_SMOKE_B_POSITIONING_PINS_H";
const P: &str = "SMOKE_B_POSITIONING";

fn sat(id: &str) -> GnssSatelliteId {
    id.parse().expect("satellite id")
}

fn sp3(name: &str) -> Sp3 {
    Sp3::parse(&read_bytes(&tests_path(&format!("fixtures/sp3/{name}")))).expect("SP3 fixture")
}

fn f(header: &str, name: &str) -> Vec<f64> {
    header_f64s(header, name)
}

fn four(v: &[f64]) -> [f64; 4] {
    [v[0], v[1], v[2], v[3]]
}

/// src/lib.rs build_spp_solve_inputs for a SidereonSppInputs.
fn inputs_from_header(
    h: &str,
    prefix: &str,
    klobuchar: bool,
    ionosphere: bool,
    troposphere: bool,
) -> SolveInputs {
    let ids = header_strings(h, &format!("{prefix}_SAT_IDS"));
    let ranges = f(h, &format!("{prefix}_PSEUDORANGE_BITS"));
    let (alpha, beta) = if klobuchar {
        (
            four(&f(h, &format!("{prefix}_KLOB_ALPHA_BITS"))),
            four(&f(h, &format!("{prefix}_KLOB_BETA_BITS"))),
        )
    } else {
        ([0.0; 4], [0.0; 4])
    };
    SolveInputs {
        observations: ids
            .iter()
            .zip(&ranges)
            .map(|(id, range)| Observation {
                satellite_id: sat(id),
                pseudorange_m: *range,
            })
            .collect(),
        t_rx_j2000_s: f(h, &format!("{prefix}_T_RX_J2000_S_BITS"))[0],
        t_rx_second_of_day_s: f(h, &format!("{prefix}_T_RX_SOD_S_BITS"))[0],
        day_of_year: f(h, &format!("{prefix}_DOY_BITS"))[0],
        initial_guess: four(&f(h, &format!("{prefix}_INITIAL_GUESS_BITS"))),
        corrections: Corrections {
            ionosphere,
            troposphere,
        },
        klobuchar: KlobucharCoeffs { alpha, beta },
        beidou_klobuchar: None,
        galileo_nequick: None,
        sbas_iono: None,
        glonass_channels: BTreeMap::new(),
        met: SurfaceMet {
            pressure_hpa: f(h, &format!("{prefix}_PRESSURE_HPA_BITS"))[0],
            temperature_k: f(h, &format!("{prefix}_TEMPERATURE_K_BITS"))[0],
            relative_humidity: f(h, &format!("{prefix}_RELATIVE_HUMIDITY_BITS"))[0],
        },
        robust: None,
        pseudorange_code: PseudorangeCode::SingleFrequency,
        troposphere_model: TroposphereModel::Rtklib,
        qzss_clock: QzssClock::Gps,
    }
}

fn pos_clock(name: &str, s: &ReceiverSolution) {
    define_bits_array(
        &format!("{P}_{name}_POS_BITS"),
        &[s.position.x_m, s.position.y_m, s.position.z_m],
    );
    define_bits(&format!("{P}_{name}_CLOCK_BITS"), s.rx_clock_s);
    define_typed(&format!("{P}_{name}_USED"), "size_t", s.used_sats.len());
}

fn meta_pin(m: Option<StalenessMetadata>) -> String {
    match m {
        Some(m) => format!(
            "true, {}, {}, {}",
            c_enum("SIDEREON_DEGRADATION_KIND_", &m.kind),
            bits(m.staleness_s),
            bits(m.source_epoch_j2000_s)
        ),
        None => "false, SIDEREON_DEGRADATION_KIND_EXACT, UINT64_C(0), UINT64_C(0)".to_string(),
    }
}

/// The provenance src/sourced.rs reports for a sourced solution.
fn sourced(name: &str, s: &SourcedSolution) {
    let (reason, selection, attempted) = match &s.source {
        FixSource::Precise(_) => (
            "SIDEREON_BROADCAST_REASON_KIND_PRECISE_UNAVAILABLE".to_string(),
            "SIDEREON_SELECTION_STATUS_OK".to_string(),
            None,
        ),
        FixSource::Broadcast(BroadcastReason::PreciseUnavailable(err)) => (
            "SIDEREON_BROADCAST_REASON_KIND_PRECISE_UNAVAILABLE".to_string(),
            c_enum("SIDEREON_SELECTION_STATUS_", err),
            None,
        ),
        FixSource::Broadcast(reason @ BroadcastReason::PreciseDegradedUnusable { .. }) => (
            "SIDEREON_BROADCAST_REASON_KIND_PRECISE_DEGRADED_UNUSABLE".to_string(),
            "SIDEREON_SELECTION_STATUS_OK".to_string(),
            reason.attempted_staleness(),
        ),
    };
    println!(
        "static const SmokeBSourcedPin {P}_{name} = {{ {}, {}, {}, {}, {}, {} }};",
        s.source.is_precise(),
        s.source.is_precise_exact(),
        meta_pin(s.source.staleness()),
        reason,
        selection,
        meta_pin(attempted)
    );
    pos_clock(name, &s.solution);
}

fn rinex_spp() {
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .expect("NAV fixture");
    let obs = ObservationFile::parse(&read(&tests_path(
        "fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx",
    )))
    .expect("OBS fixture");
    // src/spp.rs rinex_spp_options_from_c with sidereon_rinex_spp_options_init.
    let mut options = RinexSppOptions::default_for(&obs).expect("RINEX SPP options");
    options.corrections = Corrections {
        ionosphere: true,
        troposphere: true,
    };
    options.met = SurfaceMet::default();
    let assembled = spp_inputs_from_rinex_obs(&obs, &nav, &options).expect("assembled inputs");
    define_typed(
        &format!("{P}_RINEX_RAW_EPOCHS"),
        "size_t",
        obs.epochs().len(),
    );
    define_typed(&format!("{P}_RINEX_ASSEMBLED"), "size_t", assembled.len());
    define_typed(
        &format!("{P}_RINEX_FIRST_OBS_COUNT"),
        "size_t",
        assembled[0].inputs.observations.len(),
    );
    define_bits(
        &format!("{P}_RINEX_FIRST_T_RX_BITS"),
        assembled[0].inputs.t_rx_j2000_s,
    );
    let solutions = solve_spp_from_rinex_obs(&nav, &obs, &options, true, SolvePolicy::default())
        .expect("RINEX SPP solutions");
    define_typed(&format!("{P}_RINEX_SOLUTIONS"), "size_t", solutions.len());
    let ok: Vec<String> = solutions
        .iter()
        .map(|s| s.solution.is_ok().to_string())
        .collect();
    println!(
        "static const bool {P}_RINEX_SOLUTION_OK[] = {{ {} }};",
        ok.join(", ")
    );
    let first = solutions
        .iter()
        .position(|s| s.solution.is_ok())
        .expect("a solved epoch");
    define_typed(&format!("{P}_RINEX_FIRST_SOLVED"), "size_t", first);
    define_typed(
        &format!("{P}_RINEX_FIRST_SOLVED_EPOCH_INDEX"),
        "size_t",
        solutions[first].epoch_index,
    );
    pos_clock(
        "RINEX_FIRST_SOLVED",
        solutions[first].solution.as_ref().expect("solved"),
    );
    println!();
}

fn fallback() {
    let h = "broadcast_fixture.h";
    let inputs = inputs_from_header(h, "BC", false, false, true);
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(&format!(
        "fixtures/nav/{}",
        header_define_string(h, "BC_NAV_FILE")
    ))))
    .expect("NAV fixture");
    let precise = sp3(&header_define_string(h, "BC_PRECISE_SP3_FILE"));
    let prior = sp3(&header_define_string(h, "BC_PRIOR_SP3_FILE"));
    let wrong = sp3(&header_define_string(h, "BC_WRONG_EPOCH_SP3_FILE"));
    let policy = StalenessPolicy::days(3.0);

    println!("typedef struct {{");
    println!("    bool precise;");
    println!("    bool precise_exact;");
    println!("    bool has_staleness;");
    println!("    SidereonDegradationKind staleness_kind;");
    println!("    uint64_t staleness_s_bits;");
    println!("    uint64_t source_epoch_bits;");
    println!("    SidereonBroadcastReasonKind reason;");
    println!("    SidereonSelectionStatus unavailable;");
    println!("    bool has_attempted;");
    println!("    SidereonDegradationKind attempted_kind;");
    println!("    uint64_t attempted_staleness_s_bits;");
    println!("    uint64_t attempted_source_epoch_bits;");
    println!("}} SmokeBSourcedPin;");
    define_bits(&format!("{P}_FALLBACK_CAP_S_BITS"), policy.max_staleness_s);
    let broadcast = solve_broadcast(&nav, &inputs, true).expect("broadcast solve");
    pos_clock("BROADCAST", &broadcast);
    for (name, set) in [
        ("FB_EXACT", vec![precise]),
        ("FB_EMPTY", vec![]),
        ("FB_WRONG", vec![wrong]),
        ("FB_PRIOR", vec![prior]),
    ] {
        let s = solve_with_fallback(&set, &nav, &inputs, policy, true).expect("fallback solve");
        sourced(name, &s);
    }
    println!();
}

fn main_spp() {
    let h = "spp_fixture.h";
    let inputs = inputs_from_header(h, "SPP", true, false, false);
    let sp3 = sp3(&header_define_string(h, "SPP_SP3_FILE"));
    let s = solve_with_policy(&sp3, &inputs, true, SolvePolicy::default()).expect("SPP solve");
    // The same inputs with no observations. An engine refusal reaches C as
    // SIDEREON_STATUS_SOLVE (src/solve.rs guard).
    let mut empty = inputs.clone();
    empty.observations.clear();
    define_bool(
        &format!("{P}_SPP_EMPTY_OK"),
        solve_with_policy(&sp3, &empty, true, SolvePolicy::default()).is_ok(),
    );
    let g = s.geodetic.expect("geodetic");
    define_bits_array(
        &format!("{P}_SPP_GEODETIC_BITS"),
        &[g.lat_rad, g.lon_rad, g.height_m],
    );
    define_typed(
        &format!("{P}_SPP_SYSTEM_CLOCK_COUNT"),
        "size_t",
        s.system_clocks_s.len(),
    );
    let (system, clock) = s.system_clocks_s[0];
    println!(
        "#define {P}_SPP_SYSTEM_CLOCK0_SYSTEM {}",
        gnss_system_c(system)
    );
    define_bits(&format!("{P}_SPP_SYSTEM_CLOCK0_BITS"), clock);
    let m = &s.metadata;
    let q = &s.geometry_quality;
    println!("#define {P}_SPP_ITERATIONS ((size_t){})", m.iterations);
    define_bool(&format!("{P}_SPP_CONVERGED"), m.converged);
    println!(
        "#define {P}_SPP_STATUS {}",
        c_enum("SIDEREON_SPP_SOLVE_STATUS_", &m.status)
    );
    define_bool(&format!("{P}_SPP_IONO_APPLIED"), m.ionosphere_applied);
    define_bool(&format!("{P}_SPP_TROPO_APPLIED"), m.troposphere_applied);
    println!(
        "#define {P}_SPP_OUTER_ITERATIONS ((size_t){})",
        m.outer_iterations
    );
    define_bool(
        &format!("{P}_SPP_HAS_ROBUST_SCALE"),
        m.final_robust_scale_m.is_some(),
    );
    println!("#define {P}_SPP_USED_COUNT ((size_t){})", m.used_count);
    println!("#define {P}_SPP_SYSTEM_COUNT ((size_t){})", m.systems.len());
    println!("#define {P}_SPP_REDUNDANCY INT64_C({})", m.redundancy);
    define_bool(&format!("{P}_SPP_RAIM_CHECKABLE"), m.raim_checkable);
    println!(
        "#define {P}_SPP_GQ_TIER {}",
        c_enum("SIDEREON_OBSERVABILITY_TIER_", &q.tier)
    );
    println!("#define {P}_SPP_GQ_REDUNDANCY {}", q.redundancy);
    println!("#define {P}_SPP_GQ_RANK ((size_t){})", q.rank);
    define_bits(&format!("{P}_SPP_GQ_CONDITION_BITS"), q.condition_number);
    define_bits(&format!("{P}_SPP_GQ_GDOP_BITS"), q.gdop);
    define_bool(&format!("{P}_SPP_GQ_RAIM_CHECKABLE"), q.raim_checkable);
    define_bool(
        &format!("{P}_SPP_GQ_COVARIANCE_VALIDATED"),
        q.covariance_validated,
    );
    define_bits_array(&format!("{P}_SPP_RESIDUAL_BITS"), &s.residuals_m);
    define_bool(&format!("{P}_SPP_HAS_DOP"), s.dop.is_some());
    if let Some(d) = s.dop {
        define_bits_array(
            &format!("{P}_SPP_DOP_BITS"),
            &[d.gdop, d.pdop, d.hdop, d.vdop, d.tdop],
        );
    }

    // The same inputs with a QZSS observation J01 appended, carrying the
    // first observation's pseudorange.
    let mut with_j01 = inputs.clone();
    with_j01.observations.push(Observation {
        satellite_id: sat("J01"),
        pseudorange_m: inputs.observations[0].pseudorange_m,
    });
    let r = solve_with_policy(&sp3, &with_j01, true, SolvePolicy::default()).expect("J01 solve");
    define_typed(
        &format!("{P}_SPP_J01_REJECTED_COUNT"),
        "size_t",
        r.rejected_sats.len(),
    );
    let last = r.rejected_sats.last().expect("rejected satellites");
    define_text(
        &format!("{P}_SPP_J01_LAST_REJECTED_ID"),
        &last.satellite_id.to_string(),
    );
    println!(
        "#define {P}_SPP_J01_LAST_REJECTED_REASON {}",
        c_enum("SIDEREON_SPP_REJECTION_REASON_", &last.reason)
    );

    // sidereon_solve_spp_v2 with the BeiDou Klobuchar set to the GPS one,
    // robust reweighting at max_outer 2, a PDOP cap of 9999 and one coarse
    // search seed (src/lib.rs robust_config_from_c, validation_options_from_c,
    // src/solve.rs solve_policy_from_c).
    let mut v2 = inputs.clone();
    v2.beidou_klobuchar = Some(inputs.klobuchar);
    let mut robust = RobustConfig::default();
    robust.max_outer = 2;
    v2.robust = Some(robust);
    let mut validation = SolutionValidationOptions::default();
    validation.max_pdop = Some(9999.0);
    let policy = SolvePolicy {
        validation,
        coarse_search_seeds: Some(1),
    };
    let v2_solution = solve_with_policy(&sp3, &v2, true, policy).expect("v2 solve");
    println!(
        "#define {P}_SPP_V2_OUTER_ITERATIONS ((size_t){})",
        v2_solution.metadata.outer_iterations
    );
    define_bool(
        &format!("{P}_SPP_V2_HAS_ROBUST_SCALE"),
        v2_solution.metadata.final_robust_scale_m.is_some(),
    );
    define_bits(
        &format!("{P}_SPP_V2_ROBUST_SCALE_BITS"),
        v2_solution.metadata.final_robust_scale_m.unwrap_or(0.0),
    );

    // The same v2 inputs with robust reweighting and the coarse search off and
    // a PDOP cap of 0.1. An engine refusal reaches C as SIDEREON_STATUS_SOLVE.
    let mut strict = v2.clone();
    strict.robust = None;
    let mut strict_validation = SolutionValidationOptions::default();
    strict_validation.max_pdop = Some(0.1);
    let strict_policy = SolvePolicy {
        validation: strict_validation,
        coarse_search_seeds: None,
    };
    let strict_result = solve_with_policy(&sp3, &strict, true, strict_policy);
    define_bool(&format!("{P}_SPP_V2_STRICT_OK"), strict_result.is_ok());
}

fn main() {
    header_start(BIN, GUARD);
    rinex_spp();
    fallback();
    main_spp();
    header_end(GUARD);
}
