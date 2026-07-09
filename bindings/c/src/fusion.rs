use super::*;

/// Error-state update algorithm used by a GNSS/INS fusion filter.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionFilterKind {
    /// Extended Kalman filter update.
    Ekf = 0,
    /// Unscented Kalman filter update.
    Ukf = 1,
}

/// Error-state covariance layout used by a GNSS/INS fusion filter.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionErrorStateLayout {
    /// Fifteen-state layout: position, velocity, attitude, accelerometer bias,
    /// and gyroscope bias.
    Fifteen = 0,
    /// Twenty-one-state layout adding accelerometer and gyroscope scale factors.
    TwentyOne = 1,
}

/// IMU preset grade for built-in stochastic parameters.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionImuGrade {
    /// Low-cost MEMS class.
    Mems = 0,
    /// Tactical class.
    Tactical = 1,
    /// Navigation class.
    Navigation = 2,
}

/// Strapdown coning-correction selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionConingCorrection {
    /// Do not apply coning correction.
    Off = 0,
}

/// IMU sample payload selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionImuSampleKind {
    /// Specific force and angular-rate sample.
    Rate = 0,
    /// Sensor-integrated delta-velocity and delta-angle sample.
    Increment = 1,
}

/// GNSS fix status used to scale loose-fix covariance in field mode.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFusionGnssFixStatus {
    /// Code-only or otherwise autonomous fix.
    Single = 0,
    /// Carrier float fix.
    Float = 1,
    /// Carrier integer-fixed fix.
    Fixed = 2,
}

/// Copy a fusion filter kind label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_kind_label(
    kind: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_kind_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_filter_kind_label",
                out_written,
                out_required
            ));
            let label = c_try!(fusion_filter_kind_label_from_c(
                "sidereon_fusion_filter_kind_label",
                kind
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_filter_kind_label",
                "out",
                label.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a fusion error-state layout label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_error_state_layout_label(
    layout: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_error_state_layout_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_error_state_layout_label",
                out_written,
                out_required
            ));
            let label = c_try!(fusion_error_state_layout_label_from_c(
                "sidereon_fusion_error_state_layout_label",
                layout
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_error_state_layout_label",
                "out",
                label.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Return the covariance dimension for a fusion error-state layout.
///
/// Safety: out must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_error_state_layout_dimension(
    layout: u32,
    out: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_error_state_layout_dimension",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_fusion_error_state_layout_dimension",
                "out"
            ));
            *out = 0;
            let layout = c_try!(fusion_layout_from_c(
                "sidereon_fusion_error_state_layout_dimension",
                layout
            ));
            *out = layout.dimension();
            SidereonStatus::Ok
        },
    )
}

/// Copy a GNSS fix-status label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_gnss_fix_status_label(
    status: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_gnss_fix_status_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_gnss_fix_status_label",
                out_written,
                out_required
            ));
            let label = c_try!(fusion_gnss_fix_status_label_from_c(
                "sidereon_fusion_gnss_fix_status_label",
                status
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_gnss_fix_status_label",
                "out",
                label.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Opaque GNSS/INS fusion filter. Create with sidereon_fusion_filter_create and
/// release with sidereon_fusion_filter_free.
pub struct SidereonFusionFilter {
    pub(crate) inner: sidereon_core::fusion::InertialFilter,
}

/// Opaque builder for recorded fusion RTS histories. Create with
/// sidereon_fusion_rts_history_builder_new or
/// sidereon_fusion_rts_history_builder_from_filter and release with
/// sidereon_fusion_rts_history_builder_free.
pub struct SidereonFusionRtsHistoryBuilder {
    pub(crate) inner: sidereon_core::fusion::FusionRtsHistoryBuilder,
}

/// Opaque recorded fusion RTS history. Create with
/// sidereon_fusion_rts_history_builder_finish and release with
/// sidereon_fusion_rts_history_free.
pub struct SidereonFusionRtsHistory {
    pub(crate) inner: sidereon_core::fusion::FusionRtsHistory,
}

/// Opaque smoothed fusion trajectory. Create with sidereon_smooth_fusion_rts
/// and release with sidereon_smoothed_fusion_trajectory_free.
pub struct SidereonSmoothedFusionTrajectory {
    pub(crate) inner: sidereon_core::fusion::SmoothedFusionTrajectory,
}

/// Datasheet-level IMU stochastic parameters for fusion filter prediction.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionImuSpec {
    /// Accelerometer velocity random walk in m/s per square-root second.
    pub accel_vrw_mps_sqrt_s: f64,
    /// Gyroscope angular random walk in rad per square-root second.
    pub gyro_arw_rad_sqrt_s: f64,
    /// Accelerometer bias instability in m/s^2.
    pub accel_bias_instab_mps2: f64,
    /// Gyroscope bias instability in rad/s.
    pub gyro_bias_instab_rps: f64,
    /// Accelerometer Gauss-Markov bias time constant in seconds.
    pub accel_bias_tau_s: f64,
    /// Gyroscope Gauss-Markov bias time constant in seconds.
    pub gyro_bias_tau_s: f64,
    /// Whether accel_scale_instab_ppm carries a value.
    pub has_accel_scale_instab_ppm: bool,
    /// Accelerometer scale-factor instability in parts per million.
    pub accel_scale_instab_ppm: f64,
    /// Whether gyro_scale_instab_ppm carries a value.
    pub has_gyro_scale_instab_ppm: bool,
    /// Gyroscope scale-factor instability in parts per million.
    pub gyro_scale_instab_ppm: f64,
}

/// Strapdown mechanization configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionMechanizationConfig {
    /// One of SidereonFusionConingCorrection_*.
    pub coning_correction: u32,
}

/// Optional normalized-innovation screen for fusion measurement updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionInnovationGate {
    /// Rejection threshold in normalized sigma units.
    pub threshold_sigma: f64,
    /// Minimum accepted rows required to apply an update after screening.
    pub min_rows: usize,
}

/// IGG-III measurement variance inflation break points for loose GNSS updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionIggIiiMeasurementReweighting {
    /// Lower standardized-innovation break point in sigma.
    pub k0_sigma: f64,
    /// Upper standardized-innovation break point in sigma.
    pub k1_sigma: f64,
}

/// Yang predicted-covariance adaptive factor for loose GNSS updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionYangPredictionAdaptiveFactor {
    /// Two-segment threshold for the predicted-residual statistic.
    pub threshold: f64,
    /// Chi-square probability used for the Mahalanobis outlier gate.
    pub outlier_gate_probability: f64,
}

/// Per-fix-status one-sigma multipliers for loose GNSS measurements.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionFixStatusWeighting {
    /// Sigma multiplier for SidereonFusionGnssFixStatus_Single.
    pub single_sigma_multiplier: f64,
    /// Sigma multiplier for SidereonFusionGnssFixStatus_Float.
    pub float_sigma_multiplier: f64,
    /// Sigma multiplier for SidereonFusionGnssFixStatus_Fixed.
    pub fixed_sigma_multiplier: f64,
}

/// Stationary detector thresholds used before applying ZUPT/ZARU updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionStationaryDetectorConfig {
    /// Number of most-recent IMU samples considered.
    pub window_len: usize,
    /// Maximum specific-force norm error from local gravity, m/s^2.
    pub max_specific_force_norm_error_mps2: f64,
    /// Maximum body-rate norm relative to ECEF, rad/s.
    pub max_body_rate_wrt_ecef_norm_rps: f64,
}

/// Zero-velocity and zero-angular-rate update configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionStationaryUpdateConfig {
    /// Stationary detector thresholds.
    pub detector: SidereonFusionStationaryDetectorConfig,
    /// One-sigma zero-velocity update uncertainty in m/s.
    pub zero_velocity_sigma_mps: f64,
    /// One-sigma zero-angular-rate update uncertainty in rad/s.
    pub zero_angular_rate_sigma_rps: f64,
}

/// Wheeled-vehicle non-holonomic constraint configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionNonHolonomicConstraintConfig {
    /// One-sigma lateral body-frame velocity uncertainty in m/s.
    pub lateral_velocity_sigma_mps: f64,
    /// One-sigma vertical body-frame velocity uncertainty in m/s.
    pub vertical_velocity_sigma_mps: f64,
    /// Minimum speed required before applying the constraint, m/s.
    pub min_speed_mps: f64,
    /// Maximum body-rate norm relative to ECEF, rad/s.
    pub max_body_rate_wrt_ecef_norm_rps: f64,
}

/// Velocity matching outage repair configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionVelocityMatchingConfig {
    /// Maximum outage duration eligible for matching, seconds.
    pub max_outage_duration_s: f64,
}

/// Initial nominal navigation state for a fusion filter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionNavState {
    /// State epoch, seconds since J2000 on the caller's GNSS time scale.
    pub t_j2000_s: f64,
    /// IMU position in ECEF meters.
    pub position_ecef_m: [f64; 3],
    /// IMU velocity in ECEF meters per second.
    pub velocity_ecef_mps: [f64; 3],
    /// Row-major body-to-ECEF direction cosine matrix.
    pub attitude_body_to_ecef: [f64; 9],
    /// Closed-loop accelerometer bias estimate in m/s^2.
    pub accel_bias_mps2: [f64; 3],
    /// Closed-loop gyroscope bias estimate in rad/s.
    pub gyro_bias_rps: [f64; 3],
    /// Closed-loop accelerometer scale-factor estimate for the 21-state layout.
    pub accel_scale_factor: [f64; 3],
    /// Closed-loop gyroscope scale-factor estimate for the 21-state layout.
    pub gyro_scale_factor: [f64; 3],
}

