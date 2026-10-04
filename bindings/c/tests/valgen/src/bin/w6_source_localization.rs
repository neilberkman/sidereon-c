//! source_localization_1_1_smoke.c: the synthetic sensors and arrival times
//! the test solves (built here and emitted for it), and sidereon-core's
//! solutions and closed-form initial guess for them.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::source_localization::{
    closed_form_initial_guess, locate_source, locate_source_with, Sensor, SourceLocateConfig,
    SourceLocateOptions, SourceSolveMode,
};

const SPEED_M_S: f64 = 343.0;

/// The arrival time at each sensor: origin plus distance over speed, the
/// distance summed axis by axis as the test's distance_to_source does.
fn arrivals(sensors: &[Vec<f64>], source: &[f64], origin_s: f64) -> Vec<f64> {
    sensors
        .iter()
        .map(|sensor| {
            let mut squared = 0.0;
            for axis in 0..sensor.len() {
                let delta = source[axis] - sensor[axis];
                squared += delta * delta;
            }
            origin_s + squared.sqrt() / SPEED_M_S
        })
        .collect()
}

fn def_sensors(name: &str, sensors: &[Vec<f64>]) {
    let rows: Vec<String> = sensors
        .iter()
        .map(|sensor| {
            let mut padded = sensor.clone();
            padded.resize(3, 0.0);
            format!(
                "{{ {} }}",
                padded
                    .iter()
                    .map(|v| valgen::bits(*v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    println!(
        "static const uint64_t {name}[{}][3] = {{ {} }};",
        sensors.len(),
        rows.join(", ")
    );
}

fn main() {
    let guard = "SIDEREON_W6_SOURCE_LOCALIZATION_PINS_H";
    valgen::header_start("w6_source_localization", guard);

    // Five 3D sensors, a source at (320, 260, 180) m emitting at 12.5 s, and
    // fixed timing noise on each arrival.
    let sensors: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 0.0],
        vec![1200.0, 0.0, 0.0],
        vec![0.0, 900.0, 0.0],
        vec![0.0, 0.0, 700.0],
        vec![1100.0, 800.0, 600.0],
    ];
    let noise = [0.00031, -0.00022, 0.00017, -0.00008, 0.00041];
    let mut arrival_times = arrivals(&sensors, &[320.0, 260.0, 180.0], 12.5);
    for (arrival, noise) in arrival_times.iter_mut().zip(noise) {
        *arrival += noise;
    }
    comment("Inputs: sensor positions (padded to 3), arrivals, speed 343 m/s.");
    def_size("W6_SL_SENSOR_COUNT", sensors.len());
    def_sensors("W6_SL_SENSOR_POSITION_BITS", &sensors);
    def_bits_array("W6_SL_ARRIVAL_BITS", &arrival_times);
    def_bits("W6_SL_SPEED_BITS", SPEED_M_S);

    let core_sensors: Vec<Sensor> = sensors.iter().map(|s| Sensor::new(s.clone())).collect();
    let mut options = SourceLocateOptions::default();
    options.timing_sigma_s = 0.001;
    let legacy =
        locate_source(&core_sensors, &arrival_times, SPEED_M_S, &options).expect("locate_source");
    let mut config = SourceLocateConfig::default();
    config.options = options;
    config.include_influence = false;
    let lean = locate_source_with(&core_sensors, &arrival_times, SPEED_M_S, &config)
        .expect("locate_source_with");
    comment("locate_source (with influence) and locate_source_with (without).");
    def_size(
        "W6_SL_LEGACY_INFLUENCE_COUNT",
        legacy.per_sensor_influence.len(),
    );
    def_size(
        "W6_SL_LEAN_INFLUENCE_COUNT",
        lean.per_sensor_influence.len(),
    );
    let mut position = legacy.position_m.clone();
    position.resize(3, 0.0);
    def_bits_array("W6_SL_LEGACY_POSITION_BITS", &position);
    def_bits(
        "W6_SL_LEGACY_ORIGIN_BITS",
        legacy.origin_time_s.unwrap_or(0.0),
    );

    // Four 2D sensors, a source at (210, 170) m emitting at 2.75 s, no noise.
    let seed_sensors: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0],
        vec![700.0, 0.0],
        vec![0.0, 600.0],
        vec![650.0, 550.0],
    ];
    let seed_arrivals = arrivals(&seed_sensors, &[210.0, 170.0], 2.75);
    comment("Closed-form initializer inputs: 2D sensors and clean arrivals.");
    def_size("W6_SL_SEED_SENSOR_COUNT", seed_sensors.len());
    def_sensors("W6_SL_SEED_SENSOR_POSITION_BITS", &seed_sensors);
    def_bits_array("W6_SL_SEED_ARRIVAL_BITS", &seed_arrivals);
    let core_seed: Vec<Sensor> = seed_sensors
        .iter()
        .map(|s| Sensor::new(s.clone()))
        .collect();
    let guess =
        closed_form_initial_guess(&core_seed, &seed_arrivals, SPEED_M_S, SourceSolveMode::Toa)
            .expect("closed-form initial guess");
    comment("closed_form_initial_guess (TOA).");
    def_size("W6_SL_GUESS_DIMENSION", guess.position_m.len().min(3));
    let mut guess_position = guess.position_m.clone();
    guess_position.resize(3, 0.0);
    def_bits_array("W6_SL_GUESS_POSITION_BITS", &guess_position);
    def_bool("W6_SL_GUESS_HAS_ORIGIN", guess.origin_time_s.is_some());
    def_bits(
        "W6_SL_GUESS_ORIGIN_BITS",
        guess.origin_time_s.unwrap_or(0.0),
    );
    def_bits("W6_SL_GUESS_RESIDUAL_RMS_BITS", guess.residual_rms_s);
    valgen::header_end(guard);
}
