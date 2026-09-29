use super::*;

/// A lenient OMM catalog: the records that resolved to the requested system plus
/// the OMM entries that did not. Opaque to C. Create with
/// sidereon_omm_catalog_build_lenient and release with sidereon_omm_catalog_free.
pub struct SidereonOmmCatalog {
    pub(crate) inner: ConstCatalog,
    /// Count of JSON array elements that did not parse into an OMM object at all
    /// (a non-object element, or one that failed field validation). Distinct from
    /// inner.skipped, which holds OMMs that parsed but did not resolve to a record
    /// for the requested system. Surfaced via
    /// sidereon_omm_catalog_malformed_count so a wholly malformed feed is
    /// distinguishable from an empty one.
    pub(crate) malformed: usize,
    /// Parser-level malformed records with their input index and complete cause.
    pub(crate) parser_skipped: Vec<sidereon_core::astro::omm::OmmSkippedRecord>,
}

/// Build a lenient identity catalog for one constellation from a CelesTrak
/// OMM/JSON array. system is one of SidereonGnssSystem and selects which
/// constellation's identity adapter resolves the OMM OBJECT_NAMEs. Every entry
/// that resolves to a PRN for system becomes a record (read with
/// sidereon_omm_catalog_record_count / sidereon_omm_catalog_record); every entry
/// that does not is kept as a skipped identity (read with
/// sidereon_omm_catalog_skipped_count / sidereon_omm_catalog_skipped). Array
/// elements that do not parse into an OMM at all (malformed JSON objects) are
/// neither records nor skipped identities; their count is reported by
/// sidereon_omm_catalog_malformed_count so a wholly malformed feed is
/// distinguishable from an empty one. Unlike sidereon_constellation_build this
/// never fails on an unresolvable name, so it is what a caller feeds a raw
/// combined `gnss` feed. On success writes a newly owned handle to *out_catalog;
/// release it with sidereon_omm_catalog_free.
///
/// Safety: omm_json must point to omm_len readable bytes; out_catalog must point
/// to storage for a SidereonOmmCatalog*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_build_lenient(
    system: u32,
    omm_json: *const u8,
    omm_len: usize,
    out_catalog: *mut *mut SidereonOmmCatalog,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_catalog_build_lenient",
        SidereonStatus::Panic,
        || {
            let out_catalog = c_try!(require_out(
                out_catalog,
                "sidereon_omm_catalog_build_lenient",
                "out_catalog"
            ));
            let inputs = (|| {
                let system = gnss_system_from_c_code(
                    "sidereon_omm_catalog_build_lenient",
                    "system",
                    system,
                )?;
                let bytes = require_slice(
                    omm_json,
                    omm_len,
                    "sidereon_omm_catalog_build_lenient",
                    "omm_json",
                )?;
                let text = str::from_utf8(bytes)
                    .map_err(|_| {
                        set_last_error(
                            "sidereon_omm_catalog_build_lenient: omm_json is not valid UTF-8"
                                .to_string(),
                        );
                        SidereonStatus::InvalidToken
                    })?
                    .to_owned();
                Ok::<_, SidereonStatus>((system, text))
            })();
            *out_catalog = ptr::null_mut();
            let (system, omm_text) = c_try!(inputs);
            let omm_array = match parse_omm_json_array(&omm_text) {
                Ok(omm_array) => omm_array,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Omm,
                        "sidereon_omm_catalog_build_lenient",
                        crate::engine_error::omm_error_value(&err),
                    );
                    set_last_error(format!("sidereon_omm_catalog_build_lenient: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let malformed = omm_array.skipped.len();
            let catalog = from_celestrak_omm_lenient(system, &omm_array.omms);
            write_boxed_handle(
                out_catalog,
                SidereonOmmCatalog {
                    inner: catalog,
                    malformed,
                    parser_skipped: omm_array.skipped,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Return one parser-level malformed JSON array record as owned JSON detail.
/// This collection is distinct from catalog identity skips, available through
/// sidereon_omm_catalog_skipped_count and sidereon_omm_catalog_skipped.
/// The payload preserves the original zero-based index and recursively typed
/// OMM parse error. Uses the standard two-pass byte-copy contract.
///
/// Safety: catalog is a live catalog handle; out_index, out_written and
/// out_required point to writable size_t values; out is NULL only when len is 0
/// or points to len writable bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_malformed_record(
    catalog: *const SidereonOmmCatalog,
    index: usize,
    out_index: *mut usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_malformed_record",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_malformed_record",
                out_index,
                "out_index",
                catalog,
                "catalog"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_malformed_record",
                out_written,
                "out_written",
                catalog,
                "catalog"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_malformed_record",
                out_required,
                "out_required",
                catalog,
                "catalog"
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_omm_catalog_malformed_record",
                &[
                    (out.cast(), size_of::<u8>(), len, "out"),
                    (out_index.cast(), size_of::<usize>(), 1, "out_index"),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required")
                ],
                catalog,
                "catalog"
            ));
            let out_index = c_try!(require_out(
                out_index,
                "sidereon_omm_catalog_malformed_record",
                "out_index"
            ));
            *out_index = 0;
            c_try!(init_copy_counts(
                "sidereon_omm_catalog_malformed_record",
                out_written,
                out_required
            ));
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_malformed_record",
                "catalog"
            ));
            let Some(skipped) = catalog.parser_skipped.get(index) else {
                set_last_error(format!("sidereon_omm_catalog_malformed_record: index {index} out of range ({} records)", catalog.parser_skipped.len()));
                return SidereonStatus::InvalidArgument;
            };
            let payload = serde_json::json!({
                "index": skipped.index,
                "error": crate::engine_error::omm_error_value(&skipped.reason),
            })
            .to_string();
            *out_index = skipped.index;
            c_try!(copy_prefix_to_c(
                "sidereon_omm_catalog_malformed_record",
                "out",
                payload.as_bytes(),
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the number of resolved records in the catalog to *out_count.
///
/// Safety: catalog must be a live handle from sidereon_omm_catalog_build_lenient;
/// out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_record_count(
    catalog: *const SidereonOmmCatalog,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_record_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_omm_catalog_record_count",
                "out_count"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_record_count",
                out_count,
                "out_count",
                catalog,
                "catalog"
            ));
            *out_count = 0;
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_record_count",
                "catalog"
            ));
            *out_count = catalog.inner.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one resolved record (by zero-based index, ascending (system, prn) order)
