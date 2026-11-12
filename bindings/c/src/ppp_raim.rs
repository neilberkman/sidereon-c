use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};
use sidereon_core::geometry::LineOfSight;
use sidereon_core::precise_positioning as core;

/// PPP snapshot RAIM configuration. This plain C value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppRaimConfig {
    /// False-alarm probability used to derive the chi-square threshold.
    pub false_alarm_probability: f64,
    /// Missed-detection probability used for protection-level scaling.
    pub missed_detection_probability: f64,
    /// Residual sigma in metres, applied after solver row weights.
    pub measurement_sigma_m: f64,
    /// Whether `chi_square_threshold` is present.
    pub has_chi_square_threshold: bool,
    /// Optional fixed chi-square threshold.
    pub chi_square_threshold: f64,
}

/// One lossless PPP float residual input row.
#[repr(C)]
pub struct SidereonPppRaimResidual {
    /// Zero-based input epoch index.
    pub epoch_index: usize,
    /// Null-terminated UTF-8 satellite identifier.
    pub satellite_id: *const c_char,
    /// Null-terminated UTF-8 ambiguity identifier.
    pub ambiguity_id: *const c_char,
    /// Code prefit residual, in metres.
    pub code_m: f64,
    /// Phase prefit residual, in metres.
    pub phase_m: f64,
    /// Code inverse-sigma row weight.
    pub code_weight: f64,
    /// Phase inverse-sigma row weight.
    pub phase_weight: f64,
}

/// One lossless PPP snapshot geometry row.
#[repr(C)]
pub struct SidereonPppRaimGeometryRow {
    /// Null-terminated UTF-8 satellite identifier.
    pub satellite_id: *const c_char,
    /// Receiver-to-satellite ECEF line of sight.
    pub line_of_sight: SidereonLineOfSight,
}

/// PPP RAIM global-test status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppRaimStatus {
    /// The test passed.
    Passed = 0,
    /// The test statistic exceeded its threshold.
    FaultDetected = 1,
    /// The residual set had no positive redundancy.
    NotEnoughRedundancy = 2,
}

/// Fixed fields of a PPP RAIM result.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppRaimResultInfo {
    /// Global-test status.
    pub status: SidereonPppRaimStatus,
    /// Whether a fault was detected.
    pub detected: bool,
    /// Sum of squared weighted residuals.
    pub test_statistic: f64,
    /// Whether a threshold is present.
    pub has_threshold: bool,
    /// Chi-square threshold.
    pub threshold: f64,
    /// Residual redundancy.
    pub redundancy: isize,
    /// Number of satellite statistics.
    pub satellite_statistic_count: usize,
    /// Whether a most-likely-fault identifier is present.
    pub has_most_likely_fault: bool,
    /// Whether HPL is present.
    pub has_hpl_m: bool,
    /// Horizontal protection level, in metres.
    pub hpl_m: f64,
    /// Whether VPL is present.
    pub has_vpl_m: bool,
    /// Vertical protection level, in metres.
    pub vpl_m: f64,
}

/// Fixed fields of one per-satellite statistic.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppRaimSatelliteStatistic {
    /// Standardized absolute code residual.
    pub code: f64,
    /// Standardized absolute phase residual.
    pub phase: f64,
    /// Maximum testable component.
    pub statistic: f64,
    /// Whether the code row is testable.
    pub code_testable: bool,
    /// Whether the phase row is testable.
    pub phase_testable: bool,
}

/// Horizontal and vertical protection levels. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppRaimProtectionLevels {
    /// Horizontal protection level, in metres.
    pub hpl_m: f64,
    /// Vertical protection level, in metres.
    pub vpl_m: f64,
}

/// Owned global-test result.
pub struct SidereonPppRaimResult {
    inner: core::RaimResult,
}

/// Owned per-satellite identification result.
pub struct SidereonPppRaimIdentification {
    inner: core::RaimIdentification,
}

/// Terminal PPP RAIM/FDE status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppRaimFdeStatus {
    /// The first solution passed.
    Clean = 0,
    /// Exclusion restored integrity.
    Restored = 1,
    /// Exclusion would exhaust redundancy.
    CannotExclude = 2,
    /// No valid exclusion restored integrity.
    IntegrityNotRestored = 3,
}

