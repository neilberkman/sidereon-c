use super::*;

/// The result of an SPP solve. Opaque to C. Create with sidereon_solve_spp or
/// sidereon_solve_spp_v2 and release with sidereon_spp_solution_free.
pub struct SidereonSppSolution {
    pub(crate) inner: ReceiverSolution,
}

/// Caller-populated inputs for a single SPP solve. Mirrors the engine solve
/// input field for field; the binding adds no defaults or modeling of its own.
#[repr(C)]
pub struct SidereonSppInputs {
    /// Pointer to observation_count observations.
    pub observations: *const SidereonObservation,
    /// Number of observations pointed to by observations.
    pub observation_count: usize,
    /// Receiver time, seconds past J2000.
    pub t_rx_j2000_s: f64,
    /// Receiver time, second of day.
    pub t_rx_second_of_day_s: f64,
    /// Day of year (1-based, fractional allowed).
    pub day_of_year: f64,
    /// Initial state guess [x_m, y_m, z_m, clock_state].
    pub initial_guess: [f64; 4],
    /// Apply the ionosphere (Klobuchar) correction.
    pub ionosphere: bool,
    /// Apply the troposphere correction.
    pub troposphere: bool,
    /// Klobuchar alpha coefficients.
    pub klobuchar_alpha: [f64; 4],
    /// Klobuchar beta coefficients.
    pub klobuchar_beta: [f64; 4],
    /// Surface pressure, hPa.
    pub pressure_hpa: f64,
    /// Surface temperature, K.
    pub temperature_k: f64,
    /// Relative humidity, 0..1.
    pub relative_humidity: f64,
    /// Also recover the geodetic (lat/lon/height) form of the position.
    pub with_geodetic: bool,
}

/// Huber/IRLS robust reweighting controls for SPP V2 inputs. Used only when
/// robust_enabled is true.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppRobustConfig {
    /// Huber tuning constant.
    pub huber_k: f64,
    /// Minimum robust scale in meters.
    pub scale_floor_m: f64,
    /// Maximum outer robust solves, including the warm start.
    pub max_outer: usize,
    /// Outer-loop position step tolerance in meters.
    pub outer_tol_m: f64,
}

/// Business-level SPP validation gates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppValidationOptions {
    /// Whether max_pdop is enforced.
    pub max_pdop_enabled: bool,
    /// Optional PDOP ceiling, used only when max_pdop_enabled is true.
    pub max_pdop: f64,
    /// Minimum plausible geocentric receiver radius in meters.
    pub min_plausible_radius_m: f64,
    /// Maximum plausible geocentric receiver radius in meters.
    pub max_plausible_radius_m: f64,
    /// Maximum residual RMS in meters for a solution flagged converged.
    pub max_converged_residual_rms_m: f64,
}

/// SPP solve policy controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppSolvePolicy {
    /// When false, the engine default validation gates are used.
    pub use_validation_options: bool,
    /// Validation gates, used only when use_validation_options is true.
    pub validation: SidereonSppValidationOptions,
    /// Whether to try near-surface coarse-search seeds after the initial guess.
    pub coarse_search_enabled: bool,
    /// Number of coarse-search seeds to generate when enabled.
    pub coarse_search_seeds: usize,
}

/// Extended SPP inputs that expose every engine control currently hidden by the
/// legacy SidereonSppInputs ABI. Initialize with sidereon_spp_inputs_v2_init,
/// then fill base with the ordinary SPP inputs and override optional controls.
#[repr(C)]
pub struct SidereonSppInputsV2 {
    /// The legacy SPP input fields.
    pub base: SidereonSppInputs,
    /// Whether BeiDou-specific Klobuchar coefficients are supplied.
    pub beidou_klobuchar_enabled: bool,
    /// BeiDou Klobuchar alpha coefficients.
    pub beidou_klobuchar_alpha: [f64; 4],
    /// BeiDou Klobuchar beta coefficients.
    pub beidou_klobuchar_beta: [f64; 4],
    /// Whether robust Huber/IRLS reweighting is enabled.
    pub robust_enabled: bool,
    /// Robust reweighting controls.
    pub robust: SidereonSppRobustConfig,
    /// Solve policy controls.
    pub policy: SidereonSppSolvePolicy,
    /// Pointer to glonass_channel_count GLONASS FDMA channel entries, keyed by
    /// slot. Required for any GLONASS observation solved with the ionosphere
    /// correction: the per-satellite G1 carrier is resolved from this map to
    /// scale the L1 Klobuchar delay. NULL with a zero count means no channels,
    /// which leaves every non-GLONASS solve bit-identical; a GLONASS observation
    /// with the ionosphere correction but no matching (or out-of-range) channel
    /// is rejected by the engine. Duplicate slots are rejected.
    pub glonass_channels: *const SidereonGlonassChannel,
    /// Number of GLONASS channel entries pointed to by glonass_channels.
    pub glonass_channel_count: usize,
}