/// into *out_record. The fields match sidereon_constellation_record. Fails with
/// SIDEREON_STATUS_INVALID_ARGUMENT if index is out of range (see
/// sidereon_omm_catalog_record_count).
///
/// Safety: catalog must be a live handle; out_record must point to a
/// SidereonConstellationRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_record(
    catalog: *const SidereonOmmCatalog,
    index: usize,
    out_record: *mut SidereonConstellationRecord,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_catalog_record", SidereonStatus::Panic, || {
        let out_record = c_try!(require_out(
            out_record,
            "sidereon_omm_catalog_record",
            "out_record"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_omm_catalog_record",
            out_record,
            "out_record",
            catalog,
            "catalog"
        ));
        let catalog = c_try!(require_ref(
            catalog,
            "sidereon_omm_catalog_record",
            "catalog"
        ));
        let Some(record) = catalog.inner.records.get(index) else {
            set_last_error(format!(
                "sidereon_omm_catalog_record: index {index} out of range ({} records)",
                catalog.inner.records.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_record = const_record_to_c(record);
        SidereonStatus::Ok
    })
}

/// Write the number of skipped (unresolved) OMM entries to *out_count.
///
/// Safety: catalog must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_skipped_count(
    catalog: *const SidereonOmmCatalog,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_skipped_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_omm_catalog_skipped_count",
                "out_count"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_skipped_count",
                out_count,
                "out_count",
                catalog,
                "catalog"
            ));
            *out_count = 0;
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_skipped_count",
                "catalog"
            ));
            *out_count = catalog.inner.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Write the number of JSON array elements that did not parse into an OMM object
