use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, ils_error_value, record_engine_error,
    SidereonEngineErrorFamily,
};

/// Scalar outcome of an integer least-squares search. The best integer vector
/// itself is written to the caller's out_fixed buffer (n entries, parallel to the
/// input float_cycles); these are the accompanying scores and the ratio-test
/// verdict. Mirrors the scalar fields of sidereon_core::ils::IlsResult; the
/// symmetrized covariance / inverse it also carries are diagnostic and not
/// surfaced here.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIlsResult {
    /// Whether the ratio test passes at the requested threshold (the fix is
    /// accepted).
    pub fixed_status: bool,
    /// Runner-up / best score ratio. Saturates to DBL_MAX when the best score is
    /// exactly zero with a positive runner-up; 0 when there is no runner-up.
    pub ratio: f64,
    /// Best (lowest) quadratic score.
    pub best_score: f64,
    /// True when second_best_score carries a value (a runner-up lattice point
    /// existed).
    pub second_best_present: bool,
    /// Runner-up score, valid only when second_best_present is true.
    pub second_best_score: f64,
    /// Number of lattice points evaluated.
    pub candidates_evaluated: usize,
}

/// Resolve integer ambiguities with the LAMBDA method (the RTKLIB lambda() port):
/// the true integer-least-squares optimum and runner-up for any positive-definite
/// covariance, with no search box. float_cycles points to n float ambiguities;
/// covariance points to the row-major n x n covariance (covariance_len must equal
/// n*n). ratio_threshold is the acceptance threshold for the ratio test (a common
/// value is 3.0). On success the best integer vector is written to out_fixed (n
/// entries, parallel to float_cycles) and the scores/verdict to *out_result.
/// Fails with SIDEREON_STATUS_INVALID_ARGUMENT for a singular/degenerate
/// covariance, a dimension mismatch, non-finite inputs, or ambiguities outside
/// the int64 output domain (see sidereon_last_error_message).
///
/// Safety: float_cycles must point to n readable doubles; covariance must point to
/// covariance_len readable doubles; out_fixed must point to at least n writable
/// int64; out_result must point to a SidereonIlsResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_lambda_ils_search(
    float_cycles: *const f64,
    n: usize,
    covariance: *const f64,
    covariance_len: usize,
    ratio_threshold: f64,
    out_fixed: *mut i64,
    out_result: *mut SidereonIlsResult,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_lambda_ils_search", SidereonStatus::Panic, || {
        let floats = c_try!(require_slice(
            float_cycles,
            n,
            "sidereon_lambda_ils_search",
            "float_cycles"
        ));
        let cov = c_try!(ils_covariance_from_c(
            "sidereon_lambda_ils_search",
            n,
            covariance,
            covariance_len
        ));
        match lambda_ils_search(floats, &cov, ratio_threshold) {
            Ok(result) => {
                c_try!(write_ils_result(
                    "sidereon_lambda_ils_search",
                    &result,
                    out_fixed,
                    out_result
                ));
                SidereonStatus::Ok
            }
            Err(err) => ils_error_to_status("sidereon_lambda_ils_search", err),
        }
    })
}

