use super::*;
use crate::engine_error::{
    dgnss_error_value, engine_error_operation_boundary, record_engine_error,
    SidereonEngineErrorFamily,
};

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
    engine_error_operation_boundary(
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
/// out_value points to a double; out_present points to a bool. The output
/// ranges must not overlap.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dgnss_correction(
    corrections: *const SidereonDgnssCorrections,
    satellite_id: *const c_char,
    out_value: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_dgnss_correction", SidereonStatus::Panic, || {
        if !out_value.is_null() && !out_present.is_null() {
            let value_range = c_try!(super::checked_output_range(
                "sidereon_dgnss_correction",
                out_value,
                1,
                "out_value"
            ));
            let present_range = c_try!(super::checked_output_range(
                "sidereon_dgnss_correction",
                out_present,
                1,
                "out_present"
            ));
            c_try!(super::reject_overlapping_outputs(
                "sidereon_dgnss_correction",
                value_range,
                present_range,
                "out_value",
                "out_present"
            ));
        }
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
    engine_error_operation_boundary(
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
    engine_error_operation_boundary(
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
            if !out_corrected_count.is_null() {
                out_corrected_count.write(applied.corrected.len());
            }
            if !out_dropped_count.is_null() {
                out_dropped_count.write(applied.dropped.len());
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
    record_engine_error(
        SidereonEngineErrorFamily::Dgnss,
        fn_name,
        dgnss_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::dgnss::DgnssError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        sidereon_core::dgnss::DgnssError::Spp(_) => SidereonStatus::Solve,
        sidereon_core::dgnss::DgnssError::Ut1OutsideCoverage(_) => {
            SidereonStatus::Ut1OutsideCoverage
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorInfo,
    };
    use sidereon_core::astro::math::least_squares::SolveError;
    use sidereon_core::astro::time::DegradeReason;
    use sidereon_core::dgnss::DgnssError;
    use sidereon_core::positioning::SppError;

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
    fn test_dgnss_map_error_all_variants() {
        clear_engine_error();

        let cases = [
            (
                DgnssError::InvalidInput {
                    field: "base_position_m",
                    reason: "non-finite",
                },
                SidereonStatus::InvalidArgument,
                "invalid_input",
            ),
            (
                DgnssError::Spp(SppError::Singular(SolveError::SingularJacobian)),
                SidereonStatus::Solve,
                "spp",
            ),
            (
                DgnssError::Ut1OutsideCoverage(DegradeReason::BeforeCoverage),
                SidereonStatus::Ut1OutsideCoverage,
                "ut1_outside_coverage",
            ),
        ];

        for (err, expected_status, expected_kind) in cases {
            clear_engine_error();
            let status = map_dgnss_error("test_dgnss_op", err);
            assert_eq!(status, expected_status);

            let (info, payload) = get_last_engine_error_two_pass();
            assert_eq!(info.family, SidereonEngineErrorFamily::Dgnss);
            let parsed: serde_json::Value =
                serde_json::from_str(&payload).expect("valid json payload");
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["family"], "dgnss");
            assert_eq!(parsed["operation"], "test_dgnss_op");
            assert_eq!(parsed["error"]["kind"], expected_kind);
        }

        clear_engine_error();
    }

    #[test]
    fn test_dgnss_public_refusal_valid_control_undersize_and_retention() {
        clear_engine_error();

        let sat = CString::new("G01").expect("valid CString");
        let obs_valid = [SidereonCodeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: 20_000_000.0,
        }];
        let obs_invalid = [SidereonCodeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: -100.0,
        }];
        let corrections = SidereonDgnssCorrections {
            inner: BTreeMap::new(),
        };

        // 1. Create one valid owned result first
        let mut live_applied: *mut SidereonDgnssApplied = ptr::null_mut();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(obs_valid.as_ptr(), 1, &corrections, &mut live_applied)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!live_applied.is_null());

        // 2. Real public refusal into separate refusal pointer: negative pseudorange
        let mut out_applied_refusal: *mut SidereonDgnssApplied = ptr::null_mut();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(
                obs_invalid.as_ptr(),
                1,
                &corrections,
                &mut out_applied_refusal,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(out_applied_refusal.is_null());

        let msg = get_last_error_string();
        assert!(msg.contains("rover_observation.pseudorange_m"));

        let (info, payload_str) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::Dgnss);
        let parsed: serde_json::Value =
            serde_json::from_str(&payload_str).expect("valid json payload");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "dgnss");
        assert_eq!(parsed["operation"], "sidereon_dgnss_apply_corrections");
        assert_eq!(parsed["error"]["kind"], "invalid_input");
        assert_eq!(
            parsed["error"]["fields"]["field"],
            "rover_observation.pseudorange_m"
        );
        assert_eq!(parsed["error"]["fields"]["reason"], "not positive");
        assert!(parsed["error"]["fields"].get("kind").is_none());

        // 3. Undersize buffer contract: InvalidArgument, 0 written, full required, no copy
        let expected_len = info.payload_len;
        let mut written = 999;
        let mut required = 0;
        let status = unsafe {
            sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, 0);
        assert_eq!(required, expected_len);

        let mut short_buf = vec![0u8; expected_len - 1];
        let status = unsafe {
            sidereon_last_engine_error_payload(
                short_buf.as_mut_ptr(),
                short_buf.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(written, 0);
        assert_eq!(required, expected_len);
        assert!(short_buf.iter().all(|&b| b == 0));

        let mut full_buf = vec![0u8; expected_len];
        let status = unsafe {
            sidereon_last_engine_error_payload(
                full_buf.as_mut_ptr(),
                full_buf.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, expected_len);
        assert_eq!(required, expected_len);
        assert_eq!(full_buf, payload_str.as_bytes());

        // 4. Error retained through live getters and live free of valid applied handle
        let mut corr_count = 999usize;
        let mut drop_count = 999usize;
        let status = unsafe {
            sidereon_dgnss_applied_counts(live_applied, &mut corr_count, &mut drop_count)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(corr_count, 0);
        assert_eq!(drop_count, 1);

        let (info_retained, payload_retained) = get_last_engine_error_two_pass();
        assert_eq!(info_retained.family, SidereonEngineErrorFamily::Dgnss);
        assert_eq!(info_retained.payload_len, expected_len);
        assert_eq!(payload_retained, payload_str);

        let mut count = 999usize;
        let status = unsafe { sidereon_dgnss_corrections_count(&corrections, &mut count) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(count, 0);

        let (info_retained_corr, payload_retained_corr) = get_last_engine_error_two_pass();
        assert_eq!(info_retained_corr.family, SidereonEngineErrorFamily::Dgnss);
        assert_eq!(payload_retained_corr, payload_str);

        unsafe { sidereon_dgnss_applied_free(live_applied) };

        let (info_retained2, payload_retained2) = get_last_engine_error_two_pass();
        assert_eq!(info_retained2.family, SidereonEngineErrorFamily::Dgnss);
        assert_eq!(payload_retained2, payload_str);

        clear_engine_error();
    }

    #[test]
    fn test_dgnss_real_refusal_clearing_and_early_arg_reset() {
        clear_engine_error();

        let sat = CString::new("G01").expect("valid CString");
        let obs_valid = [SidereonCodeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: 20_000_000.0,
        }];
        let obs_invalid = [SidereonCodeObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: -100.0,
        }];
        let corrections = SidereonDgnssCorrections {
            inner: BTreeMap::new(),
        };

        let trigger_real_refusal = || {
            let mut out = ptr::null_mut();
            let status = unsafe {
                sidereon_dgnss_apply_corrections(obs_invalid.as_ptr(), 1, &corrections, &mut out)
            };
            assert_eq!(status, SidereonStatus::InvalidArgument);
            assert!(out.is_null());
            let (info, _) = get_last_engine_error_two_pass();
            assert_eq!(info.family, SidereonEngineErrorFamily::Dgnss);
            assert!(info.payload_len > 0);
        };

        // 1. Early argument check: null observations with count > 0
        trigger_real_refusal();
        let mut out_applied: *mut SidereonDgnssApplied = ptr::null_mut();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(ptr::null(), 1, &corrections, &mut out_applied)
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 2. Early argument check: null corrections
        trigger_real_refusal();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(obs_valid.as_ptr(), 1, ptr::null(), &mut out_applied)
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 3. Early argument check: null out_applied
        trigger_real_refusal();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(obs_valid.as_ptr(), 1, &corrections, ptr::null_mut())
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 4. Producer boundaries table for all 3 DGNSS producing functions:
        // Producer 1: sidereon_dgnss_pseudorange_corrections
        trigger_real_refusal();
        let status = unsafe {
            sidereon_dgnss_pseudorange_corrections(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                0.0,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // Producer 2: sidereon_dgnss_apply_corrections
        trigger_real_refusal();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(ptr::null(), 0, ptr::null(), ptr::null_mut())
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // Producer 3: sidereon_dgnss_position_solve
        trigger_real_refusal();
        let status = unsafe {
            sidereon_dgnss_position_solve(
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        // 5. Success reset after real producing refusal
        trigger_real_refusal();
        let mut out_applied_succ: *mut SidereonDgnssApplied = ptr::null_mut();
        let status = unsafe {
            sidereon_dgnss_apply_corrections(
                obs_valid.as_ptr(),
                1,
                &corrections,
                &mut out_applied_succ,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!out_applied_succ.is_null());
        let (info, payload) = get_last_engine_error_two_pass();
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        assert!(payload.is_empty());

        unsafe { sidereon_dgnss_applied_free(out_applied_succ) };
        clear_engine_error();
    }
}
