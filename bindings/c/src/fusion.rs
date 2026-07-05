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

/// Opaque GNSS/INS fusion filter. Create with sidereon_fusion_filter_create and
/// release with sidereon_fusion_filter_free.
pub struct SidereonFusionFilter {
    pub(crate) inner: sidereon_core::fusion::InertialFilter,
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
    /// Strapdown mechanization options.
    pub mechanization: SidereonFusionMechanizationConfig,
    /// Loose-coupling body-frame lever arm from IMU origin to GNSS antenna, meters.
    pub loose_lever_arm_body_m: [f64; 3],
    /// Whether loose_innovation_gate carries a loose-update screen.
    pub has_loose_innovation_gate: bool,
    /// EKF loose-update innovation screen.
    pub loose_innovation_gate: SidereonFusionInnovationGate,
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
                mechanization: SidereonFusionMechanizationConfig {
                    coning_correction: SidereonFusionConingCorrection::Off as u32,
                },
                loose_lever_arm_body_m: [0.0; 3],
                has_loose_innovation_gate: false,
                loose_innovation_gate: zero_fusion_innovation_gate(),
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
    config.mechanization = sidereon_core::fusion::MechanizationConfig {
        coning_correction: coning_correction_from_c(fn_name, raw.mechanization.coning_correction)?,
    };
    config.loose.lever_arm_body_m = raw.loose_lever_arm_body_m;
    config.loose.update_options.innovation_gate =
        fusion_innovation_gate_from_c(raw.has_loose_innovation_gate, raw.loose_innovation_gate);
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
