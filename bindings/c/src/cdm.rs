use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::cdm::{CdmError, CdmInputErrorKind, TextIssue};

// --- CDM conjunction data message (sidereon_core::astro::cdm) -----------------

/// A parsed Conjunction Data Message. Opaque to C. Create with
/// sidereon_cdm_parse_kvn or sidereon_cdm_parse_xml; release with
/// sidereon_cdm_free.
pub struct SidereonCdm {
    pub(crate) inner: sidereon_core::astro::cdm::CdmKvn,
}

/// Selects which CDM string field a reader returns.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCdmStringField {
    /// Message creation date.
    CreationDate = 0,
    /// Originator.
    Originator = 1,
    /// Message id.
    MessageId = 2,
    /// Time of closest approach.
    Tca = 3,
    /// Collision-probability method label.
    CollisionProbabilityMethod = 4,
    /// Object 1 designator.
    Object1Designator = 5,
    /// Object 1 name.
    Object1Name = 6,
    /// Object 2 designator.
    Object2Designator = 7,
    /// Object 2 name.
    Object2Name = 8,
}

/// Parse a CDM from KVN text. On success writes a newly owned handle to
/// *out_cdm. Delegates to sidereon_core::astro::cdm::parse_kvn.
///
/// Safety: text points to len readable bytes; out_cdm points to a SidereonCdm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_parse_kvn(
    text: *const u8,
    len: usize,
    out_cdm: *mut *mut SidereonCdm,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_cdm_parse_kvn", SidereonStatus::Panic, || {
        cdm_parse(
            "sidereon_cdm_parse_kvn",
            text,
            len,
            out_cdm,
            sidereon_core::astro::cdm::parse_kvn,
        )
    })
}

/// Parse a CDM from XML text. On success writes a newly owned handle to
/// *out_cdm. Delegates to sidereon_core::astro::cdm::parse_xml.
///
/// Safety: text points to len readable bytes; out_cdm points to a SidereonCdm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_parse_xml(
    text: *const u8,
    len: usize,
    out_cdm: *mut *mut SidereonCdm,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_cdm_parse_xml", SidereonStatus::Panic, || {
        cdm_parse(
            "sidereon_cdm_parse_xml",
            text,
            len,
            out_cdm,
            sidereon_core::astro::cdm::parse_xml,
        )
    })
}

/// Release a CDM handle. Passing NULL is a no-op.
///
/// Safety: cdm must be a handle from a sidereon_cdm_parse_* call or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_free(cdm: *mut SidereonCdm) {
    free_boxed(cdm);
}