/// Fusion filter configuration. Initialize with
/// sidereon_fusion_filter_config_init before overriding fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionFilterConfig {
    /// One of SidereonFusionFilterKind_*.
    pub filter_kind: u32,
    /// One of SidereonFusionErrorStateLayout_*.
    pub error_state_layout: u32,
    /// IMU stochastic model.
    pub imu_spec: SidereonFusionImuSpec,
    /// Accelerometer bias subtracted from raw IMU samples before mechanization.
    pub imu_bias_accel_mps2: [f64; 3],
    /// Gyroscope bias subtracted from raw IMU samples before mechanization.
    pub imu_bias_gyro_rps: [f64; 3],
    /// Row-major accelerometer scale and misalignment matrix.
    pub imu_accel_scale_misalignment: [f64; 9],
    /// Row-major gyroscope scale and misalignment matrix.
    pub imu_gyro_scale_misalignment: [f64; 9],
    /// Row-major IMU-frame to body-frame direction cosine matrix.
    pub imu_to_body_dcm: [f64; 9],
    /// Strapdown mechanization options.
    pub mechanization: SidereonFusionMechanizationConfig,
    /// Loose-coupling body-frame lever arm from IMU origin to GNSS antenna, meters.
    pub loose_lever_arm_body_m: [f64; 3],
    /// Per-fix-status one-sigma multipliers for loose GNSS covariance.
    pub loose_fix_status_weighting: SidereonFusionFixStatusWeighting,
    /// Whether loose_innovation_gate carries a loose-update screen.
    pub has_loose_innovation_gate: bool,
    /// EKF loose-update innovation screen.
    pub loose_innovation_gate: SidereonFusionInnovationGate,
    /// Whether loose_measurement_reweighting carries IGG-III robust settings.
    pub has_loose_measurement_reweighting: bool,
    /// IGG-III measurement variance inflation settings for loose updates.
    pub loose_measurement_reweighting: SidereonFusionIggIiiMeasurementReweighting,
    /// Whether loose_prediction_adaptation carries Yang robust settings.
    pub has_loose_prediction_adaptation: bool,
    /// Yang predicted-covariance adaptive factor for loose updates.
    pub loose_prediction_adaptation: SidereonFusionYangPredictionAdaptiveFactor,
    /// Whether loose_stationary_updates carries ZUPT/ZARU settings.
    pub has_loose_stationary_updates: bool,
    /// Zero-velocity and zero-angular-rate field-mode settings.
    pub loose_stationary_updates: SidereonFusionStationaryUpdateConfig,
    /// Whether loose_non_holonomic carries wheeled-vehicle constraint settings.
    pub has_loose_non_holonomic: bool,
    /// Non-holonomic constraint settings.
    pub loose_non_holonomic: SidereonFusionNonHolonomicConstraintConfig,
    /// Tight-coupling body-frame lever arm from IMU origin to GNSS antenna, meters.
    pub tight_lever_arm_body_m: [f64; 3],
    /// Whether tight raw GNSS updates apply light-time correction.
    pub tight_light_time: bool,
    /// Whether tight raw GNSS updates apply Sagnac correction.
    pub tight_sagnac: bool,
    /// Initial tight receiver-clock bias variance in square meters.
    pub tight_initial_clock_bias_variance_m2: f64,
    /// Initial tight receiver-clock drift variance in (m/s)^2.
    pub tight_initial_clock_drift_variance_m2_s2: f64,
    /// Tight clock-bias random-walk spectral density in m^2/s.
    pub tight_clock_bias_random_walk_m2_s: f64,
    /// Tight clock-drift random-walk spectral density in m^2/s^3.
    pub tight_clock_drift_random_walk_m2_s3: f64,
    /// Whether tight_innovation_gate carries a tight-update screen.
    pub has_tight_innovation_gate: bool,
    /// EKF tight-update innovation screen.
    pub tight_innovation_gate: SidereonFusionInnovationGate,
    /// UKF sigma-point spread parameter.
    pub ukf_alpha: f64,
    /// UKF prior-distribution shape parameter.
    pub ukf_beta: f64,
    /// UKF secondary sigma-point scaling parameter.
    pub ukf_kappa: f64,
    /// Whether ukf_innovation_gate carries a UKF update screen.
    pub has_ukf_innovation_gate: bool,
    /// UKF innovation screen used by loose and tight UKF updates.
    pub ukf_innovation_gate: SidereonFusionInnovationGate,
    /// Retained IMU sample capacity for time-synchronization replay.
    pub time_sync_imu_capacity: usize,
    /// Retained checkpoint capacity for time-synchronization replay.
    pub time_sync_checkpoint_capacity: usize,
}

/// IMU sample for inertial propagation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionImuSample {
    /// Sample end epoch, seconds since J2000.
    pub t_j2000_s: f64,
    /// One of SidereonFusionImuSampleKind_*.
    pub kind: u32,
    /// Body-frame specific force in m/s^2 for rate samples.
    pub specific_force_mps2: [f64; 3],
    /// Body-frame angular rate in rad/s for rate samples.
    pub angular_rate_rps: [f64; 3],
    /// Body-frame delta velocity in m/s for increment samples.
    pub delta_velocity_mps: [f64; 3],
    /// Body-frame delta angle in radians for increment samples.
    pub delta_theta_rad: [f64; 3],
    /// Sample integration interval in seconds for increment samples.
    pub dt_s: f64,
}

/// Loose GNSS position or position-velocity fix measurement.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionLooseMeasurement {
    /// Measurement epoch, seconds since J2000.
    pub t_j2000_s: f64,
    /// GNSS antenna ECEF position in meters.
    pub position_ecef_m: [f64; 3],
    /// Whether velocity_ecef_mps carries a velocity measurement.
    pub has_velocity: bool,
    /// GNSS antenna ECEF velocity in meters per second.
    pub velocity_ecef_mps: [f64; 3],
    /// Row-major covariance matrix: 3x3 for position-only, 6x6 with velocity.
    pub covariance: *const f64,
    /// Number of doubles in covariance.
    pub covariance_len: usize,
    /// Number of satellites used by the upstream GNSS fix.
    pub satellites_used: usize,
    /// Whether the upstream GNSS fix is valid.
    pub solution_valid: bool,
    /// One of SidereonFusionGnssFixStatus_*.
    pub fix_status: u32,
}

/// One state row accepted and returned by velocity matching.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionVelocityMatchState {
    /// State epoch, seconds since J2000.
    pub t_j2000_s: f64,
    /// ECEF position in meters.
    pub position_ecef_m: [f64; 3],
    /// ECEF velocity in meters per second.
    pub velocity_ecef_mps: [f64; 3],
}

/// Metadata for a velocity-matched outage trajectory.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionVelocityMatchedTrajectory {
    /// Number of states in the matched trajectory.
    pub state_count: usize,
    /// Endpoint ECEF position correction in meters.
    pub endpoint_position_correction_ecef_m: [f64; 3],
    /// Endpoint ECEF velocity correction in meters per second.
    pub endpoint_velocity_correction_ecef_mps: [f64; 3],
}

/// Doppler-derived range-rate row for a tight GNSS observation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTightRangeRate {
    /// Measured pseudorange rate in meters per second.
    pub measured_range_rate_m_s: f64,
    /// One-sigma range-rate uncertainty in meters per second.
    pub sigma_m_s: f64,
    /// Satellite clock drift as range-rate bias in meters per second.
    pub satellite_clock_drift_m_s: f64,
}

/// Carrier-phase range row for a tight GNSS observation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTightCarrierPhase {
    /// Carrier phase converted to range units in meters.
    pub phase_range_m: f64,
    /// One-sigma carrier-phase range uncertainty in meters.
    pub sigma_m: f64,
    /// Caller-supplied float ambiguity in meters.
    pub float_ambiguity_m: f64,
}

/// Raw GNSS observation row for a tight update.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTightObservation {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Measured code pseudorange in meters.
    pub pseudorange_m: f64,
    /// One-sigma pseudorange uncertainty in meters.
    pub pseudorange_sigma_m: f64,
    /// Whether range_rate carries a Doppler-derived row.
    pub has_range_rate: bool,
    /// Optional Doppler-derived range-rate row.
    pub range_rate: SidereonFusionTightRangeRate,
    /// Whether carrier_phase carries a carrier-phase row.
    pub has_carrier_phase: bool,
    /// Optional carrier-phase range row.
    pub carrier_phase: SidereonFusionTightCarrierPhase,
    /// Ionospheric group delay correction for code in meters.
    pub ionosphere_delay_m: f64,
    /// Tropospheric delay correction in meters.
    pub troposphere_delay_m: f64,
}

/// One raw GNSS measurement epoch for tight coupling.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTightEpoch {
    /// Measurement epoch, seconds since J2000.
    pub t_j2000_s: f64,
    /// Pointer to observation_count tight observation rows.
    pub observations: *const SidereonFusionTightObservation,
    /// Number of tight observation rows.
    pub observation_count: usize,
}

/// Report returned by loose and tight fusion updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionUpdate {
    /// Whether the update modified state and covariance.
    pub applied: bool,
    /// Normalized innovation squared.
    pub nis: f64,
    /// Number of measurement rows entering the update.
    pub rows: usize,
    /// Number of rows accepted by innovation screening.
    pub accepted_rows: usize,
    /// Number of rows rejected by innovation screening.
    pub rejected_rows: usize,
}

/// Report returned by time-synchronized fusion updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTimeSyncUpdate {
    /// Fusion update applied at the supplied measurement epoch.
    pub update: SidereonFusionUpdate,
    /// Whether the supplied measurement was older than the current inertial epoch.
    pub late_measurement: bool,
    /// Number of IMU segments replayed.
    pub replayed_imu_segments: usize,
    /// Checkpoint epoch used as the replay start.
    pub restored_checkpoint_epoch_j2000_s: f64,
    /// Filter epoch after replay.
    pub current_epoch_j2000_s: f64,
}

/// Retained history occupancy for time-synchronized updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionTimeSyncStatus {
    /// Configured IMU sample capacity.
    pub imu_capacity: usize,
    /// Number of retained IMU samples.
    pub imu_len: usize,
    /// Configured checkpoint capacity.
    pub checkpoint_capacity: usize,
    /// Number of retained filter checkpoints.
    pub checkpoint_len: usize,
    /// Whether oldest_imu_epoch_j2000_s carries a value.
    pub has_oldest_imu_epoch_j2000_s: bool,
    /// Oldest retained IMU sample end epoch.
    pub oldest_imu_epoch_j2000_s: f64,
    /// Whether newest_imu_epoch_j2000_s carries a value.
    pub has_newest_imu_epoch_j2000_s: bool,
    /// Newest retained IMU sample end epoch.
    pub newest_imu_epoch_j2000_s: f64,
    /// Whether oldest_checkpoint_epoch_j2000_s carries a value.
    pub has_oldest_checkpoint_epoch_j2000_s: bool,
    /// Oldest retained checkpoint epoch.
    pub oldest_checkpoint_epoch_j2000_s: f64,
    /// Whether newest_checkpoint_epoch_j2000_s carries a value.
    pub has_newest_checkpoint_epoch_j2000_s: bool,
    /// Newest retained checkpoint epoch.
    pub newest_checkpoint_epoch_j2000_s: f64,
}

/// Current fusion filter state summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionState {
    /// State epoch, seconds since J2000.
    pub t_j2000_s: f64,
    /// IMU position in ECEF meters.
    pub position_ecef_m: [f64; 3],
    /// IMU velocity in ECEF meters per second.
    pub velocity_ecef_mps: [f64; 3],
    /// Row-major body-to-ECEF direction cosine matrix.
    pub attitude_body_to_ecef: [f64; 9],
    /// Accelerometer bias estimate in m/s^2.
    pub accel_bias_mps2: [f64; 3],
    /// Gyroscope bias estimate in rad/s.
    pub gyro_bias_rps: [f64; 3],
    /// Accelerometer scale-factor estimate.
    pub accel_scale_factor: [f64; 3],
    /// Gyroscope scale-factor estimate.
    pub gyro_scale_factor: [f64; 3],
    /// Error-state covariance dimension.
    pub covariance_dimension: usize,
    /// Last body angular rate relative to ECEF, resolved in body axes.
    pub last_body_rate_wrt_ecef_rps: [f64; 3],
    /// Tight receiver-clock range bias in meters.
    pub tight_clock_bias_m: f64,
    /// Tight receiver-clock drift in meters per second.
    pub tight_clock_drift_m_s: f64,
    /// Row-major 2x2 tight receiver-clock covariance.
    pub tight_clock_covariance: [f64; 4],
}

