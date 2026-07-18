use super::*;

/// A merged GNSS constellation identity catalog. Create with
/// sidereon_constellation_build and release with sidereon_constellation_free.
pub struct SidereonConstellation {
    pub(crate) records: Vec<ConstRecord>,
}

/// Time-aware NAVCEN assessments. Create with sidereon_navcen_parse_at and
/// release with sidereon_navcen_assessments_free.
pub struct SidereonNavcenAssessments {
    pub(crate) assessments: Vec<ConstNavcenAssessment>,
}

/// NAVCEN forecast interval provenance.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNavcenTiming {
    /// NANU type does not carry a bounded forecast outage interval.
    NotApplicable = 0,
    /// A complete bounded UTC interval was parsed.
    Parsed = 1,
    /// Forecast timing was incomplete, malformed, or contradictory.
    Unparseable = 2,
}

/// Fixed-width metadata for one time-aware NAVCEN assessment. Read the NANU
/// type, subject, and cleaned Outage Start text through the matching string
/// accessors.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNavcenAssessment {
    /// GNSS system (GPS).
    pub system: SidereonGnssSystem,
    /// Within-system PRN.
    pub prn: u16,
    /// Whether svn is present.
    pub svn_present: bool,
    /// Space vehicle number when svn_present is true.
    pub svn: u16,
    /// Usability at evaluated_at_unix_us.
    pub usable: bool,
    /// Whether the NAVCEN row carried an active NANU.
    pub active_nanu: bool,
    /// Explicit UTC evaluation instant in Unix microseconds.
    pub evaluated_at_unix_us: i64,
    /// Forecast timing provenance.
    pub timing: SidereonNavcenTiming,
    /// Whether effective_start_unix_us is present.
    pub effective_start_present: bool,
    /// Inclusive parsed interval start in Unix microseconds.
    pub effective_start_unix_us: i64,
    /// Whether effective_end_unix_us is present.
    pub effective_end_present: bool,
    /// Exclusive parsed interval end in Unix microseconds.
    pub effective_end_unix_us: i64,
}

/// A constellation catalog validation report. Create with
/// sidereon_constellation_validate or
/// sidereon_constellation_validate_against_sp3_ids and release with
/// sidereon_constellation_validation_free.
pub struct SidereonConstellationValidation {
    pub(crate) inner: ConstValidation,
}

/// A constellation member identified by its GNSS system and within-system PRN.
/// Returned by the validation PRN-list accessors, which now report
/// system-qualified PRNs because a catalog can mix constellations and a bare
/// PRN is only unique within one system.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationPrn {
    /// GNSS system the PRN belongs to.
    pub system: SidereonGnssSystem,
    /// Within-system PRN / orbital slot.
    pub prn: u16,
}

/// One catalog identity record, read back as a value struct. The SP3/RINEX id
/// token is derivable from (system, prn) via sidereon_constellation_gnss_sp3_id.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationRecord {
    /// GNSS system.
    pub system: SidereonGnssSystem,
    /// Within-system PRN / orbital slot.
    pub prn: u16,
    /// True when svn carries a space vehicle number.
    pub svn_present: bool,
    /// Space vehicle number, valid only when svn_present is true.
    pub svn: u16,
    /// NORAD catalog id.
    pub norad_id: u32,
    /// True when fdma_channel carries a GLONASS FDMA channel (GLONASS only).
    pub fdma_channel_present: bool,
    /// GLONASS FDMA channel (-7..=6), valid only when fdma_channel_present is true.
    pub fdma_channel: i8,
    /// Whether the satellite is active.
    pub active: bool,
    /// Whether the satellite is usable.
    pub usable: bool,
}

/// CSV boolean rendering style for sidereon_constellation_to_csv. Pass as a
/// uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonConstellationBoolStyle {
    /// Lowercase `true` / `false` (the conventional CSV form).
    Lower = 0,
    /// Capitalized `True` / `False`.
    Title = 1,
}

