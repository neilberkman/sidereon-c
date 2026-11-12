use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::precise_positioning as core;

/// Default thin-shell ionospheric height, in metres.
pub const SIDEREON_TEC_DEFAULT_IONOSPHERIC_SHELL_HEIGHT_M: f64 = 350_000.0;
/// Number of electrons per square metre in one TECU.
pub const SIDEREON_TEC_ELECTRONS_PER_TECU_M2: f64 = 1.0e16;
/// TEC group-delay coefficient for frequencies expressed in hertz.
pub const SIDEREON_TEC_GROUP_DELAY_COEFFICIENT: f64 = 4.0308e17;

const _: () = assert!(
    SIDEREON_TEC_DEFAULT_IONOSPHERIC_SHELL_HEIGHT_M.to_bits()
        == core::DEFAULT_IONOSPHERIC_SHELL_HEIGHT_M.to_bits()
);
const _: () =
    assert!(SIDEREON_TEC_ELECTRONS_PER_TECU_M2.to_bits() == core::ELECTRONS_PER_TECU_M2.to_bits());
const _: () = assert!(
    SIDEREON_TEC_GROUP_DELAY_COEFFICIENT.to_bits() == core::TEC_GROUP_DELAY_COEFFICIENT.to_bits()
);

/// Thin-shell TEC configuration. This plain C value compares field-by-field.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecConfig {
    /// Ionospheric shell height above the spherical Earth, in metres.
    pub shell_height_m: f64,
    /// Spherical Earth radius used by the mapping function, in metres.
    pub earth_radius_m: f64,
}

/// Raw dual-frequency observation shared by TEC and PPP preparation.
#[repr(C)]
pub struct SidereonPppDualFrequencyObservation {
    /// Null-terminated UTF-8 satellite identifier.
    pub satellite_id: *const c_char,
    /// Null-terminated UTF-8 ambiguity identifier.
    pub ambiguity_id: *const c_char,
    /// First code pseudorange, in metres.
    pub p1_m: f64,
    /// Second code pseudorange, in metres.
    pub p2_m: f64,
    /// First carrier phase, in cycles.
    pub phi1_cyc: f64,
    /// Second carrier phase, in cycles.
    pub phi2_cyc: f64,
    /// First carrier frequency, in hertz.
    pub f1_hz: f64,
    /// Second carrier frequency, in hertz.
    pub f2_hz: f64,
    /// Whether `lli1` is present.
    pub has_lli1: bool,
    /// First-band loss-of-lock indicator.
    pub lli1: i64,
    /// Whether `lli2` is present.
    pub has_lli2: bool,
    /// Second-band loss-of-lock indicator.
    pub lli2: i64,
}

/// One dual-frequency TEC observation with topocentric geometry.
#[repr(C)]
pub struct SidereonTecObservation {
    /// Raw observation.
    pub observation: SidereonPppDualFrequencyObservation,
    /// Satellite elevation, in radians.
    pub elevation_rad: f64,
    /// Satellite azimuth clockwise from north, in radians.
    pub azimuth_rad: f64,
}

/// One TEC epoch.
#[repr(C)]
pub struct SidereonTecEpoch {
    /// Comparable epoch coordinate, in seconds.
    pub time_s: f64,
    /// Receiver geodetic latitude, in radians.
    pub receiver_latitude_rad: f64,
    /// Receiver geodetic longitude, in radians.
    pub receiver_longitude_rad: f64,
    /// Pointer to `observation_count` observations.
    pub observations: *const SidereonTecObservation,
    /// Number of observations.
    pub observation_count: usize,
}

/// One phase-code leveling input sample. This plain C value has value equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecLevelingSample {
    /// Code-derived slant TEC, in TECU.
    pub code_slant_tec_tecu: f64,
    /// Biased phase-derived slant TEC, in TECU.
    pub phase_slant_tec_tecu: f64,
    /// Satellite elevation, in radians.
    pub elevation_rad: f64,
}

/// One leveled output sample. This plain C value has value equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecLeveledSample {
    /// Code-derived slant TEC input, in TECU.
    pub code_slant_tec_tecu: f64,
    /// Phase-derived slant TEC input, in TECU.
    pub phase_slant_tec_tecu: f64,
    /// Leveled phase-derived slant TEC, in TECU.
    pub leveled_slant_tec_tecu: f64,
    /// Thin-shell mapping function.
    pub mapping_function: f64,
    /// Leveled vertical TEC, in TECU.
    pub vertical_tec_tecu: f64,
}

