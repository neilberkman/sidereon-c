// Print, as a C header, the engine values the C smoke tests pin. See
// Cargo.toml for how to run it.

use sidereon_core::ephemeris::Sp3;
use sidereon_core::positioning::{
    solve_static_reference_station_rinex, StaticReferenceCarrierRinexOptions,
    StaticReferenceStationRinexOptions,
};
use sidereon_core::rinex::observations::RinexObs;
use sidereon_core::rtk::BaselineReferenceSelection;
use sidereon_core::rtk_filter::defaults::{
    AMBIGUITY_TOL_M, MAX_ITERATIONS, POSITION_TOL_M, RATIO_THRESHOLD,
};
use sidereon_core::rtk_filter::{
    build_rinex_rtk_arc, CycleSlipPolicy, DynamicsModel, FixedSolveOpts, FloatSolveOpts, MeasModel,
    ResidualValidationOpts, RtkArcConfig, RtkArcPreprocessing, RtkRinexArcOptions,
    RtkStaticArcConfig, SearchOpts, StochasticModel, UpdateOpts, ValidatedFixedSolveOpts,
};
use sidereon_core::scenario::{simulate_scenario, Scenario};
use std::collections::BTreeMap;

/// WTZR and WTZZ marker positions, ECEF metres, as rtk_rinex_smoke.c states them.
const WTZR_MARKER_M: [f64; 3] = [4075580.3111, 931854.0543, 4801568.2808];

/// The sidereon-core revision checked against this crate's locked graph by
/// tests/run_generators.sh.
const CORE_REVISION: &str = include_str!("../../CORE_REVISION");
const TESTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const SCENARIO_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../domain018_smoke.c");
const NAV_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../rinex_nav_clock_smoke.c");

/// Fixture tree supplied by the source-validated runner, or locked metadata's
/// package tree when this binary is run directly.
fn core_fixtures() -> String {
    if let Ok(path) = std::env::var("SIDEREON_CORE_FIXTURES") {
        return path;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
        ])
        .output()
        .expect("run cargo metadata");
    assert!(output.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata JSON");
    let manifest = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == "sidereon-core")
        .and_then(|package| package["manifest_path"].as_str())
        .expect("sidereon-core in cargo metadata")
        .to_string();
    let dir = std::path::Path::new(&manifest)
        .parent()
        .expect("sidereon-core crate directory")
        .join("tests")
        .join("fixtures");
    dir.to_str().expect("UTF-8 fixture path").to_string()
}

fn bits(value: f64) -> String {
    format!("UINT64_C({:#018x})", value.to_bits())
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {path}: {err}"))
}

/// The text of the C string literals that follow `marker` in `source`, up to
/// the next `;`: the adjacent literals joined, with their escapes read.
fn c_string_literals(source: &str, marker: &str) -> String {
    let source = read(source);
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("{marker} in C source"));
    let end = start + source[start..].find(';').expect("end of literal");
    let mut text = String::new();
    let mut chars = source[start + marker.len()..end].chars();
    let mut in_literal = false;
    while let Some(c) = chars.next() {
        match (in_literal, c) {
            (false, '"') => in_literal = true,
            (true, '"') => in_literal = false,
            (true, '\\') => match chars.next().expect("escaped character") {
                'n' => text.push('\n'),
                other => text.push(other),
            },
            (true, c) => text.push(c),
            (false, _) => {}
        }
    }
    text
}

fn scenario_json() -> String {
    c_string_literals(SCENARIO_SOURCE, "static const char *scenario_json =")
}

/// The antenna reference point: the marker moved along its geocentric radius
/// by the header's antenna height, as rtk_rinex_smoke.c's arp_position forms
/// it (the east and north offsets are zero for these stations).
fn arp_position(marker: [f64; 3], obs: &RinexObs) -> [f64; 3] {
    let delta = obs.header().antenna_delta_hen_m.expect("antenna delta");
    assert!(delta[1].abs() < 1.0e-12 && delta[2].abs() < 1.0e-12);
    let norm = (marker[0] * marker[0] + marker[1] * marker[1] + marker[2] * marker[2]).sqrt();
    let height = delta[0];
    [
        marker[0] + height * marker[0] / norm,
        marker[1] + height * marker[1] / norm,
        marker[2] + height * marker[2] / norm,
    ]
}

