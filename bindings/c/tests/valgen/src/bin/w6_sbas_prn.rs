//! sbas_prn_smoke.c: the satellite token sidereon_core::sbas::sbas_prn_to_sat
//! gives broadcast PRN 120, and its answer for PRN 119.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::sbas::sbas_prn_to_sat;

fn main() {
    let guard = "SIDEREON_W6_SBAS_PRN_PINS_H";
    valgen::header_start("w6_sbas_prn", guard);
    let mapped = sbas_prn_to_sat(120)
        .expect("PRN 120 maps to a satellite")
        .to_string();
    comment("sbas_prn_to_sat(120), rendered as the binding renders it (to_string).");
    def_str("W6_SBAS_PRN_120_TOKEN", &mapped);
    def_size("W6_SBAS_PRN_120_TOKEN_LEN", mapped.len());
    comment("sbas_prn_to_sat(119): whether a satellite is named.");
    def_bool("W6_SBAS_PRN_119_MAPPED", sbas_prn_to_sat(119).is_some());
    valgen::header_end(guard);
}
