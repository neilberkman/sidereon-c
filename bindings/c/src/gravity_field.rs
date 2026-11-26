use super::*;
use crate::covariance::map_propagation_error;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::forces::{
    IcgemError, IcgemErrors, IcgemFormat, IcgemGravityField, IcgemHeader, IcgemNormalization,
    IcgemSigmas, IcgemTermKind, IcgemTimeVariableTerm, SphericalHarmonicCoefficient,
    SphericalHarmonicGravity,
};

/// Owned ICGEM gravity field. Release with sidereon_icgem_gravity_field_free.
pub struct SidereonIcgemGravityField {
    inner: IcgemGravityField,
}

/// Owned spherical harmonic gravity. Release with sidereon_spherical_harmonic_gravity_free.
pub struct SidereonSphericalHarmonicGravity {
    inner: Arc<SphericalHarmonicGravity>,
}

fn icgem_error_value(error: &IcgemError) -> serde_json::Value {
    use IcgemError as E;
    let (kind, fields) = match error {
        E::MissingField {
            line,
            keyword,
            field,
        } => (
            "missing_field",
            json!({"line": line, "keyword": keyword, "field": field}),
        ),
        E::InvalidField {
            line,
            keyword,
            field,
            value,
        } => (
            "invalid_field",
            json!({"line": line, "keyword": keyword, "field": field, "value": value}),
        ),
        E::MissingHeaderKeyword { keyword } => {
            ("missing_header_keyword", json!({"keyword": keyword}))
        }
        E::RepeatedHeaderKeyword {
            line,
            first_line,
            keyword,
        } => (
            "repeated_header_keyword",
            json!({"line": line, "first_line": first_line, "keyword": keyword}),
        ),
        E::MissingEndOfHead => ("missing_end_of_head", json!({})),
        E::KeywordNotInFormat { line, keyword } => (
            "keyword_not_in_format",
            json!({"line": line, "keyword": keyword}),
        ),
        E::UnknownRecordKeyword { line, keyword } => (
            "unknown_record_keyword",
            json!({"line": line, "keyword": keyword}),
        ),
        E::MissingRecordKeyword { line } => ("missing_record_keyword", json!({"line": line})),
        E::DegreeAboveMaximum {
            line,
            degree,
            max_degree,
        } => (
            "degree_above_maximum",
            json!({"line": line, "degree": degree, "max_degree": max_degree}),
        ),
        E::OrderAboveDegree {
            line,
            degree,
            order,
        } => (
            "order_above_degree",
            json!({"line": line, "degree": degree, "order": order}),
        ),
        E::RepeatedCoefficient {
            line,
            first_line,
            degree,
            order,
        } => (
            "repeated_coefficient",
            json!({"line": line, "first_line": first_line, "degree": degree, "order": order}),
        ),
        E::OverlappingTerm {
            line,
            first_line,
            keyword,
            degree,
            order,
        } => (
            "overlapping_term",
            json!({"line": line, "first_line": first_line, "keyword": keyword, "degree": degree, "order": order}),
        ),
        E::UncoveredTerm {
            line,
            keyword,
            degree,
            order,
        } => (
            "uncovered_term",
            json!({"line": line, "keyword": keyword, "degree": degree, "order": order}),
        ),
        E::MissingCoefficient { degree, order } => (
            "missing_coefficient",
            json!({"degree": degree, "order": order}),
        ),
        E::TooFewRecords {
            max_degree,
            required,
            lines,
        } => (
            "too_few_records",
            json!({"max_degree": max_degree, "required": required, "lines": lines}),
        ),
        E::NormalizationOutOfRange {
            line,
            degree,
            order,
        } => (
            "normalization_out_of_range",
            json!({"line": line, "degree": degree, "order": order}),
        ),
        E::TruncationOutOfRange {
            degree,
            order,
            max_degree,
        } => (
            "truncation_out_of_range",
            json!({"degree": degree, "order": order, "max_degree": max_degree}),
        ),
        E::UnknownTideSystem => ("unknown_tide_system", json!({})),
        E::TideSystemConflict { stated, assumed } => (
            "tide_system_conflict",
            json!({"stated": tide_system_code(*stated), "assumed": tide_system_code(*assumed)}),
        ),
        E::EpochOutsideValidity {
            degree,
            order,
            epoch_tt_seconds,
        } => (
            "epoch_outside_validity",
            json!({"degree": degree, "order": order, "epoch_tt_seconds": engine_f64(*epoch_tt_seconds)}),
        ),
        E::NonFiniteEpoch => ("non_finite_epoch", json!({})),
        E::NonFiniteCoefficient {
            degree,
            order,
            epoch_tt_seconds,
        } => (
            "non_finite_coefficient",
            json!({"degree": degree, "order": order, "epoch_tt_seconds": engine_f64(*epoch_tt_seconds)}),
        ),

        _ => ("unknown", json!({})),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_icgem_error(operation: &str, err: IcgemError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Icgem,
        operation,
        icgem_error_value(&err),
    );
    extra_invalid_arg(operation, err)
}

fn tide_system_code(value: CoreTideSystem) -> u32 {
    match value {
        CoreTideSystem::TideFree => 0,
        CoreTideSystem::ZeroTide => 1,
        CoreTideSystem::MeanTide => 2,
    }
}

/// Uncertainty columns in an ICGEM coefficient record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonIcgemErrors {
    /// No uncertainty columns.
    No = 0,
    /// Calibrated uncertainties.
    Calibrated = 1,
    /// Formal uncertainties.
    Formal = 2,
    /// Calibrated and formal uncertainties.
    CalibratedAndFormal = 3,
}