/// PPP RAIM refusal kind. Values correspond one-to-one with core variants.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppRaimErrorKind {
    /// Configuration field refusal.
    InvalidConfig = 0,
    /// Residual field refusal.
    InvalidResidual = 1,
    /// Geometry field refusal.
    InvalidGeometry = 2,
    /// Nested DOP refusal.
    Dop = 3,
    /// Singular geometry.
    SingularGeometry = 4,
}

/// PPP RAIM/FDE refusal kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppRaimFdeErrorKind {
    /// Underlying float solve refusal.
    Solve = 0,
    /// RAIM refusal.
    Raim = 1,
}

/// Fixed metadata for an owned FDE result.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppRaimFdeInfo {
    /// Terminal FDE status.
    pub status: SidereonPppRaimFdeStatus,
    /// Number of excluded satellite identifiers.
    pub excluded_satellite_count: usize,
}

/// Owned PPP RAIM/FDE result.
pub struct SidereonPppRaimFdeResult {
    inner: core::RaimFdeResult,
}

fn config_to_core(value: SidereonPppRaimConfig) -> core::RaimConfig {
    let mut config = core::RaimConfig::default();
    config.false_alarm_probability = value.false_alarm_probability;
    config.missed_detection_probability = value.missed_detection_probability;
    config.measurement_sigma_m = value.measurement_sigma_m;
    config.chi_square_threshold = value
        .has_chi_square_threshold
        .then_some(value.chi_square_threshold);
    config
}

fn config_from_core(value: core::RaimConfig) -> SidereonPppRaimConfig {
    SidereonPppRaimConfig {
        false_alarm_probability: value.false_alarm_probability,
        missed_detection_probability: value.missed_detection_probability,
        measurement_sigma_m: value.measurement_sigma_m,
        has_chi_square_threshold: value.chi_square_threshold.is_some(),
        chi_square_threshold: value.chi_square_threshold.unwrap_or(0.0),
    }
}

fn status_to_c(value: core::RaimStatus) -> SidereonPppRaimStatus {
    match value {
        core::RaimStatus::Passed => SidereonPppRaimStatus::Passed,
        core::RaimStatus::FaultDetected => SidereonPppRaimStatus::FaultDetected,
        core::RaimStatus::NotEnoughRedundancy => SidereonPppRaimStatus::NotEnoughRedundancy,
    }
}

fn fde_status_to_c(value: core::RaimFdeStatus) -> SidereonPppRaimFdeStatus {
    match value {
        core::RaimFdeStatus::Clean => SidereonPppRaimFdeStatus::Clean,
        core::RaimFdeStatus::Restored => SidereonPppRaimFdeStatus::Restored,
        core::RaimFdeStatus::CannotExclude => SidereonPppRaimFdeStatus::CannotExclude,
        core::RaimFdeStatus::IntegrityNotRestored => SidereonPppRaimFdeStatus::IntegrityNotRestored,
    }
}

unsafe fn residuals_from_c(
    fn_name: &str,
    rows: *const SidereonPppRaimResidual,
    count: usize,
) -> Result<Vec<core::FloatResidual>, SidereonStatus> {
    require_slice(rows, count, fn_name, "residuals")?
        .iter()
        .map(|row| {
            Ok(core::FloatResidual {
                epoch_index: row.epoch_index,
                satellite_id: parse_c_string(fn_name, "residual.satellite_id", row.satellite_id)?,
                ambiguity_id: parse_c_string(fn_name, "residual.ambiguity_id", row.ambiguity_id)?,
                code_m: row.code_m,
                phase_m: row.phase_m,
                code_weight: row.code_weight,
                phase_weight: row.phase_weight,
            })
        })
        .collect()
}

unsafe fn geometry_from_c(
    fn_name: &str,
    rows: *const SidereonPppRaimGeometryRow,
    count: usize,
) -> Result<Vec<core::RaimGeometryRow>, SidereonStatus> {
    require_slice(rows, count, fn_name, "geometry")?
        .iter()
        .map(|row| {
            Ok(core::RaimGeometryRow {
                satellite_id: parse_c_string(fn_name, "geometry.satellite_id", row.satellite_id)?,
                line_of_sight: LineOfSight::new(
                    row.line_of_sight.e_x,
                    row.line_of_sight.e_y,
                    row.line_of_sight.e_z,
                ),
            })
        })
        .collect()
}

