use super::*;

/// The result of an SPP solve. Opaque to C. Create with sidereon_solve_spp or
/// sidereon_solve_spp_v2 and release with sidereon_spp_solution_free.
pub struct SidereonSppSolution {
    pub(crate) inner: ReceiverSolution,
}

/// A combined SPP position plus optional Doppler velocity result. Opaque to C.
/// Create with sidereon_solve_spp_with_doppler_velocity or
/// sidereon_solve_broadcast_with_doppler_velocity and release with
/// sidereon_spp_doppler_solution_free.
pub struct SidereonSppDopplerSolution {
    pub(crate) receiver: ReceiverSolution,
    pub(crate) velocity: Option<VelocitySolution>,
    pub(crate) velocity_error: Option<VelocityError>,
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

/// One Doppler row for an SPP-family receiver velocity solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppDopplerObservation {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Doppler shift in hertz.
    pub doppler_hz: f64,
    /// Carrier frequency in hertz.
    pub carrier_hz: f64,
    /// Satellite clock drift in seconds per second.
    pub sat_clock_drift_s_s: f64,
}

/// SPP Doppler velocity solve error category.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSppDopplerVelocityErrorKind {
    /// No Doppler velocity error occurred.
    None = 0,
    /// No Doppler rows were supplied.
    NoObservations = 1,
    /// Fewer than four usable satellites remained.
    TooFewSatellites = 2,
    /// The velocity normal matrix was singular.
    SingularGeometry = 3,
    /// A satellite appeared more than once.
    DuplicateObservation = 4,
    /// Doppler conversion needed a positive finite carrier frequency.
    InvalidCarrier = 5,
    /// A scalar input was malformed.
    InvalidInput = 6,
    /// A Doppler row carried a non-finite value.
    InvalidObservation = 7,
    /// The receiver state or receive epoch was non-finite.
    InvalidReceiverState = 8,
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
    /// Geometry observability and covariance-validation diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
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

/// Write the optional receiver clock drift in seconds per second and set
/// *out_present. Pseudorange-only solves set *out_present false and drift 0.
///
/// Safety: sol must be a live solution handle; out_present and out_drift_s_s
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_rx_clock_drift_s_s(
    sol: *const SidereonSppSolution,
    out_present: *mut bool,
    out_drift_s_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_rx_clock_drift_s_s",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_spp_solution_rx_clock_drift_s_s",
                "out_present"
            ));
            *out_present = false;
            let out_drift_s_s = c_try!(require_out(
                out_drift_s_s,
                "sidereon_spp_solution_rx_clock_drift_s_s",
                "out_drift_s_s"
            ));
            *out_drift_s_s = 0.0;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_rx_clock_drift_s_s",
                "solution"
            ));
            if let Some(drift) = sol.inner.rx_clock_drift_s_s {
                *out_present = true;
                *out_drift_s_s = drift;
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy the SPP 3x3 ECEF position covariance in row-major order.
///
/// Safety: sol must be a live solution handle; out_m2 must point to len writable
/// doubles and len must be at least 9.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_position_covariance_ecef_m2(
    sol: *const SidereonSppSolution,
    out_m2: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_position_covariance_ecef_m2",
        SidereonStatus::Panic,
        || {
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_position_covariance_ecef_m2",
                "solution"
            ));
            let values = flatten_spp_mat3(sol.inner.position_covariance.ecef_m2);
            c_try!(copy_exact_f64s(
                "sidereon_spp_solution_position_covariance_ecef_m2",
                "out_m2",
                out_m2,
                len,
                &values,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the SPP 3x3 ENU position covariance in row-major order.
///
/// Safety: sol must be a live solution handle; out_m2 must point to len writable
/// doubles and len must be at least 9.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_position_covariance_enu_m2(
    sol: *const SidereonSppSolution,
    out_m2: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_solution_position_covariance_enu_m2",
        SidereonStatus::Panic,
        || {
            let sol = c_try!(require_ref(
                sol,
                "sidereon_spp_solution_position_covariance_enu_m2",
                "solution"
            ));
            let values = flatten_spp_mat3(sol.inner.position_covariance.enu_m2);
            c_try!(copy_exact_f64s(
                "sidereon_spp_solution_position_covariance_enu_m2",
                "out_m2",
                out_m2,
                len,
                &values,
            ));
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
                geometry_quality: geometry_quality_to_c(&sol.inner.geometry_quality),
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

/// Solve SPP position from an SP3 source and attach a Doppler velocity solution
/// when the Doppler rows are usable.
///
/// Safety: sp3 must be a live handle; inputs must point to a valid SPP V2 input;
/// doppler_observations points to doppler_count rows or is NULL when count is 0;
/// out_solution must point to storage for a SidereonSppDopplerSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_with_doppler_velocity(
    sp3: *const SidereonSp3,
    inputs: *const SidereonSppInputsV2,
    doppler_observations: *const SidereonSppDopplerObservation,
    doppler_count: usize,
    out_solution: *mut *mut SidereonSppDopplerSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_spp_with_doppler_velocity",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_spp_with_doppler_velocity",
                "sp3"
            ));
            solve_spp_with_doppler_common(
                "sidereon_solve_spp_with_doppler_velocity",
                &sp3.inner,
                inputs,
                doppler_observations,
                doppler_count,
                out_solution,
            )
        },
    )
}

