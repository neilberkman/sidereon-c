use super::*;

// --- Classical reliability design -----------------------------------------

/// Baarda w-test constants for one false-alarm and missed-detection pair.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonWTestNoncentrality {
    /// Minimal normalized bias.
    pub delta0: f64,
    /// Noncentrality parameter, equal to delta0 squared.
    pub lambda0: f64,
}

/// Options for classical internal and external reliability design.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReliabilityOptions {
    /// Two-sided false-alarm probability for one w-test.
    pub alpha: f64,
    /// Missed-detection probability for the target bias.
    pub beta: f64,
    /// Whether lambda0_override carries a precomputed noncentrality value.
    pub has_lambda0_override: bool,
    /// Precomputed noncentrality value, used only when has_lambda0_override is true.
    pub lambda0_override: f64,
    /// Redundancy below which an observation is reported as uncheckable.
    pub min_redundancy: f64,
}

/// One range row for pre-data reliability design.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangeReliabilityRow {
    /// Null-terminated observation identifier, at most 64 bytes.
    pub id: *const c_char,
    /// Linearized design row for this range observation.
    pub design_row: *const f64,
    /// Number of entries in design_row.
    pub design_dim: usize,
    /// Externally supplied one-sigma range model, meters.
    pub sigma_m: f64,
}

/// Reliability diagnostics for one observation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservationReliability {
    /// Observation identifier, null-terminated.
    pub id: SidereonRtkId,
    /// Redundancy number, the checked fraction of this observation.
    pub redundancy: f64,
    /// Whether mdb_m carries a minimal detectable bias.
    pub has_mdb_m: bool,
    /// Minimal detectable bias, meters, when present.
    pub mdb_m: f64,
    /// Whether external_enu_m carries an external effect vector.
    pub has_external_enu_m: bool,
    /// External effect vector, meters, when present.
    pub external_enu_m: [f64; 3],
    /// Whether bias_to_noise carries a state-space bias-to-noise ratio.
    pub has_bias_to_noise: bool,
    /// Bias-to-noise ratio, when present.
    pub bias_to_noise: f64,
    /// True when the observation redundancy is below the configured floor.
    pub uncheckable: bool,
}

/// Aggregate reliability diagnostics for a design.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReliabilitySummary {
    /// Number of observations in the design.
    pub n_obs: usize,
    /// Number of estimated parameters in the design.
    pub n_params: usize,
    /// Algebraic degrees of freedom.
    pub dof: usize,
    /// Sum of per-observation redundancy numbers.
    pub sum_redundancy: f64,
    /// Noncentrality parameter used for MDB calculations.
    pub lambda0: f64,
    /// Whether max_mdb_id and max_mdb_m carry a checkable observation.
    pub has_max_mdb_m: bool,
    /// Observation identifier for the largest finite MDB.
    pub max_mdb_id: SidereonRtkId,
    /// Largest finite MDB, meters, when present.
    pub max_mdb_m: f64,
    /// Observation identifier for the smallest redundancy number.
    pub min_redundancy_id: SidereonRtkId,
    /// Smallest redundancy number.
    pub min_redundancy: f64,
    /// Count of observations reported as uncheckable.
    pub n_uncheckable: usize,
}

/// Classical reliability report handle. Release with sidereon_reliability_report_free.
pub struct SidereonReliabilityReport {
    pub(crate) inner: CoreReliabilityReport,
}

/// Initialize reliability options with the engine defaults.
///
/// Safety: out_options must point to writable SidereonReliabilityOptions storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_options_init(
    out_options: *mut SidereonReliabilityOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reliability_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_reliability_options_init",
                "out_options"
            ));
            *out = reliability_options_to_c(CoreReliabilityOptions::default());
            SidereonStatus::Ok
        },
    )
}

/// Compute Baarda w-test delta0 and lambda0 for one alpha and beta pair.
///
/// Safety: out must point to writable SidereonWTestNoncentrality storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_wtest_noncentrality(
    alpha: f64,
    beta: f64,
    out: *mut SidereonWTestNoncentrality,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_wtest_noncentrality",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_wtest_noncentrality", "out"));
            *out = SidereonWTestNoncentrality {
                delta0: 0.0,
                lambda0: 0.0,
            };
            match core_wtest_noncentrality_components(alpha, beta) {
                Ok(components) => {
                    *out = SidereonWTestNoncentrality {
                        delta0: components.delta0,
                        lambda0: components.lambda0,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => map_reliability_quality_error("sidereon_wtest_noncentrality", err),
            }
        },
    )
}

