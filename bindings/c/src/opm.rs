use super::*;

/// A parsed CCSDS OPM (Orbit Parameter Message). Opaque to C. Create with
/// sidereon_opm_parse_kvn or sidereon_opm_parse_xml; serialize with
/// sidereon_opm_to_kvn or sidereon_opm_to_xml; release with sidereon_opm_free.
pub struct SidereonOpm {
    pub(crate) inner: Opm,
}

/// Parse a CCSDS OPM in KVN (keyword=value) form. On success writes a newly
/// owned handle to *out_opm. Release it with sidereon_opm_free.
///
/// Safety: data must point to len readable bytes; out_opm must point to storage
/// for a SidereonOpm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_parse_kvn(
    data: *const u8,
    len: usize,
    out_opm: *mut *mut SidereonOpm,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_opm_parse_kvn",
        SidereonStatus::Panic,
        || {
            let out_opm = c_try!(require_out(out_opm, "sidereon_opm_parse_kvn", "out_opm"));
            *out_opm = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_opm_parse_kvn"));
            let inner = match core_opm::parse_kvn(text) {
                Ok(opm) => opm,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Opm,
                        "sidereon_opm_parse_kvn",
                        crate::engine_error::opm_error_value(&err),
                    );
                    set_last_error(format!("sidereon_opm_parse_kvn: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_opm, SidereonOpm { inner });
            SidereonStatus::Ok
        },
    )
}

/// Parse a CCSDS OPM in XML (NDM/XML) form. On success writes a newly owned
/// handle to *out_opm. Release it with sidereon_opm_free.
///
/// Safety: data must point to len readable bytes; out_opm must point to storage
/// for a SidereonOpm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_parse_xml(
    data: *const u8,
    len: usize,
    out_opm: *mut *mut SidereonOpm,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_opm_parse_xml",
        SidereonStatus::Panic,
        || {
            let out_opm = c_try!(require_out(out_opm, "sidereon_opm_parse_xml", "out_opm"));
            *out_opm = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_opm_parse_xml"));
            let inner = match core_opm::parse_xml(text) {
                Ok(opm) => opm,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Opm,
                        "sidereon_opm_parse_xml",
                        crate::engine_error::opm_error_value(&err),
                    );
                    set_last_error(format!("sidereon_opm_parse_xml: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_opm, SidereonOpm { inner });
            SidereonStatus::Ok
        },
    )
}

