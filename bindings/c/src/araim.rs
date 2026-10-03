use super::*;
use crate::engine_error::{
    degrade_reason_name, engine_error_operation_boundary, record_engine_error,
    SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

// --- ARAIM integrity (sidereon_core::araim) ---------------------------------

/// One satellite row in an ARAIM geometry snapshot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimRow {
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
    /// Receiver-to-satellite ECEF unit line of sight.
    pub line_of_sight: SidereonLineOfSight,
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Elevation angle at the receiver, radians.
    pub elevation_rad: f64,
}

/// ARAIM geometry input. `rows` and `clock_systems` are caller-owned arrays.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimGeometry {
    /// Satellite rows.
    pub rows: *const SidereonAraimRow,
    /// Number of satellite rows.
    pub row_count: usize,
    /// Receiver WGS84 geodetic position.
    pub receiver: SidereonGeodetic,
    /// Receiver-clock systems as SidereonGnssSystem values.
    pub clock_systems: *const u32,
    /// Number of receiver-clock systems.
    pub clock_system_count: usize,
}

/// Per-satellite ARAIM integrity and accuracy model without an identity.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimSatelliteIsmModel {
    /// Integrity one-sigma SIS range error, meters.
    pub sigma_ura_m: f64,
    /// Accuracy and continuity one-sigma SIS range error, meters.
    pub sigma_ure_m: f64,
    /// Whether effective_sigma_int_m overrides the derived integrity sigma.
    pub has_effective_sigma_int_m: bool,
    /// Effective integrity one-sigma range error after local terms, meters.
    pub effective_sigma_int_m: f64,
    /// Whether effective_sigma_acc_m overrides the derived accuracy sigma.
    pub has_effective_sigma_acc_m: bool,
    /// Effective accuracy one-sigma range error after local terms, meters.
    pub effective_sigma_acc_m: f64,
    /// Nominal SIS bias bound, meters.
    pub b_nom_m: f64,
    /// Prior probability for a satellite fault.
    pub p_sat: f64,
}

/// Per-constellation ARAIM ISM default.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimConstellationIsm {
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Prior probability for a constellation-wide fault.
    pub p_const: f64,
    /// Default satellite model for this constellation.
    pub default_sat: SidereonAraimSatelliteIsmModel,
}

/// Per-satellite ARAIM ISM override.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimSatelliteIsm {
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
    /// Integrity one-sigma SIS range error, meters.
    pub sigma_ura_m: f64,
    /// Accuracy and continuity one-sigma SIS range error, meters.
    pub sigma_ure_m: f64,
    /// Whether effective_sigma_int_m overrides the derived integrity sigma.
    pub has_effective_sigma_int_m: bool,
    /// Effective integrity one-sigma range error after local terms, meters.
    pub effective_sigma_int_m: f64,
    /// Whether effective_sigma_acc_m overrides the derived accuracy sigma.
    pub has_effective_sigma_acc_m: bool,
    /// Effective accuracy one-sigma range error after local terms, meters.
    pub effective_sigma_acc_m: f64,
    /// Nominal SIS bias bound, meters.
    pub b_nom_m: f64,
    /// Prior probability for a satellite fault.
    pub p_sat: f64,
}

/// ARAIM integrity support message input. Arrays are caller-owned.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimIsm {
    /// Per-constellation defaults.
    pub constellations: *const SidereonAraimConstellationIsm,
    /// Number of constellation rows.
    pub constellation_count: usize,
    /// Per-satellite overrides.
    pub satellites: *const SidereonAraimSatelliteIsm,
    /// Number of satellite override rows.
    pub satellite_count: usize,
}