/// Summary of a recorded fusion RTS history epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFusionRtsEpoch {
    /// Epoch in seconds since J2000.
    pub t_j2000_s: f64,
    /// INS error-state covariance dimension.
    pub covariance_dimension: usize,
    /// Full augmented smoothing dimension, including tight clock states.
    pub augmented_dimension: usize,
    /// Whether transition_from_previous is present for this epoch.
    pub has_transition_from_previous: bool,
}

/// Summary of one smoothed fusion trajectory epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSmoothedFusionEpoch {
    /// Epoch in seconds since J2000.
    pub t_j2000_s: f64,
    /// Full smoothed covariance dimension.
    pub covariance_dimension: usize,
    /// Error-state correction vector length.
    pub correction_len: usize,
    /// Whether an RTS gain to the next epoch is present.
    pub has_rts_gain_to_next: bool,
}

/// Fill an IMU spec with one of the core preset grades.
///
/// Safety: out must point to a SidereonFusionImuSpec.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_imu_spec_preset(
    grade: u32,
    out: *mut SidereonFusionImuSpec,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_imu_spec_preset",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_fusion_imu_spec_preset", "out"));
            let grade = c_try!(fusion_imu_grade_from_c(
                "sidereon_fusion_imu_spec_preset",
                grade
            ));
            *out = fusion_imu_spec_to_c(sidereon_core::fusion::ImuSpec::preset(grade));
            SidereonStatus::Ok
        },
    )
}

/// Initialize fusion filter configuration with core defaults.
///
/// Safety: out must point to a SidereonFusionFilterConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_config_init(
    out: *mut SidereonFusionFilterConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_config_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_fusion_filter_config_init",
                "out"
            ));
            let tight = sidereon_core::fusion::TightCouplingConfig::default();
            let loose = sidereon_core::fusion::LooseCouplingConfig::default();
            let ukf = sidereon_core::fusion::UkfUpdateOptions::default();
            *out = SidereonFusionFilterConfig {
                filter_kind: SidereonFusionFilterKind::Ekf as u32,
                error_state_layout: SidereonFusionErrorStateLayout::Fifteen as u32,
                imu_spec: fusion_imu_spec_to_c(sidereon_core::fusion::ImuSpec::preset(
                    sidereon_core::fusion::ImuGrade::Mems,
                )),
                imu_bias_accel_mps2: [0.0; 3],
                imu_bias_gyro_rps: [0.0; 3],
                imu_accel_scale_misalignment: [0.0; 9],
                imu_gyro_scale_misalignment: [0.0; 9],
                imu_to_body_dcm: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                mechanization: SidereonFusionMechanizationConfig {
                    coning_correction: SidereonFusionConingCorrection::Off as u32,
                },
                loose_lever_arm_body_m: [0.0; 3],
                loose_fix_status_weighting: fusion_fix_status_weighting_to_c(
                    loose.fix_status_weighting,
                ),
                has_loose_innovation_gate: false,
                loose_innovation_gate: zero_fusion_innovation_gate(),
                has_loose_measurement_reweighting: false,
                loose_measurement_reweighting: fusion_igg_iii_measurement_reweighting_to_c(
                    sidereon_core::fusion::IggIiiMeasurementReweighting::standard(),
                ),
                has_loose_prediction_adaptation: false,
                loose_prediction_adaptation: fusion_yang_prediction_adaptive_factor_to_c(
                    sidereon_core::fusion::YangPredictionAdaptiveFactor::standard(),
                ),
                has_loose_stationary_updates: false,
                loose_stationary_updates: zero_stationary_update_config(),
                has_loose_non_holonomic: false,
                loose_non_holonomic: zero_non_holonomic_config(),
                tight_lever_arm_body_m: tight.lever_arm_body_m,
                tight_light_time: tight.light_time,
                tight_sagnac: tight.sagnac,
                tight_initial_clock_bias_variance_m2: tight.initial_clock_bias_variance_m2,
                tight_initial_clock_drift_variance_m2_s2: tight.initial_clock_drift_variance_m2_s2,
                tight_clock_bias_random_walk_m2_s: tight.clock_bias_random_walk_m2_s,
                tight_clock_drift_random_walk_m2_s3: tight.clock_drift_random_walk_m2_s3,
                has_tight_innovation_gate: false,
                tight_innovation_gate: zero_fusion_innovation_gate(),
                ukf_alpha: ukf.transform.alpha,
                ukf_beta: ukf.transform.beta,
                ukf_kappa: ukf.transform.kappa,
                has_ukf_innovation_gate: false,
                ukf_innovation_gate: zero_fusion_innovation_gate(),
                time_sync_imu_capacity: sidereon_core::fusion::DEFAULT_TIME_SYNC_IMU_CAPACITY,
                time_sync_checkpoint_capacity:
                    sidereon_core::fusion::DEFAULT_TIME_SYNC_CHECKPOINT_CAPACITY,
            };
            SidereonStatus::Ok
        },
    )
}

/// Create a stateful GNSS/INS fusion filter.
///
/// Safety: initial and config must point to readable structs; covariance_diagonal
/// must point to covariance_len doubles; out_filter must point to storage for a
/// SidereonFusionFilter*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_create(
    initial: *const SidereonFusionNavState,
    covariance_diagonal: *const f64,
    covariance_len: usize,
    config: *const SidereonFusionFilterConfig,
    out_filter: *mut *mut SidereonFusionFilter,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_create",
        SidereonStatus::Panic,
        || {
            let out_filter = c_try!(require_out(
                out_filter,
                "sidereon_fusion_filter_create",
                "out_filter"
            ));
            *out_filter = ptr::null_mut();
            let initial = c_try!(require_ref(
                initial,
                "sidereon_fusion_filter_create",
                "initial"
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_fusion_filter_create",
                "config"
            ));
            let layout = c_try!(fusion_layout_from_c(
                "sidereon_fusion_filter_create",
                config.error_state_layout
            ));
            let diagonal = c_try!(require_slice(
                covariance_diagonal,
                covariance_len,
                "sidereon_fusion_filter_create",
                "covariance_diagonal"
            ));
            let nominal = c_try!(nav_state_from_c("sidereon_fusion_filter_create", initial));
            let mut state = match sidereon_core::fusion::InsFilterState::from_diagonal(
                nominal, layout, diagonal,
            ) {
                Ok(state) => state,
                Err(err) => return map_fusion_error("sidereon_fusion_filter_create", err),
            };
            state.accel_scale_factor = initial.accel_scale_factor;
            state.gyro_scale_factor = initial.gyro_scale_factor;
            if let Err(err) = state.validate() {
                return map_fusion_error("sidereon_fusion_filter_create", err);
            }
            let time_sync_imu_capacity = config.time_sync_imu_capacity;
            let time_sync_checkpoint_capacity = config.time_sync_checkpoint_capacity;
            let config = c_try!(fusion_filter_config_from_c(
                "sidereon_fusion_filter_create",
                config
            ));
            let mut inner = match sidereon_core::fusion::InertialFilter::with_config(state, config)
            {
                Ok(filter) => filter,
                Err(err) => return map_fusion_error("sidereon_fusion_filter_create", err),
            };
            let history = sidereon_core::fusion::TimeSyncHistoryConfig::new(
                time_sync_imu_capacity,
                time_sync_checkpoint_capacity,
            );
            if let Err(err) = inner.configure_time_sync_history(history) {
                return map_fusion_error("sidereon_fusion_filter_create", err);
            }
            write_boxed_handle(out_filter, SidereonFusionFilter { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a fusion filter handle from sidereon_fusion_filter_create. Passing
/// NULL is a no-op.
///
/// Safety: filter must be NULL or a live SidereonFusionFilter handle that has
/// not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_free(filter: *mut SidereonFusionFilter) {
    ffi_boundary("sidereon_fusion_filter_free", (), || {
        free_boxed(filter);
    });
}

/// Copy the current fusion state summary.
///
/// Safety: filter must be a live handle and out must point to a
/// SidereonFusionState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_state(
    filter: *const SidereonFusionFilter,
    out: *mut SidereonFusionState,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_state",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_fusion_filter_state",
                "filter"
            ));
            let out = c_try!(require_out(out, "sidereon_fusion_filter_state", "out"));
            *out = c_try!(fusion_state_to_c(
                "sidereon_fusion_filter_state",
                &filter.inner
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the current error-state covariance in row-major order.
///
/// Safety: filter must be a live handle; out must point to len doubles or NULL
/// when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_covariance(
    filter: *const SidereonFusionFilter,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_filter_covariance",
                out_written,
                out_required
            ));
            let filter = c_try!(require_ref(
                filter,
                "sidereon_fusion_filter_covariance",
                "filter"
            ));
            let values = flatten_matrix(&filter.inner.state().covariance);
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_filter_covariance",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Propagate a fusion filter with one IMU sample.
///
/// Safety: filter must be a live handle and sample must point to a readable
/// SidereonFusionImuSample.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_propagate(
    filter: *mut SidereonFusionFilter,
    sample: *const SidereonFusionImuSample,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_propagate",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_propagate",
                "filter"
            ));
            let sample = c_try!(require_ref(
                sample,
                "sidereon_fusion_filter_propagate",
                "sample"
            ));
            let sample = c_try!(imu_sample_from_c(
                "sidereon_fusion_filter_propagate",
                sample
            ));
            match filter.inner.propagate(sample) {
                Ok(_) => SidereonStatus::Ok,
                Err(err) => map_fusion_error("sidereon_fusion_filter_propagate", err),
            }
        },
    )
}

/// Propagate a fusion filter and record the transition for RTS smoothing.
///
/// Safety: filter and history must be live handles; sample must point to a
/// readable SidereonFusionImuSample.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_propagate_recorded(
    filter: *mut SidereonFusionFilter,
    sample: *const SidereonFusionImuSample,
    history: *mut SidereonFusionRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_propagate_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_propagate_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_propagate_recorded",
                "history"
            ));
            let sample = c_try!(require_ref(
                sample,
                "sidereon_fusion_filter_propagate_recorded",
                "sample"
            ));
            let sample = c_try!(imu_sample_from_c(
                "sidereon_fusion_filter_propagate_recorded",
                sample
            ));
            match filter.inner.propagate_recorded(sample, &mut history.inner) {
                Ok(_) => SidereonStatus::Ok,
                Err(err) => map_fusion_error("sidereon_fusion_filter_propagate_recorded", err),
            }
        },
    )
}