/// Compare two PPP RAIM geometry rows using the engine's value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_geometry_row_equal(
    left: *const SidereonPppRaimGeometryRow,
    right: *const SidereonPppRaimGeometryRow,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_geometry_row_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(geometry_from_c(fn_name, left, 1));
        let right = c_try!(geometry_from_c(fn_name, right, 1));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left[0] == right[0];
        SidereonStatus::Ok
    })
}

fn raim_error_value(error: &core::RaimError) -> Value {
    match error {
        core::RaimError::InvalidConfig { field, reason } => json!({
            "kind": "invalid_config", "message": error.to_string(),
            "fields": {"field": field, "reason": reason}
        }),
        core::RaimError::InvalidResidual { field } => json!({
            "kind": "invalid_residual", "message": error.to_string(), "fields": {"field": field}
        }),
        core::RaimError::InvalidGeometry { field, reason } => json!({
            "kind": "invalid_geometry", "message": error.to_string(),
            "fields": {"field": field, "reason": reason}
        }),
        core::RaimError::Dop(cause) => json!({
            "kind": "dop", "message": error.to_string(),
            "fields": {"cause": crate::engine_error::dop_error_value(cause)}
        }),
        core::RaimError::SingularGeometry => json!({
            "kind": "singular_geometry", "message": error.to_string(), "fields": {}
        }),
    }
}

fn map_raim_error(fn_name: &str, error: core::RaimError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PppRaim,
        fn_name,
        raim_error_value(&error),
    );
    set_last_error(format!("{fn_name}: {error}"));
    match error {
        core::RaimError::SingularGeometry | core::RaimError::Dop(_) => SidereonStatus::Solve,
        _ => SidereonStatus::InvalidArgument,
    }
}

fn map_fde_error(fn_name: &str, error: core::RaimFdeError) -> SidereonStatus {
    let (tree, status) = match &error {
        core::RaimFdeError::Raim(cause) => (
            json!({"kind": "raim", "message": error.to_string(), "fields": {"cause": raim_error_value(cause)}}),
            match cause {
                core::RaimError::SingularGeometry | core::RaimError::Dop(_) => {
                    SidereonStatus::Solve
                }
                _ => SidereonStatus::InvalidArgument,
            },
        ),
        core::RaimFdeError::Solve(cause) => (
            json!({
                "kind": "solve", "message": error.to_string(),
                "fields": {"cause": crate::engine_error::ppp_float_solve_error_value(cause)}
            }),
            match cause {
                core::FloatSolveError::InvalidClockCount { .. }
                | core::FloatSolveError::InvalidSolveOption { .. }
                | core::FloatSolveError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
                core::FloatSolveError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
                _ => SidereonStatus::Solve,
            },
        ),
    };
    record_engine_error(SidereonEngineErrorFamily::PppRaim, fn_name, tree);
    set_last_error(format!("{fn_name}: {error}"));
    status
}

fn result_info(value: &core::RaimResult) -> SidereonPppRaimResultInfo {
    SidereonPppRaimResultInfo {
        status: status_to_c(value.status),
        detected: value.detected,
        test_statistic: value.test_statistic,
        has_threshold: value.threshold.is_some(),
        threshold: value.threshold.unwrap_or(0.0),
        redundancy: value.redundancy,
        satellite_statistic_count: value.satellite_statistics.len(),
        has_most_likely_fault: value.most_likely_fault.is_some(),
        has_hpl_m: value.hpl_m.is_some(),
        hpl_m: value.hpl_m.unwrap_or(0.0),
        has_vpl_m: value.vpl_m.is_some(),
        vpl_m: value.vpl_m.unwrap_or(0.0),
    }
}

/// Initialize RAIM configuration from the core default.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_config_default(
    out: *mut SidereonPppRaimConfig,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_config_default";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        *c_try!(require_out(out, fn_name, "out")) = config_from_core(core::RaimConfig::default());
        SidereonStatus::Ok
    })
}

