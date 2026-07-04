use super::*;

// --- Standalone range RAIM/FDE design (sidereon_core::quality) ----------------

/// One linearized range measurement for sidereon_raim_fde_design, mirroring
/// sidereon_core::quality::RangeFdeRow. `design_row` points to `design_dim`
/// doubles (the design-matrix row); every row must carry the same `design_dim`,
/// which is the estimated state dimension.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangeFdeRow {
    /// Stable measurement identifier (e.g. a satellite token "G01").
    pub id: *const c_char,
    /// Observed-minus-computed range residual, metres.
    pub residual_m: f64,
    /// Design-matrix row: partials with respect to each estimated state parameter.
    pub design_row: *const f64,
    /// Length of `design_row`, the estimated state dimension.
    pub design_dim: usize,
    /// Inverse-variance weight 1/sigma^2; must be finite and strictly positive.
    pub weight: f64,
}

/// Options for sidereon_raim_fde_design, mirroring
/// sidereon_core::quality::RangeFdeOptions. Initialize with
/// sidereon_range_fde_options_init, then override fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangeFdeOptions {
    /// False-alarm probability for the global chi-square test, in (0, 1).
    pub p_fa: f64,
    /// Maximum number of measurements the exclusion loop may remove.
    pub max_exclusions: usize,
    /// Minimum redundancy (degrees of freedom) an exclusion must leave behind.
    pub min_redundancy: usize,
}

/// Global chi-square consistency test, mirroring
/// sidereon_core::quality::RangeChiSquareTest.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangeChiSquareTest {
    /// Weighted sum of squared post-fit residuals, v^T W v.
    pub weighted_sum_squares: f64,
    /// Redundancy n_used - n_state.
    pub dof: i64,
    /// Whether a chi-square threshold is present (true when dof > 0).
    pub has_threshold: bool,
    /// Chi-square threshold chi2_inv(1 - p_fa, dof), 0 when absent.
    pub threshold: f64,
    /// False when dof <= 0 (no redundancy to test against).
    pub testable: bool,
    /// True when the test statistic exceeds the threshold (a fault remains).
    pub fault_detected: bool,
}

/// Per-measurement FDE diagnostic, mirroring
/// sidereon_core::quality::RangeMeasurementDiagnostic, in input order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangeFdeDiagnostic {
    /// Measurement identifier, echoed from the input row.
    pub id: SidereonRtkId,
    /// Whether the FDE loop excluded this measurement.
    pub excluded: bool,
    /// Post-fit residual against the protected state correction, metres.
    pub post_fit_residual_m: f64,
    /// Standardized post-fit residual post_fit_residual_m * sqrt(weight).
    pub normalized_residual: f64,
}

/// Result of sidereon_raim_fde_design. Opaque to C. Read with the
/// sidereon_range_fde_result_* accessors; release with
/// sidereon_range_fde_result_free.
pub struct SidereonRangeFdeResult {
    pub(crate) inner: RangeFdeResult,
}

/// Initialize SidereonRangeFdeOptions with the engine defaults (RTKLIB demo5
/// p_fa, unbounded exclusions, minimum redundancy 1).
///
/// Safety: options must point to a writable SidereonRangeFdeOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_options_init(
    options: *mut SidereonRangeFdeOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_options_init",
        SidereonStatus::Panic,
        || {
            let options = c_try!(require_out(
                options,
                "sidereon_range_fde_options_init",
                "options"
            ));
            let defaults = RangeFdeOptions::default();
            *options = SidereonRangeFdeOptions {
                p_fa: defaults.p_fa,
                max_exclusions: defaults.max_exclusions,
                min_redundancy: defaults.min_redundancy,
            };
            SidereonStatus::Ok
        },
    )
}