fn icgem_errors_code(value: IcgemErrors) -> u32 {
    match value {
        IcgemErrors::No => SidereonIcgemErrors::No as u32,
        IcgemErrors::Calibrated => SidereonIcgemErrors::Calibrated as u32,
        IcgemErrors::Formal => SidereonIcgemErrors::Formal as u32,
        IcgemErrors::CalibratedAndFormal => SidereonIcgemErrors::CalibratedAndFormal as u32,
    }
}

/// Normalization of coefficients declared by an ICGEM field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonIcgemNormalization {
    /// Fully normalized coefficients.
    FullyNormalized = 0,
    /// Unnormalized coefficients.
    Unnormalized = 1,
}

fn icgem_normalization_code(value: IcgemNormalization) -> u32 {
    match value {
        IcgemNormalization::FullyNormalized => SidereonIcgemNormalization::FullyNormalized as u32,
        IcgemNormalization::Unnormalized => SidereonIcgemNormalization::Unnormalized as u32,
    }
}

/// ICGEM file format.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonIcgemFormat {
    /// ICGEM 1.0.
    Icgem1 = 0,
    /// ICGEM 2.0.
    Icgem2 = 1,
}

fn icgem_format_code(value: IcgemFormat) -> u32 {
    match value {
        IcgemFormat::Icgem1_0 => SidereonIcgemFormat::Icgem1 as u32,
        IcgemFormat::Icgem2_0 => SidereonIcgemFormat::Icgem2 as u32,
    }
}

/// Kind of a time-variable ICGEM coefficient record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonIcgemTermKind {
    /// Time-variable constant part.
    Constant = 0,
    /// Trend per Julian year.
    Trend = 1,
    /// Cosine amplitude.
    Cosine = 2,
    /// Sine amplitude.
    Sine = 3,
}

/// ICGEM gravity field header. Compare fields by value and compare the model name separately.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonIcgemHeader {
    /// Gravity constant in m³/s².
    pub gravity_constant_m3_s2: f64,
    /// Reference radius in metres.
    pub radius_m: f64,
    /// Maximum degree stated in the file.
    pub max_degree: u16,
    /// Uncertainty columns as SidereonIcgemErrors.
    pub errors: u32,
    /// File normalization as SidereonIcgemNormalization.
    pub normalization: u32,
    /// Whether a tide system is stated.
    pub has_tide_system: bool,
    /// Declared tide system as SidereonGravityTideSystem when present.
    pub tide_system: u32,
    /// File format as SidereonIcgemFormat.
    pub format: u32,
}

impl From<&IcgemHeader> for SidereonIcgemHeader {
    fn from(value: &IcgemHeader) -> Self {
        Self {
            gravity_constant_m3_s2: value.gravity_constant_m3_s2,
            radius_m: value.radius_m,
            max_degree: value.max_degree,
            errors: icgem_errors_code(value.errors),
            normalization: icgem_normalization_code(value.normalization),
            has_tide_system: value.tide_system.is_some(),
            tide_system: value.tide_system.map_or(0, tide_system_code),
            format: icgem_format_code(value.format),
        }
    }
}

