use std::sync::Arc;

use sidereon_core::astro::bodies::{sun_moon_gcrs, sun_moon_gcrs_at};
use sidereon_core::astro::de::{DeBody, DeEphemeris};
use sidereon_core::astro::events::eclipse::EarthShadowModel;
use sidereon_core::astro::forces::{
    ecom_geometry_with_shadow, AccelerationPartials, EarthRadiationPressure, Ecom1Coefficients,
    Ecom1SolarRadiationPressure, Ecom2D4b1Coefficients, Ecom2SolarRadiationPressure,
    EmpiricalHarmonic, EmpiricalRtnAcceleration, ForceModel, IcgemGravityField, J2Gravity, RtnAxis,
    SolarRadiationPressure, SpaceWeatherSource,
};
use sidereon_core::astro::forces::{DragForce, SpaceWeather};
use sidereon_core::astro::frames::transforms::{
    gcrs_to_mean_of_date_matrix, mean_of_date_to_gcrs_matrix,
};
use sidereon_core::astro::frames::{
    EarthOrientationProvider, Iers2010EarthOrientationProvider, Iers2010EopSample,
};
use sidereon_core::astro::propagator::analytic::{
    self, BrouwerLyddanePropagator, CircularElements, EcksteinHechlerPropagator, KeplerianElements,
    MeanConversionOptions, ZonalHarmonics,
};
use sidereon_core::astro::propagator::{
    estimate_decay, DecayConfig, DragParameters, ForceModelComponents, ForceModelKind,
    IntegratorKind, PropagationContext, StatePropagator,
};
use sidereon_core::astro::space_weather::parse_csv;
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::TimeScales;
use valgen::{bits, core_fixtures, header_end, read, CORE_REVISION};

const GUARD: &str = "SIDEREON_ORBIT_CONTRACTS_PINS_H";

fn values(name: &str, values: impl IntoIterator<Item = f64>) {
    let pins: Vec<_> = values.into_iter().map(bits).collect();
    println!(
        "static const uint64_t CONTRACT_{name}[] = {{ {} }};",
        pins.join(", ")
    );
}

fn circular(elements: CircularElements) -> [f64; 6] {
    [
        elements.a_km,
        elements.ex,
        elements.ey,
        elements.i_rad,
        elements.raan_rad,
        elements.mean_latitude_argument_rad,
    ]
}

fn keplerian(elements: KeplerianElements) -> [f64; 6] {
    [
        elements.a_km,
        elements.e,
        elements.i_rad,
        elements.raan_rad,
        elements.arg_perigee_rad,
        elements.mean_anomaly_rad,
    ]
}

fn state(value: CartesianState) -> impl Iterator<Item = f64> {
    [value.epoch_tdb_seconds]
        .into_iter()
        .chain(value.position_array())
        .chain(value.velocity_array())
}

fn analytic() {
    let epoch = 646_315_200.25;
    let initial = CartesianState::new(epoch, [7000.0, 20.0, 30.0], [0.0, 7.5, 1.0]);
    let gravity = ZonalHarmonics::earth();
    let eh_options = MeanConversionOptions::ECKSTEIN_HECHLER_DEFAULT;
    let bl_options = MeanConversionOptions::BROUWER_LYDDANE_DEFAULT;
    values(
        "CIRCULAR",
        circular(
            CircularElements::from_cartesian(
                initial.position_array(),
                initial.velocity_array(),
                gravity.mu_km3_s2,
            )
            .unwrap(),
        ),
    );
    let eh_mean = analytic::eckstein_hechler_mean_elements(&initial, &gravity, eh_options).unwrap();
    let bl_mean = analytic::brouwer_lyddane_mean_elements(&initial, &gravity, bl_options).unwrap();
    values(
        "EH_MEAN",
        circular(eh_mean.elements)
            .into_iter()
            .chain([f64::from(eh_mean.iterations)]),
    );
    values(
        "BL_MEAN",
        keplerian(bl_mean.elements)
            .into_iter()
            .chain([f64::from(bl_mean.iterations)]),
    );
    values(
        "EH_OSCULATING",
        circular(
            analytic::eckstein_hechler_osculating_elements(&eh_mean.elements, &gravity).unwrap(),
        ),
    );
    values(
        "BL_OSCULATING",
        keplerian(
            analytic::brouwer_lyddane_osculating_elements(&bl_mean.elements, &gravity).unwrap(),
        ),
    );

    let eh = EcksteinHechlerPropagator::new(&initial, &gravity, eh_options).unwrap();
    let bl = BrouwerLyddanePropagator::new(&initial, &gravity, 0.0, bl_options).unwrap();
    let epochs = [epoch + 120.0, epoch - 60.0];
    values(
        "EH_EPHEMERIS",
        eh.ephemeris(&epochs).unwrap().into_iter().flat_map(state),
    );
    values(
        "BL_EPHEMERIS",
        bl.ephemeris(&epochs).unwrap().into_iter().flat_map(state),
    );
    values(
        "EH_AT_EPOCH",
        circular(eh.osculating_elements(epochs[0]).unwrap()),
    );
    values(
        "BL_AT_EPOCH",
        keplerian(bl.osculating_elements(epochs[0]).unwrap()),
    );
}