/// Why an SPP observation was excluded from the final solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSppRejectionReason {
    /// No usable ephemeris was available at the transmit epoch.
    NoEphemeris = 0,
    /// The satellite was below the elevation mask.
    LowElevation = 1,
    /// The SBAS correction has withdrawn this satellite.
    SbasWithdrawn = 2,
    /// The SBAS ionosphere grid does not cover this line of sight.
    SbasIonoUncovered = 3,
}

/// A rejected satellite and the first reason it was excluded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppRejectedSat {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Rejection reason.
    pub reason: SidereonSppRejectionReason,
}

/// Receiver clock for one GNSS system.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppSystemClock {
    /// GNSS system.
    pub system: SidereonGnssSystem,
    /// Absolute receiver clock for this system in seconds.
    pub rx_clock_s: f64,
}

/// Per-constellation time (clock) DOP for one GNSS system: the square root of
/// that system's clock cofactor variance from the converged geometry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppSystemTdop {
    /// GNSS system.
    pub system: SidereonGnssSystem,
    /// Time DOP for this system. The reference system's value equals
    /// SidereonDop.tdop.
    pub tdop: f64,
}

/// Solver termination status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSppSolveStatus {
    /// First-order optimality tolerance was reached.
    GradientTolerance = 0,
    /// Relative cost reduction tolerance was reached.
    CostTolerance = 1,
    /// Relative step tolerance was reached.
    StepTolerance = 2,
    /// Maximum residual evaluations were reached.
    MaxEvaluations = 3,
}

/// Iteration, convergence, correction, and validation metadata for SPP.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppMetadata {
    /// Number of accepted solver iterations.
    pub iterations: usize,
    /// Whether a convergence criterion was reached.
    pub converged: bool,
    /// Solver termination status.
    pub status: SidereonSppSolveStatus,
    /// Whether ionosphere correction was applied.
    pub ionosphere_applied: bool,
    /// Whether troposphere correction was applied.
    pub troposphere_applied: bool,
    /// Number of robust outer iterations beyond the warm start.
    pub outer_iterations: usize,
    /// Whether final_robust_scale_m is present.
    pub has_final_robust_scale_m: bool,
    /// Final robust MAD scale in meters when present.
    pub final_robust_scale_m: f64,
    /// Number of satellites used in the final solve.
    pub used_count: usize,
    /// Number of GNSS systems in the final solve.
    pub system_count: usize,
    /// Degrees of freedom, used_count minus position and clock parameters.
    pub redundancy: i64,
    /// Whether residual-based RAIM can test the final solve.
    pub raim_checkable: bool,
}

/// Initialize an SPP V2 input struct with engine defaults for optional controls.
/// After this call, fill inputs->base with the ordinary SPP fields and override
/// any V2 controls needed by the solve.
///
/// Safety: out_inputs must point to a SidereonSppInputsV2.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_inputs_v2_init(
    out_inputs: *mut SidereonSppInputsV2,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_inputs_v2_init", SidereonStatus::Panic, || {
        let out_inputs = c_try!(require_out(
            out_inputs,
            "sidereon_spp_inputs_v2_init",
            "out_inputs"
        ));
        *out_inputs = default_spp_inputs_v2();
        SidereonStatus::Ok
    })
}