/// Solve SPP position from broadcast ephemeris and attach a Doppler velocity
/// solution when the Doppler rows are usable.
///
/// Safety: broadcast must be a live handle; inputs must point to a valid SPP V2
/// input; doppler_observations points to doppler_count rows or is NULL when
/// count is 0; out_solution must point to storage for a SidereonSppDopplerSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_broadcast_with_doppler_velocity(
    broadcast: *const SidereonBroadcastEphemeris,
    inputs: *const SidereonSppInputsV2,
    doppler_observations: *const SidereonSppDopplerObservation,
    doppler_count: usize,
    out_solution: *mut *mut SidereonSppDopplerSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_broadcast_with_doppler_velocity",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_solve_broadcast_with_doppler_velocity",
                "broadcast"
            ));
            solve_spp_with_doppler_common(
                "sidereon_solve_broadcast_with_doppler_velocity",
                &broadcast.inner,
                inputs,
                doppler_observations,
                doppler_count,
                out_solution,
            )
        },
    )
}

/// Copy the receiver solution from a combined SPP Doppler result into a newly
/// owned SPP solution handle.
///
/// Safety: solution must be a live combined handle; out_receiver must point to
/// storage for a SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_receiver(
    solution: *const SidereonSppDopplerSolution,
    out_receiver: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_doppler_solution_receiver",
        SidereonStatus::Panic,
        || {
            let out_receiver = c_try!(require_out(
                out_receiver,
                "sidereon_spp_doppler_solution_receiver",
                "out_receiver"
            ));
            *out_receiver = ptr::null_mut();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_spp_doppler_solution_receiver",
                "solution"
            ));
            write_boxed_handle(
                out_receiver,
                SidereonSppSolution {
                    inner: solution.receiver.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write whether a combined result carries a Doppler velocity solution.
///
/// Safety: solution must be a live combined handle; out_has_velocity must point
/// to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_has_velocity(
    solution: *const SidereonSppDopplerSolution,
    out_has_velocity: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_doppler_solution_has_velocity",
        SidereonStatus::Panic,
        || {
            let out_has_velocity = c_try!(require_out(
                out_has_velocity,
                "sidereon_spp_doppler_solution_has_velocity",
                "out_has_velocity"
            ));
            *out_has_velocity = false;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_spp_doppler_solution_has_velocity",
                "solution"
            ));
            *out_has_velocity = solution.velocity.is_some();
            SidereonStatus::Ok
        },
    )
}