fn de() {
    let path = format!("{}/bodies/de440/lnxp1990.440", core_fixtures());
    let ephemeris = DeEphemeris::load(path).unwrap();
    let (name, &value) = ephemeris.constants().first_key_value().unwrap();
    println!(
        "#define CONTRACT_DE_RECORD_SIZE {}",
        ephemeris.record_size()
    );
    println!(
        "#define CONTRACT_DE_CONSTANT_COUNT {}",
        ephemeris.constants().len()
    );
    println!("#define CONTRACT_DE_FIRST_CONSTANT \"{name}\"");
    values(
        "DE_METADATA",
        [
            ephemeris.astronomical_unit_m(),
            ephemeris.earth_moon_mass_ratio(),
            ephemeris.gm_m3_s2(DeBody::Earth).unwrap(),
            value,
        ],
    );
    let (start, _) = ephemeris.coverage_tdb_seconds();
    let epoch = start + 3600.0;
    let bodies = ephemeris.sun_moon_at_tdb_seconds(epoch).unwrap();
    values(
        "DE_STATES",
        [epoch]
            .into_iter()
            .chain(bodies.sun.position_m)
            .chain(bodies.sun.velocity_m_s)
            .chain(bodies.moon.position_m)
            .chain(bodies.moon.velocity_m_s),
    );
    let initial = [7000.0, 20.0, 30.0];
    let velocity = [0.0, 7.5, 1.0];
    let epoch = 646_315_800.25;
    let earth = EarthRadiationPressure::new(1.25, 0.02)
        .unwrap()
        .with_moon_eclipse(false);
    let propagator = StatePropagator::new(
        epoch,
        initial,
        velocity,
        ForceModelKind::composite(
            ForceModelComponents::earth_two_body().with_earth_radiation_pressure(earth),
        ),
        IntegratorKind::Dp54,
    );
    let context = PropagationContext::new().with_de_ephemeris(std::sync::Arc::new(ephemeris));
    let states = propagator
        .ephemeris_with_context(&[epoch + 60.0], &context)
        .unwrap();
    values("DE_PROPAGATION", state(states[0].clone()));
}

fn force_partial_values(name: &str, value: AccelerationPartials) {
    values(
        name,
        (0..9)
            .map(|i| value.wrt_position[(i / 3, i % 3)])
            .chain((0..9).map(|i| value.wrt_velocity[(i / 3, i % 3)])),
    );
}

