use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::spk::SpkError;

/// A parsed JPL/NAIF SPK (DAF/SPK .bsp) ephemeris kernel. Opaque to C. Create
/// with sidereon_spk_load and release with sidereon_spk_free.
pub struct SidereonSpk {
    pub(crate) inner: Spk,
}

/// State of an SPK target body relative to a center body at a queried epoch,
/// as produced by sidereon_spk_state or sidereon_spk_state_in_frame. Both
/// vectors are in the NAIF frame `frame`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSpkState {
    /// NAIF target body identifier echoed from the query.
    pub target: i32,
    /// NAIF center body identifier echoed from the query.
    pub center: i32,
    /// Position of the target relative to the center, in kilometers.
    pub position_km: [f64; 3],
    /// Velocity of the target relative to the center, in kilometers per
    /// second. Every supported segment type yields it: type 2 as the
    /// derivative of its Chebyshev expansion over the record radius, as CSPICE
    /// `SPKE02` forms it.
    pub velocity_km_s: [f64; 3],
    /// NAIF reference-frame identifier the state is expressed in: the frame
    /// requested from sidereon_spk_state_in_frame, or for sidereon_spk_state
    /// the frame of the first segment evaluated (0 for the trivial
    /// target == center query).
    pub frame: i32,
}

/// Parse a JPL/NAIF SPK (DAF/SPK `.bsp`) ephemeris kernel from a byte buffer.
/// On success writes a newly owned handle to *out_spk. Release it with
/// sidereon_spk_free.
///
/// Safety: data must point to len readable bytes; out_spk must point to storage
/// for a SidereonSpk*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_load(
    data: *const u8,
    len: usize,
    out_spk: *mut *mut SidereonSpk,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_spk_load", SidereonStatus::Panic, || {
        let out_spk = c_try!(require_out(out_spk, "sidereon_spk_load", "out_spk"));
        let bytes =
            require_slice(data, len, "sidereon_spk_load", "data").map(|bytes| bytes.to_vec());
        *out_spk = ptr::null_mut();
        let bytes = c_try!(bytes);
        let inner = match Spk::from_bytes(&bytes) {
            Ok(spk) => spk,
            Err(err) => return map_spk_error("sidereon_spk_load", err),
        };
        write_boxed_handle(out_spk, SidereonSpk { inner });
        SidereonStatus::Ok
    })
}

/// Query the state of NAIF `target` relative to NAIF `center` at
/// `et_seconds_tdb` (ET/TDB seconds past J2000), writing the resolved relative
/// state into *out_state. Chooses segments as CSPICE `SPKSFS` and `SPKGEO` do
/// (for each body the highest-priority segment covering the epoch, the later
/// segment first) and evaluates SPK Types 2, 3, and 21. The numbers are exactly what the
/// engine's SPK reader produces.
///
/// Safety: spk must be a live handle from sidereon_spk_load that has not been
/// freed; out_state must point to a SidereonSpkState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_state(
    spk: *const SidereonSpk,
    target: i32,
    center: i32,
    et_seconds_tdb: f64,
    out_state: *mut SidereonSpkState,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_spk_state", SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, "sidereon_spk_state", "out_state"));
        c_try!(reject_output_overlaps_handle(
            "sidereon_spk_state",
            out_state,
            "out_state",
            spk,
            "spk"
        ));
        *out_state = empty_spk_state();
        let spk = c_try!(require_ref(spk, "sidereon_spk_state", "spk"));
        let state = match spk.inner.spk_state(target, center, et_seconds_tdb) {
            Ok(state) => state,
            Err(err) => return map_spk_error("sidereon_spk_state", err),
        };
        *out_state = spk_state_to_c(state);
        SidereonStatus::Ok
    })
}

/// Query the state of NAIF `target` relative to NAIF `center` at
/// `et_seconds_tdb` in the NAIF frame `frame`, as CSPICE `SPKGEO` does with
/// the reference frame `REF`: segments are chosen as for sidereon_spk_state
/// and the composed state is rotated into `frame` with the constant NAIF
/// inertial-frame rotation when it is expressed in another frame. A rotation
/// involving a frame outside the NAIF inertial frames 1-21 fails with
/// SIDEREON_STATUS_SOLVE.
///
/// Safety: spk must be a live handle from sidereon_spk_load that has not been
/// freed; out_state must point to a SidereonSpkState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_state_in_frame(
    spk: *const SidereonSpk,
    target: i32,
    center: i32,
    et_seconds_tdb: f64,
    frame: i32,
    out_state: *mut SidereonSpkState,
) -> SidereonStatus {
    let fn_name = "sidereon_spk_state_in_frame";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, fn_name, "out_state"));
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_state,
            "out_state",
            spk,
            "spk"
        ));
        *out_state = empty_spk_state();
        let spk = c_try!(require_ref(spk, fn_name, "spk"));
        let state = match spk
            .inner
            .spk_state_in_frame(target, center, et_seconds_tdb, frame)
        {
            Ok(state) => state,
            Err(err) => return map_spk_error(fn_name, err),
        };
        *out_state = spk_state_to_c(state);
        SidereonStatus::Ok
    })
}

