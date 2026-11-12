use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};
use sidereon_core::precise_positioning as core;
use std::collections::BTreeMap;

/// Position process-noise model tag.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppKinematicPositionProcessNoiseKind {
    RandomWalk = 0,
    WhiteNoiseAcceleration = 1,
}
/// Position process-noise model. `kind` is validated before conversion.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicPositionProcessNoise {
    pub kind: u32,
    pub spectral_density: f64,
}
/// Deterministic motion-model tag.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppKinematicMotionModelKind {
    Hold = 0,
    ConstantVelocity = 1,
}
/// Deterministic motion model. `kind` is validated before conversion.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicMotionModel {
    pub kind: u32,
    pub velocity_m_s: [f64; 3],
}
/// Kinematic EKF process-noise controls. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicProcessNoise {
    pub position: SidereonPppKinematicPositionProcessNoise,
    pub clock_white_m2_s: f64,
    pub ztd_random_walk_m2_s: f64,
    pub ambiguity_hold_m2_s: f64,
}
/// Fixed fields of a kinematic state.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicStateInfo {
    pub position_m: [f64; 3],
    pub clock_m: f64,
    pub ztd_residual_m: f64,
    pub ambiguity_count: usize,
    pub dimension: usize,
    pub covariance_element_count: usize,
}
/// Input ambiguity value.
#[repr(C)]
pub struct SidereonPppKinematicAmbiguity {
    pub ambiguity_id: *const c_char,
    pub value_m: f64,
}
/// Owned mutable state and covariance driven by core predict/correct operations.
pub struct SidereonPppKinematicState {
    inner: core::KinematicState,
    covariance_m2: Vec<Vec<f64>>,
}
/// Owned kinematic configuration.
pub struct SidereonPppKinematicConfig {
    inner: core::KinematicConfig,
}
/// Fixed configuration fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppKinematicConfigInfo {
    pub motion: SidereonPppKinematicMotionModel,
    pub process_noise: SidereonPppKinematicProcessNoise,
    pub new_ambiguity_variance_m2: f64,
    pub weights: SidereonPppMeasurementWeights,
    pub tropo_enabled: bool,
    pub estimate_ztd: bool,
    pub estimate_tropo_gradients: bool,
    pub initial_covariance_element_count: usize,
}
/// Update summary fixed fields.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicUpdateSummaryInfo {
    pub innovation_rms_m: f64,
    pub used_satellite_count: usize,
    pub ssr_bias_exclusion_count: usize,
    pub unplaced_observation_count: usize,
}
/// Owned measurement-update summary.
pub struct SidereonPppKinematicUpdateSummary {
    inner: core::KinematicUpdateSummary,
}
/// Per-epoch kinematic status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppKinematicEpochStatus {
    Updated = 0,
    PredictedOnly = 1,
}
/// Fixed fields of one epoch solution.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppKinematicEpochSolutionInfo {
    pub position_m: [f64; 3],
    pub clock_m: f64,
    pub ztd_residual_m: f64,
    pub position_covariance_m2: [[f64; 3]; 3],
    pub innovation_rms_m: f64,
    pub status: SidereonPppKinematicEpochStatus,
    pub ambiguity_count: usize,
    pub used_satellite_count: usize,
    pub ssr_bias_exclusion_count: usize,
    pub unplaced_observation_count: usize,
}
/// Owned ordered kinematic solve result.
pub struct SidereonPppKinematicResult {
    inner: Vec<core::KinematicEpochSolution>,
}
/// Kinematic solve error kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppKinematicSolveErrorKind {
    NoEphemeris = 0,
    SingularGeometry = 1,
    InvalidSolveOption = 2,
    InvalidInput = 3,
    MissingCorrection = 4,
    Ut1OutsideCoverage = 5,
}

