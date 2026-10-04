use super::*;

pub struct SidereonInertialMechanizer {
    inner: sidereon_core::inertial::StrapdownMechanizer,
}

pub struct SidereonImuSimulator {
    inner: sidereon_core::inertial::ImuSimulator,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialNavState {
    pub t_j2000_s: f64,
    pub position_ecef_m: [f64; 3],
    pub velocity_ecef_mps: [f64; 3],
    pub attitude_body_to_ecef: [f64; 9],
    pub accel_bias_mps2: [f64; 3],
    pub gyro_bias_rps: [f64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialImuModel {
    pub accel_bias_mps2: [f64; 3],
    pub gyro_bias_rps: [f64; 3],
    pub accel_scale_misalignment: [f64; 9],
    pub gyro_scale_misalignment: [f64; 9],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialSimulationOptions {
    pub output: u32,
    pub seed: u64,
    pub initial_accel_bias_mps2: [f64; 3],
    pub initial_gyro_bias_rps: [f64; 3],
    pub accel_scale_misalignment: [f64; 9],
    pub gyro_scale_misalignment: [f64; 9],
    pub has_rate_random_walk: bool,
    pub accel_random_walk_mps2_sqrt_s: f64,
    pub gyro_random_walk_rps_sqrt_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialIncrement {
    pub t_j2000_s: f64,
    pub delta_velocity_mps: [f64; 3],
    pub delta_theta_rad: [f64; 3],
    pub dt_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialSimulatorState {
    pub accel_bias_mps2: [f64; 3],
    pub gyro_bias_rps: [f64; 3],
    pub accel_rate_random_walk_mps2: [f64; 3],
    pub gyro_rate_random_walk_rps: [f64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialQuaternion {
    /// Scalar-first unit-quaternion scalar component.
    pub w: f64,
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialConstants {
    /// Default seed used by the core simulator options.
    pub default_imu_sim_seed: u64,
    /// WGS84 normal gravity at the equator in m/s².
    pub normal_gravity_equator_mps2: f64,
    /// WGS84 normal gravity at the pole in m/s².
    pub normal_gravity_pole_mps2: f64,
    /// WGS84 Somigliana gravity constant.
    pub somigliana_k: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonInertialErrorKind {
    None = 0,
    InvalidInput = 1,
    NonMonotonicSample = 2,
    SingularCalibration = 3,
    DegenerateAttitude = 4,
    InvalidTag = 5,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonInertialErrorText {
    Field = 0,
    Reason = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialError {
    pub kind: SidereonInertialErrorKind,
    pub has_field: bool,
    pub has_reason: bool,
}

#[derive(Clone)]
struct InertialErrorDetails {
    error: SidereonInertialError,
    field: Option<String>,
    reason: Option<String>,
}

thread_local! {
    static LAST_INERTIAL_ERROR: std::cell::RefCell<Option<InertialErrorDetails>> = const { std::cell::RefCell::new(None) };
}

fn inertial_mat3(values: [f64; 9]) -> sidereon_core::astro::math::mat3::Mat3 {
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn inertial_state_from_c(
    state: &SidereonInertialNavState,
) -> Result<sidereon_core::inertial::NavState, sidereon_core::inertial::InertialError> {
    sidereon_core::inertial::NavState::new(
        state.t_j2000_s,
        state.position_ecef_m,
        state.velocity_ecef_mps,
        inertial_mat3(state.attitude_body_to_ecef),
    )?
    .with_biases(state.accel_bias_mps2, state.gyro_bias_rps)
}

fn inertial_state_to_c(state: &sidereon_core::inertial::NavState) -> SidereonInertialNavState {
    let attitude = state.attitude_body_to_ecef;
    SidereonInertialNavState {
        t_j2000_s: state.t_j2000_s,
        position_ecef_m: state.position_ecef_m,
        velocity_ecef_mps: state.velocity_ecef_mps,
        attitude_body_to_ecef: [
            attitude[0][0],
            attitude[0][1],
            attitude[0][2],
            attitude[1][0],
            attitude[1][1],
            attitude[1][2],
            attitude[2][0],
            attitude[2][1],
            attitude[2][2],
        ],
        accel_bias_mps2: state.accel_bias_mps2,
        gyro_bias_rps: state.gyro_bias_rps,
    }
}

fn inertial_increment_from_c(
    increment: &SidereonInertialIncrement,
) -> sidereon_core::inertial::CorrectedImuIncrement {
    sidereon_core::inertial::CorrectedImuIncrement {
        t_j2000_s: increment.t_j2000_s,
        delta_velocity_mps: increment.delta_velocity_mps,
        delta_theta_rad: increment.delta_theta_rad,
        dt_s: increment.dt_s,
    }
}

fn inertial_simulation_options_from_c(
    options: &SidereonInertialSimulationOptions,
) -> Result<sidereon_core::inertial::ImuSimulationOptions, u32> {
    let output = match options.output {
        0 => sidereon_core::inertial::ImuSimulationOutput::Rate,
        1 => sidereon_core::inertial::ImuSimulationOutput::Increment,
        other => return Err(other),
    };
    let mut converted = sidereon_core::inertial::ImuSimulationOptions::default();
    converted.output = output;
    converted.seed = options.seed;
    converted.initial_bias = sidereon_core::inertial::ImuBias {
        accel_mps2: options.initial_accel_bias_mps2,
        gyro_rps: options.initial_gyro_bias_rps,
    };
    converted.calibration = sidereon_core::inertial::ImuCalibration {
        accel_scale_misalignment: inertial_mat3(options.accel_scale_misalignment),
        gyro_scale_misalignment: inertial_mat3(options.gyro_scale_misalignment),
    };
    converted.rate_random_walk =
        options
            .has_rate_random_walk
            .then_some(sidereon_core::inertial::ImuRateRandomWalk::new(
                options.accel_random_walk_mps2_sqrt_s,
                options.gyro_random_walk_rps_sqrt_s,
            ));
    Ok(converted)
}

/// Write a simulated sequence into validated caller-owned output arrays.
///
/// # Safety
/// Both pointers must be aligned and writable for `count` elements. The
/// caller must have validated their ranges and rejected overlap first.
unsafe fn inertial_write_simulated_sequence(
    sequence: sidereon_core::inertial::SimulatedImuSequence,
    count: usize,
    out_samples: *mut SidereonFusionImuSample,
    out_bias: *mut SidereonInertialSimulatorState,
) {
    for (index, ((sample, bias), random_walk)) in sequence
        .samples
        .into_iter()
        .zip(sequence.bias_history)
        .zip(sequence.rate_random_walk_history)
        .take(count)
        .enumerate()
    {
        out_samples
            .add(index)
            .write(inertial_imu_sample_to_c(sample));
        out_bias.add(index).write(SidereonInertialSimulatorState {
            accel_bias_mps2: bias.accel_mps2,
            gyro_bias_rps: bias.gyro_rps,
            accel_rate_random_walk_mps2: random_walk.accel_mps2,
            gyro_rate_random_walk_rps: random_walk.gyro_rps,
        });
    }
}

fn inertial_mat3_to_c(value: sidereon_core::astro::math::mat3::Mat3) -> [f64; 9] {
    [
        value[0][0],
        value[0][1],
        value[0][2],
        value[1][0],
        value[1][1],
        value[1][2],
        value[2][0],
        value[2][1],
        value[2][2],
    ]
}

fn inertial_error(fn_name: &str, error: sidereon_core::inertial::InertialError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {error}"));
    LAST_INERTIAL_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(inertial_error_details(&error));
    });
    SidereonStatus::InvalidArgument
}

fn inertial_error_details(error: &sidereon_core::inertial::InertialError) -> InertialErrorDetails {
    use sidereon_core::inertial::InertialError as CoreError;
    let (kind, field, reason) = match error {
        CoreError::InvalidInput { field, reason } => (
            SidereonInertialErrorKind::InvalidInput,
            Some((*field).to_owned()),
            Some((*reason).to_owned()),
        ),
        CoreError::NonMonotonicSample => {
            (SidereonInertialErrorKind::NonMonotonicSample, None, None)
        }
        CoreError::SingularCalibration => {
            (SidereonInertialErrorKind::SingularCalibration, None, None)
        }
        CoreError::DegenerateAttitude => {
            (SidereonInertialErrorKind::DegenerateAttitude, None, None)
        }
    };
    InertialErrorDetails {
        error: SidereonInertialError {
            kind,
            has_field: field.is_some(),
            has_reason: reason.is_some(),
        },
        field,
        reason,
    }
}

fn inertial_invalid_tag(fn_name: &str, field: &str, value: u32) -> SidereonStatus {
    set_last_error(format!("{fn_name}: invalid {field} tag {value}"));
    LAST_INERTIAL_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(InertialErrorDetails {
            error: SidereonInertialError {
                kind: SidereonInertialErrorKind::InvalidTag,
                has_field: true,
                has_reason: true,
            },
            field: Some(field.to_owned()),
            reason: Some(format!("unknown integer tag {value}")),
        });
    });
    SidereonStatus::InvalidArgument
}

/// Copy the last typed inertial failure for this thread, or `None` when no
/// inertial operation has failed.
///
/// # Safety
/// `out_error` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_last_error(
    out_error: *mut SidereonInertialError,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_last_error";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN, "out_error"));
        *out = LAST_INERTIAL_ERROR.with(|slot| {
            slot.borrow().as_ref().map_or(
                SidereonInertialError {
                    kind: SidereonInertialErrorKind::None,
                    has_field: false,
                    has_reason: false,
                },
                |details| details.error,
            )
        });
        SidereonStatus::Ok
    })
}

/// Copy a text field of the last typed inertial failure; bytes are not
/// NUL-terminated and use the standard count-query buffer convention.
///
/// # Safety
/// `out_written` and `out_required` must be writable. `out` must reference
/// `len` writable bytes unless `len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_last_error_text(
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_last_error_text";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let value = LAST_INERTIAL_ERROR.with(|slot| {
            slot.borrow().as_ref().and_then(|details| match part {
                value if value == SidereonInertialErrorText::Field as u32 => details.field.clone(),
                value if value == SidereonInertialErrorText::Reason as u32 => {
                    details.reason.clone()
                }
                _ => None,
            })
        });
        if part > SidereonInertialErrorText::Reason as u32 {
            set_last_error(format!("{FN}: invalid text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            value.unwrap_or_default().as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn inertial_imu_sample_from_c(
    fn_name: &str,
    sample: &SidereonFusionImuSample,
) -> Result<sidereon_core::inertial::ImuSample, SidereonStatus> {
    match sample.kind {
        value if value == SidereonFusionImuSampleKind::Rate as u32 => {
            Ok(sidereon_core::inertial::ImuSample::rate(
                sample.t_j2000_s,
                sample.specific_force_mps2,
                sample.angular_rate_rps,
            ))
        }
        value if value == SidereonFusionImuSampleKind::Increment as u32 => {
            Ok(sidereon_core::inertial::ImuSample::increment(
                sample.t_j2000_s,
                sample.delta_velocity_mps,
                sample.delta_theta_rad,
                sample.dt_s,
            ))
        }
        _ => {
            let _ = inertial_invalid_tag(fn_name, "sample.kind", sample.kind);
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn inertial_imu_sample_to_c(sample: sidereon_core::inertial::ImuSample) -> SidereonFusionImuSample {
    use sidereon_core::inertial::ImuSampleKind;
    let mut result = SidereonFusionImuSample {
        t_j2000_s: sample.t_j2000_s,
        kind: SidereonFusionImuSampleKind::Increment as u32,
        specific_force_mps2: [0.0; 3],
        angular_rate_rps: [0.0; 3],
        delta_velocity_mps: [0.0; 3],
        delta_theta_rad: [0.0; 3],
        dt_s: 0.0,
    };
    match sample.kind {
        ImuSampleKind::Rate {
            specific_force_mps2,
            angular_rate_rps,
        } => {
            result.kind = SidereonFusionImuSampleKind::Rate as u32;
            result.specific_force_mps2 = specific_force_mps2;
            result.angular_rate_rps = angular_rate_rps;
        }
        ImuSampleKind::Increment {
            delta_velocity_mps,
            delta_theta_rad,
            dt_s,
        } => {
            result.delta_velocity_mps = delta_velocity_mps;
            result.delta_theta_rad = delta_theta_rad;
            result.dt_s = dt_s;
        }
    }
    result
}

fn inertial_imu_spec_from_c(spec: SidereonFusionImuSpec) -> sidereon_core::inertial::ImuSpec {
    sidereon_core::inertial::ImuSpec::datasheet(
        spec.accel_vrw_mps_sqrt_s,
        spec.gyro_arw_rad_sqrt_s,
        spec.accel_bias_instab_mps2,
        spec.gyro_bias_instab_rps,
        spec.accel_bias_tau_s,
        spec.gyro_bias_tau_s,
        spec.has_accel_scale_instab_ppm
            .then_some(spec.accel_scale_instab_ppm),
        spec.has_gyro_scale_instab_ppm
            .then_some(spec.gyro_scale_instab_ppm),
    )
}

fn inertial_imu_spec_to_c(spec: sidereon_core::inertial::ImuSpec) -> SidereonFusionImuSpec {
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

/// Create an ECEF strapdown mechanizer with the default zero-bias IMU model.
///
/// # Safety
/// `initial` must reference a readable navigation state and `out` a writable
/// handle pointer. The returned handle must be released with
/// `sidereon_inertial_mechanizer_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_new(
    initial: *const SidereonInertialNavState,
    out: *mut *mut SidereonInertialMechanizer,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanizer_new";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let initial = c_try!(require_ref(initial, FN, "initial"));
        let initial = match inertial_state_from_c(initial) {
            Ok(state) => state,
            Err(error) => return inertial_error(FN, error),
        };
        let inner = match sidereon_core::inertial::StrapdownMechanizer::new(initial) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        write_boxed_handle(out, SidereonInertialMechanizer { inner });
        SidereonStatus::Ok
    })
}

/// Create a mechanizer with caller-supplied sensor bias and calibration.
///
/// # Safety
/// `initial`, `model`, and `out` must point to readable, readable, and writable
/// values respectively. The returned handle must be freed with
/// `sidereon_inertial_mechanizer_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_new_with_model(
    initial: *const SidereonInertialNavState,
    model: *const SidereonInertialImuModel,
    out: *mut *mut SidereonInertialMechanizer,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanizer_new_with_model";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let initial = c_try!(require_ref(initial, FN, "initial"));
        let model = c_try!(require_ref(model, FN, "model"));
        let initial = match inertial_state_from_c(initial) {
            Ok(state) => state,
            Err(error) => return inertial_error(FN, error),
        };
        let model = sidereon_core::inertial::ImuErrorModel {
            bias: sidereon_core::inertial::ImuBias {
                accel_mps2: model.accel_bias_mps2,
                gyro_rps: model.gyro_bias_rps,
            },
            calibration: sidereon_core::inertial::ImuCalibration {
                accel_scale_misalignment: inertial_mat3(model.accel_scale_misalignment),
                gyro_scale_misalignment: inertial_mat3(model.gyro_scale_misalignment),
            },
        };
        let inner = match sidereon_core::inertial::StrapdownMechanizer::new(initial)
            .and_then(|value| value.with_imu_model(model))
        {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        write_boxed_handle(out, SidereonInertialMechanizer { inner });
        SidereonStatus::Ok
    })
}

/// Create a mechanizer using the caller-selected core coning-correction mode.
///
/// # Safety
/// `initial` and `config` must point to readable values; `out` must point to a
/// writable handle pointer. Free a returned handle with
/// `sidereon_inertial_mechanizer_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_new_with_config(
    initial: *const SidereonInertialNavState,
    config: *const SidereonFusionMechanizationConfig,
    out: *mut *mut SidereonInertialMechanizer,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanizer_new_with_config";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let initial = c_try!(require_ref(initial, FN, "initial"));
        let config = c_try!(require_ref(config, FN, "config"));
        if config.coning_correction != SidereonFusionConingCorrection::Off as u32 {
            return inertial_invalid_tag(FN, "config.coning_correction", config.coning_correction);
        }
        let initial = match inertial_state_from_c(initial) {
            Ok(state) => state,
            Err(error) => return inertial_error(FN, error),
        };
        let core_config = sidereon_core::inertial::MechanizationConfig::default();
        let inner = sidereon_core::inertial::StrapdownMechanizer::new(initial)
            .map(|value| value.with_config(core_config));
        match inner {
            Ok(inner) => {
                write_boxed_handle(out, SidereonInertialMechanizer { inner });
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Propagate one rate or increment sample and copy out the updated state.
///
/// # Safety
/// `mechanizer` must be a live handle; `sample` and `out_state` must point to
/// readable and writable values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_propagate(
    mechanizer: *mut SidereonInertialMechanizer,
    sample: *const SidereonFusionImuSample,
    out_state: *mut SidereonInertialNavState,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanizer_propagate";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN, "out_state"));
        let mechanizer = c_try!(require_mut(mechanizer, FN, "mechanizer"));
        let sample = c_try!(require_ref(sample, FN, "sample"));
        let sample = c_try!(inertial_imu_sample_from_c(FN, sample));
        match mechanizer.inner.propagate(sample) {
            Ok(state) => {
                *out_state = inertial_state_to_c(state);
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Copy the current mechanizer state without advancing it.
///
/// # Safety
/// `mechanizer` must be a live handle; `out_state` must point to writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_state(
    mechanizer: *const SidereonInertialMechanizer,
    out_state: *mut SidereonInertialNavState,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanizer_state";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN, "out_state"));
        let mechanizer = c_try!(require_ref(mechanizer, FN, "mechanizer"));
        *out_state = inertial_state_to_c(mechanizer.inner.state());
        SidereonStatus::Ok
    })
}

/// Release a mechanizer handle; null is accepted.
///
/// # Safety
/// `mechanizer` must be null or a live, not-yet-freed handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanizer_free(
    mechanizer: *mut SidereonInertialMechanizer,
) {
    ffi_boundary("sidereon_inertial_mechanizer_free", (), || {
        free_boxed(mechanizer)
    });
}

/// Create the deterministic synthetic IMU simulator from an IMU specification
/// and caller options.
///
/// `output` is 0 for rate samples and 1 for integrated increments.
///
/// # Safety
/// `spec`, `options`, and `out` must point to readable, readable, and writable
/// values. Release the returned handle with `sidereon_imu_simulator_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_imu_simulator_new(
    spec: *const SidereonFusionImuSpec,
    options: *const SidereonInertialSimulationOptions,
    out: *mut *mut SidereonImuSimulator,
) -> SidereonStatus {
    const FN: &str = "sidereon_imu_simulator_new";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let spec = c_try!(require_ref(spec, FN, "spec"));
        let options = c_try!(require_ref(options, FN, "options"));
        let options = match inertial_simulation_options_from_c(options) {
            Ok(value) => value,
            Err(value) => return inertial_invalid_tag(FN, "options.output", value),
        };
        let inner = match sidereon_core::inertial::ImuSimulator::new(
            inertial_imu_spec_from_c(*spec),
            options,
        ) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        write_boxed_handle(out, SidereonImuSimulator { inner });
        SidereonStatus::Ok
    })
}

/// Generate one noisy sample from a truth increment and return updated noise
/// states.
///
/// # Safety
/// `simulator`, `truth`, `out_sample`, and `out_state` must be live/writable
/// pointers as appropriate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_imu_simulator_sample_increment(
    simulator: *mut SidereonImuSimulator,
    truth: *const SidereonInertialIncrement,
    out_sample: *mut SidereonFusionImuSample,
    out_state: *mut SidereonInertialSimulatorState,
) -> SidereonStatus {
    const FN: &str = "sidereon_imu_simulator_sample_increment";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_sample = c_try!(require_out(out_sample, FN, "out_sample"));
        let out_state = c_try!(require_out(out_state, FN, "out_state"));
        let simulator = c_try!(require_mut(simulator, FN, "simulator"));
        let truth = c_try!(require_ref(truth, FN, "truth"));
        let truth = sidereon_core::inertial::CorrectedImuIncrement {
            t_j2000_s: truth.t_j2000_s,
            delta_velocity_mps: truth.delta_velocity_mps,
            delta_theta_rad: truth.delta_theta_rad,
            dt_s: truth.dt_s,
        };
        let sample = match simulator.inner.sample_increment(&truth) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        *out_sample = inertial_imu_sample_to_c(sample);
        let bias = simulator.inner.bias();
        let random_walk = simulator.inner.rate_random_walk();
        *out_state = SidereonInertialSimulatorState {
            accel_bias_mps2: bias.accel_mps2,
            gyro_bias_rps: bias.gyro_rps,
            accel_rate_random_walk_mps2: random_walk.accel_mps2,
            gyro_rate_random_walk_rps: random_walk.gyro_rps,
        };
        SidereonStatus::Ok
    })
}