/// Serialize an OPM to KVN text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_opm_parse_kvn.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: opm must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_to_kvn(
    opm: *const SidereonOpm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_opm_to_kvn",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_opm_to_kvn",
                out_written,
                out_required
            ));
            let opm = c_try!(require_ref(opm, "sidereon_opm_to_kvn", "opm"));
            let text = match core_opm::encode_kvn(&opm.inner) {
                Ok(text) => text,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Opm,
                        "sidereon_opm_to_kvn",
                        crate::engine_error::opm_error_value(&err),
                    );
                    set_last_error(format!("sidereon_opm_to_kvn: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_opm_to_kvn",
                "out",
                text.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Serialize an OPM to XML text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_opm_parse_xml.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: opm must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_to_xml(
    opm: *const SidereonOpm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_opm_to_xml",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_opm_to_xml",
                out_written,
                out_required
            ));
            let opm = c_try!(require_ref(opm, "sidereon_opm_to_xml", "opm"));
            let text = match core_opm::encode_xml(&opm.inner) {
                Ok(text) => text,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Opm,
                        "sidereon_opm_to_xml",
                        crate::engine_error::opm_error_value(&err),
                    );
                    set_last_error(format!("sidereon_opm_to_xml: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_opm_to_xml",
                "out",
                text.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release an OPM handle from a sidereon_opm_parse_* call. Passing NULL is a
/// no-op.
///
/// Safety: opm must be NULL or a live handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_opm_free(opm: *mut SidereonOpm) {
    ffi_boundary("sidereon_opm_free", (), || {
        free_boxed(opm);
    });
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

    #[test]
    fn opm_duplicate_metadata_records_full_variant_and_resets_on_success_and_bad_output() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/opm/osprey.kvn");
        let duplicate = source.replace("X = 6878.137\n", "X = 6878.137\nX = 6879.137\n");
        let mut failed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut failed) },
            SidereonStatus::InvalidArgument
        );
        assert!(failed.is_null());
        let (info, payload) = snapshot_engine_error_for_test().expect("OPM parser detail");
        assert_eq!(info.family, SidereonEngineErrorFamily::Opm);
        assert!(payload.contains("\"operation\":\"sidereon_opm_parse_kvn\""));
        assert!(payload.contains("\"kind\":\"duplicate_field\""));
        assert!(payload.contains("\"field\":\"X\""));
        assert!(payload.contains("\"first\":\"6878.137\""));
        assert!(payload.contains("\"second\":\"6879.137\""));
        assert_eq!(
            last_error(),
            "sidereon_opm_parse_kvn: OPM keyword X occurs with different values \"6878.137\" and \"6879.137\""
        );

        let xml = include_str!("../tests/fixtures/opm/osprey.xml")
            .replace("<X>6878.137</X>", "<X>6878.137</X><X>6879.137</X>");
        let mut failed_xml = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_xml(xml.as_ptr(), xml.len(), &mut failed_xml) },
            SidereonStatus::InvalidArgument
        );
        let (xml_info, xml_payload) = snapshot_engine_error_for_test().expect("OPM XML detail");
        assert_eq!(xml_info.family, SidereonEngineErrorFamily::Opm);
        assert!(xml_payload.contains("\"operation\":\"sidereon_opm_parse_xml\""));
        assert!(xml_payload.contains("\"kind\":\"duplicate_field\""));

        let good = include_str!("../tests/fixtures/opm/osprey.kvn");
        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(good.as_ptr(), good.len(), &mut parsed) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_opm_free(parsed) };

        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut failed) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(good.as_ptr(), good.len(), ptr::null_mut()) },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_real_oem_refusal();
        let mut success = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(good.as_ptr(), good.len(), &mut success) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_opm_free(success) };
    }

    #[test]
    fn opm_xml_text_round_trips_but_kvn_writer_records_line_break_refusal() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/opm/osprey.xml");
        seed_real_oem_refusal();
        let mut valid_handle = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_xml(source.as_ptr(), source.len(), &mut valid_handle) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        let mut output = vec![0u8; 8192];
        let (mut written, mut required) = (0, 0);
        seed_real_oem_refusal();
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(
            unsafe {
                sidereon_opm_to_kvn(
                    valid_handle,
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
        unsafe { sidereon_opm_free(valid_handle) };

        let newline_value = "OSPREY\n-1";
        let with_newline = source.replace(
            "<OBJECT_NAME>OSPREY-1</OBJECT_NAME>",
            "<OBJECT_NAME>OSPREY\n-1</OBJECT_NAME>",
        );
        seed_real_oem_refusal();
        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_opm_parse_xml(with_newline.as_ptr(), with_newline.len(), &mut parsed)
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        let (mut written, mut required) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                sidereon_opm_to_kvn(
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
        let (info, payload) = snapshot_engine_error_for_test().expect("OPM writer refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Opm);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("typed payload JSON"),
            serde_json::json!({
                "schema_version": 1,
                "family": "opm",
                "operation": "sidereon_opm_to_kvn",
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
            "sidereon_opm_to_kvn: OPM OBJECT_NAME value \"OSPREY\\n-1\" contains a line break"
        );
        unsafe { sidereon_opm_free(parsed) };
        assert_snapshot_eq(&(info, payload));
    }
}

// ===========================================================================
// SP3-backed geometry: visible / visibility_series / passes.
//
// These delegate to sidereon_core::geometry::{visible, visibility_series,
// passes} with the loaded SP3 product as the ObservableEphemerisSource and the
// product's own satellite list. No geometry, weighting, or elevation algebra
// lives here; the binding only marshals options and copies results.
