use super::*;

/// Detached result of parsing multiple OMM records. Successful records and
/// skipped records retain their independent input order.
pub struct SidereonOmmArray {
    pub(crate) inner: sidereon_core::astro::omm::OmmArray,
}

/// Civil OMM epoch, including sub-microsecond precision.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SidereonOmmEpoch {
    /// Signed civil year.
    pub year: i32,
    /// Civil month, day, hour, minute, and second.
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// Fractional second split into whole microseconds and femtosecond remainder.
    pub microsecond: u32,
    pub femtosecond: u32,
}

unsafe fn validate_array_copy_outputs(
    fn_name: &str,
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let outputs = [
        (out.cast(), size_of::<u8>(), len, "out"),
        (out_written.cast(), size_of::<usize>(), 1, "out_written"),
        (out_required.cast(), size_of::<usize>(), 1, "out_required"),
    ];
    reject_outputs_overlapping_handle(fn_name, &outputs, array, "array")?;
    let written = checked_output_range(fn_name, out_written, 1, "out_written")?;
    let required = checked_output_range(fn_name, out_required, 1, "out_required")?;
    reject_overlapping_outputs(fn_name, written, required, "out_written", "out_required")?;
    if !out.is_null() && len != 0 {
        let bytes = checked_output_range(fn_name, out, len, "out")?;
        reject_overlapping_outputs(fn_name, bytes, written, "out", "out_written")?;
        reject_overlapping_outputs(fn_name, bytes, required, "out", "out_required")?;
    }
    Ok(())
}

unsafe fn parse_single(
    fn_name: &str,
    data: *const u8,
    len: usize,
    out: *mut *mut SidereonOmm,
    parse: impl FnOnce(
        &str,
    )
        -> Result<sidereon_core::astro::omm::Omm, sidereon_core::astro::omm::OmmError>,
) -> SidereonStatus {
    let out = c_try!(require_out(out, fn_name, "out_omm"));
    let output_range = checked_output_range(fn_name, out, 1, "out_omm");
    let data_range = if len != 0 && !data.is_null() {
        Some(checked_output_range(fn_name, data.cast_mut(), len, "data"))
    } else {
        None
    };
    let output_range = c_try!(output_range);
    if let Some(data_range) = data_range {
        c_try!(reject_overlapping_outputs(
            fn_name,
            c_try!(data_range),
            output_range,
            "data",
            "out_omm"
        ));
    }
    *out = ptr::null_mut();
    let text = match require_slice(data, len, fn_name, "data").and_then(|bytes| {
        str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
            set_last_error(format!("{fn_name}: data is not valid UTF-8"));
            SidereonStatus::InvalidToken
        })
    }) {
        Ok(text) => text,
        Err(status) => return status,
    };
    match parse(&text) {
        Ok(inner) => {
            write_boxed_handle(out, SidereonOmm { inner });
            SidereonStatus::Ok
        }
        Err(error) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Omm,
                fn_name,
                crate::engine_error::omm_error_value(&error),
            );
            set_last_error(format!("{fn_name}: {error}"));
            SidereonStatus::InvalidArgument
        }
    }
}

unsafe fn parse_array(
    fn_name: &str,
    data: *const u8,
    len: usize,
    out: *mut *mut SidereonOmmArray,
    parse: impl FnOnce(
        &str,
    ) -> Result<
        sidereon_core::astro::omm::OmmArray,
        sidereon_core::astro::omm::OmmError,
    >,
) -> SidereonStatus {
    let out = c_try!(require_out(out, fn_name, "out_array"));
    let output_range = checked_output_range(fn_name, out, 1, "out_array");
    let data_range = if len != 0 && !data.is_null() {
        Some(checked_output_range(fn_name, data.cast_mut(), len, "data"))
    } else {
        None
    };
    let output_range = c_try!(output_range);
    if let Some(data_range) = data_range {
        c_try!(reject_overlapping_outputs(
            fn_name,
            c_try!(data_range),
            output_range,
            "data",
            "out_array"
        ));
    }
    *out = ptr::null_mut();
    let text = match require_slice(data, len, fn_name, "data").and_then(|bytes| {
        str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
            set_last_error(format!("{fn_name}: data is not valid UTF-8"));
            SidereonStatus::InvalidToken
        })
    }) {
        Ok(text) => text,
        Err(status) => return status,
    };
    match parse(&text) {
        Ok(inner) => {
            write_boxed_handle(out, SidereonOmmArray { inner });
            SidereonStatus::Ok
        }
        Err(error) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Omm,
                fn_name,
                crate::engine_error::omm_error_value(&error),
            );
            set_last_error(format!("{fn_name}: {error}"));
            SidereonStatus::InvalidArgument
        }
    }
}