/// at all to *out_count. These are neither records nor skipped identities (the
/// element carried no usable OMM); a nonzero count on an otherwise empty catalog
/// means the feed was malformed rather than empty.
///
/// Safety: catalog must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_malformed_count(
    catalog: *const SidereonOmmCatalog,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_malformed_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_omm_catalog_malformed_count",
                "out_count"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_malformed_count",
                out_count,
                "out_count",
                catalog,
                "catalog"
            ));
            *out_count = 0;
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_malformed_count",
                "catalog"
            ));
            *out_count = catalog.malformed;
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped OMM entry (by zero-based index, in input order) into *out.
/// The object name itself, when present, is retrieved with
/// sidereon_omm_catalog_skipped_object_name. Fails with
/// SIDEREON_STATUS_INVALID_ARGUMENT if index is out of range (see
/// sidereon_omm_catalog_skipped_count).
///
/// Safety: catalog must be a live handle; out must point to a SidereonSkippedOmm.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_skipped(
    catalog: *const SidereonOmmCatalog,
    index: usize,
    out: *mut SidereonSkippedOmm,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_skipped",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_omm_catalog_skipped", "out"));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_skipped",
                out,
                "out",
                catalog,
                "catalog"
            ));
            *out = SidereonSkippedOmm {
                norad_id_present: false,
                norad_id: 0,
                object_name_present: false,
            };
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_skipped",
                "catalog"
            ));
            let Some(skipped) = catalog.inner.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_omm_catalog_skipped: index {index} out of range ({} entries)",
                    catalog.inner.skipped.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = SidereonSkippedOmm {
                norad_id_present: skipped.norad_id.is_some(),
                norad_id: skipped.norad_id.unwrap_or(0),
                object_name_present: skipped.object_name.is_some(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy the OBJECT_NAME of one skipped OMM entry (by zero-based index) into out
/// using the variable-length output contract documented at the top of the header.
/// The output is the UTF-8 name bytes and is not null-terminated. When the entry
/// carried no object name (object_name_present is false in
/// sidereon_omm_catalog_skipped), *out_required is 0 and nothing is written. Fails
/// with SIDEREON_STATUS_INVALID_ARGUMENT if index is out of range.
///
/// Safety: catalog must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_skipped_object_name(
    catalog: *const SidereonOmmCatalog,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_catalog_skipped_object_name",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_skipped_object_name",
                out_written,
                "out_written",
                catalog,
                "catalog"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_catalog_skipped_object_name",
                out_required,
                "out_required",
                catalog,
                "catalog"
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_omm_catalog_skipped_object_name",
                &[
                    (out.cast(), size_of::<u8>(), len, "out"),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required")
                ],
                catalog,
                "catalog"
            ));
            c_try!(init_copy_counts(
                "sidereon_omm_catalog_skipped_object_name",
                out_written,
                out_required
            ));
            let catalog = c_try!(require_ref(
                catalog,
                "sidereon_omm_catalog_skipped_object_name",
                "catalog"
            ));
            let Some(skipped) = catalog.inner.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_omm_catalog_skipped_object_name: index {index} out of range ({} entries)",
                    catalog.inner.skipped.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let name_bytes: &[u8] = skipped
                .object_name
                .as_deref()
                .map(str::as_bytes)
                .unwrap_or(&[]);
            c_try!(copy_prefix_to_c(
                "sidereon_omm_catalog_skipped_object_name",
                "out",
                name_bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a lenient OMM catalog handle from sidereon_omm_catalog_build_lenient.
/// Passing NULL is a no-op.
///
/// Safety: catalog must be NULL or a live handle from
/// sidereon_omm_catalog_build_lenient that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_catalog_free(catalog: *mut SidereonOmmCatalog) {
    ffi_boundary("sidereon_omm_catalog_free", (), || {
        free_boxed(catalog);
    });
}

// === Integer least-squares (LAMBDA) ambiguity kernel =======================
//
// Wraps sidereon_core::ils::{lambda_ils_search, bounded_ils_search}: the
// standalone integer-ambiguity resolution kernels. Inputs are the float
// ambiguity vector and its covariance (row-major, n x n); outputs are the best
// integer vector plus the ratio-test verdict and scores. These are pure compute
// (no engine state), exposed here so a C caller can resolve ambiguities without
// the full RTK/PPP solve.

// --- OMM (sidereon_core::astro::omm) reader + serializers --------------------

/// A parsed Orbit Mean-Elements Message. Opaque to C. Create with
/// sidereon_omm_parse_kvn / _xml / _json; serialize with sidereon_omm_to_kvn /
/// _xml / _json; release with sidereon_omm_free.
pub struct SidereonOmm {
    pub(crate) inner: sidereon_core::astro::omm::Omm,
}

/// Parse an OMM from KVN text. On success writes a newly owned handle to
/// *out_omm. Delegates to sidereon_core::astro::omm::parse_kvn.
///
/// Safety: data points to len readable bytes; out_omm points to a SidereonOmm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_kvn(
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_parse_kvn",
        SidereonStatus::Panic,
        || {
            omm_parse(
                "sidereon_omm_parse_kvn",
                data,
                len,
                out_omm,
                sidereon_core::astro::omm::parse_kvn,
            )
        },
    )
}