/// Copy the ECEF position [x_m, y_m, z_m] into out_xyz, which must hold at
/// least 3 doubles.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_position(
    sol: *const SidereonSppSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_position",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_spp_solution_position",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_position",
                "solution"
            ));
            let p = &sol.inner.position;
            c_try!(copy_exact_f64s(
                "sidereon_spp_solution_position",
                "out_xyz",
                out_xyz,
                len,
                &[p.x_m, p.y_m, p.z_m],
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the geodetic receiver position into *out_geodetic and set *out_present.
/// If the solve did not request geodetic output, *out_present is false and
/// *out_geodetic is all zeros.
///
/// Safety: sol must be a live solution handle; out_geodetic and out_present
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_geodetic(
    sol: *const SidereonSppSolution,
    out_geodetic: *mut SidereonGeodetic,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_geodetic",
        SidereonStatus::Panic,
        || {
            let out_geodetic = c_try!(require_out(
                out_geodetic,
                "sidereon_spp_solution_geodetic",
                "out_geodetic"
            ));
            *out_geodetic = empty_geodetic();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_spp_solution_geodetic",
                "out_present"
            ));
            *out_present = false;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_geodetic",
                "solution"
            ));
            if let Some(geodetic) = sol.inner.geodetic {
                *out_geodetic = SidereonGeodetic {
                    lat_rad: geodetic.lat_rad,
                    lon_rad: geodetic.lon_rad,
                    height_m: geodetic.height_m,
                };
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Write the receiver clock bias in seconds to *out_rx_clock_s.
///
/// Safety: sol must be a live solution handle; out_rx_clock_s must point to a
/// double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_rx_clock_s(
    sol: *const SidereonSppSolution,
    out_rx_clock_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_rx_clock_s",
        SidereonStatus::Panic,
        || {
            let out_rx_clock_s = c_try!(require_out(
                out_rx_clock_s,
                "sidereon_spp_solution_rx_clock_s",
                "out_rx_clock_s"
            ));
            *out_rx_clock_s = 0.0;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_rx_clock_s",
                "solution"
            ));
            *out_rx_clock_s = sol.inner.rx_clock_s;
            SidereonStatus::Ok
        },
    )
}

/// Write the number of satellites that contributed to the accepted solution to
/// *out_count.
///
/// Safety: sol must be a live solution handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_used_sat_count(
    sol: *const SidereonSppSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_used_sat_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_spp_solution_used_sat_count",
                "out_count"
            ));
            *out_count = 0;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_used_sat_count",
                "solution"
            ));
            *out_count = sol.inner.used_sats.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy used satellite tokens in solution order. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_used_sat_ids(
    sol: *const SidereonSppSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_spp_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_used_sat_ids",
                "solution"
            ));
            let values: Vec<SidereonSatelliteToken> = sol
                .inner
                .used_sats
                .iter()
                .copied()
                .map(satellite_token)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_spp_solution_used_sat_ids",
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

