//! Engine values round2_smoke.c checks, written as tests/w4_r2_pins.h. Each
//! section builds the inputs the C test passes and calls the sidereon-core
//! function the C route calls (src/frame.rs, src/almanac.rs, src/broadcast.rs,
//! src/rinex.rs, src/iod.rs, src/combination.rs, src/tca.rs, src/signal.rs,
//! src/estimation.rs, src/raim.rs, src/spp.rs).

use sidereon_core::astro::frames::transforms as ft;
use sidereon_core::astro::frames::{nutation as ft_nutation, precession as ft_precession};
use sidereon_core::astro::tca as core_tca;
use sidereon_core::astro::time::scales::TimeScales;
use valgen::{bits, c_string_literals, header_end, header_start, read, read_bytes, tests_path};

const P: &str = "W4_R2_";
const GUARD: &str = "SIDEREON_W4_R2_PINS_H";

fn f(name: &str, value: f64) {
    println!("static const uint64_t {P}{name} = {};", bits(value));
}

fn fa(name: &str, values: &[f64]) {
    let joined: Vec<String> = values.iter().map(|v| bits(*v)).collect();
    println!(
        "static const uint64_t {P}{name}[{}] = {{ {} }};",
        values.len(),
        joined.join(", ")
    );
}

fn mat(name: &str, m: [[f64; 3]; 3]) {
    let flat: Vec<f64> = m.iter().flatten().copied().collect();
    fa(name, &flat);
}

fn int(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}{name} ({value})");
}

