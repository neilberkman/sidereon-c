use sidereon_core::astro::attitude::gnss::{
    gnss_attitude, AttitudeMode, AttitudeTrajectory, GnssAttitudeLaw, GnssBlock,
    OrbitAttitudeTrajectory,
};
use sidereon_core::astro::de::DeEphemeris;
use sidereon_core::astro::frames::{
    Iers2010EarthOrientationProvider, Iers2010EopSample, TdbEarthOrientationProvider,
};
use sidereon_core::astro::propagator::analytic::{
    BrouwerLyddanePropagator, CircularElements, EcksteinHechlerPropagator, KeplerianElements,
    ZonalHarmonics,
};
use sidereon_core::astro::propagator::{
    ForceModelKind, IntegratorKind, IntegratorOptions, StatePropagator,
};
use sidereon_core::astro::time::{j2000_seconds, tdb_from_tt_j2000_seconds, Instant, TimeScale};
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::orbit_scoring::{
    BroadcastOrbit, OrbitTrajectory, PredictedSp3, TrajectorySample,
};
use valgen::{bits, core_fixtures, header_end, read, tests_path, CORE_REVISION};

const GUARD: &str = "SIDEREON_GNSS_ATTITUDE_PINS_H";

fn values(name: &str, values: impl IntoIterator<Item = f64>) {
    let pins: Vec<_> = values.into_iter().map(bits).collect();
    println!(
        "static const uint64_t ATTITUDE_{name}[{}] = {{ {} }};",
        pins.len(),
        pins.join(", ")
    );
}

fn evaluate(name: &str, trajectory: OrbitAttitudeTrajectory<'_>, t_s: f64) {
    evaluate_law(
        name,
        &trajectory,
        t_s,
        &GnssAttitudeLaw::OrbitNormal,
        AttitudeMode::OrbitNormal,
    );
}

fn evaluate_law(
    name: &str,
    trajectory: &OrbitAttitudeTrajectory<'_>,
    t_s: f64,
    law: &GnssAttitudeLaw,
    mode: AttitudeMode,
) {
    let sample = trajectory.sample(t_s).unwrap();
    let result = gnss_attitude(law, trajectory, t_s).unwrap();
    assert_eq!(result.mode, mode, "{name}");
    let dyb = result.dyb.unwrap();
    values(
        name,
        [t_s]
            .into_iter()
            .chain(sample.position_m)
            .chain(sample.velocity_m_s)
            .chain(sample.sun_position_m)
            .chain([
                result.yaw_rad,
                result.nominal_yaw_rad,
                result.beta_rad,
                result.orbit_angle_rad,
            ])
            .chain(result.body_x)
            .chain(result.body_y)
            .chain(result.body_z)
            .chain(dyb.d)
            .chain(dyb.y)
            .chain(dyb.b),
    );
}