fn position_noise_to_c(
    value: core::KinematicPositionProcessNoise,
) -> SidereonPppKinematicPositionProcessNoise {
    match value {
        core::KinematicPositionProcessNoise::RandomWalk {
            spectral_density_m2_s,
        } => SidereonPppKinematicPositionProcessNoise {
            kind: 0,
            spectral_density: spectral_density_m2_s,
        },
        core::KinematicPositionProcessNoise::WhiteNoiseAcceleration {
            spectral_density_m2_s3,
        } => SidereonPppKinematicPositionProcessNoise {
            kind: 1,
            spectral_density: spectral_density_m2_s3,
        },
    }
}
fn position_noise_from_c(
    fn_name: &str,
    value: SidereonPppKinematicPositionProcessNoise,
) -> Result<core::KinematicPositionProcessNoise, SidereonStatus> {
    match value.kind {
        0 => Ok(core::KinematicPositionProcessNoise::RandomWalk {
            spectral_density_m2_s: value.spectral_density,
        }),
        1 => Ok(
            core::KinematicPositionProcessNoise::WhiteNoiseAcceleration {
                spectral_density_m2_s3: value.spectral_density,
            },
        ),
        _ => {
            set_last_error(format!("{fn_name}: invalid process_noise.position.kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
fn motion_to_c(value: core::KinematicMotionModel) -> SidereonPppKinematicMotionModel {
    match value {
        core::KinematicMotionModel::Hold => SidereonPppKinematicMotionModel {
            kind: 0,
            velocity_m_s: [0.0; 3],
        },
        core::KinematicMotionModel::ConstantVelocity { velocity_m_s } => {
            SidereonPppKinematicMotionModel {
                kind: 1,
                velocity_m_s,
            }
        }
    }
}
fn motion_from_c(
    fn_name: &str,
    value: SidereonPppKinematicMotionModel,
) -> Result<core::KinematicMotionModel, SidereonStatus> {
    match value.kind {
        0 => Ok(core::KinematicMotionModel::Hold),
        1 => Ok(core::KinematicMotionModel::ConstantVelocity {
            velocity_m_s: value.velocity_m_s,
        }),
        _ => {
            set_last_error(format!("{fn_name}: invalid motion.kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
fn process_to_c(value: core::KinematicProcessNoise) -> SidereonPppKinematicProcessNoise {
    SidereonPppKinematicProcessNoise {
        position: position_noise_to_c(value.position),
        clock_white_m2_s: value.clock_white_m2_s,
        ztd_random_walk_m2_s: value.ztd_random_walk_m2_s,
        ambiguity_hold_m2_s: value.ambiguity_hold_m2_s,
    }
}
fn process_from_c(
    fn_name: &str,
    value: SidereonPppKinematicProcessNoise,
) -> Result<core::KinematicProcessNoise, SidereonStatus> {
    Ok(core::KinematicProcessNoise {
        position: position_noise_from_c(fn_name, value.position)?,
        clock_white_m2_s: value.clock_white_m2_s,
        ztd_random_walk_m2_s: value.ztd_random_walk_m2_s,
        ambiguity_hold_m2_s: value.ambiguity_hold_m2_s,
    })
}
fn weights_to_c(value: core::MeasurementWeights) -> SidereonPppMeasurementWeights {
    SidereonPppMeasurementWeights {
        code: value.code,
        phase: value.phase,
        elevation_weighting: value.elevation_weighting,
    }
}

unsafe fn ambiguity_map(
    fn_name: &str,
    values: *const SidereonPppKinematicAmbiguity,
    count: usize,
) -> Result<BTreeMap<String, f64>, SidereonStatus> {
    let mut out = BTreeMap::new();
    for (index, value) in require_slice(values, count, fn_name, "ambiguities")?
        .iter()
        .enumerate()
    {
        let id = parse_c_string(fn_name, "ambiguity_id", value.ambiguity_id)?;
        if out.insert(id, value.value_m).is_some() {
            set_last_error(format!("{fn_name}: duplicate ambiguity at index {index}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    }
    Ok(out)
}
unsafe fn covariance_from_flat(
    fn_name: &str,
    values: *const f64,
    count: usize,
    dimension: usize,
) -> Result<Vec<Vec<f64>>, SidereonStatus> {
    let required = dimension
        .checked_mul(dimension)
        .ok_or(SidereonStatus::InvalidArgument)?;
    if count != required {
        set_last_error(format!("{fn_name}: covariance_count must equal {required}"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(require_slice(values, count, fn_name, "covariance_m2")?
        .chunks_exact(dimension)
        .map(|row| row.to_vec())
        .collect())
}

fn no_ephemeris_value(value: &core::NoEphemerisReason) -> Value {
    match value {
        core::NoEphemerisReason::NoEphemeris => json!({"kind":"no_ephemeris"}),
        core::NoEphemerisReason::MissingSatelliteClock => json!({"kind":"missing_satellite_clock"}),
        core::NoEphemerisReason::Reason(reason) => json!({"kind":"reason","reason":reason}),
    }
}
fn missing_correction_value(value: &core::MissingCorrection) -> Value {
    use core::MissingCorrection as M;
    match value {
        M::SolidEarthTide => json!({"kind":"solid_earth_tide"}),
        M::PoleTide => json!({"kind":"pole_tide"}),
        M::OceanLoading => json!({"kind":"ocean_loading"}),
        M::PhaseWindup => json!({"kind":"phase_windup"}),
        M::SatelliteAntennaPco => json!({"kind":"satellite_antenna_pco"}),
        M::SatelliteAntennaPcv => json!({"kind":"satellite_antenna_pcv"}),
        M::CodeBias => json!({"kind":"code_bias"}),
        M::SsrCodeBias => json!({"kind":"ssr_code_bias"}),
        M::PhaseBias => json!({"kind":"phase_bias"}),
        M::ReceiverAntennaFrequency(label) => {
            json!({"kind":"receiver_antenna_frequency","label":label})
        }
        M::ReceiverAntennaPcv(label) => json!({"kind":"receiver_antenna_pcv","label":label}),
        M::ReceiverAntennaGeometry => json!({"kind":"receiver_antenna_geometry"}),
    }
}
fn error_value(error: &core::KinematicSolveError) -> Value {
    match error {
        core::KinematicSolveError::NoEphemeris {
            satellite_id,
            reason,
        } => {
            json!({"kind":"no_ephemeris","message":error.to_string(),"fields":{"satellite_id":satellite_id,"reason":no_ephemeris_value(reason)}})
        }
        core::KinematicSolveError::SingularGeometry => {
            json!({"kind":"singular_geometry","message":error.to_string(),"fields":{}})
        }
        core::KinematicSolveError::InvalidSolveOption { field, reason } => {
            json!({"kind":"invalid_solve_option","message":error.to_string(),"fields":{"field":field,"reason":reason}})
        }
        core::KinematicSolveError::InvalidInput { field, reason } => {
            json!({"kind":"invalid_input","message":error.to_string(),"fields":{"field":field,"reason":reason}})
        }
        core::KinematicSolveError::MissingCorrection {
            satellite_id,
            correction,
        } => {
            json!({"kind":"missing_correction","message":error.to_string(),"fields":{"satellite_id":satellite_id,"correction":missing_correction_value(correction)}})
        }
        core::KinematicSolveError::Ut1OutsideCoverage(reason) => {
            json!({"kind":"ut1_outside_coverage","message":error.to_string(),"fields":{"reason":format!("{reason}")}})
        }
    }
}
fn map_error(fn_name: &str, error: core::KinematicSolveError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PppKinematic,
        fn_name,
        error_value(&error),
    );
    set_last_error(format!("{fn_name}: {error}"));
    match error {
        core::KinematicSolveError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        core::KinematicSolveError::InvalidInput { .. }
        | core::KinematicSolveError::InvalidSolveOption { .. } => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}
unsafe fn copy_text(
    fn_name: &str,
    text_value: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    match copy_prefix_to_c(
        fn_name,
        "out",
        text_value.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ) {
        Ok(()) => SidereonStatus::Ok,
        Err(status) => status,
    }
}

/// Allocate the core default kinematic configuration.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_default(
    out: *mut *mut SidereonPppKinematicConfig,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_default";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        write_boxed_handle(
            out,
            SidereonPppKinematicConfig {
                inner: core::KinematicConfig::default(),
            },
        );
        SidereonStatus::Ok
    })
}
/// Read fixed configuration fields.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_info(
    config: *const SidereonPppKinematicConfig,
    out: *mut SidereonPppKinematicConfigInfo,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_info";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", config, "config"
        ));
        let v = &c_try!(require_ref(config, f, "config")).inner;
        c_try!(require_out(out, f, "out")).write(SidereonPppKinematicConfigInfo {
            motion: motion_to_c(v.motion),
            process_noise: process_to_c(v.process_noise),
            new_ambiguity_variance_m2: v.new_ambiguity_variance_m2,
            weights: weights_to_c(v.weights),
            tropo_enabled: v.tropo.enabled,
            estimate_ztd: v.tropo.estimate_ztd,
            estimate_tropo_gradients: v.tropo.estimate_tropo_gradients,
            initial_covariance_element_count: v.initial_covariance_m2.len()
                * v.initial_covariance_m2.len(),
        });
        SidereonStatus::Ok
    })
}
/// Set motion and process noise after validating their tags.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_set_dynamics(
    config: *mut SidereonPppKinematicConfig,
    motion: SidereonPppKinematicMotionModel,
    noise: SidereonPppKinematicProcessNoise,
    new_ambiguity_variance_m2: f64,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_set_dynamics";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let motion = c_try!(motion_from_c(f, motion));
        let noise = c_try!(process_from_c(f, noise));
        let v = &mut c_try!(require_mut(config, f, "config")).inner;
        v.motion = motion;
        v.process_noise = noise;
        v.new_ambiguity_variance_m2 = new_ambiguity_variance_m2;
        SidereonStatus::Ok
    })
}
/// Set measurement weights, troposphere, and corrections from the existing lossless PPP config.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_set_measurement_config(
    config: *mut SidereonPppKinematicConfig,
    input: *const SidereonPppFloatConfig,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_set_measurement_config";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let input = c_try!(require_ref(input, f, "input"));
        let weights = crate::solve::ppp_weights_from_c(&input.weights);
        let tropo = c_try!(crate::solve::ppp_tropo_from_c(f, &input.tropo));
        let corrections = c_try!(crate::solve::ppp_range_corrections_from_c(
            f,
            &input.corrections
        ));
        let v = &mut c_try!(require_mut(config, f, "config")).inner;
        v.weights = weights;
        v.tropo = tropo;
        v.corrections = corrections;
        SidereonStatus::Ok
    })
}
/// Copy a state and covariance into the configuration's initial condition.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_set_initial_state(
    config: *mut SidereonPppKinematicConfig,
    state: *const SidereonPppKinematicState,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_set_initial_state";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let s = c_try!(require_ref(state, f, "state"));
        let c = &mut c_try!(require_mut(config, f, "config")).inner;
        c.initial_state = s.inner.clone();
        c.initial_covariance_m2 = s.covariance_m2.clone();
        SidereonStatus::Ok
    })
}
/// Clone the configured initial state and covariance.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_initial_state(
    config: *const SidereonPppKinematicConfig,
    out: *mut *mut SidereonPppKinematicState,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_initial_state";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", config, "config"
        ));
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        let v = &c_try!(require_ref(config, f, "config")).inner;
        write_boxed_handle(
            out,
            SidereonPppKinematicState {
                inner: v.initial_state.clone(),
                covariance_m2: v.initial_covariance_m2.clone(),
            },
        );
        SidereonStatus::Ok
    })
}
/// Compare configurations using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_equal(
    left: *const SidereonPppKinematicConfig,
    right: *const SidereonPppKinematicConfig,
    out_equal: *mut bool,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_config_equal";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, f, "out_equal")) = c_try!(require_ref(left, f, "left"))
            .inner
            == c_try!(require_ref(right, f, "right")).inner;
        SidereonStatus::Ok
    })
}
/// Free a configuration. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_config_free(
    config: *mut SidereonPppKinematicConfig,
) {
    free_boxed(config)
}

