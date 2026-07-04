use super::*;

// --- DGNSS differential corrections (sidereon_core::dgnss) -------------------

/// One code-only pseudorange observation, mirroring
/// sidereon_core::dgnss::CodeObservation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCodeObservation {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Pseudorange in meters.
    pub pseudorange_m: f64,
}

/// A table of per-satellite DGNSS pseudorange corrections (meters). Opaque to C.
/// Create with sidereon_dgnss_pseudorange_corrections; release with
/// sidereon_dgnss_corrections_free.
pub struct SidereonDgnssCorrections {
    pub(crate) inner: BTreeMap<String, f64>,
}

/// The result of applying DGNSS corrections to rover observations. Opaque to C.
/// Create with sidereon_dgnss_apply_corrections; release with
/// sidereon_dgnss_applied_free.
pub struct SidereonDgnssApplied {
    pub(crate) corrected: Vec<sidereon_core::dgnss::CodeObservation>,
    pub(crate) dropped: Vec<String>,
}

/// A DGNSS corrected rover position solve. Opaque to C. Create with
/// sidereon_dgnss_position_solve; release with sidereon_dgnss_solution_free.
pub struct SidereonDgnssSolution {
    pub(crate) inner: sidereon_core::dgnss::PositionSolution,
}

/// Compute per-satellite DGNSS pseudorange corrections at a base station from an
/// SP3 product. On success writes a newly owned corrections handle. Delegates to
/// sidereon_core::dgnss::pseudorange_corrections (SP3 as the
/// ObservableEphemerisSource).
///
/// Safety: sp3 is a live handle; base_position_m points to 3 doubles;
/// base_observations points to base_count SidereonCodeObservation; out_corrections
/// points to a SidereonDgnssCorrections*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_pseudorange_corrections(
    sp3: *const SidereonSp3,
    base_position_m: *const f64,
    base_observations: *const SidereonCodeObservation,
    base_count: usize,
    t_rx_j2000_s: f64,
    out_corrections: *mut *mut SidereonDgnssCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_pseudorange_corrections",
        SidereonStatus::Panic,
        || {
            let out_corrections = c_try!(require_out(
                out_corrections,
                "sidereon_dgnss_pseudorange_corrections",
                "out_corrections"
            ));
            *out_corrections = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_dgnss_pseudorange_corrections",
                "sp3"
            ));
            let base_pos = c_try!(read_vec3(
                "sidereon_dgnss_pseudorange_corrections",
                "base_position_m",
                base_position_m
            ));
            let base_obs = c_try!(code_observations_from_c(
                "sidereon_dgnss_pseudorange_corrections",
                base_observations,
                base_count
            ));
            match sidereon_core::dgnss::pseudorange_corrections(
                &sp3.inner,
                base_pos,
                &base_obs,
                t_rx_j2000_s,
            ) {
                Ok(map) => {
                    write_boxed_handle(out_corrections, SidereonDgnssCorrections { inner: map });
                    SidereonStatus::Ok
                }
                Err(err) => map_dgnss_error("sidereon_dgnss_pseudorange_corrections", err),
            }
        },
    )
}

/// Write the number of correction entries to *out_count.
///
/// Safety: corrections is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_corrections_count(
    corrections: *const SidereonDgnssCorrections,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_corrections_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_dgnss_corrections_count",
                "out_count"
            ));
            *out_count = 0;
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_dgnss_corrections_count",
                "corrections"
            ));
            *out_count = corrections.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Read the correction (meters) for one satellite token. Sets *out_present to
/// whether the table has an entry for it.
///
/// Safety: corrections is a live handle; satellite_id is a null-terminated token;
/// out_value points to a double; out_present points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_correction(
    corrections: *const SidereonDgnssCorrections,
    satellite_id: *const c_char,
    out_value: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_dgnss_correction", SidereonStatus::Panic, || {
        let out_value = c_try!(require_out(
            out_value,
            "sidereon_dgnss_correction",
            "out_value"
        ));
        *out_value = 0.0;
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_dgnss_correction",
            "out_present"
        ));
        *out_present = false;
        let corrections = c_try!(require_ref(
            corrections,
            "sidereon_dgnss_correction",
            "corrections"
        ));
        let sat = c_try!(parse_satellite_token(
            "sidereon_dgnss_correction",
            satellite_id
        ));
        if let Some(value) = corrections.inner.get(&sat.to_string()) {
            *out_value = *value;
            *out_present = true;
        }
        SidereonStatus::Ok
    })
}

/// Release a DGNSS corrections handle. Passing NULL is a no-op.
///
/// Safety: corrections must be a handle from
/// sidereon_dgnss_pseudorange_corrections or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_corrections_free(
    corrections: *mut SidereonDgnssCorrections,
) {
    free_boxed(corrections);
}

