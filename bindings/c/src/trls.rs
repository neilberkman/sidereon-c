use super::*;

// --- Generic data-driven trust-region least squares (HEADLINE) --------------

/// Which built-in residual a [`SidereonDataProblem`] fits. Selects the meaning
/// of the data arrays the problem carries.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTrlsKind {
    /// Dense linear least squares `r_i = (row_i . x) - b_i`. Uses `a` (row-major
    /// `m`-by-`n`) and `b` (length `m`); `m` and `n` set the shape.
    Linear = 0,
    /// Polynomial fit of `degree` (so `n = degree + 1` coefficients, lowest
    /// order first): `r_i = horner(x, t_i) - y_i`. Uses `t` and `y`.
    Polynomial = 1,
    /// Exponential model with `n = 3` parameters `[amp, rate, offset]`:
    /// `r_i = (x0 exp(x1 t_i) + x2) - y_i`. Uses `t` and `y`.
    Exponential = 2,
}

/// SciPy `loss` selector for a [`SidereonDataProblem`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTrlsLoss {
    /// Identity loss (ordinary least squares); the default.
    Linear = 0,
    /// SciPy `soft_l1`.
    SoftL1 = 1,
    /// SciPy `huber`.
    Huber = 2,
    /// SciPy `cauchy`.
    Cauchy = 3,
    /// SciPy `arctan`.
    Arctan = 4,
}

/// SciPy `x_scale` mode for a [`SidereonDataProblem`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTrlsXScale {
    /// Unit per-parameter scale (`x_scale = 1.0`); the default.
    Unit = 0,
    /// Jacobian-derived adaptive scale (`x_scale = 'jac'`).
    Jac = 1,
    /// Explicit per-parameter scale read from `x_scale_values` (length `n`).
    Values = 2,
}

/// Which linear-algebra backend drives the trust-region iteration.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTrlsBackend {
    /// Pure-Rust in-crate SVD (works everywhere; not bit-identical to SciPy).
    Native = 0,
    /// Host LAPACK/BLAS resolved from the environment
    /// (`TRUST_REGION_LEAST_SQUARES_LAPACK_PATH`), for bit-for-bit SciPy parity.
    HostLapack = 1,
}

/// A fully specified data-driven least-squares problem, flat for FFI.
///
/// Fill the kind-specific data pointers and lengths, the starting point `x0`,
/// and the SciPy-style configuration; the residual and Jacobian for every
/// iteration are evaluated entirely in the core, so the solve crosses the C
/// boundary once in and once out. Initialize with
/// [`sidereon_data_problem_init`] for the SciPy defaults, then set the data
/// pointers. The core validates every shape before iterating.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDataProblem {
    /// Residual kind, selecting which data arrays are read. Stored as a
    /// uint32_t; use the SidereonTrlsKind discriminants.
    pub kind: u32,
    /// Linear design matrix, row-major `m`-by-`n` (Linear only). May be NULL for
    /// the other kinds.
    pub a: *const f64,
    /// Length of `a` (must equal `m * n` for Linear).
    pub a_len: usize,
    /// Linear right-hand side, length `m` (Linear only).
    pub b: *const f64,
    /// Length of `b` (must equal `m` for Linear).
    pub b_len: usize,
    /// Sample abscissae `t` (Polynomial / Exponential only).
    pub t: *const f64,
    /// Length of `t`.
    pub t_len: usize,
    /// Sample ordinates `y` (Polynomial / Exponential only).
    pub y: *const f64,
    /// Length of `y` (must equal `t_len`).
    pub y_len: usize,
    /// Residual-row count `m` (Linear only; for Polynomial/Exponential the row
    /// count is `t_len`).
    pub m: usize,
    /// Parameter count `n` (Linear only; derived from `degree`/`3` otherwise).
    pub n: usize,
    /// Polynomial degree (Polynomial only; coefficients `n = degree + 1`).
    pub degree: usize,
    /// Starting parameter vector, length equal to the kind's parameter count.
    pub x0: *const f64,
    /// Length of `x0`.
    pub x0_len: usize,
    /// SciPy `loss`. Stored as a uint32_t; use the SidereonTrlsLoss
    /// discriminants.
    pub loss: u32,
    /// SciPy `f_scale` (only consulted for a robust loss).
    pub f_scale: f64,
    /// SciPy `x_scale` mode. Stored as a uint32_t; use the SidereonTrlsXScale
    /// discriminants.
    pub x_scale_mode: u32,
    /// Per-parameter scale values, length `n` (read only when
    /// `x_scale_mode == Values`).
    pub x_scale_values: *const f64,
    /// Length of `x_scale_values`.
    pub x_scale_values_len: usize,
    /// SciPy `max_nfev`; a negative value selects the default `100 * n`.
    pub max_nfev: i64,
    /// SciPy `ftol`.
    pub ftol: f64,
    /// SciPy `xtol`.
    pub xtol: f64,
    /// SciPy `gtol`.
    pub gtol: f64,
    /// Linear-algebra backend. Stored as a uint32_t; use the
    /// SidereonTrlsBackend discriminants.
    pub backend: u32,
}