/// Copy representative core Display text for a PPP RAIM error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_error_text(
    kind: SidereonPppRaimErrorKind,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_error_text";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let error = match kind {
            SidereonPppRaimErrorKind::InvalidConfig => core::RaimError::InvalidConfig {
                field: "field",
                reason: "reason",
            },
            SidereonPppRaimErrorKind::InvalidResidual => {
                core::RaimError::InvalidResidual { field: "field" }
            }
            SidereonPppRaimErrorKind::InvalidGeometry => core::RaimError::InvalidGeometry {
                field: "field",
                reason: "reason",
            },
            SidereonPppRaimErrorKind::Dop => {
                core::RaimError::Dop(sidereon_core::geometry::DopError::Singular)
            }
            SidereonPppRaimErrorKind::SingularGeometry => core::RaimError::SingularGeometry,
        };
        copy_text(
            fn_name,
            &error.to_string(),
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy representative core Display text for a PPP RAIM/FDE error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_error_text(
    kind: SidereonPppRaimFdeErrorKind,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_error_text";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let text = match kind {
            SidereonPppRaimFdeErrorKind::Raim => {
                core::RaimFdeError::Raim(core::RaimError::SingularGeometry).to_string()
            }
            SidereonPppRaimFdeErrorKind::Solve => {
                core::RaimFdeError::Solve(core::FloatSolveError::SingularGeometry).to_string()
            }
        };
        copy_text(fn_name, &text, out, len, out_written, out_required)
    })
}

unsafe fn write_result(
    out: *mut *mut SidereonPppRaimResult,
    inner: core::RaimResult,
) -> SidereonStatus {
    write_boxed_handle(out, SidereonPppRaimResult { inner });
    SidereonStatus::Ok
}

/// Run the PPP RAIM global test over float residual rows.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_global_test(
    residuals: *const SidereonPppRaimResidual,
    residual_count: usize,
    n_states: usize,
    config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimResult,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_global_test";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let rows = c_try!(residuals_from_c(fn_name, residuals, residual_count));
        match core::raim::global_test(&rows, n_states, config_to_core(config)) {
            Ok(inner) => write_result(out, inner),
            Err(error) => map_raim_error(fn_name, error),
        }
    })
}

/// Run the PPP RAIM global test with snapshot geometry and identification.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_global_test_with_geometry(
    residuals: *const SidereonPppRaimResidual,
    residual_count: usize,
    geometry: *const SidereonPppRaimGeometryRow,
    geometry_count: usize,
    config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimResult,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_global_test_with_geometry";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let residuals = c_try!(residuals_from_c(fn_name, residuals, residual_count));
        let geometry = c_try!(geometry_from_c(fn_name, geometry, geometry_count));
        match core::raim::global_test_with_geometry(&residuals, &geometry, config_to_core(config)) {
            Ok(inner) => write_result(out, inner),
            Err(error) => map_raim_error(fn_name, error),
        }
    })
}

/// Compute per-satellite standardized residual statistics.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_per_satellite_statistics(
    residuals: *const SidereonPppRaimResidual,
    residual_count: usize,
    geometry: *const SidereonPppRaimGeometryRow,
    geometry_count: usize,
    config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimIdentification,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_per_satellite_statistics";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let residuals = c_try!(residuals_from_c(fn_name, residuals, residual_count));
        let geometry = c_try!(geometry_from_c(fn_name, geometry, geometry_count));
        match core::raim::per_satellite_statistics(&residuals, &geometry, config_to_core(config)) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppRaimIdentification { inner });
                SidereonStatus::Ok
            }
            Err(error) => map_raim_error(fn_name, error),
        }
    })
}

/// Compute PPP RAIM horizontal and vertical protection levels.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_protection_levels(
    geometry: *const SidereonPppRaimGeometryRow,
    geometry_count: usize,
    receiver: SidereonGeodetic,
    config: SidereonPppRaimConfig,
    out: *mut SidereonPppRaimProtectionLevels,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_protection_levels";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let geometry = c_try!(geometry_from_c(fn_name, geometry, geometry_count));
        let receiver = c_try!(geodetic_to_wgs84(fn_name, "receiver", receiver));
        match core::raim::protection_levels(&geometry, receiver, config_to_core(config)) {
            Ok(value) => {
                *c_try!(require_out(out, fn_name, "out")) = SidereonPppRaimProtectionLevels {
                    hpl_m: value.hpl_m,
                    vpl_m: value.vpl_m,
                };
                SidereonStatus::Ok
            }
            Err(error) => map_raim_error(fn_name, error),
        }
    })
}

/// Read fixed metadata from a RAIM result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_info(
    result: *const SidereonPppRaimResult,
    out: *mut SidereonPppRaimResultInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_result_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        *c_try!(require_out(out, fn_name, "out")) = result_info(&result.inner);
        SidereonStatus::Ok
    })
}

