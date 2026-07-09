use super::*;

// --- Standalone RAIM (sidereon_core::quality) --------------------------------

/// A RAIM integrity result, mirroring sidereon_core::quality::RaimResult. The
/// worst_sat token is null-terminated and valid only when has_worst_sat is true.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRaimResult {
    /// Whether a fault was detected.
    pub fault_detected: bool,
    /// Chi-square test statistic.
    pub test_statistic: f64,
    /// Whether a detection threshold was computed.
    pub has_threshold: bool,
    /// Detection threshold (valid when has_threshold is true).
    pub threshold: f64,
    /// Whether reduced_chi_square is valid.
    pub has_reduced_chi_square: bool,
    /// Chi-square statistic divided by dof, valid when has_reduced_chi_square
    /// is true.
    pub reduced_chi_square: f64,
    /// Root-mean-square residual, meters.
    pub rms_m: f64,
    /// Redundancy degrees of freedom.
    pub dof: i64,
    /// Whether the geometry was testable.
    pub testable: bool,
    /// Number of normalized residual rows available from
    /// sidereon_raim_normalized_residuals.
    pub normalized_residual_count: usize,
    /// Whether worst_sat carries a satellite token.
    pub has_worst_sat: bool,
    /// Worst-residual satellite token, null-terminated (valid when
    /// has_worst_sat). Sized to hold any GNSS token (16 bytes) plus the
    /// terminator; kept in step with SATELLITE_TOKEN_C_BYTES by the assert below.
    pub worst_sat: [c_char; 17],
}

/// One per-satellite normalized RAIM residual.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRaimNormalizedResidual {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Residual multiplied by sqrt(weight), meters.
    pub normalized_residual: f64,
}

/// Run the RAIM chi-square test over used satellites and their residuals.
/// weights/unit_weights/n_systems mirror SidereonFdeOptions. Weights must be
/// inverse variances derived from per-satellite residual variances; unit
/// weights on metre-scale residuals make fault_detected saturate near 100%.
/// Delegates to sidereon_core::quality::raim.
///
/// Safety: used_sat_ids points to count null-terminated tokens; residuals_m
/// points to count doubles; weights points to weight_count SidereonFdeRaimWeight
/// when unit_weights is false; out points to a SidereonRaimResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_raim(
    used_sat_ids: *const *const c_char,
    residuals_m: *const f64,
    count: usize,
    p_fa: f64,
    unit_weights: bool,
    weights: *const SidereonFdeRaimWeight,
    weight_count: usize,
    n_systems_enabled: bool,
    n_systems: i64,
    out: *mut SidereonRaimResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_raim", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_raim", "out"));
        *out = empty_raim_result();
        let (input, residuals) = c_try!(raim_input_from_c(
            "sidereon_raim",
            used_sat_ids,
            residuals_m,
            count
        ));
        let raim_weights = c_try!(raim_weights_from_c(
            "sidereon_raim",
            unit_weights,
            weights,
            weight_count
        ));
        let options = RaimOptions {
            p_fa,
            weights: raim_weights,
            n_systems: n_systems_enabled.then_some(n_systems as isize),
        };
        match sidereon_core::quality::raim(&input, &options) {
            Ok(result) => {
                *out = raim_result_to_c(&result, &residuals);
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_raim", err),
        }
    })
}

/// Copy the normalized residual rows for the direct RAIM test. Rows are ordered
/// by satellite token. Uses the variable-length output contract.
///
/// Safety: inputs match sidereon_raim; out points to len
/// SidereonRaimNormalizedResidual entries or NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_raim_normalized_residuals(
    used_sat_ids: *const *const c_char,
    residuals_m: *const f64,
    count: usize,
    p_fa: f64,
    unit_weights: bool,
    weights: *const SidereonFdeRaimWeight,
    weight_count: usize,
    n_systems_enabled: bool,
    n_systems: i64,
    out: *mut SidereonRaimNormalizedResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_raim_normalized_residuals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_raim_normalized_residuals",
                out_written,
                out_required
            ));
            let (input, _) = c_try!(raim_input_from_c(
                "sidereon_raim_normalized_residuals",
                used_sat_ids,
                residuals_m,
                count
            ));
            let raim_weights = c_try!(raim_weights_from_c(
                "sidereon_raim_normalized_residuals",
                unit_weights,
                weights,
                weight_count
            ));
            let options = RaimOptions {
                p_fa,
                weights: raim_weights,
                n_systems: n_systems_enabled.then_some(n_systems as isize),
            };
            match sidereon_core::quality::raim(&input, &options) {
                Ok(result) => {
                    let rows = raim_normalized_residuals_to_c(&result);
                    c_try!(copy_prefix_to_c(
                        "sidereon_raim_normalized_residuals",
                        "out",
                        &rows,
                        out,
                        len,
                        out_written,
                        out_required,
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_raim_normalized_residuals", err),
            }
        },
    )
}