/// Copy the current simulator bias and rate-random-walk state without drawing
/// another sample.
///
/// # Safety
/// `simulator` must be a live handle and `out_state` must point to writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_imu_simulator_state(
    simulator: *const SidereonImuSimulator,
    out_state: *mut SidereonInertialSimulatorState,
) -> SidereonStatus {
    const FN: &str = "sidereon_imu_simulator_state";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN, "out_state"));
        let simulator = c_try!(require_ref(simulator, FN, "simulator"));
        let bias = simulator.inner.bias();
        let random_walk = simulator.inner.rate_random_walk();
        *out_state = SidereonInertialSimulatorState {
            accel_bias_mps2: bias.accel_mps2,
            gyro_bias_rps: bias.gyro_rps,
            accel_rate_random_walk_mps2: random_walk.accel_mps2,
            gyro_rate_random_walk_rps: random_walk.gyro_rps,
        };
        SidereonStatus::Ok
    })
}

/// Release a synthetic IMU simulator; null is accepted.
///
/// # Safety
/// `simulator` must be null or a live, not-yet-freed handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_imu_simulator_free(simulator: *mut SidereonImuSimulator) {
    ffi_boundary("sidereon_imu_simulator_free", (), || free_boxed(simulator));
}

/// Return one built-in IMU stochastic specification; grade tags are 0 MEMS,
/// 1 tactical, and 2 navigation.
///
/// # Safety
/// `out_spec` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_imu_spec_preset(
    grade: u32,
    out_spec: *mut SidereonFusionImuSpec,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_imu_spec_preset";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_spec, FN, "out_spec"));
        *out = SidereonFusionImuSpec {
            accel_vrw_mps_sqrt_s: 0.0,
            gyro_arw_rad_sqrt_s: 0.0,
            accel_bias_instab_mps2: 0.0,
            gyro_bias_instab_rps: 0.0,
            accel_bias_tau_s: 0.0,
            gyro_bias_tau_s: 0.0,
            has_accel_scale_instab_ppm: false,
            accel_scale_instab_ppm: 0.0,
            has_gyro_scale_instab_ppm: false,
            gyro_scale_instab_ppm: 0.0,
        };
        let grade = match grade {
            0 => sidereon_core::inertial::ImuGrade::Mems,
            1 => sidereon_core::inertial::ImuGrade::Tactical,
            2 => sidereon_core::inertial::ImuGrade::Navigation,
            value => return inertial_invalid_tag(FN, "grade", value),
        };
        *out = inertial_imu_spec_to_c(sidereon_core::inertial::ImuSpec::preset(grade));
        SidereonStatus::Ok
    })
}