/// Serialize a CDM to KVN text (not null-terminated). Variable-length output
/// contract. Delegates to sidereon_core::astro::cdm::encode_kvn.
///
/// Safety: cdm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_to_kvn(
    cdm: *const SidereonCdm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_cdm_to_kvn", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_cdm_to_kvn",
            out_written,
            out_required
        ));
        let cdm = c_try!(require_ref(cdm, "sidereon_cdm_to_kvn", "cdm"));
        let text = match sidereon_core::astro::cdm::encode_kvn(&cdm.inner) {
            Ok(t) => t,
            Err(err) => return map_cdm_error("sidereon_cdm_to_kvn", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_cdm_to_kvn",
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Serialize a CDM to XML text (not null-terminated). Variable-length output
/// contract. Delegates to sidereon_core::astro::cdm::encode_xml.
///
/// Safety: cdm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_to_xml(
    cdm: *const SidereonCdm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_cdm_to_xml", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_cdm_to_xml",
            out_written,
            out_required
        ));
        let cdm = c_try!(require_ref(cdm, "sidereon_cdm_to_xml", "cdm"));
        let text = match sidereon_core::astro::cdm::encode_xml(&cdm.inner) {
            Ok(t) => t,
            Err(err) => return map_cdm_error("sidereon_cdm_to_xml", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_cdm_to_xml",
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Read a CDM string field selected by SidereonCdmStringField into a caller
/// buffer (not null-terminated). An absent optional field reports *out_required 0
/// and writes nothing. Variable-length output contract.
///
/// Safety: cdm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_string_field(
    cdm: *const SidereonCdm,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_cdm_string_field", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_cdm_string_field",
            out_written,
            out_required
        ));
        let cdm = c_try!(require_ref(cdm, "sidereon_cdm_string_field", "cdm"));
        let c = &cdm.inner;
        let value: Option<&str> = match field {
            v if v == SidereonCdmStringField::CreationDate as u32 => c.creation_date.as_deref(),
            v if v == SidereonCdmStringField::Originator as u32 => c.originator.as_deref(),
            v if v == SidereonCdmStringField::MessageId as u32 => c.message_id.as_deref(),
            v if v == SidereonCdmStringField::Tca as u32 => c.tca.as_deref(),
            v if v == SidereonCdmStringField::CollisionProbabilityMethod as u32 => {
                c.collision_probability_method.as_deref()
            }
            v if v == SidereonCdmStringField::Object1Designator as u32 => {
                c.object1.object_designator.as_deref()
            }
            v if v == SidereonCdmStringField::Object1Name as u32 => {
                c.object1.object_name.as_deref()
            }
            v if v == SidereonCdmStringField::Object2Designator as u32 => {
                c.object2.object_designator.as_deref()
            }
            v if v == SidereonCdmStringField::Object2Name as u32 => {
                c.object2.object_name.as_deref()
            }
            _ => {
                set_last_error("sidereon_cdm_string_field: invalid field code".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        let bytes = value.unwrap_or("").as_bytes();
        c_try!(copy_prefix_to_c(
            "sidereon_cdm_string_field",
            "out",
            bytes,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Read the four optional numeric CDM scalars. Each out pointer receives the
/// value, or NaN when the field is absent in the message. Any out pointer may be
/// NULL to skip that field.
///
/// Safety: cdm is a live handle; each non-null out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_numbers(
    cdm: *const SidereonCdm,
    out_miss_distance_m: *mut f64,
    out_relative_speed_m_s: *mut f64,
    out_collision_probability: *mut f64,
    out_hard_body_radius_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_cdm_numbers", SidereonStatus::Panic, || {
        let cdm = c_try!(require_ref(cdm, "sidereon_cdm_numbers", "cdm"));
        let c = &cdm.inner;
        if !out_miss_distance_m.is_null() {
            out_miss_distance_m.write(c.miss_distance_m.unwrap_or(f64::NAN));
        }
        if !out_relative_speed_m_s.is_null() {
            out_relative_speed_m_s.write(c.relative_speed_m_s.unwrap_or(f64::NAN));
        }
        if !out_collision_probability.is_null() {
            out_collision_probability.write(c.collision_probability.unwrap_or(f64::NAN));
        }
        if !out_hard_body_radius_m.is_null() {
            out_hard_body_radius_m.write(c.hard_body_radius_m.unwrap_or(f64::NAN));
        }
        SidereonStatus::Ok
    })
}

/// Read one CDM object's state vector (position xyz, velocity xyz) and RTN
/// covariance lower triangle (CR_R, CT_R, CT_T, CN_R, CN_T, CN_N). object_index
/// must be 1 or 2. Any out pointer may be NULL to skip.
///
/// Safety: cdm is a live handle; out_position and out_velocity point to 3 doubles
/// when non-null; out_covariance_rtn points to 6 doubles when non-null.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_object_state(
    cdm: *const SidereonCdm,
    object_index: u32,
    out_position: *mut f64,
    out_velocity: *mut f64,
    out_covariance_rtn: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_cdm_object_state", SidereonStatus::Panic, || {
        let cdm = c_try!(require_ref(cdm, "sidereon_cdm_object_state", "cdm"));
        let obj = match object_index {
            1 => &cdm.inner.object1,
            2 => &cdm.inner.object2,
            _ => {
                set_last_error(
                    "sidereon_cdm_object_state: object_index must be 1 or 2".to_string(),
                );
                return SidereonStatus::InvalidArgument;
            }
        };
        let ((px, py, pz), (vx, vy, vz)) = obj.state;
        if !out_position.is_null() {
            c_try!(copy_exact_f64s(
                "sidereon_cdm_object_state",
                "out_position",
                out_position,
                3,
                &[px, py, pz]
            ));
        }
        if !out_velocity.is_null() {
            c_try!(copy_exact_f64s(
                "sidereon_cdm_object_state",
                "out_velocity",
                out_velocity,
                3,
                &[vx, vy, vz]
            ));
        }
        if !out_covariance_rtn.is_null() {
            c_try!(copy_exact_f64s(
                "sidereon_cdm_object_state",
                "out_covariance_rtn",
                out_covariance_rtn,
                6,
                &obj.covariance_rtn
            ));
        }
        SidereonStatus::Ok
    })
}