/// Uncertainties of one normalized coefficient pair. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonIcgemSigmas {
    /// Whether calibrated uncertainties are present.
    pub has_calibrated: bool,
    /// Calibrated C and S uncertainties when present.
    pub calibrated: [f64; 2],
    /// Whether formal uncertainties are present.
    pub has_formal: bool,
    /// Formal C and S uncertainties when present.
    pub formal: [f64; 2],
}

impl From<IcgemSigmas> for SidereonIcgemSigmas {
    fn from(value: IcgemSigmas) -> Self {
        Self {
            has_calibrated: value.calibrated.is_some(),
            calibrated: value.calibrated.unwrap_or_default(),
            has_formal: value.formal.is_some(),
            formal: value.formal.unwrap_or_default(),
        }
    }
}

/// Terms of a time-dependent gravity coefficient. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonIcgemTimeVariableTerm {
    /// One-based source line.
    pub line: usize,
    /// Record kind as SidereonIcgemTermKind.
    pub kind: u32,
    /// Period in Julian years for sine and cosine records, zero otherwise.
    pub period_years: f64,
    /// Harmonic degree.
    pub degree: u16,
    /// Harmonic order.
    pub order: u16,
    /// Fully normalized C term.
    pub c: f64,
    /// Fully normalized S term.
    pub s: f64,
    /// Coefficient uncertainties.
    pub sigmas: SidereonIcgemSigmas,
    /// Reference epoch in TT seconds past J2000.
    pub reference_tt_seconds: f64,
    /// Inclusive start in TT seconds past J2000.
    pub valid_from_tt_seconds: f64,
    /// Exclusive end in TT seconds past J2000.
    pub valid_until_tt_seconds: f64,
}

impl From<IcgemTimeVariableTerm> for SidereonIcgemTimeVariableTerm {
    fn from(value: IcgemTimeVariableTerm) -> Self {
        Self {
            line: value.line,
            kind: match value.kind {
                IcgemTermKind::Constant => SidereonIcgemTermKind::Constant as u32,
                IcgemTermKind::Trend => SidereonIcgemTermKind::Trend as u32,
                IcgemTermKind::Cosine { .. } => SidereonIcgemTermKind::Cosine as u32,
                IcgemTermKind::Sine { .. } => SidereonIcgemTermKind::Sine as u32,
            },
            period_years: match value.kind {
                IcgemTermKind::Cosine { period_years } | IcgemTermKind::Sine { period_years } => {
                    period_years
                }
                _ => 0.0,
            },
            degree: value.degree,
            order: value.order,
            c: value.c,
            s: value.s,
            sigmas: value.sigmas.into(),
            reference_tt_seconds: value.reference_tt_seconds,
            valid_from_tt_seconds: value.valid_from_tt_seconds,
            valid_until_tt_seconds: value.valid_until_tt_seconds,
        }
    }
}

/// Fully normalized harmonic coefficient pair. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonSphericalHarmonicCoefficient {
    /// Harmonic degree.
    pub degree: u16,
    /// Harmonic order.
    pub order: u16,
    /// Normalized C coefficient.
    pub c: f64,
    /// Normalized S coefficient.
    pub s: f64,
}

impl From<SphericalHarmonicCoefficient> for SidereonSphericalHarmonicCoefficient {
    fn from(value: SphericalHarmonicCoefficient) -> Self {
        Self {
            degree: value.degree,
            order: value.order,
            c: value.c,
            s: value.s,
        }
    }
}

/// Harmonic coefficient lookup result. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonOptionalHarmonicCoefficient {
    /// Whether the coefficient exists.
    pub present: bool,
    /// Coefficient when present.
    pub coefficient: SidereonSphericalHarmonicCoefficient,
}

impl From<Option<SphericalHarmonicCoefficient>> for SidereonOptionalHarmonicCoefficient {
    fn from(value: Option<SphericalHarmonicCoefficient>) -> Self {
        Self {
            present: value.is_some(),
            coefficient: value.map(Into::into).unwrap_or_default(),
        }
    }
}