/// Build a merged GNSS identity catalog for one constellation from CelesTrak
/// OMM/JSON array text and an optional NAVCEN GPS status HTML overlay. system is
/// one of SidereonGnssSystem and selects which constellation's identity adapter
/// resolves the OMM OBJECT_NAMEs (GPS PRN, BeiDou (Cnn), Galileo GSAT, GLONASS
/// slot, QZSS slot). Parses the OMM array, derives identity records, and (when
/// navcen_len is nonzero) parses and merges the NAVCEN overlay (GPS only). On
/// success writes a newly owned handle to *out_catalog; release it with
/// sidereon_constellation_free.
///
/// Safety: omm_json must point to omm_len readable bytes; navcen_html must point
/// to navcen_len readable bytes or be NULL when navcen_len is 0; out_catalog must
/// point to storage for a SidereonConstellation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_build(
    system: u32,
    omm_json: *const u8,
    omm_len: usize,
    navcen_html: *const u8,
    navcen_len: usize,
    out_catalog: *mut *mut SidereonConstellation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_build",
        SidereonStatus::Panic,
        || {
            let out_catalog = c_try!(require_out(
                out_catalog,
                "sidereon_constellation_build",
                "out_catalog"
            ));
            *out_catalog = ptr::null_mut();
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_constellation_build",
                "system",
                system
            ));
            let omm_bytes = c_try!(require_slice(
                omm_json,
                omm_len,
                "sidereon_constellation_build",
                "omm_json"
            ));
            let omm_text = match str::from_utf8(omm_bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_constellation_build: omm_json is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let omm_array = match parse_omm_json_array(omm_text) {
                Ok(omm_array) => omm_array,
                Err(err) => {
                    set_last_error(format!("sidereon_constellation_build: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let mut records = match from_celestrak_omm(system, &omm_array.omms) {
                Ok(records) => records,
                Err(err) => {
                    set_last_error(format!("sidereon_constellation_build: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            if navcen_len > 0 {
                let navcen_bytes = c_try!(require_slice(
                    navcen_html,
                    navcen_len,
                    "sidereon_constellation_build",
                    "navcen_html"
                ));
                let statuses = match parse_navcen(navcen_bytes) {
                    Ok(statuses) => statuses,
                    Err(err) => {
                        set_last_error(format!("sidereon_constellation_build: {err}"));
                        return SidereonStatus::InvalidArgument;
                    }
                };
                records = merge_navcen(&records, &statuses);
            }
            write_boxed_handle(out_catalog, SidereonConstellation { records });
            SidereonStatus::Ok
        },
    )
}

/// Build a merged GNSS identity catalog with NAVCEN usability evaluated at an
/// explicit UTC Unix-microsecond instant. Active bounded forecasts affect the
/// catalog only on their parsed half-open interval. An ambiguous forecast is
/// retained by sidereon_navcen_parse_at but does not disable the satellite.
///
/// This is the time-aware companion to sidereon_constellation_build; the legacy
/// entry point retains its historical clock-free behavior. Release the returned
/// catalog with sidereon_constellation_free.
///
/// Safety: omm_json must point to omm_len readable bytes; navcen_html must point
/// to navcen_len readable bytes or be NULL when navcen_len is 0; out_catalog must
/// point to storage for a SidereonConstellation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_build_at(
    system: u32,
    omm_json: *const u8,
    omm_len: usize,
    navcen_html: *const u8,
    navcen_len: usize,
    evaluated_at_unix_us: i64,
    out_catalog: *mut *mut SidereonConstellation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_build_at",
        SidereonStatus::Panic,
        || {
            let out_catalog = c_try!(require_out(
                out_catalog,
                "sidereon_constellation_build_at",
                "out_catalog"
            ));
            *out_catalog = ptr::null_mut();
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_constellation_build_at",
                "system",
                system
            ));
            let omm_bytes = c_try!(require_slice(
                omm_json,
                omm_len,
                "sidereon_constellation_build_at",
                "omm_json"
            ));
            let omm_text = match str::from_utf8(omm_bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_constellation_build_at: omm_json is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let omm_array = match parse_omm_json_array(omm_text) {
                Ok(omm_array) => omm_array,
                Err(err) => {
                    set_last_error(format!("sidereon_constellation_build_at: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let mut records = match from_celestrak_omm(system, &omm_array.omms) {
                Ok(records) => records,
                Err(err) => {
                    set_last_error(format!("sidereon_constellation_build_at: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            if navcen_len > 0 {
                let navcen_bytes = c_try!(require_slice(
                    navcen_html,
                    navcen_len,
                    "sidereon_constellation_build_at",
                    "navcen_html"
                ));
                let assessments = match parse_navcen_at(
                    navcen_bytes,
                    UtcInstant::from_unix_microseconds(evaluated_at_unix_us),
                ) {
                    Ok(assessments) => assessments,
                    Err(err) => {
                        set_last_error(format!("sidereon_constellation_build_at: {err}"));
                        return SidereonStatus::InvalidArgument;
                    }
                };
                records = merge_navcen_at(&records, &assessments);
            }
            write_boxed_handle(out_catalog, SidereonConstellation { records });
            SidereonStatus::Ok
        },
    )
}

/// Parse NAVCEN status HTML and evaluate every row at explicit UTC Unix
/// microseconds. On success writes an owned assessment handle to out_assessments;
/// release it with sidereon_navcen_assessments_free.
///
/// Safety: navcen_html must point to navcen_len readable bytes; out_assessments
/// must point to storage for a SidereonNavcenAssessments*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_parse_at(
    navcen_html: *const u8,
    navcen_len: usize,
    evaluated_at_unix_us: i64,
    out_assessments: *mut *mut SidereonNavcenAssessments,
) -> SidereonStatus {
    ffi_boundary("sidereon_navcen_parse_at", SidereonStatus::Panic, || {
        let out_assessments = c_try!(require_out(
            out_assessments,
            "sidereon_navcen_parse_at",
            "out_assessments"
        ));
        *out_assessments = ptr::null_mut();
        let navcen_bytes = c_try!(require_slice(
            navcen_html,
            navcen_len,
            "sidereon_navcen_parse_at",
            "navcen_html"
        ));
        let assessments = match parse_navcen_at(
            navcen_bytes,
            UtcInstant::from_unix_microseconds(evaluated_at_unix_us),
        ) {
            Ok(assessments) => assessments,
            Err(err) => {
                set_last_error(format!("sidereon_navcen_parse_at: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_assessments, SidereonNavcenAssessments { assessments });
        SidereonStatus::Ok
    })
}

/// Write the number of parsed NAVCEN assessments to out_count.
///
/// Safety: assessments must be a live handle; out_count must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessment_count(
    assessments: *const SidereonNavcenAssessments,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_navcen_assessment_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_navcen_assessment_count",
                "out_count"
            ));
            *out_count = 0;
            let assessments = c_try!(require_ref(
                assessments,
                "sidereon_navcen_assessment_count",
                "assessments"
            ));
            *out_count = assessments.assessments.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy fixed metadata for one NAVCEN assessment by PRN-sorted index.
///
/// Safety: assessments must be a live handle; out_assessment must point to a
/// SidereonNavcenAssessment.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessment(
    assessments: *const SidereonNavcenAssessments,
    index: usize,
    out_assessment: *mut SidereonNavcenAssessment,
) -> SidereonStatus {
    ffi_boundary("sidereon_navcen_assessment", SidereonStatus::Panic, || {
        let out_assessment = c_try!(require_out(
            out_assessment,
            "sidereon_navcen_assessment",
            "out_assessment"
        ));
        *out_assessment = empty_navcen_assessment();
        let assessments = c_try!(require_ref(
            assessments,
            "sidereon_navcen_assessment",
            "assessments"
        ));
        let Some(assessment) = assessments.assessments.get(index) else {
            set_last_error(format!(
                "sidereon_navcen_assessment: index {index} out of range ({} assessments)",
                assessments.assessments.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_assessment = navcen_assessment_to_c(assessment);
        SidereonStatus::Ok
    })
}

/// Copy the NANU type for one assessment. An absent field has required length
/// zero. Output is not null-terminated.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessment_nanu_type(
    assessments: *const SidereonNavcenAssessments,
    index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    navcen_assessment_text(
        "sidereon_navcen_assessment_nanu_type",
        assessments,
        index,
        ByteCopyOut {
            out,
            out_len,
            out_written,
            out_required,
        },
        |assessment| assessment.status.nanu_type.as_deref(),
    )
}

/// Copy the NANU subject for one assessment. An absent field has required
/// length zero. Output is not null-terminated.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessment_nanu_subject(
    assessments: *const SidereonNavcenAssessments,
    index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    navcen_assessment_text(
        "sidereon_navcen_assessment_nanu_subject",
        assessments,
        index,
        ByteCopyOut {
            out,
            out_len,
            out_written,
            out_required,
        },
        |assessment| assessment.status.nanu_subject.as_deref(),
    )
}

/// Copy the cleaned Outage Start text for one assessment. Duplicate cells are
/// joined with " | ". An absent field has required length zero. Output is not
/// null-terminated.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessment_outage_start(
    assessments: *const SidereonNavcenAssessments,
    index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    navcen_assessment_text(
        "sidereon_navcen_assessment_outage_start",
        assessments,
        index,
        ByteCopyOut {
            out,
            out_len,
            out_written,
            out_required,
        },
        |assessment| assessment.outage_start.as_deref(),
    )
}

/// Write the number of records in the catalog to *out_count.
///
/// Safety: catalog must be a live handle from sidereon_constellation_build;
/// out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_record_count(
    catalog: *const SidereonConstellation,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_record_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_constellation_record_count",
                "out_count"
            ));
            *out_count = 0;
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_record_count",
                "catalog"
            ));
            *out_count = catalog.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one identity record (by zero-based index, ascending (system, prn) order)
/// into *out_record. Exposes the per-record fields including the GLONASS
/// fdma_channel. Fails with SIDEREON_STATUS_INVALID_ARGUMENT if index is out of
/// range (see sidereon_constellation_record_count).
///
/// Safety: catalog must be a live handle; out_record must point to a
/// SidereonConstellationRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_record(
    catalog: *const SidereonConstellation,
    index: usize,
    out_record: *mut SidereonConstellationRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_record",
        SidereonStatus::Panic,
        || {
            let out_record = c_try!(require_out(
                out_record,
                "sidereon_constellation_record",
                "out_record"
            ));
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_record",
                "catalog"
            ));
            let Some(record) = catalog.records.get(index) else {
                set_last_error(format!(
                    "sidereon_constellation_record: index {index} out of range ({} records)",
                    catalog.records.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_record = const_record_to_c(record);
            SidereonStatus::Ok
        },
    )
}

/// Write the SP3/RINEX id token for (system, prn) (for example "G05", "R12",
/// "C30") into out using the variable-length output contract documented at the
/// top of the header. The output is not null-terminated. system is one of
/// SidereonGnssSystem.
///
/// Safety: out must point to at least out_len writable bytes or be NULL when
/// out_len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_gnss_sp3_id(
    system: u32,
    prn: u16,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_gnss_sp3_id",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_gnss_sp3_id",
                out_written,
                out_required
            ));
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_constellation_gnss_sp3_id",
                "system",
                system
            ));
            let id = gnss_sp3_id(system, prn);
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_gnss_sp3_id",
                "out",
                id.as_bytes(),
                out,
                out_len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Export the catalog as the compact mapping CSV (header
/// `prn,norad_cat_id,active,sp3_id`). bool_style is one of
/// SidereonConstellationBoolStyle. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: catalog must be a live handle; out must point to at least out_len
/// writable bytes or be NULL when out_len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_to_csv(
    catalog: *const SidereonConstellation,
    bool_style: u32,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_to_csv",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_to_csv",
                out_written,
                out_required
            ));
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_to_csv",
                "catalog"
            ));
            let style = c_try!(constellation_bool_style_from_c(
                "sidereon_constellation_to_csv",
                bool_style
            ));
            let text = to_csv(&catalog.records, style);
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_to_csv",
                "out",
                text.as_bytes(),
                out,
                out_len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Validate the catalog identity (duplicate PRNs, duplicate NORAD ids, and
/// inactive or unusable PRNs) without an SP3 product. On success writes a newly
/// owned handle to *out_validation; release it with
/// sidereon_constellation_validation_free.
///
/// Safety: catalog must be a live handle; out_validation must point to storage
/// for a SidereonConstellationValidation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validate(
    catalog: *const SidereonConstellation,
    out_validation: *mut *mut SidereonConstellationValidation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validate",
        SidereonStatus::Panic,
        || {
            let out_validation = c_try!(require_out(
                out_validation,
                "sidereon_constellation_validate",
                "out_validation"
            ));
            *out_validation = ptr::null_mut();
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_validate",
                "catalog"
            ));
            let report = constellation_validate(&catalog.records);
            write_boxed_handle(
                out_validation,
                SidereonConstellationValidation { inner: report },
            );
            SidereonStatus::Ok
        },
    )
}