/// The carrier-only static reference-station solve rtk_rinex_smoke.c makes:
/// sidereon_static_reference_station_rinex_config_init, then the WTZR
/// single-frequency settings and the overrides test_static_reference_station
/// applies, converted as the binding converts them.
fn static_reference_options(reference_m: [f64; 3]) -> StaticReferenceStationRinexOptions {
    let mut arc_options = RtkRinexArcOptions::gps_l1_c();
    arc_options.max_epochs = Some(24);
    arc_options.min_common_satellites = 4;
    arc_options.include_prediction_time = false;
    let arc = RtkArcConfig::new(
        reference_m,
        BaselineReferenceSelection::Auto,
        MeasModel {
            code_sigma_m: 2.0,
            phase_sigma_m: 0.01,
            sagnac: true,
            stochastic: StochasticModel::Simple {
                elevation_weighting: true,
            },
        },
        30.0,
        30.0,
        [0.0; 3],
        BTreeMap::new(),
        BTreeMap::new(),
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
        },
        RtkArcPreprocessing {
            cycle_slip: Some(CycleSlipPolicy::SplitArc),
            hatch_window_cap: None,
            elevation_mask_deg: None,
        },
    );
    let opts = ValidatedFixedSolveOpts {
        float: FloatSolveOpts {
            position_tol_m: 1.0e-4,
            ambiguity_tol_m: 1.0e-4,
            max_iterations: 10,
        },
        fixed: FixedSolveOpts {
            position_tol_m: 1.0e-4,
            ambiguity_tol_m: 1.0e-4,
            max_iterations: 10,
            ratio_threshold: 3.0,
            partial_ambiguity_resolution: true,
            partial_min_ambiguities: 4,
        },
        residual: ResidualValidationOpts {
            threshold_sigma: None,
            max_exclusions: 0,
        },
    };
    StaticReferenceStationRinexOptions::new(
        None,
        Some(StaticReferenceCarrierRinexOptions::new(
            arc_options,
            RtkStaticArcConfig::new(arc, opts),
        )),
        true,
    )
}

