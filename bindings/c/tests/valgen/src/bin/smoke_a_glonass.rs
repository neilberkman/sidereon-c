// smoke.c exercise_spp_glonass_channels: the GLONASS-only SPP scenario and
// sidereon-core's solves of it.
//
// The scenario is formed here and the smoke reads it from the header: a
// receiver at 55.75 N, 37.62 E, 200 m, the product's 49th epoch of the
// multi-GNSS SP3, and every GLONASS satellite 10 degrees or more above the
// geocentric horizon, with the pseudorange the geometric range minus the
// light-time of the satellite's SP3 clock. Each satellite takes FDMA channel
// 0. The solves are sidereon_core::positioning::solve_with_policy under the
// inputs sidereon_solve_spp_v2 builds from the smoke's fields: ionosphere off;
// ionosphere on with no channel map (refused); ionosphere on with every
// channel; the same with the first used satellite's channel withheld; and
// every channel out of the FDMA allocation (refused).

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::ephemeris::{Sp3, Sp3InterpolationOptions};
use sidereon_core::positioning::{
    solve_with_policy, Corrections, KlobucharCoeffs, Observation, PseudorangeCode,
    ReceiverSolution, SolveInputs, SolvePolicy, SurfaceMet,
};
use sidereon_core::{GnssSatelliteId, GnssSystem};
use std::collections::BTreeMap;
use support::*;
use valgen::{c_string, header_end, header_start, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_SMOKE_A_GLONASS_PINS_H";
const LIGHT_M_S: f64 = 299_792_458.0;
const PI: f64 = std::f64::consts::PI;
const MAX_SATS: usize = 32;

/// WGS84 geodetic to ECEF, metres.
fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, h_m: f64) -> [f64; 3] {
    let a = 6_378_137.0;
    let f = 1.0 / 298.257_223_563;
    let e2 = f * (2.0 - f);
    let lat = lat_deg * PI / 180.0;
    let lon = lon_deg * PI / 180.0;
    let slat = lat.sin();
    let clat = lat.cos();
    let n = a / (1.0 - e2 * slat * slat).sqrt();
    [
        (n + h_m) * clat * lon.cos(),
        (n + h_m) * clat * lon.sin(),
        (n * (1.0 - e2) + h_m) * slat,
    ]
}

fn def_str_array(name: &str, values: &[String]) {
    let body: Vec<String> = values.iter().map(|v| c_string(v)).collect();
    println!(
        "static const char *const {name}[{}] = {{ {} }};",
        values.len().max(1),
        if body.is_empty() {
            "\"\"".to_string()
        } else {
            body.join(", ")
        }
    );
}

fn def_u8_array(name: &str, values: &[u8]) {
    let body: Vec<String> = values.iter().map(u8::to_string).collect();
    println!(
        "static const uint8_t {name}[{}] = {{ {} }};",
        values.len().max(1),
        if body.is_empty() {
            "0".to_string()
        } else {
            body.join(", ")
        }
    );
}

fn solution_pins(prefix: &str, solution: &ReceiverSolution) {
    def_bits_array(
        &format!("{prefix}_POSITION_BITS"),
        &[
            solution.position.x_m,
            solution.position.y_m,
            solution.position.z_m,
        ],
    );
    def_usize(&format!("{prefix}_USED_COUNT"), solution.used_sats.len());
    let used: Vec<String> = solution.used_sats.iter().map(|s| s.to_string()).collect();
    def_str_array(&format!("{prefix}_USED_IDS"), &used);
    def_usize(&format!("{prefix}_TDOP_COUNT"), solution.system_tdops.len());
    let systems: Vec<String> = solution
        .system_tdops
        .iter()
        .map(|(system, _)| variant_name(system))
        .collect();
    def_str_array(&format!("{prefix}_TDOP_SYSTEMS"), &systems);
    let tdops: Vec<f64> = solution.system_tdops.iter().map(|(_, t)| *t).collect();
    def_bits_array(&format!("{prefix}_TDOP_BITS"), &tdops);
    def_bool(&format!("{prefix}_HAS_DOP"), solution.dop.is_some());
    def_bits(
        &format!("{prefix}_DOP_TDOP_BITS"),
        solution.dop.as_ref().map(|dop| dop.tdop).unwrap_or(0.0),
    );
    def_usize(
        &format!("{prefix}_REJECTED_COUNT"),
        solution.rejected_sats.len(),
    );
    let rejected: Vec<String> = solution
        .rejected_sats
        .iter()
        .map(|r| r.satellite_id.to_string())
        .collect();
    def_str_array(&format!("{prefix}_REJECTED_IDS"), &rejected);
    let reasons: Vec<String> = solution
        .rejected_sats
        .iter()
        .map(|r| variant_name(&r.reason))
        .collect();
    def_str_array(&format!("{prefix}_REJECTED_REASONS"), &reasons);
}