/// Validate the catalog against a list of SP3/RINEX satellite id tokens (for
/// example "G03"). Reports active+usable catalog ids absent from the list as
/// missing, list ids absent from the active+usable catalog as extra, plus the
/// identity findings. On success writes a newly owned handle to *out_validation;
/// release it with sidereon_constellation_validation_free.
///
/// Safety: catalog must be a live handle; sp3_ids must point to sp3_id_count
/// non-null null-terminated C strings when sp3_id_count is nonzero; out_validation
/// must point to storage for a SidereonConstellationValidation*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validate_against_sp3_ids(
    catalog: *const SidereonConstellation,
    sp3_ids: *const *const c_char,
    sp3_id_count: usize,
    out_validation: *mut *mut SidereonConstellationValidation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validate_against_sp3_ids",
        SidereonStatus::Panic,
        || {
            let out_validation = c_try!(require_out(
                out_validation,
                "sidereon_constellation_validate_against_sp3_ids",
                "out_validation"
            ));
            *out_validation = ptr::null_mut();
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_validate_against_sp3_ids",
                "catalog"
            ));
            let id_ptrs = c_try!(require_slice(
                sp3_ids,
                sp3_id_count,
                "sidereon_constellation_validate_against_sp3_ids",
                "sp3_ids"
            ));
            let mut ids: Vec<&str> = Vec::with_capacity(id_ptrs.len());
            for (idx, &id_ptr) in id_ptrs.iter().enumerate() {
                if id_ptr.is_null() {
                    set_last_error(format!(
                        "sidereon_constellation_validate_against_sp3_ids: null sp3_ids[{idx}]"
                    ));
                    return SidereonStatus::NullPointer;
                }
                match CStr::from_ptr(id_ptr).to_str() {
                    Ok(text) => ids.push(text),
                    Err(_) => {
                        set_last_error(format!(
                            "sidereon_constellation_validate_against_sp3_ids: sp3_ids[{idx}] is not valid UTF-8"
                        ));
                        return SidereonStatus::InvalidToken;
                    }
                }
            }
            let report = validate_against_sp3_ids(&catalog.records, &ids);
            write_boxed_handle(
                out_validation,
                SidereonConstellationValidation { inner: report },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write whether the validation report has no findings to *out_valid.
///
/// Safety: validation must be a live handle; out_valid must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_is_valid(
    validation: *const SidereonConstellationValidation,
    out_valid: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_is_valid",
        SidereonStatus::Panic,
        || {
            let out_valid = c_try!(require_out(
                out_valid,
                "sidereon_constellation_validation_is_valid",
                "out_valid"
            ));
            *out_valid = false;
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_is_valid",
                "validation"
            ));
            *out_valid = constellation_is_valid(&validation.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the inactive-or-unusable PRN list from a validation report as
/// system-qualified SidereonConstellationPrn entries. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: validation must be a live handle; out must point to at least len
/// writable SidereonConstellationPrn or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_inactive_unusable_prns(
    validation: *const SidereonConstellationValidation,
    out: *mut SidereonConstellationPrn,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_inactive_unusable_prns",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_validation_inactive_unusable_prns",
                out_written,
                out_required
            ));
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_inactive_unusable_prns",
                "validation"
            ));
            let values: Vec<SidereonConstellationPrn> = validation
                .inner
                .inactive_unusable_prns
                .iter()
                .map(|(system, prn)| SidereonConstellationPrn {
                    system: gnss_system_to_c(*system),
                    prn: *prn,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_validation_inactive_unusable_prns",
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

/// Copy the duplicate-PRN list from a validation report as system-qualified
/// SidereonConstellationPrn entries. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: validation must be a live handle; out must point to at least len
/// writable SidereonConstellationPrn or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_duplicate_prns(
    validation: *const SidereonConstellationValidation,
    out: *mut SidereonConstellationPrn,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_duplicate_prns",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_validation_duplicate_prns",
                out_written,
                out_required
            ));
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_duplicate_prns",
                "validation"
            ));
            let values: Vec<SidereonConstellationPrn> = validation
                .inner
                .duplicate_prns
                .iter()
                .map(|(system, prn)| SidereonConstellationPrn {
                    system: gnss_system_to_c(*system),
                    prn: *prn,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_validation_duplicate_prns",
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

/// Copy the duplicate-NORAD-id list from a validation report. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: validation must be a live handle; out must point to at least len
/// writable uint32_t or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_duplicate_norad_ids(
    validation: *const SidereonConstellationValidation,
    out: *mut u32,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_duplicate_norad_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_validation_duplicate_norad_ids",
                out_written,
                out_required
            ));
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_duplicate_norad_ids",
                "validation"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_validation_duplicate_norad_ids",
                "out",
                &validation.inner.duplicate_norad_ids,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the missing-SP3-id list (active+usable catalog ids absent from the
/// validated id list) as null-terminated tokens. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: validation must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_missing_sp3_ids(
    validation: *const SidereonConstellationValidation,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_missing_sp3_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_validation_missing_sp3_ids",
                out_written,
                out_required
            ));
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_missing_sp3_ids",
                "validation"
            ));
            let tokens = constellation_id_tokens(&validation.inner.missing_sp3_ids);
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_validation_missing_sp3_ids",
                "out",
                &tokens,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the extra-SP3-id list (validated ids absent from the active+usable
/// catalog) as null-terminated tokens. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: validation must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_extra_sp3_ids(
    validation: *const SidereonConstellationValidation,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validation_extra_sp3_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_validation_extra_sp3_ids",
                out_written,
                out_required
            ));
            let validation = c_try!(require_ref(
                validation,
                "sidereon_constellation_validation_extra_sp3_ids",
                "validation"
            ));
            let tokens = constellation_id_tokens(&validation.inner.extra_sp3_ids);
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_validation_extra_sp3_ids",
                "out",
                &tokens,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationDiffCounts {
    pub added: usize,
    pub removed: usize,
    pub norad_reassigned: usize,
    pub sp3_id_changed: usize,
    pub svn_changed: usize,
    pub fdma_channel_changed: usize,
    pub activity_changed: usize,
    pub usability_changed: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationU32Change {
    pub system: SidereonGnssSystem,
    pub prn: u16,
    pub from: u32,
    pub to: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationBoolChange {
    pub system: SidereonGnssSystem,
    pub prn: u16,
    pub from: bool,
    pub to: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationOptionalU16Change {
    pub system: SidereonGnssSystem,
    pub prn: u16,
    pub from_present: bool,
    pub from: u16,
    pub to_present: bool,
    pub to: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationOptionalI8Change {
    pub system: SidereonGnssSystem,
    pub prn: u16,
    pub from_present: bool,
    pub from: i8,
    pub to_present: bool,
    pub to: i8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationStringChangeMeta {
    pub system: SidereonGnssSystem,
    pub prn: u16,
    pub from_len: usize,
    pub to_len: usize,
}

pub struct SidereonConstellationDiff {
    pub(crate) inner: ConstDiff,
}

/// Resolve a Galileo GSAT number to its PRN. Delegates to
/// sidereon_core::constellation::galileo_prn_for_gsat.
///
/// Safety: out_present and out_prn must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_galileo_prn_for_gsat(
    gsat: u16,
    out_present: *mut bool,
    out_prn: *mut u16,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_galileo_prn_for_gsat",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_constellation_galileo_prn_for_gsat",
                "out_present"
            ));
            let out_prn = c_try!(require_out(
                out_prn,
                "sidereon_constellation_galileo_prn_for_gsat",
                "out_prn"
            ));
            *out_present = false;
            *out_prn = 0;
            if let Some(prn) = galileo_prn_for_gsat(gsat) {
                *out_present = true;
                *out_prn = prn;
            }
            SidereonStatus::Ok
        },
    )
}