/// Validate a caller-built IMU stochastic specification.
///
/// # Safety
/// `spec` must point to a readable specification.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_imu_spec_validate(
    spec: *const SidereonFusionImuSpec,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_imu_spec_validate";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let spec = c_try!(require_ref(spec, FN, "spec"));
        match inertial_imu_spec_from_c(*spec).validate() {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Validate model biases and calibration matrices without propagating a sample.
///
/// # Safety
/// `model` must point to a readable IMU error-model record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_imu_model_validate(
    model: *const SidereonInertialImuModel,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_imu_model_validate";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let model = c_try!(require_ref(model, FN, "model"));
        let bias = sidereon_core::inertial::ImuBias {
            accel_mps2: model.accel_bias_mps2,
            gyro_rps: model.gyro_bias_rps,
        };
        let calibration = sidereon_core::inertial::ImuCalibration {
            accel_scale_misalignment: inertial_mat3(model.accel_scale_misalignment),
            gyro_scale_misalignment: inertial_mat3(model.gyro_scale_misalignment),
        };
        match bias.validate().and_then(|()| calibration.validate()) {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Validate optional rate-random-walk densities in SI units.
///
/// # Safety
/// This call has no pointer arguments.
#[no_mangle]
pub extern "C" fn sidereon_inertial_rate_random_walk_validate(
    accel_mps2_sqrt_s: f64,
    gyro_rps_sqrt_s: f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_rate_random_walk_validate";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        match sidereon_core::inertial::ImuRateRandomWalk::new(accel_mps2_sqrt_s, gyro_rps_sqrt_s)
            .validate()
        {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Copy immutable inertial-model constants to caller storage.
///
/// # Safety
/// `out_constants` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_constants(
    out_constants: *mut SidereonInertialConstants,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_constants";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_constants, FN, "out_constants"));
        *out = SidereonInertialConstants {
            default_imu_sim_seed: sidereon_core::inertial::DEFAULT_IMU_SIM_SEED,
            normal_gravity_equator_mps2: sidereon_core::inertial::WGS84_NORMAL_GRAVITY_EQUATOR_MPS2,
            normal_gravity_pole_mps2: sidereon_core::inertial::WGS84_NORMAL_GRAVITY_POLE_MPS2,
            somigliana_k: sidereon_core::inertial::WGS84_SOMIGLIANA_K,
        };
        SidereonStatus::Ok
    })
}

/// Evaluate accelerometer and gyroscope bias decay and variance increments.
/// Output order is accel decay, gyro decay, accel variance, gyro variance.
///
/// # Safety
/// `spec` must reference a readable record and `out_values` must point to four
/// writable doubles. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_imu_spec_bias_statistics(
    spec: *const SidereonFusionImuSpec,
    dt_s: f64,
    out_values: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_imu_spec_bias_statistics";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<4>(out_values, FN, "out_values"));
        *out = [0.0; 4];
        let spec = c_try!(require_ref(spec, FN, "spec"));
        let spec = inertial_imu_spec_from_c(*spec);
        if let Err(error) = spec.validate() {
            return inertial_error(FN, error);
        }
        let values = (
            spec.accel_bias_decay(dt_s),
            spec.gyro_bias_decay(dt_s),
            spec.accel_bias_variance_increment(dt_s),
            spec.gyro_bias_variance_increment(dt_s),
        );
        match values {
            (Ok(accel_decay), Ok(gyro_decay), Ok(accel_variance), Ok(gyro_variance)) => {
                *out = [accel_decay, gyro_decay, accel_variance, gyro_variance];
                SidereonStatus::Ok
            }
            (Err(error), _, _, _)
            | (_, Err(error), _, _)
            | (_, _, Err(error), _)
            | (_, _, _, Err(error)) => inertial_error(FN, error),
        }
    })
}

