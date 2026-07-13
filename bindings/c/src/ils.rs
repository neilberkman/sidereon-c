use super::*;

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
    ffi_boundary("sidereon_lambda_ils_search", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_bounded_ils_search", SidereonStatus::Panic, || {
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
    let out_result = require_out(out_result, fn_name, "out_result")?;
    *out_result = SidereonIlsResult {
        fixed_status: false,
        ratio: 0.0,
        best_score: 0.0,
        second_best_present: false,
        second_best_score: 0.0,
        candidates_evaluated: 0,
    };
    // The fixed vector length always equals the input n, so the caller sizes
    // out_fixed at n; copy_exact validates non-null and capacity.
    let n = result.fixed.len();
    if out_fixed.is_null() {
        set_last_error(format!("{fn_name}: null out_fixed"));
        return Err(SidereonStatus::NullPointer);
    }
    validate_element_count::<i64>(fn_name, "out_fixed", n)?;
    ptr::copy_nonoverlapping(result.fixed.as_ptr(), out_fixed, n);
    *out_result = SidereonIlsResult {
        fixed_status: result.fixed_status,
        ratio: result.ratio,
        best_score: result.best_score,
        second_best_present: result.second_best_score.is_some(),
        second_best_score: result.second_best_score.unwrap_or(0.0),
        candidates_evaluated: result.candidates_evaluated,
    };
    Ok(())
}