/// Thin-shell ionospheric pierce point. This plain C value has value equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecIonosphericPiercePoint {
    /// Latitude, in radians.
    pub latitude_rad: f64,
    /// Longitude in `[-pi, pi)`, in radians.
    pub longitude_rad: f64,
    /// Latitude, in degrees.
    pub latitude_deg: f64,
    /// Longitude in `[-180, 180)`, in degrees.
    pub longitude_deg: f64,
    /// Earth-central receiver-to-pierce-point angle, in radians.
    pub earth_central_angle_rad: f64,
    /// Shell height, in metres.
    pub shell_height_m: f64,
}

/// Fixed numeric fields of a code-derived slant TEC estimate.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecCodeSlantEstimateInfo {
    /// Code geometry-free combination, in metres.
    pub code_geometry_free_m: f64,
    /// Slant TEC, in TECU.
    pub slant_tec_tecu: f64,
}

/// Fixed numeric fields of a phase-derived slant TEC estimate.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecPhaseSlantEstimateInfo {
    /// Phase geometry-free combination, in metres.
    pub phase_geometry_free_m: f64,
    /// Biased slant TEC, in TECU.
    pub slant_tec_tecu: f64,
}

/// Owned code-derived slant TEC estimate.
pub struct SidereonTecCodeSlantEstimate {
    inner: core::CodeSlantTecEstimate,
}

/// Owned phase-derived slant TEC estimate.
pub struct SidereonTecPhaseSlantEstimate {
    inner: core::PhaseSlantTecEstimate,
}

/// TEC refusal kind. Values correspond one-to-one with the core error variants.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTecErrorKind {
    /// One or more observation scalars were not finite.
    NonFiniteObservation = 0,
    /// Shell height was invalid.
    InvalidShellHeight = 1,
    /// Earth radius was invalid.
    InvalidEarthRadius = 2,
    /// A carrier frequency was invalid.
    InvalidFrequency = 3,
    /// Carrier frequencies were equal or too close.
    EqualFrequencies = 4,
    /// Receiver latitude was invalid.
    InvalidReceiverLatitude = 5,
    /// Receiver longitude was invalid.
    InvalidReceiverLongitude = 6,
    /// Elevation was invalid.
    InvalidElevation = 7,
    /// Azimuth was invalid.
    InvalidAzimuth = 8,
    /// A TEC value was not finite.
    NonFiniteTec = 9,
    /// A leveling arc was empty.
    EmptyArc = 10,
    /// The epoch stream was empty.
    NoEpochs = 11,
    /// The epoch stream contained no observations.
    NoObservations = 12,
    /// An epoch time was not finite.
    NonFiniteEpochTime = 13,
    /// Epochs were not ordered.
    EpochsNotOrdered = 14,
    /// An arc had fewer than two samples.
    InsufficientArcSamples = 15,
}

/// Owned result of phase-code TEC leveling.
pub struct SidereonTecLevelingResult {
    inner: core::TecLevelingResult,
}

/// Owned TEC estimate over all continuous arcs.
pub struct SidereonTecEstimate {
    inner: core::TecEstimate,
}

/// Fixed scalar fields for one TEC estimate sample.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonTecEstimateSample {
    /// Epoch coordinate, in seconds.
    pub time_s: f64,
    /// Elevation, in radians.
    pub elevation_rad: f64,
    /// Azimuth, in radians.
    pub azimuth_rad: f64,
    /// Code geometry-free combination, in metres.
    pub code_geometry_free_m: f64,
    /// Phase geometry-free combination, in metres.
    pub phase_geometry_free_m: f64,
    /// Code-derived slant TEC, in TECU.
    pub code_slant_tec_tecu: f64,
    /// Biased phase-derived slant TEC, in TECU.
    pub phase_slant_tec_tecu: f64,
    /// Leveled slant TEC, in TECU.
    pub leveled_slant_tec_tecu: f64,
    /// Thin-shell mapping function.
    pub mapping_function: f64,
    /// Vertical TEC, in TECU.
    pub vertical_tec_tecu: f64,
    /// Ionospheric pierce point.
    pub pierce_point: SidereonTecIonosphericPiercePoint,
}

fn tec_config(value: SidereonTecConfig) -> core::TecConfig {
    let mut config = core::TecConfig::default();
    config.shell_height_m = value.shell_height_m;
    config.earth_radius_m = value.earth_radius_m;
    config
}