/// Compute reliability from supplied range design rows.
///
/// On success, *out_report receives a newly owned handle. Release it with
/// sidereon_reliability_report_free.
///
/// Safety: rows points to row_count SidereonRangeReliabilityRow entries,
/// options points to SidereonReliabilityOptions, and out_report points to a
/// SidereonReliabilityReport pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_design(
    rows: *const SidereonRangeReliabilityRow,
    row_count: usize,
    options: *const SidereonReliabilityOptions,
    out_report: *mut *mut SidereonReliabilityReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_reliability_design", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_reliability_design",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let options = c_try!(require_ref(
            options,
            "sidereon_reliability_design",
            "options"
        ));
        let rows = c_try!(reliability_rows_from_c(
            "sidereon_reliability_design",
            rows,
            row_count
        ));
        match core_reliability_design(&rows, &reliability_options_from_c(options)) {
            Ok(inner) => {
                write_boxed_handle(out_report, SidereonReliabilityReport { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_reliability_quality_error("sidereon_reliability_design", err),
        }
    })
}

/// Compute reliability for ARAIM geometry and an integrity support message.
///
/// On success, *out_report receives a newly owned handle. Release it with
/// sidereon_reliability_report_free.
///
/// Safety: geometry, ism, options, and out_report must point to their documented
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_araim(
    geometry: *const SidereonAraimGeometry,
    ism: *const SidereonAraimIsm,
    options: *const SidereonReliabilityOptions,
    out_report: *mut *mut SidereonReliabilityReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_reliability_araim", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_reliability_araim",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let geometry = c_try!(require_ref(
            geometry,
            "sidereon_reliability_araim",
            "geometry"
        ));
        let ism = c_try!(require_ref(ism, "sidereon_reliability_araim", "ism"));
        let options = c_try!(require_ref(
            options,
            "sidereon_reliability_araim",
            "options"
        ));
        let geometry = c_try!(araim_geometry_from_c(
            "sidereon_reliability_araim",
            geometry
        ));
        let ism = c_try!(araim_ism_from_c("sidereon_reliability_araim", ism));
        match core_reliability_araim(&geometry, &ism, &reliability_options_from_c(options)) {
            Ok(inner) => {
                write_boxed_handle(out_report, SidereonReliabilityReport { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_reliability_araim_error("sidereon_reliability_araim", err),
        }
    })
}

/// Read the aggregate reliability summary from a report handle.
///
/// Safety: report must be a live handle; out_summary must point to writable
/// SidereonReliabilitySummary storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_report_summary(
    report: *const SidereonReliabilityReport,
    out_summary: *mut SidereonReliabilitySummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reliability_report_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_reliability_report_summary",
                "out_summary"
            ));
            *out = empty_reliability_summary();
            let report = c_try!(require_ref(
                report,
                "sidereon_reliability_report_summary",
                "report"
            ));
            *out = reliability_summary_to_c(&report.inner.summary);
            SidereonStatus::Ok
        },
    )
}