/// Apply DGNSS corrections to rover observations, producing the corrected set and
/// the list of satellites dropped for lack of a correction. Delegates to
/// sidereon_core::dgnss::apply_corrections.
///
/// Safety: rover_observations points to rover_count SidereonCodeObservation;
/// corrections is a live handle; out_applied points to a SidereonDgnssApplied*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_apply_corrections(
    rover_observations: *const SidereonCodeObservation,
    rover_count: usize,
    corrections: *const SidereonDgnssCorrections,
    out_applied: *mut *mut SidereonDgnssApplied,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_apply_corrections",
        SidereonStatus::Panic,
        || {
            let out_applied = c_try!(require_out(
                out_applied,
                "sidereon_dgnss_apply_corrections",
                "out_applied"
            ));
            *out_applied = ptr::null_mut();
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_dgnss_apply_corrections",
                "corrections"
            ));
            let rover_obs = c_try!(code_observations_from_c(
                "sidereon_dgnss_apply_corrections",
                rover_observations,
                rover_count
            ));
            match sidereon_core::dgnss::apply_corrections(&rover_obs, &corrections.inner) {
                Ok(applied) => {
                    write_boxed_handle(
                        out_applied,
                        SidereonDgnssApplied {
                            corrected: applied.corrected,
                            dropped: applied.dropped,
                        },
                    );
                    SidereonStatus::Ok
                }
                Err(err) => map_dgnss_error("sidereon_dgnss_apply_corrections", err),
            }
        },
    )
}

/// Compute DGNSS corrections, apply them to rover observations, and solve the
/// corrected rover position. Delegates to sidereon_core::dgnss::solve_position.
/// The SPP V2 input supplies receive-time scalars, initial guess, robust
/// settings, GLONASS channels, and the geodetic flag; its observation and
/// atmospheric-correction fields are replaced by the core DGNSS driver.
///
/// Safety: sp3 is a live handle; base_position_m points to 3 doubles;
/// base_observations points to base_count SidereonCodeObservation;
/// rover_observations points to rover_count SidereonCodeObservation; inputs
/// points to a SidereonSppInputsV2; out_solution points to storage for a
/// SidereonDgnssSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_position_solve(
    sp3: *const SidereonSp3,
    base_position_m: *const f64,
    base_observations: *const SidereonCodeObservation,
    base_count: usize,
    rover_observations: *const SidereonCodeObservation,
    rover_count: usize,
    inputs: *const SidereonSppInputsV2,
    out_solution: *mut *mut SidereonDgnssSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_position_solve",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_dgnss_position_solve",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(sp3, "sidereon_dgnss_position_solve", "sp3"));
            let base_pos = c_try!(read_vec3(
                "sidereon_dgnss_position_solve",
                "base_position_m",
                base_position_m
            ));
            let base_obs = c_try!(code_observations_from_c(
                "sidereon_dgnss_position_solve",
                base_observations,
                base_count
            ));
            let rover_obs = c_try!(code_observations_from_c(
                "sidereon_dgnss_position_solve",
                rover_observations,
                rover_count
            ));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_dgnss_position_solve",
                "inputs"
            ));
            let glonass_channels = c_try!(glonass_channels_from_c(
                "sidereon_dgnss_position_solve",
                inputs
            ));
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_dgnss_position_solve",
                &inputs.base,
                beidou_klobuchar_from_c(inputs),
                robust_config_from_c(inputs),
                glonass_channels,
            ));
            match sidereon_core::dgnss::solve_position(
                &sp3.inner,
                base_pos,
                &base_obs,
                &rover_obs,
                solve_inputs,
                inputs.base.with_geodetic,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out_solution, SidereonDgnssSolution { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_dgnss_error("sidereon_dgnss_position_solve", err),
            }
        },
    )
}

/// Write the corrected-observation and dropped-satellite counts. Either out
/// pointer may be NULL.
///
/// Safety: applied is a live handle; non-null out pointers point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_applied_counts(
    applied: *const SidereonDgnssApplied,
    out_corrected_count: *mut usize,
    out_dropped_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_applied_counts",
        SidereonStatus::Panic,
        || {
            let applied = c_try!(require_ref(
                applied,
                "sidereon_dgnss_applied_counts",
                "applied"
            ));
            if let Some(p) = out_corrected_count.as_mut() {
                *p = applied.corrected.len();
            }
            if let Some(p) = out_dropped_count.as_mut() {
                *p = applied.dropped.len();
            }
            SidereonStatus::Ok
        },
    )
}

