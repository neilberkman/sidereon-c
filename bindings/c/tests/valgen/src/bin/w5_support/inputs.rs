//! Inputs the w5_* binaries share with the smoke programs: the SPP receiver
//! and epoch (from the core SPP fixture and tests/fixtures/spp_expected.json,
//! the sources tests/gen_fixture_header.py writes spp_fixture.h from) and the
//! broadcast receiver and satellites (from tests/broadcast_fixture.h, which
//! tests/fbgen writes before valgen runs).

#![allow(dead_code)]

use valgen::{core_fixtures, read, tests_path};

/// The SPP satellites, receive epoch and engine solution the smoke programs
/// read as SPP_SAT_IDS, SPP_T_RX_J2000_S_BITS and SPP_EXPECTED_X_BITS.
pub struct Spp {
    pub sat_ids: Vec<String>,
    pub t_rx_j2000_s: f64,
    pub position_m: [f64; 3],
}

pub fn spp() -> Spp {
    let fixture: serde_json::Value = serde_json::from_str(&read(&format!(
        "{}/spp_trace_L0_minimal.json",
        core_fixtures()
    )))
    .expect("SPP fixture JSON");
    let inputs = &fixture["fixture"]["inputs"];
    let sat_ids = inputs["observations"]
        .as_array()
        .expect("observations")
        .iter()
        .map(|o| o["sat_id"].as_str().expect("sat_id").to_string())
        .collect();
    let t_rx_j2000_s = fixture_f64(&inputs["t_rx_j2000_s"], "t_rx_j2000_s");
    let expected: serde_json::Value =
        serde_json::from_str(&read(&tests_path("fixtures/spp_expected.json")))
            .expect("spp_expected.json");
    let bits: Vec<f64> = expected["position_m"]
        .as_array()
        .expect("position_m")
        .iter()
        .map(|v| {
            let text = v.as_str().expect("bit pattern");
            f64::from_bits(u64::from_str_radix(text.trim_start_matches("0x"), 16).expect("hex"))
        })
        .collect();
    Spp {
        sat_ids,
        t_rx_j2000_s,
        position_m: [bits[0], bits[1], bits[2]],
    }
}

/// The text between `NAME` and the next `;` in a generated C header.
fn header_item(header: &str, name: &str) -> String {
    let text = read(&tests_path(header));
    let marker = format!(" {name}");
    let start = text
        .find(&format!("{marker}[").as_str())
        .or_else(|| text.find(&format!("{marker} =").as_str()))
        .unwrap_or_else(|| panic!("{name} in {header}"));
    let rest = &text[start + marker.len()..];
    rest[..rest.find(';').expect(";")].to_string()
}

/// The doubles a `static const uint64_t NAME... = UINT64_C(...)` item holds.
pub fn header_f64s(header: &str, name: &str) -> Vec<f64> {
    header_item(header, name)
        .split("UINT64_C(")
        .skip(1)
        .map(|chunk| {
            let hex = &chunk[..chunk.find(')').expect(")")];
            f64::from_bits(u64::from_str_radix(hex.trim_start_matches("0x"), 16).expect("hex"))
        })
        .collect()
}

/// The strings a `static const char *const NAME[] = {...}` item holds.
pub fn header_strings(header: &str, name: &str) -> Vec<String> {
    header_item(header, name)
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The broadcast receiver guess, receive epoch and satellites the smoke
/// programs read as BC_INITIAL_GUESS_BITS, BC_T_RX_J2000_S_BITS and BC_SAT_IDS.
pub struct Broadcast {
    pub sat_ids: Vec<String>,
    pub t_rx_j2000_s: f64,
    pub receiver_m: [f64; 3],
}

pub fn broadcast() -> Broadcast {
    let guess = header_f64s("broadcast_fixture.h", "BC_INITIAL_GUESS_BITS");
    Broadcast {
        sat_ids: header_strings("broadcast_fixture.h", "BC_SAT_IDS"),
        t_rx_j2000_s: header_f64s("broadcast_fixture.h", "BC_T_RX_J2000_S_BITS")[0],
        receiver_m: [guess[0], guess[1], guess[2]],
    }
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