/// Copy per-observation reliability rows from a report handle.
///
/// Uses the variable-length output contract. Pass out NULL with len 0 to query
/// the required row count.
///
/// Safety: report must be a live handle; out points to len entries or NULL when
/// len is 0; out_written and out_required must point to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_report_observations(
    report: *const SidereonReliabilityReport,
    out: *mut SidereonObservationReliability,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reliability_report_observations",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_reliability_report_observations",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_reliability_report_observations",
                "report"
            ));
            let rows = report
                .inner
                .per_observation
                .iter()
                .map(observation_reliability_to_c)
                .collect::<Vec<_>>();
            c_try!(copy_prefix_to_c(
                "sidereon_reliability_report_observations",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a reliability report handle. Passing NULL is a no-op.
///
/// Safety: report must be NULL or a live handle from a reliability function.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reliability_report_free(report: *mut SidereonReliabilityReport) {
    ffi_boundary("sidereon_reliability_report_free", (), || {
        free_boxed(report);
    });
}

fn reliability_options_to_c(value: CoreReliabilityOptions) -> SidereonReliabilityOptions {
    SidereonReliabilityOptions {
        alpha: value.alpha,
        beta: value.beta,
        has_lambda0_override: value.lambda0_override.is_some(),
        lambda0_override: value.lambda0_override.unwrap_or(0.0),
        min_redundancy: value.min_redundancy,
    }
}

fn reliability_options_from_c(value: &SidereonReliabilityOptions) -> CoreReliabilityOptions {
    CoreReliabilityOptions {
        alpha: value.alpha,
        beta: value.beta,
        lambda0_override: value.has_lambda0_override.then_some(value.lambda0_override),
        min_redundancy: value.min_redundancy,
    }
}

unsafe fn reliability_rows_from_c(
    fn_name: &str,
    rows: *const SidereonRangeReliabilityRow,
    row_count: usize,
) -> Result<Vec<CoreRangeReliabilityRow>, SidereonStatus> {
    let raw_rows = require_slice(rows, row_count, fn_name, "rows")?;
    validate_element_count::<CoreRangeReliabilityRow>(fn_name, "row_count", raw_rows.len())?;
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
        out.push(CoreRangeReliabilityRow {
            id,
            design_row: design_row.to_vec(),
            sigma_m: row.sigma_m,
        });
    }
    Ok(out)
}

fn observation_reliability_to_c(
    value: &CoreObservationReliability,
) -> SidereonObservationReliability {
    SidereonObservationReliability {
        id: rtk_id_token(&value.id),
        redundancy: value.redundancy,
        has_mdb_m: value.mdb_m.is_some(),
        mdb_m: value.mdb_m.unwrap_or(0.0),
        has_external_enu_m: value.external_enu_m.is_some(),
        external_enu_m: value.external_enu_m.unwrap_or([0.0; 3]),
        has_bias_to_noise: value.bias_to_noise.is_some(),
        bias_to_noise: value.bias_to_noise.unwrap_or(0.0),
        uncheckable: value.uncheckable,
    }
}

fn reliability_summary_to_c(value: &CoreReliabilitySummary) -> SidereonReliabilitySummary {
    let (has_max_mdb_m, max_mdb_id, max_mdb_m) = match &value.max_mdb_m {
        Some((id, mdb_m)) => (true, rtk_id_token(id), *mdb_m),
        None => (false, rtk_id_token(""), 0.0),
    };
    SidereonReliabilitySummary {
        n_obs: value.n_obs,
        n_params: value.n_params,
        dof: value.dof,
        sum_redundancy: value.sum_redundancy,
        lambda0: value.lambda0,
        has_max_mdb_m,
        max_mdb_id,
        max_mdb_m,
        min_redundancy_id: rtk_id_token(&value.min_redundancy.0),
        min_redundancy: value.min_redundancy.1,
        n_uncheckable: value.n_uncheckable,
    }
}

fn empty_reliability_summary() -> SidereonReliabilitySummary {
    SidereonReliabilitySummary {
        n_obs: 0,
        n_params: 0,
        dof: 0,
        sum_redundancy: 0.0,
        lambda0: 0.0,
        has_max_mdb_m: false,
        max_mdb_id: rtk_id_token(""),
        max_mdb_m: 0.0,
        min_redundancy_id: rtk_id_token(""),
        min_redundancy: 0.0,
        n_uncheckable: 0,
    }
}

fn map_reliability_quality_error(fn_name: &str, err: QualityError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        QualityError::SingularGeometry => SidereonStatus::Solve,
        _ => SidereonStatus::InvalidArgument,
    }
}

fn map_reliability_araim_error(fn_name: &str, err: AraimError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        AraimError::InvalidIsm | AraimError::InvalidAllocation => SidereonStatus::InvalidArgument,
        AraimError::InsufficientGeometry
        | AraimError::UnmonitorableFaultMass
        | AraimError::NumericalFailure => SidereonStatus::Solve,
    }
}