unsafe fn write_gravity_handle<T, H>(
    operation: &str,
    out: *mut *mut T,
    input: *const H,
    build: impl FnOnce() -> Result<T, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", input, "field"
        ));
        out.write(ptr::null_mut());
        let value = c_try!(build());
        out.write(Box::into_raw(Box::new(value)));
        SidereonStatus::Ok
    })
}

unsafe fn read_gravity_handle<T, U: Copy>(
    operation: &str,
    handle: *const T,
    out: *mut U,
    read: impl FnOnce(&T) -> Result<U, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", handle, "handle"
        ));
        let handle = c_try!(require_ref(handle, operation, "handle"));
        out.write(c_try!(read(handle)));
        SidereonStatus::Ok
    })
}

/// Parse an ICGEM field from UTF-8.
///
///
/// Safety: data is readable for len bytes. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_parse(
    data: *const u8,
    len: usize,
    out: *mut *mut SidereonIcgemGravityField,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_parse";
    write_gravity_handle(OP, out, ptr::null::<()>(), || {
        let text = text_bytes_from_c(OP, data, len)?;
        let inner = IcgemGravityField::parse(text).map_err(|err| map_icgem_error(OP, err))?;
        Ok(SidereonIcgemGravityField { inner })
    })
}

/// Parse an ICGEM field from UTF-8.
/// The requested truncation bounds storage; every record is validated.
///
/// Safety: data is readable for len bytes. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_parse_truncated(
    data: *const u8,
    len: usize,
    max_degree: u16,
    max_order: u16,
    out: *mut *mut SidereonIcgemGravityField,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_parse_truncated";
    write_gravity_handle(OP, out, ptr::null::<()>(), || {
        let text = text_bytes_from_c(OP, data, len)?;
        let inner = IcgemGravityField::parse_truncated(text, max_degree, max_order)
            .map_err(|err| map_icgem_error(OP, err))?;
        Ok(SidereonIcgemGravityField { inner })
    })
}

/// Release the gravity handle.
///
/// Safety: handle is NULL or live and owned.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_free(handle: *mut SidereonIcgemGravityField) {
    ffi_boundary("sidereon_icgem_gravity_field_free", (), || {
        free_boxed(handle)
    });
}

/// Release the gravity handle.
///
/// Safety: handle is NULL or live and owned.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_free(
    handle: *mut SidereonSphericalHarmonicGravity,
) {
    ffi_boundary("sidereon_spherical_harmonic_gravity_free", (), || {
        free_boxed(handle)
    });
}

/// Read the gravity field header.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_header(
    field: *const SidereonIcgemGravityField,
    out: *mut SidereonIcgemHeader,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_header";
    read_gravity_handle(OP, field, out, |field| Ok(field.inner.header().into()))
}

/// Read the maximum stored degree.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_stored_max_degree(
    field: *const SidereonIcgemGravityField,
    out: *mut u16,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_stored_max_degree";
    read_gravity_handle(OP, field, out, |field| Ok(field.inner.stored_max_degree()))
}

/// Read the maximum stored order.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_stored_max_order(
    field: *const SidereonIcgemGravityField,
    out: *mut u16,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_stored_max_order";
    read_gravity_handle(OP, field, out, |field| Ok(field.inner.stored_max_order()))
}

/// Check whether the field has time-variable coefficients.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_is_time_variable(
    field: *const SidereonIcgemGravityField,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_is_time_variable";
    read_gravity_handle(OP, field, out, |field| Ok(field.inner.is_time_variable()))
}

/// Read a static normalized harmonic coefficient.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_static_coefficient(
    field: *const SidereonIcgemGravityField,
    degree: u16,
    order: u16,
    out: *mut SidereonOptionalHarmonicCoefficient,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_static_coefficient";
    read_gravity_handle(OP, field, out, |field| {
        Ok(field.inner.static_coefficient(degree, order).into())
    })
}

/// Read static harmonic coefficient uncertainties.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_static_sigmas(
    field: *const SidereonIcgemGravityField,
    degree: u16,
    order: u16,
    out: *mut SidereonIcgemSigmas,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_static_sigmas";
    read_gravity_handle(OP, field, out, |field| {
        Ok(field
            .inner
            .static_sigmas(degree, order)
            .map(Into::into)
            .unwrap_or_default())
    })
}