/// Integrity and continuity risk allocation for one ARAIM solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimIntegrityAllocation {
    /// Total probability of hazardous misleading information.
    pub phmi_total: f64,
    /// Vertical PHMI allocation.
    pub phmi_vert: f64,
    /// Horizontal PHMI allocation.
    pub phmi_hor: f64,
    /// Vertical false-alert allocation.
    pub pfa_vert: f64,
    /// Horizontal false-alert allocation.
    pub pfa_hor: f64,
    /// Maximum acceptable unmonitored fault probability mass.
    pub p_threshold_unmonitored: f64,
    /// Fault-prior threshold used for the effective monitor threshold.
    pub p_emt: f64,
    /// Maximum enumerated satellite-fault order. Zero keeps only fault-free.
    pub max_fault_order: usize,
}

/// ARAIM protection-level summary. HPL, VPL, EMT, and accuracy sigma fields are
/// meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimSummary {
    /// Horizontal protection level, meters.
    pub hpl_m: f64,
    /// Vertical protection level, meters.
    pub vpl_m: f64,
    /// All-in-view horizontal accuracy sigma, meters.
    pub sigma_acc_h_m: f64,
    /// All-in-view vertical accuracy sigma, meters.
    pub sigma_acc_v_m: f64,
    /// Effective monitor threshold, meters.
    pub emt_m: f64,
    /// Unenumerated plus unmonitorable fault probability mass.
    pub p_unmonitored: f64,
    /// True when ARAIM met the allocation and all roots converged.
    pub available: bool,
    /// Alias for available, kept for compatibility.
    pub availability: bool,
    /// Number of fault-mode rows available.
    pub fault_mode_count: usize,
}

/// One ARAIM fault-mode row. Sigma, bias, and threshold arrays are local
/// `[east, north, up]` meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAraimFaultMode {
    /// Number of excluded satellites for this mode.
    pub excluded_count: usize,
    /// Whether excluded_constellation carries a GNSS system.
    pub has_excluded_constellation: bool,
    /// Excluded constellation as SidereonGnssSystem when present.
    pub excluded_constellation: u32,
    /// Fault prior probability for this mode.
    pub prior: f64,
    /// Integrity sigma in local ENU, meters.
    pub sigma_int_enu_m: [f64; 3],
    /// Nominal bias bound in local ENU, meters.
    pub bias_enu_m: [f64; 3],
    /// Separation monitor threshold in local ENU, meters.
    pub threshold_enu_m: [f64; 3],
    /// True when the subset geometry is full rank.
    pub monitorable: bool,
}

/// ARAIM protection-level result. Opaque to C. Create with sidereon_araim and
/// release with sidereon_araim_result_free.
pub struct SidereonAraimResult {
    pub(crate) inner: CoreAraimResult,
}

/// Initialize the ARAIM LPV-200 integrity allocation.
///
/// Safety: out_allocation must point to a SidereonAraimIntegrityAllocation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim_allocation_lpv_200(
    out_allocation: *mut SidereonAraimIntegrityAllocation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_araim_allocation_lpv_200",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_allocation,
                "sidereon_araim_allocation_lpv_200",
                "out_allocation"
            ));
            *out = araim_allocation_to_c(IntegrityAllocation::lpv_200());
            SidereonStatus::Ok
        },
    )
}

/// Run the ARAIM multi-hypothesis protection-level solve. HPL, VPL, EMT, and
/// accuracy sigma outputs are meters and are read from the result summary.
///
/// Safety: geometry, ism, allocation, and out_result must point to their
/// documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim(
    geometry: *const SidereonAraimGeometry,
    ism: *const SidereonAraimIsm,
    allocation: *const SidereonAraimIntegrityAllocation,
    out_result: *mut *mut SidereonAraimResult,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_araim", SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, "sidereon_araim", "out_result"));
        *out_result = ptr::null_mut();
        let geometry = c_try!(require_ref(geometry, "sidereon_araim", "geometry"));
        let ism = c_try!(require_ref(ism, "sidereon_araim", "ism"));
        let allocation = c_try!(require_ref(allocation, "sidereon_araim", "allocation"));
        let geometry = c_try!(araim_geometry_from_c("sidereon_araim", geometry));
        let ism = c_try!(araim_ism_from_c("sidereon_araim", ism));
        let allocation = araim_allocation_from_c(allocation);
        match core_araim(&geometry, &ism, &allocation) {
            Ok(inner) => {
                write_boxed_handle(out_result, SidereonAraimResult { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_araim_error("sidereon_araim", err),
        }
    })
}