/// Resolve a GLONASS vehicle number to its slot. Delegates to
/// sidereon_core::constellation::glonass_slot_for_number.
///
/// Safety: out_present and out_slot must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_glonass_slot_for_number(
    number: u16,
    out_present: *mut bool,
    out_slot: *mut u16,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_glonass_slot_for_number",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_constellation_glonass_slot_for_number",
                "out_present"
            ));
            let out_slot = c_try!(require_out(
                out_slot,
                "sidereon_constellation_glonass_slot_for_number",
                "out_slot"
            ));
            *out_present = false;
            *out_slot = 0;
            if let Some(slot) = glonass_slot_for_number(number) {
                *out_present = true;
                *out_slot = slot;
            }
            SidereonStatus::Ok
        },
    )
}

/// Resolve a GLONASS slot to its FDMA frequency channel. Delegates to
/// sidereon_core::constellation::glonass_fdma_channel.
///
/// Safety: out_present and out_channel must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_glonass_fdma_channel(
    slot: u16,
    out_present: *mut bool,
    out_channel: *mut i8,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_glonass_fdma_channel",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_constellation_glonass_fdma_channel",
                "out_present"
            ));
            let out_channel = c_try!(require_out(
                out_channel,
                "sidereon_constellation_glonass_fdma_channel",
                "out_channel"
            ));
            *out_present = false;
            *out_channel = 0;
            if let Some(channel) = glonass_fdma_channel(slot) {
                *out_present = true;
                *out_channel = channel;
            }
            SidereonStatus::Ok
        },
    )
}