/// Apply a loose GNSS position or position-velocity update.
///
/// Safety: filter must be a live handle; measurement must point to a readable
/// SidereonFusionLooseMeasurement; out_update must point to a
/// SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_loose(
    filter: *mut SidereonFusionFilter,
    measurement: *const SidereonFusionLooseMeasurement,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_loose",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_loose",
                "filter"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_loose",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let measurement = c_try!(loose_measurement_from_c(
                "sidereon_fusion_filter_update_loose",
                measurement
            ));
            match filter.inner.update_loose(&measurement) {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_loose", err),
            }
        },
    )
}

/// Apply a loose GNSS update and record before/after checkpoints for RTS smoothing.
///
/// Safety: filter and history must be live handles; measurement must point to a
/// readable SidereonFusionLooseMeasurement; out_update must point to a
/// SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_loose_recorded(
    filter: *mut SidereonFusionFilter,
    measurement: *const SidereonFusionLooseMeasurement,
    history: *mut SidereonFusionRtsHistoryBuilder,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_loose_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_loose_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_update_loose_recorded",
                "history"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_loose_recorded",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let measurement = c_try!(loose_measurement_from_c(
                "sidereon_fusion_filter_update_loose_recorded",
                measurement
            ));
            match filter
                .inner
                .update_loose_recorded(&measurement, &mut history.inner)
            {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_loose_recorded", err),
            }
        },
    )
}

/// Apply a time-synchronized loose GNSS update, replaying retained IMU samples
/// when the measurement is late.
///
/// Safety: filter must be a live handle; measurement must point to a readable
/// SidereonFusionLooseMeasurement; out_update must point to a
/// SidereonFusionTimeSyncUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_loose_time_sync(
    filter: *mut SidereonFusionFilter,
    measurement: *const SidereonFusionLooseMeasurement,
    out_update: *mut SidereonFusionTimeSyncUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_loose_time_sync",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_loose_time_sync",
                "filter"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_loose_time_sync",
                "out_update"
            ));
            *out_update = zero_time_sync_update();
            let measurement = c_try!(loose_measurement_from_c(
                "sidereon_fusion_filter_update_loose_time_sync",
                measurement
            ));
            match filter.inner.update_loose_time_sync(&measurement) {
                Ok(update) => {
                    *out_update = time_sync_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_loose_time_sync", err),
            }
        },
    )
}

/// Apply a gated zero-velocity and zero-angular-rate update. When no update is
/// applied, out_present is false and out_update is zeroed.
///
/// Safety: filter must be a live handle; out_update must point to a
/// SidereonFusionUpdate; out_present must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_stationary(
    filter: *mut SidereonFusionFilter,
    out_update: *mut SidereonFusionUpdate,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_stationary",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_stationary",
                "filter"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_stationary",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_fusion_filter_update_stationary",
                "out_present"
            ));
            *out_present = false;
            match filter.inner.update_stationary() {
                Ok(Some(update)) => {
                    *out_update = fusion_update_to_c(&update);
                    *out_present = true;
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_stationary", err),
            }
        },
    )
}

/// Apply a stationary update and record checkpoints when an update applies.
/// When no update is applied, out_present is false and out_update is zeroed.
///
/// Safety: filter and history must be live handles; out_update must point to a
/// SidereonFusionUpdate; out_present must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_stationary_recorded(
    filter: *mut SidereonFusionFilter,
    history: *mut SidereonFusionRtsHistoryBuilder,
    out_update: *mut SidereonFusionUpdate,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_stationary_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_stationary_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_update_stationary_recorded",
                "history"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_stationary_recorded",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_fusion_filter_update_stationary_recorded",
                "out_present"
            ));
            *out_present = false;
            match filter.inner.update_stationary_recorded(&mut history.inner) {
                Ok(Some(update)) => {
                    *out_update = fusion_update_to_c(&update);
                    *out_present = true;
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => {
                    map_fusion_error("sidereon_fusion_filter_update_stationary_recorded", err)
                }
            }
        },
    )
}

/// Apply a gated wheeled-vehicle non-holonomic constraint update. When no
/// update is applied, out_present is false and out_update is zeroed.
///
/// Safety: filter must be a live handle; out_update must point to a
/// SidereonFusionUpdate; out_present must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_non_holonomic(
    filter: *mut SidereonFusionFilter,
    out_update: *mut SidereonFusionUpdate,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_non_holonomic",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_non_holonomic",
                "filter"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_non_holonomic",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_fusion_filter_update_non_holonomic",
                "out_present"
            ));
            *out_present = false;
            match filter.inner.update_non_holonomic() {
                Ok(Some(update)) => {
                    *out_update = fusion_update_to_c(&update);
                    *out_present = true;
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_non_holonomic", err),
            }
        },
    )
}

/// Apply a non-holonomic constraint and record checkpoints when an update
/// applies. When no update is applied, out_present is false and out_update is
/// zeroed.
///
/// Safety: filter and history must be live handles; out_update must point to a
/// SidereonFusionUpdate; out_present must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_non_holonomic_recorded(
    filter: *mut SidereonFusionFilter,
    history: *mut SidereonFusionRtsHistoryBuilder,
    out_update: *mut SidereonFusionUpdate,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_non_holonomic_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_non_holonomic_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_update_non_holonomic_recorded",
                "history"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_non_holonomic_recorded",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_fusion_filter_update_non_holonomic_recorded",
                "out_present"
            ));
            *out_present = false;
            match filter
                .inner
                .update_non_holonomic_recorded(&mut history.inner)
            {
                Ok(Some(update)) => {
                    *out_update = fusion_update_to_c(&update);
                    *out_present = true;
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => {
                    map_fusion_error("sidereon_fusion_filter_update_non_holonomic_recorded", err)
                }
            }
        },
    )
}

/// Apply a tight raw GNSS update using an SP3 ephemeris source.
///
/// Safety: filter and sp3 must be live handles; epoch must point to a readable
/// SidereonFusionTightEpoch; out_update must point to a SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_sp3(
    filter: *mut SidereonFusionFilter,
    sp3: *const SidereonSp3,
    epoch: *const SidereonFusionTightEpoch,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_sp3",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_sp3",
                "filter"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fusion_filter_update_tight_sp3",
                "sp3"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_sp3",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_sp3",
                epoch
            ));
            match filter.inner.update_tight(&sp3.inner, &epoch) {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_tight_sp3", err),
            }
        },
    )
}

/// Apply a tight raw GNSS update using a broadcast ephemeris source.
///
/// Safety: filter and broadcast must be live handles; epoch must point to a
/// readable SidereonFusionTightEpoch; out_update must point to a
/// SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_broadcast(
    filter: *mut SidereonFusionFilter,
    broadcast: *const SidereonBroadcastEphemeris,
    epoch: *const SidereonFusionTightEpoch,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_broadcast",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_broadcast",
                "filter"
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_fusion_filter_update_tight_broadcast",
                "broadcast"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_broadcast",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_broadcast",
                epoch
            ));
            match filter.inner.update_tight(&broadcast.inner, &epoch) {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error("sidereon_fusion_filter_update_tight_broadcast", err),
            }
        },
    )
}

/// Apply and record a tight raw GNSS update using an SP3 ephemeris source.
///
/// Safety: filter, sp3, and history must be live handles; epoch must point to a
/// readable SidereonFusionTightEpoch; out_update must point to a
/// SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_sp3_recorded(
    filter: *mut SidereonFusionFilter,
    sp3: *const SidereonSp3,
    epoch: *const SidereonFusionTightEpoch,
    history: *mut SidereonFusionRtsHistoryBuilder,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_sp3_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_sp3_recorded",
                "filter"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fusion_filter_update_tight_sp3_recorded",
                "sp3"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_update_tight_sp3_recorded",
                "history"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_sp3_recorded",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_sp3_recorded",
                epoch
            ));
            match filter
                .inner
                .update_tight_recorded(&sp3.inner, &epoch, &mut history.inner)
            {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_fusion_error("sidereon_fusion_filter_update_tight_sp3_recorded", err)
                }
            }
        },
    )
}

/// Apply and record a tight raw GNSS update using a broadcast ephemeris source.
///
/// Safety: filter, broadcast, and history must be live handles; epoch must
/// point to a readable SidereonFusionTightEpoch; out_update must point to a
/// SidereonFusionUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_broadcast_recorded(
    filter: *mut SidereonFusionFilter,
    broadcast: *const SidereonBroadcastEphemeris,
    epoch: *const SidereonFusionTightEpoch,
    history: *mut SidereonFusionRtsHistoryBuilder,
    out_update: *mut SidereonFusionUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_broadcast_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_broadcast_recorded",
                "filter"
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_fusion_filter_update_tight_broadcast_recorded",
                "broadcast"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_fusion_filter_update_tight_broadcast_recorded",
                "history"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_broadcast_recorded",
                "out_update"
            ));
            *out_update = zero_fusion_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_broadcast_recorded",
                epoch
            ));
            match filter
                .inner
                .update_tight_recorded(&broadcast.inner, &epoch, &mut history.inner)
            {
                Ok(update) => {
                    *out_update = fusion_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error(
                    "sidereon_fusion_filter_update_tight_broadcast_recorded",
                    err,
                ),
            }
        },
    )
}

/// Apply a time-synchronized tight raw GNSS update using an SP3 ephemeris source.
///
/// Safety: filter and sp3 must be live handles; epoch must point to a readable
/// SidereonFusionTightEpoch; out_update must point to a
/// SidereonFusionTimeSyncUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_sp3_time_sync(
    filter: *mut SidereonFusionFilter,
    sp3: *const SidereonSp3,
    epoch: *const SidereonFusionTightEpoch,
    out_update: *mut SidereonFusionTimeSyncUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_sp3_time_sync",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_sp3_time_sync",
                "filter"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fusion_filter_update_tight_sp3_time_sync",
                "sp3"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_sp3_time_sync",
                "out_update"
            ));
            *out_update = zero_time_sync_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_sp3_time_sync",
                epoch
            ));
            match filter.inner.update_tight_time_sync(&sp3.inner, &epoch) {
                Ok(update) => {
                    *out_update = time_sync_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_fusion_error("sidereon_fusion_filter_update_tight_sp3_time_sync", err)
                }
            }
        },
    )
}

