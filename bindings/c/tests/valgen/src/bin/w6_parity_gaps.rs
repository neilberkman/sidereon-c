//! parity_gaps_smoke.c: sidereon-core's GPS LNAV encoding of the IS-GPS-200
//! example parameters test_lnav passes, and its decoding of those subframes.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::navigation::lnav::{
    self, LnavNumber::Float, LnavNumber::Int, LnavOptions, LnavParams,
};

fn def_bits_u8(name: &str, bits: &[u8]) {
    println!(
        "static const uint8_t {name}[{}] = {{ {} }};",
        bits.len(),
        bits.iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn main() {
    let guard = "SIDEREON_W6_PARITY_GAPS_PINS_H";
    valgen::header_start("w6_parity_gaps", guard);
    // test_lnav's parameters and options.
    let params = LnavParams {
        week_number: Int(290),
        l2_code: Int(1),
        l2_p_data_flag: Int(0),
        ura_index: Int(0),
        sv_health: Int(0),
        iodc: Int(0x2AB),
        tgd: Float(-5.587935447692871e-9),
        toc: Int(504000),
        af0: Float(-1.234e-4),
        af1: Float(-3.5e-12),
        af2: Float(0.0),
        iode: Int(0xAB),
        crs: Float(-55.625),
        delta_n: Float(1.56e-9),
        m0: Float(-0.35),
        cuc: Float(-1.2e-6),
        eccentricity: Float(0.012),
        cus: Float(8.3e-6),
        sqrt_a: Float(5153.65),
        toe: Int(504000),
        fit_interval_flag: Int(0),
        aodo: Int(0),
        cic: Float(5.0e-8),
        omega0: Float(-0.78),
        cis: Float(-2.1e-7),
        i0: Float(0.305),
        crc: Float(250.625),
        omega: Float(0.95),
        omega_dot: Float(-8.1e-9),
        idot: Float(1.5e-10),
    };
    let options = LnavOptions::new(Int(12345), Int(1), Int(0), Int(1), Int(5461));
    let subframes = lnav::encode(&params, &options).expect("encode");
    comment("lnav::encode: the three subframes, one bit per byte, MSB first.");
    def_bits_u8("W6_PG_LNAV_SF1", &subframes[0]);
    def_bits_u8("W6_PG_LNAV_SF2", &subframes[1]);
    def_bits_u8("W6_PG_LNAV_SF3", &subframes[2]);

    let decoded = lnav::decode(&subframes[0], &subframes[1], &subframes[2]).expect("decode");
    comment("lnav::decode of those subframes.");
    def("W6_PG_LNAV_WEEK_NUMBER", decoded.week_number);
    def("W6_PG_LNAV_IODC", decoded.iodc);
    def("W6_PG_LNAV_IODE", decoded.iode);
    def("W6_PG_LNAV_TOC", decoded.toc);
    def("W6_PG_LNAV_TOE", decoded.toe);
    def_bits("W6_PG_LNAV_CRS_BITS", decoded.crs);
    def_bits("W6_PG_LNAV_ECCENTRICITY_BITS", decoded.eccentricity);
    def_bits("W6_PG_LNAV_SQRT_A_BITS", decoded.sqrt_a);

    let mut flipped = subframes[0].clone();
    flipped[29] ^= 1;
    comment("lnav::decode refuses subframe 1 with bit 29 flipped (true).");
    def_bool(
        "W6_PG_LNAV_FLIPPED_PARITY_REFUSED",
        lnav::decode(&flipped, &subframes[1], &subframes[2]).is_err(),
    );
    valgen::header_end(guard);
}