/// Validate a catalog against a loaded SP3 product. Delegates to
/// sidereon_core::constellation::validate_against_sp3.
///
/// Safety: catalog and sp3 must be live handles; out_validation points to
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validate_against_sp3(
    catalog: *const SidereonConstellation,
    sp3: *const SidereonSp3,
    out_validation: *mut *mut SidereonConstellationValidation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validate_against_sp3",
        SidereonStatus::Panic,
        || {
            let out_validation = c_try!(require_out(
                out_validation,
                "sidereon_constellation_validate_against_sp3",
                "out_validation"
            ));
            *out_validation = ptr::null_mut();
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_validate_against_sp3",
                "catalog"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_constellation_validate_against_sp3",
                "sp3"
            ));
            let report = validate_against_sp3(&catalog.records, &sp3.inner);
            write_boxed_handle(
                out_validation,
                SidereonConstellationValidation { inner: report },
            );
            SidereonStatus::Ok
        },
    )
}

/// Return Ok only when SP3-id validation has no findings. Delegates to
/// sidereon_core::constellation::validate_against_sp3_ids_strict.
///
/// Safety: catalog must be a live handle; sp3_ids points to sp3_id_count
/// null-terminated UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validate_against_sp3_ids_strict(
    catalog: *const SidereonConstellation,
    sp3_ids: *const *const c_char,
    sp3_id_count: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_validate_against_sp3_ids_strict",
        SidereonStatus::Panic,
        || {
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_constellation_validate_against_sp3_ids_strict",
                "catalog"
            ));
            let ids = c_try!(constellation_sp3_ids_from_c(
                "sidereon_constellation_validate_against_sp3_ids_strict",
                sp3_ids,
                sp3_id_count
            ));
            let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            match validate_against_sp3_ids_strict(&catalog.records, &id_refs) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => extra_invalid_arg(
                    "sidereon_constellation_validate_against_sp3_ids_strict",
                    err,
                ),
            }
        },
    )
}