/// Parse an OMM from XML text. On success writes a newly owned handle to
/// *out_omm. Delegates to sidereon_core::astro::omm::parse_xml.
///
/// Safety: data points to len readable bytes; out_omm points to a SidereonOmm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_xml(
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_parse_xml",
        SidereonStatus::Panic,
        || {
            omm_parse(
                "sidereon_omm_parse_xml",
                data,
                len,
                out_omm,
                sidereon_core::astro::omm::parse_xml,
            )
        },
    )
}

/// Parse an OMM from JSON text (a single OMM object). On success writes a newly
/// owned handle to *out_omm. Delegates to sidereon_core::astro::omm::parse_json.
///
/// Safety: data points to len readable bytes; out_omm points to a SidereonOmm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_json(
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_parse_json",
        SidereonStatus::Panic,
        || {
            omm_parse(
                "sidereon_omm_parse_json",
                data,
                len,
                out_omm,
                sidereon_core::astro::omm::parse_json,
            )
        },
    )
}

/// Serialize an OMM to KVN text (not null-terminated). Round-trips with
/// sidereon_omm_parse_kvn. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_kvn.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: omm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_to_kvn(
    omm: *const SidereonOmm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_to_kvn",
        SidereonStatus::Panic,
        || {
            omm_encode(
                "sidereon_omm_to_kvn",
                omm,
                out,
                len,
                out_written,
                out_required,
                sidereon_core::astro::omm::encode_kvn,
            )
        },
    )
}

/// Serialize an OMM to XML text (not null-terminated). Round-trips with
/// sidereon_omm_parse_xml. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_xml.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: omm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_to_xml(
    omm: *const SidereonOmm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_to_xml",
        SidereonStatus::Panic,
        || {
            omm_encode(
                "sidereon_omm_to_xml",
                omm,
                out,
                len,
                out_written,
                out_required,
                sidereon_core::astro::omm::encode_xml,
            )
        },
    )
}

/// Serialize an OMM to JSON text (not null-terminated). Round-trips with
/// sidereon_omm_parse_json. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_json.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: omm is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_to_json(
    omm: *const SidereonOmm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_omm_to_json",
        SidereonStatus::Panic,
        || {
            omm_encode(
                "sidereon_omm_to_json",
                omm,
                out,
                len,
                out_written,
                out_required,
                sidereon_core::astro::omm::encode_json,
            )
        },
    )
}