pub(crate) unsafe fn dual_observation(
    fn_name: &str,
    value: &SidereonPppDualFrequencyObservation,
) -> Result<core::DualFrequencyObservation, SidereonStatus> {
    Ok(core::DualFrequencyObservation {
        satellite_id: parse_c_string(fn_name, "observation.satellite_id", value.satellite_id)?,
        ambiguity_id: parse_c_string(fn_name, "observation.ambiguity_id", value.ambiguity_id)?,
        p1_m: value.p1_m,
        p2_m: value.p2_m,
        phi1_cyc: value.phi1_cyc,
        phi2_cyc: value.phi2_cyc,
        f1_hz: value.f1_hz,
        f2_hz: value.f2_hz,
        lli1: value.has_lli1.then_some(value.lli1),
        lli2: value.has_lli2.then_some(value.lli2),
    })
}

fn tec_error(fn_name: &str, error: core::TecError) -> SidereonStatus {
    let kind = match error {
        core::TecError::NonFiniteObservation => "non_finite_observation",
        core::TecError::InvalidShellHeight => "invalid_shell_height",
        core::TecError::InvalidEarthRadius => "invalid_earth_radius",
        core::TecError::InvalidFrequency => "invalid_frequency",
        core::TecError::EqualFrequencies => "equal_frequencies",
        core::TecError::InvalidReceiverLatitude => "invalid_receiver_latitude",
        core::TecError::InvalidReceiverLongitude => "invalid_receiver_longitude",
        core::TecError::InvalidElevation => "invalid_elevation",
        core::TecError::InvalidAzimuth => "invalid_azimuth",
        core::TecError::NonFiniteTec => "non_finite_tec",
        core::TecError::EmptyArc => "empty_arc",
        core::TecError::NoEpochs => "no_epochs",
        core::TecError::NoObservations => "no_observations",
        core::TecError::NonFiniteEpochTime => "non_finite_epoch_time",
        core::TecError::EpochsNotOrdered => "epochs_not_ordered",
        core::TecError::InsufficientArcSamples => "insufficient_arc_samples",
    };
    record_engine_error(
        SidereonEngineErrorFamily::Tec,
        fn_name,
        json!({"kind": kind, "message": error.to_string(), "fields": {}}),
    );
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

fn tec_error_from_c(kind: SidereonTecErrorKind) -> core::TecError {
    match kind {
        SidereonTecErrorKind::NonFiniteObservation => core::TecError::NonFiniteObservation,
        SidereonTecErrorKind::InvalidShellHeight => core::TecError::InvalidShellHeight,
        SidereonTecErrorKind::InvalidEarthRadius => core::TecError::InvalidEarthRadius,
        SidereonTecErrorKind::InvalidFrequency => core::TecError::InvalidFrequency,
        SidereonTecErrorKind::EqualFrequencies => core::TecError::EqualFrequencies,
        SidereonTecErrorKind::InvalidReceiverLatitude => core::TecError::InvalidReceiverLatitude,
        SidereonTecErrorKind::InvalidReceiverLongitude => core::TecError::InvalidReceiverLongitude,
        SidereonTecErrorKind::InvalidElevation => core::TecError::InvalidElevation,
        SidereonTecErrorKind::InvalidAzimuth => core::TecError::InvalidAzimuth,
        SidereonTecErrorKind::NonFiniteTec => core::TecError::NonFiniteTec,
        SidereonTecErrorKind::EmptyArc => core::TecError::EmptyArc,
        SidereonTecErrorKind::NoEpochs => core::TecError::NoEpochs,
        SidereonTecErrorKind::NoObservations => core::TecError::NoObservations,
        SidereonTecErrorKind::NonFiniteEpochTime => core::TecError::NonFiniteEpochTime,
        SidereonTecErrorKind::EpochsNotOrdered => core::TecError::EpochsNotOrdered,
        SidereonTecErrorKind::InsufficientArcSamples => core::TecError::InsufficientArcSamples,
    }
}

fn ipp(value: core::IonosphericPiercePoint) -> SidereonTecIonosphericPiercePoint {
    SidereonTecIonosphericPiercePoint {
        latitude_rad: value.latitude_rad,
        longitude_rad: value.longitude_rad,
        latitude_deg: value.latitude_deg,
        longitude_deg: value.longitude_deg,
        earth_central_angle_rad: value.earth_central_angle_rad,
        shell_height_m: value.shell_height_m,
    }
}

/// Initialize a TEC configuration from the core default.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_config_default(
    out: *mut SidereonTecConfig,
) -> SidereonStatus {
    ffi_boundary("sidereon_tec_config_default", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_tec_config_default", "out"));
        let value = core::TecConfig::default();
        *out = SidereonTecConfig {
            shell_height_m: value.shell_height_m,
            earth_radius_m: value.earth_radius_m,
        };
        SidereonStatus::Ok
    })
}

