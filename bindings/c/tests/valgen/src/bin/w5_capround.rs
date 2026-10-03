//! tests/w5_capround_pins.h: sidereon-core's results for the inputs
//! capround_smoke.c passes (NeQuick-G, element conversions, observation
//! geometry, geoid grids, civil instants, moving-baseline RTK, RTCM 3).

#[path = "w5_support/pins.rs"]
mod pins;

use pins::{variant_name, Pins};
use sidereon_core::astro::elements::{coe2rv, rv2coe};
use sidereon_core::astro::observation::{
    parallactic_angle_deg, satellite_visual_magnitude, sub_observer_point, sub_solar_point,
    terminator_latitude_deg, SurfacePoint,
};
use sidereon_core::astro::time::Instant;
use sidereon_core::atmosphere::ionosphere::{
    galileo_nequick_g_native, GalileoNequickCoeffs, GalileoNequickEval,
};
use sidereon_core::geoid::{
    ellipsoidal_height_m, geoid_undulation, orthometric_height_m, GeoidGrid,
    ProjVgridshiftArithmetic, ProjVgridshiftError,
};
use sidereon_core::rtcm::{self as core_rtcm, Message, MsmKind, PreviousLock};
use sidereon_core::rtk_filter::defaults::{
    AMBIGUITY_TOL_M, CODE_SIGMA_M, MAX_ITERATIONS, PARTIAL_MIN_AMBIGUITIES, PHASE_SIGMA_M,
    POSITION_TOL_M, RATIO_THRESHOLD,
};
use sidereon_core::rtk_filter::{
    solve_moving_baseline, AmbiguityScale, AmbiguitySet, Epoch, FixedSolveOpts, FloatSolveOpts,
    MeasModel, MovingBaselineEpoch, MovingBaselineOpts, SatMeas, StochasticModel,
};
use sidereon_core::GnssSystem;
use std::collections::BTreeMap;
use valgen::{header_end, header_start, read, tests_path};

const GUARD: &str = "SIDEREON_W5_CAPROUND_PINS_H";

/// The bytes of `static const unsigned char RTCM_STREAM[]` in capround_smoke.c.
fn rtcm_stream() -> Vec<u8> {
    let source = read(&tests_path("capround_smoke.c"));
    let start = source
        .find("static const unsigned char RTCM_STREAM[] = {")
        .expect("RTCM_STREAM");
    let body = &source[start..];
    let body = &body[body.find('{').expect("{") + 1..body.find("};").expect("};")];
    body.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| u8::from_str_radix(t.trim_start_matches("0x"), 16).expect("byte"))
        .collect()
}

fn proj_error(p: &Pins, name: &str, result: &Result<f64, ProjVgridshiftError>) {
    p.bool(&format!("{name}_OK"), result.is_ok());
    let (variant, field) = match result {
        Ok(_) => ("None".to_string(), "none"),
        Err(err) => {
            let field = match err {
                ProjVgridshiftError::NonFiniteCoordinate { field }
                | ProjVgridshiftError::CoordinateOutsideGrid { field } => *field,
            };
            (variant_name(err), field)
        }
    };
    p.variant_named(
        &format!("{name}_KIND"),
        "SIDEREON_PROJ_VGRIDSHIFT_ERROR_KIND_",
        &variant,
    );
    println!(
        "#define W5_CAPROUND_{name}_COORDINATE SIDEREON_PROJ_VGRIDSHIFT_COORDINATE_{}",
        field.to_ascii_uppercase()
    );
}