/// Copy the Doppler velocity solution into a newly owned velocity handle.
/// Returns SIDEREON_STATUS_SOLVE when no velocity solution is present.
///
/// Safety: solution must be a live combined handle; out_velocity must point to
/// storage for a SidereonVelocitySolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_velocity(
    solution: *const SidereonSppDopplerSolution,
    out_velocity: *mut *mut SidereonVelocitySolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_doppler_solution_velocity",
        SidereonStatus::Panic,
        || {
            let out_velocity = c_try!(require_out(
                out_velocity,
                "sidereon_spp_doppler_solution_velocity",
                "out_velocity"
            ));
            *out_velocity = ptr::null_mut();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_spp_doppler_solution_velocity",
                "solution"
            ));
            let Some(velocity) = &solution.velocity else {
                set_last_error("sidereon_spp_doppler_solution_velocity: no velocity solution");
                return SidereonStatus::Solve;
            };
            write_boxed_handle(
                out_velocity,
                SidereonVelocitySolution {
                    inner: velocity.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write the retained Doppler velocity error kind. The kind is None when the
/// combined result has a velocity solution or no Doppler rows were supplied.
///
/// Safety: solution must be a live combined handle; out_error must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_velocity_error_kind(
    solution: *const SidereonSppDopplerSolution,
    out_error: *mut SidereonSppDopplerVelocityErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_doppler_solution_velocity_error_kind",
        SidereonStatus::Panic,
        || {
            let out_error = c_try!(require_out(
                out_error,
                "sidereon_spp_doppler_solution_velocity_error_kind",
                "out_error"
            ));
            *out_error = SidereonSppDopplerVelocityErrorKind::None;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_spp_doppler_solution_velocity_error_kind",
                "solution"
            ));
            *out_error = solution
                .velocity_error
                .as_ref()
                .map(spp_doppler_velocity_error_to_c)
                .unwrap_or(SidereonSppDopplerVelocityErrorKind::None);
            SidereonStatus::Ok
        },
    )
}