/// Compare two catalogs by system and PRN. Delegates to
/// sidereon_core::constellation::diff.
///
/// Safety: previous and current must be live handles; out_diff points to handle
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff(
    previous: *const SidereonConstellation,
    current: *const SidereonConstellation,
    out_diff: *mut *mut SidereonConstellationDiff,
) -> SidereonStatus {
    ffi_boundary("sidereon_constellation_diff", SidereonStatus::Panic, || {
        let out_diff = c_try!(require_out(
            out_diff,
            "sidereon_constellation_diff",
            "out_diff"
        ));
        *out_diff = ptr::null_mut();
        let previous = c_try!(require_ref(
            previous,
            "sidereon_constellation_diff",
            "previous"
        ));
        let current = c_try!(require_ref(
            current,
            "sidereon_constellation_diff",
            "current"
        ));
        let inner = constellation_diff(&previous.records, &current.records);
        write_boxed_handle(out_diff, SidereonConstellationDiff { inner });
        SidereonStatus::Ok
    })
}

/// Write whether a constellation diff has any findings. Delegates to
/// sidereon_core::constellation::changed.
///
/// Safety: diff must be a live handle; out_changed points to bool storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_changed(
    diff: *const SidereonConstellationDiff,
    out_changed: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_changed",
        SidereonStatus::Panic,
        || {
            let out_changed = c_try!(require_out(
                out_changed,
                "sidereon_constellation_diff_changed",
                "out_changed"
            ));
            *out_changed = false;
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_changed",
                "diff"
            ));
            *out_changed = constellation_changed(&diff.inner);
            SidereonStatus::Ok
        },
    )
}

/// Read per-list counts from a constellation diff.
///
/// Safety: diff must be a live handle; out_counts points to storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_counts(
    diff: *const SidereonConstellationDiff,
    out_counts: *mut SidereonConstellationDiffCounts,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_counts",
        SidereonStatus::Panic,
        || {
            let out_counts = c_try!(require_out(
                out_counts,
                "sidereon_constellation_diff_counts",
                "out_counts"
            ));
            *out_counts = SidereonConstellationDiffCounts {
                added: 0,
                removed: 0,
                norad_reassigned: 0,
                sp3_id_changed: 0,
                svn_changed: 0,
                fdma_channel_changed: 0,
                activity_changed: 0,
                usability_changed: 0,
            };
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_counts",
                "diff"
            ));
            *out_counts = constellation_diff_counts_to_c(&diff.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy records present only in the current catalog.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_added(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_added",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_added",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_added",
                "diff"
            ));
            let values: Vec<SidereonConstellationRecord> =
                diff.inner.added.iter().map(const_record_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_added",
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

/// Copy records present only in the previous catalog.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_removed(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_removed",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_removed",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_removed",
                "diff"
            ));
            let values: Vec<SidereonConstellationRecord> =
                diff.inner.removed.iter().map(const_record_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_removed",
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

/// Copy NORAD reassignment changes from a constellation diff.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_norad_reassigned(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationU32Change,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_norad_reassigned",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_norad_reassigned",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_norad_reassigned",
                "diff"
            ));
            let values: Vec<SidereonConstellationU32Change> = diff
                .inner
                .norad_reassigned
                .iter()
                .map(constellation_u32_change_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_norad_reassigned",
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

/// Read metadata for one SP3-id string change.
///
/// Safety: diff must be a live handle; out_meta points to storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_sp3_id_changed_meta(
    diff: *const SidereonConstellationDiff,
    index: usize,
    out_meta: *mut SidereonConstellationStringChangeMeta,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_sp3_id_changed_meta",
        SidereonStatus::Panic,
        || {
            let out_meta = c_try!(require_out(
                out_meta,
                "sidereon_constellation_diff_sp3_id_changed_meta",
                "out_meta"
            ));
            *out_meta = SidereonConstellationStringChangeMeta {
                system: SidereonGnssSystem::Gps,
                prn: 0,
                from_len: 0,
                to_len: 0,
            };
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_sp3_id_changed_meta",
                "diff"
            ));
            let Some(change) = diff.inner.sp3_id_changed.get(index) else {
                set_last_error(format!(
                    "sidereon_constellation_diff_sp3_id_changed_meta: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_meta = constellation_string_change_meta_to_c(change);
            SidereonStatus::Ok
        },
    )
}

