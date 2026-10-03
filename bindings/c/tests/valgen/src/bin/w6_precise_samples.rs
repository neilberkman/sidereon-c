//! precise_samples_smoke.c: sidereon-core's precise-ephemeris samples of the
//! GRG SP3, the batch range predictions from the SP3 and from a source rebuilt
//! from those samples, and the refusals of an empty and a single-sample set.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::ephemeris::{PreciseEphemerisSamples, Sp3};
use sidereon_core::observables::{
    predict_ranges, ObservableEphemerisSource, PredictOptions, RangePrediction,
    RangePredictionRequest,
};
use valgen::{read_bytes, tests_path};

fn predictions(
    name: &str,
    source: &dyn ObservableEphemerisSource,
    requests: &[RangePredictionRequest],
) {
    let mut out = vec![
        RangePrediction {
            geometric_range_m: 0.0,
            sat_clock_s: None,
            transmit_time_j2000_s: 0.0,
            sat_pos_ecef_m: [0.0; 3],
        };
        requests.len()
    ];
    predict_ranges(source, requests, PredictOptions::default(), &mut out).expect("predict");
    def_bits_array(
        &format!("{name}_RANGE_BITS"),
        &out.iter().map(|p| p.geometric_range_m).collect::<Vec<_>>(),
    );
    def_bits_array(
        &format!("{name}_TRANSMIT_BITS"),
        &out.iter()
            .map(|p| p.transmit_time_j2000_s)
            .collect::<Vec<_>>(),
    );
    println!(
        "static const bool {name}_HAS_CLOCK[{}] = {{ {} }};",
        out.len(),
        out.iter()
            .map(|p| p.sat_clock_s.is_some().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    def_bits_array(
        &format!("{name}_CLOCK_BITS"),
        &out.iter()
            .map(|p| p.sat_clock_s.unwrap_or(0.0))
            .collect::<Vec<_>>(),
    );
    def_bits_array(
        &format!("{name}_SAT_POS_BITS"),
        &out.iter()
            .flat_map(|p| p.sat_pos_ecef_m)
            .collect::<Vec<_>>(),
    );
}

fn main() {
    let guard = "SIDEREON_W6_PRECISE_SAMPLES_PINS_H";
    valgen::header_start("w6_precise_samples", guard);
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3");
    let samples = sp3.precise_ephemeris_samples();
    comment("Sample count and the interior sample the test predicts around.");
    def_size("W6_PS_SAMPLE_COUNT", samples.len());
    let mid = samples.len() / 2;
    let t0 = sample_j2000_s(&samples[mid]);
    def_str("W6_PS_MID_SAT", &samples[mid].sat.to_string());
    def_bits("W6_PS_MID_EPOCH_BITS", t0);
    let receiver = [4027894.0, 307046.0, 4919474.0];
    let requests: Vec<RangePredictionRequest> = [t0 - 300.0, t0, t0 + 300.0]
        .into_iter()
        .map(|t| RangePredictionRequest::new(samples[mid].sat, receiver, t))
        .collect();
    comment("Range predictions from the SP3.");
    predictions("W6_PS_SP3", &sp3, &requests);
    let rebuilt =
        PreciseEphemerisSamples::from_samples(samples_through_c(&samples)).expect("samples source");
    comment("Range predictions from the source rebuilt from the copied-out samples.");
    predictions("W6_PS_SAMPLES", &rebuilt, &requests);

    comment("from_samples refuses an empty set and a single-sample satellite (true).");
    def_bool(
        "W6_PS_EMPTY_REFUSED",
        PreciseEphemerisSamples::from_samples(Vec::new()).is_err(),
    );
    let mut lone = samples_through_c(&samples[mid..=mid])[0];
    lone.clock_event = false;
    def_bool(
        "W6_PS_LONE_REFUSED",
        PreciseEphemerisSamples::from_samples(vec![lone]).is_err(),
    );
    valgen::header_end(guard);
}