/// Build a diagonal IMU calibration from axis scale errors in ppm.
///
/// # Safety
/// `accel_scale_ppm` and `gyro_scale_ppm` must each point to three readable
/// doubles (x, y, z) and `out_model` to writable storage. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_calibration_from_scale_ppm(
    accel_scale_ppm: *const f64,
    gyro_scale_ppm: *const f64,
    out_model: *mut SidereonInertialImuModel,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_calibration_from_scale_ppm";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_model, FN, "out_model"));
        *out = SidereonInertialImuModel {
            accel_bias_mps2: [0.0; 3],
            gyro_bias_rps: [0.0; 3],
            accel_scale_misalignment: [0.0; 9],
            gyro_scale_misalignment: [0.0; 9],
        };
        let accel_scale_ppm = c_try!(require_f64_array::<3>(
            accel_scale_ppm,
            FN,
            "accel_scale_ppm"
        ));
        let gyro_scale_ppm = c_try!(require_f64_array::<3>(gyro_scale_ppm, FN, "gyro_scale_ppm"));
        match sidereon_core::inertial::ImuCalibration::from_scale_ppm(
            *accel_scale_ppm,
            *gyro_scale_ppm,
        ) {
            Ok(value) => {
                (*out).accel_scale_misalignment =
                    inertial_mat3_to_c(value.accel_scale_misalignment);
                (*out).gyro_scale_misalignment = inertial_mat3_to_c(value.gyro_scale_misalignment);
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Correct one raw IMU sample using caller-supplied bias and calibration.
///
/// # Safety
/// `sample`, `model`, and `out_increment` must point to readable, readable,
/// and writable storage respectively.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_correct_sample(
    sample: *const SidereonFusionImuSample,
    previous_t_j2000_s: f64,
    model: *const SidereonInertialImuModel,
    out_increment: *mut SidereonInertialIncrement,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_correct_sample";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_increment, FN, "out_increment"));
        *out = SidereonInertialIncrement {
            t_j2000_s: 0.0,
            delta_velocity_mps: [0.0; 3],
            delta_theta_rad: [0.0; 3],
            dt_s: 0.0,
        };
        let sample = c_try!(require_ref(sample, FN, "sample"));
        let model = c_try!(require_ref(model, FN, "model"));
        let sample = c_try!(inertial_imu_sample_from_c(FN, sample));
        let model = sidereon_core::inertial::ImuErrorModel {
            bias: sidereon_core::inertial::ImuBias {
                accel_mps2: model.accel_bias_mps2,
                gyro_rps: model.gyro_bias_rps,
            },
            calibration: sidereon_core::inertial::ImuCalibration {
                accel_scale_misalignment: inertial_mat3(model.accel_scale_misalignment),
                gyro_scale_misalignment: inertial_mat3(model.gyro_scale_misalignment),
            },
        };
        match model.correct_sample(&sample, previous_t_j2000_s) {
            Ok(value) => {
                *out = SidereonInertialIncrement {
                    t_j2000_s: value.t_j2000_s,
                    delta_velocity_mps: value.delta_velocity_mps,
                    delta_theta_rad: value.delta_theta_rad,
                    dt_s: value.dt_s,
                };
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Generate a sample and bias-history batch from corrected truth increments.
/// All output arrays have `count` entries; their contents are untouched on error.
///
/// # Safety
/// Inputs must reference `count` readable records; output arrays must each
/// reference `count` writable records unless `count` is zero. The two output
/// ranges must not overlap.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_simulate_increments(
    increments: *const SidereonInertialIncrement,
    count: usize,
    spec: *const SidereonFusionImuSpec,
    options: *const SidereonInertialSimulationOptions,
    out_samples: *mut SidereonFusionImuSample,
    out_state_history: *mut SidereonInertialSimulatorState,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_simulate_increments";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let increments = c_try!(require_slice(increments, count, FN, "increments"));
        c_try!(require_out_array(out_samples, count, FN, "out_samples"));
        c_try!(require_out_array(
            out_state_history,
            count,
            FN,
            "out_state_history"
        ));
        if count != 0 {
            let samples = c_try!(super::checked_output_range(
                FN,
                out_samples,
                count,
                "out_samples"
            ));
            let states = c_try!(super::checked_output_range(
                FN,
                out_state_history,
                count,
                "out_state_history"
            ));
            c_try!(super::reject_overlapping_outputs(
                FN,
                samples,
                states,
                "out_samples",
                "out_state_history"
            ));
        }
        let spec = c_try!(require_ref(spec, FN, "spec"));
        let options = c_try!(require_ref(options, FN, "options"));
        let options = match inertial_simulation_options_from_c(options) {
            Ok(value) => value,
            Err(value) => return inertial_invalid_tag(FN, "options.output", value),
        };
        let increments: Vec<_> = increments.iter().map(inertial_increment_from_c).collect();
        let sequence = match sidereon_core::inertial::simulate_imu_samples_from_increments(
            &increments,
            inertial_imu_spec_from_c(*spec),
            options,
        ) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        if count != 0 {
            inertial_write_simulated_sequence(sequence, count, out_samples, out_state_history);
        }
        SidereonStatus::Ok
    })
}

/// Generate a sample and bias-history batch from a navigation-state trajectory.
/// The output arrays have `count - 1` entries and are untouched on error.
///
/// # Safety
/// `trajectory` must reference `count` readable states; output arrays must
/// reference `count - 1` writable records when `count > 1`. The two output
/// ranges must not overlap.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_simulate_trajectory(
    trajectory: *const SidereonInertialNavState,
    count: usize,
    spec: *const SidereonFusionImuSpec,
    options: *const SidereonInertialSimulationOptions,
    out_samples: *mut SidereonFusionImuSample,
    out_state_history: *mut SidereonInertialSimulatorState,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_simulate_trajectory";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let trajectory = c_try!(require_slice(trajectory, count, FN, "trajectory"));
        let output_count = count.saturating_sub(1);
        c_try!(require_out_array(
            out_samples,
            output_count,
            FN,
            "out_samples"
        ));
        c_try!(require_out_array(
            out_state_history,
            output_count,
            FN,
            "out_state_history"
        ));
        if output_count != 0 {
            let samples = c_try!(super::checked_output_range(
                FN,
                out_samples,
                output_count,
                "out_samples"
            ));
            let states = c_try!(super::checked_output_range(
                FN,
                out_state_history,
                output_count,
                "out_state_history"
            ));
            c_try!(super::reject_overlapping_outputs(
                FN,
                samples,
                states,
                "out_samples",
                "out_state_history"
            ));
        }
        let spec = c_try!(require_ref(spec, FN, "spec"));
        let options = c_try!(require_ref(options, FN, "options"));
        let options = match inertial_simulation_options_from_c(options) {
            Ok(value) => value,
            Err(value) => return inertial_invalid_tag(FN, "options.output", value),
        };
        let mut states = Vec::with_capacity(count);
        for value in trajectory {
            match inertial_state_from_c(value) {
                Ok(state) => states.push(state),
                Err(error) => return inertial_error(FN, error),
            }
        }
        let sequence = match sidereon_core::inertial::simulate_imu_samples(
            &states,
            inertial_imu_spec_from_c(*spec),
            options,
        ) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        if output_count != 0 {
            inertial_write_simulated_sequence(
                sequence,
                output_count,
                out_samples,
                out_state_history,
            );
        }
        SidereonStatus::Ok
    })
}