/// Copy the previous SP3 id for one string change.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_sp3_id_changed_from(
    diff: *const SidereonConstellationDiff,
    index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_sp3_id_changed_from",
        SidereonStatus::Panic,
        || {
            constellation_copy_sp3_change_text(
                "sidereon_constellation_diff_sp3_id_changed_from",
                diff,
                index,
                true,
                ByteCopyOut {
                    out,
                    out_len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy the current SP3 id for one string change.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_sp3_id_changed_to(
    diff: *const SidereonConstellationDiff,
    index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_sp3_id_changed_to",
        SidereonStatus::Panic,
        || {
            constellation_copy_sp3_change_text(
                "sidereon_constellation_diff_sp3_id_changed_to",
                diff,
                index,
                false,
                ByteCopyOut {
                    out,
                    out_len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy SVN changes from a constellation diff.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_svn_changed(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationOptionalU16Change,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_svn_changed",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_svn_changed",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_svn_changed",
                "diff"
            ));
            let values: Vec<SidereonConstellationOptionalU16Change> = diff
                .inner
                .svn_changed
                .iter()
                .map(constellation_optional_u16_change_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_svn_changed",
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

/// Copy FDMA-channel changes from a constellation diff.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_fdma_channel_changed(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationOptionalI8Change,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_fdma_channel_changed",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_fdma_channel_changed",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_fdma_channel_changed",
                "diff"
            ));
            let values: Vec<SidereonConstellationOptionalI8Change> = diff
                .inner
                .fdma_channel_changed
                .iter()
                .map(constellation_optional_i8_change_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_fdma_channel_changed",
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

/// Copy activity flag changes from a constellation diff.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_activity_changed(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationBoolChange,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_activity_changed",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_activity_changed",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_activity_changed",
                "diff"
            ));
            let values: Vec<SidereonConstellationBoolChange> = diff
                .inner
                .activity_changed
                .iter()
                .map(constellation_bool_change_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_activity_changed",
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

/// Copy usability flag changes from a constellation diff.
///
/// Safety: diff must be a live handle; out uses the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_usability_changed(
    diff: *const SidereonConstellationDiff,
    out: *mut SidereonConstellationBoolChange,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_constellation_diff_usability_changed",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_constellation_diff_usability_changed",
                out_written,
                out_required
            ));
            let diff = c_try!(require_ref(
                diff,
                "sidereon_constellation_diff_usability_changed",
                "diff"
            ));
            let values: Vec<SidereonConstellationBoolChange> = diff
                .inner
                .usability_changed
                .iter()
                .map(constellation_bool_change_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_constellation_diff_usability_changed",
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

/// Release a constellation diff handle. Passing NULL is a no-op.
///
/// Safety: diff must be NULL or a live handle from sidereon_constellation_diff
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_diff_free(diff: *mut SidereonConstellationDiff) {
    ffi_boundary("sidereon_constellation_diff_free", (), || {
        free_boxed(diff);
    });
}

/// Release a NAVCEN assessment handle. Passing NULL is a no-op.
///
/// Safety: assessments must be NULL or a live handle from
/// sidereon_navcen_parse_at that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_navcen_assessments_free(
    assessments: *mut SidereonNavcenAssessments,
) {
    ffi_boundary("sidereon_navcen_assessments_free", (), || {
        free_boxed(assessments);
    });
}

/// Release a constellation catalog handle from sidereon_constellation_build.
/// Passing NULL is a no-op.
///
/// Safety: catalog must be NULL or a live handle from
/// sidereon_constellation_build that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_free(catalog: *mut SidereonConstellation) {
    ffi_boundary("sidereon_constellation_free", (), || {
        free_boxed(catalog);
    });
}

/// Release a validation report handle from sidereon_constellation_validate or
/// sidereon_constellation_validate_against_sp3_ids. Passing NULL is a no-op.
///
/// Safety: validation must be NULL or a live handle from a constellation validate
/// call that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_constellation_validation_free(
    validation: *mut SidereonConstellationValidation,
) {
    ffi_boundary("sidereon_constellation_validation_free", (), || {
        free_boxed(validation);
    });
}

/// Constellation physical constants, mirroring
/// sidereon_core::ephemeris::ConstellationConstants.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonConstellationConstants {
    /// Gravitational constant GM (m^3 / s^2).
    pub gm_m3_s2: f64,
    /// Earth rotation rate for the Sagnac term (rad/s).
    pub omega_e_rad_s: f64,
    /// Relativistic clock constant F = -2*sqrt(GM)/c^2 (s / sqrt(m)).
    pub dtr_f: f64,
}