/// Resolve integer ambiguities with a bounded lattice search: enumerate the
/// lattice within radius integers of each rounded float ambiguity, capped at
/// candidate_limit evaluations. Arguments mirror sidereon_lambda_ils_search plus
/// radius (per-ambiguity search half-width, integers) and candidate_limit (the
/// maximum lattice points to evaluate before failing). On success the best integer
/// vector is written to out_fixed (n entries) and the scores/verdict to
/// *out_result. Fails with SIDEREON_STATUS_INVALID_ARGUMENT for a singular
/// covariance, a lattice that exceeds candidate_limit or yields no candidate, a
/// dimension mismatch, or non-finite inputs.
///
/// Safety: float_cycles must point to n readable doubles; covariance must point to
/// covariance_len readable doubles; out_fixed must point to at least n writable
/// int64; out_result must point to a SidereonIlsResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_bounded_ils_search(
    float_cycles: *const f64,
    n: usize,
    covariance: *const f64,
    covariance_len: usize,
    radius: i64,
    candidate_limit: usize,
    ratio_threshold: f64,
    out_fixed: *mut i64,
    out_result: *mut SidereonIlsResult,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_bounded_ils_search", SidereonStatus::Panic, || {
        let floats = c_try!(require_slice(
            float_cycles,
            n,
            "sidereon_bounded_ils_search",
            "float_cycles"
        ));
        let cov = c_try!(ils_covariance_from_c(
            "sidereon_bounded_ils_search",
            n,
            covariance,
            covariance_len
        ));
        match bounded_ils_search(floats, &cov, radius, candidate_limit, ratio_threshold) {
            Ok(result) => {
                c_try!(write_ils_result(
                    "sidereon_bounded_ils_search",
                    &result,
                    out_fixed,
                    out_result
                ));
                SidereonStatus::Ok
            }
            Err(err) => ils_error_to_status("sidereon_bounded_ils_search", err),
        }
    })
}

// ---------------------------------------------------------------------------
// Product-staleness selection and broadcast/precise fallback.
//
// These wrap sidereon_core::staleness (graceful degradation for time-varying
// IONEX/SP3 products) and sidereon_core::positioning broadcast SPP + fallback.
// They are thin: the selection and the solve are the engine's, and a degraded or
// substituted answer always carries its staleness/source provenance rather than
// being silenced. Fetching the products over the network is the caller's job; the
// existing data/fetch surface covers it, and these entry points are pure compute.