/// Read ARAIM result summary fields. HPL, VPL, EMT, and accuracy sigma fields
/// are meters.
///
/// Safety: result must be a live handle; out_summary must point to a
/// SidereonAraimSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim_result_summary(
    result: *const SidereonAraimResult,
    out_summary: *mut SidereonAraimSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_araim_result_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_araim_result_summary",
                "out_summary"
            ));
            *out = SidereonAraimSummary {
                hpl_m: 0.0,
                vpl_m: 0.0,
                sigma_acc_h_m: 0.0,
                sigma_acc_v_m: 0.0,
                emt_m: 0.0,
                p_unmonitored: 0.0,
                available: false,
                availability: false,
                fault_mode_count: 0,
            };
            let result = c_try!(require_ref(
                result,
                "sidereon_araim_result_summary",
                "result"
            ));
            *out = araim_summary_to_c(&result.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy ARAIM fault-mode rows. Sigma, bias, and threshold arrays are meters in
/// local `[east, north, up]` order. Uses the variable-length output contract.
///
/// Safety: result must be a live handle; out points to len SidereonAraimFaultMode
/// entries or NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim_result_fault_modes(
    result: *const SidereonAraimResult,
    out: *mut SidereonAraimFaultMode,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_araim_result_fault_modes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_araim_result_fault_modes",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_araim_result_fault_modes",
                "result"
            ));
            let values: Vec<SidereonAraimFaultMode> = result
                .inner
                .fault_modes
                .iter()
                .map(araim_fault_mode_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_araim_result_fault_modes",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy excluded satellite tokens for one ARAIM fault mode. Uses the
/// variable-length output contract.
///
/// Safety: result must be a live handle; out points to len
/// SidereonSatelliteToken entries or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim_result_fault_mode_excluded_sats(
    result: *const SidereonAraimResult,
    mode_index: usize,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_araim_result_fault_mode_excluded_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_araim_result_fault_mode_excluded_sats",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_araim_result_fault_mode_excluded_sats",
                "result"
            ));
            let Some(mode) = result.inner.fault_modes.get(mode_index) else {
                set_last_error(format!(
                    "sidereon_araim_result_fault_mode_excluded_sats: mode_index {mode_index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            let values: Vec<SidereonSatelliteToken> =
                mode.excluded.iter().copied().map(satellite_token).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_araim_result_fault_mode_excluded_sats",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release an ARAIM result handle. Passing NULL is a no-op.
///
/// Safety: result must be NULL or a live handle from sidereon_araim.
#[no_mangle]
pub unsafe extern "C" fn sidereon_araim_result_free(result: *mut SidereonAraimResult) {
    ffi_boundary("sidereon_araim_result_free", (), || {
        free_boxed(result);
    });
}

fn araim_allocation_to_c(value: IntegrityAllocation) -> SidereonAraimIntegrityAllocation {
    SidereonAraimIntegrityAllocation {
        phmi_total: value.phmi_total,
        phmi_vert: value.phmi_vert,
        phmi_hor: value.phmi_hor,
        pfa_vert: value.pfa_vert,
        pfa_hor: value.pfa_hor,
        p_threshold_unmonitored: value.p_threshold_unmonitored,
        p_emt: value.p_emt,
        max_fault_order: value.max_fault_order,
    }
}

fn araim_allocation_from_c(value: &SidereonAraimIntegrityAllocation) -> IntegrityAllocation {
    IntegrityAllocation {
        phmi_total: value.phmi_total,
        phmi_vert: value.phmi_vert,
        phmi_hor: value.phmi_hor,
        pfa_vert: value.pfa_vert,
        pfa_hor: value.pfa_hor,
        p_threshold_unmonitored: value.p_threshold_unmonitored,
        p_emt: if value.p_emt == 0.0 {
            1.0e-5
        } else {
            value.p_emt
        },
        max_fault_order: value.max_fault_order,
    }
}

pub(crate) unsafe fn araim_geometry_from_c(
    fn_name: &str,
    value: &SidereonAraimGeometry,
) -> Result<AraimGeometry, SidereonStatus> {
    let rows = require_slice(value.rows, value.row_count, fn_name, "geometry.rows")?;
    let mut parsed_rows = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let id = parse_satellite_token(fn_name, row.sat_id)?;
        let system =
            gnss_system_from_c_code(fn_name, &format!("geometry.rows[{idx}].system"), row.system)?;
        parsed_rows.push(AraimRow {
            id,
            line_of_sight: LineOfSight::new(
                row.line_of_sight.e_x,
                row.line_of_sight.e_y,
                row.line_of_sight.e_z,
            ),
            system,
            elevation_rad: row.elevation_rad,
        });
    }
    let receiver = geodetic_to_wgs84(fn_name, "geometry.receiver", value.receiver)?;
    let raw_systems = require_slice(
        value.clock_systems,
        value.clock_system_count,
        fn_name,
        "geometry.clock_systems",
    )?;
    let mut clock_systems = Vec::with_capacity(raw_systems.len());
    for (idx, &system) in raw_systems.iter().enumerate() {
        clock_systems.push(gnss_system_from_c_code(
            fn_name,
            &format!("geometry.clock_systems[{idx}]"),
            system,
        )?);
    }
    // The caller supplies the rows directly, so no UT1 was read to form them.
    Ok(AraimGeometry {
        rows: parsed_rows,
        receiver,
        clock_systems,
        ut1_degraded: None,
    })
}

pub(crate) unsafe fn araim_ism_from_c(
    fn_name: &str,
    value: &SidereonAraimIsm,
) -> Result<Ism, SidereonStatus> {
    let raw_constellations = require_slice(
        value.constellations,
        value.constellation_count,
        fn_name,
        "ism.constellations",
    )?;
    let mut constellations = Vec::with_capacity(raw_constellations.len());
    for (idx, row) in raw_constellations.iter().enumerate() {
        let system = gnss_system_from_c_code(
            fn_name,
            &format!("ism.constellations[{idx}].system"),
            row.system,
        )?;
        constellations.push(ConstellationIsm::new(
            system,
            row.p_const,
            araim_sat_model_from_c(row.default_sat),
        ));
    }

    let raw_satellites = require_slice(
        value.satellites,
        value.satellite_count,
        fn_name,
        "ism.satellites",
    )?;
    let mut satellites = Vec::with_capacity(raw_satellites.len());
    for row in raw_satellites {
        let id = parse_satellite_token(fn_name, row.sat_id)?;
        satellites.push(
            if row.has_effective_sigma_int_m || row.has_effective_sigma_acc_m {
                SatelliteIsm::new_with_effective_sigmas(
                    id,
                    row.sigma_ura_m,
                    row.sigma_ure_m,
                    row.b_nom_m,
                    row.p_sat,
                    row.effective_sigma_int_m,
                    row.effective_sigma_acc_m,
                )
            } else {
                SatelliteIsm::new(id, row.sigma_ura_m, row.sigma_ure_m, row.b_nom_m, row.p_sat)
            },
        );
    }
    Ok(Ism::new(constellations, satellites))
}

fn araim_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn araim_error_value(error: &AraimError) -> Value {
    use AraimError as E;
    match error {
        E::InsufficientGeometry => araim_node("insufficient_geometry", json!({})),
        E::UnmonitorableFaultMass => araim_node("unmonitorable_fault_mass", json!({})),
        E::NumericalFailure => araim_node("numerical_failure", json!({})),
        E::InvalidIsm => araim_node("invalid_ism", json!({})),
        E::InvalidAllocation => araim_node("invalid_allocation", json!({})),
        E::Ut1OutsideCoverage(reason) => araim_node(
            "ut1_outside_coverage",
            json!({
                "reason": degrade_reason_name(*reason),
            }),
        ),
    }
}

pub(crate) fn map_araim_error_retaining(fn_name: &str, err: &AraimError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        AraimError::InvalidIsm | AraimError::InvalidAllocation => SidereonStatus::InvalidArgument,
        AraimError::InsufficientGeometry
        | AraimError::UnmonitorableFaultMass
        | AraimError::NumericalFailure => SidereonStatus::Solve,
        AraimError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
    }
}

fn map_araim_error(fn_name: &str, err: AraimError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Araim,
        fn_name,
        araim_error_value(&err),
    );
    map_araim_error_retaining(fn_name, &err)
}

