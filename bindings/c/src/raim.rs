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
    /// Redundancy degrees of freedom.
    pub dof: i64,
    /// Whether the geometry was testable.
    pub testable: bool,
    /// Whether worst_sat carries a satellite token.
    pub has_worst_sat: bool,
    /// Worst-residual satellite token, null-terminated (valid when
    /// has_worst_sat). Sized to hold any GNSS token (16 bytes) plus the
    /// terminator; kept in step with SATELLITE_TOKEN_C_BYTES by the assert below.
    pub worst_sat: [c_char; 17],
}

/// Run the RAIM chi-square test over used satellites and their residuals.
/// weights/unit_weights/n_systems mirror SidereonFdeOptions. Delegates to
/// sidereon_core::quality::raim.
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
        *out = SidereonRaimResult {
            fault_detected: false,
            test_statistic: 0.0,
            has_threshold: false,
            threshold: 0.0,
            dof: 0,
            testable: false,
            has_worst_sat: false,
            worst_sat: [0; 17],
        };
        let id_ptrs = c_try!(require_slice(
            used_sat_ids,
            count,
            "sidereon_raim",
            "used_sat_ids"
        ));
        let residuals = c_try!(require_slice(
            residuals_m,
            count,
            "sidereon_raim",
            "residuals_m"
        ));
        let mut used_sats = Vec::with_capacity(count);
        for ptr in id_ptrs {
            let sat = c_try!(parse_satellite_token("sidereon_raim", *ptr));
            used_sats.push(sat.to_string());
        }
        let raim_weights = if unit_weights {
            RaimWeights::Unit
        } else {
            let rows = c_try!(require_slice(
                weights,
                weight_count,
                "sidereon_raim",
                "weights"
            ));
            let mut map = BTreeMap::new();
            for row in rows {
                let sat = c_try!(parse_satellite_token("sidereon_raim", row.sat_id));
                map.insert(sat.to_string(), row.weight);
            }
            RaimWeights::BySatellite(map)
        };
        let options = RaimOptions {
            p_fa,
            weights: raim_weights,
            n_systems: n_systems_enabled.then_some(n_systems as isize),
        };
        let input = sidereon_core::quality::RaimInput {
            used_sats,
            residuals_m: residuals.to_vec(),
        };
        match sidereon_core::quality::raim(&input, &options) {
            Ok(result) => {
                out.fault_detected = result.fault_detected;
                out.test_statistic = result.test_statistic;
                if let Some(t) = result.threshold {
                    out.has_threshold = true;
                    out.threshold = t;
                }
                out.dof = result.dof as i64;
                out.testable = result.testable;
                if let Some(worst) = result.worst_sat {
                    let bytes = worst.as_bytes();
                    if bytes.len() < SATELLITE_TOKEN_C_BYTES {
                        for (slot, b) in out.worst_sat.iter_mut().zip(bytes.iter()) {
                            *slot = *b as c_char;
                        }
                        out.worst_sat[bytes.len()] = 0;
                        out.has_worst_sat = true;
                    }
                }
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_raim", err),
        }
    })
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
        *out = SidereonRaimResult {
            fault_detected: false,
            test_statistic: 0.0,
            has_threshold: false,
            threshold: 0.0,
            dof: 0,
            testable: false,
            has_worst_sat: false,
            worst_sat: [0; 17],
        };
        let solution = c_try!(require_ref(
            solution,
            "sidereon_raim_for_solution",
            "solution"
        ));
        let raim_weights = if unit_weights {
            RaimWeights::Unit
        } else {
            let rows = c_try!(require_slice(
                weights,
                weight_count,
                "sidereon_raim_for_solution",
                "weights"
            ));
            let mut map = BTreeMap::new();
            for row in rows {
                let sat = c_try!(parse_satellite_token(
                    "sidereon_raim_for_solution",
                    row.sat_id
                ));
                map.insert(sat.to_string(), row.weight);
            }
            RaimWeights::BySatellite(map)
        };
        let options = RaimOptions {
            p_fa,
            weights: raim_weights,
            n_systems: n_systems_enabled.then_some(n_systems as isize),
        };
        match sidereon_core::quality::raim_for_solution(&solution.inner, &options) {
            Ok(result) => {
                out.fault_detected = result.fault_detected;
                out.test_statistic = result.test_statistic;
                if let Some(t) = result.threshold {
                    out.has_threshold = true;
                    out.threshold = t;
                }
                out.dof = result.dof as i64;
                out.testable = result.testable;
                if let Some(worst) = result.worst_sat {
                    let bytes = worst.as_bytes();
                    if bytes.len() < SATELLITE_TOKEN_C_BYTES {
                        for (slot, b) in out.worst_sat.iter_mut().zip(bytes.iter()) {
                            *slot = *b as c_char;
                        }
                        out.worst_sat[bytes.len()] = 0;
                        out.has_worst_sat = true;
                    }
                }
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
