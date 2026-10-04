use super::*;

/// A parsed CCSDS OEM (Orbit Ephemeris Message). Opaque to C. Create with
/// sidereon_oem_parse_kvn or sidereon_oem_parse_xml; serialize with
/// sidereon_oem_to_kvn or sidereon_oem_to_xml; release with sidereon_oem_free.
pub struct SidereonOem {
    pub(crate) inner: Oem,
}

/// Parse a CCSDS OEM in KVN (keyword=value) form. On success writes a newly
/// owned handle to *out_oem. Release it with sidereon_oem_free.
///
/// Safety: data must point to len readable bytes; out_oem must point to storage
/// for a SidereonOem*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_parse_kvn(
    data: *const u8,
    len: usize,
    out_oem: *mut *mut SidereonOem,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_oem_parse_kvn",
        SidereonStatus::Panic,
        || {
            let out_oem = c_try!(require_out(out_oem, "sidereon_oem_parse_kvn", "out_oem"));
            *out_oem = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_oem_parse_kvn"));
            let inner = match core_oem::parse_kvn(text) {
                Ok(oem) => oem,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Oem,
                        "sidereon_oem_parse_kvn",
                        crate::engine_error::oem_error_value(&err),
                    );
                    set_last_error(format!("sidereon_oem_parse_kvn: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_oem, SidereonOem { inner });
            SidereonStatus::Ok
        },
    )
}

/// Parse a CCSDS OEM in XML (NDM/XML) form. On success writes a newly owned
/// handle to *out_oem. Release it with sidereon_oem_free.
///
/// Safety: data must point to len readable bytes; out_oem must point to storage
/// for a SidereonOem*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_parse_xml(
    data: *const u8,
    len: usize,
    out_oem: *mut *mut SidereonOem,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_oem_parse_xml",
        SidereonStatus::Panic,
        || {
            let out_oem = c_try!(require_out(out_oem, "sidereon_oem_parse_xml", "out_oem"));
            *out_oem = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_oem_parse_xml"));
            let inner = match core_oem::parse_xml(text) {
                Ok(oem) => oem,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Oem,
                        "sidereon_oem_parse_xml",
                        crate::engine_error::oem_error_value(&err),
                    );
                    set_last_error(format!("sidereon_oem_parse_xml: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_oem, SidereonOem { inner });
            SidereonStatus::Ok
        },
    )
}

/// Write the number of metadata/data segments in the OEM to *out_count.
///
/// Safety: oem must be a live handle from a sidereon_oem_parse_* call; out_count
/// must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_segment_count(
    oem: *const SidereonOem,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_segment_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_oem_segment_count",
            "out_count"
        ));
        *out_count = 0;
        let oem = c_try!(require_ref(oem, "sidereon_oem_segment_count", "oem"));
        *out_count = oem.inner.segments.len();
        SidereonStatus::Ok
    })
}

/// Write the number of malformed KVN state lines retained by the forgiving
/// OEM parser. XML parsing refuses malformed states and has no skipped lines.
///
/// Safety: oem is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_skipped_state_count(
    oem: *const SidereonOem,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_oem_skipped_state_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_oem_skipped_state_count",
                "out_count"
            ));
            *out_count = 0;
            let oem = c_try!(require_ref(oem, "sidereon_oem_skipped_state_count", "oem"));
            *out_count = oem.inner.skipped_states.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one retained malformed OEM state line as JSON detail, preserving its