/// Release an OMM handle. Passing NULL is a no-op.
///
/// Safety: omm must be a handle from a sidereon_omm_parse_* call or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_free(omm: *mut SidereonOmm) {
    free_boxed(omm);
}

/// One OMM entry that the lenient build could not resolve to a record for the
/// requested system, read back as a value struct. The object name (when present)
/// is copied separately with sidereon_omm_catalog_skipped_object_name because it
/// is variable length.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSkippedOmm {
    /// True when the entry stated a NORAD_CAT_ID. CCSDS 502.0-B-3 requires it
    /// only for SGP/SGP4 element sets, so an OMM may omit it.
    pub norad_id_present: bool,
    /// The OMM NORAD_CAT_ID of the skipped entry, zero when absent.
    pub norad_id: u32,
    /// True when the entry carried an OBJECT_NAME (retrievable with
    /// sidereon_omm_catalog_skipped_object_name). False means the OMM had no
    /// object name at all, distinct from an empty name.
    pub object_name_present: bool,
}

unsafe fn omm_parse(
    fn_name: &str,
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
    parse: impl FnOnce(
        &str,
    )
        -> Result<sidereon_core::astro::omm::Omm, sidereon_core::astro::omm::OmmError>,
) -> SidereonStatus {
    let out_omm = c_try!(require_out(out_omm, fn_name, "out_omm"));
    let text = require_slice(data, len, fn_name, "data")
        .map(|bytes| str::from_utf8(bytes).map(str::to_owned));
    *out_omm = ptr::null_mut();
    let text = match c_try!(text) {
        Ok(text) => text,
        Err(_) => {
            set_last_error(format!("{fn_name}: data is not valid UTF-8"));
            return SidereonStatus::InvalidToken;
        }
    };
    match parse(&text) {
        Ok(inner) => {
            write_boxed_handle(out_omm, SidereonOmm { inner });
            SidereonStatus::Ok
        }
        Err(err) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Omm,
                fn_name,
                crate::engine_error::omm_error_value(&err),
            );
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        }
    }
}

unsafe fn omm_encode(
    fn_name: &str,
    omm: *const SidereonOmm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    encode: impl FnOnce(
        &sidereon_core::astro::omm::Omm,
    ) -> Result<String, sidereon_core::astro::omm::OmmError>,
) -> SidereonStatus {
    c_try!(reject_output_overlaps_handle(
        fn_name,
        out_written,
        "out_written",
        omm,
        "omm"
    ));
    c_try!(reject_output_overlaps_handle(
        fn_name,
        out_required,
        "out_required",
        omm,
        "omm"
    ));
    c_try!(reject_outputs_overlapping_handle(
        fn_name,
        &[
            (out.cast(), size_of::<u8>(), len, "out"),
            (out_written.cast(), size_of::<usize>(), 1, "out_written"),
            (out_required.cast(), size_of::<usize>(), 1, "out_required"),
        ],
        omm,
        "omm",
    ));
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let omm = c_try!(require_ref(omm, fn_name, "omm"));
    let text = match encode(&omm.inner) {
        Ok(text) => text,
        Err(err) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Omm,
                fn_name,
                crate::engine_error::omm_error_value(&err),
            );
            set_last_error(format!("{fn_name}: {err}"));
            return SidereonStatus::InvalidArgument;
        }
    };
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        text.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

