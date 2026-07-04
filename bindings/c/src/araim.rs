use super::*;

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
    /// True when the solve met the allocation and all roots converged.
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
    ffi_boundary("sidereon_araim", SidereonStatus::Panic, || {
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
    Ok(AraimGeometry {
        rows: parsed_rows,
        receiver,
        clock_systems,
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

fn map_araim_error(fn_name: &str, err: AraimError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        AraimError::InvalidIsm | AraimError::InvalidAllocation => SidereonStatus::InvalidArgument,
        AraimError::InsufficientGeometry
        | AraimError::UnmonitorableFaultMass
        | AraimError::NumericalFailure => SidereonStatus::Solve,
    }
}

fn araim_summary_to_c(value: &CoreAraimResult) -> SidereonAraimSummary {
    SidereonAraimSummary {
        hpl_m: value.hpl_m,
        vpl_m: value.vpl_m,
        sigma_acc_h_m: value.sigma_acc_h_m,
        sigma_acc_v_m: value.sigma_acc_v_m,
        emt_m: value.emt_m,
        p_unmonitored: value.p_unmonitored,
        availability: value.availability,
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