/// one-based source line, zero-based segment index, trimmed original text, and
/// full typed parse reason. Uses the standard two-pass byte-copy contract.
///
/// Safety: oem is a live handle; out and size pointers follow the documented
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_skipped_state(
    oem: *const SidereonOem,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_oem_skipped_state", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_oem_skipped_state",
            out_written,
            out_required
        ));
        let oem = c_try!(require_ref(oem, "sidereon_oem_skipped_state", "oem"));
        let Some(skipped) = oem.inner.skipped_states.get(index) else {
            set_last_error(format!(
                "sidereon_oem_skipped_state: index {index} out of range ({} entries)",
                oem.inner.skipped_states.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        let detail = serde_json::json!({
            "line": skipped.line,
            "segment": skipped.segment,
            "text": skipped.text,
            "reason": crate::engine_error::oem_state_line_error_value(&skipped.reason),
        })
        .to_string();
        c_try!(copy_prefix_to_c(
            "sidereon_oem_skipped_state",
            "out",
            detail.as_bytes(),
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Serialize an OEM to KVN text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_oem_parse_kvn.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: oem must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_to_kvn(
    oem: *const SidereonOem,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_oem_to_kvn",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_oem_to_kvn",
                out_written,
                out_required
            ));
            let oem = c_try!(require_ref(oem, "sidereon_oem_to_kvn", "oem"));
            let text = match core_oem::encode_kvn(&oem.inner) {
                Ok(text) => text,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Oem,
                        "sidereon_oem_to_kvn",
                        crate::engine_error::oem_error_value(&err),
                    );
                    set_last_error(format!("sidereon_oem_to_kvn: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_oem_to_kvn",
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

/// Serialize an OEM to XML text. The output is not null-terminated. Uses the
/// variable-length output contract documented at the top of the header: call once
/// with out=NULL to learn *out_required, then again with a buffer of that size.
/// Round-trips with sidereon_oem_parse_xml.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT when the writer refuses the
/// message, naming the item it cannot write.
///
/// Safety: oem must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_to_xml(
    oem: *const SidereonOem,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_oem_to_xml",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_oem_to_xml",
                out_written,
                out_required
            ));
            let oem = c_try!(require_ref(oem, "sidereon_oem_to_xml", "oem"));
            let text = match core_oem::encode_xml(&oem.inner) {
                Ok(text) => text,
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Oem,
                        "sidereon_oem_to_xml",
                        crate::engine_error::oem_error_value(&err),
                    );
                    set_last_error(format!("sidereon_oem_to_xml: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_oem_to_xml",
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

/// Release an OEM handle from a sidereon_oem_parse_* call. Passing NULL is a
/// no-op.
///
/// Safety: oem must be NULL or a live handle that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_oem_free(oem: *mut SidereonOem) {
    ffi_boundary("sidereon_oem_free", (), || {
        free_boxed(oem);
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

    fn seed_real_opm_refusal() {
        let duplicate = include_str!("../tests/fixtures/opm/osprey.kvn")
            .replace("X = 6878.137\n", "X = 6878.137\nX = 6879.137\n");
        let mut out = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_opm_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut out) },
            SidereonStatus::InvalidArgument
        );
    }

    fn read_skipped_state(
        oem: *const SidereonOem,
        expected_snapshot: Option<&(crate::SidereonEngineErrorInfo, String)>,
    ) -> serde_json::Value {
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_oem_skipped_state(oem, 0, ptr::null_mut(), 0, &mut written, &mut required)
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        if let Some(expected) = expected_snapshot {
            assert_snapshot_eq(expected);
        }
        let mut canary = vec![0xA5u8; required - 1];
        assert_eq!(
            unsafe {
                sidereon_oem_skipped_state(
                    oem,
                    0,
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
        let mut bytes = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_oem_skipped_state(
                    oem,
                    0,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        if let Some(expected) = expected_snapshot {
            assert_snapshot_eq(expected);
        }
        serde_json::from_slice(&bytes[..written]).expect("well-formed OEM departure JSON")
    }

    #[test]
    fn oem_duplicate_metadata_records_typed_error_and_skipped_lines_are_owned() {
        clear_engine_error();
        let duplicate = include_str!("../tests/fixtures/oem/gps.kvn").replace(
            "ORIGINATOR = SIDEREON TEST\n",
            "ORIGINATOR = SIDEREON TEST\nORIGINATOR = OTHER\n",
        );
        let mut failed = ptr::null_mut();
        let status =
            unsafe { sidereon_oem_parse_kvn(duplicate.as_ptr(), duplicate.len(), &mut failed) };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(failed.is_null());
        let (info, payload) = snapshot_engine_error_for_test().expect("OEM parser detail");
        assert_eq!(info.family, SidereonEngineErrorFamily::Oem);
        assert!(payload.contains("\"operation\":\"sidereon_oem_parse_kvn\""));
        assert!(payload.contains("\"kind\":\"duplicate_field\""));
        assert!(payload.contains("\"first\":\"SIDEREON TEST\""));
        assert!(payload.contains("\"second\":\"OTHER\""));
        assert_eq!(
            last_error(),
            "sidereon_oem_parse_kvn: OEM keyword ORIGINATOR occurs with different values \"SIDEREON TEST\" and \"OTHER\""
        );

        let duplicate_xml = include_str!("../tests/fixtures/oem/gps.xml").replace(
            "<ORIGINATOR>SIDEREON TEST</ORIGINATOR>",
            "<ORIGINATOR>SIDEREON TEST</ORIGINATOR><ORIGINATOR>OTHER</ORIGINATOR>",
        );
        let mut failed_xml = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_oem_parse_xml(duplicate_xml.as_ptr(), duplicate_xml.len(), &mut failed_xml)
            },
            SidereonStatus::InvalidArgument
        );
        let (xml_info, xml_payload) = snapshot_engine_error_for_test().expect("OEM XML detail");
        assert_eq!(xml_info.family, SidereonEngineErrorFamily::Oem);
        assert!(xml_payload.contains("\"operation\":\"sidereon_oem_parse_xml\""));
        assert!(xml_payload.contains("\"kind\":\"duplicate_field\""));

        let valid_with_skip = include_str!("../tests/fixtures/oem/gps.kvn").replace(
            "2026-06-28T00:15:00.000 17450",
            "2026-06-28T00:07:30.000 1 2\n2026-06-28T00:15:00.000 17450",
        );
        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_oem_parse_kvn(valid_with_skip.as_ptr(), valid_with_skip.len(), &mut parsed)
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_real_opm_refusal();
        let seeded_snapshot = snapshot_engine_error_for_test().expect("genuine OPM refusal");
        let mut count = 0;
        assert_eq!(
            unsafe { sidereon_oem_skipped_state_count(parsed, &mut count) },
            SidereonStatus::Ok
        );
        assert_eq!(count, 1);
        assert_snapshot_eq(&seeded_snapshot);
        let detail = read_skipped_state(parsed, Some(&seeded_snapshot));
        assert_eq!(
            detail,
            serde_json::json!({
                "line": 22,
                "segment": 0,
                "text": "2026-06-28T00:07:30.000 1 2",
                "reason": {"kind": "item_count", "fields": {"found": 3}}
            })
        );
        let mut out_of_range_written = 0;
        let mut out_of_range_required = 0;
        let mut out_of_range = [0xA5u8; 8];
        assert_eq!(
            unsafe {
                sidereon_oem_skipped_state(
                    parsed,
                    1,
                    out_of_range.as_mut_ptr(),
                    out_of_range.len(),
                    &mut out_of_range_written,
                    &mut out_of_range_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(out_of_range_written, 0);
        assert!(out_of_range.iter().all(|byte| *byte == 0xA5));
        assert_snapshot_eq(&seeded_snapshot);

        let valid_opm = include_str!("../tests/fixtures/opm/osprey.kvn");
        let mut unrelated_success = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_opm_parse_kvn(valid_opm.as_ptr(), valid_opm.len(), &mut unrelated_success)
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_opm_free(unrelated_success) };
        assert_eq!(read_skipped_state(parsed, None), detail);

        seed_real_opm_refusal();
        let free_snapshot =
            snapshot_engine_error_for_test().expect("genuine OPM refusal before free");
        unsafe { sidereon_oem_free(parsed) };
        assert_snapshot_eq(&free_snapshot);

        seed_real_opm_refusal();
        let input = b"bad";
        assert_eq!(
            unsafe { sidereon_oem_parse_kvn(input.as_ptr(), input.len(), ptr::null_mut()) },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        seed_real_opm_refusal();
        let mut success = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_oem_parse_kvn(
                    valid_with_skip.as_ptr(),
                    valid_with_skip.len(),
                    &mut success,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_oem_free(success) };
        clear_engine_error();
    }

    #[test]
    fn oem_xml_text_round_trips_but_kvn_writer_records_line_break_refusal() {
        clear_engine_error();
        let source = include_str!("../tests/fixtures/oem/gps.xml");
        seed_real_opm_refusal();
        let mut valid = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_oem_parse_xml(source.as_ptr(), source.len(), &mut valid) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        let mut output = vec![0u8; 16384];
        let (mut written, mut required) = (0, 0);
        seed_real_opm_refusal();
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(
            unsafe {
                sidereon_oem_to_kvn(
                    valid,
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
        unsafe { sidereon_oem_free(valid) };

        let value = "GPS\nBIIRM-8";
        let with_newline = source.replace(
            "<OBJECT_NAME>GPS BIIRM-8</OBJECT_NAME>",
            "<OBJECT_NAME>GPS\nBIIRM-8</OBJECT_NAME>",
        );
        seed_real_opm_refusal();
        let mut parsed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_oem_parse_xml(with_newline.as_ptr(), with_newline.len(), &mut parsed)
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        let (mut written, mut required) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                sidereon_oem_to_kvn(
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
        let (info, payload) = snapshot_engine_error_for_test().expect("OEM writer refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Oem);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("typed payload JSON"),
            serde_json::json!({
                "schema_version": 1,
                "family": "oem",
                "operation": "sidereon_oem_to_kvn",
                "error": {
                    "kind": "unwritable_text",
                    "fields": {
                        "field": "OBJECT_NAME",
                        "value": value,
                        "issue": "line_break"
                    }
                }
            })
        );
        assert_eq!(
            last_error(),
            "sidereon_oem_to_kvn: OEM OBJECT_NAME value \"GPS\\nBIIRM-8\" contains a line break"
        );
        unsafe { sidereon_oem_free(parsed) };
        assert_snapshot_eq(&(info, payload));
    }
}