/// Evaluate a normalized coefficient at the supplied TT epoch.
///
/// Safety: field is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_coefficient_at(
    field: *const SidereonIcgemGravityField,
    degree: u16,
    order: u16,
    epoch_tt_seconds: f64,
    out: *mut SidereonOptionalHarmonicCoefficient,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_coefficient_at";
    read_gravity_handle(OP, field, out, |field| {
        field
            .inner
            .coefficient_at(degree, order, epoch_tt_seconds)
            .map(Into::into)
            .map_err(|err| map_icgem_error(OP, err))
    })
}

/// Copy the full model name in UTF-8 without a terminator.
/// NULL and zero capacity query the full element count.
///
/// Safety: field is live. All outputs are writable and disjoint from
/// each other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_model_name(
    field: *const SidereonIcgemGravityField,
    out: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_model_name";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            field,
            "field"
        ));
        let field = c_try!(require_ref(field, OP, "field"));
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            field.inner.header().model_name.as_bytes(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the time-variable terms in field order.
/// NULL and zero capacity query the full element count.
///
/// Safety: field is live. All outputs are writable and disjoint from
/// each other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_time_variable_terms(
    field: *const SidereonIcgemGravityField,
    out: *mut SidereonIcgemTimeVariableTerm,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_time_variable_terms";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            field,
            "field"
        ));
        let field = c_try!(require_ref(field, OP, "field"));
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &field
                .inner
                .time_variable_terms()
                .iter()
                .copied()
                .map(Into::into)
                .collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the field with an assumed tide system.
///
/// Safety: field is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_with_assumed_tide_system(
    field: *const SidereonIcgemGravityField,
    tide_system: u32,
    out: *mut *mut SidereonIcgemGravityField,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_with_assumed_tide_system";
    write_gravity_handle(OP, out, field, || {
        let field = require_ref(field, OP, "field")?;
        let value = field
            .inner
            .clone()
            .with_assumed_tide_system(gravity_tide_system_from_c(OP, tide_system)?)
            .map_err(|err| map_icgem_error(OP, err))?;
        Ok(SidereonIcgemGravityField { inner: value })
    })
}

/// Build spherical harmonic gravity at the selected truncation.
///
/// Safety: field is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_gravity(
    field: *const SidereonIcgemGravityField,
    max_degree: u16,
    max_order: u16,
    out: *mut *mut SidereonSphericalHarmonicGravity,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_gravity";
    write_gravity_handle(OP, out, field, || {
        let field = require_ref(field, OP, "field")?;
        let value = field
            .inner
            .gravity(max_degree, max_order)
            .map_err(|err| map_icgem_error(OP, err))?;
        Ok(SidereonSphericalHarmonicGravity {
            inner: Arc::new(value),
        })
    })
}

/// Build spherical harmonic gravity at the selected truncation.
///
/// Safety: field is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_gravity_in_tide_system(
    field: *const SidereonIcgemGravityField,
    max_degree: u16,
    max_order: u16,
    tide_system: u32,
    out: *mut *mut SidereonSphericalHarmonicGravity,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_gravity_in_tide_system";
    write_gravity_handle(OP, out, field, || {
        let field = require_ref(field, OP, "field")?;
        let value = field
            .inner
            .gravity_in_tide_system(
                max_degree,
                max_order,
                gravity_tide_system_from_c(OP, tide_system)?,
            )
            .map_err(|err| map_icgem_error(OP, err))?;
        Ok(SidereonSphericalHarmonicGravity {
            inner: Arc::new(value),
        })
    })
}

/// Create embedded EGM96 at the selected maximum degree and order.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_egm96(
    max_degree: u16,
    max_order: u16,
    out: *mut *mut SidereonSphericalHarmonicGravity,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_egm96";
    write_gravity_handle(OP, out, ptr::null::<()>(), || {
        let value = SphericalHarmonicGravity::egm96_truncated(max_degree, max_order)
            .map_err(|err| map_propagation_error(OP, err))?;
        Ok(SidereonSphericalHarmonicGravity {
            inner: Arc::new(value),
        })
    })
}

/// Retain this gravity model as a general force model.
///
/// Safety: gravity is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_as_force_model(
    gravity: *const SidereonSphericalHarmonicGravity,
    out: *mut *mut SidereonForceModel,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_as_force_model";
    crate::force_models::write_guarded_force_model(
        OP,
        out,
        || reject_output_overlaps_handle(OP, out, "out", gravity, "gravity"),
        || Ok(require_ref(gravity, OP, "gravity")?.inner.clone()),
    )
}