fn main() {
    let core = core_fixtures();
    let sp3 = Sp3::parse(
        &std::fs::read(format!(
            "{core}/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3"
        ))
        .expect("read RTK SP3"),
    )
    .expect("parse RTK SP3");
    let base = RinexObs::parse(&read(&format!(
        "{core}/obs/WTZR00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"
    )))
    .expect("parse WTZR");
    let rover = RinexObs::parse(&read(&format!(
        "{core}/obs/WTZZ00DEU_R_20201770000_01D_30S_MO_120epoch.rnx"
    )))
    .expect("parse WTZZ");
    // sidereon_rtk_rinex_arc_options_init with max_epochs 120, as the smoke
    // test sets it: GPS C1C/L1C, four common satellites, prediction time on.
    let mut options = RtkRinexArcOptions::gps_l1_c();
    options.max_epochs = Some(120);
    options.min_common_satellites = 4;
    options.include_prediction_time = true;
    let arc = build_rinex_rtk_arc(&sp3, &base, &rover, &options).expect("build RTK arc");
    let epoch = arc.epochs.first().expect("first arc epoch");
    let base_g05 = epoch.base_satellite_positions_m["G05"];
    let rover_g05 = epoch.rover_satellite_positions_m["G05"];

    let reference_m = arp_position(WTZR_MARKER_M, &base);
    let station = solve_static_reference_station_rinex(
        &sp3,
        &base,
        &rover,
        reference_m,
        &static_reference_options(reference_m),
    )
    .expect("static reference-station solve");
    let station_carrier = station.carrier_solution.as_ref().expect("carrier solution");
    let station_ratio = station_carrier.integer_ratio.expect("integer ratio");
    let station_position = station.position.as_array();
    let station_covariance = station.covariance.position_ecef_m2;

    let scenario: Scenario = serde_json::from_str(&scenario_json()).expect("scenario JSON");
    let set = simulate_scenario(&scenario).expect("simulate scenario");

    let fit = tle_fit();
    let qc = observation_qc(&core);
    let (store_records, encoded_len) = nav_counts();
    let (constellation_records, constellation_csv, constellation_validation, constellation_valid) =
        constellation();

    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("/* Generated by tests/pingen from sidereon-core. Every value is an exact");
    println!(" * IEEE-754 float64 bit pattern of the engine's result. Do not edit by hand;");
    println!(" * rerun the generator. */");
    println!("#ifndef SIDEREON_ENGINE_PINS_FIXTURE_H");
    println!("#define SIDEREON_ENGINE_PINS_FIXTURE_H");
    println!();
    println!("#include <stdbool.h>");
    println!("#include <stddef.h>");
    println!("#include <stdint.h>");
    println!();
    println!("/* WTZR/WTZZ single-frequency RINEX RTK arc, epoch 0, G05: the base and");
    println!(" * rover transmit-time satellite positions (ECEF metres). */");
    println!(
        "static const uint64_t PIN_RTK_BASE_G05_POSITION_BITS[3] = {{ {}, {}, {} }};",
        bits(base_g05[0]),
        bits(base_g05[1]),
        bits(base_g05[2])
    );
    println!(
        "static const uint64_t PIN_RTK_ROVER_G05_POSITION_BITS[3] = {{ {}, {}, {} }};",
        bits(rover_g05[0]),
        bits(rover_g05[1]),
        bits(rover_g05[2])
    );
    println!();
    println!("/* WTZR/WTZZ carrier static reference-station solve (rtk_rinex_smoke.c):");
    println!(" * the WTZR antenna reference point it is given, the integer ratio, position");
    println!(" * and baseline (ECEF metres), position covariance (ECEF square metres,");
    println!(" * row-major). */");
    println!(
        "static const uint64_t PIN_STATIC_REFERENCE_ARP_BITS[3] = {{ {}, {}, {} }};",
        bits(reference_m[0]),
        bits(reference_m[1]),
        bits(reference_m[2])
    );
    println!(
        "static const uint64_t PIN_STATIC_REFERENCE_RATIO_BITS = {};",
        bits(station_ratio)
    );
    println!(
        "static const uint64_t PIN_STATIC_REFERENCE_POSITION_BITS[3] = {{ {}, {}, {} }};",
        bits(station_position[0]),
        bits(station_position[1]),
        bits(station_position[2])
    );
    println!(
        "static const uint64_t PIN_STATIC_REFERENCE_BASELINE_BITS[3] = {{ {}, {}, {} }};",
        bits(station.baseline_vector_m[0]),
        bits(station.baseline_vector_m[1]),
        bits(station.baseline_vector_m[2])
    );
    let covariance: Vec<String> = station_covariance
        .iter()
        .flat_map(|row| row.iter().map(|value| bits(*value)))
        .collect();
    println!(
        "static const uint64_t PIN_STATIC_REFERENCE_COVARIANCE_BITS[9] = {{ {} }};",
        covariance.join(", ")
    );
    println!();
    println!("/* domain018 synthetic scenario, first observation row. */");
    println!(
        "static const uint64_t PIN_SCENARIO_PSEUDORANGE_M_BITS = {};",
        bits(set.observations.pseudorange_m[0])
    );
    println!(
        "static const uint64_t PIN_SCENARIO_CARRIER_PHASE_CYCLES_BITS = {};",
        bits(set.observations.carrier_phase_cycles[0])
    );
    println!(
        "static const uint64_t PIN_SCENARIO_DOPPLER_HZ_BITS = {};",
        bits(set.observations.doppler_hz[0])
    );
    println!(
        "static const uint64_t PIN_SCENARIO_GEOMETRIC_RANGE_M_BITS = {};",
        bits(set.truth_terms.geometric_range_m[0])
    );
    println!(
        "static const uint64_t PIN_SCENARIO_THERMAL_NOISE_M_BITS = {};",
        bits(set.truth_terms.thermal_noise_m[0])
    );
    println!();
    println!("/* round2_parity_smoke.c TLE fit: sidereon_core::astro::sgp4::fit_tle of the");
    println!(" * three samples that test builds. */");
    println!(
        "static const uint64_t PIN_FIT_RMS_POSITION_KM_BITS = {};",
        bits(fit.stats.rms_position_km)
    );
    println!(
        "static const uint64_t PIN_FIT_MAX_POSITION_KM_BITS = {};",
        bits(fit.stats.max_position_km)
    );
    println!(
        "static const uint64_t PIN_FIT_RMS_VELOCITY_KM_S_BITS = {};",
        bits(fit.stats.rms_velocity_km_s.expect("velocity RMS"))
    );
    println!(
        "static const uint64_t PIN_FIT_TLE_RMS_POSITION_KM_BITS = {};",
        bits(fit.stats.tle_rms_position_km)
    );
    println!("#define PIN_FIT_STATUS {}", fit.stats.status);
    println!("#define PIN_FIT_NFEV {}", fit.stats.nfev);
    println!("#define PIN_FIT_NJEV {}", fit.stats.njev);
    println!(
        "#define PIN_FIT_SEED_REFINE_PASSES {}",
        fit.stats.seed_refine_passes
    );
    println!("#define PIN_FIT_LINE1 {:?}", fit.line1);
    println!("#define PIN_FIT_LINE2 {:?}", fit.line2);
    println!();
    println!("/* round2_parity_smoke.c observation QC of the 120-epoch ESBC file under the");
    println!(" * default options. */");
    println!(
        "#define PIN_QC_SATELLITE_SIGNAL_COUNT {}",
        qc.satellite_signals.len()
    );
    println!(
        "#define PIN_QC_SYSTEM_SIGNAL_COUNT {}",
        qc.system_signals.len()
    );
    println!(
        "#define PIN_QC_CYCLE_SLIP_OBSERVATIONS {}",
        qc.cycle_slips.observations
    );
    println!(
        "#define PIN_QC_CYCLE_SLIP_TOTAL {}",
        qc.cycle_slips.total_slips
    );
    println!(
        "#define PIN_QC_CYCLE_SLIP_SYSTEM_COUNT {}",
        qc.cycle_slips.by_system.len()
    );
    for row in &qc.cycle_slips.by_system {
        let name = system_macro(row.system);
        println!(
            "#define PIN_QC_SLIPS_{name}_OBSERVATIONS {}",
            row.observations
        );
        println!("#define PIN_QC_SLIPS_{name}_SLIPS {}", row.slips);
    }
    println!(
        "#define PIN_QC_MULTIPATH_SYSTEM_COUNT {}",
        qc.multipath.systems.len()
    );
    for row in &qc.multipath.systems {
        let name = system_macro(row.system);
        let mp1 = row.mp1.expect("MP1");
        let mp2 = row.mp2.expect("MP2");
        println!("#define PIN_QC_MP_{name}_MP1_N {}", mp1.n);
        println!("#define PIN_QC_MP_{name}_MP2_N {}", mp2.n);
        println!(
            "static const uint64_t PIN_QC_MP_{name}_MP1_RMS_M_BITS = {};",
            bits(mp1.rms_m)
        );
        println!(
            "static const uint64_t PIN_QC_MP_{name}_MP2_RMS_M_BITS = {};",
            bits(mp2.rms_m)
        );
    }
    println!(
        "#define PIN_QC_MULTIPATH_SATELLITE_COUNT {}",
        qc.multipath.satellites.len()
    );
    let g08 = qc
        .multipath
        .satellites
        .iter()
        .find(|row| row.satellite.to_string() == "G08")
        .expect("G08 multipath row");
    let g08_mp1 = g08.mp1.expect("G08 MP1");
    let g08_mp2 = g08.mp2.expect("G08 MP2");
    println!("#define PIN_QC_MP_G08_MP1_N {}", g08_mp1.n);
    println!("#define PIN_QC_MP_G08_MP2_N {}", g08_mp2.n);
    println!(
        "static const uint64_t PIN_QC_MP_G08_MP1_RMS_M_BITS = {};",
        bits(g08_mp1.rms_m)
    );
    println!(
        "static const uint64_t PIN_QC_MP_G08_MP2_RMS_M_BITS = {};",
        bits(g08_mp2.rms_m)
    );
    println!();
    println!("/* rinex_nav_clock_smoke.c: the broadcast store of the ESBC mixed NAV file");
    println!(" * followed by that test's GLONASS record, and the NAV text written for the");
    println!(" * file's first raw record. */");
    println!("#define PIN_NAV_STORE_RECORD_COUNT {store_records}");
    println!("#define PIN_NAV_ENCODED_FIRST_RECORD_LEN {encoded_len}");
    println!();
    println!("/* smoke.c constellation checks: the merged GPS catalog of the committed");
    println!(" * CelesTrak and NAVCEN fixtures, its CSV with lowercase booleans, and its");
    println!(" * validation against PIN_CONSTELLATION_VALIDATE_SP3_IDS. */");
    println!(
        "static const char *const PIN_CONSTELLATION_VALIDATE_SP3_IDS[] = {{ {} }};",
        CONSTELLATION_VALIDATE_SP3_IDS
            .iter()
            .map(|id| c_string(id))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "#define PIN_CONSTELLATION_VALIDATE_SP3_ID_COUNT ((size_t){})",
        CONSTELLATION_VALIDATE_SP3_IDS.len()
    );
    println!("#define PIN_CONSTELLATION_RECORD_COUNT ((size_t){constellation_records})");
    println!(
        "static const char PIN_CONSTELLATION_CSV[] = {};",
        c_string(&constellation_csv)
    );
    println!(
        "#define PIN_CONSTELLATION_VALID {}",
        if constellation_valid { "true" } else { "false" }
    );
    for (system, _) in &constellation_validation.inactive_unusable_prns {
        assert_eq!(
            *system,
            sidereon_core::GnssSystem::Gps,
            "inactive or unusable PRN outside GPS"
        );
    }
    println!(
        "static const unsigned short PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRNS[] = {{ {} }};",
        if constellation_validation.inactive_unusable_prns.is_empty() {
            // C has no empty initializer; the count below is zero.
            "0".to_string()
        } else {
            constellation_validation
                .inactive_unusable_prns
                .iter()
                .map(|(_, prn)| prn.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    println!(
        "#define PIN_CONSTELLATION_INACTIVE_UNUSABLE_PRN_COUNT ((size_t){})",
        constellation_validation.inactive_unusable_prns.len()
    );
    println!(
        "#define PIN_CONSTELLATION_MISSING_SP3_ID_COUNT ((size_t){})",
        constellation_validation.missing_sp3_ids.len()
    );
    println!(
        "#define PIN_CONSTELLATION_EXTRA_SP3_ID_COUNT ((size_t){})",
        constellation_validation.extra_sp3_ids.len()
    );
    println!();
    println!("#endif /* SIDEREON_ENGINE_PINS_FIXTURE_H */");
}

/// The SP3 ids smoke.c validates the merged GPS catalog against: the
/// operational PRNs of the catalog.
const CONSTELLATION_VALIDATE_SP3_IDS: [&str; 3] = ["G03", "G05", "G13"];

/// smoke.c's constellation checks: the GPS catalog sidereon_constellation_build
/// forms from the committed CelesTrak gps-ops OMM/JSON and NAVCEN overlay, its
/// CSV with lowercase booleans, and its validation against
/// CONSTELLATION_VALIDATE_SP3_IDS.
fn constellation() -> (
    usize,
    String,
    sidereon_core::constellation::Validation,
    bool,
) {
    use sidereon_core::constellation::{
        from_celestrak_omm, is_valid, merge_navcen, parse_navcen, to_csv, validate_against_sp3_ids,
        BoolStyle,
    };
    let omm_text = read(&format!(
        "{TESTS}/fixtures/constellation/gps_ops_sample.json"
    ));
    let navcen = std::fs::read(format!(
        "{TESTS}/fixtures/constellation/navcen_gps_sample.html"
    ))
    .expect("read NAVCEN overlay");
    let omms = sidereon_core::astro::omm::parse_json_array(&omm_text).expect("OMM JSON array");
    let records =
        from_celestrak_omm(sidereon_core::GnssSystem::Gps, &omms.omms).expect("CelesTrak records");
    let statuses = parse_navcen(&navcen).expect("NAVCEN overlay");
    let records = merge_navcen(&records, &statuses);
    let csv = to_csv(&records, BoolStyle::Lower);
    let validation = validate_against_sp3_ids(&records, &CONSTELLATION_VALIDATE_SP3_IDS);
    let valid = is_valid(&validation);
    (records.len(), csv, validation, valid)
}

/// A C string literal holding `text` exactly.
fn c_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c => panic!("character {c:?} outside the C literal set"),
        }
    }
    out.push('"');
    out
}