fn forces() {
    let position = nalgebra::Vector3::new(7000.0, 20.0, 30.0);
    let velocity = nalgebra::Vector3::new(0.0, 7.5, 1.0);
    let sun = [150_000_000.0, 20_000_000.0, -10_000_000.0].into();
    let moon = [-100_000.0, 200_000.0, 300_000.0].into();
    let coefficients = [1e-8, 2e-8, 3e-8, 4e-8, 5e-8, 6e-8, 7e-8, 8e-8, 9e-8];
    let ecom1 =
        Ecom1SolarRadiationPressure::new(Ecom1Coefficients::from_array(coefficients)).unwrap();
    let ecom2 =
        Ecom2SolarRadiationPressure::d4b1(Ecom2D4b1Coefficients::from_array(coefficients)).unwrap();
    let solar = SolarRadiationPressure::new(1.25, 0.02).unwrap();
    let earth = EarthRadiationPressure::new(1.25, 0.02)
        .unwrap()
        .with_moon_eclipse(false);
    let state = CartesianState::new(646_315_200.25, position.into(), velocity.into());
    values(
        "SOLAR",
        solar
            .acceleration_with_sun_moon(position, sun, moon)
            .unwrap()
            .iter()
            .copied(),
    );
    values(
        "EARTH",
        earth
            .acceleration_with_sun_moon(&state, sun, moon)
            .unwrap()
            .iter()
            .copied(),
    );
    values(
        "ECOM1_SUN_MOON",
        ecom1
            .acceleration_with_sun_moon(position, velocity, sun, moon)
            .unwrap()
            .iter()
            .copied(),
    );
    values(
        "ECOM2_SUN_MOON",
        ecom2
            .acceleration_with_sun_moon(position, velocity, sun, moon)
            .unwrap()
            .iter()
            .copied(),
    );
    force_partial_values(
        "ECOM1_STATE_SUN_MOON",
        ecom1
            .acceleration_partials_with_sun_moon(position, velocity, sun, moon)
            .unwrap(),
    );
    force_partial_values(
        "ECOM2_STATE_SUN_MOON",
        ecom2
            .acceleration_partials_with_sun_moon(position, velocity, sun, moon)
            .unwrap(),
    );
    values(
        "ECOM1_PARAMETER_SUN_MOON",
        ecom1
            .parameter_partials_with_sun_moon(position, velocity, sun, moon)
            .unwrap()
            .into_iter()
            .flat_map(<[f64; 3]>::from),
    );
    values(
        "ECOM2_PARAMETER_SUN_MOON",
        ecom2
            .parameter_partials_with_sun_moon(position, velocity, sun, moon)
            .unwrap()
            .into_iter()
            .flat_map(<[f64; 3]>::from),
    );

    let ecom1 = ecom1.with_moon_eclipse(false);
    let ecom2 = ecom2.with_moon_eclipse(false);
    values(
        "ECOM1_SUN",
        ecom1
            .acceleration_with_sun(position, velocity, sun)
            .unwrap()
            .iter()
            .copied(),
    );
    values(
        "ECOM2_SUN",
        ecom2
            .acceleration_with_sun(position, velocity, sun)
            .unwrap()
            .iter()
            .copied(),
    );
    force_partial_values(
        "ECOM1_STATE_SUN",
        ecom1
            .acceleration_partials_with_sun(position, velocity, sun)
            .unwrap(),
    );
    force_partial_values(
        "ECOM2_STATE_SUN",
        ecom2
            .acceleration_partials_with_sun(position, velocity, sun)
            .unwrap(),
    );
    values(
        "ECOM1_PARAMETER_SUN",
        ecom1
            .parameter_partials_with_sun(position, velocity, sun)
            .unwrap()
            .into_iter()
            .flat_map(<[f64; 3]>::from),
    );
    values(
        "ECOM2_PARAMETER_SUN",
        ecom2
            .parameter_partials_with_sun(position, velocity, sun)
            .unwrap()
            .into_iter()
            .flat_map(<[f64; 3]>::from),
    );

    let geometry = ecom_geometry_with_shadow(
        position,
        velocity,
        sun,
        Some(moon),
        EarthShadowModel::Spherical,
    )
    .unwrap();
    values(
        "ECOM_GEOMETRY",
        geometry
            .dyb
            .d
            .into_iter()
            .chain(geometry.dyb.y)
            .chain(geometry.dyb.b)
            .chain([
                geometry.delta_u_rad.unwrap(),
                geometry.argument_of_latitude_rad.unwrap(),
                geometry.lighting_ratio,
            ]),
    );
}

fn frames_and_gravity() {
    let ts = TimeScales::from_utc(2020, 6, 25, 12, 0, 0.0).unwrap();
    values(
        "GCRS_TO_MOD",
        gcrs_to_mean_of_date_matrix(&ts)
            .unwrap()
            .into_iter()
            .flatten(),
    );
    values(
        "MOD_TO_GCRS",
        mean_of_date_to_gcrs_matrix(&ts)
            .unwrap()
            .into_iter()
            .flatten(),
    );
    let bodies = sun_moon_gcrs_at(&ts).unwrap();
    values("BODIES_AT", bodies.sun.into_iter().chain(bodies.moon));
    let bodies = sun_moon_gcrs(0.2).unwrap();
    values("BODIES", bodies.sun.into_iter().chain(bodies.moon));

    let sample = Iers2010EopSample {
        mjd_utc: 59025,
        xp_arcsec: 0.1,
        yp_arcsec: 0.2,
        ut1_minus_utc_s: -0.2,
        lod_s: 0.001,
        dx_arcsec: 0.0001,
        dy_arcsec: -0.0001,
        pole_rates_rad_s: Some([8e-14, -3e-14]),
    };
    let orientation = Iers2010EarthOrientationProvider::new(vec![
        sample,
        Iers2010EopSample {
            mjd_utc: 59026,
            ..sample
        },
    ])
    .unwrap()
    .orientation_at_tdb_seconds(646_315_800.25)
    .unwrap();
    let pole = orientation.polar_motion();
    values(
        "IERS_ANGLES",
        [pole.xp_rad, pole.yp_rad]
            .into_iter()
            .chain(orientation.earth_rotation_vector_itrf_rad_s())
            .chain(orientation.iers2010_quaternion().unwrap()),
    );
    values(
        "IERS_MATRICES",
        [
            orientation.gcrf_to_itrf_matrix(),
            orientation.itrf_to_gcrf_matrix(),
            orientation.gcrf_to_itrf_rotation_rate_matrix(),
            orientation.itrf_to_gcrf_rotation_rate_matrix(),
        ]
        .into_iter()
        .flatten()
        .flatten(),
    );

    let field = IcgemGravityField::parse(&read(&format!(
        "{}/icgem/synthetic_icgem2.gfc",
        core_fixtures()
    )))
    .unwrap();
    let gravity = field.gravity(2, 2).unwrap();
    values(
        "ICGEM_ACCELERATION",
        gravity
            .body_fixed_acceleration_at_km_s2([7000.0, 20.0, 30.0], 86400.0)
            .unwrap(),
    );

    let initial = CartesianState::new(646_315_800.25, [6400.0, 0.0, 0.0], [0.0, 7.9, 0.0]);
    let drag = DragParameters::from_bc_factor_m2_kg(
        0.01,
        SpaceWeather::default(),
        DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
    )
    .unwrap();
    let estimate = estimate_decay(initial, &DecayConfig::new(drag)).unwrap();
    values("DECAY_ALTITUDE", [estimate.reentry_altitude_km]);
}

