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
    ffi_boundary(
        "sidereon_omm_catalog_build_lenient",
        SidereonStatus::Panic,
        || {
            let out_catalog = c_try!(require_out(
                out_catalog,
                "sidereon_omm_catalog_build_lenient",
                "out_catalog"
            ));
            *out_catalog = ptr::null_mut();
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_omm_catalog_build_lenient",
                "system",
                system
            ));
            let omm_bytes = c_try!(require_slice(
                omm_json,
                omm_len,
                "sidereon_omm_catalog_build_lenient",
                "omm_json"
            ));
            let omm_text = match str::from_utf8(omm_bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_omm_catalog_build_lenient: omm_json is not valid UTF-8"
                            .to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let omm_array = match parse_omm_json_array(omm_text) {
                Ok(omm_array) => omm_array,
                Err(err) => {
                    set_last_error(format!("sidereon_omm_catalog_build_lenient: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let catalog = from_celestrak_omm_lenient(system, &omm_array.omms);
            write_boxed_handle(
                out_catalog,
                SidereonOmmCatalog {
                    inner: catalog,
                    malformed: omm_array.skipped,
                },
            );
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
            *out = SidereonSkippedOmm {
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
                norad_id: skipped.norad_id,
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
    ffi_boundary("sidereon_omm_parse_kvn", SidereonStatus::Panic, || {
        omm_parse(
            "sidereon_omm_parse_kvn",
            data,
            len,
            out_omm,
            sidereon_core::astro::omm::parse_kvn,
        )
    })
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
    ffi_boundary("sidereon_omm_parse_xml", SidereonStatus::Panic, || {
        omm_parse(
            "sidereon_omm_parse_xml",
            data,
            len,
            out_omm,
            sidereon_core::astro::omm::parse_xml,
        )
    })
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
    ffi_boundary("sidereon_omm_parse_json", SidereonStatus::Panic, || {
        omm_parse(
            "sidereon_omm_parse_json",
            data,
            len,
            out_omm,
            sidereon_core::astro::omm::parse_json,
        )
    })
}

/// Serialize an OMM to KVN text (not null-terminated). Round-trips with
/// sidereon_omm_parse_kvn. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_kvn.
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
    ffi_boundary("sidereon_omm_to_kvn", SidereonStatus::Panic, || {
        omm_encode(
            "sidereon_omm_to_kvn",
            omm,
            out,
            len,
            out_written,
            out_required,
            sidereon_core::astro::omm::encode_kvn,
        )
    })
}

/// Serialize an OMM to XML text (not null-terminated). Round-trips with
/// sidereon_omm_parse_xml. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_xml.
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
    ffi_boundary("sidereon_omm_to_xml", SidereonStatus::Panic, || {
        omm_encode(
            "sidereon_omm_to_xml",
            omm,
            out,
            len,
            out_written,
            out_required,
            sidereon_core::astro::omm::encode_xml,
        )
    })
}

/// Serialize an OMM to JSON text (not null-terminated). Round-trips with
/// sidereon_omm_parse_json. Variable-length output contract. Delegates to
/// sidereon_core::astro::omm::encode_json.
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
    ffi_boundary("sidereon_omm_to_json", SidereonStatus::Panic, || {
        omm_encode(
            "sidereon_omm_to_json",
            omm,
            out,
            len,
            out_written,
            out_required,
            sidereon_core::astro::omm::encode_json,
        )
    })
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
    /// The OMM NORAD_CAT_ID of the skipped entry.
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
    *out_omm = ptr::null_mut();
    let bytes = c_try!(require_slice(data, len, fn_name, "data"));
    let text = match str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            set_last_error(format!("{fn_name}: data is not valid UTF-8"));
            return SidereonStatus::InvalidToken;
        }
    };
    match parse(text) {
        Ok(inner) => {
            write_boxed_handle(out_omm, SidereonOmm { inner });
            SidereonStatus::Ok
        }
        Err(err) => {
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
    encode: impl FnOnce(&sidereon_core::astro::omm::Omm) -> String,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let omm = c_try!(require_ref(omm, fn_name, "omm"));
    let text = encode(&omm.inner);
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