fn constellation_bool_style_from_c(fn_name: &str, style: u32) -> Result<BoolStyle, SidereonStatus> {
    match style {
        value if value == SidereonConstellationBoolStyle::Lower as u32 => Ok(BoolStyle::Lower),
        value if value == SidereonConstellationBoolStyle::Title as u32 => Ok(BoolStyle::Title),
        _ => {
            set_last_error(format!("{fn_name}: invalid CSV boolean style {style}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn constellation_id_tokens(ids: &[String]) -> Vec<SidereonSatelliteToken> {
    ids.iter().map(|id| satellite_token_from_text(id)).collect()
}

fn constellation_u32_change_to_c(change: &ConstFieldChange<u32>) -> SidereonConstellationU32Change {
    SidereonConstellationU32Change {
        system: gnss_system_to_c(change.system),
        prn: change.prn,
        from: change.from,
        to: change.to,
    }
}

fn constellation_bool_change_to_c(
    change: &ConstFieldChange<bool>,
) -> SidereonConstellationBoolChange {
    SidereonConstellationBoolChange {
        system: gnss_system_to_c(change.system),
        prn: change.prn,
        from: change.from,
        to: change.to,
    }
}

fn constellation_optional_u16_change_to_c(
    change: &ConstFieldChange<Option<u16>>,
) -> SidereonConstellationOptionalU16Change {
    SidereonConstellationOptionalU16Change {
        system: gnss_system_to_c(change.system),
        prn: change.prn,
        from_present: change.from.is_some(),
        from: change.from.unwrap_or(0),
        to_present: change.to.is_some(),
        to: change.to.unwrap_or(0),
    }
}

fn constellation_optional_i8_change_to_c(
    change: &ConstFieldChange<Option<i8>>,
) -> SidereonConstellationOptionalI8Change {
    SidereonConstellationOptionalI8Change {
        system: gnss_system_to_c(change.system),
        prn: change.prn,
        from_present: change.from.is_some(),
        from: change.from.unwrap_or(0),
        to_present: change.to.is_some(),
        to: change.to.unwrap_or(0),
    }
}

fn constellation_string_change_meta_to_c(
    change: &ConstFieldChange<String>,
) -> SidereonConstellationStringChangeMeta {
    SidereonConstellationStringChangeMeta {
        system: gnss_system_to_c(change.system),
        prn: change.prn,
        from_len: change.from.len(),
        to_len: change.to.len(),
    }
}

fn constellation_diff_counts_to_c(diff: &ConstDiff) -> SidereonConstellationDiffCounts {
    SidereonConstellationDiffCounts {
        added: diff.added.len(),
        removed: diff.removed.len(),
        norad_reassigned: diff.norad_reassigned.len(),
        sp3_id_changed: diff.sp3_id_changed.len(),
        svn_changed: diff.svn_changed.len(),
        fdma_channel_changed: diff.fdma_channel_changed.len(),
        activity_changed: diff.activity_changed.len(),
        usability_changed: diff.usability_changed.len(),
    }
}

unsafe fn constellation_sp3_ids_from_c(
    fn_name: &str,
    sp3_ids: *const *const c_char,
    sp3_id_count: usize,
) -> Result<Vec<String>, SidereonStatus> {
    let id_ptrs = require_slice(sp3_ids, sp3_id_count, fn_name, "sp3_ids")?;
    let mut ids = Vec::with_capacity(id_ptrs.len());
    for (idx, &id_ptr) in id_ptrs.iter().enumerate() {
        if id_ptr.is_null() {
            set_last_error(format!("{fn_name}: null sp3_ids[{idx}]"));
            return Err(SidereonStatus::NullPointer);
        }
        match CStr::from_ptr(id_ptr).to_str() {
            Ok(text) => ids.push(text.to_string()),
            Err(_) => {
                set_last_error(format!("{fn_name}: sp3_ids[{idx}] is not valid UTF-8"));
                return Err(SidereonStatus::InvalidToken);
            }
        }
    }
    Ok(ids)
}

unsafe fn constellation_copy_sp3_change_text(
    fn_name: &str,
    diff: *const SidereonConstellationDiff,
    index: usize,
    from_value: bool,
    copy_out: ByteCopyOut,
) -> SidereonStatus {
    c_try!(init_copy_counts(
        fn_name,
        copy_out.out_written,
        copy_out.out_required
    ));
    let diff = c_try!(require_ref(diff, fn_name, "diff"));
    let Some(change) = diff.inner.sp3_id_changed.get(index) else {
        set_last_error(format!("{fn_name}: index {index} out of range"));
        return SidereonStatus::InvalidArgument;
    };
    let text = if from_value {
        change.from.as_str()
    } else {
        change.to.as_str()
    };
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        text.as_bytes(),
        copy_out.out,
        copy_out.out_len,
        copy_out.out_written,
        copy_out.out_required,
    ));
    SidereonStatus::Ok
}

fn empty_navcen_assessment() -> SidereonNavcenAssessment {
    SidereonNavcenAssessment {
        system: SidereonGnssSystem::Gps,
        prn: 0,
        svn_present: false,
        svn: 0,
        usable: false,
        active_nanu: false,
        evaluated_at_unix_us: 0,
        timing: SidereonNavcenTiming::NotApplicable,
        effective_start_present: false,
        effective_start_unix_us: 0,
        effective_end_present: false,
        effective_end_unix_us: 0,
    }
}

fn navcen_assessment_to_c(assessment: &ConstNavcenAssessment) -> SidereonNavcenAssessment {
    let (timing, start, end) = match assessment.timing {
        NavcenTiming::NotApplicable => (SidereonNavcenTiming::NotApplicable, None, None),
        NavcenTiming::Unparseable => (SidereonNavcenTiming::Unparseable, None, None),
        NavcenTiming::Parsed(interval) => (
            SidereonNavcenTiming::Parsed,
            Some(interval.start_utc.unix_microseconds()),
            Some(interval.end_utc.unix_microseconds()),
        ),
    };
    SidereonNavcenAssessment {
        system: gnss_system_to_c(assessment.status.system),
        prn: assessment.status.prn,
        svn_present: assessment.status.svn.is_some(),
        svn: assessment.status.svn.unwrap_or(0),
        usable: assessment.status.usable,
        active_nanu: assessment.status.active_nanu,
        evaluated_at_unix_us: assessment.evaluated_at_utc.unix_microseconds(),
        timing,
        effective_start_present: start.is_some(),
        effective_start_unix_us: start.unwrap_or(0),
        effective_end_present: end.is_some(),
        effective_end_unix_us: end.unwrap_or(0),
    }
}

unsafe fn navcen_assessment_text<'a>(
    fn_name: &'static str,
    assessments: *const SidereonNavcenAssessments,
    index: usize,
    copy_out: ByteCopyOut,
    select: impl Fn(&'a ConstNavcenAssessment) -> Option<&'a str>,
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            fn_name,
            copy_out.out_written,
            copy_out.out_required
        ));
        let assessments = c_try!(require_ref(assessments, fn_name, "assessments"));
        let Some(assessment) = assessments.assessments.get(index) else {
            set_last_error(format!("{fn_name}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        let text = select(assessment).unwrap_or("");
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            text.as_bytes(),
            copy_out.out,
            copy_out.out_len,
            copy_out.out_written,
            copy_out.out_required,
        ));
        SidereonStatus::Ok
    })
}

impl SidereonConstellationConstants {
    pub(crate) fn to_core(self) -> CoreConstellationConstants {
        CoreConstellationConstants {
            gm_m3_s2: self.gm_m3_s2,
            omega_e_rad_s: self.omega_e_rad_s,
            dtr_f: self.dtr_f,
        }
    }
}