/// Check whether a spherical harmonic force has time-dependent coefficients.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_is_time_variable(
    gravity: *const SidereonSphericalHarmonicGravity,
    out: *mut bool,
) -> SidereonStatus {
    read_gravity_handle(
        "sidereon_spherical_harmonic_gravity_is_time_variable",
        gravity,
        out,
        |gravity| Ok(gravity.inner.is_time_variable()),
    )
}

/// Evaluate geopotential acceleration in km/s² at a body-fixed position.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_body_fixed_acceleration_km_s2(
    gravity: *const SidereonSphericalHarmonicGravity,
    position_km: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_body_fixed_acceleration_km_s2";
    read_gravity_handle(OP, gravity, out, |gravity| {
        gravity
            .inner
            .body_fixed_acceleration_km_s2(position_km.xyz)
            .map(|xyz| SidereonVector3 { xyz })
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate geopotential acceleration in km/s² at a body-fixed position.
/// The epoch is TT seconds past J2000.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_body_fixed_acceleration_at_km_s2(
    gravity: *const SidereonSphericalHarmonicGravity,
    position_km: SidereonVector3,
    epoch_tt_seconds: f64,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_body_fixed_acceleration_at_km_s2";
    read_gravity_handle(OP, gravity, out, |gravity| {
        gravity
            .inner
            .body_fixed_acceleration_at_km_s2(position_km.xyz, epoch_tt_seconds)
            .map(|xyz| SidereonVector3 { xyz })
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate geopotential in km²/s² at a body-fixed position.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_body_fixed_potential_km2_s2(
    gravity: *const SidereonSphericalHarmonicGravity,
    position_km: SidereonVector3,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_body_fixed_potential_km2_s2";
    read_gravity_handle(OP, gravity, out, |gravity| {
        gravity
            .inner
            .body_fixed_potential_km2_s2(position_km.xyz)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate geopotential in km²/s² at a body-fixed position.
/// The epoch is TT seconds past J2000.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_body_fixed_potential_at_km2_s2(
    gravity: *const SidereonSphericalHarmonicGravity,
    position_km: SidereonVector3,
    epoch_tt_seconds: f64,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_body_fixed_potential_at_km2_s2";
    read_gravity_handle(OP, gravity, out, |gravity| {
        gravity
            .inner
            .body_fixed_potential_at_km2_s2(position_km.xyz, epoch_tt_seconds)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Evaluate a harmonic coefficient at an absolute TT epoch.
///
/// Safety: gravity is live and out is writable and disjoint from the handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_coefficient_at(
    gravity: *const SidereonSphericalHarmonicGravity,
    degree: u16,
    order: u16,
    epoch_tt_seconds: f64,
    out: *mut SidereonOptionalHarmonicCoefficient,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_coefficient_at";
    read_gravity_handle(OP, gravity, out, |gravity| {
        gravity
            .inner
            .coefficient_at(degree, order, epoch_tt_seconds)
            .map(Into::into)
            .map_err(|err| map_propagation_error(OP, err))
    })
}

/// Compare two gravity handles by value.
///
/// Safety: both handles are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_icgem_gravity_field_equal(
    left: *const SidereonIcgemGravityField,
    right: *const SidereonIcgemGravityField,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_icgem_gravity_field_equal";
    read_gravity_handle(OP, left, out, |left| {
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        Ok(left.inner == require_ref(right, OP, "right")?.inner)
    })
}

/// Compare two gravity handles by value.
///
/// Safety: both handles are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spherical_harmonic_gravity_equal(
    left: *const SidereonSphericalHarmonicGravity,
    right: *const SidereonSphericalHarmonicGravity,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_spherical_harmonic_gravity_equal";
    read_gravity_handle(OP, left, out, |left| {
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        Ok(left.inner == require_ref(right, OP, "right")?.inner)
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn icgem_error_details() {
        use IcgemError as E;
        let cases = [
            (
                E::MissingField {
                    line: 3,
                    keyword: String::from("sample"),
                    field: "sample",
                },
                "missing_field",
                json!({"line":3, "keyword":"sample", "field":"sample"}),
            ),
            (
                E::InvalidField {
                    line: 3,
                    keyword: String::from("sample"),
                    field: "sample",
                    value: String::from("sample"),
                },
                "invalid_field",
                json!({"line":3, "keyword":"sample", "field":"sample", "value":"sample"}),
            ),
            (
                E::MissingHeaderKeyword { keyword: "sample" },
                "missing_header_keyword",
                json!({"keyword":"sample"}),
            ),
            (
                E::RepeatedHeaderKeyword {
                    line: 3,
                    first_line: 3,
                    keyword: "sample",
                },
                "repeated_header_keyword",
                json!({"line":3, "first_line":3, "keyword":"sample"}),
            ),
            (E::MissingEndOfHead, "missing_end_of_head", json!({})),
            (
                E::KeywordNotInFormat {
                    line: 3,
                    keyword: String::from("sample"),
                },
                "keyword_not_in_format",
                json!({"line":3, "keyword":"sample"}),
            ),
            (
                E::UnknownRecordKeyword {
                    line: 3,
                    keyword: String::from("sample"),
                },
                "unknown_record_keyword",
                json!({"line":3, "keyword":"sample"}),
            ),
            (
                E::MissingRecordKeyword { line: 3 },
                "missing_record_keyword",
                json!({"line":3}),
            ),
            (
                E::DegreeAboveMaximum {
                    line: 3,
                    degree: 3,
                    max_degree: 3,
                },
                "degree_above_maximum",
                json!({"line":3, "degree":3, "max_degree":3}),
            ),
            (
                E::OrderAboveDegree {
                    line: 3,
                    degree: 3,
                    order: 3,
                },
                "order_above_degree",
                json!({"line":3, "degree":3, "order":3}),
            ),
            (
                E::RepeatedCoefficient {
                    line: 3,
                    first_line: 3,
                    degree: 3,
                    order: 3,
                },
                "repeated_coefficient",
                json!({"line":3, "first_line":3, "degree":3, "order":3}),
            ),
            (
                E::OverlappingTerm {
                    line: 3,
                    first_line: 3,
                    keyword: String::from("sample"),
                    degree: 3,
                    order: 3,
                },
                "overlapping_term",
                json!({"line":3, "first_line":3, "keyword":"sample", "degree":3, "order":3}),
            ),
            (
                E::UncoveredTerm {
                    line: 3,
                    keyword: String::from("sample"),
                    degree: 3,
                    order: 3,
                },
                "uncovered_term",
                json!({"line":3, "keyword":"sample", "degree":3, "order":3}),
            ),
            (
                E::MissingCoefficient {
                    degree: 3,
                    order: 3,
                },
                "missing_coefficient",
                json!({"degree":3, "order":3}),
            ),
            (
                E::TooFewRecords {
                    max_degree: 3,
                    required: 3,
                    lines: 3,
                },
                "too_few_records",
                json!({"max_degree":3, "required":3, "lines":3}),
            ),
            (
                E::NormalizationOutOfRange {
                    line: 3,
                    degree: 3,
                    order: 3,
                },
                "normalization_out_of_range",
                json!({"line":3, "degree":3, "order":3}),
            ),
            (
                E::TruncationOutOfRange {
                    degree: 3,
                    order: 3,
                    max_degree: 3,
                },
                "truncation_out_of_range",
                json!({"degree":3, "order":3, "max_degree":3}),
            ),
            (E::UnknownTideSystem, "unknown_tide_system", json!({})),
            (
                E::TideSystemConflict {
                    stated: CoreTideSystem::ZeroTide,
                    assumed: CoreTideSystem::MeanTide,
                },
                "tide_system_conflict",
                json!({"stated":1, "assumed":2}),
            ),
            (
                E::EpochOutsideValidity {
                    degree: 3,
                    order: 3,
                    epoch_tt_seconds: 0.5,
                },
                "epoch_outside_validity",
                json!({"degree":3, "order":3, "epoch_tt_seconds":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (E::NonFiniteEpoch, "non_finite_epoch", json!({})),
            (
                E::NonFiniteCoefficient {
                    degree: 3,
                    order: 3,
                    epoch_tt_seconds: 0.5,
                },
                "non_finite_coefficient",
                json!({"degree":3, "order":3, "epoch_tt_seconds":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(
                icgem_error_value(&error),
                json!({"kind":kind,"fields":fields})
            );
        }
    }
}