/// Convert a validated body-to-ECEF direction cosine matrix to a unit quaternion.
///
/// # Safety
/// `dcm` must point to nine readable doubles (row-major) and `out` to writable
/// storage. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_dcm_to_quaternion(
    dcm: *const f64,
    out: *mut SidereonInertialQuaternion,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_dcm_to_quaternion";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = SidereonInertialQuaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let dcm = c_try!(require_f64_array::<9>(dcm, FN, "dcm"));
        let dcm = inertial_mat3(*dcm);
        match sidereon_core::inertial::dcm_to_quaternion(&dcm) {
            Ok(value) => {
                *out = SidereonInertialQuaternion {
                    w: value.w,
                    x: value.x,
                    y: value.y,
                    z: value.z,
                };
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Convert a unit quaternion to a row-major body-to-ECEF direction cosine matrix.
///
/// # Safety
/// `quaternion` must point to a readable record and `out` to nine writable
/// doubles (row-major). A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_quaternion_to_dcm(
    quaternion: *const SidereonInertialQuaternion,
    out: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_quaternion_to_dcm";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<9>(out, FN, "out"));
        *out = [0.0; 9];
        let quaternion = c_try!(require_ref(quaternion, FN, "quaternion"));
        let quaternion = match sidereon_core::inertial::AttitudeQuaternion::new(
            quaternion.w,
            quaternion.x,
            quaternion.y,
            quaternion.z,
        ) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        *out = inertial_mat3_to_c(sidereon_core::inertial::quaternion_to_dcm(quaternion));
        SidereonStatus::Ok
    })
}

