//! tests/w5_cap013_pins.h: sidereon-core's results for the inputs
//! cap013_smoke.c passes (SP3 observable states, estimation primitives,
//! source localization).

#[path = "w5_support/pins.rs"]
mod pins;

use pins::{variant_name, Pins};
use sidereon_core::astro::time::InstantRepr;
use sidereon_core::ephemeris::Sp3;
use sidereon_core::estimation as est;
use sidereon_core::observables::ObservableEphemerisSource;
use sidereon_core::source_localization::{
    closed_form_initial_guess, locate_source, source_crlb, source_dop, Sensor, SourceLocateOptions,
    SourceSolveMode,
};
use valgen::{header_end, header_start, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_W5_CAP013_PINS_H";

/// Arrival times as cap013_smoke.c's fill_arrivals forms them.
fn arrivals(sensors: &[Vec<f64>], source: &[f64], origin: f64, speed: f64) -> Vec<f64> {
    sensors
        .iter()
        .map(|position| {
            let mut sum = 0.0;
            for axis in 0..position.len() {
                let delta = source[axis] - position[axis];
                sum += delta * delta;
            }
            origin + sum.sqrt() / speed
        })
        .collect()
}

fn main() {
    header_start("w5_cap013", GUARD);
    let p = Pins::new("W5_CAP013");

    // Observable states on the GRG SP3.
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3")
    .with_interpolation_options(Default::default());
    let samples = sp3.precise_ephemeris_samples();
    let mid = &samples[samples.len() / 2];
    let t0 = match mid.epoch.repr {
        InstantRepr::JulianDate(jd) => {
            sidereon_core::astro::time::civil::j2000_seconds_from_split(jd.jd_whole, jd.fraction)
        }
        InstantRepr::Nanos(_) => panic!("SP3 sample epoch is a Julian date"),
    };
    let sats = [mid.sat, mid.sat, mid.sat];
    let batch = sp3
        .observable_states_at_j2000_s(&sats, &[t0 - 300.0, t0, f64::INFINITY])
        .expect("observable states");
    p.comment("Sp3::precise_ephemeris_samples and observable_states_at_j2000_s at the\nmiddle sample's satellite and epoch (t0 - 300 s, t0, +infinity).");
    p.int("SAMPLE_COUNT", samples.len() as i128);
    p.str("MID_SATELLITE", &mid.sat.to_string());
    p.f64("MID_EPOCH_J2000_S_BITS", t0);
    let mut positions = Vec::new();
    let mut clocks = Vec::new();
    let mut has_clocks = Vec::new();
    for idx in 0..3 {
        positions.extend_from_slice(&batch.positions_ecef_m[idx]);
        clocks.push(batch.clocks_s[idx].unwrap_or(0.0));
        has_clocks.push(i128::from(batch.clocks_s[idx].is_some()));
        let status = batch
            .element_status(idx)
            .map(|s| variant_name(&s))
            .unwrap_or_else(|| "Error".to_string());
        p.variant_named(
            &format!("ELEMENT_{idx}_STATUS"),
            "SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS_",
            &status,
        );
        p.bool(
            &format!("ELEMENT_{idx}_RESULT_OK"),
            batch.element_results[idx].is_ok(),
        );
    }
    p.f64s("STATE_POSITIONS_ECEF_M_BITS", &positions);
    p.f64s("STATE_CLOCKS_S_BITS", &clocks);
    p.ints("STATE_HAS_CLOCKS", "bool", &has_clocks);

    // Estimation primitives.
    p.comment("sidereon_core::estimation primitives on cap013_smoke.c's inputs.");
    let gains = est::alpha_beta_steady_state_gains(0.4).expect("alpha-beta gains");
    p.f64("ALPHA_BITS", gains.alpha);
    p.f64("BETA_BITS", gains.beta);
    let kalman = est::kalman_cv_steady_state_gains(0.4, 2.0, 9.0).expect("kalman gains");
    p.f64("KALMAN_POSITION_GAIN_BITS", kalman.position_gain);
    p.f64("KALMAN_RATE_GAIN_BITS", kalman.rate_gain);
    let step = est::alpha_beta_filter_step(
        est::AlphaBetaState {
            level: 10.0,
            rate: 1.0,
        },
        14.0,
        2.0,
        est::AlphaBetaGains {
            alpha: gains.alpha,
            beta: gains.beta,
        },
    )
    .expect("alpha-beta step");
    p.f64("STEP_PREDICTED_LEVEL_BITS", step.predicted.level);
    p.f64("STEP_PREDICTED_RATE_BITS", step.predicted.rate);
    p.f64("STEP_INNOVATION_BITS", step.innovation);
    p.f64("STEP_UPDATED_LEVEL_BITS", step.updated.level);
    p.f64("STEP_UPDATED_RATE_BITS", step.updated.rate);
    p.f64(
        "NORMALIZED_INNOVATION_BITS",
        est::normalized_innovation(2.0, 4.0).expect("normalized innovation"),
    );
    p.f64("NIS_BITS", est::nis_statistic(2.0, 4.0).expect("nis"));
    p.f64(
        "NIS_EXPECTED_BITS",
        est::nis_expected_value(3).expect("nis expected"),
    );
    p.f64(
        "NIS_THRESHOLD_BITS",
        est::nis_gate_threshold(1, 0.95).expect("nis threshold"),
    );
    let gate = est::nis_gate_test(2.0, 4.0, 1, 0.95).expect("nis gate");
    p.bool("GATE_IN_GATE", gate.in_gate);
    p.f64("GATE_NIS_BITS", gate.nis);
    p.f64("GATE_THRESHOLD_BITS", gate.threshold);
    p.int("GATE_DOF", gate.dof as i128);
    p.f64("MAD_CONSTANT_BITS", est::MAD_GAUSSIAN_CONSISTENCY);
    p.f64(
        "MAD_SPREAD_BITS",
        est::mad_spread(&[1.0, 2.0, 100.0], 0.0).expect("mad spread"),
    );
    p.f64(
        "EWMA_BITS",
        est::ewma_update(10.0, 14.0, 0.25).expect("ewma"),
    );
    p.f64(
        "EWMA_POW2_BITS",
        est::ewma_update_power_of_two(10.0, 14.0, 2).expect("ewma power of two"),
    );
    let multiplier = est::cfar_ca_multiplier_from_pfa(16, 1.0e-3).expect("cfar multiplier");
    p.f64("CFAR_MULTIPLIER_BITS", multiplier);
    p.f64(
        "CFAR_PFA_BITS",
        est::cfar_ca_pfa_from_multiplier(16, multiplier).expect("cfar pfa"),
    );
    let threshold = est::cfar_ca_threshold(16, 1.0e-3, 2.5).expect("cfar threshold");
    p.f64("CFAR_THRESHOLD_BITS", threshold);
    p.f64(
        "CFAR_FALSE_ALARM_BITS",
        est::cfar_ca_false_alarm_probability(16, threshold, 2.5).expect("cfar false alarm"),
    );

    // ToA source localization.
    let toa_positions: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 0.0],
        vec![2.0, 0.0, 0.0],
        vec![0.0, 2.0, 0.0],
        vec![0.0, 0.0, 2.0],
        vec![2.0, 2.0, 2.0],
    ];
    let toa_sensors: Vec<Sensor> = toa_positions.iter().cloned().map(Sensor::new).collect();
    let toa_times = arrivals(&toa_positions, &[0.4, 0.6, 0.5], 1.25, 1.0);
    let guess = closed_form_initial_guess(&toa_sensors, &toa_times, 1.0, SourceSolveMode::Toa)
        .expect("chan-ho toa");
    p.comment(
        "sidereon_core::source_localization on cap013_smoke.c's ToA, TDOA and DOP\nsensor sets.",
    );
    p.int("GUESS_DIMENSION", guess.position_m.len() as i128);
    p.f64s("GUESS_POSITION_M_BITS", &guess.position_m);
    p.bool("GUESS_HAS_ORIGIN_TIME", guess.origin_time_s.is_some());
    p.f64(
        "GUESS_ORIGIN_TIME_S_BITS",
        guess.origin_time_s.unwrap_or(0.0),
    );
    p.f64("GUESS_RESIDUAL_RMS_S_BITS", guess.residual_rms_s);
    let mut options = SourceLocateOptions::default();
    options.timing_sigma_s = 0.001;
    let solution =
        locate_source(&toa_sensors, &toa_times, 1.0, &options).expect("locate source toa");
    p.int("TOA_DIMENSION", solution.position_m.len() as i128);
    p.f64s("TOA_POSITION_M_BITS", &solution.position_m);
    p.bool("TOA_HAS_ORIGIN_TIME", solution.origin_time_s.is_some());
    p.f64(
        "TOA_ORIGIN_TIME_S_BITS",
        solution.origin_time_s.unwrap_or(0.0),
    );
    p.bool("TOA_HAS_COVARIANCE", solution.covariance.is_some());
    p.int("TOA_RESIDUAL_COUNT", solution.residuals.len() as i128);
    p.int(
        "TOA_INFLUENCE_COUNT",
        solution.per_sensor_influence.len() as i128,
    );
    let q = &solution.geometry_quality;
    p.variant("TOA_TIER", "SIDEREON_OBSERVABILITY_TIER_", &q.tier);
    p.int("TOA_REDUNDANCY", i128::from(q.redundancy));
    p.int("TOA_RANK", q.rank as i128);
    p.bool("TOA_RAIM_CHECKABLE", q.raim_checkable);
    p.bool("TOA_COVARIANCE_VALIDATED", q.covariance_validated);
    p.f64("TOA_CONDITION_NUMBER_BITS", q.condition_number);
    p.f64("TOA_GDOP_BITS", q.gdop);
    let covariance = solution.covariance.as_ref().expect("covariance");
    p.int(
        "TOA_COVARIANCE_DIMENSION",
        covariance.position_m2.len() as i128,
    );
    p.int(
        "TOA_COVARIANCE_STATE_DIMENSION",
        covariance.state.len() as i128,
    );
    let residual_s: Vec<f64> = solution.residuals.iter().map(|r| r.residual_s).collect();
    p.f64s("TOA_RESIDUALS_S_BITS", &residual_s);
    let residual_index: Vec<i128> = solution
        .residuals
        .iter()
        .map(|r| r.sensor_index as i128)
        .collect();
    p.ints("TOA_RESIDUAL_SENSOR_INDEX", "size_t", &residual_index);
    let residual_has_ref: Vec<i128> = solution
        .residuals
        .iter()
        .map(|r| i128::from(r.reference_sensor_index.is_some()))
        .collect();
    p.ints("TOA_RESIDUAL_HAS_REFERENCE", "bool", &residual_has_ref);
    let influence_index: Vec<i128> = solution
        .per_sensor_influence
        .iter()
        .map(|i| i.sensor_index as i128)
        .collect();
    p.ints("TOA_INFLUENCE_SENSOR_INDEX", "size_t", &influence_index);
    let scores: Vec<f64> = solution
        .per_sensor_influence
        .iter()
        .map(|i| i.score)
        .collect();
    p.f64s("TOA_INFLUENCE_SCORE_BITS", &scores);

    // TDOA.
    let tdoa_positions: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0],
        vec![1000.0, 0.0],
        vec![0.0, 800.0],
        vec![900.0, 900.0],
    ];
    let tdoa_sensors: Vec<Sensor> = tdoa_positions.iter().cloned().map(Sensor::new).collect();
    let tdoa_times = arrivals(&tdoa_positions, &[300.0, 260.0], 4.0, 340.0);
    options.mode = SourceSolveMode::Tdoa {
        reference_sensor: 0,
    };
    let tdoa = locate_source(&tdoa_sensors, &tdoa_times, 340.0, &options).expect("tdoa");
    p.int("TDOA_DIMENSION", tdoa.position_m.len() as i128);
    p.int("TDOA_RESIDUAL_COUNT", tdoa.residuals.len() as i128);
    p.f64s("TDOA_POSITION_M_BITS", &tdoa.position_m);
    p.f64("TDOA_ORIGIN_TIME_S_BITS", tdoa.origin_time_s.unwrap_or(0.0));

    // DOP and CRLB.
    let dop_sensors: Vec<Sensor> = vec![
        Sensor::new(vec![100.0, 0.0]),
        Sensor::new(vec![-100.0, 0.0]),
        Sensor::new(vec![0.0, 100.0]),
        Sensor::new(vec![0.0, -100.0]),
    ];
    let dop = source_dop(&dop_sensors, &[0.0, 0.0], 10.0).expect("source dop");
    p.f64("DOP_GDOP_BITS", dop.gdop);
    p.f64("DOP_PDOP_BITS", dop.pdop);
    p.f64("DOP_HDOP_BITS", dop.hdop);
    p.f64("DOP_VDOP_BITS", dop.vdop);
    p.f64("DOP_TDOP_BITS", dop.tdop);
    let crlb = source_crlb(&dop_sensors, &[0.0, 0.0], 10.0, 0.01).expect("source crlb");
    p.f64(
        "CRLB_POSITION_M2_00_BITS",
        crlb.covariance.position_m2[0][0],
    );
    p.f64(
        "CRLB_POSITION_M2_11_BITS",
        crlb.covariance.position_m2[1][1],
    );
    p.bool(
        "CRLB_HAS_ORIGIN_TIME_S2",
        crlb.covariance.origin_time_s2.is_some(),
    );
    p.f64(
        "CRLB_ORIGIN_TIME_S2_BITS",
        crlb.covariance.origin_time_s2.unwrap_or(0.0),
    );
    let singular = source_dop(
        &[
            Sensor::new(vec![0.0, 0.0]),
            Sensor::new(vec![100.0, 0.0]),
            Sensor::new(vec![200.0, 0.0]),
            Sensor::new(vec![300.0, 0.0]),
        ],
        &[50.0, 0.0],
        300.0,
    );
    p.outcome("SINGULAR_DOP", &singular);
    p.str(
        "SINGULAR_DOP_ERROR_TEXT",
        &singular
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default(),
    );

    header_end(GUARD);
}