/// Parse one OMM using the core format autodetector.
///
/// Safety: data points to len readable bytes; out_omm points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse(
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_parse", SidereonStatus::Panic, || {
        parse_single(
            "sidereon_omm_parse",
            data,
            len,
            out_omm,
            sidereon_core::astro::omm::parse,
        )
    })
}

/// Parse one OMM from GP CSV.
///
/// Safety: data points to len readable bytes; out_omm points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_csv(
    data: *const u8,
    len: usize,
    out_omm: *mut *mut SidereonOmm,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_parse_csv", SidereonStatus::Panic, || {
        parse_single(
            "sidereon_omm_parse_csv",
            data,
            len,
            out_omm,
            sidereon_core::astro::omm::parse_csv,
        )
    })
}

/// Parse every OMM message in an XML document, retaining per-message refusals.
///
/// Safety: data points to len readable bytes; out_array points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_xml_all(
    data: *const u8,
    len: usize,
    out_array: *mut *mut SidereonOmmArray,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_parse_xml_all", SidereonStatus::Panic, || {
        parse_array(
            "sidereon_omm_parse_xml_all",
            data,
            len,
            out_array,
            sidereon_core::astro::omm::parse_xml_all,
        )
    })
}

/// Parse a GP JSON array (or one object), retaining malformed element details.
///
/// Safety: data points to len readable bytes; out_array points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_json_array(
    data: *const u8,
    len: usize,
    out_array: *mut *mut SidereonOmmArray,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_parse_json_array",
        SidereonStatus::Panic,
        || {
            parse_array(
                "sidereon_omm_parse_json_array",
                data,
                len,
                out_array,
                sidereon_core::astro::omm::parse_json_array,
            )
        },
    )
}

/// Parse a GP CSV table, retaining malformed row details.
///
/// Safety: data points to len readable bytes; out_array points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_csv_array(
    data: *const u8,
    len: usize,
    out_array: *mut *mut SidereonOmmArray,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_parse_csv_array",
        SidereonStatus::Panic,
        || {
            parse_array(
                "sidereon_omm_parse_csv_array",
                data,
                len,
                out_array,
                sidereon_core::astro::omm::parse_csv_array,
            )
        },
    )
}

/// Parse a civil OMM epoch without requiring a full message.
///
/// Safety: text points to len readable bytes; out points to writable epoch storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_parse_epoch(
    text: *const u8,
    len: usize,
    out: *mut SidereonOmmEpoch,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_parse_epoch", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_omm_parse_epoch", "out"));
        let output_range = checked_output_range("sidereon_omm_parse_epoch", out, 1, "out");
        let input_range = if len != 0 && !text.is_null() {
            Some(checked_output_range(
                "sidereon_omm_parse_epoch",
                text.cast_mut(),
                len,
                "text",
            ))
        } else {
            None
        };
        let output_range = c_try!(output_range);
        if let Some(input_range) = input_range {
            c_try!(reject_overlapping_outputs(
                "sidereon_omm_parse_epoch",
                c_try!(input_range),
                output_range,
                "text",
                "out"
            ));
        }
        *out = SidereonOmmEpoch::default();
        let bytes = c_try!(require_slice(text, len, "sidereon_omm_parse_epoch", "text"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_omm_parse_epoch: text is not valid UTF-8");
                return SidereonStatus::InvalidToken;
            }
        };
        match sidereon_core::astro::omm::parse_epoch(text) {
            Ok(value) => {
                *out = SidereonOmmEpoch {
                    year: value.year,
                    month: value.month,
                    day: value.day,
                    hour: value.hour,
                    minute: value.minute,
                    second: value.second,
                    microsecond: value.microsecond,
                    femtosecond: value.femtosecond,
                };
                SidereonStatus::Ok
            }
            Err(error) => {
                crate::engine_error::record_engine_error(
                    crate::engine_error::SidereonEngineErrorFamily::Omm,
                    "sidereon_omm_parse_epoch",
                    crate::engine_error::omm_error_value(&error),
                );
                set_last_error(format!("sidereon_omm_parse_epoch: {error}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Create an empty owned OMM collection for caller-assembled writer input.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_new(out: *mut *mut SidereonOmmArray) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_omm_array_new", "out"));
        *out = ptr::null_mut();
        write_boxed_handle(
            out,
            SidereonOmmArray {
                inner: sidereon_core::astro::omm::OmmArray {
                    omms: Vec::new(),
                    skipped: Vec::new(),
                },
            },
        );
        SidereonStatus::Ok
    })
}