// --- CDM comprehensive metadata + velocity covariance -----------------------

/// Selects which per-object CDM metadata string field a reader returns. Pass to
/// sidereon_cdm_object_string_field as a uint32_t. Mirrors the CCSDS 508.0-B-1
/// object metadata block order.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCdmObjectStringField {
    /// OBJECT_DESIGNATOR.
    ObjectDesignator = 0,
    /// CATALOG_NAME.
    CatalogName = 1,
    /// OBJECT_NAME.
    ObjectName = 2,
    /// INTERNATIONAL_DESIGNATOR.
    InternationalDesignator = 3,
    /// OBJECT_TYPE.
    ObjectType = 4,
    /// OPERATOR_CONTACT_POSITION.
    OperatorContactPosition = 5,
    /// OPERATOR_ORGANIZATION.
    OperatorOrganization = 6,
    /// OPERATOR_PHONE.
    OperatorPhone = 7,
    /// OPERATOR_EMAIL.
    OperatorEmail = 8,
    /// EPHEMERIS_NAME.
    EphemerisName = 9,
    /// COVARIANCE_METHOD.
    CovarianceMethod = 10,
    /// MANEUVERABLE.
    Maneuverable = 11,
    /// ORBIT_CENTER.
    OrbitCenter = 12,
    /// REF_FRAME.
    RefFrame = 13,
    /// GRAVITY_MODEL.
    GravityModel = 14,
    /// ATMOSPHERIC_MODEL.
    AtmosphericModel = 15,
    /// N_BODY_PERTURBATIONS.
    NBodyPerturbations = 16,
    /// SOLAR_RAD_PRESSURE.
    SolarRadPressure = 17,
    /// EARTH_TIDES.
    EarthTides = 18,
    /// INTRACK_THRUST.
    IntrackThrust = 19,
}