fn propagation_with_forces() {
    let epoch = 120_700_800.0;
    let initial = CartesianState::new(epoch, [6778.0, 20.0, 30.0], [0.0, 7.5, 1.0]);
    let ctx = PropagationContext::default();
    let j2 = J2Gravity::default();
    let unread = CartesianState::new(f64::NAN, initial.position_array(), [f64::NAN; 3]);
    let partials = j2.acceleration_partials(&[], &unread, &ctx).unwrap();
    values(
        "J2",
        j2.acceleration(&unread, &ctx).unwrap().iter().copied(),
    );
    values(
        "J2_PARTIALS",
        (0..3).flat_map(|row| (0..3).map(move |column| partials.wrt_position[(row, column)])),
    );
    let extreme = CartesianState::new(f64::NAN, [1e80, -2e80, 3e80], [f64::NAN; 3]);
    values(
        "J2_EXTREME",
        j2.acceleration(&extreme, &ctx).unwrap().iter().copied(),
    );

    let mut propagator = StatePropagator::new(
        epoch,
        initial.position_array(),
        initial.velocity_array(),
        ForceModelKind::two_body_j2(),
        IntegratorKind::Dp54,
    )
    .with_drag(
        DragParameters::from_bc_factor_m2_kg(
            0.01,
            SpaceWeather::default(),
            DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
        )
        .unwrap(),
    );
    let forces: [Arc<dyn ForceModel>; 2] = [
        Arc::new(
            EmpiricalRtnAcceleration::default()
                .with_term(RtnAxis::AlongTrack, EmpiricalHarmonic::Constant, 2e-6)
                .unwrap(),
        ),
        Arc::new(
            EmpiricalRtnAcceleration::default()
                .with_term(RtnAxis::CrossTrack, EmpiricalHarmonic::Constant, -1e-6)
                .unwrap(),
        ),
    ];
    let epochs = [epoch + 60.0, epoch + 120.0];
    let fixed = propagator
        .ephemeris_with_additional_forces(&epochs, &forces, &ctx)
        .unwrap();
    values(
        "PROPAGATION_FIXED_WEATHER",
        fixed.into_iter().flat_map(state),
    );

    propagator.options.dense_output = true;
    let dense = propagator
        .propagate_to_with_additional_forces(epochs[1], &forces, &ctx)
        .unwrap();
    values("PROPAGATION_DENSE_FORCES", state(dense.final_state));

    let table = parse_csv(&read(&format!(
        "{}/space_weather/SW-All-20260702-trim.csv",
        core_fixtures()
    )))
    .unwrap();
    propagator.space_weather = Some(SpaceWeatherSource::Table(Arc::new(table.value)));
    let sourced = propagator
        .ephemeris_with_additional_forces(&epochs, &forces, &ctx)
        .unwrap();
    values(
        "PROPAGATION_TABLE_WEATHER",
        sourced.into_iter().flat_map(state),
    );
}

fn main() {
    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("#ifndef {GUARD}\n#define {GUARD}\n#include <stdint.h>\n");
    analytic();
    de();
    forces();
    frames_and_gravity();
    propagation_with_forces();
    header_end(GUARD);
}
