use serde_json::{json, Value};
use sidereon_core::astro::constants::{MU_EARTH, RE_EARTH};
use sidereon_core::astro::forces::{
    EarthRadiationPressure, EmpiricalHarmonic, EmpiricalRtnAcceleration, RtnAxis,
    SolidEarthTideGravity, TideSystem, ZonalCoefficients, ZonalDegrees, ZonalGravity,
};
use sidereon_core::astro::frames::TdbEarthOrientationProvider;
use sidereon_core::astro::propagator::{
    ForceModelComponents, ForceModelKind, IntegratorKind, IntegratorOptions, StatePropagator,
};
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::model::{Instant, TimeScale};
use sidereon_core::orbit_determination::{
    fit_inertial_orbit_parameters, fit_sp3_ecef_orbit_parameters, InertialPositionSample,
    OrbitParameterCovariance, OrbitParameterFitError, OrbitParameterFitOptions,
};
use sidereon_core::orbit_scoring::{
    score_orbit, OrbitScoreOptions, PredictedSp3, SisreWeightSelection, SisreWeights,
};
use sidereon_core::{ephemeris::Sp3, GnssSatelliteId};
use valgen::{bits, core_fixtures, header_end, read_bytes, CORE_REVISION};

const GUARD: &str = "SIDEREON_ORBIT_ESTIMATES_PINS_H";

fn values(name: &str, values: impl IntoIterator<Item = f64>) {
    let values: Vec<_> = values.into_iter().map(bits).collect();
    println!(
        "static const uint64_t ESTIMATE_{name}[{}] = {{ {} }};",
        values.len(),
        values.join(", ")
    );
}

fn error_float(value: f64) -> Value {
    json!({"decimal": value.to_string(), "bits_hex": format!("{:016x}", value.to_bits())})
}

fn error_pin(name: &str, kind: &str, fields: Value) {
    let error = json!({"kind":kind, "fields":fields}).to_string();
    println!("#define ESTIMATE_ERROR_{name} {error:?}");
}

