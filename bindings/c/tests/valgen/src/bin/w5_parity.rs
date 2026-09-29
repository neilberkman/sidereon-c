//! tests/w5_parity_pins.h: sidereon-core's results for the inputs
//! parity_smoke.c passes (SP3 geometry, observables, broadcast velocity,
//! reduced orbits, NRLMSISE-00).

#[path = "w5_support/inputs.rs"]
mod inputs;
#[path = "w5_support/pins.rs"]
mod pins;

use pins::Pins;
use sidereon_core::astro::atmosphere::{
    nrlmsise00_with_lst, NrlmsiseInput, DEFAULT_AP, DEFAULT_F107, DEFAULT_F107A,
};
use sidereon_core::astro::time::TimeScale;
use sidereon_core::ephemeris::{BroadcastEphemeris, Sp3};
use sidereon_core::geometry::{passes, visibility_series, visible, VisibilityOptions};
use sidereon_core::observables::{predict, PredictOptions, PredictedObservables};
use sidereon_core::orbit::{
    drift, fit_piecewise, fit_with_model, piecewise_drift, piecewise_position,
    piecewise_position_velocity, position, position_velocity, select_piecewise_segment,
    CalendarEpoch, EcefSample, Frame, Model,
};
use sidereon_core::velocity::{VelocityObservable, VelocityObservation, VelocitySolveOptions};
use sidereon_core::GnssSystem;
use std::collections::BTreeSet;
use valgen::{header_end, header_start, read, read_bytes, tests_path};

const GUARD: &str = "SIDEREON_W5_PARITY_PINS_H";
const RO_SAMPLE_COUNT: usize = 9;

fn observables(p: &Pins, name: &str, o: &PredictedObservables) {
    p.f64(&format!("{name}_RANGE_M_BITS"), o.geometric_range_m);
    p.f64(&format!("{name}_RANGE_RATE_M_S_BITS"), o.range_rate_m_s);
    p.f64(&format!("{name}_DOPPLER_HZ_BITS"), o.doppler_hz);
    p.f64(&format!("{name}_ELEVATION_DEG_BITS"), o.elevation_deg);
    p.f64(&format!("{name}_AZIMUTH_DEG_BITS"), o.azimuth_deg);
    p.f64s(&format!("{name}_LOS_UNIT_BITS"), &o.los_unit);
}

