//! Engine values domain018_smoke.c checks, written as tests/w4_d018_pins.h.
//! Each section builds the inputs the C test passes and calls the
//! sidereon-core function the C route calls, with the configuration the
//! binding forms from the C structs (src/fusion.rs, src/signal.rs,
//! src/scenario.rs).

use sidereon_core::fusion as fu;
use sidereon_core::signal::analysis as sa;
use valgen::{bits, c_string_literals, header_end, header_start, tests_path};

const P: &str = "W4_D018_";
const GUARD: &str = "SIDEREON_W4_D018_PINS_H";

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

fn int(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}{name} ({value})");
}

fn flag(name: &str, value: bool) {
    println!("#define {P}{name} {value}");
}

/// FNV-1a 64 of the little-endian bytes of `values`, the hash the C test
/// recomputes over the doubles a route copies out.
fn hash_f64s(name: &str, values: &[f64]) {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for value in values {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    println!("#define {P}{name} UINT64_C({hash:#018x})");
}

fn hash_bytes(name: &str, data: &[u8]) {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    println!("#define {P}{name} UINT64_C({hash:#018x})");
}

fn flatten(matrix: &[Vec<f64>]) -> Vec<f64> {
    matrix.iter().flatten().copied().collect()
}

fn signal() {
    println!("/* test_signal_analysis: sidereon_core::signal::analysis. */");
    let bpsk = sa::SignalModulation::bpsk(1.0).expect("BPSK(1)");
    let boc = sa::SignalModulation::boc_sine(1.0, 1.0).expect("BOCsin(1,1)");
    let boc_cosine = sa::SignalModulation::boc_cosine(10.0, 5.0).expect("BOCcos(10,5)");
    let mboc = sa::SignalModulation::mboc_6_1_1_over_11();
    let tmboc = sa::SignalModulation::tmboc_6_1_4_over_33();
    let cboc_plus = sa::SignalModulation::cboc_6_1_1_over_11(sa::CbocSign::Plus);
    let cboc_minus = sa::SignalModulation::cboc_6_1_1_over_11(sa::CbocSign::Minus);
    f("REFERENCE_CHIP_RATE_BITS", sa::REFERENCE_CHIP_RATE_HZ);
    f("BETZ_L1_BANDWIDTH_BITS", sa::BETZ_L1_RECEIVER_BANDWIDTH_HZ);
    let label = bpsk.label();
    println!(
        "static const char {P}BPSK_LABEL[] = {};",
        valgen::c_string(label)
    );
    f(
        "BPSK_CODE_RATE_BITS",
        bpsk.code_rate_hz().expect("code rate"),
    );
    let half = 0.5 * 1023000.0;
    f("PSD_BPSK_0_BITS", bpsk.psd_hz(0.0).expect("PSD"));
    f("PSD_BOC_HALF_BITS", boc.psd_hz(half).expect("PSD"));
    f(
        "PSD_BOC_COSINE_HALF_BITS",
        boc_cosine.psd_hz(half).expect("PSD"),
    );
    f("PSD_MBOC_HALF_BITS", mboc.psd_hz(half).expect("PSD"));
    f("PSD_TMBOC_HALF_BITS", tmboc.psd_hz(half).expect("PSD"));
    f(
        "PSD_CBOC_PLUS_HALF_BITS",
        cboc_plus.psd_hz(half).expect("PSD"),
    );
    f(
        "PSD_CBOC_MINUS_HALF_BITS",
        cboc_minus.psd_hz(half).expect("PSD"),
    );
    let bw = 24000000.0;
    f(
        "FRACTION_POWER_BITS",
        sa::fraction_power_in_band(&bpsk, bw).expect("fraction power"),
    );
    f(
        "POWER_IN_BAND_BITS",
        sa::power_in_band(&bpsk, bw).expect("power in band"),
    );
    f(
        "RMS_BANDWIDTH_BITS",
        sa::rms_bandwidth_hz(&boc, bw).expect("RMS bandwidth"),
    );
    f(
        "SSC_HZ_BITS",
        sa::spectral_separation_coefficient_hz(&bpsk, &boc, bw).expect("SSC"),
    );
    f(
        "SSC_DB_HZ_BITS",
        sa::spectral_separation_coefficient_db_hz(&bpsk, &boc, bw).expect("SSC dB"),
    );
    f(
        "WHITE_NOISE_SSC_BITS",
        sa::white_noise_spectral_separation_hz(&bpsk, bw).expect("white-noise SSC"),
    );
    let interference = [sa::InterferenceTerm::new(boc.clone(), 0.01)];
    let degradation =
        sa::effective_cn0_degradation(&bpsk, 45.0, bw, &interference).expect("C/N0 degradation");
    f("EFFECTIVE_CN0_HZ_BITS", degradation.effective_cn0_hz);
    f("EFFECTIVE_CN0_DB_HZ_BITS", degradation.effective_cn0_db_hz);
    f("CN0_DEGRADATION_DB_BITS", degradation.degradation_db);
    let dll = sa::DllTrackingOptions::new(45.0, 1.0, 0.02, 0.5, 100000000.0);
    let jitter =
        sa::dll_thermal_noise_jitter(&bpsk, dll, sa::DllProcessing::Coherent).expect("DLL jitter");
    f("DLL_JITTER_CHIPS_BITS", jitter.chips);
    f("DLL_JITTER_METERS_BITS", jitter.meters);
    let bound = sa::dll_lower_bound(&boc, dll).expect("DLL lower bound");
    f("DLL_LOWER_BOUND_SECONDS_BITS", bound.seconds);
    let mp = sa::MultipathOptions::new(0.5, 1.0, 100000000.0);
    let points = sa::multipath_error_envelope(&bpsk, mp, &[0.0, 0.5, 1.0]).expect("envelope");
    int("MULTIPATH_POINT_COUNT", points.len());
    f("MULTIPATH_1_IN_PHASE_BITS", points[1].in_phase_chips);
    f("MULTIPATH_1_ANTI_PHASE_BITS", points[1].anti_phase_chips);
    f(
        "MULTIPATH_1_RUNNING_AVERAGE_BITS",
        points[1].running_average_chips,
    );
    println!();
}

fn scenario() {
    let json = c_string_literals(
        &tests_path("domain018_smoke.c"),
        "static const char *scenario_json =",
    );
    let scenario: sidereon_core::scenario::Scenario =
        serde_json::from_str(&json).expect("scenario JSON");
    let set = sidereon_core::scenario::simulate_scenario(&scenario).expect("simulate");
    // sidereon_scenario_simulate_json serializes the output with serde_json.
    let bytes = serde_json::to_vec(&set).expect("scenario output JSON");
    println!("/* test_scenario: sidereon_core::scenario::simulate_scenario of the test's JSON. */");
    int("SCENARIO_OBSERVATION_COUNT", set.observation_count());
    int(
        "SCENARIO_EPOCH_OFFSET_COUNT",
        set.observations.epoch_offsets.len(),
    );
    println!(
        "#define {P}SCENARIO_FINGERPRINT UINT64_C({:#018x})",
        set.determinism_fingerprint()
    );
    int("SCENARIO_JSON_LEN", bytes.len());
    hash_bytes("SCENARIO_JSON_FNV1A64", &bytes);
    println!();
}

/// The C-side SidereonFusionFilterConfig fields the test sets, with the values
/// sidereon_fusion_filter_config_init writes.
#[derive(Clone)]
struct CConfig {
    filter_kind: fu::FusionFilterKind,
    imu_spec: fu::ImuSpec,
    imu_to_body_dcm: [[f64; 3]; 3],
    loose_fix_weighting: fu::GnssFixStatusWeighting,
    innovation_gate: Option<fu::InnovationGate>,
    reweighting: Option<fu::IggIiiMeasurementReweighting>,
    adaptation: Option<fu::YangPredictionAdaptiveFactor>,
    stationary: Option<(usize, f64, f64, f64, f64)>,
    non_holonomic: Option<(f64, f64, f64, f64)>,
    time_sync_imu_capacity: usize,
    time_sync_checkpoint_capacity: usize,
}

/// The ImuSpec fields as the C struct carries them, rebuilt as
/// imu_spec_from_c rebuilds them.
fn spec_through_c(spec: fu::ImuSpec) -> fu::ImuSpec {
    fu::ImuSpec::datasheet(
        spec.accel_vrw_mps_sqrt_s,
        spec.gyro_arw_rad_sqrt_s,
        spec.accel_bias_instab_mps2,
        spec.gyro_bias_instab_rps,
        spec.accel_bias_tau_s,
        spec.gyro_bias_tau_s,
        spec.accel_scale_instab_ppm,
        spec.gyro_scale_instab_ppm,
    )
}

impl CConfig {
    /// sidereon_fusion_filter_config_init.
    fn init() -> Self {
        let loose = fu::LooseCouplingConfig::default();
        Self {
            filter_kind: fu::FusionFilterKind::Ekf,
            imu_spec: spec_through_c(fu::ImuSpec::preset(fu::ImuGrade::Mems)),
            imu_to_body_dcm: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            loose_fix_weighting: loose.fix_status_weighting,
            innovation_gate: None,
            reweighting: None,
            adaptation: None,
            stationary: None,
            non_holonomic: None,
            time_sync_imu_capacity: fu::DEFAULT_TIME_SYNC_IMU_CAPACITY,
            time_sync_checkpoint_capacity: fu::DEFAULT_TIME_SYNC_CHECKPOINT_CAPACITY,
        }
    }

    /// domain018_smoke.c init_field_config: the defaults with a zero IMU
    /// noise specification.
    fn field() -> Self {
        let mut config = Self::init();
        config.imu_spec =
            fu::ImuSpec::datasheet(0.0, 0.0, 0.0, 0.0, f64::INFINITY, f64::INFINITY, None, None);
        config
    }

    /// fusion_filter_config_from_c.
    fn to_core(&self) -> fu::InertialFilterConfig {
        let tight = fu::TightCouplingConfig::default();
        let ukf = fu::UkfUpdateOptions::default();
        let mut config = fu::InertialFilterConfig::new(self.imu_spec).expect("filter config");
        config.filter_kind = self.filter_kind;
        config.imu_model = fu::ImuErrorModel {
            bias: fu::ImuBias {
                accel_mps2: [0.0; 3],
                gyro_rps: [0.0; 3],
            },
            calibration: fu::ImuCalibration {
                accel_scale_misalignment: [[0.0; 3]; 3],
                gyro_scale_misalignment: [[0.0; 3]; 3],
            },
        };
        config.imu_to_body_dcm = self.imu_to_body_dcm;
        let mut mechanization = fu::MechanizationConfig::default();
        mechanization.coning_correction = fu::ConingCorrection::Off;
        config.mechanization = mechanization;
        config.loose.lever_arm_body_m = [0.0; 3];
        config.loose.fix_status_weighting = self.loose_fix_weighting;
        config.loose.update_options.innovation_gate = self.innovation_gate;
        config.loose.measurement_reweighting = self.reweighting;
        config.loose.prediction_adaptation = self.adaptation;
        config.loose.stationary_updates =
            self.stationary
                .map(|(window, force, rate, zero_velocity, zero_rate)| {
                    fu::StationaryUpdateConfig::new(
                        fu::StationaryDetectorConfig::new(window, force, rate),
                        zero_velocity,
                        zero_rate,
                    )
                });
        config.loose.non_holonomic = self.non_holonomic.map(|(lateral, vertical, speed, rate)| {
            fu::NonHolonomicConstraintConfig::new(lateral, vertical, speed, rate)
        });
        config.tight.lever_arm_body_m = tight.lever_arm_body_m;
        config.tight.light_time = tight.light_time;
        config.tight.sagnac = tight.sagnac;
        config.tight.initial_clock_bias_variance_m2 = tight.initial_clock_bias_variance_m2;
        config.tight.initial_clock_drift_variance_m2_s2 = tight.initial_clock_drift_variance_m2_s2;
        config.tight.clock_bias_random_walk_m2_s = tight.clock_bias_random_walk_m2_s;
        config.tight.clock_drift_random_walk_m2_s3 = tight.clock_drift_random_walk_m2_s3;
        config.tight.update_options.innovation_gate = None;
        let mut transform = fu::UnscentedTransformOptions::default();
        transform.alpha = ukf.transform.alpha;
        transform.beta = ukf.transform.beta;
        transform.kappa = ukf.transform.kappa;
        config.ukf_update_options.transform = transform;
        config.ukf_update_options.innovation_gate = None;
        config.validate().expect("valid filter config");
        config
    }
}

/// domain018_smoke.c init_nav: the zeroed nav state at 6378137 m on x with
/// identity attitude.
fn nav(velocity: [f64; 3]) -> fu::NavState {
    fu::NavState::new(
        0.0,
        [6378137.0, 0.0, 0.0],
        velocity,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    )
    .expect("nav state")
    .with_biases([0.0; 3], [0.0; 3])
    .expect("nav biases")
}

/// sidereon_fusion_filter_create with a constant covariance diagonal.
fn create(config: &CConfig, nav: fu::NavState, diag: f64) -> fu::InertialFilter {
    let layout = fu::ErrorStateLayout::Fifteen;
    let diagonal = vec![diag; 15];
    let mut state = fu::InsFilterState::from_diagonal(nav, layout, &diagonal).expect("state");
    state.accel_scale_factor = [0.0; 3];
    state.gyro_scale_factor = [0.0; 3];
    state.validate().expect("valid state");
    let mut filter =
        fu::InertialFilter::with_config(state, config.to_core()).expect("inertial filter");
    filter
        .configure_time_sync_history(fu::TimeSyncHistoryConfig::new(
            config.time_sync_imu_capacity,
            config.time_sync_checkpoint_capacity,
        ))
        .expect("time sync history");
    filter
}

fn identity(dimension: usize) -> Vec<Vec<f64>> {
    (0..dimension)
        .map(|i| {
            (0..dimension)
                .map(|j| if i == j { 1.0 } else { 0.0 })
                .collect()
        })
        .collect()
}

fn fix(
    t: f64,
    position: [f64; 3],
    velocity: Option<[f64; 3]>,
    covariance: Vec<Vec<f64>>,
    satellites: usize,
    status: fu::GnssFixStatus,
) -> fu::GnssFixMeasurement {
    let measurement = fu::GnssFixMeasurement {
        t_j2000_s: t,
        position_ecef_m: position,
        velocity_ecef_mps: velocity,
        covariance,
        satellites_used: satellites,
        solution_valid: true,
        fix_status: status,
    };
    measurement.validate().expect("valid fix");
    measurement
}

/// init_default_position_velocity_fix.
fn default_fix(status: fu::GnssFixStatus) -> fu::GnssFixMeasurement {
    fix(
        0.0,
        [6378138.0, 2.0, -3.0],
        Some([0.4, -0.2, 0.1]),
        identity(6),
        8,
        status,
    )
}

fn update(prefix: &str, update: &fu::FusionUpdate) {
    flag(&format!("{prefix}_APPLIED"), update.applied);
    int(&format!("{prefix}_ROWS"), update.rows);
    int(&format!("{prefix}_ACCEPTED_ROWS"), update.accepted_rows);
    int(&format!("{prefix}_REJECTED_ROWS"), update.rejected_rows);
    f(&format!("{prefix}_NIS_BITS"), update.nis);
}

fn cov_diag(filter: &fu::InertialFilter, offset: usize, count: usize) -> Vec<f64> {
    let covariance = &filter.state().covariance;
    (offset..offset + count).map(|i| covariance[i][i]).collect()
}

fn fusion() {
    println!("/* test_fusion_labels: ErrorStateLayout::dimension. */");
    let fifteen = fu::ErrorStateLayout::Fifteen.dimension();
    int("FIFTEEN_COVARIANCE_LEN", fifteen * fifteen);
    int(
        "LAYOUT_TWENTY_ONE_DIMENSION",
        fu::ErrorStateLayout::TwentyOne.dimension(),
    );
    println!();

    println!("/* test_fusion: EKF, two IMU increments, a late loose fix. */");
    let mut config = CConfig::init();
    config.time_sync_imu_capacity = 8;
    config.time_sync_checkpoint_capacity = 4;
    let mut filter = create(&config, nav([0.0; 3]), 10.0);
    let mut ukf_config = config.clone();
    ukf_config.filter_kind = fu::FusionFilterKind::Ukf;
    let _ukf = create(&ukf_config, nav([0.0; 3]), 10.0);
    let sample = |t: f64| fu::ImuSample::increment(t, [9.7803253359, 0.0, 0.0], [0.0; 3], 1.0);
    filter.propagate(sample(1.0)).expect("propagate first");
    let state1 = filter.state().nominal.position_ecef_m;
    filter.propagate(sample(2.0)).expect("propagate second");
    let big = vec![
        vec![1.0e12, 0.0, 0.0],
        vec![0.0, 1.0e12, 0.0],
        vec![0.0, 0.0, 1.0e12],
    ];
    let late = fix(1.5, state1, None, big, 4, fu::GnssFixStatus::Single);
    let sync = filter
        .update_loose_time_sync(&late)
        .expect("time sync update");
    int("SYNC_ROWS", sync.update.rows);
    int("SYNC_REPLAYED_IMU_SEGMENTS", sync.replayed_imu_segments);
    flag("SYNC_LATE_MEASUREMENT", sync.late_measurement);
    f("SYNC_NIS_BITS", sync.update.nis);
    f("SYNC_CURRENT_EPOCH_BITS", sync.current_epoch_j2000_s);
    let state = filter.state();
    fa("SYNC_STATE_POSITION_BITS", &state.nominal.position_ecef_m);
    let clock = filter.tight_clock_state().expect("tight clock state");
    f("SYNC_STATE_CLOCK_BIAS_BITS", clock.bias_m);
    f("SYNC_STATE_CLOCK_DRIFT_BITS", clock.drift_m_s);
    int("SYNC_STATE_COVARIANCE_DIMENSION", state.dimension());
    let encoded = filter.encode_state().expect("encode state");
    int("ENCODED_STATE_LEN", encoded.len());
    hash_bytes("ENCODED_STATE_FNV1A64", &encoded);
    let status = filter.time_sync_history_status();
    int("TIME_SYNC_IMU_CAPACITY", status.imu_capacity);
    int("TIME_SYNC_CHECKPOINT_CAPACITY", status.checkpoint_capacity);
    println!();

    println!("/* test_fusion_field_mode. */");
    let defaults = CConfig::init();
    f(
        "DEFAULT_SINGLE_SIGMA_MULTIPLIER_BITS",
        defaults.loose_fix_weighting.single_sigma_multiplier,
    );
    f(
        "DEFAULT_FLOAT_SIGMA_MULTIPLIER_BITS",
        defaults.loose_fix_weighting.float_sigma_multiplier,
    );
    f(
        "DEFAULT_FIXED_SIGMA_MULTIPLIER_BITS",
        defaults.loose_fix_weighting.fixed_sigma_multiplier,
    );
    int(
        "DEFAULT_TIME_SYNC_IMU_CAPACITY",
        defaults.time_sync_imu_capacity,
    );

    let mut filter = create(&CConfig::field(), nav([0.0; 3]), 1.0);
    let u = filter
        .update_loose(&default_fix(fu::GnssFixStatus::Single))
        .expect("field loose update");
    update("FIELD_UPDATE", &u);
    fa(
        "FIELD_POSITION_BITS",
        &filter.state().nominal.position_ecef_m,
    );
    fa(
        "FIELD_VELOCITY_BITS",
        &filter.state().nominal.velocity_ecef_mps,
    );
    fa("FIELD_COV_DIAG_BITS", &cov_diag(&filter, 0, 6));

    let mut stationary = CConfig::field();
    stationary.stationary = Some((1, 100.0, 1.0, 0.5, 0.05));
    let mut filter = create(&stationary, nav([0.0; 3]), 1.0);
    filter
        .propagate(fu::ImuSample::increment(1.0, [0.0; 3], [0.0; 3], 1.0))
        .expect("stationary propagate");
    let u = filter.update_stationary().expect("stationary update");
    flag("STATIONARY_PRESENT", u.is_some());
    update("STATIONARY_UPDATE", &u.expect("stationary update applied"));
    fa(
        "STATIONARY_VELOCITY_BITS",
        &filter.state().nominal.velocity_ecef_mps,
    );
    fa(
        "STATIONARY_GYRO_BIAS_BITS",
        &filter.state().nominal.gyro_bias_rps,
    );
    fa("STATIONARY_COV_DIAG_BITS", &cov_diag(&filter, 3, 6));

    let mut plain = create(&CConfig::field(), nav([0.0; 3]), 1.0);
    flag(
        "PLAIN_STATIONARY_PRESENT",
        plain
            .update_stationary()
            .expect("plain stationary")
            .is_some(),
    );

    let mut weighting = CConfig::field();
    weighting.loose_fix_weighting = fu::GnssFixStatusWeighting {
        single_sigma_multiplier: 3.0,
        float_sigma_multiplier: 2.0,
        fixed_sigma_multiplier: 1.0,
    };
    let statuses = [
        ("SINGLE", fu::GnssFixStatus::Single),
        ("FLOAT", fu::GnssFixStatus::Float),
        ("FIXED", fu::GnssFixStatus::Fixed),
    ];
    for (name, status) in statuses {
        let mut filter = create(&weighting, nav([0.0; 3]), 1.0);
        let u = filter
            .update_loose(&default_fix(status))
            .expect("weighting update");
        flag(&format!("WEIGHTING_{name}_APPLIED"), u.applied);
        int(&format!("WEIGHTING_{name}_ROWS"), u.rows);
        f(&format!("WEIGHTING_{name}_NIS_BITS"), u.nis);
        fa(
            &format!("WEIGHTING_{name}_COV_DIAG_BITS"),
            &cov_diag(&filter, 0, 6),
        );
    }

    let states = [
        (0.0, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        (1.0, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        (2.0, [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
    ]
    .map(|(t, p, v)| fu::VelocityMatchState::new(t, p, v).expect("velocity match state"));
    let first_good = fix(
        2.0,
        [4.0, 1.0, 0.0],
        Some([2.0, 0.0, 0.0]),
        identity(6),
        8,
        fu::GnssFixStatus::Single,
    );
    let matched =
        fu::velocity_match_outage(&states, &first_good, fu::VelocityMatchingConfig::new(5.0))
            .expect("velocity matching");
    int("VM_STATE_COUNT", matched.states.len());
    fa(
        "VM_ENDPOINT_POSITION_BITS",
        &matched.endpoint_position_correction_ecef_m,
    );
    fa(
        "VM_ENDPOINT_VELOCITY_BITS",
        &matched.endpoint_velocity_correction_ecef_mps,
    );
    for (i, state) in matched.states.iter().enumerate() {
        fa(
            &format!("VM_STATE{i}_POSITION_BITS"),
            &state.position_ecef_m,
        );
        fa(
            &format!("VM_STATE{i}_VELOCITY_BITS"),
            &state.velocity_ecef_mps,
        );
    }

    let mut nhc = CConfig::field();
    nhc.imu_to_body_dcm = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    nhc.non_holonomic = Some((0.5, 0.5, 0.1, 1.0));
    let mut filter = create(&nhc, nav([2.0, 0.4, -0.2]), 1.0);
    let u = filter.update_non_holonomic().expect("non-holonomic update");
    flag("NHC_PRESENT", u.is_some());
    update("NHC_UPDATE", &u.expect("non-holonomic update applied"));
    fa(
        "NHC_VELOCITY_BITS",
        &filter.state().nominal.velocity_ecef_mps,
    );
    println!();
}

fn recorded_rts() {
    println!("/* test_fusion_recorded_rts: recorded history and RTS smoothing. */");
    let mut config = CConfig::init();
    config.innovation_gate = Some(fu::InnovationGate {
        threshold_sigma: 4.0,
        min_rows: 2,
    });
    config.reweighting = Some(fu::IggIiiMeasurementReweighting {
        k0_sigma: 2.0,
        k1_sigma: 5.0,
    });
    config.adaptation = Some(fu::YangPredictionAdaptiveFactor {
        threshold: 1.0,
        outlier_gate_probability: 0.99,
    });
    let mut filter = create(&config, nav([0.0; 3]), 1.0);
    let mut builder = fu::FusionRtsHistoryBuilder::from_filter(&filter).expect("history builder");
    filter
        .propagate_recorded(fu::ImuSample::rate(1.0, [0.0; 3], [0.0; 3]), &mut builder)
        .expect("recorded propagate");
    let measurement = fix(
        1.0,
        [6378137.35, 0.2, -0.1],
        None,
        vec![
            vec![0.5, 0.0, 0.0],
            vec![0.0, 0.5, 0.0],
            vec![0.0, 0.0, 0.5],
        ],
        7,
        fu::GnssFixStatus::Single,
    );
    let u = filter
        .update_loose_recorded(&measurement, &mut builder)
        .expect("recorded update");
    update("RECORDED_UPDATE", &u);
    fa(
        "RECORDED_STATE_POSITION_BITS",
        &filter.state().nominal.position_ecef_m,
    );
    let history = builder.clone().finish().expect("history");
    int("HISTORY_EPOCH_COUNT", history.epochs.len());
    for (i, epoch) in history.epochs.iter().enumerate() {
        int(
            &format!("HISTORY_EPOCH{i}_COVARIANCE_DIMENSION"),
            epoch.updated.state.dimension(),
        );
        int(
            &format!("HISTORY_EPOCH{i}_AUGMENTED_DIMENSION"),
            epoch.updated.tight.augmented_covariance.len(),
        );
        flag(
            &format!("HISTORY_EPOCH{i}_HAS_TRANSITION"),
            epoch.transition_from_previous.is_some(),
        );
        let transition = epoch
            .transition_from_previous
            .as_ref()
            .map_or_else(Vec::new, |m| flatten(m));
        int(
            &format!("HISTORY_EPOCH{i}_TRANSITION_LEN"),
            transition.len(),
        );
        hash_f64s(&format!("HISTORY_EPOCH{i}_TRANSITION_FNV1A64"), &transition);
    }
    let smoothed = fu::smooth_fusion_rts(&history).expect("RTS smoothing");
    int("SMOOTHED_EPOCH_COUNT", smoothed.epochs.len());
    for (i, epoch) in smoothed.epochs.iter().enumerate() {
        int(
            &format!("SMOOTHED_EPOCH{i}_COVARIANCE_DIMENSION"),
            epoch.covariance.len(),
        );
        int(
            &format!("SMOOTHED_EPOCH{i}_CORRECTION_LEN"),
            epoch.error_state_correction.len(),
        );
        flag(
            &format!("SMOOTHED_EPOCH{i}_HAS_RTS_GAIN"),
            epoch.rts_gain_to_next.is_some(),
        );
        fa(
            &format!("SMOOTHED_EPOCH{i}_POSITION_BITS"),
            &epoch.snapshot.state.nominal.position_ecef_m,
        );
        hash_f64s(
            &format!("SMOOTHED_EPOCH{i}_CORRECTION_FNV1A64"),
            &epoch.error_state_correction,
        );
        let covariance = flatten(&epoch.covariance);
        int(
            &format!("SMOOTHED_EPOCH{i}_COVARIANCE_LEN"),
            covariance.len(),
        );
        hash_f64s(
            &format!("SMOOTHED_EPOCH{i}_COVARIANCE_FNV1A64"),
            &covariance,
        );
        let gain = epoch
            .rts_gain_to_next
            .as_ref()
            .map_or_else(Vec::new, |m| flatten(m));
        int(&format!("SMOOTHED_EPOCH{i}_RTS_GAIN_LEN"), gain.len());
        hash_f64s(&format!("SMOOTHED_EPOCH{i}_RTS_GAIN_FNV1A64"), &gain);
    }
    println!();
}

fn main() {
    header_start("w4_d018", GUARD);
    signal();
    scenario();
    fusion();
    recorded_rts();
    header_end(GUARD);
}