fn flag(name: &str, value: bool) {
    println!("#define {P}{name} {value}");
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash(name: &str, data: &[u8]) {
    println!("#define {P}{name} UINT64_C({:#018x})", fnv1a64(data));
}

/// The C enum constant for a core variant: `prefix` plus the variant's Debug
/// name in upper snake case.
fn variant(prefix: &str, value: impl std::fmt::Debug) -> String {
    let debug = format!("{value:?}");
    let name = debug
        .split(|c: char| c == '(' || c == '{' || c == ' ')
        .next()
        .expect("variant name");
    let mut out = String::from(prefix);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

fn frames() {
    println!("/* exercise_frames: sidereon_core::astro::frames::transforms. */");
    let ts = TimeScales::from_utc(2020, 6, 25, 12, 0, 0.0).expect("time scales");
    f("TS_JD_WHOLE_BITS", ts.jd_whole);
    f("TS_JD_TT_BITS", ts.jd_tt);
    mat(
        "GCRS_TO_ITRS_MATRIX_BITS",
        ft::gcrs_to_itrs_matrix(&ts).expect("gcrs->itrs matrix"),
    );
    mat(
        "ITRS_TO_GCRS_MATRIX_BITS",
        ft::itrs_to_gcrs_matrix(&ts).expect("itrs->gcrs matrix"),
    );
    let pole = ft::PolarMotion::from_arcseconds(0.0, 0.0).expect("polar motion");
    mat(
        "POLAR_MOTION_ZERO_BITS",
        ft::polar_motion_matrix(pole).expect("polar motion matrix"),
    );
    f(
        "GMST_BITS",
        ft::greenwich_mean_sidereal_time_radians(&ts).expect("GMST"),
    );
    f(
        "GAST_BITS",
        ft::greenwich_apparent_sidereal_time_radians(&ts).expect("GAST"),
    );
    let (x, y, z) =
        ft::gcrs_to_itrs_compute(7000.0, 1500.0, -2200.0, &ts, false).expect("gcrs->itrs");
    fa("GCRS_TO_ITRS_BITS", &[x, y, z]);
    let (bx, by, bz) = ft::itrs_to_gcrs_compute(x, y, z, &ts).expect("itrs->gcrs");
    fa("ITRS_TO_GCRS_BACK_BITS", &[bx, by, bz]);
    let (gx, gy, gz) = ft::geodetic_to_itrs(37.0, -122.0, 0.1).expect("geodetic->itrs");
    fa("GEODETIC_TO_ITRS_BITS", &[gx, gy, gz]);
    let (lat, lon, alt) = ft::itrs_to_geodetic_compute(gx, gy, gz).expect("itrs->geodetic");
    fa("ITRS_TO_GEODETIC_BITS", &[lat, lon, alt]);
    let state = ft::TemeStateKm {
        position_km: [-4000.0, 5000.0, 3000.0],
        velocity_km_s: [-3.0, -2.0, 6.0],
    };
    let (p, v) = ft::teme_to_gcrs_compute(&state, &ts, true).expect("teme->gcrs");
    fa("TEME_TO_GCRS_POSITION_BITS", &[p.0, p.1, p.2]);
    fa("TEME_TO_GCRS_VELOCITY_BITS", &[v.0, v.1, v.2]);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    fa(
        "MAT3_VEC3_IDENTITY_BITS",
        &ft::mat3_vec3_mul(&identity, &[1.5, -2.5, 3.5]).expect("mat3 vec3"),
    );
    let station = ft::GeodeticStationKm {
        latitude_deg: 37.0,
        longitude_deg: -122.0,
        altitude_km: 0.0,
    };
    let (az, el, range) =
        ft::gcrs_to_topocentric_compute([7000.0, 1500.0, -2200.0], &station, &ts, false)
            .expect("topocentric");
    fa("TOPOCENTRIC_BITS", &[az, el, range]);
    println!();
}

fn nutation() {
    println!("/* exercise_nutation_precession: sidereon_core::astro::frames nutation and");
    println!(" * precession. */");
    let (dpsi, deps) = ft_nutation::skyfield_iau2000a_radians(2459000.0).expect("nutation");
    f("NUTATION_DPSI_BITS", dpsi);
    f("NUTATION_DEPS_BITS", deps);
    let mean_ob = ft_nutation::skyfield_mean_obliquity_radians(2459000.0).expect("obliquity");
    f("MEAN_OBLIQUITY_BITS", mean_ob);
    let fa_values = ft_nutation::skyfield_fundamental_arguments(0.2).expect("arguments");
    fa("FUNDAMENTAL_ARGUMENTS_BITS", &fa_values);
    f(
        "EQUATION_OF_EQUINOXES_BITS",
        ft_nutation::skyfield_equation_of_the_equinoxes_complimentary_terms(2459000.0)
            .expect("equation of the equinoxes"),
    );
    mat(
        "NUTATION_MATRIX_BITS",
        ft_nutation::build_skyfield_nutation_matrix(mean_ob, mean_ob + deps, dpsi)
            .expect("nutation matrix"),
    );
    mat(
        "PRECESSION_MATRIX_BITS",
        ft_precession::compute_skyfield_precession_matrix(2459000.0).expect("precession"),
    );
    mat(
        "ICRS_TO_J2000_MATRIX_BITS",
        ft_precession::build_icrs_to_j2000(),
    );
    println!();
}

fn broadcast() {
    use sidereon_core::ephemeris::{
        satellite_clock_offset_s, satellite_position_ecef, ClockPolynomial, ConstellationConstants,
        KeplerianElements,
    };
    println!("/* exercise_broadcast_keplerian: sidereon_core::ephemeris. */");
    let consts = ConstellationConstants {
        gm_m3_s2: 3.986005e14,
        omega_e_rad_s: 7.2921151467e-5,
        dtr_f: -4.442807633e-10,
    };
    let el = KeplerianElements {
        sqrt_a: 5153.65,
        e: 0.005,
        m0: 0.3,
        delta_n: 4.5e-9,
        omega0: -1.0,
        i0: 0.96,
        omega: 0.5,
        omega_dot: -8.0e-9,
        idot: 1.0e-10,
        cuc: 0.0,
        cus: 0.0,
        crc: 0.0,
        crs: 0.0,
        cic: 0.0,
        cis: 0.0,
        toe_sow: 432000.0,
    };
    let orbit = satellite_position_ecef(&el, &consts, 432000.0, false).expect("orbit");
    fa("ORBIT_ECEF_BITS", &[orbit.x_m, orbit.y_m, orbit.z_m]);
    let clock = ClockPolynomial {
        af0: 1.0e-4,
        af1: 1.0e-11,
        af2: 0.0,
        toc_sow: 432000.0,
    };
    let offset = satellite_clock_offset_s(&clock, &consts, &el, orbit.sin_e, 432000.0, 3.0e-9)
        .expect("clock offset");
    f("CLOCK_TOTAL_BITS", offset.dt_clock_total_s);
    println!();
}

fn encode_nav() {
    println!("/* exercise_rinex_encode_nav: sidereon_core::rinex::nav::encode_nav of the");
    println!(" * store read from fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx. */");
    let text = read(&tests_path(
        "fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx",
    ));
    let store = sidereon_core::rinex::nav::BroadcastEphemeris::from_nav(&text).expect("NAV store");
    let encoded = sidereon_core::rinex::nav::encode_nav(store.records()).expect("encode NAV");
    int("ENCODED_NAV_LEN", encoded.len());
    hash("ENCODED_NAV_FNV1A64", encoded.as_bytes());
    println!();
}

fn iod() {
    println!("/* exercise_iod_gauss: sidereon_core::astro::iod::gauss_angles. */");
    let d2r = std::f64::consts::PI / 180.0;
    let decl = [18.667717 * d2r, 35.664741 * d2r, 36.996583 * d2r];
    let rtasc = [0.939913 * d2r, 45.025748 * d2r, 67.886655 * d2r];
    let jd = [2456159.5, 2456159.5, 2456159.5];
    let jdf = [0.4864351851851852, 0.49199074074074073, 0.4947685185185185];
    let rseci = [
        [4054.881, 2748.195, 4074.237],
        [3956.224, 2888.232, 4074.364],
        [3905.073, 2956.935, 4074.430],
    ];
    let (position, velocity) =
        sidereon_core::astro::iod::gauss_angles(&decl, &rtasc, &jd, &jdf, &rseci).expect("IOD");
    fa("IOD_POSITION_BITS", &position);
    fa("IOD_VELOCITY_BITS", &velocity);
    println!();
}

fn combination() {
    use sidereon_core::combinations::{ionosphere_free_phase_cycles, ionosphere_free_pseudoranges};
    println!("/* exercise_combination_and_covariance: sidereon_core::combinations and");
    println!(" * sidereon_core::astro::conjunction. */");
    f(
        "IONO_FREE_PHASE_BITS",
        ionosphere_free_phase_cycles(1.0e8, 0.9e8, 1575.42e6, 1227.6e6).expect("iono-free"),
    );
    let band1 = vec![("G01".to_string(), 2.0e7), ("G02".to_string(), 2.1e7)];
    let band2 = vec![("G01".to_string(), 2.0e7 + 5.0)];
    let (combined, dropped) =
        ionosphere_free_pseudoranges(&band1, &band2, &[]).expect("iono-free pseudoranges");
    int("IONO_FREE_COMBINED_COUNT", combined.len());
    int("IONO_FREE_DROPPED_COUNT", dropped.len());
    println!(
        "#define {P}IONO_FREE_DROPPED0_REASON {}",
        variant("SIDEREON_PSEUDORANGE_DROP_", dropped[0].1)
    );
    let frame = sidereon_core::astro::conjunction::encounter_frame(
        [7000.0, 0.0, 0.0],
        [0.0, 7.5, 0.0],
        [7000.05, 0.0, 0.5],
        [0.0, -7.5, 0.1],
    )
    .expect("encounter frame");
    let plane = sidereon_core::astro::conjunction::encounter_plane_covariance(
        &frame,
        &[[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
    )
    .expect("encounter plane covariance");
    fa(
        "ENCOUNTER_PLANE_COVARIANCE_BITS",
        &[plane[0][0], plane[0][1], plane[1][0], plane[1][1]],
    );
    println!();
}

fn signal() {
    use sidereon_core::signal::{
        acquire, autocorrelation, ca_code, correlate, correlate_against, correlation_at,
        cross_correlation, replica, AcquisitionOptions, CorrelateOptions, IqSample, ReplicaOptions,
    };
    println!("/* exercise_signal: sidereon_core::signal. */");
    let code = ca_code(1).expect("C/A code");
    int("CA_CODE_LEN", code.len());
    let code_bytes: Vec<u8> = code.iter().map(|c| *c as u8).collect();
    hash("CA_CODE_FNV1A64", &code_bytes);
    int("AUTOCORRELATION_0", autocorrelation(&code)[0]);
    int(
        "CORRELATION_AT_0",
        correlation_at(&code, &code, 0).expect("correlation"),
    );
    int(
        "CROSS_CORRELATION_0",
        cross_correlation(&code, &code).expect("cross correlation")[0],
    );
    let rep = replica(1, ReplicaOptions::new(2.046e6, 2046, 0.0, 0.0)).expect("replica");
    let iq: Vec<IqSample> = rep
        .iter()
        .map(|chip| IqSample::new(f64::from(*chip), 0.0))
        .collect();
    let (ci, cq) = correlate_against(&iq, &rep, 2.046e6, 0.0).expect("correlate against");
    f("CORRELATE_AGAINST_I_BITS", ci);
    f("CORRELATE_AGAINST_Q_BITS", cq);
    let mut copts = CorrelateOptions::default();
    copts.sample_rate_hz = 2.046e6;
    copts.doppler_hz = 0.0;
    copts.code_phase_chips = 0.0;
    copts.code_doppler_hz = 0.0;
    let result = correlate(&iq, 1, copts).expect("correlate");
    f("CORRELATE_POWER_BITS", result.power);
    let mut aopts = AcquisitionOptions::default();
    aopts.sample_rate_hz = 2.046e6;
    aopts.doppler_min_hz = -1000.0;
    aopts.doppler_max_hz = 1000.0;
    aopts.doppler_step_hz = 500.0;
    let acquisition = acquire(&iq, 1, aopts).expect("acquire");
    int("ACQUIRE_BIN_COUNT", acquisition.grid.doppler_hz.len());
    f("ACQUIRE_PEAK_POWER_BITS", acquisition.peak_power);
    f("ACQUIRE_CODE_PHASE_BITS", acquisition.code_phase_chips);
    f("ACQUIRE_DOPPLER_BITS", acquisition.doppler_hz);
    println!();
}

fn quality() {
    use sidereon_core::ephemeris::Sp3;
    use sidereon_core::positioning::{
        solve_with_policy, Corrections, KlobucharCoeffs, Observation, SolveInputs, SolvePolicy,
        SurfaceMet,
    };
    use sidereon_core::quality::{PseudorangeVarianceOptions, WeightEntry};
    println!("/* exercise_quality: sidereon_core::quality sigmas, weight_vector,");
    println!(" * raim_for_solution and validate_receiver_solution on the spp_fixture.h");
    println!(" * inputs (the tests/sppgen solve). */");
    let entries = vec![
        WeightEntry {
            satellite_id: "G01".to_string(),
            elevation_deg: 30.0,
            cn0_dbhz: None,
        },
        WeightEntry {
            satellite_id: "G02".to_string(),
            elevation_deg: 90.0,
            cn0_dbhz: None,
        },
    ];
    let options = PseudorangeVarianceOptions::default();
    let sigmas = sidereon_core::quality::sigmas(&entries, options);
    let weights = sidereon_core::quality::weight_vector(&entries, options);
    for (i, entry) in entries.iter().enumerate() {
        let sigma = sigmas.get(&entry.satellite_id);
        flag(&format!("SIGMA{i}_PRESENT"), sigma.is_some());
        f(&format!("SIGMA{i}_BITS"), sigma.copied().unwrap_or(0.0));
        let weight = weights.get(&entry.satellite_id);
        flag(&format!("WEIGHT{i}_PRESENT"), weight.is_some());
        f(&format!("WEIGHT{i}_BITS"), weight.copied().unwrap_or(0.0));
    }

    // The SPP inputs round2_smoke.c reads from spp_fixture.h, which
    // tests/gen_fixture_header.py copies from the same fields of the core
    // fixture spp_trace_L0_minimal.json read here: observations (sat_id,
    // p_meas_m), t_rx_j2000_s, t_rx_sod_s, doy, frozen.initial_guess_x0,
    // klobuchar_alpha/beta and met. The C test turns both corrections off,
    // requests geodetic output and keeps the sidereon_spp_inputs_v2_init
    // defaults (no BeiDou Klobuchar, no robust weighting, no GLONASS
    // channels, single-frequency code, default validation, no coarse seeds),
    // which SolveInputs::default and SolvePolicy::default give. The SP3 is the
    // file the test passes as argv[1], read as sidereon_sp3_load reads it.
    let text = read(&format!(
        "{}/spp_trace_L0_minimal.json",
        valgen::core_fixtures()
    ));
    let root: serde_json::Value = serde_json::from_str(&text).expect("SPP fixture");
    let fixture = &root["fixture"];
    let inputs = &fixture["inputs"];
    let hex = |value: &serde_json::Value| {
        let digits = value.as_str().expect("hex").strip_prefix("0x").expect("0x");
        f64::from_bits(u64::from_str_radix(digits, 16).expect("bits"))
    };
    let four = |value: &serde_json::Value| {
        let values: Vec<f64> = value.as_array().expect("array").iter().map(hex).collect();
        let array: [f64; 4] = values.try_into().expect("four values");
        array
    };
    let sp3_file = inputs["sp3_file"].as_str().expect("sp3_file");
    let sp3 = Sp3::parse(&read_bytes(&tests_path(&format!(
        "fixtures/sp3/{sp3_file}"
    ))))
    .expect("SP3")
    .with_interpolation_options(sidereon_core::ephemeris::Sp3InterpolationOptions::default());
    let mut solve_inputs = SolveInputs::default();
    solve_inputs.observations = inputs["observations"]
        .as_array()
        .expect("observations")
        .iter()
        .map(|obs| Observation {
            satellite_id: obs["sat_id"].as_str().expect("sat").parse().expect("token"),
            pseudorange_m: hex(&obs["p_meas_m"]),
        })
        .collect();
    solve_inputs.t_rx_j2000_s = hex(&inputs["t_rx_j2000_s"]);
    solve_inputs.t_rx_second_of_day_s = hex(&inputs["t_rx_sod_s"]);
    solve_inputs.day_of_year = hex(&inputs["doy"]);
    solve_inputs.initial_guess = four(&fixture["frozen"]["initial_guess_x0"]);
    solve_inputs.corrections = Corrections::NONE;
    solve_inputs.klobuchar = KlobucharCoeffs {
        alpha: four(&inputs["klobuchar_alpha"]),
        beta: four(&inputs["klobuchar_beta"]),
    };
    solve_inputs.met = SurfaceMet {
        pressure_hpa: hex(&inputs["met"]["pressure_hpa"]),
        temperature_k: hex(&inputs["met"]["temperature_k"]),
        relative_humidity: hex(&inputs["met"]["relative_humidity"]),
    };
    let solution =
        solve_with_policy(&sp3, &solve_inputs, true, SolvePolicy::default()).expect("SPP solve");
    let mut raim_options = sidereon_core::quality::RaimOptions::default();
    raim_options.p_fa = 0.001;
    raim_options.weights = sidereon_core::quality::RaimWeights::Unit;
    raim_options.n_systems = None;
    let raim = sidereon_core::quality::raim_for_solution(&solution, &raim_options).expect("RAIM");
    f("RAIM_TEST_STATISTIC_BITS", raim.test_statistic);
    let valid = sidereon_core::quality::validate_receiver_solution(
        &solution,
        sidereon_core::quality::SolutionValidationOptions::default(),
    );
    flag("SOLUTION_VALID", valid.is_ok());
    println!();
}

fn tca() {
    println!("/* exercise_tca: sidereon_core::astro::tca on the test's TLE pair. */");
    let source = tests_path("round2_smoke.c");
    let a1 = c_string_literals(&source, "static const char *const TLE_A1 =");
    let a2 = c_string_literals(&source, "static const char *const TLE_A2 =");
    let b1 = c_string_literals(&source, "static const char *const TLE_B1 =");
    let b2 = c_string_literals(&source, "static const char *const TLE_B2 =");
    let start = sidereon_core::astro::sgp4::JulianDate(2459023.0, 0.0);
    let end = sidereon_core::astro::sgp4::JulianDate(2459023.0, 1.0);
    let finder = core_tca::TcaFinderOptions::default();
    let candidates =
        core_tca::find_tca_candidates_from_tles(&a1, &a2, &b1, &b2, start, end, finder)
            .expect("TCA candidates");
    int("CANDIDATE_COUNT", candidates.len());
    let pc = core_tca::TcaPcOptions::with_default_covariance(
        0.02,
        sidereon_core::astro::conjunction::PcMethod::FosterEqualArea,
    );
    if let Some(first) = candidates.first() {
        f("CANDIDATE0_MISS_DISTANCE_BITS", first.miss_distance_km);
        let conjunction =
            core_tca::tca_collision_probability(*first, pc.clone()).expect("collision Pc");
        f("CANDIDATE0_PC_BITS", conjunction.collision_probability.pc);
    }
    let conjunctions = core_tca::find_tca_conjunctions_from_tles(
        core_tca::TcaTle::new(&a1, &a2),
        core_tca::TcaTle::new(&b1, &b2),
        start,
        end,
        finder,
        pc.clone(),
    )
    .expect("TCA conjunctions");
    int("CONJUNCTION_COUNT", conjunctions.len());
    let secondaries = [core_tca::TcaTle::new(&b1, &b2)];
    let window = core_tca::TcaWindow::new(start, end);
    let hits = core_tca::screen_tca_candidates_from_tle_catalog_serial(
        core_tca::TcaTle::new(&a1, &a2),
        &secondaries,
        window,
        1000.0,
        finder,
    )
    .expect("screen candidates");
    int("SCREEN_CANDIDATE_COUNT", hits.len());
    let conjunction_hits = core_tca::screen_tca_conjunctions_from_tle_catalog_serial(
        core_tca::TcaTle::new(&a1, &a2),
        &secondaries,
        window,
        1000.0,
        finder,
        pc,
    )
    .expect("screen conjunctions");
    int("SCREEN_CONJUNCTION_COUNT", conjunction_hits.len());

    // The propagated-covariance finder with the test's options
    // (src/tca.rs sidereon_find_tca_conjunctions_with_propagated_covariance_from_tles).
    let mut config =
        sidereon_core::astro::propagator::PropagationConfig::new(0.0, [0.0; 3], [0.0; 3]);
    config.force_model = sidereon_core::astro::propagator::PropagationForceModel::TwoBodyJ2;
    config.mu_km3_s2 = None;
    let mut covariance = [[0.0; 6]; 6];
    for (i, row) in covariance.iter_mut().enumerate() {
        row[i] = 1.0e-2;
    }
    let covariance0 = sidereon_core::astro::covariance::Covariance6::try_from_matrix(covariance)
        .expect("covariance");
    let mut integrator_options = sidereon_core::astro::propagator::IntegratorOptions::default();
    integrator_options.abs_tol = 1.0e-9;
    integrator_options.rel_tol = 1.0e-9;
    integrator_options.initial_step = 1.0;
    integrator_options.min_step = 1.0e-3;
    integrator_options.max_step = 60.0;
    integrator_options.max_steps = 100000;
    integrator_options.dense_output = false;
    let mut prop_options = core_tca::TcaPropagatedCovariancePcOptions::new(
        0.02,
        sidereon_core::astro::conjunction::PcMethod::FosterEqualArea,
        covariance0,
        covariance0,
    );
    prop_options.force_model = config.force_model_kind();
    prop_options.integrator = sidereon_core::astro::propagator::IntegratorKind::Dp54;
    prop_options.integrator_options = integrator_options;
    prop_options.process_noise = sidereon_core::astro::propagator::ProcessNoise::None;
    let propagated = core_tca::find_tca_conjunctions_with_propagated_covariance_from_tles(
        core_tca::TcaTle::new(&a1, &a2),
        core_tca::TcaTle::new(&b1, &b2),
        start,
        end,
        finder,
        prop_options,
    );
    flag("PROPAGATED_OK", propagated.is_ok());
    int(
        "PROPAGATED_CONJUNCTION_COUNT",
        propagated.as_ref().map_or(0, Vec::len),
    );
    println!();
}

fn main() {
    header_start("w4_r2", GUARD);
    frames();
    nutation();
    broadcast();
    encode_nav();
    iod();
    combination();
    signal();
    quality();
    tca();
    header_end(GUARD);
}
