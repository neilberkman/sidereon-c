//! fixed_policy_smoke.c: sidereon-core's six-by-six covariance results,
//! calendar values, RINEX frequency policy, default ionosphere-free pairs and
//! LNAV word decoding for the inputs that test passes.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::covariance::{
    covariance6_km_to_m, eci_to_rtn_covariance6, interpolate_covariance_psd,
    rtn_to_eci_covariance6, Covariance6,
};
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::civil;
use sidereon_core::frequencies;
use sidereon_core::navigation::lnav;
use sidereon_core::GnssSystem;

fn def_matrix(name: &str, covariance: &Covariance6) {
    let flat: Vec<f64> = covariance.as_matrix().iter().flatten().copied().collect();
    def_bits_array(name, &flat);
}

fn def_optional_bits(name: &str, value: Option<f64>) {
    def_bool(&format!("{name}_PRESENT"), value.is_some());
    def_bits(&format!("{name}_BITS"), value.unwrap_or(0.0));
}

/// The test's set_bits: `value` written MSB first as 0/1 bytes.
fn set_bits(bits: &mut [u8], offset: usize, width: usize, value: u64) {
    for index in 0..width {
        bits[offset + index] = ((value >> (width - index - 1)) & 1) as u8;
    }
}

fn main() {
    let guard = "SIDEREON_W6_FIXED_POLICY_PINS_H";
    valgen::header_start("w6_fixed_policy", guard);

    // Covariance.
    let covariance =
        Covariance6::from_diagonal([1.0, 4.0, 9.0, 16.0, 25.0, 36.0]).expect("diagonal");
    comment("Covariance6::from_diagonal(1, 4, 9, 16, 25, 36), row major.");
    def_matrix("W6_FP_COVARIANCE_BITS", &covariance);
    def_bool("W6_FP_COVARIANCE_SYMMETRIC", covariance.is_symmetric());
    def_bool(
        "W6_FP_COVARIANCE_PSD",
        covariance.is_positive_semidefinite(),
    );
    comment("Refusals: a negative diagonal, and the same matrix with element [5][5] = -1.");
    def_bool(
        "W6_FP_NEGATIVE_DIAGONAL_REFUSED",
        Covariance6::from_diagonal([1.0, 2.0, 3.0, 4.0, 5.0, -1.0]).is_err(),
    );
    let mut invalid = *covariance.as_matrix();
    invalid[5][5] = -1.0;
    def_bool(
        "W6_FP_INVALID_MATRIX_REFUSED",
        Covariance6::try_from_matrix(invalid).is_err(),
    );
    comment("covariance6_km_to_m.");
    def_matrix(
        "W6_FP_KM_TO_M_BITS",
        &covariance6_km_to_m(&covariance).expect("km to m"),
    );
    let other = Covariance6::from_diagonal([4.0, 9.0, 16.0, 25.0, 36.0, 49.0]).expect("other");
    comment("interpolate_covariance_psd at u = 0.5.");
    def_matrix(
        "W6_FP_INTERPOLATED_HALF_BITS",
        &interpolate_covariance_psd(&covariance, &other, 0.5).expect("interpolate"),
    );
    let state = CartesianState::new(0.0, [7000.0, 1000.0, 2000.0], [-1.0, 7.2, 2.0]);
    let rtn = eci_to_rtn_covariance6(&covariance, &state).expect("ECI to RTN");
    comment("eci_to_rtn_covariance6 and back with rtn_to_eci_covariance6.");
    def_matrix("W6_FP_RTN_BITS", &rtn);
    def_matrix(
        "W6_FP_RTN_ROUND_TRIP_BITS",
        &rtn_to_eci_covariance6(&rtn, &state).expect("RTN to ECI"),
    );
    let zero_state = CartesianState::new(0.0, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    def_bool(
        "W6_FP_ZERO_POSITION_RTN_REFUSED",
        eci_to_rtn_covariance6(&covariance, &zero_state).is_err(),
    );

    // Calendar.
    comment("Calendar values.");
    def_bits("W6_FP_SECOND_OF_DAY_BITS", civil::second_of_day(1, 2, 3.5));
    def_bits(
        "W6_FP_DOY_2024_01_01_BITS",
        civil::day_of_year(2024, 1, 1, 0, 0, 0.0),
    );
    def_bits(
        "W6_FP_DOY_2024_02_29_BITS",
        civil::day_of_year(2024, 2, 29, 12, 0, 0.25),
    );
    def(
        "W6_FP_PRODUCT_DOY_2020_03_01",
        sidereon_core::data::day_of_year(
            sidereon_core::data::ProductDate::new(2020, 3, 1).expect("date"),
        ),
    );
    def_bool(
        "W6_FP_2023_02_29_REFUSED",
        sidereon_core::data::ProductDate::new(2023, 2, 29).is_err(),
    );

    // RINEX frequency policy.
    comment("RINEX band and observation frequencies (PRESENT false: no frequency).");
    def_optional_bits(
        "W6_FP_GPS_BAND1_HZ",
        frequencies::rinex_band_frequency_hz(GnssSystem::Gps, '1', None),
    );
    def_optional_bits(
        "W6_FP_GPS_BAND1_WAVELENGTH_M",
        frequencies::rinex_band_wavelength_m(GnssSystem::Gps, '1', None),
    );
    def_optional_bits(
        "W6_FP_GLONASS_BAND1_CHANNEL_M4_HZ",
        frequencies::rinex_band_frequency_hz(GnssSystem::Glonass, '1', Some(-4)),
    );
    def_optional_bits(
        "W6_FP_GLONASS_BAND1_NO_CHANNEL_HZ",
        frequencies::rinex_band_frequency_hz(GnssSystem::Glonass, '1', None),
    );
    def_optional_bits(
        "W6_FP_BDS_C1I_302_HZ",
        frequencies::rinex_observation_frequency_hz(GnssSystem::BeiDou, "C1I", 3.02, None),
    );
    def_optional_bits(
        "W6_FP_BDS_C1I_303_HZ",
        frequencies::rinex_observation_frequency_hz(GnssSystem::BeiDou, "C1I", 3.03, None),
    );
    def_optional_bits(
        "W6_FP_BDS_C1I_303_WAVELENGTH_M",
        frequencies::rinex_observation_wavelength_m(GnssSystem::BeiDou, "C1I", 3.03, None),
    );
    comment("default_iono_free_pair per system.");
    for (label, system) in [
        ("GPS", GnssSystem::Gps),
        ("GALILEO", GnssSystem::Galileo),
        ("BEIDOU", GnssSystem::BeiDou),
        ("GLONASS", GnssSystem::Glonass),
    ] {
        let pair = frequencies::default_iono_free_pair(system);
        def_bool(&format!("W6_FP_PAIR_{label}_PRESENT"), pair.is_some());
        if let Some(pair) = pair {
            def(
                &format!("W6_FP_PAIR_{label}_BAND1"),
                c_enum("SIDEREON_CARRIER_BAND", &pair.band1),
            );
            def(
                &format!("W6_FP_PAIR_{label}_BAND2"),
                c_enum("SIDEREON_CARRIER_BAND", &pair.band2),
            );
        }
    }

    // LNAV words, built as the test builds them.
    let mut how = [0u8; 30];
    set_bits(&mut how, 0, 17, 12345);
    set_bits(&mut how, 19, 3, 5);
    let mut subframe = [0u8; 300];
    subframe[30..60].copy_from_slice(&how);
    comment("LNAV TOW and subframe ID of the HOW word and of the subframe holding it.");
    let opt = |value: Option<u64>| value.map_or(u64::MAX, |v| v);
    def(
        "W6_FP_LNAV_HOW_TOW",
        format!("UINT64_C({})", opt(lnav::tow(&how))),
    );
    def(
        "W6_FP_LNAV_HOW_SUBFRAME_ID",
        format!("UINT64_C({})", opt(lnav::subframe_id(&how))),
    );
    def(
        "W6_FP_LNAV_SUBFRAME_TOW",
        format!("UINT64_C({})", opt(lnav::tow(&subframe))),
    );
    def(
        "W6_FP_LNAV_SUBFRAME_SUBFRAME_ID",
        format!("UINT64_C({})", opt(lnav::subframe_id(&subframe))),
    );
    def_bool(
        "W6_FP_LNAV_29_BIT_TOW_REFUSED",
        lnav::tow(&[0u8; 29]).is_none(),
    );
    let parity = lnav::parity(&[0u8; 24], 1, 0).expect("parity");
    comment("LNAV parity of 24 zero data bits with D29* = 1, D30* = 0.");
    println!(
        "static const uint8_t W6_FP_LNAV_PARITY[6] = {{ {} }};",
        parity
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut word = [0u8; 30];
    word[24..].copy_from_slice(&parity);
    def_bool("W6_FP_LNAV_WORD_VALID", lnav::parity_valid(&word, 1, 0));
    word[0] = 1;
    def_bool(
        "W6_FP_LNAV_FLIPPED_WORD_VALID",
        lnav::parity_valid(&word, 1, 0),
    );
    def_bool(
        "W6_FP_LNAV_29_BIT_PARITY_REFUSED",
        lnav::parity(&[0u8; 29], 0, 0).is_err(),
    );
    valgen::header_end(guard);
}