/// Apply a time-synchronized tight raw GNSS update using a broadcast ephemeris
/// source.
///
/// Safety: filter and broadcast must be live handles; epoch must point to a
/// readable SidereonFusionTightEpoch; out_update must point to a
/// SidereonFusionTimeSyncUpdate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_update_tight_broadcast_time_sync(
    filter: *mut SidereonFusionFilter,
    broadcast: *const SidereonBroadcastEphemeris,
    epoch: *const SidereonFusionTightEpoch,
    out_update: *mut SidereonFusionTimeSyncUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_update_tight_broadcast_time_sync",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_update_tight_broadcast_time_sync",
                "filter"
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_fusion_filter_update_tight_broadcast_time_sync",
                "broadcast"
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_fusion_filter_update_tight_broadcast_time_sync",
                "out_update"
            ));
            *out_update = zero_time_sync_update();
            let epoch = c_try!(tight_epoch_from_c(
                "sidereon_fusion_filter_update_tight_broadcast_time_sync",
                epoch
            ));
            match filter
                .inner
                .update_tight_time_sync(&broadcast.inner, &epoch)
            {
                Ok(update) => {
                    *out_update = time_sync_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_fusion_error(
                    "sidereon_fusion_filter_update_tight_broadcast_time_sync",
                    err,
                ),
            }
        },
    )
}

/// Replace retained-history capacities for later time-synchronized updates.
///
/// Safety: filter must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_configure_time_sync(
    filter: *mut SidereonFusionFilter,
    imu_capacity: usize,
    checkpoint_capacity: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_configure_time_sync",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_configure_time_sync",
                "filter"
            ));
            let config = sidereon_core::fusion::TimeSyncHistoryConfig::new(
                imu_capacity,
                checkpoint_capacity,
            );
            match filter.inner.configure_time_sync_history(config) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_fusion_error("sidereon_fusion_filter_configure_time_sync", err),
            }
        },
    )
}

/// Copy retained-history capacity and occupancy for time synchronization.
///
/// Safety: filter must be a live handle and out must point to a
/// SidereonFusionTimeSyncStatus.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_time_sync_status(
    filter: *const SidereonFusionFilter,
    out: *mut SidereonFusionTimeSyncStatus,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_time_sync_status",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_fusion_filter_time_sync_status",
                "filter"
            ));
            let out = c_try!(require_out(
                out,
                "sidereon_fusion_filter_time_sync_status",
                "out"
            ));
            *out = time_sync_status_to_c(filter.inner.time_sync_history_status());
            SidereonStatus::Ok
        },
    )
}

/// Encode the current fusion state, including retained time-sync history, as
/// versioned bytes.
///
/// Safety: filter must be a live handle; out must point to len bytes or NULL
/// when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_encode_state(
    filter: *const SidereonFusionFilter,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_encode_state",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_filter_encode_state",
                out_written,
                out_required
            ));
            let filter = c_try!(require_ref(
                filter,
                "sidereon_fusion_filter_encode_state",
                "filter"
            ));
            let bytes = match filter.inner.encode_state() {
                Ok(bytes) => bytes,
                Err(err) => {
                    return map_fusion_codec_error("sidereon_fusion_filter_encode_state", err)
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_filter_encode_state",
                "out",
                &bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Restore a fusion filter from bytes produced by
/// sidereon_fusion_filter_encode_state.
///
/// Safety: filter must be a live handle and data must point to len readable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_filter_restore_state(
    filter: *mut SidereonFusionFilter,
    data: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_filter_restore_state",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_fusion_filter_restore_state",
                "filter"
            ));
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_fusion_filter_restore_state",
                "data"
            ));
            match filter.inner.restore_encoded_state(bytes) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_fusion_codec_error("sidereon_fusion_filter_restore_state", err),
            }
        },
    )
}

/// Create an empty recorded fusion history builder.
///
/// Safety: out_history must point to storage for a
/// SidereonFusionRtsHistoryBuilder*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_builder_new(
    out_history: *mut *mut SidereonFusionRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_builder_new",
        SidereonStatus::Panic,
        || {
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_fusion_rts_history_builder_new",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            write_boxed_handle(
                out_history,
                SidereonFusionRtsHistoryBuilder {
                    inner: sidereon_core::fusion::FusionRtsHistoryBuilder::empty(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Create a recorded fusion history builder from the filter's current checkpoint.
///
/// Safety: filter must be a live handle and out_history must point to storage
/// for a SidereonFusionRtsHistoryBuilder*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_builder_from_filter(
    filter: *const SidereonFusionFilter,
    out_history: *mut *mut SidereonFusionRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_builder_from_filter",
        SidereonStatus::Panic,
        || {
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_fusion_rts_history_builder_from_filter",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            let filter = c_try!(require_ref(
                filter,
                "sidereon_fusion_rts_history_builder_from_filter",
                "filter"
            ));
            let inner =
                match sidereon_core::fusion::FusionRtsHistoryBuilder::from_filter(&filter.inner) {
                    Ok(inner) => inner,
                    Err(err) => {
                        return map_fusion_error(
                            "sidereon_fusion_rts_history_builder_from_filter",
                            err,
                        )
                    }
                };
            write_boxed_handle(out_history, SidereonFusionRtsHistoryBuilder { inner });
            SidereonStatus::Ok
        },
    )
}

/// Validate and clone a recorded fusion history out of a builder.
///
/// Safety: history must be a live builder handle and out_history must point to
/// storage for a SidereonFusionRtsHistory*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_builder_finish(
    history: *const SidereonFusionRtsHistoryBuilder,
    out_history: *mut *mut SidereonFusionRtsHistory,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_builder_finish",
        SidereonStatus::Panic,
        || {
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_fusion_rts_history_builder_finish",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_builder_finish",
                "history"
            ));
            let inner = match history.inner.clone().finish() {
                Ok(inner) => inner,
                Err(err) => {
                    return map_fusion_error("sidereon_fusion_rts_history_builder_finish", err)
                }
            };
            write_boxed_handle(out_history, SidereonFusionRtsHistory { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a recorded fusion history builder. Passing NULL is a no-op.
///
/// Safety: history must be NULL or a live SidereonFusionRtsHistoryBuilder
/// handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_builder_free(
    history: *mut SidereonFusionRtsHistoryBuilder,
) {
    ffi_boundary("sidereon_fusion_rts_history_builder_free", (), || {
        free_boxed(history);
    });
}

/// Return the number of epochs in a recorded fusion RTS history.
///
/// Safety: history must be a live handle and out_count must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_epoch_count(
    history: *const SidereonFusionRtsHistory,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_epoch_count",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_epoch_count",
                "history"
            ));
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_fusion_rts_history_epoch_count",
                "out_count"
            ));
            *out_count = history.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy a recorded fusion RTS epoch summary.
///
/// Safety: history must be a live handle and out_epoch must point to a
/// SidereonFusionRtsEpoch.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_epoch(
    history: *const SidereonFusionRtsHistory,
    index: usize,
    out_epoch: *mut SidereonFusionRtsEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_epoch",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_epoch",
                "history"
            ));
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_fusion_rts_history_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_fusion_rts_epoch();
            let epoch = c_try!(fusion_history_epoch(
                "sidereon_fusion_rts_history_epoch",
                history,
                index
            ));
            *out_epoch = fusion_rts_epoch_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Copy a recorded epoch's predicted ECEF position in meters.
///
/// Safety: history must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_epoch_predicted_position_ecef_m(
    history: *const SidereonFusionRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_epoch_predicted_position_ecef_m",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_rts_history_epoch_predicted_position_ecef_m",
                out_written,
                out_required
            ));
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_epoch_predicted_position_ecef_m",
                "history"
            ));
            let epoch = c_try!(fusion_history_epoch(
                "sidereon_fusion_rts_history_epoch_predicted_position_ecef_m",
                history,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_rts_history_epoch_predicted_position_ecef_m",
                "out",
                &epoch.predicted.state.nominal.position_ecef_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a recorded epoch's updated ECEF position in meters.
///
/// Safety: history must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_epoch_updated_position_ecef_m(
    history: *const SidereonFusionRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_epoch_updated_position_ecef_m",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_rts_history_epoch_updated_position_ecef_m",
                out_written,
                out_required
            ));
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_epoch_updated_position_ecef_m",
                "history"
            ));
            let epoch = c_try!(fusion_history_epoch(
                "sidereon_fusion_rts_history_epoch_updated_position_ecef_m",
                history,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_rts_history_epoch_updated_position_ecef_m",
                "out",
                &epoch.updated.state.nominal.position_ecef_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a recorded epoch transition matrix in row-major order.
///
/// Safety: history must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_epoch_transition_from_previous(
    history: *const SidereonFusionRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_rts_history_epoch_transition_from_previous",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_fusion_rts_history_epoch_transition_from_previous",
                out_written,
                out_required
            ));
            let history = c_try!(require_ref(
                history,
                "sidereon_fusion_rts_history_epoch_transition_from_previous",
                "history"
            ));
            let epoch = c_try!(fusion_history_epoch(
                "sidereon_fusion_rts_history_epoch_transition_from_previous",
                history,
                index
            ));
            let values = epoch
                .transition_from_previous
                .as_ref()
                .map_or_else(Vec::new, |transition| flatten_matrix(transition));
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_rts_history_epoch_transition_from_previous",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a recorded fusion RTS history. Passing NULL is a no-op.
///
/// Safety: history must be NULL or a live SidereonFusionRtsHistory handle that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_rts_history_free(history: *mut SidereonFusionRtsHistory) {
    ffi_boundary("sidereon_fusion_rts_history_free", (), || {
        free_boxed(history);
    });
}

/// Apply fixed-interval RTS smoothing to a recorded fusion history.
///
/// Safety: history must be a live handle and out_smoothed must point to storage
/// for a SidereonSmoothedFusionTrajectory*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smooth_fusion_rts(
    history: *const SidereonFusionRtsHistory,
    out_smoothed: *mut *mut SidereonSmoothedFusionTrajectory,
) -> SidereonStatus {
    ffi_boundary("sidereon_smooth_fusion_rts", SidereonStatus::Panic, || {
        let out_smoothed = c_try!(require_out(
            out_smoothed,
            "sidereon_smooth_fusion_rts",
            "out_smoothed"
        ));
        *out_smoothed = ptr::null_mut();
        let history = c_try!(require_ref(
            history,
            "sidereon_smooth_fusion_rts",
            "history"
        ));
        let inner = match sidereon_core::fusion::smooth_fusion_rts(&history.inner) {
            Ok(inner) => inner,
            Err(err) => return map_fusion_error("sidereon_smooth_fusion_rts", err),
        };
        write_boxed_handle(out_smoothed, SidereonSmoothedFusionTrajectory { inner });
        SidereonStatus::Ok
    })
}