/// Append a clone of one OMM to an owned collection.
///
/// Safety: array and omm are live handles returned by the OMM API.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_push(
    array: *mut SidereonOmmArray,
    omm: *const SidereonOmm,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_push", SidereonStatus::Panic, || {
        let array_range = checked_output_range("sidereon_omm_array_push", array, 1, "array");
        let omm_range = checked_output_range("sidereon_omm_array_push", omm.cast_mut(), 1, "omm");
        c_try!(reject_overlapping_outputs(
            "sidereon_omm_array_push",
            c_try!(array_range),
            c_try!(omm_range),
            "array",
            "omm"
        ));
        let array = c_try!(require_mut(array, "sidereon_omm_array_push", "array"));
        let omm = c_try!(require_ref(omm, "sidereon_omm_array_push", "omm"));
        array.inner.omms.push(omm.inner.clone());
        SidereonStatus::Ok
    })
}

/// Return the number of successfully parsed or appended records.
///
/// Safety: array is a live collection handle; out points to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_count(
    array: *const SidereonOmmArray,
    out: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_count", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_omm_array_count", "out"));
        c_try!(reject_output_overlaps_handle(
            "sidereon_omm_array_count",
            out,
            "out",
            array,
            "array"
        ));
        *out = 0;
        let array = c_try!(require_ref(array, "sidereon_omm_array_count", "array"));
        *out = array.inner.omms.len();
        SidereonStatus::Ok
    })
}

