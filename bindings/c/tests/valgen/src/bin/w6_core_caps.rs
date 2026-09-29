//! core_caps_smoke.c: sidereon-core's SP3 clock-reference offsets, reduced
//! orbit fit and drift, the DGNSS position solve (and the synthetic code
//! observations the test solves, built here and emitted for it), the ANTEX
//! antenna count and the RINEX observation helpers on the fixtures that test
//! reads.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::antex::Antex;
use sidereon_core::dgnss::{solve_position, CodeObservation};
use sidereon_core::ephemeris::{align_clock_reference, clock_reference_offset, Sp3};
use sidereon_core::observables::{predict, PredictOptions};
use sidereon_core::orbit::{
    drift_reduced_orbit_source, fit_reduced_orbit_source, CalendarEpoch, Model, ReducedOrbitSource,
    ReducedOrbitSourceDriftOptions, ReducedOrbitSourceFitOptions, ReducedOrbitSourceSampling,
};
use sidereon_core::positioning::{
    Corrections, KlobucharCoeffs, PseudorangeCode, QzssClock, SolveInputs, SurfaceMet,
    TroposphereModel,
};
use sidereon_core::rinex::observations::{
    carrier_phase_rows, observation_values, pseudoranges, ObservationFilter, RinexObs, SignalPolicy,
};
use std::collections::BTreeMap;
use valgen::{read, read_bytes, tests_path};

const C_M_S: f64 = 299792458.0;

fn g01() -> sidereon_core::GnssSatelliteId {
    "G01".parse().expect("G01")
}

fn sampling(t1_hour: i32) -> ReducedOrbitSourceSampling {
    ReducedOrbitSourceSampling::new(
        CalendarEpoch::new(2020, 6, 24, 0, 0, 0.0),
        CalendarEpoch::new(2020, 6, 24, t1_hour, 0, 0.0),
        900.0,
    )
}