/// Copy rejected satellites and reasons. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_rejected_sats(
    sol: *const SidereonSppSolution,
    out: *mut SidereonSppRejectedSat,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_rejected_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_spp_solution_rejected_sats",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_rejected_sats",
                "solution"
            ));
            let values: Vec<SidereonSppRejectedSat> = sol
                .inner
                .rejected_sats
                .iter()
                .map(|rejected| SidereonSppRejectedSat {
                    sat_id: satellite_token(rejected.satellite_id),
                    reason: rejection_reason_to_c(rejected.reason),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_spp_solution_rejected_sats",
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

/// Copy per-system receiver clocks. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_system_clocks(
    sol: *const SidereonSppSolution,
    out: *mut SidereonSppSystemClock,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_system_clocks",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_spp_solution_system_clocks",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_system_clocks",
                "solution"
            ));
            let values: Vec<SidereonSppSystemClock> = sol
                .inner
                .system_clocks_s
                .iter()
                .map(|(system, rx_clock_s)| SidereonSppSystemClock {
                    system: gnss_system_to_c(*system),
                    rx_clock_s: *rx_clock_s,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_spp_solution_system_clocks",
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

/// Copy the per-constellation time (clock) DOP, one SidereonSppSystemTdop per
/// GNSS in the solve, in ascending system order (matching system_clocks). The
/// first entry's value equals SidereonDop.tdop. Empty only when the geometry is
/// rank-deficient (no DOP). Uses the variable-length output contract documented
/// at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable SidereonSppSystemTdop or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_system_tdops(
    sol: *const SidereonSppSolution,
    out: *mut SidereonSppSystemTdop,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_system_tdops",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_spp_solution_system_tdops",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_system_tdops",
                "solution"
            ));
            let values: Vec<SidereonSppSystemTdop> = sol
                .inner
                .system_tdops
                .iter()
                .map(|(system, tdop)| SidereonSppSystemTdop {
                    system: gnss_system_to_c(*system),
                    tdop: *tdop,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_spp_solution_system_tdops",
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

/// Copy the post-fit residuals (meters, in used-satellite order) into out.
/// Variable-length output contract: out_written and out_required must be valid
/// pointers. Pass out as NULL and len as 0 to query the required count without
/// copying. Otherwise out must point to at least len writable doubles. If len is
/// smaller than *out_required, the function returns
/// SIDEREON_STATUS_INVALID_ARGUMENT, copies nothing, and leaves *out_written as 0.
/// On success, *out_written is the number copied.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable doubles or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_residuals(
    sol: *const SidereonSppSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_residuals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_spp_solution_residuals",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_residuals",
                "solution"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_spp_solution_residuals",
                "out",
                &sol.inner.residuals_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the DOP (geometry-covariance) scalars into *out_dop. Fails with
/// SIDEREON_STATUS_INVALID_ARGUMENT if the converged geometry was rank-deficient
/// (the engine produced no DOP).
///
/// Safety: sol must be a live solution handle; out_dop must point to a
/// SidereonDop.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_dop(
    sol: *const SidereonSppSolution,
    out_dop: *mut SidereonDop,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_solution_dop", SidereonStatus::Panic, || {
        let out_dop = c_try!(require_out(out_dop, "sidereon_spp_solution_dop", "out_dop"));
        *out_dop = SidereonDop {
            gdop: 0.0,
            pdop: 0.0,
            hdop: 0.0,
            vdop: 0.0,
            tdop: 0.0,
        };
        let sol = c_try!(require_ref(sol, "sidereon_spp_solution_dop", "solution"));
        let Some(dop) = sol.inner.dop.as_ref() else {
            set_last_error("sidereon_spp_solution_dop: geometry is rank-deficient, no DOP");
            return SidereonStatus::InvalidArgument;
        };
        *out_dop = SidereonDop {
            gdop: dop.gdop,
            pdop: dop.pdop,
            hdop: dop.hdop,
            vdop: dop.vdop,
            tdop: dop.tdop,
        };
        SidereonStatus::Ok
    })
}

/// Copy solver metadata into *out_metadata.
///
/// Safety: sol must be a live solution handle; out_metadata must point to a
/// SidereonSppMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_metadata(
    sol: *const SidereonSppSolution,
    out_metadata: *mut SidereonSppMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_spp_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = empty_metadata();
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_metadata",
                "solution"
            ));
            let metadata = &sol.inner.metadata;
            *out_metadata = SidereonSppMetadata {
                iterations: metadata.iterations,
                converged: metadata.converged,
                status: solve_status_to_c(metadata.status),
                ionosphere_applied: metadata.ionosphere_applied,
                troposphere_applied: metadata.troposphere_applied,
                outer_iterations: metadata.outer_iterations,
                has_final_robust_scale_m: metadata.final_robust_scale_m.is_some(),
                final_robust_scale_m: metadata.final_robust_scale_m.unwrap_or(0.0),
                used_count: metadata.used_count,
                system_count: metadata.systems.len(),
                redundancy: metadata.redundancy as i64,
                raim_checkable: metadata.raim_checkable,
            };
            SidereonStatus::Ok
        },
    )
}

/// Release a solution handle. Null is a no-op. A non-null handle must come from
/// sidereon_solve_spp or sidereon_solve_spp_v2 and must be freed exactly once
/// with this function.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_spp or
/// sidereon_solve_spp_v2. Passing a handle after it has already been freed is
/// invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_free(sol: *mut SidereonSppSolution) {
    ffi_boundary("sidereon_spp_solution_free", (), || {
        free_boxed(sol);
    });
}