fn araim_summary_to_c(value: &CoreAraimResult) -> SidereonAraimSummary {
    SidereonAraimSummary {
        hpl_m: value.hpl_m,
        vpl_m: value.vpl_m,
        sigma_acc_h_m: value.sigma_acc_h_m,
        sigma_acc_v_m: value.sigma_acc_v_m,
        emt_m: value.emt_m,
        p_unmonitored: value.p_unmonitored,
        available: value.available,
        availability: value.available,
        fault_mode_count: value.fault_modes.len(),
    }
}

fn araim_fault_mode_to_c(value: &sidereon_core::araim::FaultMode) -> SidereonAraimFaultMode {
    SidereonAraimFaultMode {
        excluded_count: value.excluded.len(),
        has_excluded_constellation: value.excluded_constellation.is_some(),
        excluded_constellation: value
            .excluded_constellation
            .map(|system| gnss_system_to_c(system) as u32)
            .unwrap_or(SidereonGnssSystem::Gps as u32),
        prior: value.prior,
        sigma_int_enu_m: value.sigma_int_enu_m,
        bias_enu_m: value.bias_enu_m,
        threshold_enu_m: value.threshold_enu_m,
        monitorable: value.monitorable,
    }
}

fn araim_sat_model_from_c(value: SidereonAraimSatelliteIsmModel) -> SatelliteIsmModel {
    if value.has_effective_sigma_int_m || value.has_effective_sigma_acc_m {
        SatelliteIsmModel::new_with_effective_sigmas(
            value.sigma_ura_m,
            value.sigma_ure_m,
            value.b_nom_m,
            value.p_sat,
            value.effective_sigma_int_m,
            value.effective_sigma_acc_m,
        )
    } else {
        SatelliteIsmModel::new(
            value.sigma_ura_m,
            value.sigma_ure_m,
            value.b_nom_m,
            value.p_sat,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorInfo,
    };
    use sidereon_core::astro::time::DegradeReason;

    #[test]
    fn table_driven_araim_error_mapping() {
        let cases = [
            (
                AraimError::InsufficientGeometry,
                "insufficient_geometry",
                json!({}),
                SidereonStatus::Solve,
            ),
            (
                AraimError::UnmonitorableFaultMass,
                "unmonitorable_fault_mass",
                json!({}),
                SidereonStatus::Solve,
            ),
            (
                AraimError::NumericalFailure,
                "numerical_failure",
                json!({}),
                SidereonStatus::Solve,
            ),
            (
                AraimError::InvalidIsm,
                "invalid_ism",
                json!({}),
                SidereonStatus::InvalidArgument,
            ),
            (
                AraimError::InvalidAllocation,
                "invalid_allocation",
                json!({}),
                SidereonStatus::InvalidArgument,
            ),
            (
                AraimError::Ut1OutsideCoverage(DegradeReason::BeforeCoverage),
                "ut1_outside_coverage",
                json!({"reason": "before_coverage"}),
                SidereonStatus::Ut1OutsideCoverage,
            ),
            (
                AraimError::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
                "ut1_outside_coverage",
                json!({"reason": "after_coverage"}),
                SidereonStatus::Ut1OutsideCoverage,
            ),
        ];

        for (err, kind, fields, status) in cases {
            let v = araim_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            assert_eq!(v["fields"], fields);
            assert_eq!(map_araim_error_retaining("test", &err), status);
        }
    }

    const INV_SQRT_3: f64 = 0.577_350_269_189_625_8;

    #[test]
    fn araim_producer_real_refusal_valid_control_and_retention() {
        clear_engine_error();

        let constellation = SidereonAraimConstellationIsm {
            system: SidereonGnssSystem::Gps as u32,
            p_const: 0.0,
            default_sat: SidereonAraimSatelliteIsmModel {
                sigma_ura_m: 2.0,
                sigma_ure_m: 1.0,
                has_effective_sigma_int_m: false,
                effective_sigma_int_m: 0.0,
                has_effective_sigma_acc_m: false,
                effective_sigma_acc_m: 0.0,
                b_nom_m: 0.25,
                p_sat: 0.0,
            },
        };
        let ism = SidereonAraimIsm {
            constellations: &constellation,
            constellation_count: 1,
            satellites: ptr::null(),
            satellite_count: 0,
        };

        let allocation = SidereonAraimIntegrityAllocation {
            phmi_total: 1.0e-7,
            phmi_vert: 9.8e-8,
            phmi_hor: 2.0e-9,
            pfa_vert: 3.9e-6,
            pfa_hor: 9.0e-8,
            p_threshold_unmonitored: 0.0,
            p_emt: 1.0e-5,
            max_fault_order: 0,
        };

        let clock_systems = [SidereonGnssSystem::Gps as u32];
        let empty_geometry = SidereonAraimGeometry {
            rows: ptr::null(),
            row_count: 0,
            receiver: SidereonGeodetic {
                lat_rad: 0.0,
                lon_rad: 0.0,
                height_m: 0.0,
            },
            clock_systems: clock_systems.as_ptr(),
            clock_system_count: clock_systems.len(),
        };

        unsafe {
            // Real public refusal: empty geometry rows
            let mut out_handle: *mut SidereonAraimResult = ptr::null_mut();
            assert_eq!(
                sidereon_araim(&empty_geometry, &ism, &allocation, &mut out_handle,),
                SidereonStatus::Solve
            );
            assert!(out_handle.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            assert!(info.payload_len > 0);
            let expected_len = info.payload_len;

            // Two-pass payload retrieval: Pass 1 query with null/0 buffer
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument, 0 written, full required, and retains
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

            let parsed: Value =
                serde_json::from_slice(&buf).expect("ARAIM error payload must be valid JSON");
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["family"], "araim");
            assert_eq!(parsed["operation"], "sidereon_araim");
            assert_eq!(parsed["error"]["kind"], "insufficient_geometry");
            assert_eq!(parsed["error"]["fields"], json!({}));

            // Valid control: 4-satellite tetrahedron geometry
            let rows = [
                SidereonAraimRow {
                    sat_id: c"G01".as_ptr(),
                    line_of_sight: SidereonLineOfSight {
                        e_x: INV_SQRT_3,
                        e_y: INV_SQRT_3,
                        e_z: INV_SQRT_3,
                    },
                    system: SidereonGnssSystem::Gps as u32,
                    elevation_rad: std::f64::consts::FRAC_PI_2,
                },
                SidereonAraimRow {
                    sat_id: c"G02".as_ptr(),
                    line_of_sight: SidereonLineOfSight {
                        e_x: INV_SQRT_3,
                        e_y: -INV_SQRT_3,
                        e_z: -INV_SQRT_3,
                    },
                    system: SidereonGnssSystem::Gps as u32,
                    elevation_rad: std::f64::consts::FRAC_PI_2,
                },
                SidereonAraimRow {
                    sat_id: c"G03".as_ptr(),
                    line_of_sight: SidereonLineOfSight {
                        e_x: -INV_SQRT_3,
                        e_y: INV_SQRT_3,
                        e_z: -INV_SQRT_3,
                    },
                    system: SidereonGnssSystem::Gps as u32,
                    elevation_rad: std::f64::consts::FRAC_PI_2,
                },
                SidereonAraimRow {
                    sat_id: c"G04".as_ptr(),
                    line_of_sight: SidereonLineOfSight {
                        e_x: -INV_SQRT_3,
                        e_y: -INV_SQRT_3,
                        e_z: INV_SQRT_3,
                    },
                    system: SidereonGnssSystem::Gps as u32,
                    elevation_rad: std::f64::consts::FRAC_PI_2,
                },
            ];
            let valid_geometry = SidereonAraimGeometry {
                rows: rows.as_ptr(),
                row_count: rows.len(),
                receiver: SidereonGeodetic {
                    lat_rad: 0.0,
                    lon_rad: 0.0,
                    height_m: 0.0,
                },
                clock_systems: clock_systems.as_ptr(),
                clock_system_count: clock_systems.len(),
            };

            let mut valid_handle: *mut SidereonAraimResult = ptr::null_mut();
            assert_eq!(
                sidereon_araim(&valid_geometry, &ism, &allocation, &mut valid_handle,),
                SidereonStatus::Ok
            );
            assert!(!valid_handle.is_null());

            // Success resets the engine-error slot
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Re-seed a real public refusal while valid_handle is live
            let mut failed_handle: *mut SidereonAraimResult = ptr::null_mut();
            assert_eq!(
                sidereon_araim(&empty_geometry, &ism, &allocation, &mut failed_handle),
                SidereonStatus::Solve
            );
            assert!(failed_handle.is_null());

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            let reseeded_len = info.payload_len;
            assert!(reseeded_len > 0);

            // Capture owned payload bytes
            let mut payload_before = vec![0u8; reseeded_len];
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    payload_before.as_mut_ptr(),
                    payload_before.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, reseeded_len);

            // Summary reader retains
            let mut summary = SidereonAraimSummary {
                hpl_m: 0.0,
                vpl_m: 0.0,
                sigma_acc_h_m: 0.0,
                sigma_acc_v_m: 0.0,
                emt_m: 0.0,
                p_unmonitored: 0.0,
                available: false,
                availability: false,
                fault_mode_count: 0,
            };
            assert_eq!(
                sidereon_araim_result_summary(valid_handle, &mut summary),
                SidereonStatus::Ok
            );
            assert!(summary.available);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            assert_eq!(info.payload_len, reseeded_len);

            // Fault modes reader retains
            let mut mode_written = 0;
            let mut mode_required = 0;
            assert_eq!(
                sidereon_araim_result_fault_modes(
                    valid_handle,
                    ptr::null_mut(),
                    0,
                    &mut mode_written,
                    &mut mode_required,
                ),
                SidereonStatus::Ok
            );
            assert!(mode_required > 0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            assert_eq!(info.payload_len, reseeded_len);

            // Excluded sats reader retains
            let mut sat_written = 0;
            let mut sat_required = 0;
            assert_eq!(
                sidereon_araim_result_fault_mode_excluded_sats(
                    valid_handle,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut sat_written,
                    &mut sat_required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            assert_eq!(info.payload_len, reseeded_len);

            // Verify payload is retained after getters and compare bytes
            let mut payload_after = vec![0u8; reseeded_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    payload_after.as_mut_ptr(),
                    payload_after.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, reseeded_len);
            assert_eq!(payload_before, payload_after);

            // Destructor retains
            sidereon_araim_result_free(valid_handle);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Araim);
            assert_eq!(info.payload_len, reseeded_len);

            // Early argument reset: passing null out_result clears slot
            assert_eq!(
                sidereon_araim(&valid_geometry, &ism, &allocation, ptr::null_mut(),),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