/// Scalar summary of a converged trust-region solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrlsSummary {
    /// Final cost `0.5 * sum(residual^2)`.
    pub cost: f64,
    /// First-order optimality `||J^T f||_inf` at the solution.
    pub optimality: f64,
    /// Residual-evaluation count.
    pub nfev: usize,
    /// Jacobian-evaluation count.
    pub njev: usize,
    /// SciPy status: 0 max-eval, 1 gtol, 2 ftol, 3 xtol, 4 ftol+xtol.
    pub status: i32,
    /// Whether the solve converged (status > 0).
    pub success: bool,
    /// Parameter count `n` (length of the solution vector).
    pub n: usize,
    /// Residual count `m`.
    pub m: usize,
}

/// Opaque handle to a converged TRLS solution. Release with
/// [`sidereon_trls_solution_free`].
pub struct SidereonTrlsSolution {
    pub(crate) inner: TrfResult,
}

/// Opaque handle to a leave-one-out (RAIM/FDE) report: the base solve plus one
/// re-solve per dropped residual row. Release with
/// [`sidereon_trls_drop_one_free`].
pub struct SidereonTrlsDropOne {
    pub(crate) inner: DropOneReport,
}

// The data-problem enum fields and the convention argument cross the C boundary
// as their raw uint32_t repr: matching the integer against the known
// discriminants here means an out-of-range value reported by C becomes an
// InvalidArgument status rather than an invalid Rust enum (undefined behavior).

/// Populate `*out_problem` with the SciPy `least_squares` defaults for `kind`:
/// linear loss, `f_scale = 1`, unit `x_scale`, default evaluation budget, the
/// SciPy `ftol = xtol = 1e-8` / `gtol = 1e-10` tolerances, and the native
/// backend. All data pointers are zeroed; set them (and `m`/`n`/`degree`) before
/// solving.
///
/// Safety: out_problem must point to a SidereonDataProblem.
#[no_mangle]
pub unsafe extern "C" fn sidereon_data_problem_init(
    kind: u32,
    out_problem: *mut SidereonDataProblem,
) -> SidereonStatus {
    ffi_boundary("sidereon_data_problem_init", SidereonStatus::Panic, || {
        let out_problem = c_try!(require_out(
            out_problem,
            "sidereon_data_problem_init",
            "out_problem"
        ));
        // Validate the C-supplied kind against the known discriminants before
        // storing it, so an out-of-range integer is rejected here.
        let kind = c_try!(trls_kind_from_c("sidereon_data_problem_init", "kind", kind)) as u32;
        // Mirror DataProblem::new (TrfOptions::default) for the scalar fields.
        *out_problem = SidereonDataProblem {
            kind,
            a: ptr::null(),
            a_len: 0,
            b: ptr::null(),
            b_len: 0,
            t: ptr::null(),
            t_len: 0,
            y: ptr::null(),
            y_len: 0,
            m: 0,
            n: 0,
            degree: 0,
            x0: ptr::null(),
            x0_len: 0,
            loss: SidereonTrlsLoss::Linear as u32,
            f_scale: 1.0,
            x_scale_mode: SidereonTrlsXScale::Unit as u32,
            x_scale_values: ptr::null(),
            x_scale_values_len: 0,
            max_nfev: -1,
            ftol: 1e-8,
            xtol: 1e-8,
            gtol: 1e-10,
            backend: SidereonTrlsBackend::Native as u32,
        };
        SidereonStatus::Ok
    })
}