/// Receiver-solution plausibility-gate options, mirroring
/// sidereon_core::quality::SolutionValidationOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSolutionValidationOptions {
    /// Whether max_pdop is enforced.
    pub has_max_pdop: bool,
    /// PDOP ceiling when has_max_pdop is true.
    pub max_pdop: f64,
    /// Minimum plausible geocentric radius, meters.
    pub min_plausible_radius_m: f64,
    /// Maximum plausible geocentric radius, meters.
    pub max_plausible_radius_m: f64,
    /// Maximum plausible RMS for a converged solution, meters.
    pub max_converged_residual_rms_m: f64,
}

/// Fill *out_options with the engine default receiver-solution validation gates.
///
/// Safety: out_options must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solution_validation_options_init(
    out_options: *mut SidereonSolutionValidationOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solution_validation_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_solution_validation_options_init",
                "out_options"
            ));
            let d = SolutionValidationOptions::default();
            *out_options = SidereonSolutionValidationOptions {
                has_max_pdop: d.max_pdop.is_some(),
                max_pdop: d.max_pdop.unwrap_or(0.0),
                min_plausible_radius_m: d.min_plausible_radius_m,
                max_plausible_radius_m: d.max_plausible_radius_m,
                max_converged_residual_rms_m: d.max_converged_residual_rms_m,
            };
            SidereonStatus::Ok
        },
    )
}

/// Apply the receiver-solution plausibility gates to an SPP solution handle.
/// Returns SIDEREON_STATUS_OK when the solution passes; otherwise returns
/// SIDEREON_STATUS_INVALID_ARGUMENT and records the failing gate. Delegates to
/// sidereon_core::quality::validate_receiver_solution.
///
/// Safety: solution is a live SPP-solution handle; options points to a
/// SidereonSolutionValidationOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_validate_receiver_solution(
    solution: *const SidereonSppSolution,
    options: *const SidereonSolutionValidationOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_validate_receiver_solution",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_validate_receiver_solution",
                "solution"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_validate_receiver_solution",
                "options"
            ));
            let opts = SolutionValidationOptions {
                max_pdop: options.has_max_pdop.then_some(options.max_pdop),
                min_plausible_radius_m: options.min_plausible_radius_m,
                max_plausible_radius_m: options.max_plausible_radius_m,
                max_converged_residual_rms_m: options.max_converged_residual_rms_m,
            };
            match validate_receiver_solution(&solution.inner, opts) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => extra_invalid_arg("sidereon_validate_receiver_solution", err),
            }
        },
    )
}

// ============================================================================

// --- Batch SPP (sidereon_core::spp::solve_spp_batch_serial / _parallel) ------

/// A batch of independent SPP epoch solves over one shared ephemeris. Opaque to
/// C. Create with sidereon_solve_spp_batch_serial or
/// sidereon_solve_spp_batch_parallel; read per-epoch results with the
/// sidereon_spp_batch_* accessors; release with sidereon_spp_batch_free. Element
/// i is the solve of input i; a per-epoch solve failure is recorded for that
/// element and does not fail the batch.
pub struct SidereonSppBatch {
    pub(crate) inner: Vec<Result<ReceiverSolution, String>>,
}

/// Write the number of per-epoch results in a batch to *out_count.
///
/// Safety: batch is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_count(
    batch: *const SidereonSppBatch,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_batch_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_spp_batch_count",
            "out_count"
        ));
        *out_count = 0;
        let batch = c_try!(require_ref(batch, "sidereon_spp_batch_count", "batch"));
        *out_count = batch.inner.len();
        SidereonStatus::Ok
    })
}