fn ils_error_to_status(fn_name: &str, err: IlsError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Ils,
        fn_name,
        ils_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

/// Build the n x n covariance as Vec<Vec<f64>> from a row-major C buffer of
/// n*n doubles, validating the length and that n >= 1.
unsafe fn ils_covariance_from_c(
    fn_name: &str,
    n: usize,
    covariance: *const f64,
    covariance_len: usize,
) -> Result<Vec<Vec<f64>>, SidereonStatus> {
    if n == 0 {
        set_last_error(format!("{fn_name}: n must be at least 1"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let expected = n.checked_mul(n).ok_or_else(|| {
        set_last_error(format!("{fn_name}: covariance dimension {n} overflows"));
        SidereonStatus::InvalidArgument
    })?;
    if covariance_len != expected {
        set_last_error(format!(
            "{fn_name}: covariance_len {covariance_len} must equal n*n ({expected})"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let flat = require_slice(covariance, covariance_len, fn_name, "covariance")?;
    Ok(flat.chunks_exact(n).map(<[f64]>::to_vec).collect())
}

/// Convert an ils kernel result into the C scalar struct and copy the fixed
/// integer vector (n entries) into out_fixed.
unsafe fn write_ils_result(
    fn_name: &str,
    result: &IlsResult,
    out_fixed: *mut i64,
    out_result: *mut SidereonIlsResult,
) -> Result<(), SidereonStatus> {
    if out_result.is_null() {
        set_last_error(format!("{fn_name}: null out_result"));
        return Err(SidereonStatus::NullPointer);
    }
    // Preserve the established default result on missing/oversized fixed
    // output, while never forming &mut over uninitialized caller storage.
    let empty_result = SidereonIlsResult {
        fixed_status: false,
        ratio: 0.0,
        best_score: 0.0,
        second_best_present: false,
        second_best_score: 0.0,
        candidates_evaluated: 0,
    };
    let n = result.fixed.len();
    if out_fixed.is_null() {
        out_result.write(empty_result);
        set_last_error(format!("{fn_name}: null out_fixed"));
        return Err(SidereonStatus::NullPointer);
    }
    if let Err(status) = validate_element_count::<i64>(fn_name, "out_fixed", n) {
        out_result.write(empty_result);
        return Err(status);
    }
    let result_range = checked_output_range(fn_name, out_result, 1, "out_result")?;
    let fixed_range = checked_output_range(fn_name, out_fixed, n, "out_fixed")?;
    reject_overlapping_outputs(
        fn_name,
        result_range,
        fixed_range,
        "out_result",
        "out_fixed",
    )?;
    ptr::write(out_result, empty_result);
    ptr::copy_nonoverlapping(result.fixed.as_ptr(), out_fixed, n);
    ptr::write(
        out_result,
        SidereonIlsResult {
            fixed_status: result.fixed_status,
            ratio: result.ratio,
            best_score: result.best_score,
            second_best_present: result.second_best_score.is_some(),
            second_best_score: result.second_best_score.unwrap_or(0.0),
            candidates_evaluated: result.candidates_evaluated,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, ils_error_value, record_engine_error, sidereon_last_engine_error_info,
        sidereon_last_engine_error_payload, SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use serde_json::json;
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::ptr;

    fn get_last_error_string() -> String {
        unsafe {
            let len = sidereon_last_error_message(ptr::null_mut(), 0);
            if len == 0 {
                return String::new();
            }
            let mut buf = vec![0 as c_char; len + 1];
            sidereon_last_error_message(buf.as_mut_ptr(), buf.len());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    fn get_last_engine_error_two_pass() -> (SidereonEngineErrorInfo, String) {
        unsafe {
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            let status = sidereon_last_engine_error_info(&mut info);
            assert_eq!(status, SidereonStatus::Ok);

            let mut written = 0usize;
            let mut required = 0usize;
            let status =
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required);
            assert_eq!(status, SidereonStatus::Ok);
            assert_eq!(written, 0);
            assert_eq!(required, info.payload_len);

            if required == 0 {
                return (info, String::new());
            }

            let mut buf = vec![0u8; required];
            let status = sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            );
            assert_eq!(status, SidereonStatus::Ok);
            assert_eq!(written, required);
            let payload = String::from_utf8(buf).expect("valid utf-8 payload");
            (info, payload)
        }
    }

    #[test]
    fn test_all_seven_ils_error_mapper_variants_and_exact_payloads() {
        clear_engine_error();

        let cases: Vec<(IlsError, serde_json::Value)> = vec![
            (
                IlsError::Singular,
                json!({
                    "kind": "singular",
                    "fields": {}
                }),
            ),
            (
                IlsError::NoCandidates(42),
                json!({
                    "kind": "no_candidates",
                    "fields": { "count": 42 }
                }),
            ),
            (
                IlsError::TooManyCandidates {
                    evaluated: 128,
                    limit: 64,
                },
                json!({
                    "kind": "too_many_candidates",
                    "fields": { "evaluated": 128, "limit": 64 }
                }),
            ),
            (
                IlsError::InvalidDimensions { n: 4, rows: 3 },
                json!({
                    "kind": "invalid_dimensions",
                    "fields": { "n": 4, "rows": 3 }
                }),
            ),
            (
                IlsError::NonFinite,
                json!({
                    "kind": "non_finite",
                    "fields": {}
                }),
            ),
            (
                IlsError::InvalidInput {
                    field: "ils ratio_threshold",
                    reason: "negative",
                },
                json!({
                    "kind": "invalid_input",
                    "fields": {
                        "field": "ils ratio_threshold",
                        "reason": "negative"
                    }
                }),
            ),
            (
                IlsError::SearchLimitExceeded,
                json!({
                    "kind": "search_limit_exceeded",
                    "fields": {}
                }),
            ),
        ];

        for (err, expected_err_value) in cases {
            let mapped_val = ils_error_value(&err);
            assert_eq!(mapped_val, expected_err_value);

            record_engine_error(
                SidereonEngineErrorFamily::Ils,
                "test_ils_op",
                mapped_val.clone(),
            );

            let (info, payload_str) = get_last_engine_error_two_pass();
            assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
            assert_eq!(info.family as u32, 4);
            assert_eq!(info.payload_len, payload_str.len());

            let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["family"], "ils");
            assert_eq!(parsed["operation"], "test_ils_op");
            assert_eq!(parsed["error"], expected_err_value);

            clear_engine_error();
        }
    }

    #[test]
    fn test_lambda_ils_search_valid_control_and_real_producer_failures() {
        clear_engine_error();

        // 1. Valid control: well-conditioned 2x2 identity covariance
        let float_cycles = [0.0, 0.0];
        let covariance_ok = [1.0, 0.0, 0.0, 1.0];
        let mut out_fixed = [0i64; 2];
        let mut out_result = SidereonIlsResult {
            fixed_status: false,
            ratio: 0.0,
            best_score: 0.0,
            second_best_present: false,
            second_best_score: 0.0,
            candidates_evaluated: 0,
        };

        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(out_fixed, [0, 0]);
        assert!(out_result.fixed_status);

        // Verify engine error slot is cleared on success
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 2. Real core failure: singular covariance
        let covariance_singular = [1.0, 1.0, 1.0, 1.0];
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        assert_eq!(info.payload_len, payload_str.len());

        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "ils");
        assert_eq!(parsed["operation"], "sidereon_lambda_ils_search");
        assert_eq!(parsed["error"]["kind"], "singular");
        assert_eq!(parsed["error"]["fields"], json!({}));

        let msg = get_last_error_string();
        assert!(msg.contains("sidereon_lambda_ils_search"));
        assert!(msg.contains("covariance is singular"));

        // 3. Real core failure: negative ratio_threshold -> InvalidInput
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                -1.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "ils");
        assert_eq!(parsed["operation"], "sidereon_lambda_ils_search");
        assert_eq!(parsed["error"]["kind"], "invalid_input");
        assert_eq!(parsed["error"]["fields"]["field"], "ils ratio_threshold");
        assert_eq!(parsed["error"]["fields"]["reason"], "negative");

        // 4. Real core failure: non-finite float cycles -> NonFinite
        let float_nan = [f64::NAN, 0.0];
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_nan.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["error"]["kind"], "non_finite");

        clear_engine_error();
    }

    #[test]
    fn test_bounded_ils_search_valid_control_and_real_producer_failures() {
        clear_engine_error();

        // 1. Valid control
        let float_cycles = [0.0, 0.0];
        let covariance_ok = [1.0, 0.0, 0.0, 1.0];
        let mut out_fixed = [0i64; 2];
        let mut out_result = SidereonIlsResult {
            fixed_status: false,
            ratio: 0.0,
            best_score: 0.0,
            second_best_present: false,
            second_best_score: 0.0,
            candidates_evaluated: 0,
        };

        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(out_fixed, [0, 0]);
        assert!(out_result.fixed_status);

        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 2. Real core failure: candidate_limit exceeded (limit = 0, radius = 1 rejects at first dimension with 3 candidates)
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                1,
                0,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "ils");
        assert_eq!(parsed["operation"], "sidereon_bounded_ils_search");
        assert_eq!(parsed["error"]["kind"], "too_many_candidates");
        assert_eq!(parsed["error"]["fields"]["limit"], 0);
        assert_eq!(parsed["error"]["fields"]["evaluated"], 3);

        let msg = get_last_error_string();
        assert!(msg.contains("sidereon_bounded_ils_search"));
        assert!(msg.contains("exceeding limit 0"));

        // 3. Real core failure: negative radius -> NoCandidates
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                -1,
                100,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "ils");
        assert_eq!(parsed["operation"], "sidereon_bounded_ils_search");
        assert_eq!(parsed["error"]["kind"], "no_candidates");
        assert_eq!(parsed["error"]["fields"]["count"], 0);

        // 4. Real core failure: singular covariance
        let covariance_singular = [1.0, 1.0, 1.0, 1.0];
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);
        let parsed: serde_json::Value = serde_json::from_str(&payload_str).expect("parse json");
        assert_eq!(parsed["operation"], "sidereon_bounded_ils_search");
        assert_eq!(parsed["error"]["kind"], "singular");

        clear_engine_error();
    }

    #[test]
    fn test_engine_error_retention_through_two_pass_reads() {
        clear_engine_error();

        let float_cycles = [0.0, 0.0];
        let covariance_singular = [1.0, 1.0, 1.0, 1.0];
        let mut out_fixed = [0i64; 2];
        let mut out_result = SidereonIlsResult {
            fixed_status: false,
            ratio: 0.0,
            best_score: 0.0,
            second_best_present: false,
            second_best_score: 0.0,
            candidates_evaluated: 0,
        };

        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        // First pass reads:
        let mut info_1 = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        let status = unsafe { sidereon_last_engine_error_info(&mut info_1) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(info_1.family, SidereonEngineErrorFamily::Ils);
        assert!(info_1.payload_len > 0);

        let mut written_1 = 0usize;
        let mut required_1 = 0usize;
        let status = unsafe {
            sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written_1, &mut required_1)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written_1, 0);
        assert_eq!(required_1, info_1.payload_len);

        let mut buf_1 = vec![0u8; required_1];
        let status = unsafe {
            sidereon_last_engine_error_payload(
                buf_1.as_mut_ptr(),
                buf_1.len(),
                &mut written_1,
                &mut required_1,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written_1, required_1);

        let msg_1 = get_last_error_string();
        assert!(!msg_1.is_empty());

        // Second pass reads: verify that info, payload, and message are retained exactly
        let mut info_2 = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        let status = unsafe { sidereon_last_engine_error_info(&mut info_2) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(info_2.family, info_1.family);
        assert_eq!(info_2.payload_len, info_1.payload_len);

        let mut written_2 = 0usize;
        let mut required_2 = 0usize;
        let mut buf_2 = vec![0u8; info_2.payload_len];
        let status = unsafe {
            sidereon_last_engine_error_payload(
                buf_2.as_mut_ptr(),
                buf_2.len(),
                &mut written_2,
                &mut required_2,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written_2, info_2.payload_len);
        assert_eq!(buf_1, buf_2);

        let msg_2 = get_last_error_string();
        assert_eq!(msg_1, msg_2);

        clear_engine_error();
    }

    #[test]
    fn test_engine_error_clearing_after_success_and_early_argument_failure() {
        clear_engine_error();

        let float_cycles = [0.0, 0.0];
        let covariance_ok = [1.0, 0.0, 0.0, 1.0];
        let covariance_singular = [1.0, 1.0, 1.0, 1.0];
        let mut out_fixed = [0i64; 2];
        let mut out_result = SidereonIlsResult {
            fixed_status: false,
            ratio: 0.0,
            best_score: 0.0,
            second_best_present: false,
            second_best_score: 0.0,
            candidates_evaluated: 0,
        };

        // --- Clearing after early argument failure: null float_cycles ---
        // 1. Prime error slot with a real producer failure
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let (info, _) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);

        // 2. Invoke early argument failure: null float_cycles with n = 2
        let status = unsafe {
            sidereon_lambda_ils_search(
                ptr::null(),
                2,
                covariance_ok.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);

        // 3. Engine error slot must have been cleared at entry, no payload invented
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // --- Clearing after early argument failure: invalid dimension (n = 0) ---
        // 1. Prime error slot again
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let (info, _) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);

        // 2. Invoke early argument failure: n = 0 in bounded search
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                0,
                covariance_ok.as_ptr(),
                0,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        // 3. Slot cleared at entry; C precheck status without core engine payload
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());
        let msg = get_last_error_string();
        assert!(msg.contains("n must be at least 1"));

        // --- Clearing after successful producer: lambda search ---
        // 1. Prime error slot
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let (info, _) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);

        // 2. Successful solve
        let status = unsafe {
            sidereon_lambda_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);

        // 3. Engine error slot is cleared
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // --- Clearing after successful producer: bounded search ---
        // 1. Prime error slot
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_singular.as_ptr(),
                4,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let (info, _) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Ils);

        // 2. Successful bounded solve
        let status = unsafe {
            sidereon_bounded_ils_search(
                float_cycles.as_ptr(),
                2,
                covariance_ok.as_ptr(),
                4,
                1,
                27,
                3.0,
                out_fixed.as_mut_ptr(),
                &mut out_result,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);

        // 3. Slot is cleared
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        clear_engine_error();
    }
}