/// Blend a first good post-outage fix back over an outage span. The matched
/// states use the same count as the input states and follow the variable-length
/// output contract.
///
/// Safety: states must point to state_count rows or NULL when state_count is 0;
/// first_good_fix and config must point to readable structs; out_states must
/// point to out_state_len writable rows or be NULL when out_state_len is 0;
/// out_written, out_required, and out_trajectory must point to writable values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fusion_velocity_match_outage(
    states: *const SidereonFusionVelocityMatchState,
    state_count: usize,
    first_good_fix: *const SidereonFusionLooseMeasurement,
    config: *const SidereonFusionVelocityMatchingConfig,
    out_states: *mut SidereonFusionVelocityMatchState,
    out_state_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_trajectory: *mut SidereonFusionVelocityMatchedTrajectory,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fusion_velocity_match_outage",
        SidereonStatus::Panic,
        || {
            let out_trajectory = c_try!(require_out(
                out_trajectory,
                "sidereon_fusion_velocity_match_outage",
                "out_trajectory"
            ));
            *out_trajectory = SidereonFusionVelocityMatchedTrajectory {
                state_count: 0,
                endpoint_position_correction_ecef_m: [0.0; 3],
                endpoint_velocity_correction_ecef_mps: [0.0; 3],
            };
            let raw_states = c_try!(require_slice(
                states,
                state_count,
                "sidereon_fusion_velocity_match_outage",
                "states"
            ));
            let mut core_states = Vec::with_capacity(raw_states.len());
            for state in raw_states {
                match sidereon_core::fusion::VelocityMatchState::new(
                    state.t_j2000_s,
                    state.position_ecef_m,
                    state.velocity_ecef_mps,
                ) {
                    Ok(state) => core_states.push(state),
                    Err(err) => {
                        return map_fusion_error("sidereon_fusion_velocity_match_outage", err)
                    }
                }
            }
            let first_good_fix = c_try!(loose_measurement_from_c(
                "sidereon_fusion_velocity_match_outage",
                first_good_fix
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_fusion_velocity_match_outage",
                "config"
            ));
            let matched = match sidereon_core::fusion::velocity_match_outage(
                &core_states,
                &first_good_fix,
                sidereon_core::fusion::VelocityMatchingConfig {
                    max_outage_duration_s: config.max_outage_duration_s,
                },
            ) {
                Ok(matched) => matched,
                Err(err) => return map_fusion_error("sidereon_fusion_velocity_match_outage", err),
            };
            let rows: Vec<SidereonFusionVelocityMatchState> = matched
                .states
                .iter()
                .map(|state| SidereonFusionVelocityMatchState {
                    t_j2000_s: state.t_j2000_s,
                    position_ecef_m: state.position_ecef_m,
                    velocity_ecef_mps: state.velocity_ecef_mps,
                })
                .collect();
            *out_trajectory = SidereonFusionVelocityMatchedTrajectory {
                state_count: rows.len(),
                endpoint_position_correction_ecef_m: matched.endpoint_position_correction_ecef_m,
                endpoint_velocity_correction_ecef_mps: matched
                    .endpoint_velocity_correction_ecef_mps,
            };
            c_try!(copy_prefix_to_c(
                "sidereon_fusion_velocity_match_outage",
                "out_states",
                &rows,
                out_states,
                out_state_len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Return the number of epochs in a smoothed fusion trajectory.
///
/// Safety: smoothed must be a live handle and out_count must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch_count(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch_count",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch_count",
                "smoothed"
            ));
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_smoothed_fusion_trajectory_epoch_count",
                "out_count"
            ));
            *out_count = smoothed.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy a smoothed fusion epoch summary.
///
/// Safety: smoothed must be a live handle and out_epoch must point to a
/// SidereonSmoothedFusionEpoch.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    index: usize,
    out_epoch: *mut SidereonSmoothedFusionEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch",
                "smoothed"
            ));
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_smoothed_fusion_trajectory_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_smoothed_fusion_epoch();
            let epoch = c_try!(smoothed_fusion_epoch(
                "sidereon_smoothed_fusion_trajectory_epoch",
                smoothed,
                index
            ));
            *out_epoch = smoothed_fusion_epoch_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Copy a smoothed epoch's ECEF position in meters.
///
/// Safety: smoothed must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m",
                out_written,
                out_required
            ));
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_fusion_epoch(
                "sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m",
                smoothed,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_fusion_trajectory_epoch_position_ecef_m",
                "out",
                &epoch.snapshot.state.nominal.position_ecef_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a smoothed epoch correction vector.
///
/// Safety: smoothed must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch_error_state_correction(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch_error_state_correction",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smoothed_fusion_trajectory_epoch_error_state_correction",
                out_written,
                out_required
            ));
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch_error_state_correction",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_fusion_epoch(
                "sidereon_smoothed_fusion_trajectory_epoch_error_state_correction",
                smoothed,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_fusion_trajectory_epoch_error_state_correction",
                "out",
                &epoch.error_state_correction,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a smoothed epoch covariance matrix in row-major order.
///
/// Safety: smoothed must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch_covariance(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smoothed_fusion_trajectory_epoch_covariance",
                out_written,
                out_required
            ));
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch_covariance",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_fusion_epoch(
                "sidereon_smoothed_fusion_trajectory_epoch_covariance",
                smoothed,
                index
            ));
            let values = flatten_matrix(&epoch.covariance);
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_fusion_trajectory_epoch_covariance",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a smoothed epoch RTS gain to the next epoch in row-major order.
///
/// Safety: smoothed must be a live handle; out must point to len doubles or
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next(
    smoothed: *const SidereonSmoothedFusionTrajectory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next",
                out_written,
                out_required
            ));
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_fusion_epoch(
                "sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next",
                smoothed,
                index
            ));
            let values = epoch
                .rts_gain_to_next
                .as_ref()
                .map_or_else(Vec::new, |gain| flatten_matrix(gain));
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_fusion_trajectory_epoch_rts_gain_to_next",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a smoothed fusion trajectory. Passing NULL is a no-op.
///
/// Safety: smoothed must be NULL or a live SidereonSmoothedFusionTrajectory
/// handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_fusion_trajectory_free(
    smoothed: *mut SidereonSmoothedFusionTrajectory,
) {
    ffi_boundary("sidereon_smoothed_fusion_trajectory_free", (), || {
        free_boxed(smoothed);
    });
}