/// Allocate the default state with the core default covariance.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_default(
    out: *mut *mut SidereonPppKinematicState,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_default";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        let config = core::KinematicConfig::default();
        write_boxed_handle(
            out,
            SidereonPppKinematicState {
                inner: config.initial_state,
                covariance_m2: config.initial_covariance_m2,
            },
        );
        SidereonStatus::Ok
    })
}
/// Allocate a state from every state field and a row-major covariance.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_new(
    position_m: *const f64,
    clock_m: f64,
    ztd_residual_m: f64,
    ambiguities: *const SidereonPppKinematicAmbiguity,
    ambiguity_count: usize,
    covariance_m2: *const f64,
    covariance_count: usize,
    out: *mut *mut SidereonPppKinematicState,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_new";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        let position = *c_try!(require_f64_array::<3>(position_m, f, "position_m"));
        let map = c_try!(ambiguity_map(f, ambiguities, ambiguity_count));
        let dimension = 5 + map.len();
        let covariance = c_try!(covariance_from_flat(
            f,
            covariance_m2,
            covariance_count,
            dimension
        ));
        write_boxed_handle(
            out,
            SidereonPppKinematicState {
                inner: core::KinematicState {
                    position_m: position,
                    clock_m,
                    ztd_residual_m,
                    ambiguities_m: map,
                },
                covariance_m2: covariance,
            },
        );
        SidereonStatus::Ok
    })
}
/// Read fixed state fields and counts.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_info(
    state: *const SidereonPppKinematicState,
    out: *mut SidereonPppKinematicStateInfo,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_info";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(f, out, "out", state, "state"));
        let v = c_try!(require_ref(state, f, "state"));
        c_try!(require_out(out, f, "out")).write(SidereonPppKinematicStateInfo {
            position_m: v.inner.position_m,
            clock_m: v.inner.clock_m,
            ztd_residual_m: v.inner.ztd_residual_m,
            ambiguity_count: v.inner.ambiguities_m.len(),
            dimension: v.inner.dimension(),
            covariance_element_count: v.covariance_m2.len() * v.covariance_m2.len(),
        });
        SidereonStatus::Ok
    })
}
unsafe fn state_ambiguity<'a>(
    f: &str,
    state: *const SidereonPppKinematicState,
    index: usize,
) -> Result<(&'a str, f64), SidereonStatus> {
    require_ref(state, f, "state")?
        .inner
        .ambiguities_m
        .iter()
        .nth(index)
        .map(|(k, v)| (k.as_str(), *v))
        .ok_or_else(|| {
            set_last_error(format!("{f}: index out of range"));
            SidereonStatus::InvalidArgument
        })
}
/// Copy one ambiguity id losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_ambiguity_id(
    state: *const SidereonPppKinematicState,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_ambiguity_id";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let (k, _) = c_try!(state_ambiguity(f, state, index));
        copy_text(f, k, out, len, out_written, out_required)
    })
}
/// Read one ambiguity value.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_ambiguity_m(
    state: *const SidereonPppKinematicState,
    index: usize,
    out: *mut f64,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_ambiguity_m";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(f, out, "out", state, "state"));
        let out = c_try!(require_out(out, f, "out"));
        out.write(0.0);
        let (_, v) = c_try!(state_ambiguity(f, state, index));
        out.write(v);
        SidereonStatus::Ok
    })
}
/// Copy row-major covariance with the standard size-query contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_covariance(
    state: *const SidereonPppKinematicState,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_covariance";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let v = c_try!(require_ref(state, f, "state"));
        let flat = v
            .covariance_m2
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let written = c_try!(require_out(out_written, f, "out_written"));
        let required = c_try!(require_out(out_required, f, "out_required"));
        written.write(0);
        required.write(flat.len());
        if flat.is_empty() {
            return SidereonStatus::Ok;
        }
        if out.is_null() {
            if len == 0 {
                return SidereonStatus::Ok;
            }
            set_last_error(format!("{f}: null out"));
            return SidereonStatus::NullPointer;
        }
        if len < flat.len() {
            zero_f64_prefix(out, len, flat.len());
            set_last_error(format!("{f}: out needs room for {} doubles", flat.len()));
            return SidereonStatus::InvalidArgument;
        }
        ptr::copy_nonoverlapping(flat.as_ptr(), out, flat.len());
        written.write(flat.len());
        SidereonStatus::Ok
    })
}
/// Compare states and covariance using core state equality and value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_equal(
    left: *const SidereonPppKinematicState,
    right: *const SidereonPppKinematicState,
    out_equal: *mut bool,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_state_equal";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        let l = c_try!(require_ref(left, f, "left"));
        let r = c_try!(require_ref(right, f, "right"));
        *c_try!(require_out(out_equal, f, "out_equal")) =
            l.inner == r.inner && l.covariance_m2 == r.covariance_m2;
        SidereonStatus::Ok
    })
}
/// Free a state. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_state_free(state: *mut SidereonPppKinematicState) {
    free_boxed(state)
}