unsafe fn copy_text(
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

/// Copy the optional most-likely-fault identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_most_likely_fault(
    result: *const SidereonPppRaimResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_result_most_likely_fault";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        let Some(text) = &result.inner.most_likely_fault else {
            set_last_error(format!("{fn_name}: result has no most_likely_fault"));
            return SidereonStatus::InvalidArgument;
        };
        copy_text(fn_name, text, out, len, out_written, out_required)
    })
}

/// Read one per-satellite statistic from a RAIM result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_satellite_statistic(
    result: *const SidereonPppRaimResult,
    index: usize,
    out: *mut SidereonPppRaimSatelliteStatistic,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_result_satellite_statistic";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        let Some(value) = result.inner.satellite_statistics.get(index) else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *c_try!(require_out(out, fn_name, "out")) = SidereonPppRaimSatelliteStatistic {
            code: value.code,
            phase: value.phase,
            statistic: value.statistic,
            code_testable: value.code_testable,
            phase_testable: value.phase_testable,
        };
        SidereonStatus::Ok
    })
}

/// Copy one statistic's satellite identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_satellite_id(
    result: *const SidereonPppRaimResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_result_satellite_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        let Some(value) = result.inner.satellite_statistics.get(index) else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        copy_text(
            fn_name,
            &value.satellite_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare two RAIM results by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_equal(
    left: *const SidereonPppRaimResult,
    right: *const SidereonPppRaimResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_result_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release a RAIM result. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_result_free(result: *mut SidereonPppRaimResult) {
    ffi_boundary("sidereon_ppp_raim_result_free", (), || {
        if !result.is_null() {
            drop(Box::from_raw(result));
        }
    });
}

/// Return fixed metadata from an identification result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_info(
    identification: *const SidereonPppRaimIdentification,
    out_statistic_count: *mut usize,
    out_has_most_likely_fault: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_identification_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(identification, fn_name, "identification"));
        *c_try!(require_out(
            out_statistic_count,
            fn_name,
            "out_statistic_count"
        )) = value.inner.statistics.len();
        *c_try!(require_out(
            out_has_most_likely_fault,
            fn_name,
            "out_has_most_likely_fault"
        )) = value.inner.most_likely_fault.is_some();
        SidereonStatus::Ok
    })
}

/// Read one statistic from an identification result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_statistic(
    identification: *const SidereonPppRaimIdentification,
    index: usize,
    out: *mut SidereonPppRaimSatelliteStatistic,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_identification_statistic";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(identification, fn_name, "identification"));
        let Some(row) = value.inner.statistics.get(index) else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *c_try!(require_out(out, fn_name, "out")) = SidereonPppRaimSatelliteStatistic {
            code: row.code,
            phase: row.phase,
            statistic: row.statistic,
            code_testable: row.code_testable,
            phase_testable: row.phase_testable,
        };
        SidereonStatus::Ok
    })
}

/// Copy an identification statistic's satellite identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_satellite_id(
    identification: *const SidereonPppRaimIdentification,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_identification_satellite_id";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(identification, fn_name, "identification"));
        let Some(row) = value.inner.statistics.get(index) else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        copy_text(
            fn_name,
            &row.satellite_id,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy the identification's optional most-likely-fault identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_most_likely_fault(
    identification: *const SidereonPppRaimIdentification,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_identification_most_likely_fault";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(identification, fn_name, "identification"));
        let Some(text) = &value.inner.most_likely_fault else {
            set_last_error(format!("{fn_name}: result has no most_likely_fault"));
            return SidereonStatus::InvalidArgument;
        };
        copy_text(fn_name, text, out, len, out_written, out_required)
    })
}

/// Compare two identification results by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_equal(
    left: *const SidereonPppRaimIdentification,
    right: *const SidereonPppRaimIdentification,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_identification_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release an identification result. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_identification_free(
    identification: *mut SidereonPppRaimIdentification,
) {
    ffi_boundary("sidereon_ppp_raim_identification_free", (), || {
        if !identification.is_null() {
            drop(Box::from_raw(identification));
        }
    });
}