/// Copy the scalar summary (cost, optimality, evaluation counts, status, shape)
/// into *out_summary.
///
/// Safety: sol must be a live handle; out_summary must point to a
/// SidereonTrlsSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_summary(
    sol: *const SidereonTrlsSolution,
    out_summary: *mut SidereonTrlsSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_solution_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_trls_solution_summary",
                "out_summary"
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_trls_solution_summary",
                "solution"
            ));
            *out_summary = trls_summary(&sol.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the solution vector `x` (length `n`) into out. Variable-length output
/// contract: pass out NULL with len 0 to query the count via *out_required.
///
/// Safety: sol must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_x(
    sol: *const SidereonTrlsSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_trls_solution_x", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_trls_solution_x",
            out_written,
            out_required
        ));
        let sol = c_try!(require_ref(sol, "sidereon_trls_solution_x", "solution"));
        c_try!(copy_prefix_to_c(
            "sidereon_trls_solution_x",
            "out",
            &sol.inner.x,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the residual vector `fun` (length `m`) at the solution into out. Same
/// variable-length output contract as sidereon_trls_solution_x.
///
/// Safety: sol must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_residuals(
    sol: *const SidereonTrlsSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_solution_residuals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_trls_solution_residuals",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_trls_solution_residuals",
                "solution"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_trls_solution_residuals",
                "out",
                &sol.inner.fun,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the gradient `J^T f` (length `n`) at the solution into out. Same
/// variable-length output contract as sidereon_trls_solution_x.
///
/// Safety: sol must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_gradient(
    sol: *const SidereonTrlsSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_solution_gradient",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_trls_solution_gradient",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_trls_solution_gradient",
                "solution"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_trls_solution_gradient",
                "out",
                &sol.inner.grad,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the row-major `m`-by-`n` Jacobian at the solution into out (length
/// `m * n`). Same variable-length output contract as sidereon_trls_solution_x.
///
/// Safety: sol must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_jacobian(
    sol: *const SidereonTrlsSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_solution_jacobian",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_trls_solution_jacobian",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_trls_solution_jacobian",
                "solution"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_trls_solution_jacobian",
                "out",
                &sol.inner.jac,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a TRLS solution handle. Null is a no-op.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_data_problem,
/// freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_solution_free(sol: *mut SidereonTrlsSolution) {
    ffi_boundary("sidereon_trls_solution_free", (), || {
        free_boxed(sol);
    });
}

/// Write the number of dropped-row re-solves (equal to the residual count `m`)
/// into *out_count.
///
/// Safety: report must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_count(
    report: *const SidereonTrlsDropOne,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_drop_one_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_trls_drop_one_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_trls_drop_one_count",
                "report"
            ));
            *out_count = report.inner.drops.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the base (all-rows) solve summary into *out_summary.
///
/// Safety: report must be a live handle; out_summary must point to a
/// SidereonTrlsSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_base_summary(
    report: *const SidereonTrlsDropOne,
    out_summary: *mut SidereonTrlsSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_drop_one_base_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_trls_drop_one_base_summary",
                "out_summary"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_trls_drop_one_base_summary",
                "report"
            ));
            *out_summary = trls_summary(&report.inner.base);
            SidereonStatus::Ok
        },
    )
}

/// Copy the summary of the re-solve with residual row `index` masked into
/// *out_summary. `index` must be less than sidereon_trls_drop_one_count.
///
/// Safety: report must be a live handle; out_summary must point to a
/// SidereonTrlsSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_drop_summary(
    report: *const SidereonTrlsDropOne,
    index: usize,
    out_summary: *mut SidereonTrlsSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_drop_one_drop_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_trls_drop_one_drop_summary",
                "out_summary"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_trls_drop_one_drop_summary",
                "report"
            ));
            let Some(drop) = report.inner.drops.get(index) else {
                set_last_error(format!(
                    "sidereon_trls_drop_one_drop_summary: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_summary = trls_summary(drop);
            SidereonStatus::Ok
        },
    )
}

/// Copy the solution vector of the re-solve with residual row `index` masked
/// into out. Same variable-length output contract as sidereon_trls_solution_x.
///
/// Safety: report must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_drop_x(
    report: *const SidereonTrlsDropOne,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_drop_one_drop_x",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_trls_drop_one_drop_x",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_trls_drop_one_drop_x",
                "report"
            ));
            let Some(drop) = report.inner.drops.get(index) else {
                set_last_error(format!(
                    "sidereon_trls_drop_one_drop_x: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_trls_drop_one_drop_x",
                "out",
                &drop.x,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-row cost deltas (`drops[i].cost - base.cost`, length `m`) into
/// out: how much the optimum cost moves when each row is removed, the influence
/// signal for fault detection. Same variable-length output contract as
/// sidereon_trls_solution_x.
///
/// Safety: report must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_cost_delta(
    report: *const SidereonTrlsDropOne,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_trls_drop_one_cost_delta",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_trls_drop_one_cost_delta",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_trls_drop_one_cost_delta",
                "report"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_trls_drop_one_cost_delta",
                "out",
                &report.inner.cost_delta,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a leave-one-out report handle. Null is a no-op.
///
/// Safety: report must be NULL or a live handle from
/// sidereon_solve_data_problem_drop_one, freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trls_drop_one_free(report: *mut SidereonTrlsDropOne) {
    ffi_boundary("sidereon_trls_drop_one_free", (), || {
        free_boxed(report);
    });
}

fn trls_summary(result: &TrfResult) -> SidereonTrlsSummary {
    SidereonTrlsSummary {
        cost: result.cost,
        optimality: result.optimality,
        nfev: result.nfev,
        njev: result.njev,
        status: result.status,
        success: result.success(),
        n: result.x.len(),
        m: result.fun.len(),
    }
}