/// Re-orthonormalize a row-major 3-by-3 direction cosine matrix.
///
/// # Safety
/// `dcm` must point to nine readable doubles and `out` to nine writable doubles,
/// both row-major. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_reorthonormalize_dcm(
    dcm: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_reorthonormalize_dcm";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<9>(out, FN, "out"));
        *out = [0.0; 9];
        let dcm = c_try!(require_f64_array::<9>(dcm, FN, "dcm"));
        match sidereon_core::inertial::reorthonormalize_dcm(&inertial_mat3(*dcm)) {
            Ok(value) => {
                *out = inertial_mat3_to_c(value);
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Extract yaw, pitch, and roll radians from a row-major body-to-ECEF matrix.
///
/// # Safety
/// `dcm` must point to nine readable doubles (row-major) and
/// `out_yaw_pitch_roll_rad` to three writable doubles. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_attitude_yaw_pitch_roll(
    dcm: *const f64,
    out_yaw_pitch_roll_rad: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_attitude_yaw_pitch_roll";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<3>(
            out_yaw_pitch_roll_rad,
            FN,
            "out_yaw_pitch_roll_rad"
        ));
        *out = [0.0; 3];
        let dcm = c_try!(require_f64_array::<9>(dcm, FN, "dcm"));
        *out = sidereon_core::inertial::attitude_yaw_pitch_roll_rad(&inertial_mat3(*dcm));
        SidereonStatus::Ok
    })
}