/// Run RAIM over an SPP receiver solution handle (used satellites + post-fit
/// residuals come from the solution). weights/unit_weights/n_systems mirror
/// sidereon_raim. Delegates to sidereon_core::quality::raim_for_solution.
///
/// Safety: solution is a live SPP-solution handle; weights points to
/// weight_count SidereonFdeRaimWeight when unit_weights is false; out points to a
/// SidereonRaimResult.
#[no_mangle]
pub unsafe extern "C" fn sidereon_raim_for_solution(
    solution: *const SidereonSppSolution,
    p_fa: f64,
    unit_weights: bool,
    weights: *const SidereonFdeRaimWeight,
    weight_count: usize,
    n_systems_enabled: bool,
    n_systems: i64,
    out: *mut SidereonRaimResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_raim_for_solution", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_raim_for_solution", "out"));
        *out = empty_raim_result();
        let solution = c_try!(require_ref(
            solution,
            "sidereon_raim_for_solution",
            "solution"
        ));
        let raim_weights = c_try!(raim_weights_from_c(
            "sidereon_raim_for_solution",
            unit_weights,
            weights,
            weight_count
        ));
        let options = RaimOptions {
            p_fa,
            weights: raim_weights,
            n_systems: n_systems_enabled.then_some(n_systems as isize),
        };
        match sidereon_core::quality::raim_for_solution(&solution.inner, &options) {
            Ok(result) => {
                *out = raim_result_to_c(&result, &solution.inner.residuals_m);
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_raim_for_solution", err),
        }
    })
}

/// Run the standalone range RAIM/FDE over a linearized measurement set. On
/// success writes a newly owned result handle to *out_result. Release it with
/// sidereon_range_fde_result_free. Delegates to
/// sidereon_core::quality::raim_fde_design.
///
/// Safety: rows points to row_count SidereonRangeFdeRow (or NULL when 0);
/// options points to a SidereonRangeFdeOptions; out_result to a
/// SidereonRangeFdeResult*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_raim_fde_design(
    rows: *const SidereonRangeFdeRow,
    row_count: usize,
    options: *const SidereonRangeFdeOptions,
    out_result: *mut *mut SidereonRangeFdeResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_raim_fde_design", SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(
            out_result,
            "sidereon_raim_fde_design",
            "out_result"
        ));
        *out_result = ptr::null_mut();
        let options = c_try!(require_ref(options, "sidereon_raim_fde_design", "options"));
        let rows = c_try!(range_fde_rows_from_c(
            "sidereon_raim_fde_design",
            rows,
            row_count
        ));
        let core_options = RangeFdeOptions {
            p_fa: options.p_fa,
            max_exclusions: options.max_exclusions,
            min_redundancy: options.min_redundancy,
        };
        match raim_fde_design(&rows, &core_options) {
            Ok(inner) => {
                write_boxed_handle(out_result, SidereonRangeFdeResult { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_quality_error("sidereon_raim_fde_design", err),
        }
    })
}

fn map_quality_error(fn_name: &str, err: QualityError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        QualityError::SingularGeometry => SidereonStatus::Solve,
        _ => SidereonStatus::InvalidArgument,
    }
}

unsafe fn raim_input_from_c(
    fn_name: &str,
    used_sat_ids: *const *const c_char,
    residuals_m: *const f64,
    count: usize,
) -> Result<(sidereon_core::quality::RaimInput, Vec<f64>), SidereonStatus> {
    let id_ptrs = require_slice(used_sat_ids, count, fn_name, "used_sat_ids")?;
    let residuals = require_slice(residuals_m, count, fn_name, "residuals_m")?;
    let mut used_sats = Vec::with_capacity(count);
    for ptr in id_ptrs {
        let sat = parse_satellite_token(fn_name, *ptr)?;
        used_sats.push(sat.to_string());
    }
    let residuals = residuals.to_vec();
    Ok((
        sidereon_core::quality::RaimInput {
            used_sats,
            residuals_m: residuals.clone(),
        },
        residuals,
    ))
}