fn main() {
    header_start("w5_parity", GUARD);
    let p = Pins::new("W5_PARITY");

    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3")
    .with_interpolation_options(Default::default());
    let spp = inputs::spp();
    let receiver = spp.position_m;
    let t_rx = spp.t_rx_j2000_s;
    let sats = sp3.satellites().to_vec();

    // Geometry.
    let mut options = VisibilityOptions::default();
    options.elevation_mask_deg = 10.0;
    options.systems = None;
    let rows = visible(&sp3, &sats, receiver, t_rx, &options).expect("visible");
    p.comment("sidereon_core::geometry over the GRG SP3 at the SPP receiver and epoch,\n10 degree mask; the series and passes run 1800 s at 600 s steps.");
    p.int("VISIBLE_COUNT", rows.len() as i128);
    let tokens: Vec<String> = rows.iter().map(|r| r.satellite.to_string()).collect();
    println!(
        "static const char *const W5_PARITY_VISIBLE_SATELLITES[{}] = {{ {} }};",
        tokens.len().max(1),
        if tokens.is_empty() {
            "\"\"".to_string()
        } else {
            tokens
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    p.f64s(
        "VISIBLE_ELEVATION_DEG_BITS",
        &rows.iter().map(|r| r.elevation_deg).collect::<Vec<_>>(),
    );
    p.f64s(
        "VISIBLE_AZIMUTH_DEG_BITS",
        &rows.iter().map(|r| r.azimuth_deg).collect::<Vec<_>>(),
    );
    let mut gps = options.clone();
    gps.systems = Some(BTreeSet::from([GnssSystem::Gps]));
    p.int(
        "VISIBLE_GPS_COUNT",
        visible(&sp3, &sats, receiver, t_rx, &gps)
            .expect("visible GPS")
            .len() as i128,
    );
    let series = visibility_series(&sp3, &sats, receiver, (t_rx, t_rx + 1800.0), 600, &options)
        .expect("series");
    p.int("SERIES_COUNT", series.len() as i128);
    p.ints(
        "SERIES_N_VISIBLE",
        "size_t",
        &series
            .iter()
            .map(|s| s.n_visible as i128)
            .collect::<Vec<_>>(),
    );
    p.int(
        "PASS_COUNT",
        passes(&sp3, &sats, receiver, (t_rx, t_rx + 1800.0), 600, &options)
            .expect("passes")
            .len() as i128,
    );

    // Observables.
    let defaults = PredictOptions::default();
    p.comment("PredictOptions::default and sidereon_core::observables::predict from the\nGRG SP3 (first SPP satellite) and the ESBC broadcast NAV (each broadcast\nfixture satellite).");
    p.f64("OPTIONS_CARRIER_HZ_BITS", defaults.carrier_hz);
    p.bool("OPTIONS_LIGHT_TIME", defaults.light_time);
    p.bool("OPTIONS_SAGNAC", defaults.sagnac);
    let sp3_obs = predict(
        &sp3,
        spp.sat_ids[0].parse().expect("sat"),
        receiver,
        t_rx,
        defaults,
    )
    .expect("SP3 observables");
    observables(&p, "SP3_OBS", &sp3_obs);
    let nav = BroadcastEphemeris::from_nav(&read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    )))
    .expect("NAV");
    let bc = inputs::broadcast();
    let mut bc_ok = Vec::new();
    let mut bc_rate = Vec::new();
    let mut bc_range = Vec::new();
    let mut velocity_obs = Vec::new();
    for sat in &bc.sat_ids {
        let id = sat.parse().expect("sat");
        match predict(&nav, id, bc.receiver_m, bc.t_rx_j2000_s, defaults) {
            Ok(o) => {
                bc_ok.push(1);
                bc_rate.push(o.range_rate_m_s);
                bc_range.push(o.geometric_range_m);
                velocity_obs.push(VelocityObservation {
                    satellite_id: id,
                    value: o.range_rate_m_s,
                    carrier_hz: defaults.carrier_hz,
                    sat_clock_drift_s_s: 0.0,
                });
            }
            Err(_) => {
                bc_ok.push(0);
                bc_rate.push(0.0);
                bc_range.push(0.0);
            }
        }
    }
    p.ints("BROADCAST_OBS_OK", "bool", &bc_ok);
    p.f64s("BROADCAST_OBS_RANGE_M_BITS", &bc_range);
    p.f64s("BROADCAST_OBS_RANGE_RATE_M_S_BITS", &bc_rate);

    // Broadcast velocity from the predicted range rates.
    let mut vopts = VelocitySolveOptions::default();
    vopts.observable = VelocityObservable::RangeRate;
    let solution =
        sidereon_core::velocity::solve(&nav, &velocity_obs, bc.receiver_m, bc.t_rx_j2000_s, vopts)
            .expect("broadcast velocity");
    p.comment("sidereon_core::velocity::solve on those range rates (static receiver,\nzero clock drift), range-rate observable.");
    p.f64s("VELOCITY_M_S_BITS", &solution.velocity_m_s);
    p.f64("SPEED_M_S_BITS", solution.speed_m_s);
    p.f64("CLOCK_DRIFT_S_S_BITS", solution.clock_drift_s_s);
    p.int("USED_SAT_COUNT", solution.used_sats.len() as i128);

    // Reduced orbit on the first nine GRG epochs of the first SPP satellite.
    let epochs = sp3.epochs_j2000_seconds();
    let sat = spp.sat_ids[0].parse().expect("sat");
    let samples: Vec<EcefSample> = (0..RO_SAMPLE_COUNT)
        .map(|i| {
            let state = sp3
                .position_at_j2000_seconds(sat, epochs[i])
                .expect("interpolate");
            let pos = state.position.as_array();
            let step_s = i as i64 * 900;
            let epoch = CalendarEpoch::new(
                2020,
                6,
                24,
                (step_s / 3600) as i32,
                ((step_s % 3600) / 60) as i32,
                (step_s % 60) as f64,
            );
            EcefSample::new(epoch, pos[0], pos[1], pos[2])
        })
        .collect();
    let orbit =
        fit_with_model(&samples, TimeScale::Gpst, Model::CircularSecular).expect("reduced fit");
    p.comment("sidereon_core::orbit on nine GRG samples interpolated as\nexercise_reduced_orbit forms them (GPST, circular secular model).");
    p.int("FIT_N_SAMPLES", orbit.stats.n_samples as i128);
    p.f64("FIT_RMS_M_BITS", orbit.stats.rms_m);
    p.f64("FIT_MAX_M_BITS", orbit.stats.max_m);
    p.f64("FIT_A_M_BITS", orbit.elements.a_m);
    p.f64(
        "FIT_MEAN_MOTION_RAD_S_BITS",
        orbit.elements.mean_motion_rad_s,
    );
    let e0 = samples[0].epoch;
    p.f64s(
        "POSITION_ECEF_BITS",
        &position(&orbit.elements, e0, TimeScale::Gpst, Frame::Ecef).expect("position"),
    );
    let (_, vel) = position_velocity(&orbit.elements, e0, TimeScale::Gpst, Frame::Gcrs)
        .expect("position velocity");
    p.f64s("VELOCITY_GCRS_BITS", &vel);
    let report = drift(&orbit.elements, &samples, TimeScale::Gpst, 1.0e9).expect("drift");
    p.int("DRIFT_ENTRY_COUNT", report.per_epoch.len() as i128);
    p.f64s(
        "DRIFT_ERROR_M_BITS",
        &report
            .per_epoch
            .iter()
            .map(|e| e.error_m)
            .collect::<Vec<_>>(),
    );
    p.f64("DRIFT_MAX_M_BITS", report.max_m);
    p.f64("DRIFT_RMS_M_BITS", report.rms_m);
    p.bool("DRIFT_HAS_CROSSING", report.threshold_index.is_some());
    let crossed = drift(&orbit.elements, &samples, TimeScale::Gpst, 0.0).expect("drift 0 m");
    p.bool("CROSSED_HAS_CROSSING", crossed.threshold_index.is_some());
    p.int(
        "CROSSED_THRESHOLD_INDEX",
        crossed.threshold_index.unwrap_or(0) as i128,
    );
    let piecewise = fit_piecewise(
        &samples,
        TimeScale::Gpst,
        Model::CircularSecular,
        samples[0].epoch,
        samples[RO_SAMPLE_COUNT - 1].epoch,
        3600,
    )
    .expect("piecewise fit");
    p.int("PIECEWISE_N_SEGMENTS", piecewise.segments.len() as i128);
    p.int("PIECEWISE_SEGMENT_S", i128::from(piecewise.segment_s));
    p.ints(
        "PIECEWISE_SEGMENT_N_SAMPLES",
        "size_t",
        &piecewise
            .segments
            .iter()
            .map(|s| s.orbit.stats.n_samples as i128)
            .collect::<Vec<_>>(),
    );
    p.f64s(
        "PIECEWISE_SEGMENT_RMS_M_BITS",
        &piecewise
            .segments
            .iter()
            .map(|s| s.orbit.stats.rms_m)
            .collect::<Vec<_>>(),
    );
    p.f64s(
        "PIECEWISE_SEGMENT_A_M_BITS",
        &piecewise
            .segments
            .iter()
            .map(|s| s.orbit.elements.a_m)
            .collect::<Vec<_>>(),
    );
    let e4 = samples[4].epoch;
    let selected = select_piecewise_segment(&piecewise, e4).expect("select");
    let index = piecewise
        .segments
        .iter()
        .position(|s| std::ptr::eq(s, selected))
        .expect("segment index");
    p.int("SELECTED_INDEX", index as i128);
    p.f64("SELECTED_RMS_M_BITS", selected.orbit.stats.rms_m);
    p.f64s(
        "PIECEWISE_POSITION_ECEF_BITS",
        &piecewise_position(&piecewise, e4, TimeScale::Gpst, Frame::Ecef)
            .expect("piecewise position"),
    );
    let (_, pvel) = piecewise_position_velocity(&piecewise, e4, TimeScale::Gpst, Frame::Gcrs)
        .expect("piecewise position velocity");
    p.f64s("PIECEWISE_VELOCITY_GCRS_BITS", &pvel);
    let preport =
        piecewise_drift(&piecewise, &samples, TimeScale::Gpst, 1.0e9).expect("piecewise drift");
    p.int(
        "PIECEWISE_DRIFT_ENTRY_COUNT",
        preport.per_epoch.len() as i128,
    );
    p.f64("PIECEWISE_DRIFT_MAX_M_BITS", preport.max_m);
    p.f64("PIECEWISE_DRIFT_RMS_M_BITS", preport.rms_m);
    p.bool(
        "PIECEWISE_DRIFT_HAS_CROSSING",
        preport.threshold_index.is_some(),
    );

    // NRLMSISE-00.
    let input = NrlmsiseInput {
        year: 2020,
        doy: 175,
        sec: 43200.0,
        alt: 400.0,
        g_lat: 55.0,
        g_long: 12.0,
        lst: 0.0,
        f107a: DEFAULT_F107A,
        f107: DEFAULT_F107,
        ap: DEFAULT_AP,
        ap_array: None,
    };
    let nominal = nrlmsise00_with_lst(&input, None).expect("nominal");
    let mut aph = input;
    aph.ap_array = Some([100.0; 7]);
    let history = nrlmsise00_with_lst(&aph, None).expect("ap history");
    let mut bad = input;
    bad.alt = 5000.0;
    p.comment("nrlmsise00_with_lst at 400 km, 55 N, 12 E, 2020 DOY 175 12 h, default\nf107/f107a/ap, then with an Ap history of 100 and at 5000 km.");
    p.f64("ATMOSPHERE_DENSITY_KG_M3_BITS", nominal.density());
    p.f64("ATMOSPHERE_TEMPERATURE_K_BITS", nominal.temperature_alt());
    p.f64(
        "ATMOSPHERE_AP_HISTORY_DENSITY_KG_M3_BITS",
        history.density(),
    );
    p.f64(
        "ATMOSPHERE_AP_HISTORY_TEMPERATURE_K_BITS",
        history.temperature_alt(),
    );
    p.outcome("ATMOSPHERE_5000_KM", &nrlmsise00_with_lst(&bad, None));

    header_end(GUARD);
}