/// Generate a Rodrigues body rotation from an angular increment.
///
/// # Safety
/// `delta_theta_rad` must point to three readable doubles and `out_dcm` to nine
/// writable doubles (row-major). A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_rodrigues_delta_dcm(
    delta_theta_rad: *const f64,
    out_dcm: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_rodrigues_delta_dcm";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<9>(out_dcm, FN, "out_dcm"));
        *out = [0.0; 9];
        let delta = c_try!(require_f64_array::<3>(
            delta_theta_rad,
            FN,
            "delta_theta_rad"
        ));
        match sidereon_core::inertial::rodrigues_delta_dcm(*delta) {
            Ok(value) => {
                *out = inertial_mat3_to_c(value);
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Propagate a navigation state by one corrected increment without a handle.
///
/// # Safety
/// `state`, `increment`, and `config` must be readable; `out_state` writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_mechanize_ecef(
    state: *const SidereonInertialNavState,
    increment: *const SidereonInertialIncrement,
    config: *const SidereonFusionMechanizationConfig,
    out_state: *mut SidereonInertialNavState,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_mechanize_ecef";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_state, FN, "out_state"));
        *out = SidereonInertialNavState {
            t_j2000_s: 0.0,
            position_ecef_m: [0.0; 3],
            velocity_ecef_mps: [0.0; 3],
            attitude_body_to_ecef: [0.0; 9],
            accel_bias_mps2: [0.0; 3],
            gyro_bias_rps: [0.0; 3],
        };
        let state = c_try!(require_ref(state, FN, "state"));
        let increment = c_try!(require_ref(increment, FN, "increment"));
        let config = c_try!(require_ref(config, FN, "config"));
        if config.coning_correction != SidereonFusionConingCorrection::Off as u32 {
            return inertial_invalid_tag(FN, "config.coning_correction", config.coning_correction);
        }
        let state = match inertial_state_from_c(state) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        let increment = inertial_increment_from_c(increment);
        let config = sidereon_core::inertial::MechanizationConfig::default();
        match sidereon_core::inertial::mechanize_ecef(&state, &increment, config) {
            Ok(value) => {
                *out = inertial_state_to_c(&value);
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Compute normal gravity magnitude at geodetic latitude and ellipsoidal height.
///
/// # Safety
/// `out_gravity_mps2` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_normal_gravity(
    latitude_rad: f64,
    height_m: f64,
    out_gravity_mps2: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_normal_gravity";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_gravity_mps2, FN, "out_gravity_mps2"));
        *out = 0.0;
        match sidereon_core::inertial::normal_gravity_mps2(latitude_rad, height_m) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Compute the ECEF gravitational acceleration at an ECEF position.
///
/// # Safety
/// `position_ecef_m` must point to three readable doubles and
/// `out_gravity_ecef_mps2` to three writable doubles. A NULL pointer is refused with SIDEREON_STATUS_NULL_POINTER.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_gravity_ecef(
    position_ecef_m: *const f64,
    out_gravity_ecef_mps2: *mut f64,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_gravity_ecef";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out_f64_array::<3>(
            out_gravity_ecef_mps2,
            FN,
            "out_gravity_ecef_mps2"
        ));
        *out = [0.0; 3];
        let position = c_try!(require_f64_array::<3>(
            position_ecef_m,
            FN,
            "position_ecef_m"
        ));
        match sidereon_core::inertial::gravity_ecef_mps2(*position) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

/// Reconstruct the corrected truth increment between two navigation states.
///
/// # Safety
/// `start`, `end`, and `out_increment` must point to readable/writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_true_increment_between(
    start: *const SidereonInertialNavState,
    end: *const SidereonInertialNavState,
    out_increment: *mut SidereonInertialIncrement,
) -> SidereonStatus {
    const FN: &str = "sidereon_inertial_true_increment_between";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_increment, FN, "out_increment"));
        *out = SidereonInertialIncrement {
            t_j2000_s: 0.0,
            delta_velocity_mps: [0.0; 3],
            delta_theta_rad: [0.0; 3],
            dt_s: 0.0,
        };
        let start = c_try!(require_ref(start, FN, "start"));
        let end = c_try!(require_ref(end, FN, "end"));
        let start = match inertial_state_from_c(start) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        let end = match inertial_state_from_c(end) {
            Ok(value) => value,
            Err(error) => return inertial_error(FN, error),
        };
        match sidereon_core::inertial::true_imu_increment_between(&start, &end) {
            Ok(value) => {
                *out = SidereonInertialIncrement {
                    t_j2000_s: value.t_j2000_s,
                    delta_velocity_mps: value.delta_velocity_mps,
                    delta_theta_rad: value.delta_theta_rad,
                    dt_s: value.dt_s,
                };
                SidereonStatus::Ok
            }
            Err(error) => inertial_error(FN, error),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_state() -> SidereonInertialNavState {
        SidereonInertialNavState {
            t_j2000_s: 0.0,
            position_ecef_m: [6_378_137.0, 0.0, 0.0],
            velocity_ecef_mps: [0.0; 3],
            attitude_body_to_ecef: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            accel_bias_mps2: [0.0; 3],
            gyro_bias_rps: [0.0; 3],
        }
    }

    #[test]
    fn failed_constructor_clears_handle_and_preserves_typed_input_error() {
        let mut initial = identity_state();
        initial.position_ecef_m[0] = f64::NAN;
        let mut handle = std::ptr::dangling_mut::<SidereonInertialMechanizer>();
        let status = unsafe { sidereon_inertial_mechanizer_new(&initial, &mut handle) };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(handle.is_null());
        let mut error = SidereonInertialError {
            kind: SidereonInertialErrorKind::None,
            has_field: false,
            has_reason: false,
        };
        assert_eq!(
            unsafe { sidereon_inertial_last_error(&mut error) },
            SidereonStatus::Ok
        );
        assert_eq!(error.kind, SidereonInertialErrorKind::InvalidInput);
        assert!(error.has_field && error.has_reason);
        let details = LAST_INERTIAL_ERROR.with(|slot| slot.borrow().clone().unwrap());
        assert_eq!(details.field.as_deref(), Some("position_ecef_m"));
        assert_eq!(details.reason.as_deref(), Some("must be finite"));
        let mut text = [0u8; 32];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_inertial_last_error_text(
                    SidereonInertialErrorText::Reason as u32,
                    text.as_mut_ptr(),
                    text.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(&text[..written], b"must be finite");
    }

    #[test]
    fn failed_mutation_retains_live_mechanizer_state_and_typed_error() {
        let initial = identity_state();
        let mut handle = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_inertial_mechanizer_new(&initial, &mut handle) },
            SidereonStatus::Ok
        );
        assert!(!handle.is_null());
        let sample = SidereonFusionImuSample {
            t_j2000_s: initial.t_j2000_s,
            kind: SidereonFusionImuSampleKind::Rate as u32,
            specific_force_mps2: [0.0; 3],
            angular_rate_rps: [0.0; 3],
            delta_velocity_mps: [0.0; 3],
            delta_theta_rad: [0.0; 3],
            dt_s: 0.0,
        };
        let mut state = initial;
        assert_eq!(
            unsafe { sidereon_inertial_mechanizer_propagate(handle, &sample, &mut state) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(state.t_j2000_s.to_bits(), initial.t_j2000_s.to_bits());
        let mut error = SidereonInertialError {
            kind: SidereonInertialErrorKind::None,
            has_field: false,
            has_reason: false,
        };
        assert_eq!(
            unsafe { sidereon_inertial_last_error(&mut error) },
            SidereonStatus::Ok
        );
        assert_eq!(error.kind, SidereonInertialErrorKind::NonMonotonicSample);
        unsafe { sidereon_inertial_mechanizer_free(handle) };
    }

    #[test]
    fn invalid_simulator_output_tag_is_typed_and_transfers_no_handle() {
        let spec = SidereonFusionImuSpec {
            accel_vrw_mps_sqrt_s: 1.0,
            gyro_arw_rad_sqrt_s: 1.0,
            accel_bias_instab_mps2: 1.0,
            gyro_bias_instab_rps: 1.0,
            accel_bias_tau_s: 1.0,
            gyro_bias_tau_s: 1.0,
            has_accel_scale_instab_ppm: false,
            accel_scale_instab_ppm: 0.0,
            has_gyro_scale_instab_ppm: false,
            gyro_scale_instab_ppm: 0.0,
        };
        let options = SidereonInertialSimulationOptions {
            output: 17,
            seed: 1,
            initial_accel_bias_mps2: [0.0; 3],
            initial_gyro_bias_rps: [0.0; 3],
            accel_scale_misalignment: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            gyro_scale_misalignment: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            has_rate_random_walk: false,
            accel_random_walk_mps2_sqrt_s: 0.0,
            gyro_random_walk_rps_sqrt_s: 0.0,
        };
        let mut handle = std::ptr::dangling_mut::<SidereonImuSimulator>();
        assert_eq!(
            unsafe { sidereon_imu_simulator_new(&spec, &options, &mut handle) },
            SidereonStatus::InvalidArgument
        );
        assert!(handle.is_null());
        let details = LAST_INERTIAL_ERROR.with(|slot| slot.borrow().clone().unwrap());
        assert_eq!(details.error.kind, SidereonInertialErrorKind::InvalidTag);
        assert_eq!(details.field.as_deref(), Some("options.output"));
        assert_eq!(details.reason.as_deref(), Some("unknown integer tag 17"));
    }

    #[test]
    fn config_tag_is_checked_and_simulator_state_is_read_without_sampling() {
        let initial = identity_state();
        let config = SidereonFusionMechanizationConfig {
            coning_correction: 99,
        };
        let mut mechanizer = std::ptr::dangling_mut::<SidereonInertialMechanizer>();
        assert_eq!(
            unsafe {
                sidereon_inertial_mechanizer_new_with_config(&initial, &config, &mut mechanizer)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(mechanizer.is_null());
        let config_error = LAST_INERTIAL_ERROR.with(|slot| slot.borrow().clone().unwrap());
        assert_eq!(
            config_error.error.kind,
            SidereonInertialErrorKind::InvalidTag
        );
        assert_eq!(
            config_error.field.as_deref(),
            Some("config.coning_correction")
        );

        let spec = SidereonFusionImuSpec {
            accel_vrw_mps_sqrt_s: 1.0,
            gyro_arw_rad_sqrt_s: 1.0,
            accel_bias_instab_mps2: 1.0,
            gyro_bias_instab_rps: 1.0,
            accel_bias_tau_s: 1.0,
            gyro_bias_tau_s: 1.0,
            has_accel_scale_instab_ppm: false,
            accel_scale_instab_ppm: 0.0,
            has_gyro_scale_instab_ppm: false,
            gyro_scale_instab_ppm: 0.0,
        };
        let options = SidereonInertialSimulationOptions {
            output: 1,
            seed: 2,
            initial_accel_bias_mps2: [0.1, 0.2, 0.3],
            initial_gyro_bias_rps: [0.01, 0.02, 0.03],
            accel_scale_misalignment: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            gyro_scale_misalignment: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            has_rate_random_walk: false,
            accel_random_walk_mps2_sqrt_s: 0.0,
            gyro_random_walk_rps_sqrt_s: 0.0,
        };
        let mut simulator = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_imu_simulator_new(&spec, &options, &mut simulator) },
            SidereonStatus::Ok
        );
        let mut state = SidereonInertialSimulatorState {
            accel_bias_mps2: [0.0; 3],
            gyro_bias_rps: [0.0; 3],
            accel_rate_random_walk_mps2: [1.0; 3],
            gyro_rate_random_walk_rps: [1.0; 3],
        };
        assert_eq!(
            unsafe { sidereon_imu_simulator_state(simulator, &mut state) },
            SidereonStatus::Ok
        );
        assert_eq!(state.accel_bias_mps2, options.initial_accel_bias_mps2);
        assert_eq!(state.gyro_bias_rps, options.initial_gyro_bias_rps);
        assert_eq!(state.accel_rate_random_walk_mps2, [0.0; 3]);
        assert_eq!(state.gyro_rate_random_walk_rps, [0.0; 3]);
        unsafe { sidereon_imu_simulator_free(simulator) };
    }

    #[test]
    fn pure_helpers_return_core_values_and_preserve_typed_refusals() {
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let mut quaternion = SidereonInertialQuaternion {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(
            unsafe { sidereon_inertial_dcm_to_quaternion(identity.as_ptr(), &mut quaternion) },
            SidereonStatus::Ok
        );
        assert_eq!(quaternion.w.to_bits(), 1.0f64.to_bits());
        let mut reconstructed = [0.0; 9];
        assert_eq!(
            unsafe { sidereon_inertial_quaternion_to_dcm(&quaternion, reconstructed.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(reconstructed, identity);

        let invalid = [f64::NAN, 0.0, 0.0];
        let mut matrix = [1.0; 9];
        assert_eq!(
            unsafe { sidereon_inertial_rodrigues_delta_dcm(invalid.as_ptr(), matrix.as_mut_ptr()) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(matrix, [0.0; 9]);
        let details = LAST_INERTIAL_ERROR.with(|slot| slot.borrow().clone().unwrap());
        assert_eq!(details.error.kind, SidereonInertialErrorKind::InvalidInput);
        assert_eq!(details.field.as_deref(), Some("delta_theta_rad"));
        assert!(details.reason.is_some());
    }
}