/// Copy one successful record into a new independently owned OMM handle.
///
/// Safety: array is live; out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_record(
    array: *const SidereonOmmArray,
    index: usize,
    out: *mut *mut SidereonOmm,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_record", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_omm_array_record", "out"));
        c_try!(reject_output_overlaps_handle(
            "sidereon_omm_array_record",
            out,
            "out",
            array,
            "array"
        ));
        *out = ptr::null_mut();
        let array = c_try!(require_ref(array, "sidereon_omm_array_record", "array"));
        let Some(value) = array.inner.omms.get(index) else {
            set_last_error(format!(
                "sidereon_omm_array_record: index {index} out of range (count {})",
                array.inner.omms.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(
            out,
            SidereonOmm {
                inner: value.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Return the number of records that were skipped by a parser.
///
/// Safety: array is a live collection handle; out points to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_skipped_count(
    array: *const SidereonOmmArray,
    out: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_array_skipped_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_omm_array_skipped_count", "out"));
            c_try!(reject_output_overlaps_handle(
                "sidereon_omm_array_skipped_count",
                out,
                "out",
                array,
                "array"
            ));
            *out = 0;
            let array = c_try!(require_ref(
                array,
                "sidereon_omm_array_skipped_count",
                "array"
            ));
            *out = array.inner.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped record's original index and recursively typed OMM error JSON.
///
/// Safety: array is live; out_index/out_written/out_required point to writable size_t storage;
/// out points to len writable bytes or is null when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_skipped(
    array: *const SidereonOmmArray,
    index: usize,
    out_index: *mut usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_skipped", SidereonStatus::Panic, || {
        c_try!(validate_array_copy_outputs(
            "sidereon_omm_array_skipped",
            array,
            out,
            len,
            out_written,
            out_required
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_omm_array_skipped",
            out_index,
            "out_index",
            array,
            "array"
        ));
        let out_index_range =
            checked_output_range("sidereon_omm_array_skipped", out_index, 1, "out_index");
        let written_range =
            checked_output_range("sidereon_omm_array_skipped", out_written, 1, "out_written");
        let required_range = checked_output_range(
            "sidereon_omm_array_skipped",
            out_required,
            1,
            "out_required",
        );
        c_try!(reject_overlapping_outputs(
            "sidereon_omm_array_skipped",
            c_try!(out_index_range),
            c_try!(written_range),
            "out_index",
            "out_written"
        ));
        c_try!(reject_overlapping_outputs(
            "sidereon_omm_array_skipped",
            c_try!(out_index_range),
            c_try!(required_range),
            "out_index",
            "out_required"
        ));
        if !out.is_null() && len != 0 {
            let out_range = checked_output_range("sidereon_omm_array_skipped", out, len, "out");
            c_try!(reject_overlapping_outputs(
                "sidereon_omm_array_skipped",
                c_try!(out_range),
                c_try!(out_index_range),
                "out",
                "out_index"
            ));
        }
        let out_index = c_try!(require_out(
            out_index,
            "sidereon_omm_array_skipped",
            "out_index"
        ));
        c_try!(init_copy_counts(
            "sidereon_omm_array_skipped",
            out_written,
            out_required
        ));
        *out_index = 0;
        let array = c_try!(require_ref(array, "sidereon_omm_array_skipped", "array"));
        let Some(skipped) = array.inner.skipped.get(index) else {
            set_last_error(format!(
                "sidereon_omm_array_skipped: index {index} out of range (count {})",
                array.inner.skipped.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_index = skipped.index;
        let json = match serde_json::to_vec(&crate::engine_error::omm_error_value(&skipped.reason))
        {
            Ok(json) => json,
            Err(error) => {
                set_last_error(format!(
                    "sidereon_omm_array_skipped: error payload serialization failed: {error}"
                ));
                return SidereonStatus::Panic;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_omm_array_skipped",
            "out",
            &json,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

unsafe fn encode_array(
    fn_name: &str,
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    encode: impl FnOnce(
        &[sidereon_core::astro::omm::Omm],
    ) -> Result<String, sidereon_core::astro::omm::OmmError>,
) -> SidereonStatus {
    c_try!(validate_array_copy_outputs(
        fn_name,
        array,
        out,
        len,
        out_written,
        out_required
    ));
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let array = c_try!(require_ref(array, fn_name, "array"));
    let text = match encode(&array.inner.omms) {
        Ok(text) => text,
        Err(error) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Omm,
                fn_name,
                crate::engine_error::omm_error_value(&error),
            );
            set_last_error(format!("{fn_name}: {error}"));
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
        out_required
    ));
    SidereonStatus::Ok
}

/// Write every collection record as a JSON array; refuse comments that JSON cannot preserve.
///
/// Safety: array is live; out points to len writable bytes or is null when len is zero;
/// out_written and out_required point to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_to_json(
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_to_json", SidereonStatus::Panic, || {
        encode_array(
            "sidereon_omm_array_to_json",
            array,
            out,
            len,
            out_written,
            out_required,
            sidereon_core::astro::omm::encode_json_array,
        )
    })
}

/// Write every record as a JSON array, discarding only comments JSON cannot carry.
///
/// Safety: array is live; out points to len writable bytes or is null when len is zero;
/// out_written and out_required point to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_to_json_discarding_comments(
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_array_to_json_discarding_comments",
        SidereonStatus::Panic,
        || {
            encode_array(
                "sidereon_omm_array_to_json_discarding_comments",
                array,
                out,
                len,
                out_written,
                out_required,
                sidereon_core::astro::omm::encode_json_array_discarding_comments,
            )
        },
    )
}

/// Write every record as GP CSV; refuse comments that CSV cannot preserve.
///
/// Safety: array is live; out points to len writable bytes or is null when len is zero;
/// out_written and out_required point to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_to_csv(
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_omm_array_to_csv", SidereonStatus::Panic, || {
        encode_array(
            "sidereon_omm_array_to_csv",
            array,
            out,
            len,
            out_written,
            out_required,
            sidereon_core::astro::omm::encode_csv,
        )
    })
}

/// Write every record as GP CSV, discarding only comments CSV cannot carry.
///
/// Safety: array is live; out points to len writable bytes or is null when len is zero;
/// out_written and out_required point to writable size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_to_csv_discarding_comments(
    array: *const SidereonOmmArray,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_omm_array_to_csv_discarding_comments",
        SidereonStatus::Panic,
        || {
            encode_array(
                "sidereon_omm_array_to_csv_discarding_comments",
                array,
                out,
                len,
                out_written,
                out_required,
                sidereon_core::astro::omm::encode_csv_discarding_comments,
            )
        },
    )
}

