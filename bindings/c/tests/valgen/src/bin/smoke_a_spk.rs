// smoke.c exercise_spk_surface: sidereon-core's own Eros-from-Sun states at
// the CSPICE reference epochs of spk_fixture.h, which the smoke compares bit
// for bit (the CSPICE rows stay a separate reference check), and the engine's
// refusals of a garbage kernel and a NaN epoch.

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::astro::Spk;
use support::*;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_SPK_PINS_H";

/// The first column (ET seconds TDB) of every SPK_REFERENCE row, read as the
/// C compiler reads the decimal literal (both round correctly).
fn reference_epochs(header: &str) -> Vec<f64> {
    let start = header
        .find("SPK_REFERENCE[][7] = {")
        .expect("SPK_REFERENCE in spk_fixture.h");
    let body = &header[start + "SPK_REFERENCE[][7] = {".len()..];
    let end = body.find("};").expect("end of SPK_REFERENCE");
    body[..end]
        .split('{')
        .skip(1)
        .map(|row| {
            row.split(',')
                .next()
                .expect("epoch column")
                .trim()
                .parse::<f64>()
                .expect("epoch literal")
        })
        .collect()
}

/// A `#define NAME value` integer from spk_fixture.h.
fn header_i32(header: &str, name: &str) -> i32 {
    let marker = format!("#define {name} ");
    let start = header.find(&marker).expect("define") + marker.len();
    header[start..]
        .lines()
        .next()
        .expect("value")
        .trim()
        .parse()
        .expect("integer")
}

fn main() {
    let header = read(&tests_path("spk_fixture.h"));
    let target = header_i32(&header, "SPK_TARGET");
    let center = header_i32(&header, "SPK_CENTER");
    let epochs = reference_epochs(&header);
    let spk = Spk::from_bytes(&read_bytes(&tests_path(
        "fixtures/spk/horizons_eros_type21.bsp",
    )))
    .expect("load SPK kernel");

    header_start("smoke_a_spk", GUARD);
    comment("Spk::spk_state(target, center, et) at each SPK_REFERENCE epoch.");
    def_usize("SMOKE_A_SPK_STATE_COUNT", epochs.len());
    let mut positions = Vec::new();
    let mut velocities = Vec::new();
    for et in &epochs {
        let state = spk
            .spk_state(target, center, *et)
            .expect("reference-epoch state");
        positions.extend_from_slice(&state.position_km);
        velocities.extend_from_slice(&state.velocity_km_s);
    }
    def_bits_array("SMOKE_A_SPK_POSITION_KM_BITS", &positions);
    def_bits_array("SMOKE_A_SPK_VELOCITY_KM_S_BITS", &velocities);

    comment("Spk::from_bytes of smoke.c's 64 bytes of 0xAB.");
    match Spk::from_bytes(&[0xAB; 64]) {
        Ok(_) => {
            def_bool("SMOKE_A_SPK_GARBAGE_REFUSED", false);
            def_str("SMOKE_A_SPK_GARBAGE_ERROR_TEXT", "");
        }
        Err(err) => {
            def_bool("SMOKE_A_SPK_GARBAGE_REFUSED", true);
            def_str("SMOKE_A_SPK_GARBAGE_ERROR_TEXT", &err.to_string());
        }
    }
    comment("Spk::spk_state at a NaN epoch.");
    match spk.spk_state(target, center, f64::NAN) {
        Ok(_) => {
            def_bool("SMOKE_A_SPK_NAN_REFUSED", false);
            def_str("SMOKE_A_SPK_NAN_ERROR_TEXT", "");
        }
        Err(err) => {
            def_bool("SMOKE_A_SPK_NAN_REFUSED", true);
            def_str("SMOKE_A_SPK_NAN_ERROR_TEXT", &err.to_string());
        }
    }
    header_end(GUARD);
}