fn main() {
    println!("/* sidereon-core revision {} */", CORE_REVISION.trim());
    println!("#ifndef {GUARD}\n#define {GUARD}\n#include <stdint.h>\n");

    let sp3 = Sp3::parse(
        &std::fs::read(tests_path(
            "fixtures/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3",
        ))
        .unwrap(),
    )
    .unwrap();
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .unwrap();
    let orientation = TdbEarthOrientationProvider::new();
    let satellite = "G03".parse().unwrap();
    let sp3_gps = sp3.epochs_j2000_seconds()[0] + 1200.0;
    let sp3_tdb = tdb_from_tt_j2000_seconds(sp3_gps + 51.184);
    let nav_gps = j2000_seconds(2020, 6, 25, 18, 10, 0.25);
    let nav_tdb = tdb_from_tt_j2000_seconds(nav_gps + 51.184);
    let origin = Instant::from_nanos(TimeScale::Gpst, 0);
    let eop = Iers2010EopSample {
        mjd_utc: 59025,
        xp_arcsec: 0.1,
        yp_arcsec: 0.2,
        ut1_minus_utc_s: -0.2,
        lod_s: 0.001,
        dx_arcsec: 0.0001,
        dy_arcsec: -0.0001,
        pole_rates_rad_s: Some([0.0, 0.0]),
    };
    let iers = Iers2010EarthOrientationProvider::new(vec![
        eop,
        Iers2010EopSample {
            mjd_utc: 59026,
            ..eop
        },
    ])
    .unwrap();
    evaluate(
        "SP3",
        OrbitAttitudeTrajectory::from_sp3(&sp3, satellite, &iers)
            .with_velocity_half_step_s(10.0)
            .unwrap(),
        sp3_tdb,
    );
    evaluate(
        "NAV",
        OrbitAttitudeTrajectory::from_broadcast(&nav, satellite, &orientation),
        nav_tdb,
    );
    let broadcast = BroadcastOrbit::new(&nav, origin).unwrap();
    assert_eq!(
        OrbitAttitudeTrajectory::from_broadcast(&nav, satellite, &orientation).sample(nav_tdb),
        OrbitAttitudeTrajectory::from_orbit_source(&broadcast, satellite, &orientation)
            .sample(nav_tdb),
    );
    let predicted = PredictedSp3::with_cutoff(&sp3, origin).unwrap();
    evaluate(
        "PREDICTED",
        OrbitAttitudeTrajectory::from_orbit_source(&predicted, satellite, &orientation),
        sp3_tdb,
    );
    let nodes = [(-1, 0.0), (0, 100.0), (1, 200.0)].map(|(offset, y)| TrajectorySample {
        satellite,
        epoch: Instant::from_nanos(TimeScale::Gpst, (646_315_200_i128 + offset) * 1_000_000_000),
        position_ecef_m: [26_000_000.0, y, 1_000_000.0],
        clock_s: None,
    });
    let sampled = OrbitTrajectory::new("IGS14", origin, nodes).unwrap();
    evaluate(
        "SAMPLED",
        OrbitAttitudeTrajectory::from_orbit_source(&sampled, satellite, &orientation)
            .with_velocity_half_step_s(1.0)
            .unwrap(),
        tdb_from_tt_j2000_seconds(646_315_200.0 + 51.184),
    );

    let epoch = 646_315_200.25;
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
    evaluate(
        "EH",
        OrbitAttitudeTrajectory::from_eckstein_hechler(&eh),
        epoch + 600.0,
    );
    let eh_trajectory = OrbitAttitudeTrajectory::from_eckstein_hechler(&eh);
    let query = epoch + 600.0;
    let nominal_law = GnssAttitudeLaw::for_block(GnssBlock::GpsIIR, None).unwrap();
    let nominal = gnss_attitude(&nominal_law, &eh_trajectory, query).unwrap();
    for (block, prn) in [
        (GnssBlock::GpsIIA, Some(1)),
        (GnssBlock::GpsIIF, None),
        (GnssBlock::GlonassM, None),
        (GnssBlock::GalileoIov, None),
        (GnssBlock::GalileoFoc, None),
        (GnssBlock::Beidou2Meo, None),
        (GnssBlock::Beidou3Meo { manufacturer: None }, None),
    ] {
        let law = GnssAttitudeLaw::for_block(block, prn).unwrap();
        assert_eq!(gnss_attitude(&law, &eh_trajectory, query).unwrap(), nominal);
    }
    evaluate_law(
        "NOMINAL",
        &eh_trajectory,
        query,
        &nominal_law,
        AttitudeMode::Nominal,
    );
    let fixed = GnssAttitudeLaw::FixedBetaYawSteering { beta0_rad: 1.5 };
    evaluate_law(
        "FIXED_BETA",
        &eh_trajectory,
        query,
        &fixed,
        AttitudeMode::ModifiedYawSteering,
    );

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
    evaluate(
        "BL",
        OrbitAttitudeTrajectory::from_brouwer_lyddane(&bl),
        epoch + 600.0,
    );

    let mut options = IntegratorOptions::default();
    options.dense_output = true;
    let numerical = StatePropagator::new(
        epoch,
        [7000.0, 20.0, 30.0],
        [0.0, 7.5, 1.0],
        ForceModelKind::two_body(),
        IntegratorKind::Dp54,
    )
    .with_options(options);
    let result = numerical.propagate_to(epoch + 600.0).unwrap();
    let output = result.dense.as_ref().unwrap();
    let (start, end) = output.coverage_tdb_seconds().unwrap();
    values("DENSE_COVERAGE_FORWARD", [start, end]);
    let backward = numerical.propagate_to(epoch - 600.0).unwrap();
    let (start, end) = backward.dense.unwrap().coverage_tdb_seconds().unwrap();
    values("DENSE_COVERAGE_BACKWARD", [start, end]);
    let segment = &output.segments[0];
    let interior = segment.t_start + 0.375 * segment.h;
    assert!(output.segments.iter().all(|step| interior != step.t_end()));
    evaluate(
        "DENSE",
        OrbitAttitudeTrajectory::from_dense_output(output),
        interior,
    );
    values(
        "FINAL",
        [result.final_state.epoch_tdb_seconds]
            .into_iter()
            .chain(result.final_state.position_array())
            .chain(result.final_state.velocity_array()),
    );

    let de = DeEphemeris::load(format!("{}/bodies/de440/lnxp1990.440", core_fixtures())).unwrap();
    evaluate(
        "DE",
        OrbitAttitudeTrajectory::from_eckstein_hechler(&eh).with_de_ephemeris(&de),
        epoch + 600.0,
    );
    header_end(GUARD);
}
