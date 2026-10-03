//! track_smoke.c: sidereon-core's track filter predictions, innovations,
//! gated and recorded updates and RTS smoothing for the inputs that test
//! passes.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::estimation::{
    smooth_track_rts, TrackCoordinateFrame, TrackFilter, TrackFilterConfig, TrackRtsHistoryBuilder,
};

fn flat(matrix: &[Vec<f64>]) -> Vec<f64> {
    matrix.iter().flatten().copied().collect()
}

fn gated_spike() {
    comment("1D filter from position 0, velocity 1, unit covariance, q 0.1; predict 1 s.");
    let config = TrackFilterConfig::from_position_velocity(
        TrackCoordinateFrame::CallerDefinedCartesian,
        0.0,
        vec![0.0],
        vec![1.0],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        0.1,
    )
    .expect("config");
    let mut filter = TrackFilter::new(config).expect("filter");
    let mut history = TrackRtsHistoryBuilder::from_filter(&filter).expect("history");
    filter.predict_recorded(1.0, &mut history).expect("predict");
    def_bits_array(
        "W6_TRACK_SPIKE_PREDICTED_POSITION_BITS",
        &filter.state().position_m,
    );
    def_bits_array(
        "W6_TRACK_SPIKE_PREDICTED_COVARIANCE_BITS",
        &flat(&filter.state().covariance),
    );

    comment("Innovation of a 100 m spike with 0.01 m^2 variance, and the gated update at 0.95.");
    let spike = [100.0];
    let spike_covariance = vec![vec![0.01]];
    let innovation = filter
        .position_innovation(&spike, &spike_covariance)
        .expect("innovation");
    def_bits_array("W6_TRACK_SPIKE_INNOVATION_BITS", &innovation.innovation);
    def_bits_array(
        "W6_TRACK_SPIKE_INNOVATION_COVARIANCE_BITS",
        &flat(&innovation.innovation_covariance),
    );
    def_bits("W6_TRACK_SPIKE_NIS_BITS", innovation.nis);
    let gated = filter
        .update_position_gated_recorded(&spike, &spike_covariance, 0.95, &mut history)
        .expect("gated update");
    def_bool("W6_TRACK_SPIKE_IN_GATE", gated.gate.in_gate);
    def_bool("W6_TRACK_SPIKE_HAS_UPDATE", gated.update.is_some());
    def_bits("W6_TRACK_SPIKE_GATE_NIS_BITS", gated.gate.nis);
    def_bits("W6_TRACK_SPIKE_GATE_THRESHOLD_BITS", gated.gate.threshold);
    def_bits_array(
        "W6_TRACK_SPIKE_AFTER_POSITION_BITS",
        &filter.state().position_m,
    );
    def_bits_array(
        "W6_TRACK_SPIKE_AFTER_COVARIANCE_BITS",
        &flat(&filter.state().covariance),
    );

    let recorded = history.finish().expect("finish");
    let smoothed = smooth_track_rts(&recorded).expect("smooth");
    comment("Recorded and smoothed epochs, and the last smoothed position.");
    def_size("W6_TRACK_SPIKE_RECORDED_COUNT", recorded.epochs.len());
    def_size("W6_TRACK_SPIKE_SMOOTHED_COUNT", smoothed.epochs.len());
    def_bits_array(
        "W6_TRACK_SPIKE_LAST_SMOOTHED_POSITION_BITS",
        &smoothed
            .epochs
            .last()
            .expect("smoothed epoch")
            .state
            .position_m,
    );
}

fn recorded_fix() {
    comment("3D ECEF filter from the origin, unit covariance, velocity variance 25, q 0.05.");
    let identity = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    let config = TrackFilterConfig::from_position(
        TrackCoordinateFrame::Ecef,
        0.0,
        vec![0.0, 0.0, 0.0],
        identity,
        25.0,
        0.05,
    )
    .expect("config");
    let mut filter = TrackFilter::new(config).expect("filter");
    let mut history = TrackRtsHistoryBuilder::from_filter(&filter).expect("history");
    filter.predict_recorded(1.0, &mut history).expect("predict");
    let fix_covariance = vec![
        vec![0.25, 0.0, 0.0],
        vec![0.0, 0.25, 0.0],
        vec![0.0, 0.0, 0.25],
    ];
    let update = filter
        .update_position_recorded(&[1.0, 0.0, 0.0], &fix_covariance, &mut history)
        .expect("update");
    comment("Update of a fix at (1, 0, 0) m with 0.25 m^2 variances.");
    def(
        "W6_TRACK_FIX_UPDATED_FRAME",
        c_enum("SIDEREON_TRACK_COORDINATE_FRAME", &update.updated.frame),
    );
    def_size(
        "W6_TRACK_FIX_INNOVATION_DIMENSION",
        update.innovation.innovation.len(),
    );
    def_bits("W6_TRACK_FIX_NIS_BITS", update.innovation.nis);
    def_bits_array(
        "W6_TRACK_FIX_UPDATED_POSITION_BITS",
        &filter.state().position_m,
    );

    let recorded = history.finish().expect("finish");
    let smoothed = smooth_track_rts(&recorded).expect("smooth");
    comment("Recorded and smoothed epochs, the first epoch's RTS gain and covariance.");
    def_size("W6_TRACK_FIX_RECORDED_COUNT", recorded.epochs.len());
    def_size("W6_TRACK_FIX_SMOOTHED_COUNT", smoothed.epochs.len());
    let first = &smoothed.epochs[0];
    def_bool(
        "W6_TRACK_FIX_FIRST_HAS_GAIN",
        first.rts_gain_to_next.is_some(),
    );
    def_size(
        "W6_TRACK_FIX_FIRST_GAIN_LEN",
        first
            .rts_gain_to_next
            .as_ref()
            .map_or(0, |gain| flat(gain).len()),
    );
    def_bits_array(
        "W6_TRACK_FIX_FIRST_COVARIANCE_BITS",
        &flat(&first.state.covariance),
    );
    let last = smoothed.epochs.last().expect("last epoch");
    def_size(
        "W6_TRACK_FIX_LAST_GAIN_LEN",
        last.rts_gain_to_next
            .as_ref()
            .map_or(0, |gain| flat(gain).len()),
    );
}

fn main() {
    let guard = "SIDEREON_W6_TRACK_PINS_H";
    valgen::header_start("w6_track", guard);
    gated_spike();
    recorded_fix();
    valgen::header_end(guard);
}