fn main() {
    let guard = "SIDEREON_W6_CORE_CAPS_PINS_H";
    valgen::header_start("w6_core_caps", guard);
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("GRG SP3");

    comment(
        "clock_reference_offset of the product against itself and its alignment, min_common 3.",
    );
    let offsets = clock_reference_offset(&sp3, &sp3, 3);
    def_size("W6_CC_OFFSET_COUNT", offsets.len());
    def_bits("W6_CC_OFFSET0_BITS", offsets[0].offset_s);
    def_size("W6_CC_OFFSET0_SATELLITES", offsets[0].satellites);
    let aligned = align_clock_reference(&sp3, &sp3, 3);
    def_size(
        "W6_CC_ALIGNED_OFFSET_COUNT",
        clock_reference_offset(&sp3, &aligned, 3).len(),
    );

    comment("Reduced orbit fit of G01 (circular secular, 00:00-03:00, 900 s) and drift to 04:00.");
    let fit = fit_reduced_orbit_source(
        ReducedOrbitSource::Sp3 {
            product: &sp3,
            satellite: g01(),
        },
        ReducedOrbitSourceFitOptions::new(sampling(3), Model::CircularSecular),
    )
    .expect("reduced orbit fit");
    let e = &fit.orbit.elements;
    def_bits_array(
        "W6_CC_FIT_ELEMENT_BITS",
        &[
            e.a_m,
            e.e,
            e.i_rad,
            e.raan_rad,
            e.raan_rate_rad_s,
            e.raan_rate_j2_rad_s,
            e.arg_lat_rad,
            e.mean_motion_rad_s,
            e.h,
            e.k,
            e.arg_perigee_rad,
        ],
    );
    def_size("W6_CC_FIT_N_SAMPLES", fit.orbit.stats.n_samples);
    def_size("W6_CC_FIT_REQUESTED", fit.requested_samples);
    def_bits("W6_CC_FIT_RMS_BITS", fit.orbit.stats.rms_m);
    def_bits("W6_CC_FIT_MAX_BITS", fit.orbit.stats.max_m);
    let drift = drift_reduced_orbit_source(
        e,
        ReducedOrbitSource::Sp3 {
            product: &sp3,
            satellite: g01(),
        },
        ReducedOrbitSourceDriftOptions::new(sampling(4), 1.0e9),
    )
    .expect("reduced orbit drift");
    def_bits("W6_CC_DRIFT_MAX_BITS", drift.report.max_m);
    def_bits("W6_CC_DRIFT_RMS_BITS", drift.report.rms_m);
    def_size("W6_CC_DRIFT_REQUESTED", drift.requested_samples);

    // test_dgnss_position: synthetic code observations at the first epoch
    // plus one hour, from the product's own predictions, at most 18.
    let t_rx = sp3.epochs_j2000_seconds()[0] + 3600.0;
    let base = [1130773.0, -4831253.0, 3994200.0];
    let rover = [1130833.0, -4831203.0, 3994230.0];
    let mut sats = Vec::new();
    let mut base_obs = Vec::new();
    let mut rover_obs = Vec::new();
    for sat in sp3.satellites().iter().copied() {
        let (Ok(b), Ok(r)) = (
            predict(&sp3, sat, base, t_rx, PredictOptions::default()),
            predict(&sp3, sat, rover, t_rx, PredictOptions::default()),
        ) else {
            continue;
        };
        let (Some(b_clock), Some(r_clock)) = (b.sat_clock_s, r.sat_clock_s) else {
            continue;
        };
        sats.push(sat.to_string());
        base_obs.push(b.geometric_range_m - C_M_S * b_clock);
        rover_obs.push(r.geometric_range_m - C_M_S * r_clock);
        if sats.len() == 18 {
            break;
        }
    }
    comment("DGNSS inputs: receive time, satellites and base/rover pseudoranges.");
    def_bits("W6_CC_DGNSS_T_RX_BITS", t_rx);
    def_size("W6_CC_DGNSS_COUNT", sats.len());
    println!(
        "static const char *const W6_CC_DGNSS_SATS[{}] = {{ {} }};",
        sats.len(),
        sats.iter()
            .map(|s| valgen::c_string(s))
            .collect::<Vec<_>>()
            .join(", ")
    );
    def_bits_array("W6_CC_DGNSS_BASE_PR_BITS", &base_obs);
    def_bits_array("W6_CC_DGNSS_ROVER_PR_BITS", &rover_obs);
    let code = |prs: &[f64]| -> Vec<CodeObservation> {
        sats.iter()
            .zip(prs)
            .map(|(sat, pr)| CodeObservation {
                satellite_id: sat.clone(),
                pseudorange_m: *pr,
            })
            .collect()
    };
    // sidereon_spp_inputs_v2_init defaults with the fields the test sets.
    let inputs = SolveInputs {
        observations: Vec::new(),
        t_rx_j2000_s: t_rx,
        t_rx_second_of_day_s: 3600.0,
        day_of_year: 176.0,
        initial_guess: [rover[0] + 20.0, rover[1] - 20.0, rover[2] + 10.0, 0.0],
        corrections: Corrections {
            ionosphere: false,
            troposphere: false,
        },
        klobuchar: KlobucharCoeffs {
            alpha: [0.0; 4],
            beta: [0.0; 4],
        },
        beidou_klobuchar: None,
        galileo_nequick: None,
        sbas_iono: None,
        glonass_channels: BTreeMap::new(),
        met: SurfaceMet {
            pressure_hpa: 0.0,
            temperature_k: 0.0,
            relative_humidity: 0.0,
        },
        robust: None,
        pseudorange_code: PseudorangeCode::SingleFrequency,
        troposphere_model: TroposphereModel::Rtklib,
        qzss_clock: QzssClock::Gps,
    };
    let solution = solve_position(
        &sp3,
        base,
        &code(&base_obs),
        &code(&rover_obs),
        inputs,
        false,
    )
    .expect("DGNSS solve");
    comment("DGNSS solution.");
    def_bits_array(
        "W6_CC_DGNSS_BASELINE_VECTOR_BITS",
        &solution.baseline_vector_m,
    );
    def_bits("W6_CC_DGNSS_BASELINE_BITS", solution.baseline_m);
    let p = &solution.solution.position;
    def_bits_array("W6_CC_DGNSS_POSITION_BITS", &[p.x_m, p.y_m, p.z_m]);
    def_size("W6_CC_DGNSS_DROPPED", solution.dropped_sats.len());

    let antex =
        Antex::parse(&read(&tests_path("fixtures/antex/igs20_wettzell_trim.atx"))).expect("ANTEX");
    comment("ANTEX antennas.");
    def_size("W6_CC_ANTEX_ANTENNAS", antex.antennas.len());
    // sidereon_antex_encode writes Antex::encode's text; the test reads it
    // back with sidereon_antex_parse.
    let encoded = antex.encode().expect("ANTEX encode");
    let reread = Antex::parse(&encoded).expect("ANTEX re-read");
    comment("ANTEX encode: text length, its FNV-1a 64 hash, and the antennas read back.");
    def_size("W6_CC_ANTEX_ENCODED_LEN", encoded.len());
    def(
        "W6_CC_ANTEX_ENCODED_FNV1A",
        format!("UINT64_C({:#018x})", fnv1a64(encoded.as_bytes())),
    );
    def_size("W6_CC_ANTEX_REREAD_ANTENNAS", reread.antennas.len());

    let obs = RinexObs::parse(&read(&tests_path(
        "fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx",
    )))
    .expect("RINEX obs");
    let header = obs.header();
    comment("RINEX observation helpers, epoch 0.");
    def_bits("W6_CC_OBS_VERSION_BITS", header.version);
    let codes: Vec<&String> = header.obs_codes.values().flatten().collect();
    def_size("W6_CC_OBS_CODE_COUNT", codes.len());
    def_str("W6_CC_OBS_CODE0", codes[0]);
    def_size("W6_CC_OBS_EPOCH_COUNT", obs.epochs().len());
    let epoch = &obs.epochs()[0];
    def_size("W6_CC_OBS_EPOCH0_SATS", epoch.sats.len());
    let values: Vec<(String, String)> = observation_values(&obs, epoch, &ObservationFilter::all())
        .expect("values")
        .into_iter()
        .flat_map(|(sat, rows)| rows.into_iter().map(move |row| (sat.to_string(), row.code)))
        .collect();
    def_size("W6_CC_OBS_VALUE_COUNT", values.len());
    def_str("W6_CC_OBS_VALUE0_SAT", &values[0].0);
    let policy = SignalPolicy::default_for(header.version).expect("signal policy");
    let prs = pseudoranges(&obs, epoch, &policy).expect("pseudoranges");
    def_size("W6_CC_OBS_PR_COUNT", prs.len());
    def_str("W6_CC_OBS_PR0_SAT", &prs[0].0.to_string());
    def_bits("W6_CC_OBS_PR0_BITS", prs[0].1);
    let header0 = obs.header_at(0).expect("header at epoch 0");
    let phase: Vec<(String, String)> =
        carrier_phase_rows(&header0, epoch, &ObservationFilter::all())
            .expect("carrier phase")
            .into_iter()
            .flat_map(|(sat, rows)| rows.into_iter().map(move |row| (sat.to_string(), row.code)))
            .collect();
    def_size("W6_CC_OBS_PHASE_COUNT", phase.len());
    def_str("W6_CC_OBS_PHASE0_CODE", &phase[0].1);
    valgen::header_end(guard);
}

/// FNV-1a 64 of `bytes`, as core_caps_smoke.c computes it.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