/// The names solution_pins writes, standing empty, for a solve the engine
/// refused, so the header keeps one shape.
fn empty_solution_pins(prefix: &str) {
    def_bits_array(&format!("{prefix}_POSITION_BITS"), &[0.0; 3]);
    def_usize(&format!("{prefix}_USED_COUNT"), 0);
    def_str_array(&format!("{prefix}_USED_IDS"), &[]);
    def_usize(&format!("{prefix}_TDOP_COUNT"), 0);
    def_str_array(&format!("{prefix}_TDOP_SYSTEMS"), &[]);
    def_bits_array(&format!("{prefix}_TDOP_BITS"), &[]);
    def_bool(&format!("{prefix}_HAS_DOP"), false);
    def_bits(&format!("{prefix}_DOP_TDOP_BITS"), 0.0);
    def_usize(&format!("{prefix}_REJECTED_COUNT"), 0);
    def_str_array(&format!("{prefix}_REJECTED_IDS"), &[]);
    def_str_array(&format!("{prefix}_REJECTED_REASONS"), &[]);
}

fn refusal_pins(prefix: &str, result: &Result<ReceiverSolution, String>) {
    match result {
        Ok(_) => {
            def_bool(&format!("{prefix}_REFUSED"), false);
            def_str(&format!("{prefix}_ERROR_TEXT"), "");
        }
        Err(text) => {
            def_bool(&format!("{prefix}_REFUSED"), true);
            def_str(&format!("{prefix}_ERROR_TEXT"), text);
        }
    }
}