/// Release an SPK kernel handle returned by sidereon_spk_load. Passing NULL is
/// a no-op.
///
/// Safety: spk must be NULL or a live handle from sidereon_spk_load that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spk_free(spk: *mut SidereonSpk) {
    ffi_boundary("sidereon_spk_free", (), || {
        free_boxed(spk);
    });
}

fn spk_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

/// Convert an `SpkError` into a structured JSON error node.
pub(crate) fn spk_error_value(error: &SpkError) -> serde_json::Value {
    match error {
        SpkError::Io { path, message } => spk_node(
            "io",
            json!({
                "path": path,
                "message": message,
            }),
        ),
        SpkError::Truncated {
            context,
            needed,
            actual,
        } => spk_node(
            "truncated",
            json!({
                "context": context,
                "needed": needed,
                "actual": actual,
            }),
        ),
        SpkError::UnsupportedDafId { id_word } => spk_node(
            "unsupported_daf_id",
            json!({
                "id_word": id_word,
            }),
        ),
        SpkError::UnsupportedBinaryFormat { binary_format } => spk_node(
            "unsupported_binary_format",
            json!({
                "binary_format": binary_format,
            }),
        ),
        SpkError::UnsupportedSummaryShape { nd, ni } => spk_node(
            "unsupported_summary_shape",
            json!({
                "nd": nd,
                "ni": ni,
            }),
        ),
        SpkError::InvalidField { field, value } => spk_node(
            "invalid_field",
            json!({
                "field": field,
                "value": value,
            }),
        ),
        SpkError::InvalidDoubleField { field, value } => spk_node(
            "invalid_double_field",
            json!({
                "field": field,
                "value": engine_f64(*value),
            }),
        ),
        SpkError::OutOfCoverage {
            et,
            start_et,
            stop_et,
        } => spk_node(
            "out_of_coverage",
            json!({
                "et": engine_f64(*et),
                "start_et": engine_f64(*start_et),
                "stop_et": engine_f64(*stop_et),
            }),
        ),
        SpkError::UnsupportedSegmentType { expected, actual } => spk_node(
            "unsupported_segment_type",
            json!({
                "expected": expected,
                "actual": actual,
            }),
        ),
        SpkError::InvalidSegmentLayout { context } => spk_node(
            "invalid_segment_layout",
            json!({
                "context": context,
            }),
        ),
        SpkError::UnknownBody { body } => spk_node(
            "unknown_body",
            json!({
                "body": body,
            }),
        ),
        SpkError::NoSegmentPath { target, center } => spk_node(
            "no_segment_path",
            json!({
                "target": target,
                "center": center,
            }),
        ),
        SpkError::CoverageGap { target, center, et } => spk_node(
            "coverage_gap",
            json!({
                "target": target,
                "center": center,
                "et": engine_f64(*et),
            }),
        ),
        SpkError::UnsupportedStateSegmentType { data_type } => spk_node(
            "unsupported_state_segment_type",
            json!({
                "data_type": data_type,
            }),
        ),
        SpkError::NonInertialFrameRotation { from, to } => spk_node(
            "non_inertial_frame_rotation",
            json!({
                "from": from,
                "to": to,
            }),
        ),
    }
}

/// Map an SPK reader error to a binding status code, recording its message.
///
/// Malformed-kernel and bad-caller-input conditions report
/// SIDEREON_STATUS_INVALID_ARGUMENT (an unusable buffer or an unknown body /
/// non-finite epoch). Conditions where the kernel is well-formed but cannot
/// satisfy the query (no covering segment for the epoch, no connecting path, or
/// a segment data type the reader does not evaluate) report
/// SIDEREON_STATUS_SOLVE, mirroring the SP3 reader's argument-vs-range split.
fn map_spk_error(fn_name: &str, err: SpkError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Spk,
        fn_name,
        spk_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        SpkError::Io { .. }
        | SpkError::Truncated { .. }
        | SpkError::UnsupportedDafId { .. }
        | SpkError::UnsupportedBinaryFormat { .. }
        | SpkError::UnsupportedSummaryShape { .. }
        | SpkError::InvalidField { .. }
        | SpkError::InvalidSegmentLayout { .. }
        | SpkError::InvalidDoubleField { .. }
        | SpkError::UnknownBody { .. } => SidereonStatus::InvalidArgument,
        SpkError::OutOfCoverage { .. }
        | SpkError::CoverageGap { .. }
        | SpkError::NoSegmentPath { .. }
        | SpkError::UnsupportedSegmentType { .. }
        | SpkError::UnsupportedStateSegmentType { .. }
        | SpkError::NonInertialFrameRotation { .. } => SidereonStatus::Solve,
    }
}

