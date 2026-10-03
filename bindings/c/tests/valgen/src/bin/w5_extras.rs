//! tests/w5_extras_pins.h: sidereon-core's results for the inputs
//! extras_smoke.c passes.

#[path = "w5_support/pins.rs"]
mod pins;

use pins::Pins;
use sidereon_core::astro::time::{Instant, JulianDateSplit, TimeScale};
use sidereon_core::astro::{angles, bodies, conjunction, iod, lambert, rf};
use sidereon_core::carrier_phase::{self as cp, ArcEpoch, CycleSlipOptions};
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::frame::Wgs84Geodetic;
use sidereon_core::frequencies::{self as freq, CarrierBand};
use sidereon_core::observables::ObservableEphemerisSource;
use sidereon_core::quality::{self, RaimInput, RaimOptions, RaimWeights};
use sidereon_core::GnssSystem;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_W5_EXTRAS_PINS_H";

fn main() {
    header_start("w5_extras", GUARD);
    let p = Pins::new("W5_EXTRAS");

    // RF link budget, chained as test_rf chains it.
    let fspl = rf::fspl(36000.0, 1575.42).expect("fspl");
    let eirp = rf::eirp(40.0, 30.0).expect("eirp");
    p.comment("sidereon_core::astro::rf, chained as test_rf chains it.");
    p.f64("RF_FSPL_BITS", fspl);
    p.f64("RF_EIRP_BITS", eirp);
    p.f64("RF_CN0_BITS", rf::cn0(eirp, fspl, 5.0, 2.0).expect("cn0"));
    p.f64(
        "RF_LINK_MARGIN_BITS",
        rf::link_margin(&rf::LinkBudget {
            eirp_dbw: eirp,
            fspl_db: fspl,
            receiver_gt_dbk: 5.0,
            other_losses_db: 2.0,
            required_cn0_dbhz: 35.0,
        })
        .expect("link margin"),
    );
    p.f64(
        "RF_WAVELENGTH_BITS",
        rf::wavelength(1.57542e9).expect("wavelength"),
    );
    p.f64(
        "RF_DISH_GAIN_BITS",
        rf::dish_gain(2.4, 1.57542e9, 0.6).expect("dish gain"),
    );

    // Frequencies and combinations.
    let f1 = freq::frequency_hz(GnssSystem::Gps, CarrierBand::L1).expect("L1");
    let f2 = freq::frequency_hz(GnssSystem::Gps, CarrierBand::L2).expect("L2");
    p.comment("sidereon_core::frequencies and combinations for GPS L1/L2.");
    p.f64("GPS_L1_HZ_BITS", f1);
    p.f64("GPS_L2_HZ_BITS", f2);
    p.f64(
        "GPS_L1_WAVELENGTH_M_BITS",
        freq::wavelength_m(GnssSystem::Gps, CarrierBand::L1).expect("L1 wavelength"),
    );
    p.f64(
        "GLONASS_G1_CHANNEL_0_HZ_BITS",
        freq::glonass_g1_frequency_hz(0),
    );
    p.f64(
        "GPS_DEFAULT_SPP_HZ_BITS",
        freq::default_spp_frequency_hz(GnssSystem::Gps).expect("default SPP frequency"),
    );
    use sidereon_core::combinations as comb;
    p.f64("GAMMA_BITS", comb::gamma(f1, f2).expect("gamma"));
    p.f64(
        "NOISE_AMPLIFICATION_BITS",
        comb::noise_amplification(f1, f2).expect("noise amplification"),
    );
    p.f64(
        "IONOSPHERE_FREE_BITS",
        comb::ionosphere_free(2.0e7, 2.0e7, f1, f2).expect("iono-free"),
    );
    p.f64(
        "IONOSPHERE_FREE_PHASE_M_BITS",
        comb::ionosphere_free_phase_m(2.0e7, 2.0e7, f1, f2).expect("iono-free phase"),
    );

    // Carrier-phase scalars.
    p.comment("sidereon_core::carrier_phase scalars.");
    p.f64(
        "PHASE_METERS_BITS",
        cp::phase_meters(1.0e8, 1.57542e9).expect("phase meters"),
    );
    p.f64(
        "GEOMETRY_FREE_BITS",
        cp::geometry_free(2.0e7, 2.0e7).expect("geometry free"),
    );
    p.f64(
        "WIDE_LANE_WAVELENGTH_BITS",
        cp::wide_lane_wavelength(1.57542e9, 1.22760e9).expect("wide-lane wavelength"),
    );
    p.f64(
        "NARROW_LANE_CODE_BITS",
        cp::narrow_lane_code(2.0e7, 2.0e7, 1.57542e9, 1.22760e9).expect("narrow-lane code"),
    );
    p.f64(
        "MELBOURNE_WUBBENA_BITS",
        cp::melbourne_wubbena(1.0e8, 8.0e7, 2.0e7, 2.0e7, 1.57542e9, 1.22760e9).expect("MW"),
    );
    p.f64(
        "WIDE_LANE_CYCLES_BITS",
        cp::wide_lane_cycles(1.0e8, 8.0e7, 2.0e7, 2.0e7, 1.57542e9, 1.22760e9)
            .expect("wide-lane cycles"),
    );
    p.f64(
        "CODE_MINUS_CARRIER_BITS",
        cp::code_minus_carrier(2.0e7, 1.0e8, 1.57542e9).expect("CMC"),
    );

    // Signal quality.
    use sidereon_core::signal as sig;
    p.comment("sidereon_core::signal and quality scalars.");
    p.int(
        "CA_CHIP_PRN1_INDEX0",
        i128::from(sig::ca_chip(1, 0).expect("chip")),
    );
    p.f64(
        "COHERENT_LOSS_BITS",
        sig::coherent_loss(100.0, 0.001).expect("coherent loss"),
    );
    p.f64(
        "COHERENT_LOSS_DB_BITS",
        sig::coherent_loss_db(100.0, 0.001).expect("coherent loss dB"),
    );
    p.f64(
        "SNR_POST_DB_BITS",
        sig::snr_post_db(45.0, 0.02).expect("SNR"),
    );
    p.f64(
        "PSEUDORANGE_VARIANCE_BITS",
        quality::pseudorange_variance(30.0, quality::PseudorangeVarianceOptions::default())
            .expect("pseudorange variance"),
    );
    p.f64(
        "CHI2_INV_BITS",
        quality::chi2_inv(0.999, 1).expect("chi2 inverse"),
    );

    // RAIM.
    let sats = ["G01", "G02", "G03", "G04", "G05"];
    let residuals = [0.4, -0.3, 0.5, -0.2, 0.35];
    let mut options = RaimOptions::default();
    options.p_fa = 1.0e-3;
    options.weights = RaimWeights::Unit;
    options.n_systems = None;
    let raim = quality::raim(
        &RaimInput {
            used_sats: sats.iter().map(|s| s.to_string()).collect(),
            residuals_m: residuals.to_vec(),
            variances_m2: None,
        },
        &options,
    )
    .expect("RAIM");
    p.comment("sidereon_core::quality::raim on test_raim's residuals, unit weights.\nThe reduced chi-square and RMS are the binding's (src/raim.rs,\nraim_result_to_c and residual_rms_m) over the engine's statistic.");
    p.bool("RAIM_FAULT_DETECTED", raim.fault_detected);
    p.f64("RAIM_TEST_STATISTIC_BITS", raim.test_statistic);
    p.bool("RAIM_HAS_THRESHOLD", raim.threshold.is_some());
    p.f64("RAIM_THRESHOLD_BITS", raim.threshold.unwrap_or(0.0));
    p.bool("RAIM_HAS_REDUCED_CHI_SQUARE", raim.dof > 0);
    p.f64(
        "RAIM_REDUCED_CHI_SQUARE_BITS",
        raim.test_statistic / raim.dof as f64,
    );
    let sum_squares: f64 = residuals.iter().map(|r| r * r).sum();
    p.f64(
        "RAIM_RMS_M_BITS",
        (sum_squares / residuals.len() as f64).sqrt(),
    );
    p.int("RAIM_DOF", raim.dof as i128);
    p.bool("RAIM_TESTABLE", raim.testable);
    p.int(
        "RAIM_NORMALIZED_RESIDUAL_COUNT",
        raim.normalized_residuals.len() as i128,
    );
    p.bool("RAIM_HAS_WORST_SAT", raim.worst_sat.is_some());
    p.str("RAIM_WORST_SAT", raim.worst_sat.as_deref().unwrap_or(""));
    let (first_sat, first_value) = raim.normalized_residuals.iter().next().expect("row");
    p.str("RAIM_FIRST_NORMALIZED_SAT", first_sat);
    p.f64("RAIM_FIRST_NORMALIZED_RESIDUAL_BITS", *first_value);

    // The engine default weights: each residual over its own variance.
    let variances = [0.09, 0.16, 0.25, 0.36, 0.49];
    let solution_input = RaimInput {
        used_sats: sats.iter().map(|s| s.to_string()).collect(),
        residuals_m: residuals.to_vec(),
        variances_m2: Some(variances.to_vec()),
    };
    let mut solution_options = RaimOptions::default();
    solution_options.p_fa = 1.0e-3;
    let solution = quality::raim(&solution_input, &solution_options).expect("RAIM");
    p.comment("sidereon_core::quality::raim on the same residuals over variances\n0.09, 0.16, 0.25, 0.36, 0.49 m^2 under the default Solution weights.");
    p.f64s("RAIM_VARIANCES_BITS", &variances);
    p.bool("RAIM_SOLUTION_FAULT_DETECTED", solution.fault_detected);
    p.f64("RAIM_SOLUTION_TEST_STATISTIC_BITS", solution.test_statistic);
    let (_, solution_first) = solution.normalized_residuals.iter().next().expect("row");
    p.f64(
        "RAIM_SOLUTION_FIRST_NORMALIZED_RESIDUAL_BITS",
        *solution_first,
    );
    let without = RaimInput {
        variances_m2: None,
        ..solution_input
    };
    p.bool(
        "RAIM_SOLUTION_WITHOUT_VARIANCES_REFUSED",
        quality::raim(&without, &solution_options).is_err(),
    );

    // Troposphere.
    let receiver = Wgs84Geodetic::new(0.7, 0.1, 100.0).expect("receiver");
    let surface = sidereon_core::positioning::SurfaceMet::default();
    let met = sidereon_core::atmosphere::troposphere::Met::new(
        surface.pressure_hpa,
        surface.temperature_k,
        surface.relative_humidity,
    )
    .expect("met");
    let tt = Instant::from_julian_date(
        TimeScale::Tt,
        JulianDateSplit::new(2451545.0, 0.0).expect("jd"),
    );
    use sidereon_core::atmosphere::troposphere as tropo;
    let zenith =
        tropo::tropo_zenith(tropo::TropoModel::Saastamoinen, receiver, met).expect("zenith");
    let mapping =
        tropo::tropo_mapping(tropo::MappingModel::Niell, 0.5, receiver, tt).expect("mapping");
    p.comment("SurfaceMet::default and the Saastamoinen/Niell troposphere at 0.5 rad.");
    p.f64("MET_PRESSURE_HPA_BITS", surface.pressure_hpa);
    p.f64("MET_TEMPERATURE_K_BITS", surface.temperature_k);
    p.f64("MET_RELATIVE_HUMIDITY_BITS", surface.relative_humidity);
    p.f64("ZENITH_DRY_M_BITS", zenith.dry_m);
    p.f64("ZENITH_WET_M_BITS", zenith.wet_m);
    p.f64("MAPPING_DRY_BITS", mapping.dry);
    p.f64("MAPPING_WET_BITS", mapping.wet);
    p.f64(
        "SLANT_DELAY_BITS",
        tropo::tropo_slant(0.5, receiver, met, tt).expect("slant"),
    );

    // Tides.
    let station = [4517590.0, 837270.0, 4527420.0];
    use sidereon_core::tides;
    p.comment("sidereon_core::tides at 2020-06-24 12 h.");
    p.f64s(
        "SOLID_EARTH_TIDE_M_BITS",
        &tides::solid_earth_tide(
            &station,
            2020,
            6,
            24,
            12.0,
            &[1.4e11, 0.4e11, 0.2e11],
            &[3.0e8, 1.5e8, 1.0e8],
        )
        .expect("solid tide"),
    );
    let blq = tides::OceanLoadingBlq {
        amplitude_m: [[0.0; tides::NUM_OCEAN_CONSTITUENTS]; 3],
        phase_deg: [[0.0; tides::NUM_OCEAN_CONSTITUENTS]; 3],
    };
    p.f64s(
        "OCEAN_TIDE_ZERO_BLQ_M_BITS",
        &tides::ocean_tide_loading(&station, 2020, 6, 24, 12.0, &blq).expect("ocean tide"),
    );
    p.f64s(
        "POLE_TIDE_M_BITS",
        &tides::solid_earth_pole_tide(&station, 2020, 6, 24, 12.0, 0.1, 0.3).expect("pole tide"),
    );

    // Angles, eclipse, Sun/Moon.
    let sat = [7000.0, 0.0, 0.0];
    let sun = [1.5e8, 0.0, 0.0];
    let moon = [3.8e5, 0.0, 0.0];
    p.comment("sidereon_core::astro angles, eclipse and Sun/Moon ephemerides.");
    p.f64(
        "SUN_ANGLE_DEG_BITS",
        angles::sun_angle(sat, sun).expect("sun angle"),
    );
    p.f64(
        "MOON_ANGLE_DEG_BITS",
        angles::moon_angle(sat, moon).expect("moon angle"),
    );
    p.f64(
        "SUN_ELEVATION_DEG_BITS",
        angles::sun_elevation(sat, sun).expect("sun elevation"),
    );
    p.f64(
        "PHASE_ANGLE_DEG_BITS",
        angles::phase_angle(sat, sun, [6371.0, 0.0, 0.0]).expect("phase angle"),
    );
    p.f64(
        "EARTH_ANGULAR_RADIUS_DEG_BITS",
        angles::earth_angular_radius(sat).expect("earth radius"),
    );
    use sidereon_core::astro::events::eclipse;
    p.f64(
        "ECLIPSE_SHADOW_FRACTION_BITS",
        eclipse::shadow_fraction(sat, sun).expect("shadow fraction"),
    );
    p.variant(
        "ECLIPSE_STATUS",
        "SIDEREON_ECLIPSE_STATUS_",
        &eclipse::status(sat, sun).expect("eclipse status"),
    );
    let sm = bodies::sun_moon_eci(0.21).expect("sun/moon ECI");
    p.f64s("SUN_ECI_021_M_BITS", &sm.sun);
    p.f64s("MOON_ECI_021_M_BITS", &sm.moon);
    let epochs = [946728000000000i64, 1593002096000000i64];
    let mut eci_sun = Vec::new();
    let mut eci_moon = Vec::new();
    let mut ecef_sun = Vec::new();
    let mut ecef_moon = Vec::new();
    for e in epochs {
        let ts = sidereon_core::astro::passes::UtcInstant::from_unix_microseconds(e).time_scales();
        let a = bodies::sun_moon_eci_at(&ts).expect("sun/moon ECI at");
        eci_sun.extend_from_slice(&a.sun);
        eci_moon.extend_from_slice(&a.moon);
        let b = bodies::sun_moon_ecef(&ts).expect("sun/moon ECEF");
        ecef_sun.extend_from_slice(&b.sun);
        ecef_moon.extend_from_slice(&b.moon);
    }
    p.f64s("SUN_ECI_BATCH_M_BITS", &eci_sun);
    p.f64s("MOON_ECI_BATCH_M_BITS", &eci_moon);
    p.f64s("SUN_ECEF_BATCH_M_BITS", &ecef_sun);
    p.f64s("MOON_ECEF_BATCH_M_BITS", &ecef_moon);

    // IOD, Lambert, conjunction.
    p.comment("Gibbs, Herrick-Gibbs, Battin and the conjunction functions on the\nVallado-style inputs test_iod_lambert_conjunction passes.");
    let (v2, ..) = iod::gibbs(
        &[0.0, 0.0, 6378.1363],
        &[0.0, -4464.696, -5102.509],
        &[0.0, 5740.323, 3189.068],
    )
    .expect("gibbs");
    p.f64s("GIBBS_V2_BITS", &v2);
    let (v2, ..) = iod::hgibbs(
        &[3419.85564, 6019.82602, 2784.60022],
        &[2935.91195, 6326.18324, 2660.59584],
        &[2434.95202, 6597.38674, 2521.52311],
        0.0,
        (60.0 + 16.48) / 86400.0,
        (120.0 + 33.04) / 86400.0,
    )
    .expect("hgibbs");
    p.f64s("HGIBBS_V2_BITS", &v2);
    let re = 6378.1363;
    let (lv1, lv2) = lambert::battin(
        &[2.5 * re, 0.0, 0.0],
        &[1.9151111 * re, 1.6069690 * re, 0.0],
        &[0.0, 4.999792554221911, 0.0],
        lambert::DirectionOfMotion::Short,
        lambert::DirectionOfEnergy::High,
        1,
        92854.234,
    )
    .expect("battin");
    p.f64s("LAMBERT_V1_BITS", &lv1);
    p.f64s("LAMBERT_V2_BITS", &lv2);
    let frame = conjunction::encounter_frame(
        [7000.0, 0.0, 0.0],
        [0.0, 7.5, 0.0],
        [7000.05, 0.02, 0.0],
        [0.0, -7.5, 0.1],
    )
    .expect("encounter frame");
    p.f64("ENCOUNTER_MISS_KM_BITS", frame.miss_km);
    p.f64(
        "ENCOUNTER_RELATIVE_SPEED_KM_S_BITS",
        frame.relative_speed_km_s,
    );
    let cov = [[0.01, 0.0, 0.0], [0.0, 0.01, 0.0], [0.0, 0.0, 0.01]];
    let pc = conjunction::collision_probability(
        &conjunction::ConjunctionState {
            position_km: [7000.0, 0.0, 0.0],
            velocity_km_s: [0.0, 7.5, 0.0],
            covariance_km2: cov,
        },
        &conjunction::ConjunctionState {
            position_km: [7000.05, 0.02, 0.0],
            velocity_km_s: [0.0, -7.5, 0.1],
            covariance_km2: cov,
        },
        0.02,
        conjunction::PcMethod::FosterEqualArea,
    )
    .expect("collision probability");
    p.f64("COLLISION_PC_BITS", pc.pc);
    p.f64("COLLISION_MISS_KM_BITS", pc.miss_km);

    // Civil time.
    use sidereon_core::astro::time::civil;
    let sec = civil::j2000_seconds(2020, 6, 25, 12, 0, 0.0);
    let (y, mo, d, h, mi, s) = civil::civil_from_j2000_seconds(sec.round() as i64);
    p.comment("sidereon_core::astro::time::civil on 2020-06-25 12:00:00.");
    p.f64("CIVIL_J2000_S_BITS", sec);
    p.f64(
        "SPLIT_JD_J2000_S_BITS",
        civil::j2000_seconds_from_split(2451545.0, 0.0),
    );
    p.ints(
        "CIVIL_ROUND_TRIP",
        "int64_t",
        &[y, mo, d, h, mi, s].map(i128::from),
    );

    // Carrier smoothing over the synthetic four-epoch arc.
    let arc: Vec<ArcEpoch> = (0..4)
        .map(|i| {
            let k = i as f64;
            ArcEpoch {
                phi1_cycles: Some(1.0e8 + k * 1000.0),
                phi2_cycles: Some(0.78e8 + k * 780.0),
                p1_m: Some(2.0e7 + k * 190.0),
                p2_m: Some(2.0e7 + k * 244.0),
                lli1: None,
                lli2: None,
                f1_hz: Some(1.57542e9),
                f2_hz: Some(1.22760e9),
                gap_time_s: if i == 0 { None } else { Some(30.0) },
                gap_epoch: None,
            }
        })
        .collect();
    let smooth = cp::smooth_code(&arc, CycleSlipOptions::default(), 100).expect("smooth code");
    let iono = cp::smooth_iono_free_code(&arc, CycleSlipOptions::default(), 100)
        .expect("smooth iono-free code");
    p.comment("carrier_phase::smooth_code and smooth_iono_free_code, default slip\noptions, window cap 100; an absent value is NaN as the binding writes it.");
    let nan = |v: Option<f64>| v.unwrap_or(f64::NAN);
    p.f64s(
        "SMOOTH_P_M_BITS",
        &smooth.iter().map(|r| nan(r.p_smooth_m)).collect::<Vec<_>>(),
    );
    p.ints(
        "SMOOTH_WINDOW",
        "size_t",
        &smooth.iter().map(|r| r.window as i128).collect::<Vec<_>>(),
    );
    p.ints(
        "SMOOTH_RESET",
        "bool",
        &smooth
            .iter()
            .map(|r| i128::from(r.reset))
            .collect::<Vec<_>>(),
    );
    p.f64s(
        "IONO_FREE_SMOOTH_P_M_BITS",
        &iono.iter().map(|r| nan(r.p_smooth_m)).collect::<Vec<_>>(),
    );
    p.f64s(
        "IONO_FREE_P_IF_M_BITS",
        &iono.iter().map(|r| nan(r.p_if_m)).collect::<Vec<_>>(),
    );
    p.f64s(
        "IONO_FREE_L_IF_M_BITS",
        &iono.iter().map(|r| nan(r.l_if_m)).collect::<Vec<_>>(),
    );

    // Eccentric anomaly.
    let ea = sidereon_core::ephemeris::eccentric_anomaly(0.5, 0.01).expect("eccentric anomaly");
    p.f64("ECCENTRIC_ANOMALY_RAD_BITS", ea.value);
    p.int("ECCENTRIC_ANOMALY_ITERATIONS", ea.iterations as i128);

    // CDM.
    use sidereon_core::astro::cdm;
    let kvn =
        cdm::parse_kvn(&read(&tests_path("fixtures/cdm/ccsds_example2.kvn"))).expect("CDM KVN");
    p.comment("sidereon_core::astro::cdm on fixtures/cdm/ccsds_example2.{kvn,xml}; an\nabsent number is NaN as the binding writes it.");
    p.f64(
        "CDM_MISS_DISTANCE_M_BITS",
        kvn.miss_distance_m.unwrap_or(f64::NAN),
    );
    p.f64(
        "CDM_RELATIVE_SPEED_M_S_BITS",
        kvn.relative_speed_m_s.unwrap_or(f64::NAN),
    );
    p.f64(
        "CDM_COLLISION_PROBABILITY_BITS",
        kvn.collision_probability.unwrap_or(f64::NAN),
    );
    p.f64(
        "CDM_HARD_BODY_RADIUS_M_BITS",
        kvn.hard_body_radius_m.unwrap_or(f64::NAN),
    );
    let ((px, py, pz), (vx, vy, vz)) = kvn.object1.state;
    p.f64s("CDM_OBJECT1_POSITION_BITS", &[px, py, pz]);
    p.f64s("CDM_OBJECT1_VELOCITY_BITS", &[vx, vy, vz]);
    p.f64s(
        "CDM_OBJECT1_COVARIANCE_RTN_BITS",
        &kvn.object1.covariance_rtn,
    );
    p.int(
        "CDM_KVN_TEXT_LEN",
        cdm::encode_kvn(&kvn).expect("CDM encode").len() as i128,
    );
    p.str("CDM_TCA", kvn.tca.as_deref().unwrap_or(""));
    p.str(
        "CDM_OBJECT1_REF_FRAME",
        kvn.object1.ref_frame.as_deref().unwrap_or(""),
    );
    p.str(
        "CDM_OBJECT2_COVARIANCE_METHOD",
        kvn.object2.covariance_method.as_deref().unwrap_or(""),
    );
    p.str(
        "CDM_OBJECT1_MANEUVERABLE",
        kvn.object1.maneuverable.as_deref().unwrap_or(""),
    );
    p.bool(
        "CDM_OBJECT1_HAS_VELOCITY_COVARIANCE",
        kvn.object1.velocity_covariance_rtn.is_some(),
    );
    p.bool(
        "CDM_OBJECT2_HAS_VELOCITY_COVARIANCE",
        kvn.object2.velocity_covariance_rtn.is_some(),
    );
    p.outcome(
        "CDM_XML",
        &cdm::parse_xml(&read(&tests_path("fixtures/cdm/ccsds_example2.xml"))),
    );

    // RINEX clock.
    let clock = sidereon_core::rinex::clock::RinexClock::parse(&read(&tests_path(
        "fixtures/clk/synthetic_rinex_clock.clk",
    )))
    .expect("RINEX clock");
    p.comment("RinexClock::parse(fixtures/clk/synthetic_rinex_clock.clk) and the writer.");
    p.int("RINEX_CLOCK_SATELLITE_COUNT", clock.series().len() as i128);
    p.int(
        "RINEX_CLOCK_TEXT_LEN",
        clock.to_rinex_string().expect("clock text").len() as i128,
    );
    let gps = sidereon_core::rinex::clock::civil_to_gps_seconds(2020, 6, 25, 0, 0, 0.0);
    p.bool("CIVIL_TO_GPS_AVAILABLE", gps.is_some());
    p.f64("CIVIL_TO_GPS_SECONDS_BITS", gps.unwrap_or(0.0));

    // SP3-coupled: G01 state at the middle GRG epoch and one DGNSS round.
    let grg = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3")
    .with_interpolation_options(Default::default());
    let epochs = grg.epochs_j2000_seconds();
    let mid = epochs[epochs.len() / 2];
    let state = grg
        .observable_state_at_j2000_s("G01".parse().expect("G01"), mid)
        .expect("G01 state");
    let pos = state.position_ecef_m;
    let base = [1130773.0, -4831253.0, 3994200.0];
    let (dx, dy, dz) = (pos[0] - base[0], pos[1] - base[1], pos[2] - base[2]);
    let range = (dx * dx + dy * dy + dz * dz).sqrt();
    let obs = vec![sidereon_core::dgnss::CodeObservation {
        satellite_id: "G01".to_string(),
        pseudorange_m: range,
    }];
    let corrections = sidereon_core::dgnss::pseudorange_corrections(&grg, base, &obs, mid)
        .expect("DGNSS corrections");
    let applied = sidereon_core::dgnss::apply_corrections(&obs, &corrections).expect("apply");
    p.comment("GRG SP3 G01 at its middle epoch and one DGNSS base observation formed\nas test_sp3_coupled forms it.");
    p.f64s("G01_MID_POSITION_M_BITS", &pos);
    p.bool("G01_MID_HAS_CLOCK", state.clock_s.is_some());
    p.f64("G01_MID_CLOCK_S_BITS", state.clock_s.unwrap_or(0.0));
    p.int("DGNSS_CORRECTION_COUNT", corrections.len() as i128);
    p.int("DGNSS_CORRECTED_COUNT", applied.corrected.len() as i128);
    p.int("DGNSS_DROPPED_COUNT", applied.dropped.len() as i128);

    // Broadcast-vs-precise comparison.
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .expect("NAV");
    let precise = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/COD0MGXFIN_20201770000_01D_05M_ORB.SP3",
    )))
    .expect("COD SP3")
    .with_interpolation_options(Default::default());
    let epochs = precise.epochs_j2000_seconds();
    let t = epochs[epochs.len() / 2];
    let jd = t / 86400.0 + 2451545.0;
    let whole = jd.floor();
    let frac = jd - whole;
    let half = 1.0 / 86400.0;
    let g01: Vec<sidereon_core::GnssSatelliteId> = vec!["G01".parse().expect("G01")];
    let split = |w: f64, f: f64| JulianDateSplit::new(w, f);
    let compared = match (
        split(whole, frac),
        split(whole, frac + half),
        split(whole, frac - half),
    ) {
        (Ok(precise_jd), Ok(plus), Ok(minus)) => sidereon_core::broadcast_comparison::compare(
            &nav,
            &precise,
            &g01,
            &[sidereon_core::broadcast_comparison::EpochInputs {
                broadcast_t_j2000_s: t,
                precise: precise_jd,
                precise_plus: plus,
                precise_minus: minus,
            }],
            1.0,
        )
        .map_err(|e| e.to_string()),
        _ => Err("split Julian date".to_string()),
    };
    p.comment("broadcast_comparison::compare and compare_window for G01 at the COD SP3\nmiddle epoch, as test_broadcast_comparison forms the epochs.");
    p.bool("COMPARE_OK", compared.is_ok());
    p.int(
        "COMPARE_OVERALL_COUNT",
        compared.as_ref().map_or(0, |r| r.overall.count as i128),
    );
    p.int(
        "COMPARE_SATELLITE_COUNT",
        compared
            .as_ref()
            .map_or(0, |r| r.per_satellite.len() as i128),
    );
    let windowed = split(whole, frac)
        .map_err(|e| e.to_string())
        .and_then(|start| {
            sidereon_core::broadcast_comparison::compare_window(
                &nav,
                &precise,
                &g01,
                &sidereon_core::broadcast_comparison::CompareWindow {
                    broadcast_window_j2000_s: (t, t + 900.0),
                    precise_start: start,
                    step_s: 900.0,
                    velocity_half_s: half,
                },
            )
            .map_err(|e| e.to_string())
        });
    p.bool("COMPARE_WINDOW_OK", windowed.is_ok());
    p.int(
        "COMPARE_WINDOW_OVERALL_COUNT",
        windowed.as_ref().map_or(0, |r| r.overall.count as i128),
    );

    header_end(GUARD);
}