#[cfg(test)]
mod codec_error_tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, snapshot_engine_error_for_test, SidereonEngineErrorFamily,
    };

    fn last_error() -> String {
        let required = unsafe { crate::sidereon_last_error_message(ptr::null_mut(), 0) };
        let mut bytes = vec![0 as std::ffi::c_char; required + 1];
        unsafe {
            crate::sidereon_last_error_message(bytes.as_mut_ptr(), bytes.len());
            std::ffi::CStr::from_ptr(bytes.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }

    fn assert_snapshot_eq(expected: &(crate::SidereonEngineErrorInfo, String)) {
        let (actual_info, actual_payload) =
            snapshot_engine_error_for_test().expect("retained detail");
        assert_eq!(actual_info.family, expected.0.family);
        assert_eq!(actual_info.payload_len, expected.0.payload_len);
        assert_eq!(actual_payload, expected.1);
    }

    fn seed_real_oem_refusal() {
        let duplicate = include_str!("../tests/fixtures/oem/gps.kvn").replace(
            "ORIGINATOR = SIDEREON TEST\n",
            "ORIGINATOR = SIDEREON TEST\nORIGINATOR = OTHER\n",
        );
        let mut out = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_oem_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut out) },
            SidereonStatus::InvalidArgument
        );
    }

    fn read_malformed_record(
        catalog: *const SidereonOmmCatalog,
        index: usize,
        expected_snapshot: Option<&(crate::SidereonEngineErrorInfo, String)>,
    ) -> (usize, serde_json::Value) {
        let mut record_index = usize::MAX;
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_malformed_record(
                    catalog,
                    index,
                    &mut record_index,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        if let Some(expected) = expected_snapshot {
            assert_snapshot_eq(expected);
        }
        let mut canary = vec![0xA5u8; required.saturating_sub(1)];
        if !canary.is_empty() {
            assert_eq!(
                unsafe {
                    sidereon_omm_catalog_malformed_record(
                        catalog,
                        index,
                        &mut record_index,
                        canary.as_mut_ptr(),
                        canary.len(),
                        &mut written,
                        &mut required,
                    )
                },
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, canary.len() + 1);
            assert!(canary.iter().all(|byte| *byte == 0xA5));
            if let Some(expected) = expected_snapshot {
                assert_snapshot_eq(expected);
            }
        }
        let mut output = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_malformed_record(
                    catalog,
                    index,
                    &mut record_index,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        if let Some(expected) = expected_snapshot {
            assert_snapshot_eq(expected);
        }
        (
            record_index,
            serde_json::from_slice(&output[..written]).expect("well-formed skipped-record JSON"),
        )
    }

    #[test]
    fn omm_duplicate_metadata_and_catalog_parser_skips_keep_full_owned_causes() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/omm/24876.kvn");
        let duplicate = source.replacen(
            "MEAN_MOTION    = 2.00563771",
            "MEAN_MOTION    = 2.00563771\nMEAN_MOTION = 2.0057",
            1,
        );
        let mut failed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_omm_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut failed) },
            SidereonStatus::InvalidArgument
        );
        assert!(failed.is_null());
        let (info, payload) = snapshot_engine_error_for_test().expect("OMM parser detail");
        assert_eq!(info.family, SidereonEngineErrorFamily::Omm);
        assert!(payload.contains("\"operation\":\"sidereon_omm_parse_kvn\""));
        assert!(payload.contains("\"kind\":\"duplicate_field\""));
        assert!(payload.contains("\"field\":\"MEAN_MOTION\""));
        assert_eq!(
            last_error(),
            "sidereon_omm_parse_kvn: OMM keyword MEAN_MOTION occurs with different values \"2.00563771\" and \"2.0057\""
        );

        let valid_array = include_str!("../tests/fixtures/omm/25544.json");
        let valid_object = valid_array
            .trim()
            .strip_prefix('[')
            .expect("fixture array starts with bracket")
            .strip_suffix(']')
            .expect("fixture array ends with bracket")
            .to_string();
        let duplicate_json = valid_object.replace(
            "\"NORAD_CAT_ID\":25544",
            "\"NORAD_CAT_ID\":25544,\"NORAD_CAT_ID\":25545",
        );
        let mut failed_json = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_omm_parse_json(
                    duplicate_json.as_ptr(),
                    duplicate_json.len(),
                    &mut failed_json,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let (json_info, json_payload) = snapshot_engine_error_for_test().expect("OMM JSON detail");
        assert_eq!(json_info.family, SidereonEngineErrorFamily::Omm);
        assert!(json_payload.contains("\"operation\":\"sidereon_omm_parse_json\""));
        assert!(json_payload.contains("\"kind\":\"duplicate_field\""));
        assert!(json_payload.contains("\"field\":\"NORAD_CAT_ID\""));
        let feed = format!("[{valid_object},{{}}]");
        let mut catalog = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_build_lenient(
                    SidereonGnssSystem::Gps as u32,
                    feed.as_ptr(),
                    feed.len(),
                    &mut catalog,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        let mut malformed_count = 0;
        assert_eq!(
            unsafe { sidereon_omm_catalog_malformed_count(catalog, &mut malformed_count) },
            SidereonStatus::Ok
        );
        assert_eq!(malformed_count, 1);
        seed_real_oem_refusal();
        let seeded_snapshot = snapshot_engine_error_for_test().expect("genuine OEM refusal");
        let (record_index, detail) = read_malformed_record(catalog, 0, Some(&seeded_snapshot));
        assert_eq!(record_index, 1);
        assert_eq!(
            detail,
            serde_json::json!({
                "index": 1,
                "error": {
                    "kind": "missing_field",
                    "fields": {"field": "EPOCH"}
                }
            })
        );
        let (mut out_of_range_index, mut out_of_range_written, mut out_of_range_required) =
            (usize::MAX, 0, 0);
        let mut out_of_range_payload = [0xA5u8; 8];
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_malformed_record(
                    catalog,
                    1,
                    &mut out_of_range_index,
                    out_of_range_payload.as_mut_ptr(),
                    out_of_range_payload.len(),
                    &mut out_of_range_written,
                    &mut out_of_range_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(out_of_range_written, 0);
        assert!(out_of_range_payload.iter().all(|byte| *byte == 0xA5));
        assert_snapshot_eq(&seeded_snapshot);
        let mut records = 0;
        let mut identity_skips = 0;
        assert_eq!(
            unsafe { sidereon_omm_catalog_record_count(catalog, &mut records) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_omm_catalog_skipped_count(catalog, &mut identity_skips) },
            SidereonStatus::Ok
        );
        assert_eq!(records + identity_skips, 1);

        let mut count = 0;
        assert_eq!(
            unsafe { sidereon_omm_catalog_malformed_count(catalog, &mut count) },
            SidereonStatus::Ok
        );
        assert_eq!(count, 1);
        assert_snapshot_eq(&seeded_snapshot);
        let valid_kvn = include_str!("../tests/fixtures/opm/osprey.kvn");
        let mut unrelated = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(valid_kvn.as_ptr(), valid_kvn.len(), &mut unrelated) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_opm_free(unrelated) };
        let (retained_index, retained_detail) = read_malformed_record(catalog, 0, None);
        assert_eq!(retained_index, 1);
        assert_eq!(retained_detail, detail);

        let duplicate = source.replacen(
            "MEAN_MOTION    = 2.00563771",
            "MEAN_MOTION    = 2.00563771\nMEAN_MOTION = 2.0057",
            1,
        );
        let mut failed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_omm_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut failed) },
            SidereonStatus::InvalidArgument
        );
        let free_snapshot = snapshot_engine_error_for_test().expect("genuine OMM refusal");
        unsafe { sidereon_omm_catalog_free(catalog) };
        assert_snapshot_eq(&free_snapshot);

        seed_real_oem_refusal();
        assert_eq!(
            unsafe { sidereon_omm_parse_json(b"{}".as_ptr(), 2, ptr::null_mut()) },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_real_oem_refusal();
        let fixture = include_str!("../tests/fixtures/omm/25544.json");
        let mut success = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_omm_parse_json(fixture.as_ptr(), fixture.len(), &mut success) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_omm_free(success) };
        clear_engine_error();
    }

    #[test]
    fn omm_xml_and_catalog_refusals_keep_real_typed_errors() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/omm/24876.xml");
        let duplicate = source.replace(
            "<EPOCH>2026-06-16T04:54:23.504544</EPOCH>",
            "<EPOCH>2026-06-16T04:54:23.504544</EPOCH><EPOCH>2026-06-16T04:54:23.504545</EPOCH>",
        );
        let mut failed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_omm_parse_xml(duplicate.as_ptr(), duplicate.len(), &mut failed) },
            SidereonStatus::InvalidArgument
        );
        let (xml_info, xml_payload) =
            snapshot_engine_error_for_test().expect("OMM XML parser refusal");
        assert_eq!(xml_info.family, SidereonEngineErrorFamily::Omm);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&xml_payload).expect("OMM XML payload"),
            serde_json::json!({
                "schema_version": 1,
                "family": "omm",
                "operation": "sidereon_omm_parse_xml",
                "error": {
                    "kind": "duplicate_field",
                    "fields": {
                        "field": "EPOCH",
                        "first": "2026-06-16T04:54:23.504544",
                        "second": "2026-06-16T04:54:23.504545"
                    }
                }
            })
        );

        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_omm_parse_xml(source.as_ptr(), source.len(), &mut parsed) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_omm_free(parsed) };

        let null_json = b"null";
        let mut catalog = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_build_lenient(
                    SidereonGnssSystem::Gps as u32,
                    null_json.as_ptr(),
                    null_json.len(),
                    &mut catalog,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let (catalog_info, catalog_payload) =
            snapshot_engine_error_for_test().expect("top-level catalog parse refusal");
        assert_eq!(catalog_info.family, SidereonEngineErrorFamily::Omm);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&catalog_payload)
                .expect("catalog payload JSON"),
            serde_json::json!({
                "schema_version": 1,
                "family": "omm",
                "operation": "sidereon_omm_catalog_build_lenient",
                "error": {
                    "kind": "field",
                    "fields": {"message": "expected a JSON object or array"}
                }
            })
        );
        assert_eq!(
            last_error(),
            "sidereon_omm_catalog_build_lenient: OMM field error: expected a JSON object or array"
        );

        seed_real_oem_refusal();
        let valid = include_str!("../tests/fixtures/omm/25544.json");
        assert_eq!(
            unsafe {
                sidereon_omm_catalog_build_lenient(
                    SidereonGnssSystem::Gps as u32,
                    valid.as_ptr(),
                    valid.len(),
                    &mut catalog,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_omm_catalog_free(catalog) };
    }

    #[test]
    fn omm_json_line_break_round_trips_but_kvn_writer_refuses_it() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/omm/25544.json");
        let newline_value = "ISS\n(ZARYA)";
        let with_newline = source.replace(
            "\"OBJECT_NAME\":\"ISS (ZARYA)\"",
            "\"OBJECT_NAME\":\"ISS\\n(ZARYA)\"",
        );
        seed_real_oem_refusal();
        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_omm_parse_json(with_newline.as_ptr(), with_newline.len(), &mut parsed)
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let mut output = vec![0u8; 8192];
        let (mut written, mut required) = (0, 0);
        seed_real_oem_refusal();
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(
            unsafe {
                sidereon_omm_to_json(
                    parsed,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(written > 0);
        assert!(snapshot_engine_error_for_test().is_none());
        seed_real_oem_refusal();
        let (mut written, mut required) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                sidereon_omm_to_kvn(
                    parsed,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));
        let (info, payload) = snapshot_engine_error_for_test().expect("OMM KVN writer refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Omm);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("typed OMM payload"),
            serde_json::json!({
                "schema_version": 1,
                "family": "omm",
                "operation": "sidereon_omm_to_kvn",
                "error": {
                    "kind": "unwritable_text",
                    "fields": {
                        "field": "OBJECT_NAME",
                        "value": newline_value,
                        "issue": "line_break"
                    }
                }
            })
        );
        assert_eq!(
            last_error(),
            "sidereon_omm_to_kvn: OMM OBJECT_NAME value \"ISS\\n(ZARYA)\" contains a line break"
        );
        unsafe { sidereon_omm_free(parsed) };
        assert_snapshot_eq(&(info, payload));
    }
}