fn empty_spk_state() -> SidereonSpkState {
    SidereonSpkState {
        target: 0,
        center: 0,
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
        frame: 0,
    }
}

fn spk_state_to_c(state: SpkState) -> SidereonSpkState {
    SidereonSpkState {
        target: state.target,
        center: state.center,
        position_km: state.position_km,
        velocity_km_s: state.velocity_km_s,
        frame: state.frame,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, engine_f64, sidereon_last_engine_error_info,
        sidereon_last_engine_error_payload, SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::Value;
    use std::ptr;

    const KERNEL_BYTES: &[u8] = include_bytes!("../tests/fixtures/spk/horizons_eros_type21.bsp");

    #[test]
    fn table_driven_spk_error_mapping() {
        use sidereon_core::astro::spk::SpkError as E;
        let cases: Vec<(E, &'static str)> = vec![
            (
                E::Io {
                    path: "kernel.bsp".into(),
                    message: "not found".into(),
                },
                "io",
            ),
            (
                E::Truncated {
                    context: "file record",
                    needed: 1024,
                    actual: 512,
                },
                "truncated",
            ),
            (
                E::UnsupportedDafId {
                    id_word: "NAIF/NSP".into(),
                },
                "unsupported_daf_id",
            ),
            (
                E::UnsupportedBinaryFormat {
                    binary_format: "BIG-IEEE".into(),
                },
                "unsupported_binary_format",
            ),
            (
                E::UnsupportedSummaryShape { nd: 3, ni: 5 },
                "unsupported_summary_shape",
            ),
            (
                E::InvalidField {
                    field: "bward",
                    value: -1,
                },
                "invalid_field",
            ),
            (
                E::InvalidDoubleField {
                    field: "radius",
                    value: 123.456,
                },
                "invalid_double_field",
            ),
            (
                E::OutOfCoverage {
                    et: 100.0,
                    start_et: 200.0,
                    stop_et: 300.0,
                },
                "out_of_coverage",
            ),
            (
                E::UnsupportedSegmentType {
                    expected: 2,
                    actual: 1,
                },
                "unsupported_segment_type",
            ),
            (
                E::InvalidSegmentLayout {
                    context: "directory count",
                },
                "invalid_segment_layout",
            ),
            (E::UnknownBody { body: 9999 }, "unknown_body"),
            (
                E::NoSegmentPath {
                    target: 499,
                    center: 399,
                },
                "no_segment_path",
            ),
            (
                E::CoverageGap {
                    target: 499,
                    center: 0,
                    et: 50.0,
                },
                "coverage_gap",
            ),
            (
                E::UnsupportedStateSegmentType { data_type: 14 },
                "unsupported_state_segment_type",
            ),
            (
                E::NonInertialFrameRotation { from: 10001, to: 1 },
                "non_inertial_frame_rotation",
            ),
        ];

        for (err, expected_kind) in cases {
            let val = spk_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            match &err {
                E::Io { path, message } => {
                    assert_eq!(val["fields"]["path"], path.as_str());
                    assert_eq!(val["fields"]["message"], message.as_str());
                }
                E::Truncated {
                    context,
                    needed,
                    actual,
                } => {
                    assert_eq!(val["fields"]["context"], *context);
                    assert_eq!(val["fields"]["needed"], *needed);
                    assert_eq!(val["fields"]["actual"], *actual);
                }
                E::UnsupportedDafId { id_word } => {
                    assert_eq!(val["fields"]["id_word"], id_word.as_str());
                }
                E::UnsupportedBinaryFormat { binary_format } => {
                    assert_eq!(val["fields"]["binary_format"], binary_format.as_str());
                }
                E::UnsupportedSummaryShape { nd, ni } => {
                    assert_eq!(val["fields"]["nd"], *nd);
                    assert_eq!(val["fields"]["ni"], *ni);
                }
                E::InvalidField { field, value } => {
                    assert_eq!(val["fields"]["field"], *field);
                    assert_eq!(val["fields"]["value"], *value);
                }
                E::InvalidDoubleField { field, value } => {
                    assert_eq!(val["fields"]["field"], *field);
                    assert_eq!(val["fields"]["value"]["decimal"], value.to_string());
                    assert_eq!(
                        val["fields"]["value"]["bits_hex"],
                        format!("{:016x}", value.to_bits())
                    );
                }
                E::OutOfCoverage {
                    et,
                    start_et,
                    stop_et,
                } => {
                    assert_eq!(val["fields"]["et"]["decimal"], et.to_string());
                    assert_eq!(val["fields"]["start_et"]["decimal"], start_et.to_string());
                    assert_eq!(val["fields"]["stop_et"]["decimal"], stop_et.to_string());
                }
                E::UnsupportedSegmentType { expected, actual } => {
                    assert_eq!(val["fields"]["expected"], *expected);
                    assert_eq!(val["fields"]["actual"], *actual);
                }
                E::InvalidSegmentLayout { context } => {
                    assert_eq!(val["fields"]["context"], *context);
                }
                E::UnknownBody { body } => {
                    assert_eq!(val["fields"]["body"], *body);
                }
                E::NoSegmentPath { target, center } => {
                    assert_eq!(val["fields"]["target"], *target);
                    assert_eq!(val["fields"]["center"], *center);
                }
                E::CoverageGap { target, center, et } => {
                    assert_eq!(val["fields"]["target"], *target);
                    assert_eq!(val["fields"]["center"], *center);
                    assert_eq!(val["fields"]["et"], engine_f64(*et));
                }
                E::UnsupportedStateSegmentType { data_type } => {
                    assert_eq!(val["fields"]["data_type"], *data_type);
                }
                E::NonInertialFrameRotation { from, to } => {
                    assert_eq!(val["fields"]["from"], *from);
                    assert_eq!(val["fields"]["to"], *to);
                }
            }
        }
    }

    #[test]
    fn spk_public_producer_control_and_refusals() {
        clear_engine_error();

        // Valid control: load kernel, query valid state, free.
        unsafe {
            let mut spk: *mut SidereonSpk = ptr::null_mut();
            assert_eq!(
                sidereon_spk_load(KERNEL_BYTES.as_ptr(), KERNEL_BYTES.len(), &mut spk),
                SidereonStatus::Ok
            );
            assert!(!spk.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Spk,
                payload_len: 123,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            let mut state = empty_spk_state();
            // Epoch from Eros reference fixture: 757339200.0, target 20000433 (Eros), center 10 (Sun)
            assert_eq!(
                sidereon_spk_state(spk, 20000433, 10, 757339200.0, &mut state),
                SidereonStatus::Ok
            );
            assert_eq!(state.target, 20000433);
            assert_eq!(state.center, 10);
            assert_eq!(state.frame, 1);
            assert_ne!(state.position_km, [0.0; 3]);
            assert_ne!(state.velocity_km_s, [0.0; 3]);

            // Free the handle
            sidereon_spk_free(spk);
        }

        // Real public refusal 1: invalid kernel bytes in sidereon_spk_load
        unsafe {
            let mut spk: *mut SidereonSpk = ptr::null_mut();
            let bad_bytes = b"NOT_A_VALID_SPK_HEADER";
            assert_eq!(
                sidereon_spk_load(bad_bytes.as_ptr(), bad_bytes.len(), &mut spk),
                SidereonStatus::InvalidArgument
            );
            assert!(spk.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Spk);
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
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "spk");
            assert_eq!(payload["operation"], "sidereon_spk_load");
            assert_eq!(payload["error"]["kind"], "truncated");
        }

        // Real public refusal 2: coverage gap query in sidereon_spk_state
        unsafe {
            let mut spk: *mut SidereonSpk = ptr::null_mut();
            assert_eq!(
                sidereon_spk_load(KERNEL_BYTES.as_ptr(), KERNEL_BYTES.len(), &mut spk),
                SidereonStatus::Ok
            );

            let mut state = empty_spk_state();
            // Epoch 1.0e12 is far past any ephemeris coverage
            assert_eq!(
                sidereon_spk_state(spk, 20000433, 10, 1.0e12, &mut state),
                SidereonStatus::Solve
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Spk);
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
            assert_eq!(payload["family"], "spk");
            assert_eq!(payload["operation"], "sidereon_spk_state");
            assert_eq!(payload["error"]["kind"], "coverage_gap");
            assert_eq!(payload["error"]["fields"]["target"], 20000433);
            assert_eq!(payload["error"]["fields"]["center"], 10);
            assert_eq!(payload["error"]["fields"]["et"], engine_f64(1.0e12));

            sidereon_spk_free(spk);
        }
    }

    #[test]
    fn spk_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // First, cause a real refusal to seed the retained engine error
            let mut spk: *mut SidereonSpk = ptr::null_mut();
            assert_eq!(
                sidereon_spk_load(b"BAD".as_ptr(), 3, &mut spk),
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
            assert_eq!(info.family, SidereonEngineErrorFamily::Spk);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Calling free retains the error
            sidereon_spk_free(ptr::null_mut());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Spk);
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
            assert_eq!(info.family, SidereonEngineErrorFamily::Spk);

            // Now test producer early-validation clearing:
            // Call sidereon_spk_load with a null out pointer
            assert_eq!(
                sidereon_spk_load(b"TEST".as_ptr(), 4, ptr::null_mut()),
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