/// Read one corrected observation: its satellite token (null-terminated) into
/// out_sat_id and its corrected pseudorange (meters) into out_pseudorange_m.
///
/// Safety: applied is a live handle; out_sat_id points to sat_id_len writable
/// bytes; out_pseudorange_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_applied_corrected(
    applied: *const SidereonDgnssApplied,
    index: usize,
    out_sat_id: *mut c_char,
    sat_id_len: usize,
    out_pseudorange_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_applied_corrected",
        SidereonStatus::Panic,
        || {
            let out_pseudorange_m = c_try!(require_out(
                out_pseudorange_m,
                "sidereon_dgnss_applied_corrected",
                "out_pseudorange_m"
            ));
            *out_pseudorange_m = 0.0;
            let applied = c_try!(require_ref(
                applied,
                "sidereon_dgnss_applied_corrected",
                "applied"
            ));
            let obs = match applied.corrected.get(index) {
                Some(o) => o,
                None => {
                    set_last_error(
                        "sidereon_dgnss_applied_corrected: index out of range".to_string(),
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(write_c_token(
                "sidereon_dgnss_applied_corrected",
                out_sat_id,
                sat_id_len,
                &obs.satellite_id
            ));
            *out_pseudorange_m = obs.pseudorange_m;
            SidereonStatus::Ok
        },
    )
}

/// Read one dropped satellite token (null-terminated) into out_sat_id.
///
/// Safety: applied is a live handle; out_sat_id points to sat_id_len bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_applied_dropped(
    applied: *const SidereonDgnssApplied,
    index: usize,
    out_sat_id: *mut c_char,
    sat_id_len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_applied_dropped",
        SidereonStatus::Panic,
        || {
            let applied = c_try!(require_ref(
                applied,
                "sidereon_dgnss_applied_dropped",
                "applied"
            ));
            let token = match applied.dropped.get(index) {
                Some(t) => t,
                None => {
                    set_last_error(
                        "sidereon_dgnss_applied_dropped: index out of range".to_string(),
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(write_c_token(
                "sidereon_dgnss_applied_dropped",
                out_sat_id,
                sat_id_len,
                token
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a DGNSS applied-corrections handle. Passing NULL is a no-op.
///
/// Safety: applied must be a handle from sidereon_dgnss_apply_corrections or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_applied_free(applied: *mut SidereonDgnssApplied) {
    free_boxed(applied);
}

/// Copy the embedded corrected-rover SPP solution into a newly owned SPP
/// solution handle. Release it with sidereon_spp_solution_free.
///
/// Safety: solution must be a live handle from sidereon_dgnss_position_solve;
/// out_spp must point to storage for a SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_solution_solution(
    solution: *const SidereonDgnssSolution,
    out_spp: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_solution_solution",
        SidereonStatus::Panic,
        || {
            let out_spp = c_try!(require_out(
                out_spp,
                "sidereon_dgnss_solution_solution",
                "out_spp"
            ));
            *out_spp = ptr::null_mut();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_dgnss_solution_solution",
                "solution"
            ));
            write_boxed_handle(
                out_spp,
                SidereonSppSolution {
                    inner: solution.inner.solution.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Copy the rover-minus-base ECEF baseline vector and baseline length.
///
/// Safety: solution must be a live handle from sidereon_dgnss_position_solve;
/// out_vector_m must point to len writable doubles; out_baseline_m must point to
/// a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_solution_baseline(
    solution: *const SidereonDgnssSolution,
    out_vector_m: *mut f64,
    len: usize,
    out_baseline_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_solution_baseline",
        SidereonStatus::Panic,
        || {
            let out_baseline_m = c_try!(require_out(
                out_baseline_m,
                "sidereon_dgnss_solution_baseline",
                "out_baseline_m"
            ));
            *out_baseline_m = 0.0;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_dgnss_solution_baseline",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_dgnss_solution_baseline",
                "out_vector_m",
                out_vector_m,
                len,
                &solution.inner.baseline_vector_m,
            ));
            *out_baseline_m = solution.inner.baseline_m;
            SidereonStatus::Ok
        },
    )
}

/// Copy rover satellites dropped for lack of matching base corrections. Uses
/// the variable-length output contract documented at the top of the header.
///
/// Safety: solution must be a live handle from sidereon_dgnss_position_solve;
/// out must point to at least len writable SidereonSatelliteToken entries or be
/// NULL when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_solution_dropped_sats(
    solution: *const SidereonDgnssSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dgnss_solution_dropped_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_dgnss_solution_dropped_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_dgnss_solution_dropped_sats",
                "solution"
            ));
            let values: Vec<SidereonSatelliteToken> = solution
                .inner
                .dropped_sats
                .iter()
                .map(|sat| satellite_token_from_text(sat))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_dgnss_solution_dropped_sats",
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

/// Release a DGNSS position solution handle. Passing NULL is a no-op.
///
/// Safety: solution must be a handle from sidereon_dgnss_position_solve or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_solution_free(solution: *mut SidereonDgnssSolution) {
    free_boxed(solution);
}

unsafe fn code_observations_from_c(
    fn_name: &str,
    obs: *const SidereonCodeObservation,
    count: usize,
) -> Result<Vec<sidereon_core::dgnss::CodeObservation>, SidereonStatus> {
    let rows = require_slice(obs, count, fn_name, "observations")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        out.push(sidereon_core::dgnss::CodeObservation {
            satellite_id: sat.to_string(),
            pseudorange_m: row.pseudorange_m,
        });
    }
    Ok(out)
}

fn map_dgnss_error(fn_name: &str, err: sidereon_core::dgnss::DgnssError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::dgnss::DgnssError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        sidereon_core::dgnss::DgnssError::Spp(_) => SidereonStatus::Solve,
    }
}
