use std::path::Path;

use nalgebra::Vector3;
use sidereon_core::astro::attitude::gnss;
use sidereon_core::astro::bodies::sun_moon_gcrs_at_tdb_seconds;
use sidereon_core::astro::de::DeEphemeris;
use sidereon_core::astro::events::eclipse::{self, EarthShadowModel};
use sidereon_core::astro::forces::{
    ecom_geometry, Ecom1Coefficients, Ecom1SolarRadiationPressure, ForceModel, IcgemGravityField,
    SolarRadiationPressure,
};
use sidereon_core::astro::frames::{Iers2010EarthOrientationProvider, Iers2010EopSample};
use sidereon_core::astro::propagator::analytic::{
    BrouwerLyddanePropagator, CircularElements, EcksteinHechlerPropagator, KeplerianElements,
    ZonalHarmonics,
};
use sidereon_core::astro::propagator::PropagationContext;
use sidereon_core::astro::state::CartesianState;
use sidereon_core::astro::time::scales::{
    tdb_from_tt_j2000_seconds, tdb_minus_tt_seconds, tt_from_tdb_j2000_seconds,
};
use valgen::{bits, core_fixtures, header_end, read, CORE_REVISION};

const GUARD: &str = "SIDEREON_ORBIT_MODELS_PINS_H";

fn values(name: &str, values: impl IntoIterator<Item = f64>) {
    let values: Vec<_> = values.into_iter().map(bits).collect();
    println!(
        "static const uint64_t ORBIT_{name}[{}] = {{ {} }};",
        values.len(),
        values.join(", ")
    );
}

fn main() {
    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("#ifndef {GUARD}\n#define {GUARD}\n#include <stdint.h>\n");

    let epoch = 646_315_200.25;
    values(
        "TIME",
        [
            tdb_minus_tt_seconds(epoch),
            tdb_from_tt_j2000_seconds(epoch),
            tt_from_tdb_j2000_seconds(epoch),
        ],
    );
    let bodies = sun_moon_gcrs_at_tdb_seconds(epoch).unwrap();
    values("SUN", bodies.sun);
    values("MOON", bodies.moon);

    let position = [7000.0, 20.0, 30.0];
    let velocity = [0.0, 7.5, 1.0];
    let sun = [150_000_000.0, 20_000_000.0, -10_000_000.0];
    let moon = [-100_000.0, 200_000.0, 300_000.0];
    let state = CartesianState::new(epoch, position, velocity);
    let ctx = PropagationContext::default();
    let srp = SolarRadiationPressure::new(1.25, 0.02).unwrap();
    let partials = srp.acceleration_partials(&[1.25], &state, &ctx).unwrap();
    values(
        "SRP",
        srp.acceleration(&state, &ctx).unwrap().iter().copied(),
    );
    values(
        "SRP_PARTIAL",
        (0..9).map(|i| partials.wrt_position[(i / 3, i % 3)]),
    );
    let ecom = Ecom1SolarRadiationPressure::new(Ecom1Coefficients {
        d0: 1.0e-7,
        ..Ecom1Coefficients::default()
    })
    .unwrap();
    values(
        "ECOM",
        ecom.acceleration_with_sun_moon(position.into(), velocity.into(), sun.into(), moon.into())
            .unwrap()
            .iter()
            .copied(),
    );
    let geometry = ecom_geometry(position.into(), velocity.into(), sun.into()).unwrap();
    values("ECOM_D", geometry.dyb.d);
    let y = gnss::nominal_yaw(position, velocity, sun).unwrap();
    values("YAW", [y.yaw_rad, y.beta_rad]);
    values("BODY_X", y.body_x);
    values("BODY_Y", y.body_y);
    values("BODY_Z", y.body_z);
    let dyb = gnss::dyb_axes(position, sun).unwrap();
    values("DYB_D", dyb.d);
    values("DYB_Y", dyb.y);
    values("DYB_B", dyb.b);

    let eclipse = eclipse::shadow_fraction_with_model_and_gradient(
        [-7000.0, 6378.0, 0.0],
        [150_000_000.0, 0.0, 0.0],
        EarthShadowModel::Wgs84Oblate,
    )
    .unwrap();
    values("ECLIPSE", [eclipse.0].into_iter().chain(eclipse.1));

    let gravity = ZonalHarmonics::earth();
    let eh = EcksteinHechlerPropagator::from_mean_elements(
        epoch,
        CircularElements {
            a_km: 7200.0,
            ex: 0.001,
            ey: 0.002,
            i_rad: 0.5,
            raan_rad: 0.25,
            mean_latitude_argument_rad: 0.125,
        },
        &gravity,
    )
    .unwrap();
    let bl = BrouwerLyddanePropagator::from_mean_elements(
        epoch,
        KeplerianElements {
            a_km: 7200.0,
            e: 0.01,
            i_rad: 0.5,
            raan_rad: 0.25,
            arg_perigee_rad: 0.75,
            mean_anomaly_rad: 0.125,
        },
        &gravity,
        0.0,
    )
    .unwrap();
    for (name, state) in [
        ("ECKSTEIN_HECHLER", eh.propagate(epoch + 600.0).unwrap()),
        ("BROUWER_LYDDANE", bl.propagate(epoch + 600.0).unwrap()),
    ] {
        values(
            name,
            state
                .position_array()
                .into_iter()
                .chain(state.velocity_array()),
        );
    }

    let sample = Iers2010EopSample {
        mjd_utc: 59025,
        xp_arcsec: 0.1,
        yp_arcsec: 0.2,
        ut1_minus_utc_s: -0.2,
        lod_s: 0.001,
        dx_arcsec: 0.0001,
        dy_arcsec: -0.0001,
        pole_rates_rad_s: Some([0.0, 0.0]),
    };
    let provider = Iers2010EarthOrientationProvider::new(vec![
        sample,
        Iers2010EopSample {
            mjd_utc: 59026,
            ..sample
        },
    ])
    .unwrap();
    let orientation = provider.orientation_at_utc_mjd(59025, 43200.0).unwrap();
    values("QUATERNION", orientation.iers2010_quaternion().unwrap());

    let root = core_fixtures();
    let root = Path::new(&root);
    let de = DeEphemeris::load(root.join("bodies/de440/lnxp1990.440")).unwrap();
    let (start, end) = de.coverage_tdb_seconds();
    values("DE_COVERAGE", [start, end]);
    let de_state = de.sun_moon_at_tdb_seconds(start + 3600.0).unwrap();
    values(
        "DE_SUN",
        de_state
            .sun
            .position_m
            .into_iter()
            .chain(de_state.sun.velocity_m_s),
    );

    let field = IcgemGravityField::parse(&read(
        root.join("icgem/synthetic_icgem2.gfc").to_str().unwrap(),
    ))
    .unwrap();
    let gravity = field.gravity(2, 2).unwrap();
    let field_position: Vector3<f64> = position.into();
    values(
        "ICGEM_POTENTIAL",
        [gravity
            .body_fixed_potential_at_km2_s2(field_position.into(), 86400.0)
            .unwrap()],
    );
    header_end(GUARD);
}