/// Predict a mutable state and covariance through the core.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_predict_kinematic_state(
    state: *mut SidereonPppKinematicState,
    dt_s: f64,
    active_ambiguity_ids: *const *const c_char,
    active_ambiguity_count: usize,
    config: *const SidereonPppKinematicConfig,
) -> SidereonStatus {
    let f = "sidereon_ppp_predict_kinematic_state";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let ids = c_try!(require_slice(
            active_ambiguity_ids,
            active_ambiguity_count,
            f,
            "active_ambiguity_ids"
        ))
        .iter()
        .map(|p| parse_c_string(f, "active_ambiguity_id", *p))
        .collect::<Result<Vec<_>, _>>();
        let ids = c_try!(ids);
        let config = c_try!(require_ref(config, f, "config"));
        let state = c_try!(require_mut(state, f, "state"));
        match core::predict_kinematic_state(
            &mut state.inner,
            &mut state.covariance_m2,
            dt_s,
            &ids,
            &config.inner,
        ) {
            Ok(()) => SidereonStatus::Ok,
            Err(e) => map_error(f, e),
        }
    })
}

/// Correct a mutable state with one PPP epoch through the core.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_correct_kinematic_state(
    sp3: *const SidereonSp3,
    epoch_index: usize,
    epoch: *const SidereonPppEpoch,
    state: *mut SidereonPppKinematicState,
    config: *const SidereonPppKinematicConfig,
    out: *mut *mut SidereonPppKinematicUpdateSummary,
) -> SidereonStatus {
    let f = "sidereon_ppp_correct_kinematic_state";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        c_try!(reject_output_overlaps_handle(f, out, "out", sp3, "sp3"));
        c_try!(reject_output_overlaps_handle(f, out, "out", state, "state"));
        c_try!(reject_output_overlaps_handle(
            f, out, "out", config, "config"
        ));
        let source = c_try!(require_ref(sp3, f, "sp3"));
        let epochs = c_try!(crate::solve::ppp_epochs_from_c(f, epoch, 1));
        let state = c_try!(require_mut(state, f, "state"));
        let config = c_try!(require_ref(config, f, "config"));
        match core::correct_kinematic_state(
            &source.inner,
            epoch_index,
            &epochs[0],
            &mut state.inner,
            &mut state.covariance_m2,
            &config.inner,
        ) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppKinematicUpdateSummary { inner });
                SidereonStatus::Ok
            }
            Err(e) => map_error(f, e),
        }
    })
}
/// Read update summary fields and counts.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_info(
    summary: *const SidereonPppKinematicUpdateSummary,
    out: *mut SidereonPppKinematicUpdateSummaryInfo,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_update_summary_info";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", summary, "summary"
        ));
        let v = &c_try!(require_ref(summary, f, "summary")).inner;
        c_try!(require_out(out, f, "out")).write(SidereonPppKinematicUpdateSummaryInfo {
            innovation_rms_m: v.innovation_rms_m,
            used_satellite_count: v.used_sats.len(),
            ssr_bias_exclusion_count: v.ssr_bias_exclusions.len(),
            unplaced_observation_count: v.unplaced_observations.len(),
        });
        SidereonStatus::Ok
    })
}
/// Copy one used-satellite id losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_used_satellite(
    summary: *const SidereonPppKinematicUpdateSummary,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_update_summary_used_satellite";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let v = c_try!(require_ref(summary, f, "summary"))
            .inner
            .used_sats
            .get(index)
            .ok_or(SidereonStatus::InvalidArgument);
        copy_text(f, c_try!(v), out, len, out_written, out_required)
    })
}
/// Read one complete SSR-bias exclusion using the existing PPP value representation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_ssr_bias_exclusion(
    summary: *const SidereonPppKinematicUpdateSummary,
    index: usize,
    out: *mut SidereonPppSsrBiasExclusion,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_update_summary_ssr_bias_exclusion";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", summary, "summary"
        ));
        let value = c_try!(require_ref(summary, f, "summary"))
            .inner
            .ssr_bias_exclusions
            .get(index)
            .ok_or(SidereonStatus::InvalidArgument);
        let converted = c_try!(crate::ppp::ppp_ssr_bias_exclusion_to_c(f, c_try!(value)));
        c_try!(require_uninit_out(out, f, "out")).write(converted);
        SidereonStatus::Ok
    })
}
/// Read one complete unplaced observation using the additive PPP representation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_unplaced_observation(
    summary: *const SidereonPppKinematicUpdateSummary,
    index: usize,
    out: *mut SidereonPppUnplacedObservationV2,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_update_summary_unplaced_observation";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", summary, "summary"
        ));
        let values = &c_try!(require_ref(summary, f, "summary"))
            .inner
            .unplaced_observations;
        let value = c_try!(values.get(index).ok_or(SidereonStatus::InvalidArgument));
        let converted = c_try!(crate::ppp::ppp_unplaced_observations_v2_to_c(
            f,
            std::slice::from_ref(value)
        ));
        c_try!(require_uninit_out(out, f, "out")).write(converted[0]);
        SidereonStatus::Ok
    })
}
/// Compare update summaries using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_equal(
    left: *const SidereonPppKinematicUpdateSummary,
    right: *const SidereonPppKinematicUpdateSummary,
    out_equal: *mut bool,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_update_summary_equal";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, f, "out_equal")) = c_try!(require_ref(left, f, "left"))
            .inner
            == c_try!(require_ref(right, f, "right")).inner;
        SidereonStatus::Ok
    })
}
/// Free an update summary. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_update_summary_free(
    summary: *mut SidereonPppKinematicUpdateSummary,
) {
    free_boxed(summary)
}