/// Validate a TEC configuration with the core validator.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_config_validate(config: SidereonTecConfig) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tec_config_validate",
        SidereonStatus::Panic,
        || match tec_config(config).validate() {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => tec_error("sidereon_tec_config_validate", error),
        },
    )
}

/// Convert a code geometry-free delay to slant TEC through the core.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_slant_from_code_geometry_free_m(
    code_geometry_free_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_slant_from_code_geometry_free_m";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = 0.0;
        match core::slant_tec_from_code_geometry_free_m(code_geometry_free_m, f1_hz, f2_hz) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Convert a phase geometry-free delay to biased slant TEC through the core.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_slant_from_phase_geometry_free_m(
    phase_geometry_free_m: f64,
    f1_hz: f64,
    f2_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_slant_from_phase_geometry_free_m";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = 0.0;
        match core::slant_tec_from_phase_geometry_free_m(phase_geometry_free_m, f1_hz, f2_hz) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Compute the thin-shell mapping function.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_thin_shell_mapping_function(
    elevation_rad: f64,
    config: SidereonTecConfig,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_thin_shell_mapping_function";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = 0.0;
        match core::thin_shell_mapping_function(elevation_rad, tec_config(config)) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Convert slant TEC to vertical TEC using the core thin-shell model.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_vertical_from_slant(
    slant_tec_tecu: f64,
    elevation_rad: f64,
    config: SidereonTecConfig,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_vertical_from_slant";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = 0.0;
        match core::vertical_tec_from_slant_tec(slant_tec_tecu, elevation_rad, tec_config(config)) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Compute an ionospheric pierce point with the core model.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_ionospheric_pierce_point(
    receiver_latitude_rad: f64,
    receiver_longitude_rad: f64,
    elevation_rad: f64,
    azimuth_rad: f64,
    config: SidereonTecConfig,
    out: *mut SidereonTecIonosphericPiercePoint,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_ionospheric_pierce_point";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        match core::ionospheric_pierce_point(
            receiver_latitude_rad,
            receiver_longitude_rad,
            elevation_rad,
            azimuth_rad,
            tec_config(config),
        ) {
            Ok(value) => {
                *out = ipp(value);
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Estimate code-derived slant TEC for one raw observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_code_slant(
    observation: *const SidereonPppDualFrequencyObservation,
    out: *mut *mut SidereonTecCodeSlantEstimate,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_code_slant";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let raw = c_try!(require_ref(observation, fn_name, "observation"));
        let observation = c_try!(dual_observation(fn_name, raw));
        match core::estimate_code_slant_tec(&observation) {
            Ok(value) => {
                write_boxed_handle(out, SidereonTecCodeSlantEstimate { inner: value });
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Estimate phase-derived slant TEC for one raw observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_phase_slant(
    observation: *const SidereonPppDualFrequencyObservation,
    out: *mut *mut SidereonTecPhaseSlantEstimate,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_phase_slant";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let raw = c_try!(require_ref(observation, fn_name, "observation"));
        let observation = c_try!(dual_observation(fn_name, raw));
        match core::estimate_phase_slant_tec(&observation) {
            Ok(value) => {
                write_boxed_handle(out, SidereonTecPhaseSlantEstimate { inner: value });
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Compute the code geometry-free combination for one raw observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_geometry_free_m(
    observation: *const SidereonPppDualFrequencyObservation,
    out: *mut f64,
) -> SidereonStatus {
    tec_observation_scalar(
        "sidereon_tec_code_geometry_free_m",
        observation,
        out,
        core::code_geometry_free_m,
    )
}

/// Compute the phase geometry-free combination for one raw observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_geometry_free_m(
    observation: *const SidereonPppDualFrequencyObservation,
    out: *mut f64,
) -> SidereonStatus {
    tec_observation_scalar(
        "sidereon_tec_phase_geometry_free_m",
        observation,
        out,
        core::phase_geometry_free_m,
    )
}

unsafe fn tec_observation_scalar(
    fn_name: &str,
    observation: *const SidereonPppDualFrequencyObservation,
    out: *mut f64,
    operation: fn(&core::DualFrequencyObservation) -> Result<f64, core::TecError>,
) -> SidereonStatus {
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let observation = c_try!(require_ref(observation, fn_name, "observation"));
        let observation = c_try!(dual_observation(fn_name, observation));
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = 0.0;
        match operation(&observation) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

unsafe fn copy_tec_text(
    fn_name: &str,
    text: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    match copy_prefix_to_c(
        fn_name,
        "out",
        text.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ) {
        Ok(()) => SidereonStatus::Ok,
        Err(status) => status,
    }
}

/// Read the numeric fields of a code-derived slant estimate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_slant_estimate_info(
    estimate: *const SidereonTecCodeSlantEstimate,
    out: *mut SidereonTecCodeSlantEstimateInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_code_slant_estimate_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        *c_try!(require_out(out, fn_name, "out")) = SidereonTecCodeSlantEstimateInfo {
            code_geometry_free_m: value.inner.code_geometry_free_m,
            slant_tec_tecu: value.inner.slant_tec_tecu,
        };
        SidereonStatus::Ok
    })
}

/// Copy a code estimate's satellite identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_slant_estimate_satellite_id(
    estimate: *const SidereonTecCodeSlantEstimate,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_code_slant_estimate_satellite_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        copy_tec_text(
            fn_name,
            &value.inner.satellite_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy a code estimate's ambiguity identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_slant_estimate_ambiguity_id(
    estimate: *const SidereonTecCodeSlantEstimate,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_code_slant_estimate_ambiguity_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        copy_tec_text(
            fn_name,
            &value.inner.ambiguity_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare code-derived slant estimates by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_slant_estimate_equal(
    left: *const SidereonTecCodeSlantEstimate,
    right: *const SidereonTecCodeSlantEstimate,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_code_slant_estimate_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release a code-derived slant estimate. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_code_slant_estimate_free(
    estimate: *mut SidereonTecCodeSlantEstimate,
) {
    ffi_boundary("sidereon_tec_code_slant_estimate_free", (), || {
        if !estimate.is_null() {
            drop(Box::from_raw(estimate));
        }
    });
}

/// Read the numeric fields of a phase-derived slant estimate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_slant_estimate_info(
    estimate: *const SidereonTecPhaseSlantEstimate,
    out: *mut SidereonTecPhaseSlantEstimateInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_phase_slant_estimate_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        *c_try!(require_out(out, fn_name, "out")) = SidereonTecPhaseSlantEstimateInfo {
            phase_geometry_free_m: value.inner.phase_geometry_free_m,
            slant_tec_tecu: value.inner.slant_tec_tecu,
        };
        SidereonStatus::Ok
    })
}

/// Copy a phase estimate's satellite identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_slant_estimate_satellite_id(
    estimate: *const SidereonTecPhaseSlantEstimate,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_phase_slant_estimate_satellite_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        copy_tec_text(
            fn_name,
            &value.inner.satellite_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy a phase estimate's ambiguity identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_slant_estimate_ambiguity_id(
    estimate: *const SidereonTecPhaseSlantEstimate,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_phase_slant_estimate_ambiguity_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(estimate, fn_name, "estimate"));
        copy_tec_text(
            fn_name,
            &value.inner.ambiguity_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare phase-derived slant estimates by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_slant_estimate_equal(
    left: *const SidereonTecPhaseSlantEstimate,
    right: *const SidereonTecPhaseSlantEstimate,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_phase_slant_estimate_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release a phase-derived slant estimate. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_phase_slant_estimate_free(
    estimate: *mut SidereonTecPhaseSlantEstimate,
) {
    ffi_boundary("sidereon_tec_phase_slant_estimate_free", (), || {
        if !estimate.is_null() {
            drop(Box::from_raw(estimate));
        }
    });
}

/// Copy the core Display text for a TEC error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_error_text(
    kind: SidereonTecErrorKind,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_error_text";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        copy_tec_text(
            fn_name,
            &tec_error_from_c(kind).to_string(),
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare two TEC observations by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_observation_equal(
    left: *const SidereonTecObservation,
    right: *const SidereonTecObservation,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_observation_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        let left = core::TecObservation {
            observation: c_try!(dual_observation(fn_name, &left.observation)),
            elevation_rad: left.elevation_rad,
            azimuth_rad: left.azimuth_rad,
        };
        let right = core::TecObservation {
            observation: c_try!(dual_observation(fn_name, &right.observation)),
            elevation_rad: right.elevation_rad,
            azimuth_rad: right.azimuth_rad,
        };
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left == right;
        SidereonStatus::Ok
    })
}

/// Level a continuous TEC arc and return an owned result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_level_slant_arc(
    samples: *const SidereonTecLevelingSample,
    sample_count: usize,
    config: SidereonTecConfig,
    out: *mut *mut SidereonTecLevelingResult,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_level_slant_arc";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let values = c_try!(require_slice(samples, sample_count, fn_name, "samples"));
        let values = values
            .iter()
            .map(|v| core::TecLevelingSample {
                code_slant_tec_tecu: v.code_slant_tec_tecu,
                phase_slant_tec_tecu: v.phase_slant_tec_tecu,
                elevation_rad: v.elevation_rad,
            })
            .collect::<Vec<_>>();
        match core::level_slant_tec_arc(&values, tec_config(config)) {
            Ok(inner) => write_boxed_handle(out, SidereonTecLevelingResult { inner }),
            Err(error) => return tec_error(fn_name, error),
        }
        SidereonStatus::Ok
    })
}

/// Read the phase bias and sample count from a leveling result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_leveling_result_info(
    result: *const SidereonTecLevelingResult,
    out_phase_bias_tecu: *mut f64,
    out_sample_count: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_leveling_result_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        *c_try!(require_out(
            out_phase_bias_tecu,
            fn_name,
            "out_phase_bias_tecu"
        )) = result.inner.phase_bias_tecu;
        *c_try!(require_out(out_sample_count, fn_name, "out_sample_count")) =
            result.inner.samples.len();
        SidereonStatus::Ok
    })
}

/// Copy leveled samples using the standard variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_leveling_result_samples(
    result: *const SidereonTecLevelingResult,
    out: *mut SidereonTecLeveledSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_leveling_result_samples";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        let values = result
            .inner
            .samples
            .iter()
            .map(|v| SidereonTecLeveledSample {
                code_slant_tec_tecu: v.code_slant_tec_tecu,
                phase_slant_tec_tecu: v.phase_slant_tec_tecu,
                leveled_slant_tec_tecu: v.leveled_slant_tec_tecu,
                mapping_function: v.mapping_function,
                vertical_tec_tecu: v.vertical_tec_tecu,
            })
            .collect::<Vec<_>>();
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            &values,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Compare two leveling results by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_leveling_result_equal(
    left: *const SidereonTecLevelingResult,
    right: *const SidereonTecLevelingResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_leveling_result_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release a TEC leveling result. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_leveling_result_free(result: *mut SidereonTecLevelingResult) {
    ffi_boundary("sidereon_tec_leveling_result_free", (), || {
        if !result.is_null() {
            drop(Box::from_raw(result));
        }
    });
}

unsafe fn tec_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonTecEpoch,
    epoch_count: usize,
) -> Result<Vec<core::TecEpoch>, SidereonStatus> {
    require_slice(epochs, epoch_count, fn_name, "epochs")?
        .iter()
        .map(|epoch| {
            let observations = require_slice(
                epoch.observations,
                epoch.observation_count,
                fn_name,
                "epoch.observations",
            )?
            .iter()
            .map(|value| {
                Ok(core::TecObservation {
                    observation: dual_observation(fn_name, &value.observation)?,
                    elevation_rad: value.elevation_rad,
                    azimuth_rad: value.azimuth_rad,
                })
            })
            .collect::<Result<Vec<_>, SidereonStatus>>()?;
            Ok(core::TecEpoch {
                time_s: epoch.time_s,
                receiver_latitude_rad: epoch.receiver_latitude_rad,
                receiver_longitude_rad: epoch.receiver_longitude_rad,
                observations,
            })
        })
        .collect()
}

/// Compare two TEC epochs, including every nested observation, by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_epoch_equal(
    left: *const SidereonTecEpoch,
    right: *const SidereonTecEpoch,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_epoch_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(tec_epochs_from_c(fn_name, left, 1)).remove(0);
        let right = c_try!(tec_epochs_from_c(fn_name, right, 1)).remove(0);
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left == right;
        SidereonStatus::Ok
    })
}