/// The estimated state dimension (the protected state-correction length).
///
/// Safety: result is a live handle; out_dim points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_state_dim(
    result: *const SidereonRangeFdeResult,
    out_dim: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_state_dim",
        SidereonStatus::Panic,
        || {
            let out_dim = c_try!(require_out(
                out_dim,
                "sidereon_range_fde_result_state_dim",
                "out_dim"
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_state_dim",
                "result"
            ));
            *out_dim = result.inner.state_correction.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the protected weighted-least-squares state correction (length state_dim)
/// into out. Variable-length output contract.
///
/// Safety: result is a live handle; out points to len doubles or NULL when 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_state_correction(
    result: *const SidereonRangeFdeResult,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_state_correction",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_range_fde_result_state_correction",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_state_correction",
                "result"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_range_fde_result_state_correction",
                "out",
                &result.inner.state_correction,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the protected state covariance (H^T W H)^-1, row-major state_dim x
/// state_dim, into out. Variable-length output contract.
///
/// Safety: result is a live handle; out points to len doubles or NULL when 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_covariance(
    result: *const SidereonRangeFdeResult,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_range_fde_result_covariance",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_covariance",
                "result"
            ));
            let flat: Vec<f64> = result
                .inner
                .state_covariance
                .iter()
                .flat_map(|row| row.iter().copied())
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_range_fde_result_covariance",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the global chi-square consistency test for the accepted set into *out.
///
/// Safety: result is a live handle; out points to a SidereonRangeChiSquareTest.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_global_test(
    result: *const SidereonRangeFdeResult,
    out: *mut SidereonRangeChiSquareTest,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_global_test",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_range_fde_result_global_test",
                "out"
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_global_test",
                "result"
            ));
            let test = &result.inner.global_test;
            *out = SidereonRangeChiSquareTest {
                weighted_sum_squares: test.weighted_sum_squares,
                dof: test.dof as i64,
                has_threshold: test.threshold.is_some(),
                threshold: test.threshold.unwrap_or(0.0),
                testable: test.testable,
                fault_detected: test.fault_detected,
            };
            SidereonStatus::Ok
        },
    )
}

/// The number of exclusions the FDE loop performed.
///
/// Safety: result is a live handle; out_iterations points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_iterations(
    result: *const SidereonRangeFdeResult,
    out_iterations: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_iterations",
        SidereonStatus::Panic,
        || {
            let out_iterations = c_try!(require_out(
                out_iterations,
                "sidereon_range_fde_result_iterations",
                "out_iterations"
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_iterations",
                "result"
            ));
            *out_iterations = result.inner.iterations;
            SidereonStatus::Ok
        },
    )
}

/// Copy the excluded measurement ids (in exclusion order) into out. Variable-
/// length output contract.
///
/// Safety: result is a live handle; out points to len SidereonRtkId or NULL when
/// 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_excluded(
    result: *const SidereonRangeFdeResult,
    out: *mut SidereonRtkId,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_excluded",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_range_fde_result_excluded",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_excluded",
                "result"
            ));
            let rows: Vec<SidereonRtkId> = result
                .inner
                .excluded
                .iter()
                .map(|id| rtk_id_token(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_range_fde_result_excluded",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-measurement diagnostics (input order) into out. Variable-length
/// output contract.
///
/// Safety: result is a live handle; out points to len SidereonRangeFdeDiagnostic
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_diagnostics(
    result: *const SidereonRangeFdeResult,
    out: *mut SidereonRangeFdeDiagnostic,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_range_fde_result_diagnostics",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_range_fde_result_diagnostics",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_range_fde_result_diagnostics",
                "result"
            ));
            let rows: Vec<SidereonRangeFdeDiagnostic> = result
                .inner
                .diagnostics
                .iter()
                .map(|diag| SidereonRangeFdeDiagnostic {
                    id: rtk_id_token(&diag.id),
                    excluded: diag.excluded,
                    post_fit_residual_m: diag.post_fit_residual_m,
                    normalized_residual: diag.normalized_residual,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_range_fde_result_diagnostics",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a range RAIM/FDE result handle. Passing NULL is a no-op.
///
/// Safety: result is a handle from sidereon_raim_fde_design or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_range_fde_result_free(result: *mut SidereonRangeFdeResult) {
    free_boxed(result);
}