fn system_macro(system: sidereon_core::GnssSystem) -> String {
    format!("{system:?}").to_uppercase()
}

/// round2_parity_smoke.c test_tle_fit: the ISS TLE propagated at three
/// instants, each sample labelled with the split Julian date the test forms
/// from its civil fields, fitted with the test's configuration.
fn tle_fit() -> sidereon_core::astro::sgp4::TleFit {
    use sidereon_core::astro::passes::{propagate_teme_arc, UtcInstant};
    use sidereon_core::astro::sgp4::{
        fit_tle, parse_tle_file_with_policy, FitConfig, FitEpoch, FitSample, JulianDate, Loss,
        OpsMode, TleMetadata,
    };
    use sidereon_core::astro::tle::TlePolicy;
    let text = "ISS\n\
        1 25544U 98067A   26168.18949189  .00009113  00000+0  17172-3 0  9996\n\
        2 25544  51.6332 300.0813 0004737 195.1146 164.9702 15.49273435571752\n";
    let file = parse_tle_file_with_policy(text, OpsMode::Improved, TlePolicy::Strict);
    let satellite = &file.satellites.first().expect("fit TLE").satellite;
    let unix_us: [i64; 3] = [1781670172099296, 1781670772099296, 1781671372099296];
    let minute = [22.0_f64, 32.0, 42.0];
    let instants: Vec<UtcInstant> = unix_us
        .iter()
        .map(|us| UtcInstant::from_unix_microseconds(*us))
        .collect();
    let states = propagate_teme_arc(satellite, &instants).expect("propagate fit samples");
    let samples: Vec<FitSample> = states
        .iter()
        .zip(minute)
        .map(|(state, minute)| FitSample {
            // The test's expression, in its order.
            epoch: JulianDate(
                2461208.5,
                4.0 / 24.0 + minute / 1440.0 + 52.0 / 86400.0 + 99296.0 / 86400000000.0,
            ),
            position_teme_km: state.position,
            velocity_teme_km_s: Some(state.velocity),
        })
        .collect();
    // sidereon_sgp4_fit_config_init with the test's overrides.
    let mut config = FitConfig::default();
    config.epoch = FitEpoch::Midpoint;
    config.fit_bstar = true;
    config.bstar_seed = 0.0;
    config.use_velocity = true;
    config.velocity_weight_s = None;
    config.weights = None;
    config.opsmode = OpsMode::Improved;
    config.ftol = None;
    config.xtol = None;
    config.gtol = None;
    config.max_nfev = Some(80);
    config.x_scale = None;
    config.loss = Loss::Linear;
    config.f_scale = 1.0;
    config.metadata = TleMetadata {
        catalog_number: 25544,
        classification: "U".to_string(),
        international_designator: String::new(),
        element_set_number: 999,
        rev_at_epoch: 57175,
        object_name: String::new(),
    };
    fit_tle(&samples, &config).expect("fit TLE")
}