/// Estimate TEC for a time-ordered epoch stream and return an owned result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate(
    epochs: *const SidereonTecEpoch,
    epoch_count: usize,
    config: SidereonTecConfig,
    out: *mut *mut SidereonTecEstimate,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        if out.is_null() {
            set_last_error(format!("{fn_name}: null out"));
            return SidereonStatus::NullPointer;
        }
        *out = ptr::null_mut();
        let epochs = c_try!(tec_epochs_from_c(fn_name, epochs, epoch_count));
        match core::estimate_tec(&epochs, tec_config(config)) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonTecEstimate { inner });
                SidereonStatus::Ok
            }
            Err(error) => tec_error(fn_name, error),
        }
    })
}

/// Return the number of continuous arcs in a TEC estimate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_arc_count(
    estimate: *const SidereonTecEstimate,
    out: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_arc_count";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let estimate = c_try!(require_ref(estimate, fn_name, "estimate"));
        *c_try!(require_out(out, fn_name, "out")) = estimate.inner.arcs.len();
        SidereonStatus::Ok
    })
}

/// Copy an arc's UTF-8 satellite identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_arc_satellite_id(
    estimate: *const SidereonTecEstimate,
    arc_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_arc_satellite_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let estimate = c_try!(require_ref(estimate, fn_name, "estimate"));
        let Some(arc) = estimate.inner.arcs.get(arc_index) else {
            set_last_error(format!("{fn_name}: arc_index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        copy_tec_text(
            fn_name,
            &arc.satellite_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy an arc's UTF-8 ambiguity identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_arc_ambiguity_id(
    estimate: *const SidereonTecEstimate,
    arc_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_arc_ambiguity_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let estimate = c_try!(require_ref(estimate, fn_name, "estimate"));
        let Some(arc) = estimate.inner.arcs.get(arc_index) else {
            set_last_error(format!("{fn_name}: arc_index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        copy_tec_text(
            fn_name,
            &arc.ambiguity_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Read an arc's phase bias and sample count.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_arc_info(
    estimate: *const SidereonTecEstimate,
    arc_index: usize,
    out_phase_bias_tecu: *mut f64,
    out_sample_count: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_arc_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let estimate = c_try!(require_ref(estimate, fn_name, "estimate"));
        let Some(arc) = estimate.inner.arcs.get(arc_index) else {
            set_last_error(format!("{fn_name}: arc_index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *c_try!(require_out(
            out_phase_bias_tecu,
            fn_name,
            "out_phase_bias_tecu"
        )) = arc.phase_bias_tecu;
        *c_try!(require_out(out_sample_count, fn_name, "out_sample_count")) = arc.samples.len();
        SidereonStatus::Ok
    })
}

/// Read one sample from a TEC estimate arc.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_sample(
    estimate: *const SidereonTecEstimate,
    arc_index: usize,
    sample_index: usize,
    out: *mut SidereonTecEstimateSample,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_sample";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let estimate = c_try!(require_ref(estimate, fn_name, "estimate"));
        let Some(sample) = estimate
            .inner
            .arcs
            .get(arc_index)
            .and_then(|arc| arc.samples.get(sample_index))
        else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *c_try!(require_out(out, fn_name, "out")) = SidereonTecEstimateSample {
            time_s: sample.time_s,
            elevation_rad: sample.elevation_rad,
            azimuth_rad: sample.azimuth_rad,
            code_geometry_free_m: sample.code_geometry_free_m,
            phase_geometry_free_m: sample.phase_geometry_free_m,
            code_slant_tec_tecu: sample.code_slant_tec_tecu,
            phase_slant_tec_tecu: sample.phase_slant_tec_tecu,
            leveled_slant_tec_tecu: sample.leveled_slant_tec_tecu,
            mapping_function: sample.mapping_function,
            vertical_tec_tecu: sample.vertical_tec_tecu,
            pierce_point: ipp(sample.pierce_point),
        };
        SidereonStatus::Ok
    })
}

/// Compare two TEC estimates by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_equal(
    left: *const SidereonTecEstimate,
    right: *const SidereonTecEstimate,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_tec_estimate_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release a TEC estimate. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_estimate_free(estimate: *mut SidereonTecEstimate) {
    ffi_boundary("sidereon_tec_estimate_free", (), || {
        if !estimate.is_null() {
            drop(Box::from_raw(estimate));
        }
    });
}