unsafe fn raim_weights_from_c(
    fn_name: &str,
    unit_weights: bool,
    weights: *const SidereonFdeRaimWeight,
    weight_count: usize,
) -> Result<RaimWeights, SidereonStatus> {
    if unit_weights {
        return Ok(RaimWeights::Unit);
    }
    let rows = require_slice(weights, weight_count, fn_name, "weights")?;
    let mut map = BTreeMap::new();
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        map.insert(sat.to_string(), row.weight);
    }
    Ok(RaimWeights::BySatellite(map))
}

fn empty_raim_result() -> SidereonRaimResult {
    SidereonRaimResult {
        fault_detected: false,
        test_statistic: 0.0,
        has_threshold: false,
        threshold: 0.0,
        has_reduced_chi_square: false,
        reduced_chi_square: 0.0,
        rms_m: 0.0,
        dof: 0,
        testable: false,
        normalized_residual_count: 0,
        has_worst_sat: false,
        worst_sat: [0; 17],
    }
}

fn raim_result_to_c(
    value: &sidereon_core::quality::RaimResult,
    residuals_m: &[f64],
) -> SidereonRaimResult {
    let mut out = empty_raim_result();
    out.fault_detected = value.fault_detected;
    out.test_statistic = value.test_statistic;
    if let Some(t) = value.threshold {
        out.has_threshold = true;
        out.threshold = t;
    }
    if value.dof > 0 {
        out.has_reduced_chi_square = true;
        out.reduced_chi_square = value.test_statistic / value.dof as f64;
    }
    out.rms_m = residual_rms_m(residuals_m);
    out.dof = value.dof as i64;
    out.testable = value.testable;
    out.normalized_residual_count = value.normalized_residuals.len();
    if let Some(worst) = &value.worst_sat {
        let bytes = worst.as_bytes();
        if bytes.len() < SATELLITE_TOKEN_C_BYTES {
            for (slot, byte) in out.worst_sat.iter_mut().zip(bytes.iter()) {
                *slot = *byte as c_char;
            }
            out.worst_sat[bytes.len()] = 0;
            out.has_worst_sat = true;
        }
    }
    out
}

fn raim_normalized_residuals_to_c(
    value: &sidereon_core::quality::RaimResult,
) -> Vec<SidereonRaimNormalizedResidual> {
    value
        .normalized_residuals
        .iter()
        .map(
            |(sat_id, normalized_residual)| SidereonRaimNormalizedResidual {
                sat_id: satellite_token_from_text(sat_id),
                normalized_residual: *normalized_residual,
            },
        )
        .collect()
}

fn residual_rms_m(residuals_m: &[f64]) -> f64 {
    if residuals_m.is_empty() {
        return 0.0;
    }
    let sum_squares: f64 = residuals_m.iter().map(|residual| residual * residual).sum();
    (sum_squares / residuals_m.len() as f64).sqrt()
}

unsafe fn range_fde_rows_from_c(
    fn_name: &str,
    rows: *const SidereonRangeFdeRow,
    row_count: usize,
) -> Result<Vec<RangeFdeRow>, SidereonStatus> {
    let raw_rows = require_slice(rows, row_count, fn_name, "rows")?;
    validate_element_count::<RangeFdeRow>(fn_name, "row_count", raw_rows.len())?;
    let mut out = Vec::with_capacity(raw_rows.len());
    for (idx, row) in raw_rows.iter().enumerate() {
        let id = parse_bounded_c_string(
            fn_name,
            &format!("rows[{idx}].id"),
            row.id,
            MAX_RTK_ID_BYTES,
        )?;
        let design_row = require_slice(
            row.design_row,
            row.design_dim,
            fn_name,
            &format!("rows[{idx}].design_row"),
        )?;
        out.push(RangeFdeRow {
            id,
            residual_m: row.residual_m,
            design_row: design_row.to_vec(),
            weight: row.weight,
        });
    }
    Ok(out)
}