/// Write whether epoch `index` solved to *out_ok.
///
/// Safety: batch is a live handle; out_ok points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_epoch_ok(
    batch: *const SidereonSppBatch,
    index: usize,
    out_ok: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_batch_epoch_ok", SidereonStatus::Panic, || {
        let out_ok = c_try!(require_out(out_ok, "sidereon_spp_batch_epoch_ok", "out_ok"));
        *out_ok = false;
        let batch = c_try!(require_ref(batch, "sidereon_spp_batch_epoch_ok", "batch"));
        let entry = match batch.inner.get(index) {
            Some(entry) => entry,
            None => {
                set_last_error(format!(
                    "sidereon_spp_batch_epoch_ok: index {index} out of range ({} results)",
                    batch.inner.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        *out_ok = entry.is_ok();
        SidereonStatus::Ok
    })
}

/// Copy epoch `index`'s solution into a newly owned SidereonSppSolution handle,
/// readable with the ordinary sidereon_spp_solution_* accessors and released with
/// sidereon_spp_solution_free. Returns SIDEREON_STATUS_SOLVE if that epoch did
/// not solve (its message is recorded for sidereon_last_error_message).
///
/// Safety: batch is a live handle; out_solution points to storage for a
/// SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_solution(
    batch: *const SidereonSppBatch,
    index: usize,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_batch_solution", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_spp_batch_solution",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let batch = c_try!(require_ref(batch, "sidereon_spp_batch_solution", "batch"));
        let entry = match batch.inner.get(index) {
            Some(entry) => entry,
            None => {
                set_last_error(format!(
                    "sidereon_spp_batch_solution: index {index} out of range ({} results)",
                    batch.inner.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        match entry {
            Ok(solution) => {
                write_boxed_handle(
                    out_solution,
                    SidereonSppSolution {
                        inner: solution.clone(),
                    },
                );
                SidereonStatus::Ok
            }
            Err(message) => {
                set_last_error(format!(
                    "sidereon_spp_batch_solution: epoch {index} did not solve: {message}"
                ));
                SidereonStatus::Solve
            }
        }
    })
}

/// Copy epoch `index`'s solve-failure message into a caller buffer (not
/// null-terminated). An epoch that solved reports *out_required 0 and writes
/// nothing. Variable-length output contract.
///
/// Safety: batch is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_error(
    batch: *const SidereonSppBatch,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_spp_batch_error", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_spp_batch_error",
            out_written,
            out_required
        ));
        let batch = c_try!(require_ref(batch, "sidereon_spp_batch_error", "batch"));
        let entry = match batch.inner.get(index) {
            Some(entry) => entry,
            None => {
                set_last_error(format!(
                    "sidereon_spp_batch_error: index {index} out of range ({} results)",
                    batch.inner.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        let message = match entry {
            Ok(_) => "",
            Err(message) => message.as_str(),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_spp_batch_error",
            "out",
            message.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release a batch SPP handle. Passing NULL is a no-op.
///
/// Safety: batch must be a handle from a sidereon_solve_spp_batch_* call or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_free(batch: *mut SidereonSppBatch) {
    free_boxed(batch);
}

fn rejection_reason_to_c(
    reason: sidereon_core::positioning::RejectionReason,
) -> SidereonSppRejectionReason {
    match reason {
        sidereon_core::positioning::RejectionReason::NoEphemeris => {
            SidereonSppRejectionReason::NoEphemeris
        }
        sidereon_core::positioning::RejectionReason::LowElevation => {
            SidereonSppRejectionReason::LowElevation
        }
        sidereon_core::positioning::RejectionReason::SbasWithdrawn => {
            SidereonSppRejectionReason::SbasWithdrawn
        }
        sidereon_core::positioning::RejectionReason::SbasIonoUncovered => {
            SidereonSppRejectionReason::SbasIonoUncovered
        }
    }
}

fn solve_status_to_c(status: Status) -> SidereonSppSolveStatus {
    match status {
        Status::GradientTolerance => SidereonSppSolveStatus::GradientTolerance,
        Status::CostTolerance => SidereonSppSolveStatus::CostTolerance,
        Status::StepTolerance => SidereonSppSolveStatus::StepTolerance,
        Status::MaxEvaluations => SidereonSppSolveStatus::MaxEvaluations,
    }
}

fn empty_metadata() -> SidereonSppMetadata {
    SidereonSppMetadata {
        iterations: 0,
        converged: false,
        status: SidereonSppSolveStatus::GradientTolerance,
        ionosphere_applied: false,
        troposphere_applied: false,
        outer_iterations: 0,
        has_final_robust_scale_m: false,
        final_robust_scale_m: 0.0,
        used_count: 0,
        system_count: 0,
        redundancy: 0,
        raim_checkable: false,
    }
}