fn main() {
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("parse SP3")
    .with_interpolation_options(Sp3InterpolationOptions::default());

    let epochs = sp3.epochs_j2000_seconds();
    let t_rx = epochs[48];
    let rx = geodetic_to_ecef(55.75, 37.62, 200.0);
    let rx_radius = (rx[0] * rx[0] + rx[1] * rx[1] + rx[2] * rx[2]).sqrt();

    let mut ids: Vec<GnssSatelliteId> = Vec::new();
    let mut pseudoranges: Vec<f64> = Vec::new();
    for sat in sp3.satellites().iter().copied() {
        if ids.len() >= MAX_SATS || sat.system != GnssSystem::Glonass {
            continue;
        }
        let Ok(state) = sp3.position_at_j2000_seconds(sat, t_rx) else {
            continue;
        };
        let pos = state.position.as_array();
        let clk = state.clock_s.unwrap_or(f64::NAN);
        if !pos.iter().all(|v| v.is_finite()) || !clk.is_finite() {
            continue;
        }
        let los = [pos[0] - rx[0], pos[1] - rx[1], pos[2] - rx[2]];
        let range = (los[0] * los[0] + los[1] * los[1] + los[2] * los[2]).sqrt();
        let up_dot = ((los[0] * rx[0] + los[1] * rx[1] + los[2] * rx[2]) / (range * rx_radius))
            .clamp(-1.0, 1.0);
        if up_dot.asin() * 180.0 / PI < 10.0 {
            continue;
        }
        ids.push(sat);
        pseudoranges.push(range - LIGHT_M_S * clk);
    }
    let slots: Vec<u8> = ids.iter().map(|sat| sat.prn).collect();

    let base_inputs = |ionosphere: bool, channels: BTreeMap<u8, i8>| {
        let mut inputs = SolveInputs::default();
        inputs.observations = ids
            .iter()
            .zip(&pseudoranges)
            .map(|(sat, pr)| Observation {
                satellite_id: *sat,
                pseudorange_m: *pr,
            })
            .collect();
        inputs.t_rx_j2000_s = t_rx;
        inputs.t_rx_second_of_day_s = 0.0;
        inputs.day_of_year = 176.0;
        inputs.initial_guess = [6_378_137.0, 0.0, 0.0, 0.0];
        inputs.corrections = Corrections {
            ionosphere,
            troposphere: false,
        };
        inputs.klobuchar = KlobucharCoeffs {
            alpha: [1e-8, 0.0, 0.0, 0.0],
            beta: [1e5, 0.0, 0.0, 0.0],
        };
        inputs.beidou_klobuchar = None;
        inputs.galileo_nequick = None;
        inputs.sbas_iono = None;
        inputs.glonass_channels = channels;
        inputs.met = SurfaceMet {
            pressure_hpa: 1013.25,
            temperature_k: 288.15,
            relative_humidity: 0.5,
        };
        inputs.robust = None;
        inputs.pseudorange_code = PseudorangeCode::SingleFrequency;
        inputs
    };
    let solve = |inputs: &SolveInputs| {
        solve_with_policy(&sp3, inputs, true, SolvePolicy::default()).map_err(|e| e.to_string())
    };
    let all_channels: BTreeMap<u8, i8> = slots.iter().map(|slot| (*slot, 0)).collect();

    let off = solve(&base_inputs(false, BTreeMap::new())).expect("ionosphere-off solve");
    let gated = solve(&base_inputs(true, BTreeMap::new()));
    let on = solve(&base_inputs(true, all_channels.clone())).expect("ionosphere-on solve");
    let withheld = on.used_sats.first().copied().expect("a used satellite");
    let partial_channels: BTreeMap<u8, i8> = all_channels
        .iter()
        .filter(|(slot, _)| **slot != withheld.prn)
        .map(|(slot, channel)| (*slot, *channel))
        .collect();
    let partial = solve(&base_inputs(true, partial_channels));
    let bad_channels: BTreeMap<u8, i8> = slots.iter().map(|slot| (*slot, 9)).collect();
    let bad = solve(&base_inputs(true, bad_channels));

    header_start("smoke_a_glonass", GUARD);
    comment("The scenario the smoke passes to sidereon_solve_spp_v2.");
    def_bits("SMOKE_A_GLONASS_T_RX_BITS", t_rx);
    def_bits_array("SMOKE_A_GLONASS_RX_BITS", &rx);
    def_usize("SMOKE_A_GLONASS_SAT_COUNT", ids.len());
    let id_texts: Vec<String> = ids.iter().map(|sat| sat.to_string()).collect();
    def_str_array("SMOKE_A_GLONASS_SAT_IDS", &id_texts);
    def_bits_array("SMOKE_A_GLONASS_PSEUDORANGE_BITS", &pseudoranges);
    def_u8_array("SMOKE_A_GLONASS_SLOTS", &slots);

    comment("Ionosphere off, no channel map.");
    solution_pins("SMOKE_A_GLONASS_OFF", &off);
    comment("Ionosphere on, no channel map.");
    refusal_pins("SMOKE_A_GLONASS_GATED", &gated);
    comment("Ionosphere on, every channel 0.");
    solution_pins("SMOKE_A_GLONASS_ON", &on);
    comment("Ionosphere on, the first used satellite's channel withheld.");
    def_str("SMOKE_A_GLONASS_WITHHELD_ID", &withheld.to_string());
    refusal_pins("SMOKE_A_GLONASS_PARTIAL", &partial);
    match &partial {
        Ok(partial) => solution_pins("SMOKE_A_GLONASS_PARTIAL", partial),
        // The engine refused the solve: the smoke checks the refusal, and the
        // solution pins stand empty so the header keeps one shape.
        Err(_) => empty_solution_pins("SMOKE_A_GLONASS_PARTIAL"),
    }
    comment("Ionosphere on, every channel 9 (outside the FDMA allocation).");
    refusal_pins("SMOKE_A_GLONASS_BAD_CHANNEL", &bad);
    header_end(GUARD);
}