unsafe fn fde_impl(
    fn_name: &str,
    sp3: *const SidereonSp3,
    config: *const SidereonPppFloatConfigV2,
    raim_config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimFdeResult,
    solve_driver: bool,
) -> SidereonStatus {
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        *out = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, fn_name, "sp3"));
        let config = c_try!(require_ref(config, fn_name, "config"));
        if config.legacy.epoch_count != 1 {
            set_last_error(format!("{fn_name}: config.legacy.epoch_count must be 1"));
            return SidereonStatus::InvalidArgument;
        }
        let mut epochs = c_try!(ppp_epochs_from_c(fn_name, config.legacy.epochs, 1));
        let initial_state = c_try!(ppp_float_state_from_c(
            fn_name,
            &config.legacy.initial_state,
            1
        ));
        let mut solve_config = c_try!(ppp_float_config_from_c(fn_name, &config.legacy));
        solve_config.estimate_residual_ionosphere = config.estimate_residual_ionosphere;
        let result = if solve_driver {
            core::solve_float_epoch_with_raim(
                &sp3.inner,
                epochs.remove(0),
                initial_state,
                solve_config,
                config_to_core(raim_config),
            )
        } else {
            core::raim::fde_float_epoch(
                &sp3.inner,
                epochs.remove(0),
                initial_state,
                solve_config,
                config_to_core(raim_config),
            )
        };
        match result {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppRaimFdeResult { inner });
                SidereonStatus::Ok
            }
            Err(error) => map_fde_error(fn_name, error),
        }
    })
}

/// Run one-epoch PPP RAIM fault detection and exclusion.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_float_epoch(
    sp3: *const SidereonSp3,
    config: *const SidereonPppFloatConfigV2,
    raim_config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimFdeResult,
) -> SidereonStatus {
    fde_impl(
        "sidereon_ppp_raim_fde_float_epoch",
        sp3,
        config,
        raim_config,
        out,
        false,
    )
}

/// Solve one float epoch with PPP RAIM/FDE.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_solve_float_epoch_with_raim(
    sp3: *const SidereonSp3,
    config: *const SidereonPppFloatConfigV2,
    raim_config: SidereonPppRaimConfig,
    out: *mut *mut SidereonPppRaimFdeResult,
) -> SidereonStatus {
    fde_impl(
        "sidereon_ppp_solve_float_epoch_with_raim",
        sp3,
        config,
        raim_config,
        out,
        true,
    )
}

/// Read fixed FDE metadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_info(
    result: *const SidereonPppRaimFdeResult,
    out: *mut SidereonPppRaimFdeInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_result_info";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        *c_try!(require_out(out, fn_name, "out")) = SidereonPppRaimFdeInfo {
            status: fde_status_to_c(result.inner.status),
            excluded_satellite_count: result.inner.excluded_sats.len(),
        };
        SidereonStatus::Ok
    })
}

/// Copy one excluded satellite identifier without truncation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_excluded_satellite(
    result: *const SidereonPppRaimFdeResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_result_excluded_satellite";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        let Some(text) = result.inner.excluded_sats.get(index) else {
            set_last_error(format!("{fn_name}: index out of range"));
            return SidereonStatus::InvalidArgument;
        };
        copy_text(fn_name, text, out, len, out_written, out_required)
    })
}

/// Clone the final float solution into its standard owned C handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_solution(
    result: *const SidereonPppRaimFdeResult,
    out: *mut *mut SidereonPppFloatSolution,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_result_solution";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        write_boxed_handle(
            out,
            SidereonPppFloatSolution {
                inner: result.inner.solution.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Clone the final RAIM result into an owned RAIM handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_raim(
    result: *const SidereonPppRaimFdeResult,
    out: *mut *mut SidereonPppRaimResult,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_result_raim";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let result = c_try!(require_ref(result, fn_name, "result"));
        write_boxed_handle(
            out,
            SidereonPppRaimResult {
                inner: result.inner.raim.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Compare two FDE results by core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_equal(
    left: *const SidereonPppRaimFdeResult,
    right: *const SidereonPppRaimFdeResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_raim_fde_result_equal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(require_ref(left, fn_name, "left"));
        let right = c_try!(require_ref(right, fn_name, "right"));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left.inner == right.inner;
        SidereonStatus::Ok
    })
}

/// Release an FDE result. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_raim_fde_result_free(result: *mut SidereonPppRaimFdeResult) {
    ffi_boundary("sidereon_ppp_raim_fde_result_free", (), || {
        if !result.is_null() {
            drop(Box::from_raw(result));
        }
    });
}
