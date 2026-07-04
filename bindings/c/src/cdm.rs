use super::*;

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
    ffi_boundary("sidereon_cdm_parse_kvn", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_cdm_parse_xml", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_cdm_to_kvn", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_cdm_to_xml", SidereonStatus::Panic, || {
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
        if let Some(p) = out_miss_distance_m.as_mut() {
            *p = c.miss_distance_m.unwrap_or(f64::NAN);
        }
        if let Some(p) = out_relative_speed_m_s.as_mut() {
            *p = c.relative_speed_m_s.unwrap_or(f64::NAN);
        }
        if let Some(p) = out_collision_probability.as_mut() {
            *p = c.collision_probability.unwrap_or(f64::NAN);
        }
        if let Some(p) = out_hard_body_radius_m.as_mut() {
            *p = c.hard_body_radius_m.unwrap_or(f64::NAN);
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

fn map_cdm_error(fn_name: &str, err: sidereon_core::astro::cdm::CdmError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