/// Read one CDM object's metadata string field selected by
/// SidereonCdmObjectStringField into a caller buffer (not null-terminated).
/// object_index must be 1 or 2. An absent optional field reports *out_required 0
/// and writes nothing. Variable-length output contract.
///
/// Safety: cdm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_object_string_field(
    cdm: *const SidereonCdm,
    object_index: u32,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cdm_object_string_field",
        SidereonStatus::Panic,
        || {
            let fn_name = "sidereon_cdm_object_string_field";
            c_try!(init_copy_counts(fn_name, out_written, out_required));
            let cdm = c_try!(require_ref(cdm, fn_name, "cdm"));
            let obj = match object_index {
                1 => &cdm.inner.object1,
                2 => &cdm.inner.object2,
                _ => {
                    set_last_error(format!("{fn_name}: object_index must be 1 or 2"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let value: Option<&str> = match field {
                v if v == SidereonCdmObjectStringField::ObjectDesignator as u32 => {
                    obj.object_designator.as_deref()
                }
                v if v == SidereonCdmObjectStringField::CatalogName as u32 => {
                    obj.catalog_name.as_deref()
                }
                v if v == SidereonCdmObjectStringField::ObjectName as u32 => {
                    obj.object_name.as_deref()
                }
                v if v == SidereonCdmObjectStringField::InternationalDesignator as u32 => {
                    obj.international_designator.as_deref()
                }
                v if v == SidereonCdmObjectStringField::ObjectType as u32 => {
                    obj.object_type.as_deref()
                }
                v if v == SidereonCdmObjectStringField::OperatorContactPosition as u32 => {
                    obj.operator_contact_position.as_deref()
                }
                v if v == SidereonCdmObjectStringField::OperatorOrganization as u32 => {
                    obj.operator_organization.as_deref()
                }
                v if v == SidereonCdmObjectStringField::OperatorPhone as u32 => {
                    obj.operator_phone.as_deref()
                }
                v if v == SidereonCdmObjectStringField::OperatorEmail as u32 => {
                    obj.operator_email.as_deref()
                }
                v if v == SidereonCdmObjectStringField::EphemerisName as u32 => {
                    obj.ephemeris_name.as_deref()
                }
                v if v == SidereonCdmObjectStringField::CovarianceMethod as u32 => {
                    obj.covariance_method.as_deref()
                }
                v if v == SidereonCdmObjectStringField::Maneuverable as u32 => {
                    obj.maneuverable.as_deref()
                }
                v if v == SidereonCdmObjectStringField::OrbitCenter as u32 => {
                    obj.orbit_center.as_deref()
                }
                v if v == SidereonCdmObjectStringField::RefFrame as u32 => obj.ref_frame.as_deref(),
                v if v == SidereonCdmObjectStringField::GravityModel as u32 => {
                    obj.gravity_model.as_deref()
                }
                v if v == SidereonCdmObjectStringField::AtmosphericModel as u32 => {
                    obj.atmospheric_model.as_deref()
                }
                v if v == SidereonCdmObjectStringField::NBodyPerturbations as u32 => {
                    obj.n_body_perturbations.as_deref()
                }
                v if v == SidereonCdmObjectStringField::SolarRadPressure as u32 => {
                    obj.solar_rad_pressure.as_deref()
                }
                v if v == SidereonCdmObjectStringField::EarthTides as u32 => {
                    obj.earth_tides.as_deref()
                }
                v if v == SidereonCdmObjectStringField::IntrackThrust as u32 => {
                    obj.intrack_thrust.as_deref()
                }
                _ => {
                    set_last_error(format!("{fn_name}: invalid field code"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let bytes = value.unwrap_or("").as_bytes();
            c_try!(copy_prefix_to_c(
                fn_name,
                "out",
                bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one CDM object's RTN velocity-covariance block (the 15 lower-triangle
/// elements completing the 6x6 matrix) into out_covariance and set *out_present
/// to whether the producer carried the full velocity block. object_index must be
/// 1 or 2. When absent, out_covariance is zeroed and *out_present is false.
///
/// Safety: cdm is a live handle; out_covariance points to 15 writable doubles;
/// out_present points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cdm_object_velocity_covariance(
    cdm: *const SidereonCdm,
    object_index: u32,
    out_covariance: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cdm_object_velocity_covariance",
        SidereonStatus::Panic,
        || {
            let fn_name = "sidereon_cdm_object_velocity_covariance";
            let out_present = c_try!(require_out(out_present, fn_name, "out_present"));
            *out_present = false;
            let cdm = c_try!(require_ref(cdm, fn_name, "cdm"));
            let obj = match object_index {
                1 => &cdm.inner.object1,
                2 => &cdm.inner.object2,
                _ => {
                    set_last_error(format!("{fn_name}: object_index must be 1 or 2"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let values = obj.velocity_covariance_rtn.unwrap_or([0.0; 15]);
            c_try!(copy_exact_f64s(
                fn_name,
                "out_covariance",
                out_covariance,
                15,
                &values,
            ));
            *out_present = obj.velocity_covariance_rtn.is_some();
            SidereonStatus::Ok
        },
    )
}

// ===========================================================================
// Newer core additions. Each entry marshals the caller's flat C inputs into the
// merged-core types and delegates: the numbers are exactly what the core
// produces. Grouped by capability:
//   - generic data-driven trust-region least squares (solve + leave-one-out)
//   - Jacobian-derived covariance / Hessian trace / 2x2 error ellipse
//   - DOP with an explicit ENU convention
//   - residual-distribution statistics (moments + normality tests)
//   - batch forward-observable prediction
//   - leap-second accessors (GPS-UTC, TAI-UTC)
//   - embedded EGM96 geoid undulation and height conversions
//   - ground-observer Sun/Moon geometry, illumination, rise/set, transits

unsafe fn cdm_parse(
    fn_name: &str,
    text: *const u8,
    len: usize,
    out_cdm: *mut *mut SidereonCdm,
    parse: impl FnOnce(
        &str,
    )
        -> Result<sidereon_core::astro::cdm::CdmKvn, sidereon_core::astro::cdm::CdmError>,
) -> SidereonStatus {
    let out_cdm = match require_out(out_cdm, fn_name, "out_cdm") {
        Ok(out) => out,
        Err(status) => return status,
    };
    *out_cdm = ptr::null_mut();
    let bytes = match require_slice(text, len, fn_name, "text") {
        Ok(b) => b,
        Err(status) => return status,
    };
    let text = match str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            set_last_error(format!("{fn_name}: text is not valid UTF-8"));
            return SidereonStatus::InvalidToken;
        }
    };
    match parse(text) {
        Ok(inner) => {
            write_boxed_handle(out_cdm, SidereonCdm { inner });
            SidereonStatus::Ok
        }
        Err(err) => map_cdm_error(fn_name, err),
    }
}

fn cdm_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

fn cdm_input_error_kind_name(kind: CdmInputErrorKind) -> &'static str {
    match kind {
        CdmInputErrorKind::Missing => "missing",
        CdmInputErrorKind::NonFinite => "non_finite",
        CdmInputErrorKind::FloatParse => "float_parse",
        CdmInputErrorKind::IntParse => "int_parse",
        CdmInputErrorKind::NotPositive => "not_positive",
        CdmInputErrorKind::Negative => "negative",
        CdmInputErrorKind::OutOfRange => "out_of_range",
        CdmInputErrorKind::InvalidCivilDate => "invalid_civil_date",
        CdmInputErrorKind::InvalidCivilTime => "invalid_civil_time",
    }
}

fn text_issue_name(issue: TextIssue) -> &'static str {
    match issue {
        TextIssue::LineBreak => "line_break",
        TextIssue::SurroundingWhitespace => "surrounding_whitespace",
        TextIssue::InteriorWhitespace => "interior_whitespace",
        TextIssue::KeywordSeparator => "keyword_separator",
        TextIssue::XmlIllegalCharacter => "xml_illegal_character",
        TextIssue::Empty => "empty",
        TextIssue::DetachedComment => "detached_comment",
        TextIssue::RepeatedParameter => "repeated_parameter",
        TextIssue::CommentNotCarried => "comment_not_carried",
    }
}

/// Convert a `CdmError` into a structured JSON error node.
pub(crate) fn cdm_error_value(error: &CdmError) -> serde_json::Value {
    match error {
        CdmError::IncompleteStateVector => cdm_node("incomplete_state_vector", json!({})),
        CdmError::InvalidField { field, kind } => cdm_node(
            "invalid_field",
            json!({
                "field": field,
                "kind": cdm_input_error_kind_name(*kind),
            }),
        ),
        CdmError::MalformedXml(message) => cdm_node(
            "malformed_xml",
            json!({
                "message": message,
            }),
        ),
        CdmError::DuplicateField {
            field,
            first,
            second,
        } => cdm_node(
            "duplicate_field",
            json!({
                "field": field,
                "first": first,
                "second": second,
            }),
        ),
        CdmError::UnitMismatch {
            field,
            unit,
            expected,
        } => cdm_node(
            "unit_mismatch",
            json!({
                "field": field,
                "unit": unit,
                "expected": expected,
            }),
        ),
        CdmError::UnexpectedObjectCount(count) => cdm_node(
            "unexpected_object_count",
            json!({
                "count": count,
            }),
        ),
        CdmError::MultipleMessages { count } => cdm_node(
            "multiple_messages",
            json!({
                "count": count,
            }),
        ),
        CdmError::UnknownField(field) => cdm_node(
            "unknown_field",
            json!({
                "field": field,
            }),
        ),
        CdmError::MalformedLine { line, text } => cdm_node(
            "malformed_line",
            json!({
                "line": line,
                "text": text,
            }),
        ),
        CdmError::UnknownObject(object) => cdm_node(
            "unknown_object",
            json!({
                "object": object,
            }),
        ),
        CdmError::RepeatedObject(object) => cdm_node(
            "repeated_object",
            json!({
                "object": object,
            }),
        ),
        CdmError::UnwritableText {
            field,
            value,
            issue,
        } => cdm_node(
            "unwritable_text",
            json!({
                "field": field,
                "value": value,
                "issue": text_issue_name(*issue),
            }),
        ),
        CdmError::HardBodyRadiusComment { comment } => cdm_node(
            "hard_body_radius_comment",
            json!({
                "comment": comment,
            }),
        ),
    }
}

fn map_cdm_error(fn_name: &str, err: CdmError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Cdm,
        fn_name,
        cdm_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::{json, Value};
    use std::ptr;

    const CDM_KVN_FIXTURE: &str = include_str!("../tests/fixtures/cdm/ccsds_example2.kvn");
    const CDM_XML_FIXTURE: &str = include_str!("../tests/fixtures/cdm/ccsds_example2.xml");

    #[test]
    fn table_driven_cdm_error_mapping() {
        let cases: Vec<(CdmError, &'static str)> = vec![
            (CdmError::IncompleteStateVector, "incomplete_state_vector"),
            (
                CdmError::InvalidField {
                    field: "MISS_DISTANCE",
                    kind: CdmInputErrorKind::NotPositive,
                },
                "invalid_field",
            ),
            (
                CdmError::MalformedXml("syntax error".into()),
                "malformed_xml",
            ),
            (
                CdmError::DuplicateField {
                    field: "TCA".into(),
                    first: "2026-01-01T00:00:00".into(),
                    second: "2026-01-02T00:00:00".into(),
                },
                "duplicate_field",
            ),
            (
                CdmError::UnitMismatch {
                    field: "MISS_DISTANCE".into(),
                    unit: "km".into(),
                    expected: Some("m"),
                },
                "unit_mismatch",
            ),
            (
                CdmError::UnitMismatch {
                    field: "COLLISION_PROBABILITY".into(),
                    unit: "n/a".into(),
                    expected: None,
                },
                "unit_mismatch",
            ),
            (
                CdmError::UnexpectedObjectCount(4),
                "unexpected_object_count",
            ),
            (CdmError::MultipleMessages { count: 3 }, "multiple_messages"),
            (
                CdmError::UnknownField("UNKNOWN_TAG".into()),
                "unknown_field",
            ),
            (
                CdmError::MalformedLine {
                    line: 15,
                    text: "NOT_AN_ASSIGNMENT".into(),
                },
                "malformed_line",
            ),
            (CdmError::UnknownObject("OBJECT3".into()), "unknown_object"),
            (
                CdmError::RepeatedObject("OBJECT1".into()),
                "repeated_object",
            ),
            (
                CdmError::UnwritableText {
                    field: "COMMENT".into(),
                    value: "line\nbreak".into(),
                    issue: TextIssue::LineBreak,
                },
                "unwritable_text",
            ),
            (
                CdmError::HardBodyRadiusComment {
                    comment: "HBR = 5.0".into(),
                },
                "hard_body_radius_comment",
            ),
        ];

        for (err, expected_kind) in cases {
            let val = cdm_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            match &err {
                CdmError::IncompleteStateVector => {
                    assert_eq!(val["fields"], json!({}));
                }
                CdmError::InvalidField { field, kind } => {
                    assert_eq!(val["fields"]["field"], *field);
                    assert_eq!(val["fields"]["kind"], cdm_input_error_kind_name(*kind));
                }
                CdmError::MalformedXml(msg) => {
                    assert_eq!(val["fields"]["message"], msg.as_str());
                }
                CdmError::DuplicateField {
                    field,
                    first,
                    second,
                } => {
                    assert_eq!(val["fields"]["field"], field.as_str());
                    assert_eq!(val["fields"]["first"], first.as_str());
                    assert_eq!(val["fields"]["second"], second.as_str());
                }
                CdmError::UnitMismatch {
                    field,
                    unit,
                    expected,
                } => {
                    assert_eq!(val["fields"]["field"], field.as_str());
                    assert_eq!(val["fields"]["unit"], unit.as_str());
                    assert_eq!(val["fields"]["expected"], json!(expected));
                }
                CdmError::UnexpectedObjectCount(count) => {
                    assert_eq!(val["fields"]["count"], *count);
                }
                CdmError::MultipleMessages { count } => {
                    assert_eq!(val["fields"]["count"], *count);
                }
                CdmError::UnknownField(f) => {
                    assert_eq!(val["fields"]["field"], f.as_str());
                }
                CdmError::MalformedLine { line, text } => {
                    assert_eq!(val["fields"]["line"], *line);
                    assert_eq!(val["fields"]["text"], text.as_str());
                }
                CdmError::UnknownObject(obj) => {
                    assert_eq!(val["fields"]["object"], obj.as_str());
                }
                CdmError::RepeatedObject(obj) => {
                    assert_eq!(val["fields"]["object"], obj.as_str());
                }
                CdmError::UnwritableText {
                    field,
                    value,
                    issue,
                } => {
                    assert_eq!(val["fields"]["field"], field.as_str());
                    assert_eq!(val["fields"]["value"], value.as_str());
                    assert_eq!(val["fields"]["issue"], text_issue_name(*issue));
                }
                CdmError::HardBodyRadiusComment { comment } => {
                    assert_eq!(val["fields"]["comment"], comment.as_str());
                }
            }
        }

        // Exhaustively verify all CdmInputErrorKind variants
        let input_kinds = [
            (CdmInputErrorKind::Missing, "missing"),
            (CdmInputErrorKind::NonFinite, "non_finite"),
            (CdmInputErrorKind::FloatParse, "float_parse"),
            (CdmInputErrorKind::IntParse, "int_parse"),
            (CdmInputErrorKind::NotPositive, "not_positive"),
            (CdmInputErrorKind::Negative, "negative"),
            (CdmInputErrorKind::OutOfRange, "out_of_range"),
            (CdmInputErrorKind::InvalidCivilDate, "invalid_civil_date"),
            (CdmInputErrorKind::InvalidCivilTime, "invalid_civil_time"),
        ];
        for (kind, name) in input_kinds {
            assert_eq!(cdm_input_error_kind_name(kind), name);
            let val = cdm_error_value(&CdmError::InvalidField {
                field: "TEST",
                kind,
            });
            assert_eq!(val["fields"]["kind"], name);
        }

        // Exhaustively verify all TextIssue variants
        let text_issues = [
            (TextIssue::LineBreak, "line_break"),
            (TextIssue::SurroundingWhitespace, "surrounding_whitespace"),
            (TextIssue::InteriorWhitespace, "interior_whitespace"),
            (TextIssue::KeywordSeparator, "keyword_separator"),
            (TextIssue::XmlIllegalCharacter, "xml_illegal_character"),
            (TextIssue::Empty, "empty"),
            (TextIssue::DetachedComment, "detached_comment"),
            (TextIssue::RepeatedParameter, "repeated_parameter"),
            (TextIssue::CommentNotCarried, "comment_not_carried"),
        ];
        for (issue, name) in text_issues {
            assert_eq!(text_issue_name(issue), name);
            let val = cdm_error_value(&CdmError::UnwritableText {
                field: "PARAM".into(),
                value: "VAL".into(),
                issue,
            });
            assert_eq!(val["fields"]["issue"], name);
        }
    }

    #[test]
    fn cdm_public_producer_control_and_refusals() {
        clear_engine_error();

        // Valid control: parse KVN, serialize to KVN, free
        unsafe {
            let mut cdm: *mut SidereonCdm = ptr::null_mut();
            assert_eq!(
                sidereon_cdm_parse_kvn(CDM_KVN_FIXTURE.as_ptr(), CDM_KVN_FIXTURE.len(), &mut cdm,),
                SidereonStatus::Ok
            );
            assert!(!cdm.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Cdm,
                payload_len: 99,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Serialize to KVN
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_cdm_to_kvn(cdm, ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert!(required > 0);
            let mut out_buf = vec![0u8; required];
            assert_eq!(
                sidereon_cdm_to_kvn(
                    cdm,
                    out_buf.as_mut_ptr(),
                    out_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, required);

            sidereon_cdm_free(cdm);
        }

        // Valid control XML: parse XML, serialize to XML, free
        unsafe {
            let mut cdm: *mut SidereonCdm = ptr::null_mut();
            assert_eq!(
                sidereon_cdm_parse_xml(CDM_XML_FIXTURE.as_ptr(), CDM_XML_FIXTURE.len(), &mut cdm,),
                SidereonStatus::Ok
            );
            assert!(!cdm.is_null());

            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_cdm_to_xml(cdm, ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert!(required > 0);

            sidereon_cdm_free(cdm);
        }

        // Real public refusal 1: malformed KVN in sidereon_cdm_parse_kvn
        unsafe {
            let mut cdm: *mut SidereonCdm = ptr::null_mut();
            let bad_kvn = b"CCSDS_CDM_VERS = 1.0\nNOT_A_VALID_KVN_LINE\n";
            assert_eq!(
                sidereon_cdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), &mut cdm),
                SidereonStatus::InvalidArgument
            );
            assert!(cdm.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Cdm);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "cdm");
            assert_eq!(payload["operation"], "sidereon_cdm_parse_kvn");
            assert_eq!(payload["error"]["kind"], "malformed_line");
            assert_eq!(payload["error"]["fields"]["line"], 2);
        }

        // Real public refusal 2: malformed XML in sidereon_cdm_parse_xml
        unsafe {
            let mut cdm: *mut SidereonCdm = ptr::null_mut();
            let bad_xml = b"<cdm><unclosed>";
            assert_eq!(
                sidereon_cdm_parse_xml(bad_xml.as_ptr(), bad_xml.len(), &mut cdm),
                SidereonStatus::InvalidArgument
            );
            assert!(cdm.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Cdm);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "cdm");
            assert_eq!(payload["operation"], "sidereon_cdm_parse_xml");
            assert_eq!(payload["error"]["kind"], "malformed_xml");
        }
    }

    #[test]
    fn cdm_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // Seed engine error via a real refusal
            let mut cdm: *mut SidereonCdm = ptr::null_mut();
            let bad_kvn = b"CCSDS_CDM_VERS = 1.0\nNOT_A_VALID_LINE\n";
            assert_eq!(
                sidereon_cdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), &mut cdm),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Cdm);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Free retains the error
            sidereon_cdm_free(ptr::null_mut());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Cdm);
            assert_eq!(info.payload_len, expected_len);

            // Two-pass payload retrieval: Pass 1 query length
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument and retains
            let mut short_buf = vec![0u8; expected_len - 1];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Pass 2: Exact buffer query succeeds
            let mut buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, expected_len);
            assert_eq!(required, expected_len);

            // Record is still retained after read
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Cdm);

            // Early-validation clearing: null out pointer on sidereon_cdm_parse_kvn
            assert_eq!(
                sidereon_cdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), ptr::null_mut()),
                SidereonStatus::NullPointer
            );

            // The producer operation boundary cleared the slot before early checks!
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
