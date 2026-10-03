// Solve the inputs of sidereon-core's spp_trace_L0_minimal fixture with the
// engine's default SPP range model and print the solution's exact bits as JSON
// for tests/gen_fixture_header.py. The solve is the one the C smoke tests make
// through sidereon_solve_spp: L0 minimal (no ionosphere, no troposphere), the
// fixture's frozen initial guess, the default solve policy.

use sidereon_core::ephemeris::Sp3;
use sidereon_core::positioning::{
    solve_with_policy, Corrections, KlobucharCoeffs, Observation, SolveInputs, SolvePolicy,
    SurfaceMet,
};
use sidereon_core::GnssSatelliteId;
use std::str::FromStr;

/// The sidereon-core revision checked against this crate's locked graph by
/// tests/run_generators.sh.
const CORE_REVISION: &str = include_str!("../../CORE_REVISION");

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
const FIXTURE: &str = "spp_trace_L0_minimal.json";
const SP3_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/sp3");

fn f64_hex(value: &serde_json::Value) -> f64 {
    let text = value.as_str().expect("fixture value is a hex bit string");
    let digits = text.strip_prefix("0x").expect("hex bit string");
    f64::from_bits(u64::from_str_radix(digits, 16).expect("hex bits"))
}

fn f64x4(value: &serde_json::Value) -> [f64; 4] {
    let values: Vec<f64> = value
        .as_array()
        .expect("fixture array")
        .iter()
        .map(f64_hex)
        .collect();
    values.try_into().expect("four values")
}

fn hex(value: f64) -> String {
    format!("\"{:#018x}\"", value.to_bits())
}

fn main() {
    let text = std::fs::read_to_string(format!("{}/{FIXTURE}", core_fixtures()))
        .expect("read SPP fixture");
    let root: serde_json::Value = serde_json::from_str(&text).expect("parse SPP fixture");
    let fixture = &root["fixture"];
    let inputs = &fixture["inputs"];

    let sp3_file = inputs["sp3_file"].as_str().expect("sp3_file");
    let sp3_bytes = std::fs::read(format!("{SP3_DIR}/{sp3_file}")).expect("read SP3 fixture");
    let sp3 = Sp3::parse(&sp3_bytes).expect("parse SP3 fixture");

    let observations = inputs["observations"]
        .as_array()
        .expect("observations")
        .iter()
        .map(|obs| Observation {
            satellite_id: GnssSatelliteId::from_str(obs["sat_id"].as_str().expect("sat_id"))
                .expect("satellite token"),
            pseudorange_m: f64_hex(&obs["p_meas_m"]),
        })
        .collect();
    let mut solve_inputs = SolveInputs::default();
    solve_inputs.observations = observations;
    solve_inputs.t_rx_j2000_s = f64_hex(&inputs["t_rx_j2000_s"]);
    solve_inputs.t_rx_second_of_day_s = f64_hex(&inputs["t_rx_sod_s"]);
    solve_inputs.day_of_year = f64_hex(&inputs["doy"]);
    solve_inputs.initial_guess = f64x4(&fixture["frozen"]["initial_guess_x0"]);
    solve_inputs.corrections = Corrections::NONE;
    solve_inputs.klobuchar = KlobucharCoeffs {
        alpha: f64x4(&inputs["klobuchar_alpha"]),
        beta: f64x4(&inputs["klobuchar_beta"]),
    };
    solve_inputs.met = SurfaceMet {
        pressure_hpa: f64_hex(&inputs["met"]["pressure_hpa"]),
        temperature_k: f64_hex(&inputs["met"]["temperature_k"]),
        relative_humidity: f64_hex(&inputs["met"]["relative_humidity"]),
    };

    let solution = solve_with_policy(&sp3, &solve_inputs, false, SolvePolicy::default())
        .expect("default SPP solve");
    let position = [
        solution.position.x_m,
        solution.position.y_m,
        solution.position.z_m,
    ];
    println!("{{");
    println!(
        "  \"source\": \"sidereon-core default SPP solve of the spp_trace_L0_minimal inputs, written by tests/sppgen\","
    );
    println!("  \"core_revision\": \"{}\",", CORE_REVISION.trim());
    println!(
        "  \"position_m\": [{}, {}, {}],",
        hex(position[0]),
        hex(position[1]),
        hex(position[2])
    );
    println!("  \"rx_clock_s\": {}", hex(solution.rx_clock_s));
    println!("}}");
}