/// round2_parity_smoke.c test_qc: observation QC of the 120-epoch ESBC file
/// under sidereon_observation_qc_options_init's options, the engine defaults.
fn observation_qc(core: &str) -> sidereon_core::observation_qc::ObservationQcReport {
    let text = read(&format!(
        "{core}/obs/ESBC00DNK_R_20201770000_01D_30S_MO_120epoch.rnx"
    ));
    let obs = RinexObs::parse(&text).expect("parse QC observation file");
    sidereon_core::observation_qc::observation_qc_with_options(
        &obs,
        sidereon_core::observation_qc::ObservationQcOptions::default(),
    )
    .expect("observation QC")
}

/// rinex_nav_clock_smoke.c: the record count of the broadcast store the test
/// builds from the ESBC mixed NAV file and its GLONASS record, and the length
/// of the NAV text written for the file's first raw record.
fn nav_counts() -> (usize, usize) {
    let nav = read(&format!(
        "{TESTS}/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx"
    ));
    let glonass = c_string_literals(NAV_SOURCE, "static const char *glonass_fixture(void) {");
    let combined = format!("{nav}{glonass}");
    let store =
        sidereon_core::ephemeris::BroadcastEphemeris::from_nav(&combined).expect("broadcast store");
    let records = sidereon_core::rinex::nav::parse_nav(&nav).expect("raw NAV records");
    let encoded = sidereon_core::rinex::nav::encode_nav(&records[..1]).expect("encode NAV");
    (store.records().len(), encoded.len())
}