/// Solve ordered PPP epochs with the core kinematic EKF.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_solve_kinematic_ppp(
    sp3: *const SidereonSp3,
    epochs: *const SidereonPppEpoch,
    epoch_count: usize,
    config: *const SidereonPppKinematicConfig,
    out: *mut *mut SidereonPppKinematicResult,
) -> SidereonStatus {
    let f = "sidereon_ppp_solve_kinematic_ppp";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, f, "out"));
        out.write(ptr::null_mut());
        c_try!(reject_output_overlaps_handle(f, out, "out", sp3, "sp3"));
        c_try!(reject_output_overlaps_handle(
            f, out, "out", config, "config"
        ));
        let source = c_try!(require_ref(sp3, f, "sp3"));
        let epochs = c_try!(crate::solve::ppp_epochs_from_c(f, epochs, epoch_count));
        let config = c_try!(require_ref(config, f, "config"));
        match core::solve_kinematic_ppp(&source.inner, &epochs, config.inner.clone()) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppKinematicResult { inner });
                SidereonStatus::Ok
            }
            Err(e) => map_error(f, e),
        }
    })
}
fn status_to_c(value: core::KinematicEpochStatus) -> SidereonPppKinematicEpochStatus {
    match value {
        core::KinematicEpochStatus::Updated => SidereonPppKinematicEpochStatus::Updated,
        core::KinematicEpochStatus::PredictedOnly => SidereonPppKinematicEpochStatus::PredictedOnly,
    }
}
unsafe fn solution<'a>(
    f: &str,
    result: *const SidereonPppKinematicResult,
    index: usize,
) -> Result<&'a core::KinematicEpochSolution, SidereonStatus> {
    require_ref(result, f, "result")?
        .inner
        .get(index)
        .ok_or_else(|| {
            set_last_error(format!("{f}: index out of range"));
            SidereonStatus::InvalidArgument
        })
}
/// Return the number of epoch solutions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_epoch_count(
    result: *const SidereonPppKinematicResult,
    out: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_epoch_count";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", result, "result"
        ));
        *c_try!(require_out(out, f, "out")) = c_try!(require_ref(result, f, "result")).inner.len();
        SidereonStatus::Ok
    })
}
/// Read one epoch solution's fixed fields.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_epoch_info(
    result: *const SidereonPppKinematicResult,
    index: usize,
    out: *mut SidereonPppKinematicEpochSolutionInfo,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_epoch_info";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", result, "result"
        ));
        let v = c_try!(solution(f, result, index));
        c_try!(require_out(out, f, "out")).write(SidereonPppKinematicEpochSolutionInfo {
            position_m: v.position_m,
            clock_m: v.clock_m,
            ztd_residual_m: v.ztd_residual_m,
            position_covariance_m2: v.position_covariance_m2,
            innovation_rms_m: v.innovation_rms_m,
            status: status_to_c(v.status),
            ambiguity_count: v.ambiguities_m.len(),
            used_satellite_count: v.used_sats.len(),
            ssr_bias_exclusion_count: v.ssr_bias_exclusions.len(),
            unplaced_observation_count: v.unplaced_observations.len(),
        });
        SidereonStatus::Ok
    })
}
unsafe fn solution_ambiguity<'a>(
    f: &str,
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
) -> Result<(&'a str, f64), SidereonStatus> {
    solution(f, result, epoch)?
        .ambiguities_m
        .iter()
        .nth(index)
        .map(|(k, v)| (k.as_str(), *v))
        .ok_or(SidereonStatus::InvalidArgument)
}
/// Copy one epoch ambiguity id losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_ambiguity_id(
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_ambiguity_id";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let (k, _) = c_try!(solution_ambiguity(f, result, epoch, index));
        copy_text(f, k, out, len, out_written, out_required)
    })
}
/// Read one epoch ambiguity value.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_ambiguity_m(
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
    out: *mut f64,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_ambiguity_m";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", result, "result"
        ));
        let out = c_try!(require_out(out, f, "out"));
        out.write(0.0);
        let (_, v) = c_try!(solution_ambiguity(f, result, epoch, index));
        out.write(v);
        SidereonStatus::Ok
    })
}
/// Copy one epoch used-satellite id losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_used_satellite(
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_used_satellite";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        let v = c_try!(solution(f, result, epoch))
            .used_sats
            .get(index)
            .ok_or(SidereonStatus::InvalidArgument);
        copy_text(f, c_try!(v), out, len, out_written, out_required)
    })
}
/// Read one epoch SSR-bias exclusion.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_ssr_bias_exclusion(
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
    out: *mut SidereonPppSsrBiasExclusion,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_ssr_bias_exclusion";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", result, "result"
        ));
        let value = c_try!(solution(f, result, epoch))
            .ssr_bias_exclusions
            .get(index)
            .ok_or(SidereonStatus::InvalidArgument);
        let converted = c_try!(crate::ppp::ppp_ssr_bias_exclusion_to_c(f, c_try!(value)));
        c_try!(require_uninit_out(out, f, "out")).write(converted);
        SidereonStatus::Ok
    })
}
/// Read one epoch unplaced observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_unplaced_observation(
    result: *const SidereonPppKinematicResult,
    epoch: usize,
    index: usize,
    out: *mut SidereonPppUnplacedObservationV2,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_unplaced_observation";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f, out, "out", result, "result"
        ));
        let value = c_try!(solution(f, result, epoch))
            .unplaced_observations
            .get(index)
            .ok_or(SidereonStatus::InvalidArgument);
        let converted = c_try!(crate::ppp::ppp_unplaced_observations_v2_to_c(
            f,
            std::slice::from_ref(c_try!(value))
        ));
        c_try!(require_uninit_out(out, f, "out")).write(converted[0]);
        SidereonStatus::Ok
    })
}
/// Compare ordered solve results using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_equal(
    left: *const SidereonPppKinematicResult,
    right: *const SidereonPppKinematicResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_result_equal";
    engine_error_operation_boundary(f, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            f,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, f, "out_equal")) = c_try!(require_ref(left, f, "left"))
            .inner
            == c_try!(require_ref(right, f, "right")).inner;
        SidereonStatus::Ok
    })
}
/// Free a solve result. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_result_free(
    result: *mut SidereonPppKinematicResult,
) {
    free_boxed(result)
}