fn main() {
    header_start("w5_capround", GUARD);
    let p = Pins::new("W5_CAPROUND");

    // NeQuick-G.
    let eval = GalileoNequickEval {
        lat_deg: 45.0,
        lon_deg: 9.0,
        el_deg: 30.0,
        t_gal_s: 43200.0,
        day_of_year: 80.0,
        frequency_hz: 1.57542e9,
    };
    p.comment("galileo_nequick_g_native with zero and with broadcast coefficients.");
    p.f64(
        "NEQUICK_DEFAULT_DELAY_M_BITS",
        galileo_nequick_g_native(
            &GalileoNequickCoeffs {
                ai0: 0.0,
                ai1: 0.0,
                ai2: 0.0,
            },
            eval,
        )
        .expect("nequick default"),
    );
    p.f64(
        "NEQUICK_BROADCAST_DELAY_M_BITS",
        galileo_nequick_g_native(
            &GalileoNequickCoeffs {
                ai0: 80.0,
                ai1: 0.1,
                ai2: 0.05,
            },
            eval,
        )
        .expect("nequick broadcast"),
    );

    // Elements.
    let mu = 398600.4418;
    let coe = rv2coe([-6045.0, -3490.0, 2500.0], [-3.457, 6.618, 2.533], mu).expect("rv2coe");
    p.comment("rv2coe then coe2rv on the LEO state.");
    p.f64("COE_A_BITS", coe.a);
    p.f64("COE_ECC_BITS", coe.ecc);
    let (r2, v2) = coe2rv(&coe, mu).expect("coe2rv");
    p.f64s("COE2RV_R_BITS", &r2);
    p.f64s("COE2RV_V_BITS", &v2);

    // Observation geometry.
    let sp = sub_solar_point([1.4959787e11, 0.0, 0.0]).expect("sub-solar");
    p.comment("sidereon_core::astro::observation on capround_smoke.c's inputs.");
    p.f64("SUB_SOLAR_LATITUDE_DEG_BITS", sp.latitude_deg);
    p.f64("SUB_SOLAR_LONGITUDE_DEG_BITS", sp.longitude_deg);
    p.f64(
        "TERMINATOR_LATITUDE_DEG_BITS",
        terminator_latitude_deg(
            SurfacePoint {
                latitude_deg: sp.latitude_deg,
                longitude_deg: sp.longitude_deg,
            },
            90.0,
        )
        .expect("terminator"),
    );
    p.f64(
        "PARALLACTIC_ANGLE_DEG_BITS",
        parallactic_angle_deg(40.0, 0.0, 20.0).expect("parallactic"),
    );
    p.f64(
        "VISUAL_MAGNITUDE_BITS",
        satellite_visual_magnitude(1000.0, 0.0, 5.0, 1000.0).expect("magnitude"),
    );
    let sub = sub_observer_point([1.0, 0.0, 0.0], 0.0, 90.0, 0.0).expect("sub-observer");
    p.f64("SUB_OBSERVER_LATITUDE_DEG_BITS", sub.latitude_deg);
    p.f64("SUB_OBSERVER_LONGITUDE_DEG_BITS", sub.longitude_deg);

    // Geoid.
    p.comment("Built-in geoid, a 2x2 text grid, the same grid from samples, and the\nzero-filled PROJ EGM96 GTX.");
    p.f64("GEOID_UNDULATION_M_BITS", geoid_undulation(0.7, 0.1));
    let ortho = orthometric_height_m(100.0, 0.7, 0.1);
    p.f64("ORTHOMETRIC_HEIGHT_M_BITS", ortho);
    p.f64(
        "ELLIPSOIDAL_HEIGHT_M_BITS",
        ellipsoidal_height_m(ortho, 0.7, 0.1),
    );
    let grid =
        GeoidGrid::from_text("# lat_min lon_min dlat dlon n_lat n_lon\n0 0 1 1 2 2\n0 10 20 30\n")
            .expect("text grid");
    p.f64("TEXT_GRID_MID_DEG_BITS", grid.undulation_deg(0.5, 0.5));
    let deg = std::f64::consts::PI / 180.0;
    p.f64(
        "TEXT_GRID_MID_RAD_BITS",
        grid.undulation_rad(0.5 * deg, 0.5 * deg),
    );
    let rows = 721usize;
    let cols = 1440usize;
    let header = 40usize;
    let mut gtx = vec![0u8; header + rows * cols * 4];
    gtx[0..8].copy_from_slice(&(-90.0f64).to_be_bytes());
    gtx[8..16].copy_from_slice(&(-180.0f64).to_be_bytes());
    gtx[16..24].copy_from_slice(&0.25f64.to_be_bytes());
    gtx[24..32].copy_from_slice(&0.25f64.to_be_bytes());
    gtx[32..36].copy_from_slice(&(rows as u32).to_be_bytes());
    gtx[36..40].copy_from_slice(&(cols as u32).to_be_bytes());
    let put = |gtx: &mut Vec<u8>, index: usize, value: f32| {
        let at = header + index * 4;
        gtx[at..at + 4].copy_from_slice(&value.to_be_bytes());
    };
    put(&mut gtx, 0, 1.0);
    put(&mut gtx, 1, 2.0);
    put(&mut gtx, cols, 3.0);
    put(&mut gtx, cols + 1, 4.0);
    p.outcome(
        "GTX_TRUNCATED",
        &GeoidGrid::from_proj_egm96_gtx(&gtx[..gtx.len() - 1]),
    );
    let proj = GeoidGrid::from_proj_egm96_gtx(&gtx).expect("PROJ GTX");
    // capround_smoke.c forms these as -89.875 * M_PI / 180.0, left to right.
    let lat = -89.875 * std::f64::consts::PI / 180.0;
    let lon = -179.875 * std::f64::consts::PI / 180.0;
    p.f64(
        "PROJ_SEPARATE_BITS",
        proj.undulation_proj_rad(lat, lon, ProjVgridshiftArithmetic::SeparateMultiplyAdd)
            .expect("separate"),
    );
    p.f64(
        "PROJ_FUSED_BITS",
        proj.undulation_proj_rad(lat, lon, ProjVgridshiftArithmetic::FusedMultiplyAdd)
            .expect("fused"),
    );
    proj_error(
        &p,
        "PROJ_NAN_LATITUDE",
        &proj.undulation_proj_rad(f64::NAN, 0.0, ProjVgridshiftArithmetic::SeparateMultiplyAdd),
    );
    proj_error(
        &p,
        "PROJ_INFINITE_LONGITUDE",
        &proj.undulation_proj_rad(
            0.0,
            f64::INFINITY,
            ProjVgridshiftArithmetic::SeparateMultiplyAdd,
        ),
    );
    proj_error(
        &p,
        "PROJ_OUTSIDE_LATITUDE",
        &proj.undulation_proj_rad(2.0, 0.0, ProjVgridshiftArithmetic::SeparateMultiplyAdd),
    );

    // Civil instant.
    let instant = Instant::from_utc_civil(2020, 6, 25, 12, 0, 0.0).expect("instant");
    let jd = instant.julian_date().expect("julian date");
    p.comment("Instant::from_utc_civil(2020-06-25 12:00:00 UTC).");
    p.f64("INSTANT_JD_WHOLE_BITS", jd.jd_whole);
    p.f64("INSTANT_JD_FRACTION_BITS", jd.fraction);
    p.f64(
        "INSTANT_J2000_S_BITS",
        sidereon_core::astro::time::civil::j2000_seconds_from_split(jd.jd_whole, jd.fraction),
    );

    // Moving-baseline RTK on the synthetic five-satellite epoch.
    let c = 299792458.0;
    let f_l1 = 1575420000.0;
    let lambda = c / f_l1;
    let sats: [(&str, [f64; 3], i64); 5] = [
        ("G01", [15000000.0, 7000000.0, 21000000.0], 0),
        ("G02", [-12000000.0, 18000000.0, 19000000.0], 4),
        ("G03", [20000000.0, -10000000.0, 17000000.0], -7),
        ("G04", [-19000000.0, -13000000.0, 20000000.0], 9),
        ("G05", [9000000.0, 22000000.0, 16000000.0], -3),
    ];
    let base = [-2700000.0, -4300000.0, 3850000.0];
    let baseline = [12.0, -7.0, 5.0];
    let rover = [
        base[0] + baseline[0],
        base[1] + baseline[1],
        base[2] + baseline[2],
    ];
    let row = |i: usize| {
        let (id, pos, cycles) = sats[i];
        let mut db = 0.0;
        let mut dr = 0.0;
        for k in 0..3 {
            db += (pos[k] - base[k]) * (pos[k] - base[k]);
            dr += (pos[k] - rover[k]) * (pos[k] - rover[k]);
        }
        let db = f64::sqrt(db);
        let dr = f64::sqrt(dr);
        SatMeas {
            sat: id.to_string(),
            sd_ambiguity_id: id.to_string(),
            base_code_m: db,
            base_phase_m: db,
            rover_code_m: dr,
            rover_phase_m: dr + cycles as f64 * lambda,
            base_tx_pos: pos,
            rover_tx_pos: pos,
            pos,
        }
    };
    let epoch = Epoch {
        references: vec![row(0)],
        nonref: (1..5).map(row).collect(),
        velocity_mps: None,
        dt_s: 0.0,
    };
    let ids: Vec<String> = ["G02", "G03", "G04", "G05"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let satellites: BTreeMap<String, String> = ids.iter().map(|i| (i.clone(), i.clone())).collect();
    let wavelengths: BTreeMap<String, f64> = ids.iter().map(|i| (i.clone(), lambda)).collect();
    let offsets: BTreeMap<String, f64> = ids.iter().map(|i| (i.clone(), 0.0)).collect();
    let float_only: Vec<String> = Vec::new();
    let mb_epochs = [MovingBaselineEpoch {
        base_position_m: base,
        epoch: &epoch,
        ambiguities: AmbiguitySet {
            ids: &ids,
            satellites: &satellites,
            scale: AmbiguityScale {
                wavelengths_m: &wavelengths,
                offsets_m: &offsets,
            },
            float_only_systems: &float_only,
        },
    }];
    // sidereon_rtk_measurement_model_init, _float_options_init and
    // _fixed_options_init defaults.
    let opts = MovingBaselineOpts {
        model: MeasModel {
            code_sigma_m: CODE_SIGMA_M,
            phase_sigma_m: PHASE_SIGMA_M,
            sagnac: true,
            stochastic: StochasticModel::Simple {
                elevation_weighting: false,
            },
        },
        float: FloatSolveOpts {
            position_tol_m: POSITION_TOL_M,
            ambiguity_tol_m: AMBIGUITY_TOL_M,
            max_iterations: MAX_ITERATIONS,
        },
        fixed: FixedSolveOpts {
            position_tol_m: POSITION_TOL_M,
            ambiguity_tol_m: AMBIGUITY_TOL_M,
            max_iterations: MAX_ITERATIONS,
            ratio_threshold: RATIO_THRESHOLD,
            partial_ambiguity_resolution: false,
            partial_min_ambiguities: PARTIAL_MIN_AMBIGUITIES,
        },
        initial_baseline_m: [0.0; 3],
        warm_start: false,
    };
    let solved = solve_moving_baseline(&mb_epochs, opts, None).expect("moving baseline");
    p.comment("solve_moving_baseline on the synthetic epoch, binding default options.");
    p.int("MB_EPOCH_COUNT", solved.len() as i128);
    p.f64s("MB_BASELINE_M_BITS", &solved[0].baseline_m);
    p.f64("MB_BASELINE_LENGTH_M_BITS", solved[0].baseline_length_m);

    // RTCM.
    let stream = rtcm_stream();
    let messages = core_rtcm::decode_messages(&stream).expect("RTCM stream");
    p.comment("sidereon_core::rtcm on capround_smoke.c's RTCM_STREAM.");
    p.int("RTCM_MESSAGE_COUNT", messages.len() as i128);
    for (i, m) in messages.iter().enumerate() {
        p.variant(
            &format!("RTCM_MESSAGE_{i}_KIND"),
            "SIDEREON_RTCM_MESSAGE_KIND_",
            m,
        );
        p.int(
            &format!("RTCM_MESSAGE_{i}_NUMBER"),
            i128::from(m.message_number()),
        );
    }
    let Message::StationCoordinates(station) = &messages[0] else {
        panic!("message 0 is 1006")
    };
    p.int(
        "STATION_REFERENCE_STATION_ID",
        i128::from(station.reference_station_id),
    );
    p.bool(
        "STATION_HAS_ANTENNA_HEIGHT",
        station.antenna_height.is_some(),
    );
    p.f64(
        "STATION_ANTENNA_HEIGHT_M_BITS",
        station.antenna_height_m().unwrap_or(0.0),
    );
    p.f64("STATION_X_M_BITS", station.x_m());
    let Message::AntennaDescriptor(antenna) = &messages[1] else {
        panic!("message 1 is 1008")
    };
    p.bool(
        "ANTENNA_HAS_SERIAL_NUMBER",
        antenna.antenna_serial_number.is_some(),
    );
    p.bool("ANTENNA_HAS_RECEIVER_TYPE", antenna.receiver_type.is_some());
    p.str("ANTENNA_DESCRIPTOR", antenna.antenna_descriptor.as_str());
    let Message::GpsEphemeris(gps) = &messages[2] else {
        panic!("message 2 is 1019")
    };
    p.int("GPS_SATELLITE_ID", i128::from(gps.satellite_id));
    p.int("GPS_WEEK_NUMBER", i128::from(gps.week_number));
    p.int("GPS_A_F0", i128::from(gps.a_f0));
    let Message::GlonassEphemeris(glo) = &messages[3] else {
        panic!("message 3 is 1020")
    };
    p.int("GLONASS_SATELLITE_ID", i128::from(glo.satellite_id));
    p.int(
        "GLONASS_FREQUENCY_CHANNEL",
        i128::from(glo.frequency_channel),
    );
    p.int("GLONASS_M_N_T", i128::from(glo.m_n_t));
    let Message::Msm(msm) = &messages[4] else {
        panic!("message 4 is 1077")
    };
    p.int("MSM_MESSAGE_NUMBER", i128::from(msm.message_number));
    p.variant("MSM_KIND", "SIDEREON_RTCM_MSM_KIND_", &msm.kind);
    p.variant("MSM_SYSTEM", "SIDEREON_GNSS_SYSTEM_", &msm.system);
    p.int("MSM_SATELLITE_COUNT", msm.satellites.len() as i128);
    p.int("MSM_SIGNAL_COUNT", msm.signals.len() as i128);
    p.int(
        "MSM_REFERENCE_STATION_ID",
        i128::from(msm.header.reference_station_id),
    );
    p.int("MSM_SATELLITE_0_ID", i128::from(msm.satellites[0].id));
    p.bool(
        "MSM_SATELLITE_0_HAS_EXTENDED_INFO",
        msm.satellites[0].extended_info.is_some(),
    );
    p.int("MSM_SIGNAL_0_ID", i128::from(msm.signals[0].signal_id));
    p.bool(
        "MSM_SIGNAL_0_HAS_FINE_PHASE_RANGE_RATE",
        msm.signals[0].fine_phase_range_rate.is_some(),
    );
    p.int("LLI_LOSS_OF_LOCK", i128::from(core_rtcm::LLI_LOSS_OF_LOCK));
    p.int("LLI_HALF_CYCLE", i128::from(core_rtcm::LLI_HALF_CYCLE));
    let min64 = core_rtcm::minimum_lock_time_ms(MsmKind::Msm7, 64);
    p.bool("MIN_LOCK_64_PRESENT", min64.is_some());
    p.int("MIN_LOCK_64_MS", i128::from(min64.unwrap_or(0)));
    let min705 = core_rtcm::minimum_lock_time_ms(MsmKind::Msm7, 705);
    p.bool("MIN_LOCK_705_PRESENT", min705.is_some());
    p.int("MIN_LOCK_705_MS", i128::from(min705.unwrap_or(0)));
    p.int(
        "DERIVED_LLI",
        i128::from(core_rtcm::derive_lli(
            Some(PreviousLock {
                min_lock_time_ms: Some(512),
                elapsed_ms: 1000,
            }),
            Some(512),
            true,
        )),
    );
    p.int(
        "MSM_EPOCH_DT_MS",
        i128::from(core_rtcm::msm_epoch_dt_ms(GnssSystem::Gps, 604799000, 500)),
    );
    p.str(
        "GPS_SIGNAL_2_RINEX_CODE",
        core_rtcm::msm_signal_rinex_code(GnssSystem::Gps, 2).unwrap_or(""),
    );
    let mut tracker = core_rtcm::LockTimeTracker::new();
    let cells = tracker.observe(msm);
    p.int("TRACKER_CELL_COUNT", cells.len() as i128);
    p.int(
        "TRACKER_CELL_0_SATELLITE_ID",
        i128::from(cells[0].satellite_id),
    );
    p.int("TRACKER_CELL_0_SIGNAL_ID", i128::from(cells[0].signal_id));
    p.int("TRACKER_CELL_0_LLI", i128::from(cells[0].lli));
    p.bool(
        "TRACKER_CELL_0_HAS_MIN_LOCK_TIME",
        cells[0].min_lock_time_ms.is_some(),
    );
    p.int(
        "MESSAGE_0_FRAME_LEN",
        messages[0].to_frame().expect("to_frame").len() as i128,
    );
    p.int(
        "MESSAGE_0_BODY_LEN",
        messages[0].encode().expect("encode").len() as i128,
    );

    // The noisy stream: two junk bytes, a truncated MSM frame, then the stream.
    let truncated = core_rtcm::encode_frame(&[0x43, 0x50]).expect("truncated MSM frame");
    let mut noisy = vec![0x11, 0x22];
    noisy.extend_from_slice(&truncated);
    noisy.extend_from_slice(&stream);
    let decoded = core_rtcm::decode_stream_with_policy(&noisy, core_rtcm::RtcmPolicy::Strict);
    p.int("NOISY_MESSAGE_COUNT", decoded.messages.len() as i128);
    p.int(
        "NOISY_RESYNC_BYTES",
        decoded.diagnostics.resync_bytes as i128,
    );
    p.int(
        "NOISY_SKIPPED_COUNT",
        decoded.diagnostics.skipped_frames.len() as i128,
    );
    let skip = &decoded.diagnostics.skipped_frames[0];
    p.int("NOISY_SKIP_OFFSET", skip.offset as i128);
    p.bool(
        "NOISY_SKIP_HAS_MESSAGE_NUMBER",
        skip.message_number.is_some(),
    );
    p.int(
        "NOISY_SKIP_MESSAGE_NUMBER",
        i128::from(skip.message_number.unwrap_or(0)),
    );
    p.variant(
        "NOISY_SKIP_REASON",
        "SIDEREON_RTCM_FRAME_SKIP_REASON_",
        &skip.reason,
    );
    let skip_message = match &skip.reason {
        core_rtcm::FrameSkipReason::Truncated => String::new(),
        core_rtcm::FrameSkipReason::Malformed(message) => message.clone(),
        core_rtcm::FrameSkipReason::Departure(departure) => departure.to_string(),
    };
    p.int("NOISY_SKIP_MESSAGE_LEN", skip_message.len() as i128);

    let frames: Vec<_> = core_rtcm::FrameScanner::new(&stream).collect();
    p.int("FRAME_COUNT", frames.len() as i128);
    p.int("FRAME_0_LEN", frames[0].frame_len as i128);
    p.int("FRAME_0_BODY_LEN", frames[0].body.len() as i128);
    p.int(
        "PAYLOAD_FRAME_LEN",
        core_rtcm::encode_frame(&[0x10, 0x20, 0x30, 0x40, 0x50])
            .expect("payload frame")
            .len() as i128,
    );

    header_end(GUARD);
}