fn fusion_imu_grade_from_c(
    fn_name: &str,
    grade: u32,
) -> Result<sidereon_core::fusion::ImuGrade, SidereonStatus> {
    match grade {
        value if value == SidereonFusionImuGrade::Mems as u32 => {
            Ok(sidereon_core::fusion::ImuGrade::Mems)
        }
        value if value == SidereonFusionImuGrade::Tactical as u32 => {
            Ok(sidereon_core::fusion::ImuGrade::Tactical)
        }
        value if value == SidereonFusionImuGrade::Navigation as u32 => {
            Ok(sidereon_core::fusion::ImuGrade::Navigation)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid IMU grade"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn fusion_filter_kind_from_c(
    fn_name: &str,
    kind: u32,
) -> Result<sidereon_core::fusion::FusionFilterKind, SidereonStatus> {
    match kind {
        value if value == SidereonFusionFilterKind::Ekf as u32 => {
            Ok(sidereon_core::fusion::FusionFilterKind::Ekf)
        }
        value if value == SidereonFusionFilterKind::Ukf as u32 => {
            Ok(sidereon_core::fusion::FusionFilterKind::Ukf)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid fusion filter kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn fusion_filter_kind_label_from_c(
    fn_name: &str,
    kind: u32,
) -> Result<&'static str, SidereonStatus> {
    match fusion_filter_kind_from_c(fn_name, kind)? {
        sidereon_core::fusion::FusionFilterKind::Ekf => Ok("FusionFilterKind.EKF"),
        sidereon_core::fusion::FusionFilterKind::Ukf => Ok("FusionFilterKind.UKF"),
    }
}

fn fusion_layout_from_c(
    fn_name: &str,
    layout: u32,
) -> Result<sidereon_core::fusion::ErrorStateLayout, SidereonStatus> {
    match layout {
        value if value == SidereonFusionErrorStateLayout::Fifteen as u32 => {
            Ok(sidereon_core::fusion::ErrorStateLayout::Fifteen)
        }
        value if value == SidereonFusionErrorStateLayout::TwentyOne as u32 => {
            Ok(sidereon_core::fusion::ErrorStateLayout::TwentyOne)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid fusion error-state layout"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn fusion_error_state_layout_label_from_c(
    fn_name: &str,
    layout: u32,
) -> Result<&'static str, SidereonStatus> {
    match fusion_layout_from_c(fn_name, layout)? {
        sidereon_core::fusion::ErrorStateLayout::Fifteen => Ok("ErrorStateLayout.FIFTEEN"),
        sidereon_core::fusion::ErrorStateLayout::TwentyOne => Ok("ErrorStateLayout.TWENTY_ONE"),
    }
}

fn fusion_fix_status_from_c(
    fn_name: &str,
    status: u32,
) -> Result<sidereon_core::fusion::GnssFixStatus, SidereonStatus> {
    match status {
        value if value == SidereonFusionGnssFixStatus::Single as u32 => {
            Ok(sidereon_core::fusion::GnssFixStatus::Single)
        }
        value if value == SidereonFusionGnssFixStatus::Float as u32 => {
            Ok(sidereon_core::fusion::GnssFixStatus::Float)
        }
        value if value == SidereonFusionGnssFixStatus::Fixed as u32 => {
            Ok(sidereon_core::fusion::GnssFixStatus::Fixed)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid GNSS fix status"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn fusion_gnss_fix_status_label_from_c(
    fn_name: &str,
    status: u32,
) -> Result<&'static str, SidereonStatus> {
    match fusion_fix_status_from_c(fn_name, status)? {
        sidereon_core::fusion::GnssFixStatus::Single => Ok("GnssFixStatus.SINGLE"),
        sidereon_core::fusion::GnssFixStatus::Float => Ok("GnssFixStatus.FLOAT"),
        sidereon_core::fusion::GnssFixStatus::Fixed => Ok("GnssFixStatus.FIXED"),
    }
}

fn coning_correction_from_c(
    fn_name: &str,
    value: u32,
) -> Result<sidereon_core::fusion::ConingCorrection, SidereonStatus> {
    match value {
        v if v == SidereonFusionConingCorrection::Off as u32 => {
            Ok(sidereon_core::fusion::ConingCorrection::Off)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid coning correction"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn fusion_imu_spec_to_c(spec: sidereon_core::fusion::ImuSpec) -> SidereonFusionImuSpec {
    SidereonFusionImuSpec {
        accel_vrw_mps_sqrt_s: spec.accel_vrw_mps_sqrt_s,
        gyro_arw_rad_sqrt_s: spec.gyro_arw_rad_sqrt_s,
        accel_bias_instab_mps2: spec.accel_bias_instab_mps2,
        gyro_bias_instab_rps: spec.gyro_bias_instab_rps,
        accel_bias_tau_s: spec.accel_bias_tau_s,
        gyro_bias_tau_s: spec.gyro_bias_tau_s,
        has_accel_scale_instab_ppm: spec.accel_scale_instab_ppm.is_some(),
        accel_scale_instab_ppm: spec.accel_scale_instab_ppm.unwrap_or(0.0),
        has_gyro_scale_instab_ppm: spec.gyro_scale_instab_ppm.is_some(),
        gyro_scale_instab_ppm: spec.gyro_scale_instab_ppm.unwrap_or(0.0),
    }
}

fn imu_spec_from_c(raw: SidereonFusionImuSpec) -> sidereon_core::fusion::ImuSpec {
    sidereon_core::fusion::ImuSpec::datasheet(
        raw.accel_vrw_mps_sqrt_s,
        raw.gyro_arw_rad_sqrt_s,
        raw.accel_bias_instab_mps2,
        raw.gyro_bias_instab_rps,
        raw.accel_bias_tau_s,
        raw.gyro_bias_tau_s,
        raw.has_accel_scale_instab_ppm
            .then_some(raw.accel_scale_instab_ppm),
        raw.has_gyro_scale_instab_ppm
            .then_some(raw.gyro_scale_instab_ppm),
    )
}

fn zero_fusion_innovation_gate() -> SidereonFusionInnovationGate {
    SidereonFusionInnovationGate {
        threshold_sigma: 0.0,
        min_rows: 0,
    }
}

fn fusion_igg_iii_measurement_reweighting_to_c(
    value: sidereon_core::fusion::IggIiiMeasurementReweighting,
) -> SidereonFusionIggIiiMeasurementReweighting {
    SidereonFusionIggIiiMeasurementReweighting {
        k0_sigma: value.k0_sigma,
        k1_sigma: value.k1_sigma,
    }
}

fn fusion_yang_prediction_adaptive_factor_to_c(
    value: sidereon_core::fusion::YangPredictionAdaptiveFactor,
) -> SidereonFusionYangPredictionAdaptiveFactor {
    SidereonFusionYangPredictionAdaptiveFactor {
        threshold: value.threshold,
        outlier_gate_probability: value.outlier_gate_probability,
    }
}

fn fusion_fix_status_weighting_to_c(
    value: sidereon_core::fusion::GnssFixStatusWeighting,
) -> SidereonFusionFixStatusWeighting {
    SidereonFusionFixStatusWeighting {
        single_sigma_multiplier: value.single_sigma_multiplier,
        float_sigma_multiplier: value.float_sigma_multiplier,
        fixed_sigma_multiplier: value.fixed_sigma_multiplier,
    }
}

fn zero_stationary_update_config() -> SidereonFusionStationaryUpdateConfig {
    SidereonFusionStationaryUpdateConfig {
        detector: SidereonFusionStationaryDetectorConfig {
            window_len: 0,
            max_specific_force_norm_error_mps2: 0.0,
            max_body_rate_wrt_ecef_norm_rps: 0.0,
        },
        zero_velocity_sigma_mps: 0.0,
        zero_angular_rate_sigma_rps: 0.0,
    }
}

fn zero_non_holonomic_config() -> SidereonFusionNonHolonomicConstraintConfig {
    SidereonFusionNonHolonomicConstraintConfig {
        lateral_velocity_sigma_mps: 0.0,
        vertical_velocity_sigma_mps: 0.0,
        min_speed_mps: 0.0,
        max_body_rate_wrt_ecef_norm_rps: 0.0,
    }
}

fn flat9_from_c(values: [f64; 9]) -> [[f64; 3]; 3] {
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn fusion_innovation_gate_from_c(
    has_gate: bool,
    gate: SidereonFusionInnovationGate,
) -> Option<sidereon_core::fusion::InnovationGate> {
    has_gate.then_some(sidereon_core::fusion::InnovationGate {
        threshold_sigma: gate.threshold_sigma,
        min_rows: gate.min_rows,
    })
}

fn fusion_filter_config_from_c(
    fn_name: &str,
    raw: &SidereonFusionFilterConfig,
) -> Result<sidereon_core::fusion::InertialFilterConfig, SidereonStatus> {
    let imu_spec = imu_spec_from_c(raw.imu_spec);
    let mut config = match sidereon_core::fusion::InertialFilterConfig::new(imu_spec) {
        Ok(config) => config,
        Err(err) => return Err(map_fusion_error(fn_name, err)),
    };
    config.filter_kind = fusion_filter_kind_from_c(fn_name, raw.filter_kind)?;
    config.imu_model = sidereon_core::fusion::ImuErrorModel {
        bias: sidereon_core::fusion::ImuBias {
            accel_mps2: raw.imu_bias_accel_mps2,
            gyro_rps: raw.imu_bias_gyro_rps,
        },
        calibration: sidereon_core::fusion::ImuCalibration {
            accel_scale_misalignment: flat9_from_c(raw.imu_accel_scale_misalignment),
            gyro_scale_misalignment: flat9_from_c(raw.imu_gyro_scale_misalignment),
        },
    };
    config.imu_to_body_dcm = flat9_from_c(raw.imu_to_body_dcm);
    config.mechanization = sidereon_core::fusion::MechanizationConfig {
        coning_correction: coning_correction_from_c(fn_name, raw.mechanization.coning_correction)?,
    };
    config.loose.lever_arm_body_m = raw.loose_lever_arm_body_m;
    config.loose.fix_status_weighting = sidereon_core::fusion::GnssFixStatusWeighting {
        single_sigma_multiplier: raw.loose_fix_status_weighting.single_sigma_multiplier,
        float_sigma_multiplier: raw.loose_fix_status_weighting.float_sigma_multiplier,
        fixed_sigma_multiplier: raw.loose_fix_status_weighting.fixed_sigma_multiplier,
    };
    config.loose.update_options.innovation_gate =
        fusion_innovation_gate_from_c(raw.has_loose_innovation_gate, raw.loose_innovation_gate);
    config.loose.measurement_reweighting = raw.has_loose_measurement_reweighting.then_some(
        sidereon_core::fusion::IggIiiMeasurementReweighting {
            k0_sigma: raw.loose_measurement_reweighting.k0_sigma,
            k1_sigma: raw.loose_measurement_reweighting.k1_sigma,
        },
    );
    config.loose.prediction_adaptation = raw.has_loose_prediction_adaptation.then_some(
        sidereon_core::fusion::YangPredictionAdaptiveFactor {
            threshold: raw.loose_prediction_adaptation.threshold,
            outlier_gate_probability: raw.loose_prediction_adaptation.outlier_gate_probability,
        },
    );
    config.loose.stationary_updates =
        raw.has_loose_stationary_updates
            .then_some(sidereon_core::fusion::StationaryUpdateConfig {
                detector: sidereon_core::fusion::StationaryDetectorConfig {
                    window_len: raw.loose_stationary_updates.detector.window_len,
                    max_specific_force_norm_error_mps2: raw
                        .loose_stationary_updates
                        .detector
                        .max_specific_force_norm_error_mps2,
                    max_body_rate_wrt_ecef_norm_rps: raw
                        .loose_stationary_updates
                        .detector
                        .max_body_rate_wrt_ecef_norm_rps,
                },
                zero_velocity_sigma_mps: raw.loose_stationary_updates.zero_velocity_sigma_mps,
                zero_angular_rate_sigma_rps: raw
                    .loose_stationary_updates
                    .zero_angular_rate_sigma_rps,
            });
    config.loose.non_holonomic = raw.has_loose_non_holonomic.then_some(
        sidereon_core::fusion::NonHolonomicConstraintConfig {
            lateral_velocity_sigma_mps: raw.loose_non_holonomic.lateral_velocity_sigma_mps,
            vertical_velocity_sigma_mps: raw.loose_non_holonomic.vertical_velocity_sigma_mps,
            min_speed_mps: raw.loose_non_holonomic.min_speed_mps,
            max_body_rate_wrt_ecef_norm_rps: raw
                .loose_non_holonomic
                .max_body_rate_wrt_ecef_norm_rps,
        },
    );
    config.tight.lever_arm_body_m = raw.tight_lever_arm_body_m;
    config.tight.light_time = raw.tight_light_time;
    config.tight.sagnac = raw.tight_sagnac;
    config.tight.initial_clock_bias_variance_m2 = raw.tight_initial_clock_bias_variance_m2;
    config.tight.initial_clock_drift_variance_m2_s2 = raw.tight_initial_clock_drift_variance_m2_s2;
    config.tight.clock_bias_random_walk_m2_s = raw.tight_clock_bias_random_walk_m2_s;
    config.tight.clock_drift_random_walk_m2_s3 = raw.tight_clock_drift_random_walk_m2_s3;
    config.tight.update_options.innovation_gate =
        fusion_innovation_gate_from_c(raw.has_tight_innovation_gate, raw.tight_innovation_gate);
    config.ukf_update_options.transform = sidereon_core::fusion::UnscentedTransformOptions {
        alpha: raw.ukf_alpha,
        beta: raw.ukf_beta,
        kappa: raw.ukf_kappa,
    };
    config.ukf_update_options.innovation_gate =
        fusion_innovation_gate_from_c(raw.has_ukf_innovation_gate, raw.ukf_innovation_gate);
    if let Err(err) = config.validate() {
        return Err(map_fusion_error(fn_name, err));
    }
    Ok(config)
}

unsafe fn nav_state_from_c(
    fn_name: &str,
    raw: &SidereonFusionNavState,
) -> Result<sidereon_core::fusion::NavState, SidereonStatus> {
    let attitude = read_mat3(
        fn_name,
        "attitude_body_to_ecef",
        raw.attitude_body_to_ecef.as_ptr(),
    )?;
    let state = match sidereon_core::fusion::NavState::new(
        raw.t_j2000_s,
        raw.position_ecef_m,
        raw.velocity_ecef_mps,
        attitude,
    ) {
        Ok(state) => state,
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    match state.with_biases(raw.accel_bias_mps2, raw.gyro_bias_rps) {
        Ok(state) => Ok(state),
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn matrix_from_flat(
    fn_name: &str,
    values: &[f64],
    dimension: usize,
) -> Result<Vec<Vec<f64>>, SidereonStatus> {
    let required = dimension.checked_mul(dimension).ok_or_else(|| {
        set_last_error(format!("{fn_name}: covariance dimension is too large"));
        SidereonStatus::InvalidArgument
    })?;
    if values.len() != required {
        set_last_error(format!(
            "{fn_name}: covariance needs {required} doubles, got {}",
            values.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let mut rows = Vec::with_capacity(dimension);
    for row in 0..dimension {
        let start = row * dimension;
        rows.push(values[start..start + dimension].to_vec());
    }
    Ok(rows)
}

unsafe fn loose_measurement_from_c(
    fn_name: &str,
    measurement: *const SidereonFusionLooseMeasurement,
) -> Result<sidereon_core::fusion::GnssFixMeasurement, SidereonStatus> {
    let raw = require_ref(measurement, fn_name, "measurement")?;
    let dimension = if raw.has_velocity { 6 } else { 3 };
    let cov_values = require_slice(raw.covariance, raw.covariance_len, fn_name, "covariance")?;
    let covariance = matrix_from_flat(fn_name, cov_values, dimension)?;
    let measurement = sidereon_core::fusion::GnssFixMeasurement {
        t_j2000_s: raw.t_j2000_s,
        position_ecef_m: raw.position_ecef_m,
        velocity_ecef_mps: raw.has_velocity.then_some(raw.velocity_ecef_mps),
        covariance,
        satellites_used: raw.satellites_used,
        solution_valid: raw.solution_valid,
        fix_status: fusion_fix_status_from_c(fn_name, raw.fix_status)?,
    };
    match measurement.validate() {
        Ok(()) => Ok(measurement),
        Err(err) => Err(map_fusion_error(fn_name, err)),
    }
}

unsafe fn imu_sample_from_c(
    fn_name: &str,
    raw: &SidereonFusionImuSample,
) -> Result<sidereon_core::fusion::ImuSample, SidereonStatus> {
    match raw.kind {
        value if value == SidereonFusionImuSampleKind::Rate as u32 => {
            Ok(sidereon_core::fusion::ImuSample::rate(
                raw.t_j2000_s,
                raw.specific_force_mps2,
                raw.angular_rate_rps,
            ))
        }
        value if value == SidereonFusionImuSampleKind::Increment as u32 => {
            Ok(sidereon_core::fusion::ImuSample::increment(
                raw.t_j2000_s,
                raw.delta_velocity_mps,
                raw.delta_theta_rad,
                raw.dt_s,
            ))
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid IMU sample kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn tight_epoch_from_c(
    fn_name: &str,
    epoch: *const SidereonFusionTightEpoch,
) -> Result<sidereon_core::fusion::TightGnssEpoch, SidereonStatus> {
    let raw = require_ref(epoch, fn_name, "epoch")?;
    let rows = require_slice(
        raw.observations,
        raw.observation_count,
        fn_name,
        "observations",
    )?;
    let mut observations = Vec::with_capacity(rows.len());
    for row in rows {
        let satellite_id = parse_satellite_token(fn_name, row.sat_id)?;
        let range_rate =
            row.has_range_rate
                .then_some(sidereon_core::fusion::TightRangeRateObservation {
                    measured_range_rate_m_s: row.range_rate.measured_range_rate_m_s,
                    sigma_m_s: row.range_rate.sigma_m_s,
                    satellite_clock_drift_m_s: row.range_rate.satellite_clock_drift_m_s,
                });
        let carrier_phase =
            row.has_carrier_phase
                .then_some(sidereon_core::fusion::TightCarrierPhaseObservation {
                    phase_range_m: row.carrier_phase.phase_range_m,
                    sigma_m: row.carrier_phase.sigma_m,
                    float_ambiguity_m: row.carrier_phase.float_ambiguity_m,
                });
        observations.push(sidereon_core::fusion::TightGnssObservation {
            satellite_id,
            pseudorange_m: row.pseudorange_m,
            pseudorange_sigma_m: row.pseudorange_sigma_m,
            range_rate,
            carrier_phase,
            ionosphere_delay_m: row.ionosphere_delay_m,
            troposphere_delay_m: row.troposphere_delay_m,
        });
    }
    match sidereon_core::fusion::TightGnssEpoch::new(raw.t_j2000_s, observations) {
        Ok(epoch) => Ok(epoch),
        Err(err) => Err(map_fusion_error(fn_name, err)),
    }
}

fn fusion_update_to_c(update: &sidereon_core::fusion::FusionUpdate) -> SidereonFusionUpdate {
    SidereonFusionUpdate {
        applied: update.applied,
        nis: update.nis,
        rows: update.rows,
        accepted_rows: update.accepted_rows,
        rejected_rows: update.rejected_rows,
    }
}

fn time_sync_update_to_c(
    update: &sidereon_core::fusion::TimeSyncUpdate,
) -> SidereonFusionTimeSyncUpdate {
    SidereonFusionTimeSyncUpdate {
        update: fusion_update_to_c(&update.update),
        late_measurement: update.late_measurement,
        replayed_imu_segments: update.replayed_imu_segments,
        restored_checkpoint_epoch_j2000_s: update.restored_checkpoint_epoch_j2000_s,
        current_epoch_j2000_s: update.current_epoch_j2000_s,
    }
}

fn time_sync_status_to_c(
    status: sidereon_core::fusion::TimeSyncHistoryStatus,
) -> SidereonFusionTimeSyncStatus {
    SidereonFusionTimeSyncStatus {
        imu_capacity: status.imu_capacity,
        imu_len: status.imu_len,
        checkpoint_capacity: status.checkpoint_capacity,
        checkpoint_len: status.checkpoint_len,
        has_oldest_imu_epoch_j2000_s: status.oldest_imu_epoch_j2000_s.is_some(),
        oldest_imu_epoch_j2000_s: status.oldest_imu_epoch_j2000_s.unwrap_or(0.0),
        has_newest_imu_epoch_j2000_s: status.newest_imu_epoch_j2000_s.is_some(),
        newest_imu_epoch_j2000_s: status.newest_imu_epoch_j2000_s.unwrap_or(0.0),
        has_oldest_checkpoint_epoch_j2000_s: status.oldest_checkpoint_epoch_j2000_s.is_some(),
        oldest_checkpoint_epoch_j2000_s: status.oldest_checkpoint_epoch_j2000_s.unwrap_or(0.0),
        has_newest_checkpoint_epoch_j2000_s: status.newest_checkpoint_epoch_j2000_s.is_some(),
        newest_checkpoint_epoch_j2000_s: status.newest_checkpoint_epoch_j2000_s.unwrap_or(0.0),
    }
}

fn fusion_state_to_c(
    fn_name: &str,
    filter: &sidereon_core::fusion::InertialFilter,
) -> Result<SidereonFusionState, SidereonStatus> {
    let state = filter.state();
    let clock = match filter.tight_clock_state() {
        Ok(clock) => clock,
        Err(err) => return Err(map_fusion_error(fn_name, err)),
    };
    Ok(SidereonFusionState {
        t_j2000_s: state.nominal.t_j2000_s,
        position_ecef_m: state.nominal.position_ecef_m,
        velocity_ecef_mps: state.nominal.velocity_ecef_mps,
        attitude_body_to_ecef: flatten_mat3_local(state.nominal.attitude_body_to_ecef),
        accel_bias_mps2: state.nominal.accel_bias_mps2,
        gyro_bias_rps: state.nominal.gyro_bias_rps,
        accel_scale_factor: state.accel_scale_factor,
        gyro_scale_factor: state.gyro_scale_factor,
        covariance_dimension: state.dimension(),
        last_body_rate_wrt_ecef_rps: filter.last_body_rate_wrt_ecef_rps(),
        tight_clock_bias_m: clock.bias_m,
        tight_clock_drift_m_s: clock.drift_m_s,
        tight_clock_covariance: [
            clock.covariance[0][0],
            clock.covariance[0][1],
            clock.covariance[1][0],
            clock.covariance[1][1],
        ],
    })
}

fn zero_fusion_update() -> SidereonFusionUpdate {
    SidereonFusionUpdate {
        applied: false,
        nis: 0.0,
        rows: 0,
        accepted_rows: 0,
        rejected_rows: 0,
    }
}

fn zero_time_sync_update() -> SidereonFusionTimeSyncUpdate {
    SidereonFusionTimeSyncUpdate {
        update: zero_fusion_update(),
        late_measurement: false,
        replayed_imu_segments: 0,
        restored_checkpoint_epoch_j2000_s: 0.0,
        current_epoch_j2000_s: 0.0,
    }
}

fn flatten_mat3_local(m: [[f64; 3]; 3]) -> [f64; 9] {
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    ]
}

fn flatten_matrix(matrix: &[Vec<f64>]) -> Vec<f64> {
    let mut values = Vec::with_capacity(matrix.iter().map(Vec::len).sum());
    for row in matrix {
        values.extend_from_slice(row);
    }
    values
}

fn fusion_rts_epoch_to_c(epoch: &sidereon_core::fusion::FusionRtsEpoch) -> SidereonFusionRtsEpoch {
    SidereonFusionRtsEpoch {
        t_j2000_s: epoch.t_j2000_s,
        covariance_dimension: epoch.updated.state.dimension(),
        augmented_dimension: epoch.updated.tight.augmented_covariance.len(),
        has_transition_from_previous: epoch.transition_from_previous.is_some(),
    }
}

fn empty_fusion_rts_epoch() -> SidereonFusionRtsEpoch {
    SidereonFusionRtsEpoch {
        t_j2000_s: 0.0,
        covariance_dimension: 0,
        augmented_dimension: 0,
        has_transition_from_previous: false,
    }
}

fn smoothed_fusion_epoch_to_c(
    epoch: &sidereon_core::fusion::SmoothedFusionEpoch,
) -> SidereonSmoothedFusionEpoch {
    SidereonSmoothedFusionEpoch {
        t_j2000_s: epoch.t_j2000_s,
        covariance_dimension: epoch.covariance.len(),
        correction_len: epoch.error_state_correction.len(),
        has_rts_gain_to_next: epoch.rts_gain_to_next.is_some(),
    }
}

fn empty_smoothed_fusion_epoch() -> SidereonSmoothedFusionEpoch {
    SidereonSmoothedFusionEpoch {
        t_j2000_s: 0.0,
        covariance_dimension: 0,
        correction_len: 0,
        has_rts_gain_to_next: false,
    }
}

fn fusion_history_epoch<'a>(
    fn_name: &str,
    history: &'a SidereonFusionRtsHistory,
    index: usize,
) -> Result<&'a sidereon_core::fusion::FusionRtsEpoch, SidereonStatus> {
    history.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: history epoch index out of range"));
        SidereonStatus::InvalidArgument
    })
}

fn smoothed_fusion_epoch<'a>(
    fn_name: &str,
    smoothed: &'a SidereonSmoothedFusionTrajectory,
    index: usize,
) -> Result<&'a sidereon_core::fusion::SmoothedFusionEpoch, SidereonStatus> {
    smoothed.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: smoothed epoch index out of range"));
        SidereonStatus::InvalidArgument
    })
}

fn map_fusion_error(fn_name: &str, err: sidereon_core::fusion::FusionError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::fusion::FusionError::SingularInnovation
        | sidereon_core::fusion::FusionError::NonPositiveSemidefinite { .. }
        | sidereon_core::fusion::FusionError::NonPositiveDefinite { .. } => SidereonStatus::Solve,
        sidereon_core::fusion::FusionError::InvalidInput { .. }
        | sidereon_core::fusion::FusionError::DimensionMismatch { .. }
        | sidereon_core::fusion::FusionError::NominalState => SidereonStatus::InvalidArgument,
    }
}

fn map_fusion_codec_error(
    fn_name: &str,
    err: sidereon_core::fusion::FusionStateCodecError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