/// Release a combined SPP Doppler solution handle. Passing NULL is a no-op.
///
/// Safety: solution must be NULL or a live handle from a combined SPP Doppler
/// solve that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_free(
    solution: *mut SidereonSppDopplerSolution,
) {
    ffi_boundary("sidereon_spp_doppler_solution_free", (), || {
        free_boxed(solution);
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

#[allow(clippy::too_many_arguments)]
unsafe fn solve_spp_with_doppler_common<E>(
    fn_name: &str,
    source: &E,
    inputs: *const SidereonSppInputsV2,
    doppler_observations: *const SidereonSppDopplerObservation,
    doppler_count: usize,
    out_solution: *mut *mut SidereonSppDopplerSolution,
) -> SidereonStatus
where
    E: EphemerisSource + ObservableEphemerisSource,
{
    let out_solution = c_try!(require_out(out_solution, fn_name, "out_solution"));
    *out_solution = ptr::null_mut();
    let inputs = c_try!(require_ref(inputs, fn_name, "inputs"));
    let glonass_channels = c_try!(glonass_channels_from_c(fn_name, inputs));
    let solve_inputs = c_try!(build_spp_solve_inputs(
        fn_name,
        &inputs.base,
        beidou_klobuchar_from_c(inputs),
        robust_config_from_c(inputs),
        glonass_channels,
    ));
    let doppler = c_try!(spp_doppler_observations_from_c(
        fn_name,
        doppler_observations,
        doppler_count
    ));
    match core_solve_with_doppler_velocity(
        source,
        &solve_inputs,
        &doppler,
        inputs.base.with_geodetic,
    ) {
        Ok(solution) => {
            write_boxed_handle(
                out_solution,
                SidereonSppDopplerSolution {
                    receiver: solution.receiver,
                    velocity: solution.velocity,
                    velocity_error: solution.velocity_error,
                },
            );
            SidereonStatus::Ok
        }
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::Solve
        }
    }
}

unsafe fn spp_doppler_observations_from_c(
    fn_name: &str,
    observations: *const SidereonSppDopplerObservation,
    count: usize,
) -> Result<Vec<CoreDopplerObservation>, SidereonStatus> {
    let raw = require_slice(observations, count, fn_name, "doppler_observations")?;
    let mut parsed = Vec::with_capacity(raw.len());
    for obs in raw {
        parsed.push(CoreDopplerObservation {
            satellite_id: parse_satellite_token(fn_name, obs.sat_id)?,
            doppler_hz: obs.doppler_hz,
            carrier_hz: obs.carrier_hz,
            sat_clock_drift_s_s: obs.sat_clock_drift_s_s,
        });
    }
    Ok(parsed)
}

fn spp_doppler_velocity_error_to_c(error: &VelocityError) -> SidereonSppDopplerVelocityErrorKind {
    match error {
        VelocityError::NoObservations => SidereonSppDopplerVelocityErrorKind::NoObservations,
        VelocityError::TooFewSatellites { .. } => {
            SidereonSppDopplerVelocityErrorKind::TooFewSatellites
        }
        VelocityError::SingularGeometry => SidereonSppDopplerVelocityErrorKind::SingularGeometry,
        VelocityError::DuplicateObservation { .. } => {
            SidereonSppDopplerVelocityErrorKind::DuplicateObservation
        }
        VelocityError::InvalidCarrier { .. } => SidereonSppDopplerVelocityErrorKind::InvalidCarrier,
        VelocityError::InvalidInput { .. } => SidereonSppDopplerVelocityErrorKind::InvalidInput,
        VelocityError::InvalidObservation { .. } => {
            SidereonSppDopplerVelocityErrorKind::InvalidObservation
        }
        VelocityError::InvalidReceiverState => {
            SidereonSppDopplerVelocityErrorKind::InvalidReceiverState
        }
    }
}

fn flatten_spp_mat3(matrix: [[f64; 3]; 3]) -> [f64; 9] {
    [
        matrix[0][0],
        matrix[0][1],
        matrix[0][2],
        matrix[1][0],
        matrix[1][1],
        matrix[1][2],
        matrix[2][0],
        matrix[2][1],
        matrix[2][2],
    ]
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
        geometry_quality: empty_geometry_quality(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    const T_RX_J2000_S: f64 = 646_272_000.0;
    const T_RX_SOD_S: f64 = 43_200.0;
    const DAY_OF_YEAR: f64 = 176.5;
    const RECEIVER: [f64; 3] = [4_500_000.0, 500_000.0, 4_500_000.0];
    const RECEIVER_VELOCITY: [f64; 3] = [12.0, -7.0, 3.0];
    const CLOCK_BIAS_M: f64 = 8.0;
    const CLOCK_DRIFT_S_S: f64 = 1.0e-9;

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    fn visible_gps(sp3: &Sp3) -> Vec<GnssSatelliteId> {
        let planning = PredictOptions {
            light_time: false,
            ..PredictOptions::default()
        };
        sp3.satellites()
            .iter()
            .copied()
            .filter(|sat| sat.system == GnssSystem::Gps)
            .filter(|sat| {
                observables_predict(sp3, *sat, RECEIVER, T_RX_J2000_S, planning)
                    .map(|obs| obs.elevation_deg >= 5.0)
                    .unwrap_or(false)
            })
            .take(8)
            .collect()
    }

    fn pseudorange(sp3: &Sp3, sat: GnssSatelliteId) -> f64 {
        let obs = observables_predict(sp3, sat, RECEIVER, T_RX_J2000_S, PredictOptions::default())
            .expect("predict pseudorange");
        obs.geometric_range_m - sidereon_core::constants::C_M_S * obs.sat_clock_s.unwrap_or(0.0)
            + CLOCK_BIAS_M
    }

    fn doppler(sp3: &Sp3, sat: GnssSatelliteId) -> f64 {
        let obs = observables_predict(sp3, sat, RECEIVER, T_RX_J2000_S, PredictOptions::default())
            .expect("predict Doppler");
        let receiver_projection = obs.los_unit[0] * RECEIVER_VELOCITY[0]
            + obs.los_unit[1] * RECEIVER_VELOCITY[1]
            + obs.los_unit[2] * RECEIVER_VELOCITY[2];
        let range_rate = obs.range_rate_m_s - receiver_projection
            + sidereon_core::constants::C_M_S * CLOCK_DRIFT_S_S;
        sidereon_core::velocity::range_rate_to_doppler(
            range_rate,
            sidereon_core::constants::F_L1_HZ,
        )
        .expect("range-rate to Doppler")
    }

    fn core_inputs(
        sp3: &Sp3,
        sats: &[GnssSatelliteId],
    ) -> (SolveInputs, Vec<CoreDopplerObservation>) {
        let observations = sats
            .iter()
            .copied()
            .map(|sat| Observation {
                satellite_id: sat,
                pseudorange_m: pseudorange(sp3, sat),
            })
            .collect::<Vec<_>>();
        let doppler_observations = sats
            .iter()
            .copied()
            .map(|sat| CoreDopplerObservation {
                satellite_id: sat,
                doppler_hz: doppler(sp3, sat),
                carrier_hz: sidereon_core::constants::F_L1_HZ,
                sat_clock_drift_s_s: 0.0,
            })
            .collect::<Vec<_>>();
        (
            SolveInputs {
                observations,
                t_rx_j2000_s: T_RX_J2000_S,
                t_rx_second_of_day_s: T_RX_SOD_S,
                day_of_year: DAY_OF_YEAR,
                initial_guess: [
                    RECEIVER[0] + 25.0,
                    RECEIVER[1] - 20.0,
                    RECEIVER[2] + 15.0,
                    0.0,
                ],
                corrections: Corrections::NONE,
                klobuchar: KlobucharCoeffs {
                    alpha: [0.0; 4],
                    beta: [0.0; 4],
                },
                beidou_klobuchar: None,
                galileo_nequick: None,
                sbas_iono: None,
                glonass_channels: BTreeMap::new(),
                met: SurfaceMet::default(),
                robust: None,
            },
            doppler_observations,
        )
    }

    fn c_inputs(
        inputs: &SolveInputs,
        doppler_observations: &[CoreDopplerObservation],
        tokens: &[CString],
    ) -> (
        Vec<SidereonObservation>,
        Vec<SidereonSppDopplerObservation>,
        SidereonSppInputsV2,
    ) {
        let observations = inputs
            .observations
            .iter()
            .zip(tokens)
            .map(|(obs, token)| SidereonObservation {
                sat_id: token.as_ptr(),
                pseudorange_m: obs.pseudorange_m,
            })
            .collect::<Vec<_>>();
        let c_doppler = doppler_observations
            .iter()
            .zip(tokens)
            .map(|(obs, token)| SidereonSppDopplerObservation {
                sat_id: token.as_ptr(),
                doppler_hz: obs.doppler_hz,
                carrier_hz: obs.carrier_hz,
                sat_clock_drift_s_s: obs.sat_clock_drift_s_s,
            })
            .collect::<Vec<_>>();

        let mut c_inputs = MaybeUninit::<SidereonSppInputsV2>::uninit();
        let status = unsafe { sidereon_spp_inputs_v2_init(c_inputs.as_mut_ptr()) };
        assert_eq!(status, SidereonStatus::Ok);
        let mut c_inputs = unsafe { c_inputs.assume_init() };
        c_inputs.base = SidereonSppInputs {
            observations: observations.as_ptr(),
            observation_count: observations.len(),
            t_rx_j2000_s: inputs.t_rx_j2000_s,
            t_rx_second_of_day_s: inputs.t_rx_second_of_day_s,
            day_of_year: inputs.day_of_year,
            initial_guess: inputs.initial_guess,
            ionosphere: false,
            troposphere: false,
            klobuchar_alpha: [0.0; 4],
            klobuchar_beta: [0.0; 4],
            pressure_hpa: SurfaceMet::default().pressure_hpa,
            temperature_k: SurfaceMet::default().temperature_k,
            relative_humidity: SurfaceMet::default().relative_humidity,
            with_geodetic: true,
        };
        (observations, c_doppler, c_inputs)
    }

    fn assert_close(got: f64, want: f64, tol: f64) {
        assert!(
            (got - want).abs() <= tol,
            "got {got:e}, want {want:e}, tol {tol:e}"
        );
    }

    #[test]
    fn spp_doppler_solution_surfaces_receiver_drift_and_covariance() {
        let sp3 = fixture_sp3();
        let sats = visible_gps(&sp3);
        assert!(sats.len() >= 4);
        let (inputs, doppler_observations) = core_inputs(&sp3, &sats);
        let expected = core_solve_with_doppler_velocity(&sp3, &inputs, &doppler_observations, true)
            .expect("core combined solve");
        assert!(expected.velocity.is_some());
        assert!(expected.receiver.rx_clock_drift_s_s.is_some());

        let tokens = sats
            .iter()
            .map(|sat| CString::new(sat.to_string()).expect("sat token"))
            .collect::<Vec<_>>();
        let (observations, c_doppler, c_inputs) = c_inputs(&inputs, &doppler_observations, &tokens);
        let sp3_handle = SidereonSp3 { inner: sp3 };
        let mut solution = ptr::null_mut();
        let status = unsafe {
            sidereon_solve_spp_with_doppler_velocity(
                &sp3_handle,
                &c_inputs,
                c_doppler.as_ptr(),
                c_doppler.len(),
                &mut solution,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!solution.is_null());
        drop(observations);

        let mut has_velocity = false;
        let status =
            unsafe { sidereon_spp_doppler_solution_has_velocity(solution, &mut has_velocity) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(has_velocity);

        let mut velocity_error = SidereonSppDopplerVelocityErrorKind::InvalidInput;
        let status = unsafe {
            sidereon_spp_doppler_solution_velocity_error_kind(solution, &mut velocity_error)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(velocity_error, SidereonSppDopplerVelocityErrorKind::None);

        let mut receiver = ptr::null_mut();
        let status = unsafe { sidereon_spp_doppler_solution_receiver(solution, &mut receiver) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!receiver.is_null());

        let mut position = [0.0; 3];
        let status = unsafe { sidereon_spp_solution_position(receiver, position.as_mut_ptr(), 3) };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in position.iter().zip(expected.receiver.position.as_array()) {
            assert_close(*got, want, 1.0e-8);
        }

        let mut drift_present = false;
        let mut drift = 0.0;
        let status = unsafe {
            sidereon_spp_solution_rx_clock_drift_s_s(receiver, &mut drift_present, &mut drift)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(drift_present);
        assert_close(
            drift,
            expected.receiver.rx_clock_drift_s_s.expect("clock drift"),
            1.0e-18,
        );

        let mut covariance = [0.0; 9];
        let status = unsafe {
            sidereon_spp_solution_position_covariance_ecef_m2(receiver, covariance.as_mut_ptr(), 9)
        };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in covariance.iter().zip(flatten_spp_mat3(
            expected.receiver.position_covariance.ecef_m2,
        )) {
            assert_close(*got, want, 1.0e-12);
        }

        let mut velocity = ptr::null_mut();
        let status = unsafe { sidereon_spp_doppler_solution_velocity(solution, &mut velocity) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!velocity.is_null());

        let mut velocity_xyz = [0.0; 3];
        let status =
            unsafe { sidereon_velocity_solution_velocity(velocity, velocity_xyz.as_mut_ptr(), 3) };
        assert_eq!(status, SidereonStatus::Ok);
        let expected_velocity = expected.velocity.as_ref().expect("velocity");
        for (got, want) in velocity_xyz.iter().zip(expected_velocity.velocity_m_s) {
            assert_close(*got, want, 1.0e-9);
        }

        unsafe {
            sidereon_velocity_solution_free(velocity);
            sidereon_spp_solution_free(receiver);
            sidereon_spp_doppler_solution_free(solution);
        }
    }
}