/// Return core Display text for a representative error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_kinematic_solve_error_text(
    kind: SidereonPppKinematicSolveErrorKind,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let f = "sidereon_ppp_kinematic_solve_error_text";
    let error = match kind {
        SidereonPppKinematicSolveErrorKind::NoEphemeris => core::KinematicSolveError::NoEphemeris {
            satellite_id: "G01".to_string(),
            reason: core::NoEphemerisReason::NoEphemeris,
        },
        SidereonPppKinematicSolveErrorKind::SingularGeometry => {
            core::KinematicSolveError::SingularGeometry
        }
        SidereonPppKinematicSolveErrorKind::InvalidSolveOption => {
            core::KinematicSolveError::InvalidSolveOption {
                field: "field",
                reason: "reason",
            }
        }
        SidereonPppKinematicSolveErrorKind::InvalidInput => {
            core::KinematicSolveError::InvalidInput {
                field: "field",
                reason: "reason",
            }
        }
        SidereonPppKinematicSolveErrorKind::MissingCorrection => {
            core::KinematicSolveError::MissingCorrection {
                satellite_id: "G01".to_string(),
                correction: core::MissingCorrection::CodeBias,
            }
        }
        SidereonPppKinematicSolveErrorKind::Ut1OutsideCoverage => {
            core::KinematicSolveError::Ut1OutsideCoverage(
                sidereon_core::astro::time::DegradeReason::BeforeCoverage,
            )
        }
    };
    copy_text(f, &error.to_string(), out, len, out_written, out_required)
}