/// Release an owned OMM collection.
#[no_mangle]
pub unsafe extern "C" fn sidereon_omm_array_free(array: *mut SidereonOmmArray) {
    free_boxed(array);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISS_JSON: &str = r#"{"OBJECT_NAME":"ISS (ZARYA)","OBJECT_ID":"1998-067A","EPOCH":"2026-06-17T04:32:52.099296","MEAN_MOTION":15.49273435,"ECCENTRICITY":0.0004737,"INCLINATION":51.6332,"RA_OF_ASC_NODE":300.0813,"ARG_OF_PERICENTER":195.1146,"MEAN_ANOMALY":164.9702,"EPHEMERIS_TYPE":0,"CLASSIFICATION_TYPE":"U","NORAD_CAT_ID":25544,"ELEMENT_SET_NO":999,"REV_AT_EPOCH":57175,"BSTAR":0.00017172,"MEAN_MOTION_DOT":9.113e-5,"MEAN_MOTION_DDOT":0}"#;

    #[test]
    fn json_array_keeps_successes_and_full_skip_payload() {
        let source = format!("[{ISS_JSON},null,{ISS_JSON}]");
        let mut array = ptr::null_mut();
        let status =
            unsafe { sidereon_omm_parse_json_array(source.as_ptr(), source.len(), &mut array) };
        assert_eq!(status, SidereonStatus::Ok);
        let raw_array = array;
        let array = unsafe { &*raw_array };
        assert_eq!(array.inner.omms.len(), 2);
        assert_eq!(array.inner.omms[0].norad_cat_id, Some(25544));
        let skipped = &array.inner.skipped;
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].index, 1);
        let payload = crate::engine_error::omm_error_value(&skipped[0].reason);
        assert_eq!(payload["kind"], "field");
        assert!(payload.get("fields").is_some());
        unsafe { sidereon_omm_array_free(raw_array) };
    }

    #[test]
    fn standalone_epoch_preserves_leap_second_and_femtosecond_remainder() {
        let text = "2016-12-31T23:59:60.123456789123456";
        let mut epoch = SidereonOmmEpoch::default();
        let status = unsafe { sidereon_omm_parse_epoch(text.as_ptr(), text.len(), &mut epoch) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(epoch.year, 2016);
        assert_eq!(
            (
                epoch.month,
                epoch.day,
                epoch.hour,
                epoch.minute,
                epoch.second
            ),
            (12, 31, 23, 59, 60)
        );
        assert_eq!((epoch.microsecond, epoch.femtosecond), (123456, 789123456));
    }

    #[test]
    fn parser_rejects_aliased_input_and_output_without_mutating_input() {
        #[repr(align(8))]
        struct Aligned([u8; 16]);
        let mut source = Aligned([0; 16]);
        source.0[..2].copy_from_slice(b"[]");
        let before = source.0;
        let input = source.0.as_ptr();
        let output = source.0.as_mut_ptr().cast::<*mut SidereonOmmArray>();
        let status = unsafe { sidereon_omm_parse_json_array(input, 2, output) };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(source.0, before);

        let invalid = b"null";
        let mut output = std::ptr::dangling_mut::<SidereonOmmArray>();
        let status =
            unsafe { sidereon_omm_parse_json_array(invalid.as_ptr(), invalid.len(), &mut output) };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(output.is_null());
    }

    #[test]
    fn omm_initialized_satellite_outlives_source_handle() {
        let mut omm = ptr::null_mut();
        let status = unsafe {
            crate::omm::sidereon_omm_parse_json(ISS_JSON.as_ptr(), ISS_JSON.len(), &mut omm)
        };
        assert_eq!(status, SidereonStatus::Ok);
        let mut satellite = ptr::null_mut();
        let status = unsafe { crate::tle::sidereon_sgp4_satellite_from_omm(omm, &mut satellite) };
        assert_eq!(status, SidereonStatus::Ok);
        unsafe { crate::omm::sidereon_omm_free(omm) };

        let mut at_epoch = SidereonTemeState {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        assert_eq!(
            unsafe {
                crate::tle::sidereon_sgp4_satellite_propagate_minutes(satellite, 0.0, &mut at_epoch)
            },
            SidereonStatus::Ok
        );
        let mut elements = SidereonOmmElementSet::default();
        let mut source = ptr::null_mut();
        assert_eq!(
            unsafe {
                crate::omm::sidereon_omm_parse_json(ISS_JSON.as_ptr(), ISS_JSON.len(), &mut source)
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { crate::omm::sidereon_omm_to_element_set(source, &mut elements) },
            SidereonStatus::Ok
        );
        let mut at_same_epoch = SidereonTemeState {
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        };
        assert_eq!(
            unsafe {
                crate::tle::sidereon_sgp4_satellite_propagate_jd(
                    satellite,
                    elements.epoch_whole,
                    elements.epoch_fraction,
                    &mut at_same_epoch,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(at_epoch.position_km, at_same_epoch.position_km);
        assert_eq!(at_epoch.velocity_km_s, at_same_epoch.velocity_km_s);
        unsafe {
            crate::omm::sidereon_omm_free(source);
            crate::tle::sidereon_sgp4_satellite_free(satellite);
        }
    }
}