fn main() {
    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("#ifndef {GUARD}\n#define {GUARD}\n#include <stdint.h>\n");
    let initial = CartesianState::new(0.0, [7000.0, 20.0, 30.0], [0.0, 7.5, 1.0]);
    let mut integration_options = IntegratorOptions::default();
    integration_options.dense_output = false;
    let propagator = StatePropagator {
        initial,
        force_model: ForceModelKind::two_body(),
        integrator: IntegratorKind::Dp54,
        options: integration_options,
        drag: None,
        space_weather: None,
    };
    let seconds = [0, 60, 120, 180, 240, 300, 360, 420];
    let states = propagator.ephemeris(&seconds.map(f64::from)).unwrap();
    let samples: Vec<_> = states
        .iter()
        .zip(seconds)
        .map(|(state, seconds)| InertialPositionSample {
            epoch: Instant::from_nanos(TimeScale::Tdb, i128::from(seconds) * 1_000_000_000),
            position_gcrf_m: state.position_array().map(|km| km * 1000.0),
        })
        .collect();
    values(
        "OBSERVATIONS",
        samples.iter().flat_map(|sample| sample.position_gcrf_m),
    );
    let mut options = OrbitParameterFitOptions::default();
    options.force_model = ForceModelKind::two_body();
    options.integrator_options = integration_options;
    let satellite: GnssSatelliteId = "G03".parse().unwrap();
    let fit = fit_inertial_orbit_parameters(&samples, satellite, None, &options).unwrap();
    values(
        "FIT",
        [
            fit.residual_stats.rms_3d_m,
            fit.seed_rms_3d_m,
            fit.condition_number,
            fit.sigma("x_km").unwrap(),
            fit.correlation("x_km", "y_km").unwrap(),
        ],
    );
    println!("#define ESTIMATE_ITERATIONS {}", fit.iterations);
    values(
        "FIT_INITIAL",
        fit.initial_state
            .position_array()
            .into_iter()
            .chain(fit.initial_state.velocity_array())
            .chain([fit.initial_state.epoch_tdb_seconds]),
    );

    let OrbitParameterCovariance::Estimated {
        matrix,
        correlation,
        variance_factor,
        degrees_of_freedom,
    } = &fit.covariance
    else {
        panic!("covariance")
    };
    values("COVARIANCE", (0..36).map(|i| matrix[(i / 6, i % 6)]));
    values("CORRELATION", (0..36).map(|i| correlation[(i / 6, i % 6)]));
    values(
        "FIT_STATS",
        [
            *variance_factor,
            *degrees_of_freedom as f64,
            fit.residual_stats.radial_rms_m,
            fit.residual_stats.along_rms_m,
            fit.residual_stats.cross_rms_m,
        ],
    );
    values(
        "RESIDUALS",
        fit.residuals.iter().flat_map(|row| {
            [
                row.epoch_tdb_j2000_s,
                row.elapsed_tt_s,
                row.radial_m,
                row.along_m,
                row.cross_m,
            ]
        }),
    );

    let mut refused = options.clone();
    refused.max_condition_number = 1.5;
    let error = fit_inertial_orbit_parameters(&samples, satellite, None, &refused).unwrap_err();
    let OrbitParameterFitError::IllConditionedNormalEquations {
        satellite: sat,
        condition_number,
        limit,
        weakest,
    } = error
    else {
        panic!("{error:?}")
    };
    let weakest: Vec<_> = weakest
        .iter()
        .map(|(name, value)| json!([name, error_float(*value)]))
        .collect();
    error_pin(
        "ILL_CONDITIONED",
        "ill_conditioned_normal_equations",
        json!({
            "satellite": sat.to_string(), "condition_number": error_float(condition_number),
            "limit": error_float(limit), "weakest": weakest,
        }),
    );

    refused = options.clone();
    refused.max_iterations = 1;
    let error = fit_inertial_orbit_parameters(&samples, satellite, None, &refused).unwrap_err();
    let OrbitParameterFitError::DidNotConverge {
        satellite: sat,
        iterations,
        last_step_sigmas,
    } = error
    else {
        panic!("{error:?}")
    };
    error_pin(
        "DID_NOT_CONVERGE",
        "did_not_converge",
        json!({
            "satellite": sat.to_string(), "iterations": iterations, "last_step_sigmas": error_float(last_step_sigmas),
        }),
    );

    let mut radiation_options = options.clone();
    radiation_options.force_model = ForceModelKind::composite(
        ForceModelComponents::earth_two_body().with_earth_radiation_pressure(
            EarthRadiationPressure::new(1.25, 0.02)
                .unwrap()
                .with_moon_eclipse(false),
        ),
    );
    let radiation_fit =
        fit_inertial_orbit_parameters(&samples, satellite, None, &radiation_options).unwrap();
    values(
        "RADIATION_FIT",
        [radiation_fit.residual_stats.rms_3d_m]
            .into_iter()
            .chain(radiation_fit.initial_state.position_array())
            .chain(radiation_fit.initial_state.velocity_array()),
    );
    let mut force_options = options.clone();
    let empirical = EmpiricalRtnAcceleration::with_zero_terms(&[(
        RtnAxis::Radial,
        EmpiricalHarmonic::Constant,
    )]);
    force_options.additional_forces = vec![std::sync::Arc::new(empirical)];
    force_options.estimated_parameters = vec!["empirical.radial.constant".to_owned()];
    let fit_with_force =
        fit_inertial_orbit_parameters(&samples, satellite, None, &force_options).unwrap();
    let fitted_parameter = fit_with_force
        .force_parameter("empirical.radial.constant")
        .unwrap();
    values(
        "FITTED_FORCE",
        [
            fitted_parameter.initial_value,
            fitted_parameter.value,
            fit_with_force.sigma(&fitted_parameter.name).unwrap(),
        ],
    );

    let mut tide_options = options.clone();
    tide_options.force_model = ForceModelKind::composite(
        ForceModelComponents::earth_two_body()
            .with_zonal(ZonalGravity::new(
                MU_EARTH,
                RE_EARTH,
                ZonalDegrees::through(3).unwrap(),
                ZonalCoefficients {
                    tide_system: TideSystem::ZeroTide,
                    ..ZonalCoefficients::default()
                },
            ))
            .with_solid_earth_tide(SolidEarthTideGravity {
                tide_system: TideSystem::ZeroTide,
                ..SolidEarthTideGravity::default()
            }),
    );
    tide_options.propagation_context = tide_options
        .propagation_context
        .with_body_fixed_frame_provider(std::sync::Arc::new(TdbEarthOrientationProvider::new()));
    let tide_fit = fit_inertial_orbit_parameters(&samples, satellite, None, &tide_options).unwrap();
    values(
        "ZERO_TIDE_FIT",
        [tide_fit.residual_stats.rms_3d_m]
            .into_iter()
            .chain(tide_fit.initial_state.position_array())
            .chain(tide_fit.initial_state.velocity_array()),
    );

    let product = Sp3::parse(&read_bytes(&format!(
        "{}/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3",
        core_fixtures(),
    )))
    .unwrap();
    let earth_fixed_fit = fit_sp3_ecef_orbit_parameters(
        &product,
        satellite,
        &TdbEarthOrientationProvider::new(),
        None,
        &options,
    )
    .unwrap();
    values(
        "SP3_FIT",
        [
            earth_fixed_fit.residual_stats.rms_3d_m,
            earth_fixed_fit.condition_number,
            earth_fixed_fit.initial_state.epoch_tdb_seconds,
        ]
        .into_iter()
        .chain(earth_fixed_fit.initial_state.position_array())
        .chain(earth_fixed_fit.initial_state.velocity_array()),
    );
    let origin = product.epochs[0];
    let source = PredictedSp3::with_cutoff(&product, origin).unwrap();
    let mut options = OrbitScoreOptions::new(vec![0.0, 1800.0, 7200.0]);
    options.sisre_weights = SisreWeightSelection::Fixed(SisreWeights::new(1.0, 0.02).unwrap());
    let report = score_orbit(&source, &product, &[satellite], &options).unwrap();
    println!("#define ESTIMATE_SCORED {}", report.coverage.scored);
    println!("#define ESTIMATE_EPOCHS {}", report.coverage.epochs);
    println!(
        "#define ESTIMATE_BIN0_COUNT {}",
        report.per_satellite[0].bins[0].count
    );
    println!(
        "#define ESTIMATE_BIN1_COUNT {}",
        report.per_satellite[0].bins[1].count
    );
    header_end(GUARD);
}
