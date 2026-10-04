use super::*;
use serde_json::json;

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
    pub(crate) velocity_error_detail: Option<SppBatchRowError>,
}

/// Caller-populated inputs for a single SPP solve. Mirrors the engine solve
/// input field for field; the binding adds no defaults or modeling of its own.
#[repr(C)]
#[derive(Clone, Copy)]
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
    /// Which code the pseudoranges are. The broadcast single-frequency group
    /// delay (TGD, BGD) applies to single-frequency code only, as RTKLIB
    /// `prange` applies it to P1 and none under IFLC. A SidereonPseudorangeCode
    /// value; zero-initialized inputs read as
    /// SIDEREON_PSEUDORANGE_CODE_SINGLE_FREQUENCY, and any other value is
    /// refused with SIDEREON_STATUS_INVALID_ARGUMENT.
    pub pseudorange_code: u32,
}

/// Which code an SPP solve's pseudoranges are. Mirrors
/// sidereon_core::positioning::PseudorangeCode.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPseudorangeCode {
    /// A single-frequency code (L1 C/A, E1, B1I): the broadcast group delay
    /// applies.
    SingleFrequency = 0,
    /// The ionosphere-free combination: no broadcast group delay applies.
    IonosphereFree = 1,
}

pub(crate) fn pseudorange_code_from_c(
    fn_name: &str,
    code: u32,
) -> Result<sidereon_core::positioning::PseudorangeCode, SidereonStatus> {
    match code {
        x if x == SidereonPseudorangeCode::SingleFrequency as u32 => {
            Ok(sidereon_core::positioning::PseudorangeCode::SingleFrequency)
        }
        x if x == SidereonPseudorangeCode::IonosphereFree as u32 => {
            Ok(sidereon_core::positioning::PseudorangeCode::IonosphereFree)
        }
        other => {
            set_last_error(format!("{fn_name}: unknown pseudorange_code {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

pub(crate) fn pseudorange_code_to_c(value: sidereon_core::positioning::PseudorangeCode) -> u32 {
    match value {
        sidereon_core::positioning::PseudorangeCode::SingleFrequency => {
            SidereonPseudorangeCode::SingleFrequency as u32
        }
        sidereon_core::positioning::PseudorangeCode::IonosphereFree => {
            SidereonPseudorangeCode::IonosphereFree as u32
        }
    }
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
    /// Maximum outer robust solves, including the warm start. The engine
    /// default (DEFAULT_ROBUST_MAX_OUTER, 100) is a safeguard: the reweighting
    /// ends when the position settles, when it cycles, or at this cap.
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
#[derive(Clone, Copy)]
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
    /// which leaves every non-GLONASS solve bit-identical. A GLONASS observation
    /// solved with the ionosphere correction and no matching channel, or a
    /// channel outside the -7..=6 FDMA allocation, is left out of the solve and
    /// reported as a rejected satellite with
    /// SIDEREON_SPP_REJECTION_REASON_IONOSPHERE_CARRIER_UNRESOLVED; the rest of
    /// the epoch is solved. Duplicate slots are rejected.
    pub glonass_channels: *const SidereonGlonassChannel,
    /// Number of GLONASS channel entries pointed to by glonass_channels.
    pub glonass_channel_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonQzssClock {
    Gps = 0,
    Separate = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTroposphereModel {
    Rtklib = 0,
    SaastamoinenNiell = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppModelOptions {
    pub qzss_clock: u32,
    pub troposphere_model: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppBatchInputV2 {
    pub inputs: SidereonSppInputsV2,
    pub models: SidereonSppModelOptions,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_model_options_init(
    out_options: *mut SidereonSppModelOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_spp_model_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_spp_model_options_init",
                "out_options"
            ));
            *out = SidereonSppModelOptions {
                qzss_clock: SidereonQzssClock::Gps as u32,
                troposphere_model: SidereonTroposphereModel::Rtklib as u32,
            };
            SidereonStatus::Ok
        },
    )
}

pub(super) fn spp_model_options_from_c(
    fn_name: &str,
    options: &SidereonSppModelOptions,
) -> Result<
    (
        sidereon_core::positioning::QzssClock,
        sidereon_core::positioning::TroposphereModel,
    ),
    SidereonStatus,
> {
    let qzss_clock = match options.qzss_clock {
        value if value == SidereonQzssClock::Gps as u32 => {
            sidereon_core::positioning::QzssClock::Gps
        }
        value if value == SidereonQzssClock::Separate as u32 => {
            sidereon_core::positioning::QzssClock::Separate
        }
        value => {
            set_last_error(format!("{fn_name}: invalid QZSS clock selector {value}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let troposphere_model = match options.troposphere_model {
        value if value == SidereonTroposphereModel::Rtklib as u32 => {
            sidereon_core::positioning::TroposphereModel::Rtklib
        }
        value if value == SidereonTroposphereModel::SaastamoinenNiell as u32 => {
            sidereon_core::positioning::TroposphereModel::SaastamoinenNiell
        }
        value => {
            set_last_error(format!("{fn_name}: invalid troposphere selector {value}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok((qzss_clock, troposphere_model))
}

pub(super) unsafe fn build_spp_solve_inputs_with_models(
    fn_name: &str,
    inputs: &SidereonSppInputsV2,
    options: *const SidereonSppModelOptions,
) -> Result<SolveInputs, SidereonStatus> {
    let mut solve_inputs = build_spp_solve_inputs(
        fn_name,
        &inputs.base,
        beidou_klobuchar_from_c(inputs),
        robust_config_from_c(inputs),
        glonass_channels_from_c(fn_name, inputs)?,
    )?;
    if !options.is_null() {
        let (qzss_clock, troposphere_model) = spp_model_options_from_c(fn_name, &*options)?;
        solve_inputs.qzss_clock = qzss_clock;
        solve_inputs.troposphere_model = troposphere_model;
    }
    Ok(solve_inputs)
}

/// Why an SPP observation was excluded from the final solve.
///
/// Selection reports the first reason in the core policy order: strict SSR
/// correction-size refusal, NoEphemeris, LowElevation, SbasIonoUncovered, then
/// IonosphereCarrierUnresolved. A satellite both below the mask and without a
/// carrier is reported as LowElevation. SbasWithdrawn is not reported by SPP.
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
    /// The ionosphere correction was requested and the satellite has no
    /// resolvable carrier frequency to scale the L1 delay to: a GLONASS
    /// satellite with no channel in glonass_channels, or a channel outside the
    /// -7..=6 FDMA allocation. GPS, QZSS, SBAS, Galileo, BeiDou and NavIC have
    /// fixed carriers and are never reported with this reason. The satellite is
    /// left out, as RTKLIB `rescode` leaves out a satellite whose `sat2freq` is
    /// zero, and the rest of the epoch is solved.
    IonosphereCarrierUnresolved = 4,
    /// Strict SSR size policy refused the orbit/clock correction. Read the exact
    /// magnitudes from the V2 rejected-satellite record.
    SsrCorrectionExceedsLimit = 5,
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

/// Rejected satellite with the exact SSR size payload when the rejection was
/// caused by the strict correction-size policy.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppRejectedSatV2 {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// First rejection reason.
    pub reason: SidereonSppRejectionReason,
    /// Whether the reason carries SSR correction magnitudes.
    pub has_size: bool,
    /// Refused orbit correction magnitude in metres; zero when absent.
    pub orbit_m: f64,
    /// Refused clock correction in metres, preserving its sign; zero when absent.
    pub clock_m: f64,
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
    /// The positioning solve ended with a least-squares step below its
    /// tolerance at a satellite selection that held (RTKLIB `estpos`'s
    /// `norm(dx) < 1E-4`), or a robust solve's position and selection
    /// settled. The solve converged.
    SelectionSettled = 4,
    /// A robust-reweighted solve spent its outer solve budget before its
    /// position and selection settled. The solve did not converge.
    OuterBudgetExhausted = 5,
    /// A robust reweighting returned within outer_tol_m of a state it reached
    /// two or more solves earlier, at the same selection, by a step no smaller
    /// than the one that led there: it was cycling (typically the robust scale
    /// alternating between two medians) and stopped there. The solve did not
    /// converge.
    OuterOscillation = 6,
}

/// Iteration, convergence, correction, and validation metadata for SPP.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSppMetadata {
    /// The trust-region iterations of every solve plus one per least-squares
    /// step.
    pub iterations: usize,
    /// Whether the whole solve converged. A robust solve that spent its outer
    /// budget reports false with OUTER_BUDGET_EXHAUSTED, and one that cycled
    /// false with OUTER_OSCILLATION.
    pub converged: bool,
    /// How the whole solve ended; a settled solve reports SELECTION_SETTLED.
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
    /// UT1 departure a permissive UT1 policy of the ephemeris source accepted
    /// while forming the solve; SIDEREON_UT1_DEGRADATION_NONE when UT1 came
    /// from the table or was not read.
    pub ut1_degraded: SidereonUt1Degradation,
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

/// Copy rejected satellites with optional strict-SSR refusal magnitudes.
/// Existing rejected-satellite records remain ABI-compatible through the V1
/// accessor.
///
/// # Safety
/// `sol` must be a live handle; output/count pointers must satisfy the standard
/// variable-length buffer contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_rejected_sats_v2(
    sol: *const SidereonSppSolution,
    out: *mut SidereonSppRejectedSatV2,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_spp_solution_rejected_sats_v2";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let sol = c_try!(require_ref(sol, FN, "solution"));
        let values = rejected_sats_v2_to_c(&sol.inner.rejected_sats);
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            &values,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
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

fn solution_pseudorange_variances(solution: &ReceiverSolution) -> &[f64] {
    &solution.pseudorange_variances_m2
}

fn solution_weights(solution: &ReceiverSolution) -> &[f64] {
    &solution.weights
}

unsafe fn copy_solution_f64s(
    fn_name: &str,
    sol: *const SidereonSppSolution,
    select: fn(&ReceiverSolution) -> &[f64],
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let sol = c_try!(require_ref(sol, fn_name, "solution"));
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        select(&sol.inner),
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

/// Copy the pseudorange variance (square metres) of each used satellite, in
/// used-satellite order: the RTKLIB rescode variance the solve weighted its
/// residual by, which RAIM standardizes the residual with. Uses the
/// variable-length output contract.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable doubles or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_pseudorange_variances(
    sol: *const SidereonSppSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_spp_solution_pseudorange_variances";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        copy_solution_f64s(
            FN_NAME,
            sol,
            solution_pseudorange_variances,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy the weight each used satellite carried in the reported solve, in
/// used-satellite order: the inverse pseudorange variance, times the final
/// Huber factor on the robust path. Uses the variable-length output contract.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable doubles or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_solution_weights(
    sol: *const SidereonSppSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_spp_solution_weights";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        copy_solution_f64s(
            FN_NAME,
            sol,
            solution_weights,
            out,
            len,
            out_written,
            out_required,
        )
    })
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
        let out_dop = c_try!(require_uninit_out(
            out_dop,
            "sidereon_spp_solution_dop",
            "out_dop"
        ));
        out_dop.write(SidereonDop {
            gdop: 0.0,
            pdop: 0.0,
            hdop: 0.0,
            vdop: 0.0,
            tdop: 0.0,
        });
        let sol = c_try!(require_ref(sol, "sidereon_spp_solution_dop", "solution"));
        let Some(dop) = sol.inner.dop.as_ref() else {
            set_last_error("sidereon_spp_solution_dop: geometry is rank-deficient, no DOP");
            return SidereonStatus::InvalidArgument;
        };
        out_dop.write(SidereonDop {
            gdop: dop.gdop,
            pdop: dop.pdop,
            hdop: dop.hdop,
            vdop: dop.vdop,
            tdop: dop.tdop,
        });
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
                ut1_degraded: SidereonUt1Degradation::from_core(metadata.ut1_degraded),
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
    crate::engine_error::engine_error_operation_boundary(
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
    crate::engine_error::engine_error_operation_boundary(
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

/// Read the owned structured velocity-error summary retained by a combined
/// SPP Doppler result. Successful or absent velocity results report None/0.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_velocity_error_info(
    solution: *const SidereonSppDopplerSolution,
    out_info: *mut SidereonEngineErrorInfo,
) -> SidereonStatus {
    const FN: &str = "sidereon_spp_doppler_solution_velocity_error_info";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_info = c_try!(require_out(out_info, FN, "out_info"));
        *out_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        let solution = c_try!(require_ref(solution, FN, "solution"));
        if let Some(error) = solution.velocity_error_detail.as_ref() {
            *out_info = SidereonEngineErrorInfo {
                family: error.family,
                payload_len: error.payload.len(),
            };
        }
        SidereonStatus::Ok
    })
}

/// Copy the complete owned velocity-error JSON payload retained by a combined
/// SPP Doppler result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_doppler_solution_velocity_error_payload(
    solution: *const SidereonSppDopplerSolution,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_spp_doppler_solution_velocity_error_payload";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let solution = c_try!(require_ref(solution, FN, "solution"));
        let payload = solution
            .velocity_error_detail
            .as_ref()
            .map_or(&[][..], |error| error.payload.as_bytes());
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            payload,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
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
    crate::engine_error::engine_error_operation_boundary(
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
            let mut opts = SolutionValidationOptions::default();
            opts.max_pdop = options.has_max_pdop.then_some(options.max_pdop);
            opts.min_plausible_radius_m = options.min_plausible_radius_m;
            opts.max_plausible_radius_m = options.max_plausible_radius_m;
            opts.max_converged_residual_rms_m = options.max_converged_residual_rms_m;
            match validate_receiver_solution(&solution.inner, opts) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => {
                    set_last_error(format!("sidereon_validate_receiver_solution: {err}"));
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::SolutionValidation,
                        "sidereon_validate_receiver_solution",
                        crate::engine_error::solution_validation_error_value(&err),
                    );
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

// ============================================================================

// --- Batch SPP (sidereon_core::positioning::solve_spp_batch_serial / _parallel) ------

/// Structured per-row failure recorded for an epoch solve in an SPP batch.
#[derive(Clone, Debug)]
pub(crate) struct SppBatchRowError {
    pub(crate) message: String,
    pub(crate) family: SidereonEngineErrorFamily,
    pub(crate) payload: String,
}

pub(crate) fn spp_batch_row_error_from_facade(
    row_fn: &str,
    err: sidereon::Error,
) -> SppBatchRowError {
    let message = err.to_string();
    let (family, error_node) = match &err {
        sidereon::Error::Spp(sidereon_core::positioning::SolvePolicyError::Solve(spp_err)) => (
            SidereonEngineErrorFamily::Spp,
            crate::engine_error::spp_error_value(spp_err),
        ),
        sidereon::Error::Spp(policy_err) => (
            SidereonEngineErrorFamily::SppPolicy,
            crate::engine_error::solve_policy_error_value(policy_err),
        ),
        _ => (
            SidereonEngineErrorFamily::Facade,
            crate::engine_error::facade_error_value(&err),
        ),
    };
    let payload = json!({
        "schema_version": 1,
        "family": family.name(),
        "operation": row_fn,
        "error": error_node,
    })
    .to_string();
    SppBatchRowError {
        message,
        family,
        payload,
    }
}

/// A batch of independent SPP epoch solves over one shared ephemeris. Opaque to
/// C. Create with sidereon_solve_spp_batch_serial or
/// sidereon_solve_spp_batch_parallel; read per-epoch results with the
/// sidereon_spp_batch_* accessors; release with sidereon_spp_batch_free. Element
/// i is the solve of input i; a per-epoch solve failure is recorded for that
/// element and does not fail the batch.
pub struct SidereonSppBatch {
    pub(crate) inner: Vec<Result<ReceiverSolution, SppBatchRowError>>,
}

/// Options for assembling RINEX OBS epochs into SPP inputs. Initialize with
/// sidereon_rinex_spp_options_init, then override fields as needed. A NULL
/// options pointer on the RINEX-SPP entry points uses these defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexSppOptions {
    /// Apply the ionosphere correction requested by the broadcast NAV product.
    pub ionosphere: bool,
    /// Apply the troposphere correction.
    pub troposphere: bool,
    /// Whether initial_guess overrides the RINEX header APPROX POSITION XYZ.
    pub initial_guess_enabled: bool,
    /// Optional initial state guess [x_m, y_m, z_m, clock_state].
    pub initial_guess: [f64; 4],
    /// Surface pressure, hPa.
    pub pressure_hpa: f64,
    /// Surface temperature, K.
    pub temperature_k: f64,
    /// Relative humidity, 0..1.
    pub relative_humidity: f64,
    /// Whether robust Huber/IRLS reweighting is applied to each assembled solve.
    pub robust_enabled: bool,
    /// Robust reweighting controls, used only when robust_enabled is true.
    pub robust: SidereonSppRobustConfig,
}

/// One assembled RINEX OBS epoch summary for SPP.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexSppEpoch {
    /// Index in the source RINEX observation file.
    pub epoch_index: usize,
    /// Civil epoch exactly as it appears in the RINEX observation file.
    pub epoch: SidereonCalendarEpoch,
    /// Number of selected pseudorange observations in the assembled SPP input.
    pub observation_count: usize,
}

/// A set of SPP inputs assembled from RINEX OBS epochs. Opaque to C. Create with
/// sidereon_spp_inputs_from_rinex_obs and release with
/// sidereon_rinex_spp_inputs_free.
pub struct SidereonRinexSppInputs {
    pub(crate) inner: Vec<RinexSppEpochInputs>,
    c_rows: Vec<SidereonSppInputsV2>,
    _observations: Vec<Vec<SidereonObservation>>,
    _sat_ids: Vec<Vec<CString>>,
    _glonass_channels: Vec<Vec<SidereonGlonassChannel>>,
}

/// Serial per-epoch SPP solve results assembled from RINEX OBS epochs. Opaque to
/// C. Create with sidereon_solve_spp_from_rinex_obs and release with
/// sidereon_rinex_spp_solutions_free.
pub struct SidereonRinexSppSolutions {
    pub(crate) inner: Vec<RinexSppEpochSolution>,
}

/// Initialize RINEX-SPP options to the engine defaults: RINEX-version-specific
/// signal selection, ionosphere and troposphere on, RINEX header initial guess,
/// standard atmosphere, and robust reweighting off.
///
/// Safety: out_options must point to writable SidereonRinexSppOptions storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_options_init(
    out_options: *mut SidereonRinexSppOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_rinex_spp_options_init",
                "out_options"
            ));
            *out_options = default_rinex_spp_options();
            SidereonStatus::Ok
        },
    )
}

/// Assemble every usable RINEX OBS epoch into SPP inputs using a broadcast NAV
/// source for atmosphere metadata and GLONASS channel context. On success writes
/// a newly owned handle to *out_inputs. Release it with
/// sidereon_rinex_spp_inputs_free. Delegates to
/// sidereon::spp_inputs_from_rinex_obs.
///
/// Safety: obs and broadcast must be live handles; options may be NULL for
/// defaults; out_inputs must point to storage for a SidereonRinexSppInputs*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_inputs_from_rinex_obs(
    obs: *const SidereonRinexObs,
    broadcast: *const SidereonBroadcastEphemeris,
    options: *const SidereonRinexSppOptions,
    out_inputs: *mut *mut SidereonRinexSppInputs,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_spp_inputs_from_rinex_obs",
        SidereonStatus::Panic,
        || {
            let out_inputs = c_try!(require_out(
                out_inputs,
                "sidereon_spp_inputs_from_rinex_obs",
                "out_inputs"
            ));
            *out_inputs = ptr::null_mut();
            let obs = c_try!(require_ref(
                obs,
                "sidereon_spp_inputs_from_rinex_obs",
                "obs"
            ));
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_spp_inputs_from_rinex_obs",
                "broadcast"
            ));
            let options = c_try!(rinex_spp_options_from_c(
                "sidereon_spp_inputs_from_rinex_obs",
                obs,
                options
            ));
            let inner =
                match sidereon::spp_inputs_from_rinex_obs(&obs.inner, &broadcast.inner, &options) {
                    Ok(inner) => inner,
                    Err(err) => {
                        return map_rinex_spp_error("sidereon_spp_inputs_from_rinex_obs", err);
                    }
                };
            write_boxed_handle(out_inputs, SidereonRinexSppInputs::from_core(inner));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_inputs_from_rinex_obs_with_models(
    obs: *const SidereonRinexObs,
    broadcast: *const SidereonBroadcastEphemeris,
    options: *const SidereonRinexSppOptions,
    models: *const SidereonSppModelOptions,
    out_inputs: *mut *mut SidereonRinexSppInputs,
) -> SidereonStatus {
    const FN: &str = "sidereon_spp_inputs_from_rinex_obs_with_models";
    crate::engine_error::engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_inputs = c_try!(require_out(out_inputs, FN, "out_inputs"));
        *out_inputs = ptr::null_mut();
        let obs = c_try!(require_ref(obs, FN, "obs"));
        let broadcast = c_try!(require_ref(broadcast, FN, "broadcast"));
        let mut options = c_try!(rinex_spp_options_from_c(FN, obs, options));
        if !models.is_null() {
            let (qzss_clock, troposphere_model) =
                c_try!(crate::spp::spp_model_options_from_c(FN, &*models));
            options.qzss_clock = qzss_clock;
            options.troposphere_model = troposphere_model;
        }
        let inner =
            match sidereon::spp_inputs_from_rinex_obs(&obs.inner, &broadcast.inner, &options) {
                Ok(inner) => inner,
                Err(err) => return map_rinex_spp_error(FN, err),
            };
        write_boxed_handle(out_inputs, SidereonRinexSppInputs::from_core(inner));
        SidereonStatus::Ok
    })
}

/// Solve every usable RINEX OBS epoch serially against a broadcast NAV source.
/// Per-epoch solve failures are retained in the returned handle and do not fail
/// the overall call. Release the returned handle with
/// sidereon_rinex_spp_solutions_free. Delegates to
/// sidereon::solve_spp_from_rinex_obs.
///
/// Safety: broadcast and obs must be live handles; options and policy may be
/// NULL for defaults; out_solutions must point to storage for a
/// SidereonRinexSppSolutions*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_from_rinex_obs(
    broadcast: *const SidereonBroadcastEphemeris,
    obs: *const SidereonRinexObs,
    options: *const SidereonRinexSppOptions,
    with_geodetic: bool,
    policy: *const SidereonSppSolvePolicy,
    out_solutions: *mut *mut SidereonRinexSppSolutions,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_solve_spp_from_rinex_obs",
        SidereonStatus::Panic,
        || {
            let out_solutions = c_try!(require_out(
                out_solutions,
                "sidereon_solve_spp_from_rinex_obs",
                "out_solutions"
            ));
            *out_solutions = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_solve_spp_from_rinex_obs",
                "broadcast"
            ));
            let obs = c_try!(require_ref(obs, "sidereon_solve_spp_from_rinex_obs", "obs"));
            let options = c_try!(rinex_spp_options_from_c(
                "sidereon_solve_spp_from_rinex_obs",
                obs,
                options,
            ));
            let policy = if policy.is_null() {
                SolvePolicy::default()
            } else {
                c_try!(crate::solve::solve_policy_from_c(
                    "sidereon_solve_spp_from_rinex_obs",
                    &*policy
                ))
            };
            let inner = match sidereon::solve_spp_from_rinex_obs(
                &broadcast.inner,
                &obs.inner,
                &options,
                with_geodetic,
                policy,
            ) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_rinex_spp_error("sidereon_solve_spp_from_rinex_obs", err);
                }
            };
            write_boxed_handle(out_solutions, SidereonRinexSppSolutions { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_from_rinex_obs_with_models(
    broadcast: *const SidereonBroadcastEphemeris,
    obs: *const SidereonRinexObs,
    options: *const SidereonRinexSppOptions,
    models: *const SidereonSppModelOptions,
    with_geodetic: bool,
    policy: *const SidereonSppSolvePolicy,
    out_solutions: *mut *mut SidereonRinexSppSolutions,
) -> SidereonStatus {
    const FN: &str = "sidereon_solve_spp_from_rinex_obs_with_models";
    crate::engine_error::engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_solutions = c_try!(require_out(out_solutions, FN, "out_solutions"));
        *out_solutions = ptr::null_mut();
        let broadcast = c_try!(require_ref(broadcast, FN, "broadcast"));
        let obs = c_try!(require_ref(obs, FN, "obs"));
        let mut options = c_try!(rinex_spp_options_from_c(FN, obs, options));
        if !models.is_null() {
            let (qzss_clock, troposphere_model) =
                c_try!(crate::spp::spp_model_options_from_c(FN, &*models));
            options.qzss_clock = qzss_clock;
            options.troposphere_model = troposphere_model;
        }
        let policy = if policy.is_null() {
            SolvePolicy::default()
        } else {
            c_try!(crate::solve::solve_policy_from_c(FN, &*policy))
        };
        let inner = match sidereon::solve_spp_from_rinex_obs(
            &broadcast.inner,
            &obs.inner,
            &options,
            with_geodetic,
            policy,
        ) {
            Ok(inner) => inner,
            Err(err) => return map_rinex_spp_error(FN, err),
        };
        write_boxed_handle(out_solutions, SidereonRinexSppSolutions { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of assembled RINEX-SPP epochs to *out_count.
///
/// Safety: inputs is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_inputs_count(
    inputs: *const SidereonRinexSppInputs,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_inputs_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rinex_spp_inputs_count",
                "out_count"
            ));
            *out_count = 0;
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_rinex_spp_inputs_count",
                "inputs"
            ));
            *out_count = inputs.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy metadata for assembled RINEX-SPP epoch `index`.
///
/// Safety: inputs is a live handle; out_epoch points to
/// SidereonRinexSppEpoch storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_inputs_epoch(
    inputs: *const SidereonRinexSppInputs,
    index: usize,
    out_epoch: *mut SidereonRinexSppEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_inputs_epoch",
        SidereonStatus::Panic,
        || {
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_rinex_spp_inputs_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_rinex_spp_epoch();
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_rinex_spp_inputs_epoch",
                "inputs"
            ));
            let epoch = c_try!(rinex_spp_input_at(
                "sidereon_rinex_spp_inputs_epoch",
                &inputs.inner,
                index
            ));
            *out_epoch = rinex_spp_epoch_inputs_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Copy assembled SPP inputs for epoch `index`. Pointer fields in the copied
/// SidereonSppInputsV2 borrow storage owned by `inputs` and remain valid until
/// sidereon_rinex_spp_inputs_free is called.
///
/// Safety: inputs is a live handle; out_inputs points to SidereonSppInputsV2
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_inputs_epoch_inputs(
    inputs: *const SidereonRinexSppInputs,
    index: usize,
    out_inputs: *mut SidereonSppInputsV2,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_inputs_epoch_inputs",
        SidereonStatus::Panic,
        || {
            let out_inputs = c_try!(require_out(
                out_inputs,
                "sidereon_rinex_spp_inputs_epoch_inputs",
                "out_inputs"
            ));
            *out_inputs = default_spp_inputs_v2();
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_rinex_spp_inputs_epoch_inputs",
                "inputs"
            ));
            let row = match inputs.c_rows.get(index) {
                Some(row) => row,
                None => {
                    set_last_error(format!(
                        "sidereon_rinex_spp_inputs_epoch_inputs: index {index} out of range ({} epochs)",
                        inputs.c_rows.len()
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out_inputs = *row;
            SidereonStatus::Ok
        },
    )
}

/// Release RINEX-SPP assembled inputs. Passing NULL is a no-op.
///
/// Safety: inputs must be NULL or a live handle from
/// sidereon_spp_inputs_from_rinex_obs that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_inputs_free(inputs: *mut SidereonRinexSppInputs) {
    free_boxed(inputs);
}

/// Write the number of per-epoch RINEX-SPP solve results to *out_count.
///
/// Safety: solutions is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solutions_count(
    solutions: *const SidereonRinexSppSolutions,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_solutions_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rinex_spp_solutions_count",
                "out_count"
            ));
            *out_count = 0;
            let solutions = c_try!(require_ref(
                solutions,
                "sidereon_rinex_spp_solutions_count",
                "solutions"
            ));
            *out_count = solutions.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy metadata for RINEX-SPP solve result `index`.
///
/// Safety: solutions is a live handle; out_epoch points to
/// SidereonRinexSppEpoch storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solutions_epoch(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out_epoch: *mut SidereonRinexSppEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_solutions_epoch",
        SidereonStatus::Panic,
        || {
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_rinex_spp_solutions_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_rinex_spp_epoch();
            let solutions = c_try!(require_ref(
                solutions,
                "sidereon_rinex_spp_solutions_epoch",
                "solutions"
            ));
            let epoch = c_try!(rinex_spp_solution_at(
                "sidereon_rinex_spp_solutions_epoch",
                &solutions.inner,
                index
            ));
            *out_epoch = rinex_spp_epoch_solution_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Write whether RINEX-SPP result `index` solved to *out_ok.
///
/// Safety: solutions is a live handle; out_ok points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solution_ok(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out_ok: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_solution_ok",
        SidereonStatus::Panic,
        || {
            let out_ok = c_try!(require_out(
                out_ok,
                "sidereon_rinex_spp_solution_ok",
                "out_ok"
            ));
            *out_ok = false;
            let solutions = c_try!(require_ref(
                solutions,
                "sidereon_rinex_spp_solution_ok",
                "solutions"
            ));
            let epoch = c_try!(rinex_spp_solution_at(
                "sidereon_rinex_spp_solution_ok",
                &solutions.inner,
                index
            ));
            *out_ok = epoch.solution.is_ok();
            SidereonStatus::Ok
        },
    )
}

/// Copy RINEX-SPP result `index` into a newly owned SidereonSppSolution handle.
/// Returns SIDEREON_STATUS_SOLVE when that epoch failed to solve.
///
/// Safety: solutions is a live handle; out_solution points to storage for a
/// SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solution(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_spp_solution", SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(
            out_solution,
            "sidereon_rinex_spp_solution",
            "out_solution"
        ));
        *out_solution = ptr::null_mut();
        let solutions = c_try!(require_ref(
            solutions,
            "sidereon_rinex_spp_solution",
            "solutions"
        ));
        let epoch = c_try!(rinex_spp_solution_at(
            "sidereon_rinex_spp_solution",
            &solutions.inner,
            index
        ));
        match &epoch.solution {
            Ok(solution) => {
                write_boxed_handle(
                    out_solution,
                    SidereonSppSolution {
                        inner: solution.clone(),
                    },
                );
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!(
                    "sidereon_rinex_spp_solution: epoch {index} did not solve: {err}"
                ));
                SidereonStatus::Solve
            }
        }
    })
}

/// Copy RINEX-SPP result `index`'s solve-failure message into a caller buffer
/// (not null-terminated). A solved epoch reports *out_required 0. Uses the
/// variable-length output contract.
///
/// Safety: solutions is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solution_error(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_spp_solution_error",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_spp_solution_error",
                out_written,
                out_required
            ));
            let solutions = c_try!(require_ref(
                solutions,
                "sidereon_rinex_spp_solution_error",
                "solutions"
            ));
            let epoch = c_try!(rinex_spp_solution_at(
                "sidereon_rinex_spp_solution_error",
                &solutions.inner,
                index
            ));
            let message = match &epoch.solution {
                Ok(_) => String::new(),
                Err(err) => err.to_string(),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_spp_solution_error",
                "out",
                message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Read the typed engine-error family and required JSON payload size for one
/// RINEX-SPP epoch. A successful epoch reports family None and payload_len 0.
/// The payload bytes can be copied with sidereon_rinex_spp_solution_error_payload.
///
/// Safety: solutions is a live handle; out_info points to a
/// SidereonEngineErrorInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solution_error_info(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out_info: *mut SidereonEngineErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_spp_solution_error_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_info = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        let solutions = c_try!(require_ref(solutions, FN_NAME, "solutions"));
        let epoch = c_try!(rinex_spp_solution_at(FN_NAME, &solutions.inner, index));
        if let Err(error) = &epoch.solution {
            let row_error = rinex_spp_row_error(error);
            *out_info = SidereonEngineErrorInfo {
                family: row_error.family,
                payload_len: row_error.payload.len(),
            };
        }
        SidereonStatus::Ok
    })
}

/// Copy the owned-schema JSON details for a failed RINEX-SPP epoch. The
/// legacy sidereon_rinex_spp_solution_error continues to return its diagnostic
/// string. A successful epoch copies zero bytes.
///
/// Safety: solutions is a live handle; out points to len writable bytes or
/// NULL when len is zero; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solution_error_payload(
    solutions: *const SidereonRinexSppSolutions,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_spp_solution_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let solutions = c_try!(require_ref(solutions, FN_NAME, "solutions"));
        let epoch = c_try!(rinex_spp_solution_at(FN_NAME, &solutions.inner, index));
        let payload = match &epoch.solution {
            Ok(_) => String::new(),
            Err(error) => rinex_spp_row_error(error).payload,
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            payload.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn rinex_spp_row_error(error: &sidereon_core::positioning::SolvePolicyError) -> SppBatchRowError {
    spp_batch_row_error_from_facade(
        "sidereon_rinex_spp_solution_error_payload",
        sidereon::Error::Spp(error.clone()),
    )
}

/// Release RINEX-SPP solve results. Passing NULL is a no-op.
///
/// Safety: solutions must be NULL or a live handle from
/// sidereon_solve_spp_from_rinex_obs that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_spp_solutions_free(
    solutions: *mut SidereonRinexSppSolutions,
) {
    free_boxed(solutions);
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
            Err(row_err) => {
                set_last_error(format!(
                    "sidereon_spp_batch_solution: epoch {index} did not solve: {}",
                    row_err.message
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
            Err(row_err) => row_err.message.as_str(),
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

/// Read the structured error summary for epoch `index` in an SPP batch.
/// An epoch that solved reports SidereonEngineErrorFamily::None and payload_len 0.
///
/// Safety: batch is a live handle; out_info points to a SidereonEngineErrorInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_error_info(
    batch: *const SidereonSppBatch,
    index: usize,
    out_info: *mut SidereonEngineErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_spp_batch_error_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_info = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        let batch = c_try!(require_ref(batch, FN_NAME, "batch"));
        let entry = match batch.inner.get(index) {
            Some(entry) => entry,
            None => {
                set_last_error(format!(
                    "{FN_NAME}: index {index} out of range ({} results)",
                    batch.inner.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        match entry {
            Ok(_) => {
                *out_info = SidereonEngineErrorInfo {
                    family: SidereonEngineErrorFamily::None,
                    payload_len: 0,
                };
                SidereonStatus::Ok
            }
            Err(row_err) => {
                *out_info = SidereonEngineErrorInfo {
                    family: row_err.family,
                    payload_len: row_err.payload.len(),
                };
                SidereonStatus::Ok
            }
        }
    })
}

/// Copy the owned Schema 1 UTF-8 JSON payload for epoch `index` in an SPP batch.
/// Follows standard variable-length copy semantics. An epoch that solved reports
/// *out_written 0 and *out_required 0.
///
/// Safety: batch is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_spp_batch_error_payload(
    batch: *const SidereonSppBatch,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_spp_batch_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let batch = c_try!(require_ref(batch, FN_NAME, "batch"));
        let entry = match batch.inner.get(index) {
            Some(entry) => entry,
            None => {
                set_last_error(format!(
                    "{FN_NAME}: index {index} out of range ({} results)",
                    batch.inner.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        let payload = match entry {
            Ok(_) => "",
            Err(row_err) => row_err.payload.as_str(),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            payload.as_bytes(),
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_spp_batch_v2_serial(
    sp3: *const SidereonSp3,
    epochs: *const SidereonSppBatchInputV2,
    epoch_count: usize,
    out_batch: *mut *mut SidereonSppBatch,
) -> SidereonStatus {
    const FN: &str = "sidereon_solve_spp_batch_v2_serial";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_batch = c_try!(require_out(out_batch, FN, "out_batch"));
        *out_batch = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, FN, "sp3"));
        let epochs = c_try!(require_slice(epochs, epoch_count, FN, "epochs"));
        let mut results = Vec::with_capacity(epochs.len());
        for (index, epoch) in epochs.iter().enumerate() {
            let row_fn = format!("{FN} epoch {index}");
            let solve_inputs =
                match build_spp_solve_inputs_with_models(&row_fn, &epoch.inputs, &epoch.models) {
                    Ok(inputs) => inputs,
                    Err(status) => return status,
                };
            let policy = match crate::solve::solve_policy_from_c(&row_fn, &epoch.inputs.policy) {
                Ok(policy) => policy,
                Err(status) => return status,
            };
            let result = sidereon::solve_spp(
                &sp3.inner,
                &solve_inputs,
                epoch.inputs.base.with_geodetic,
                policy,
            )
            .map_err(|error| spp_batch_row_error_from_facade(&row_fn, error));
            results.push(result);
        }
        write_boxed_handle(out_batch, SidereonSppBatch { inner: results });
        SidereonStatus::Ok
    })
}

impl SidereonRinexSppInputs {
    fn from_core(inner: Vec<RinexSppEpochInputs>) -> Self {
        let mut sat_ids = Vec::with_capacity(inner.len());
        let mut observations: Vec<Vec<SidereonObservation>> = Vec::with_capacity(inner.len());
        let mut glonass_channels: Vec<Vec<SidereonGlonassChannel>> =
            Vec::with_capacity(inner.len());

        for epoch in &inner {
            let ids: Vec<CString> = epoch
                .inputs
                .observations
                .iter()
                .map(|obs| CString::new(obs.satellite_id.to_string()).expect("satellite token"))
                .collect::<Vec<_>>();
            let obs_rows = epoch
                .inputs
                .observations
                .iter()
                .zip(ids.iter())
                .map(|(obs, sat_id)| SidereonObservation {
                    sat_id: sat_id.as_ptr(),
                    pseudorange_m: obs.pseudorange_m,
                })
                .collect::<Vec<_>>();
            let channel_rows = epoch
                .inputs
                .glonass_channels
                .iter()
                .map(|(slot, channel)| SidereonGlonassChannel {
                    slot: *slot,
                    channel: *channel,
                })
                .collect();
            sat_ids.push(ids);
            observations.push(obs_rows);
            glonass_channels.push(channel_rows);
        }

        let c_rows = inner
            .iter()
            .enumerate()
            .map(|(idx, epoch)| {
                rinex_spp_solve_inputs_to_c_row(
                    &epoch.inputs,
                    &observations[idx],
                    &glonass_channels[idx],
                )
            })
            .collect();

        Self {
            inner,
            c_rows,
            _observations: observations,
            _sat_ids: sat_ids,
            _glonass_channels: glonass_channels,
        }
    }
}

fn default_rinex_spp_options() -> SidereonRinexSppOptions {
    let met = SurfaceMet::default();
    SidereonRinexSppOptions {
        ionosphere: true,
        troposphere: true,
        initial_guess_enabled: false,
        initial_guess: [0.0; 4],
        pressure_hpa: met.pressure_hpa,
        temperature_k: met.temperature_k,
        relative_humidity: met.relative_humidity,
        robust_enabled: false,
        robust: default_robust_config(),
    }
}

unsafe fn rinex_spp_options_from_c(
    fn_name: &str,
    obs: &SidereonRinexObs,
    options: *const SidereonRinexSppOptions,
) -> Result<RinexSppOptions, SidereonStatus> {
    let mut out = RinexSppOptions::default_for(&obs.inner).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })?;
    if options.is_null() {
        return Ok(out);
    }
    let options = &*options;
    out.corrections = Corrections {
        ionosphere: options.ionosphere,
        troposphere: options.troposphere,
    };
    out.qzss_clock = sidereon_core::positioning::QzssClock::Gps;
    out.troposphere_model = sidereon_core::positioning::TroposphereModel::Rtklib;
    if options.initial_guess_enabled {
        out.initial_guess = Some(options.initial_guess);
    }
    out.met = SurfaceMet {
        pressure_hpa: options.pressure_hpa,
        temperature_k: options.temperature_k,
        relative_humidity: options.relative_humidity,
    };
    if options.robust_enabled {
        let mut robust = RobustConfig::default();
        robust.huber_k = options.robust.huber_k;
        robust.scale_floor_m = options.robust.scale_floor_m;
        robust.max_outer = options.robust.max_outer;
        robust.outer_tol_m = options.robust.outer_tol_m;
        out.robust = Some(robust);
    }
    Ok(out)
}

fn rinex_spp_solve_inputs_to_c_row(
    inputs: &SolveInputs,
    observations: &[SidereonObservation],
    glonass_channels: &[SidereonGlonassChannel],
) -> SidereonSppInputsV2 {
    let robust = inputs.robust.unwrap_or_default();
    SidereonSppInputsV2 {
        base: SidereonSppInputs {
            observations: observations.as_ptr(),
            observation_count: observations.len(),
            t_rx_j2000_s: inputs.t_rx_j2000_s,
            t_rx_second_of_day_s: inputs.t_rx_second_of_day_s,
            day_of_year: inputs.day_of_year,
            initial_guess: inputs.initial_guess,
            ionosphere: inputs.corrections.ionosphere,
            troposphere: inputs.corrections.troposphere,
            klobuchar_alpha: inputs.klobuchar.alpha,
            klobuchar_beta: inputs.klobuchar.beta,
            pressure_hpa: inputs.met.pressure_hpa,
            temperature_k: inputs.met.temperature_k,
            relative_humidity: inputs.met.relative_humidity,
            with_geodetic: false,
            pseudorange_code: pseudorange_code_to_c(inputs.pseudorange_code),
        },
        beidou_klobuchar_enabled: inputs.beidou_klobuchar.is_some(),
        beidou_klobuchar_alpha: inputs
            .beidou_klobuchar
            .map(|coeffs| coeffs.alpha)
            .unwrap_or([0.0; 4]),
        beidou_klobuchar_beta: inputs
            .beidou_klobuchar
            .map(|coeffs| coeffs.beta)
            .unwrap_or([0.0; 4]),
        robust_enabled: inputs.robust.is_some(),
        robust: SidereonSppRobustConfig {
            huber_k: robust.huber_k,
            scale_floor_m: robust.scale_floor_m,
            max_outer: robust.max_outer,
            outer_tol_m: robust.outer_tol_m,
        },
        policy: default_solve_policy(),
        glonass_channels: glonass_channels.as_ptr(),
        glonass_channel_count: glonass_channels.len(),
    }
}

fn empty_rinex_spp_epoch() -> SidereonRinexSppEpoch {
    SidereonRinexSppEpoch {
        epoch_index: 0,
        epoch: SidereonCalendarEpoch {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        observation_count: 0,
    }
}

fn rinex_spp_epoch_inputs_to_c(epoch: &RinexSppEpochInputs) -> SidereonRinexSppEpoch {
    SidereonRinexSppEpoch {
        epoch_index: epoch.epoch_index,
        epoch: rinex_epoch_time_to_c(epoch.epoch),
        observation_count: epoch.inputs.observations.len(),
    }
}

fn rinex_spp_epoch_solution_to_c(epoch: &RinexSppEpochSolution) -> SidereonRinexSppEpoch {
    let observation_count = epoch
        .solution
        .as_ref()
        .map(|solution| solution.metadata.used_count + solution.rejected_sats.len())
        .unwrap_or(0);
    SidereonRinexSppEpoch {
        epoch_index: epoch.epoch_index,
        epoch: rinex_epoch_time_to_c(epoch.epoch),
        observation_count,
    }
}

fn rinex_spp_input_at<'a>(
    fn_name: &str,
    inputs: &'a [RinexSppEpochInputs],
    index: usize,
) -> Result<&'a RinexSppEpochInputs, SidereonStatus> {
    inputs.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: index {index} out of range ({} epochs)",
            inputs.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

fn rinex_spp_solution_at<'a>(
    fn_name: &str,
    solutions: &'a [RinexSppEpochSolution],
    index: usize,
) -> Result<&'a RinexSppEpochSolution, SidereonStatus> {
    solutions.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: index {index} out of range ({} results)",
            solutions.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

fn map_rinex_spp_error(fn_name: &str, err: RinexSppError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::RinexSpp,
        fn_name,
        crate::engine_error::rinex_spp_error_value(&err),
    );
    match &err {
        RinexSppError::Observation(CoreError::Ut1OutsideCoverage(_)) => {
            SidereonStatus::Ut1OutsideCoverage
        }
        RinexSppError::Observation(_) | RinexSppError::MissingApproxPosition => {
            SidereonStatus::InvalidArgument
        }
        // `RinexSppError` is non-exhaustive: a failure a later engine adds
        // keeps the engine's text, variant name included, in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::InvalidArgument
        }
    }
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
        sidereon_core::positioning::RejectionReason::IonosphereCarrierUnresolved => {
            SidereonSppRejectionReason::IonosphereCarrierUnresolved
        }
        sidereon_core::positioning::RejectionReason::SsrCorrectionExceedsLimit(_) => {
            SidereonSppRejectionReason::SsrCorrectionExceedsLimit
        }
    }
}

pub(super) fn rejected_sats_v2_to_c(
    values: &[sidereon_core::positioning::RejectedSat],
) -> Vec<SidereonSppRejectedSatV2> {
    values
        .iter()
        .map(|value| {
            let (reason, size) = match value.reason {
                sidereon_core::positioning::RejectionReason::SsrCorrectionExceedsLimit(size) => (
                    SidereonSppRejectionReason::SsrCorrectionExceedsLimit,
                    Some(size),
                ),
                other => (rejection_reason_to_c(other), None),
            };
            SidereonSppRejectedSatV2 {
                sat_id: satellite_token(value.satellite_id),
                reason,
                has_size: size.is_some(),
                orbit_m: size.map_or(0.0, |size| size.orbit_m),
                clock_m: size.map_or(0.0, |size| size.clock_m),
            }
        })
        .collect()
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
            let velocity_error_detail = solution.velocity_error.as_ref().map(|error| {
                spp_batch_row_error_from_facade(fn_name, sidereon::Error::Velocity(*error))
            });
            write_boxed_handle(
                out_solution,
                SidereonSppDopplerSolution {
                    receiver: solution.receiver,
                    velocity: solution.velocity,
                    velocity_error: solution.velocity_error,
                    velocity_error_detail,
                },
            );
            SidereonStatus::Ok
        }
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            crate::engine_error::RecordEngineError::record_engine_error(fn_name, &err);
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
        Status::SelectionSettled => SidereonSppSolveStatus::SelectionSettled,
        Status::OuterBudgetExhausted => SidereonSppSolveStatus::OuterBudgetExhausted,
        Status::OuterOscillation => SidereonSppSolveStatus::OuterOscillation,
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
        ut1_degraded: SidereonUt1Degradation::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    #[test]
    fn rinex_spp_epoch_error_retains_policy_family_and_typed_payload() {
        let error = sidereon_core::positioning::SolvePolicyError::NoCoarseSolution;
        let retained = rinex_spp_row_error(&error);
        assert_eq!(retained.family, SidereonEngineErrorFamily::SppPolicy);
        let value: serde_json::Value =
            serde_json::from_str(&retained.payload).expect("valid retained row JSON");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["family"], "spp_policy");
        assert_eq!(
            value["operation"],
            "sidereon_rinex_spp_solution_error_payload"
        );
        assert_eq!(value["error"]["kind"], "no_coarse_solution");

        let nested = sidereon_core::positioning::SolvePolicyError::Solve(
            sidereon_core::positioning::SppError::TooFewSatellites {
                used: 2,
                required: 4,
            },
        );
        let retained = rinex_spp_row_error(&nested);
        assert_eq!(retained.family, SidereonEngineErrorFamily::Spp);
        let value: serde_json::Value =
            serde_json::from_str(&retained.payload).expect("valid nested row JSON");
        assert_eq!(value["family"], "spp");
        assert_eq!(value["error"]["kind"], "too_few_satellites");
        assert_eq!(value["error"]["fields"]["used"], 2);
        assert_eq!(value["error"]["fields"]["required"], 4);
    }

    #[test]
    fn rinex_spp_epoch_error_accessors_report_and_copy_row_owned_payload() {
        let handle = SidereonRinexSppSolutions {
            inner: vec![RinexSppEpochSolution {
                epoch_index: 0,
                epoch: sidereon_core::rinex::observations::ObsEpochTime {
                    year: 2024,
                    month: 1,
                    day: 1,
                    hour: 0,
                    minute: 0,
                    second: 0.0,
                },
                solution: Err(sidereon_core::positioning::SolvePolicyError::NoCoarseSolution),
            }],
        };
        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        unsafe {
            assert_eq!(
                sidereon_rinex_spp_solution_error_info(&handle, 0, &mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::SppPolicy);
            assert!(info.payload_len > 0);
            let mut written = usize::MAX;
            let mut required = 0;
            assert_eq!(
                sidereon_rinex_spp_solution_error_payload(
                    &handle,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, info.payload_len);
            let mut payload = vec![0u8; required];
            assert_eq!(
                sidereon_rinex_spp_solution_error_payload(
                    &handle,
                    0,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, payload.len());
            let value: serde_json::Value =
                serde_json::from_slice(&payload).expect("valid row payload");
            assert_eq!(value["error"]["kind"], "no_coarse_solution");
        }
    }

    const T_RX_J2000_S: f64 = 646_272_000.0;
    const T_RX_SOD_S: f64 = 43_200.0;
    const DAY_OF_YEAR: f64 = 176.5;
    const RECEIVER: [f64; 3] = [4_500_000.0, 500_000.0, 4_500_000.0];
    const RECEIVER_VELOCITY: [f64; 3] = [12.0, -7.0, 3.0];
    const CLOCK_BIAS_M: f64 = 8.0;
    const CLOCK_DRIFT_S_S: f64 = 1.0e-9;

    #[test]
    fn rejected_satellite_v2_retains_size_payload_and_variable_buffer_values() {
        let (converted, legacy_reason) = {
            let rejected = [sidereon_core::positioning::RejectedSat {
                satellite_id: GnssSatelliteId {
                    system: GnssSystem::Gps,
                    prn: 7,
                },
                reason: sidereon_core::positioning::RejectionReason::SsrCorrectionExceedsLimit(
                    sidereon_core::ssr::SsrCorrectionSize {
                        orbit_m: 130.25,
                        clock_m: -55.5,
                    },
                ),
            }];
            (
                rejected_sats_v2_to_c(&rejected),
                rejection_reason_to_c(rejected[0].reason),
            )
        };
        assert_eq!(converted.len(), 1);
        assert_eq!(
            legacy_reason,
            SidereonSppRejectionReason::SsrCorrectionExceedsLimit
        );
        assert_eq!(
            converted[0].reason,
            SidereonSppRejectionReason::SsrCorrectionExceedsLimit
        );
        assert!(converted[0].has_size);
        assert_eq!(converted[0].orbit_m, 130.25);
        assert_eq!(converted[0].clock_m, -55.5);

        let mut output = [SidereonSppRejectedSatV2 {
            sat_id: satellite_token(GnssSatelliteId {
                system: GnssSystem::Gps,
                prn: 1,
            }),
            reason: SidereonSppRejectionReason::NoEphemeris,
            has_size: false,
            orbit_m: 9.0,
            clock_m: 9.0,
        }];
        let mut written = 0;
        let mut required = 0;
        unsafe {
            copy_prefix_to_c(
                "test",
                "out",
                &converted,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        }
        .unwrap();
        assert_eq!((written, required), (0, 1));
        unsafe {
            copy_prefix_to_c(
                "test",
                "out",
                &converted,
                output.as_mut_ptr(),
                output.len(),
                &mut written,
                &mut required,
            )
        }
        .unwrap();
        assert_eq!((written, required), (1, 1));
        assert_eq!(output[0].sat_id.bytes, converted[0].sat_id.bytes);
        assert_eq!(output[0].orbit_m.to_bits(), 130.25f64.to_bits());
        assert_eq!(output[0].clock_m.to_bits(), (-55.5f64).to_bits());
    }

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    fn visible_gps(sp3: &Sp3) -> Vec<GnssSatelliteId> {
        let mut planning = PredictOptions::default();
        planning.light_time = false;
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
                pseudorange_code: sidereon_core::positioning::PseudorangeCode::SingleFrequency,
                troposphere_model: sidereon_core::positioning::TroposphereModel::Rtklib,
                qzss_clock: sidereon_core::positioning::QzssClock::Gps,
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
            pseudorange_code: SidereonPseudorangeCode::SingleFrequency as u32,
        };
        (observations, c_doppler, c_inputs)
    }

    fn assert_same_bits(got: f64, want: f64) {
        assert_eq!(got.to_bits(), want.to_bits(), "got {got:e}, want {want:e}");
    }

    fn assert_engine_error_snapshot_eq(
        expected: &(crate::engine_error::SidereonEngineErrorInfo, String),
    ) {
        let actual = crate::engine_error::snapshot_engine_error_for_test()
            .expect("engine error snapshot remains populated");
        assert_eq!(actual.0.family, expected.0.family);
        assert_eq!(actual.0.payload_len, expected.0.payload_len);
        assert_eq!(actual.1, expected.1);
    }

    #[test]
    fn rinex_spp_mapper_retains_both_typed_error_variants() {
        crate::engine_error::clear_engine_error();
        assert_eq!(
            map_rinex_spp_error(
                "sidereon_spp_inputs_from_rinex_obs",
                RinexSppError::MissingApproxPosition,
            ),
            SidereonStatus::InvalidArgument
        );
        let (info, payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("missing approximate position is typed");
        assert_eq!(info.family, SidereonEngineErrorFamily::RinexSpp);
        let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
        assert_eq!(value["operation"], "sidereon_spp_inputs_from_rinex_obs");
        assert_eq!(value["error"]["kind"], "missing_approx_position");
        assert_eq!(value["error"]["fields"], serde_json::json!({}));

        crate::engine_error::clear_engine_error();
        assert_eq!(
            map_rinex_spp_error(
                "sidereon_solve_spp_from_rinex_obs",
                RinexSppError::Observation(sidereon_core::Error::InvalidInput(
                    "bad epoch value".into(),
                )),
            ),
            SidereonStatus::InvalidArgument
        );
        let (info, payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("observation cause is typed");
        assert_eq!(info.family, SidereonEngineErrorFamily::RinexSpp);
        let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
        assert_eq!(value["operation"], "sidereon_solve_spp_from_rinex_obs");
        assert_eq!(value["error"]["kind"], "observation");
        assert_eq!(value["error"]["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(
            value["error"]["fields"]["cause"]["fields"]["message"],
            "bad epoch value"
        );

        let operation = "sidereon_solve_spp_from_rinex_obs";
        assert_eq!(
            map_rinex_spp_error(
                operation,
                RinexSppError::Observation(sidereon_core::Error::Ut1OutsideCoverage(
                    sidereon_core::astro::time::DegradeReason::AfterCoverage,
                )),
            ),
            SidereonStatus::Ut1OutsideCoverage
        );
        let (info, payload) =
            crate::engine_error::snapshot_engine_error_for_test().expect("UT1 typed detail");
        assert_eq!(info.family, SidereonEngineErrorFamily::RinexSpp);
        let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
        assert_eq!(value["error"]["kind"], "observation");
        assert_eq!(
            value["error"]["fields"]["cause"]["kind"],
            "ut1_outside_coverage"
        );
        assert_eq!(
            value["error"]["fields"]["cause"]["fields"]["reason"],
            "after_coverage"
        );
        let mut legacy = vec![0 as std::os::raw::c_char; 256];
        let needed =
            unsafe { crate::sidereon_last_error_message(legacy.as_mut_ptr(), legacy.len()) };
        assert!(needed > 0);
        let legacy = unsafe { CStr::from_ptr(legacy.as_ptr()) }
            .to_str()
            .expect("legacy UTF-8");
        assert_eq!(
            legacy,
            "sidereon_solve_spp_from_rinex_obs: RINEX SPP observation assembly failed: UT1 outside the table: instant follows the UT1 table coverage"
        );
        crate::engine_error::clear_engine_error();
    }

    #[test]
    fn rinex_spp_missing_position_records_real_producer_error() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx");
        let source = fs::read_to_string(fixture).expect("read RINEX OBS fixture");
        let mut removed = 0;
        let mutated = source
            .lines()
            .filter(|line| {
                let is_approx = line.contains("APPROX POSITION XYZ");
                removed += usize::from(is_approx);
                !is_approx
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(removed, 1, "mutate only the fixture's approximate position");
        let obs = SidereonRinexObs {
            inner: RinexObs::parse(&mutated).expect("mutated observation file remains parseable"),
        };
        assert!(obs.inner.header().approx_position_m.is_none());

        let nav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx");
        let nav = fs::read(nav_path).expect("read broadcast NAV fixture");
        let mut broadcast = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_broadcast_ephemeris_parse_nav(nav.as_ptr(), nav.len(), &mut broadcast)
            },
            SidereonStatus::Ok
        );
        assert!(!broadcast.is_null());

        let mut assembled = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut assembled)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(assembled.is_null());
        let (info, payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("public producer retained its typed failure");
        assert_eq!(info.family, SidereonEngineErrorFamily::RinexSpp);
        let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
        assert_eq!(value["operation"], "sidereon_spp_inputs_from_rinex_obs");
        assert_eq!(value["error"]["kind"], "missing_approx_position");
        assert_eq!(value["error"]["fields"], serde_json::json!({}));

        let mut assembled_with_models = ptr::null_mut();
        crate::engine_error::clear_engine_error();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs_with_models(
                    &obs,
                    broadcast,
                    ptr::null(),
                    ptr::null(),
                    &mut assembled_with_models,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(assembled_with_models.is_null());
        let (_, model_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("model route records missing position");
        let model_json: serde_json::Value = serde_json::from_str(&model_payload).expect("JSON");
        assert_eq!(
            model_json["operation"],
            "sidereon_spp_inputs_from_rinex_obs_with_models"
        );
        assert_eq!(model_json["error"]["kind"], "missing_approx_position");

        let mut solutions = ptr::null_mut();
        crate::engine_error::clear_engine_error();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_from_rinex_obs(
                    broadcast,
                    &obs,
                    ptr::null(),
                    true,
                    ptr::null(),
                    &mut solutions,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(solutions.is_null());
        let (_, solve_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("solve route records missing position");
        let solve_json: serde_json::Value = serde_json::from_str(&solve_payload).expect("JSON");
        assert_eq!(solve_json["operation"], "sidereon_solve_spp_from_rinex_obs");
        assert_eq!(solve_json["error"]["kind"], "missing_approx_position");

        let mut solutions_with_models = ptr::null_mut();
        crate::engine_error::clear_engine_error();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_from_rinex_obs_with_models(
                    broadcast,
                    &obs,
                    ptr::null(),
                    ptr::null(),
                    true,
                    ptr::null(),
                    &mut solutions_with_models,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(solutions_with_models.is_null());
        let (_, solve_models_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("model solve route records missing position");
        let solve_models_json: serde_json::Value =
            serde_json::from_str(&solve_models_payload).expect("JSON");
        assert_eq!(
            solve_models_json["operation"],
            "sidereon_solve_spp_from_rinex_obs_with_models"
        );
        assert_eq!(
            solve_models_json["error"]["kind"],
            "missing_approx_position"
        );

        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(
                    ptr::null(),
                    broadcast,
                    ptr::null(),
                    &mut assembled,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(assembled.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());

        crate::engine_error::clear_engine_error();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut assembled)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_some());
        let valid_obs = SidereonRinexObs {
            inner: RinexObs::parse(&source).expect("original observation fixture parses"),
        };
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(
                    &valid_obs,
                    broadcast,
                    ptr::null(),
                    &mut assembled,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!assembled.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());

        crate::engine_error::clear_engine_error();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(
                    &obs,
                    broadcast,
                    ptr::null(),
                    &mut assembled_with_models,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let retained = crate::engine_error::snapshot_engine_error_for_test()
            .expect("live input reader retention seed");
        let mut count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_rinex_spp_inputs_count(assembled, &mut count) },
            SidereonStatus::Ok
        );
        assert!(count > 0);
        let mut epoch = MaybeUninit::<SidereonRinexSppEpoch>::uninit();
        assert_eq!(
            unsafe { sidereon_rinex_spp_inputs_epoch(assembled, 0, epoch.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let epoch = unsafe { epoch.assume_init() };
        assert!(epoch.observation_count > 0);
        let mut row_inputs = MaybeUninit::<SidereonSppInputsV2>::uninit();
        assert_eq!(
            unsafe {
                sidereon_rinex_spp_inputs_epoch_inputs(assembled, 0, row_inputs.as_mut_ptr())
            },
            SidereonStatus::Ok
        );
        let row_inputs = unsafe { row_inputs.assume_init() };
        assert!(row_inputs.base.observation_count > 0);
        assert_engine_error_snapshot_eq(&retained);
        unsafe { sidereon_rinex_spp_inputs_free(assembled) };
        assert_engine_error_snapshot_eq(&retained);

        let mut valid_assembled_with_models = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs_with_models(
                    &valid_obs,
                    broadcast,
                    ptr::null(),
                    ptr::null(),
                    &mut valid_assembled_with_models,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!valid_assembled_with_models.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_rinex_spp_inputs_free(valid_assembled_with_models) };

        let mut refusal_seed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut refusal_seed)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_some());
        let mut valid_solutions = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_from_rinex_obs(
                    broadcast,
                    &valid_obs,
                    ptr::null(),
                    true,
                    ptr::null(),
                    &mut valid_solutions,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!valid_solutions.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_rinex_spp_solutions_free(valid_solutions) };

        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut refusal_seed)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_some());
        let mut valid_solutions_with_models = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_from_rinex_obs_with_models(
                    broadcast,
                    &valid_obs,
                    ptr::null(),
                    ptr::null(),
                    true,
                    ptr::null(),
                    &mut valid_solutions_with_models,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!valid_solutions_with_models.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_rinex_spp_solutions_free(valid_solutions_with_models) };
        unsafe { sidereon_broadcast_ephemeris_free(broadcast) };
    }

    #[test]
    fn rinex_spp_real_event_timeline_error_retains_nested_core_cause() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx");
        let source = fs::read_to_string(fixture).expect("read RINEX OBS fixture");
        let approx = source
            .lines()
            .find(|line| line.contains("APPROX POSITION XYZ"))
            .expect("fixture approximate position")
            .to_string();
        let last_epoch = source
            .lines()
            .rev()
            .find(|line| line.starts_with('>'))
            .expect("fixture epoch");
        let mut epoch_fields = last_epoch
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(epoch_fields.first().map(String::as_str), Some(">"));
        assert!(epoch_fields.len() >= 9);
        epoch_fields[7] = "3".to_string();
        epoch_fields[8] = "1".to_string();
        let event_line = epoch_fields.join(" ");
        let event_input = format!("{}\n{}\n{}", source.trim_end(), event_line, approx);
        assert!(event_input.contains(&format!("\n{event_line}\n{approx}")));

        // Parsing accepts the valid fixture-derived event. Replacing its owned
        // header record afterward mirrors the core's post-parse timeline guard.
        let mut parsed = RinexObs::parse(&event_input).expect("fixture plus valid event parses");
        let last = parsed.epochs.last_mut().expect("event epoch");
        assert_eq!(last.flag, 3);
        assert_eq!(last.special_records, vec![approx.clone()]);
        let mut malformed = approx.clone();
        malformed.replace_range(..60, &format!("{:<60}", "not a position"));
        assert_ne!(malformed, approx);
        last.special_records[0] = malformed.clone();
        assert!(parsed.header_timeline().is_err());
        let obs = SidereonRinexObs { inner: parsed };

        let nav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx");
        let nav = fs::read(nav_path).expect("read broadcast NAV fixture");
        let mut broadcast = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_broadcast_ephemeris_parse_nav(nav.as_ptr(), nav.len(), &mut broadcast)
            },
            SidereonStatus::Ok
        );
        let mut assembled = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut assembled)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(assembled.is_null());
        let (info, payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("event timeline failure is typed");
        assert_eq!(info.family, SidereonEngineErrorFamily::RinexSpp);
        let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON");
        assert_eq!(value["operation"], "sidereon_spp_inputs_from_rinex_obs");
        assert_eq!(value["error"]["kind"], "observation");
        assert_eq!(value["error"]["fields"]["cause"]["kind"], "parse");
        let message = value["error"]["fields"]["cause"]["fields"]["message"]
            .as_str()
            .expect("parse cause message");
        assert!(message.contains("event records"), "{message}");
        assert!(message.contains("not a position"), "{message}");

        let mut solutions = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_from_rinex_obs(
                    broadcast,
                    &obs,
                    ptr::null(),
                    true,
                    ptr::null(),
                    &mut solutions,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(solutions.is_null());
        let (solve_info, solve_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("event timeline solve failure is typed");
        assert_eq!(solve_info.family, SidereonEngineErrorFamily::RinexSpp);
        assert_eq!(solve_info.payload_len, solve_payload.len());
        let solve_value: serde_json::Value = serde_json::from_str(&solve_payload).expect("JSON");
        assert_eq!(
            solve_value["operation"],
            "sidereon_solve_spp_from_rinex_obs"
        );
        assert_eq!(solve_value["error"]["kind"], "observation");
        assert_eq!(solve_value["error"]["fields"]["cause"]["kind"], "parse");
        let solve_message = solve_value["error"]["fields"]["cause"]["fields"]["message"]
            .as_str()
            .expect("solve parse cause message");
        assert_eq!(solve_message, message);
        assert!(solve_message.contains("event records"), "{solve_message}");
        assert!(solve_message.contains("not a position"), "{solve_message}");
        unsafe { sidereon_broadcast_ephemeris_free(broadcast) };
        crate::engine_error::clear_engine_error();
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
        assert_eq!(has_velocity, expected.velocity.is_some());

        let mut velocity_error = SidereonSppDopplerVelocityErrorKind::InvalidInput;
        let status = unsafe {
            sidereon_spp_doppler_solution_velocity_error_kind(solution, &mut velocity_error)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(velocity_error, SidereonSppDopplerVelocityErrorKind::None);
        let mut error_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Spp,
            payload_len: 99,
        };
        assert_eq!(
            unsafe { sidereon_spp_doppler_solution_velocity_error_info(solution, &mut error_info) },
            SidereonStatus::Ok
        );
        assert_eq!(error_info.family, SidereonEngineErrorFamily::None);
        assert_eq!(error_info.payload_len, 0);
        let mut error_written = 99;
        let mut error_required = 99;
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    ptr::null_mut(),
                    0,
                    &mut error_written,
                    &mut error_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!((error_written, error_required), (0, 0));

        let mut receiver = ptr::null_mut();
        let status = unsafe { sidereon_spp_doppler_solution_receiver(solution, &mut receiver) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!receiver.is_null());

        let mut validation = MaybeUninit::<SidereonSolutionValidationOptions>::uninit();
        assert_eq!(
            unsafe { sidereon_solution_validation_options_init(validation.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let mut validation = unsafe { validation.assume_init() };
        validation.min_plausible_radius_m = 1.0e99;
        validation.max_plausible_radius_m = 1.0e100;
        assert_eq!(
            unsafe { sidereon_validate_receiver_solution(receiver, &validation) },
            SidereonStatus::InvalidArgument
        );
        let (validation_info, validation_payload) =
            crate::engine_error::snapshot_engine_error_for_test()
                .expect("validation detail retained");
        assert_eq!(
            validation_info.family,
            SidereonEngineErrorFamily::SolutionValidation
        );
        let validation_json: serde_json::Value =
            serde_json::from_str(&validation_payload).expect("valid JSON");
        assert_eq!(
            validation_json["operation"],
            "sidereon_validate_receiver_solution"
        );
        let radius = expected
            .receiver
            .position
            .as_array()
            .iter()
            .map(|component| component * component)
            .sum::<f64>()
            .sqrt();
        assert_eq!(validation_json["error"]["kind"], "implausible_position");
        assert_eq!(
            validation_json["error"]["fields"]["radius_m"]["decimal"],
            radius.to_string()
        );
        assert_eq!(
            validation_json["error"]["fields"]["radius_m"]["bits_hex"],
            format!("{:016x}", radius.to_bits())
        );
        validation.min_plausible_radius_m = 1.0;
        validation.max_plausible_radius_m = 1.0e8;
        assert_eq!(
            unsafe { sidereon_validate_receiver_solution(receiver, &validation) },
            SidereonStatus::Ok
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());

        let mut position = [0.0; 3];
        let status = unsafe { sidereon_spp_solution_position(receiver, position.as_mut_ptr(), 3) };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in position.iter().zip(expected.receiver.position.as_array()) {
            assert_same_bits(*got, want);
        }

        let mut drift_present = false;
        let mut drift = 0.0;
        let status = unsafe {
            sidereon_spp_solution_rx_clock_drift_s_s(receiver, &mut drift_present, &mut drift)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(drift_present);
        assert_same_bits(
            drift,
            expected.receiver.rx_clock_drift_s_s.expect("clock drift"),
        );

        let mut covariance = [0.0; 9];
        let status = unsafe {
            sidereon_spp_solution_position_covariance_ecef_m2(receiver, covariance.as_mut_ptr(), 9)
        };
        assert_eq!(status, SidereonStatus::Ok);
        for (got, want) in covariance.iter().zip(flatten_spp_mat3(
            expected.receiver.position_covariance.ecef_m2,
        )) {
            assert_same_bits(*got, want);
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
            assert_same_bits(*got, want);
        }

        unsafe {
            sidereon_velocity_solution_free(velocity);
            sidereon_spp_solution_free(receiver);
            sidereon_spp_doppler_solution_free(solution);
        }
    }

    #[test]
    fn spp_doppler_retains_typed_velocity_error_payload_on_owned_handle() {
        let sp3 = fixture_sp3();
        let sats = visible_gps(&sp3);
        assert!(sats.len() >= 4);
        let (inputs, doppler_observations) = core_inputs(&sp3, &sats);
        let expected =
            core_solve_with_doppler_velocity(&sp3, &inputs, &doppler_observations[..1], true)
                .expect("position solve keeps underdetermined Doppler as owned velocity error");
        assert!(expected.velocity.is_none());
        assert!(expected.velocity_error.is_some());

        let tokens = sats
            .iter()
            .map(|sat| CString::new(sat.to_string()).expect("sat token"))
            .collect::<Vec<_>>();
        let (_observations, c_doppler, c_inputs) =
            c_inputs(&inputs, &doppler_observations, &tokens);
        let sp3_handle = SidereonSp3 { inner: sp3 };
        let mut solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &c_inputs,
                    c_doppler.as_ptr(),
                    1,
                    &mut solution,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!solution.is_null());
        // `c_inputs.base.observations` borrows this vector for the C calls below.

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_spp_doppler_solution_velocity_error_info(solution, &mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Facade);
        assert!(info.payload_len > 0);
        let mut velocity_kind = SidereonSppDopplerVelocityErrorKind::None;
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_kind(solution, &mut velocity_kind)
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            velocity_kind,
            spp_doppler_velocity_error_to_c(
                expected.velocity_error.as_ref().expect("expected error")
            )
        );

        let mut written = usize::MAX;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert_eq!(required, info.payload_len);
        let mut short = vec![0xa5; required.saturating_sub(1)];
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    short.as_mut_ptr(),
                    short.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(written, 0);
        assert!(short.iter().all(|byte| *byte == 0xa5));

        let mut payload = vec![0; required];
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, payload.len());
        assert_eq!(info.payload_len, written);
        let original = payload.clone();
        let json: serde_json::Value = serde_json::from_slice(&payload).expect("payload JSON");
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["family"], "facade");
        assert_eq!(
            json["operation"],
            "sidereon_solve_spp_with_doppler_velocity"
        );
        assert_eq!(json["error"]["kind"], "velocity");
        let expected_error = expected
            .velocity_error
            .as_ref()
            .expect("core velocity error");
        assert_eq!(
            json["error"]["fields"]["cause"],
            crate::engine_error::velocity_error_value(expected_error)
        );
        let (used, required_satellites) = match expected_error {
            VelocityError::TooFewSatellites { used, required } => (*used, *required),
            other => panic!("expected selected-row velocity refusal, got {other:?}"),
        };
        assert!(used < required_satellites);
        assert_eq!(
            json["error"]["fields"]["cause"]["kind"],
            "too_few_satellites"
        );
        assert_eq!(json["error"]["fields"]["cause"]["fields"]["used"], used);
        assert_eq!(
            json["error"]["fields"]["cause"]["fields"]["required"],
            required_satellites
        );

        let mut succeeding_solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &c_inputs,
                    c_doppler.as_ptr(),
                    c_doppler.len(),
                    &mut succeeding_solution,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!succeeding_solution.is_null());
        unsafe { sidereon_spp_doppler_solution_free(succeeding_solution) };

        // The owned handle remains the source of its full JSON bytes after a
        // later successful producer has cleared/replaced generic TLS state.
        let mut reread_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_info(solution, &mut reread_info)
            },
            SidereonStatus::Ok
        );
        assert_eq!(reread_info.family, info.family);
        assert_eq!(reread_info.payload_len, original.len());
        let mut reread = vec![0; reread_info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    reread.as_mut_ptr(),
                    reread.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(reread, original);

        // Seed independent TLS with a genuine public core SPP refusal while the
        // owned velocity-error handle stays live.
        let mut empty_inputs = inputs.clone();
        empty_inputs.observations.clear();
        let expected_spp_error =
            core_solve_with_doppler_velocity(&sp3_handle.inner, &empty_inputs, &[], true)
                .expect_err("empty receiver observation set must refuse");
        let mut invalid_c_inputs = c_inputs;
        invalid_c_inputs.base.observations = ptr::null();
        invalid_c_inputs.base.observation_count = 0;
        let mut failed_solution = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &invalid_c_inputs,
                    ptr::null(),
                    0,
                    &mut failed_solution,
                )
            },
            SidereonStatus::Solve
        );
        assert!(failed_solution.is_null());
        let retained_tls = crate::engine_error::snapshot_engine_error_for_test()
            .expect("genuine SPP producer refusal seeds TLS");
        assert_eq!(retained_tls.0.family, SidereonEngineErrorFamily::Spp);
        assert_eq!(retained_tls.0.payload_len, retained_tls.1.len());
        let tls_json: serde_json::Value = serde_json::from_str(&retained_tls.1).expect("JSON");
        assert_eq!(
            tls_json["operation"],
            "sidereon_solve_spp_with_doppler_velocity"
        );
        assert_eq!(tls_json["family"], "spp");
        assert_eq!(
            tls_json["error"],
            crate::engine_error::spp_error_value(&expected_spp_error)
        );

        let mut retained_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_info(solution, &mut retained_info)
            },
            SidereonStatus::Ok
        );
        assert_eq!(retained_info.family, info.family);
        assert_eq!(retained_info.payload_len, info.payload_len);
        assert_engine_error_snapshot_eq(&retained_tls);
        let mut retained_payload = vec![0; retained_info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    retained_payload.as_mut_ptr(),
                    retained_payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(retained_payload, original);
        assert_engine_error_snapshot_eq(&retained_tls);

        let mut null_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Spp,
            payload_len: usize::MAX,
        };
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_info(ptr::null(), &mut null_info)
            },
            SidereonStatus::NullPointer
        );
        assert_eq!(null_info.family, SidereonEngineErrorFamily::None);
        assert_eq!(null_info.payload_len, 0);
        assert_engine_error_snapshot_eq(&retained_tls);
        written = usize::MAX;
        required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    ptr::null(),
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::NullPointer
        );
        assert_eq!((written, required), (0, 0));
        assert_engine_error_snapshot_eq(&retained_tls);

        // Reader failure on a short output buffer is non-destructive and keeps
        // the complete independent producer error in generic TLS.
        let mut short_for_retention = vec![0x5a; original.len() - 1];
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_payload(
                    solution,
                    short_for_retention.as_mut_ptr(),
                    short_for_retention.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(written, 0);
        assert_eq!(required, original.len());
        assert!(short_for_retention.iter().all(|byte| *byte == 0x5a));
        assert_engine_error_snapshot_eq(&retained_tls);
        unsafe { sidereon_spp_doppler_solution_free(solution) };
        assert_engine_error_snapshot_eq(&retained_tls);
        assert_eq!(payload, original);
    }

    #[test]
    fn both_combined_producers_record_real_spp_refusals_and_reset_on_success() {
        let sp3 = fixture_sp3();
        let mut empty_v2 = MaybeUninit::<SidereonSppInputsV2>::uninit();
        assert_eq!(
            unsafe { sidereon_spp_inputs_v2_init(empty_v2.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let mut empty_v2 = unsafe { empty_v2.assume_init() };
        empty_v2.base.observations = ptr::null();
        empty_v2.base.observation_count = 0;
        let empty_inputs = unsafe {
            build_spp_solve_inputs_with_models("test_empty_combined_inputs", &empty_v2, ptr::null())
        }
        .expect("initialized empty inputs parse");
        let expected_sp3 = core_solve_with_doppler_velocity(&sp3, &empty_inputs, &[], true)
            .expect_err("real SP3 producer has no usable observations");

        let nav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx");
        let nav = fs::read(nav_path).expect("read broadcast NAV fixture");
        let mut broadcast = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_broadcast_ephemeris_parse_nav(nav.as_ptr(), nav.len(), &mut broadcast)
            },
            SidereonStatus::Ok
        );
        let expected_broadcast = core_solve_with_doppler_velocity(
            unsafe { &(*broadcast).inner },
            &empty_inputs,
            &[],
            true,
        )
        .expect_err("real broadcast producer has no usable observations");

        let sp3_handle = SidereonSp3 { inner: sp3 };
        let mut failed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut failed,
                )
            },
            SidereonStatus::Solve
        );
        assert!(failed.is_null());
        let (sp3_info, sp3_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("SP3 combined producer records SppError");
        assert_eq!(sp3_info.family, SidereonEngineErrorFamily::Spp);
        let sp3_json: serde_json::Value = serde_json::from_str(&sp3_payload).expect("JSON");
        assert_eq!(
            sp3_json["operation"],
            "sidereon_solve_spp_with_doppler_velocity"
        );
        assert_eq!(
            sp3_json["error"],
            crate::engine_error::spp_error_value(&expected_sp3)
        );

        let mut failed_broadcast = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_broadcast_with_doppler_velocity(
                    broadcast,
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut failed_broadcast,
                )
            },
            SidereonStatus::Solve
        );
        assert!(failed_broadcast.is_null());
        let (broadcast_info, broadcast_payload) =
            crate::engine_error::snapshot_engine_error_for_test()
                .expect("broadcast combined producer records SppError");
        assert_eq!(broadcast_info.family, SidereonEngineErrorFamily::Spp);
        let broadcast_json: serde_json::Value =
            serde_json::from_str(&broadcast_payload).expect("JSON");
        assert_eq!(
            broadcast_json["operation"],
            "sidereon_solve_broadcast_with_doppler_velocity"
        );
        assert_eq!(
            broadcast_json["error"],
            crate::engine_error::spp_error_value(&expected_broadcast)
        );

        let mut early_output = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    ptr::null(),
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut early_output,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        let mut seed_broadcast = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_broadcast_with_doppler_velocity(
                    broadcast,
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut seed_broadcast,
                )
            },
            SidereonStatus::Solve
        );
        assert!(seed_broadcast.is_null());
        let (seed_info, seed_payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("broadcast refusal populates TLS before early-null reset");
        assert_eq!(seed_info.family, SidereonEngineErrorFamily::Spp);
        assert_eq!(seed_info.payload_len, seed_payload.len());
        assert_eq!(
            unsafe {
                sidereon_solve_broadcast_with_doppler_velocity(
                    ptr::null(),
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut early_output,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());

        let sats = visible_gps(&sp3_handle.inner);
        assert!(sats.len() >= 4);
        let (inputs, doppler_observations) = core_inputs(&sp3_handle.inner, &sats);
        let tokens = sats
            .iter()
            .map(|sat| CString::new(sat.to_string()).expect("sat token"))
            .collect::<Vec<_>>();
        let (observations, c_doppler, c_inputs) = c_inputs(&inputs, &doppler_observations, &tokens);
        let mut reset_seed = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut reset_seed,
                )
            },
            SidereonStatus::Solve
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_some());
        let mut sp3_success = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_spp_with_doppler_velocity(
                    &sp3_handle,
                    &c_inputs,
                    c_doppler.as_ptr(),
                    c_doppler.len(),
                    &mut sp3_success,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!sp3_success.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        unsafe { sidereon_spp_doppler_solution_free(sp3_success) };
        drop(observations);

        let obs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx");
        let obs_text = fs::read_to_string(obs_path).expect("read matching OBS fixture");
        let obs = SidereonRinexObs {
            inner: RinexObs::parse(&obs_text).expect("parse matching OBS fixture"),
        };
        let mut assembled = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_spp_inputs_from_rinex_obs(&obs, broadcast, ptr::null(), &mut assembled)
            },
            SidereonStatus::Ok
        );
        let mut row = MaybeUninit::<SidereonSppInputsV2>::uninit();
        assert_eq!(
            unsafe { sidereon_rinex_spp_inputs_epoch_inputs(assembled, 0, row.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let row = unsafe { row.assume_init() };
        assert!(row.base.observation_count >= 4);
        let first = unsafe { &*row.base.observations };
        let doppler = [SidereonSppDopplerObservation {
            sat_id: first.sat_id,
            doppler_hz: 0.0,
            carrier_hz: sidereon_core::constants::F_L1_HZ,
            sat_clock_drift_s_s: 0.0,
        }];
        assert_eq!(
            unsafe {
                sidereon_solve_broadcast_with_doppler_velocity(
                    broadcast,
                    &empty_v2,
                    ptr::null(),
                    0,
                    &mut failed_broadcast,
                )
            },
            SidereonStatus::Solve
        );
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_some());
        let mut broadcast_success = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_solve_broadcast_with_doppler_velocity(
                    broadcast,
                    &row,
                    doppler.as_ptr(),
                    doppler.len(),
                    &mut broadcast_success,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!broadcast_success.is_null());
        assert!(crate::engine_error::snapshot_engine_error_for_test().is_none());
        let mut retained_velocity_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe {
                sidereon_spp_doppler_solution_velocity_error_info(
                    broadcast_success,
                    &mut retained_velocity_info,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            retained_velocity_info.family,
            SidereonEngineErrorFamily::Facade
        );
        assert!(retained_velocity_info.payload_len > 0);
        unsafe {
            sidereon_spp_doppler_solution_free(broadcast_success);
            sidereon_rinex_spp_inputs_free(assembled);
            sidereon_broadcast_ephemeris_free(broadcast);
        }
    }

    #[test]
    fn spp_batch_owned_error_info_and_payload_accessors_and_retention() {
        use crate::engine_error::{
            sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
            SidereonEngineErrorFamily, SidereonEngineErrorInfo,
        };
        use crate::solve::{
            sidereon_solve_spp, sidereon_solve_spp_batch_parallel, sidereon_solve_spp_batch_serial,
        };
        use serde_json::Value;

        let sp3 = fixture_sp3();
        let sats = visible_gps(&sp3);
        assert!(sats.len() >= 4);

        // Epoch 0: all visible satellites (at least 4) -> success
        let (inputs0, doppler0) = core_inputs(&sp3, &sats);
        let tokens0: Vec<CString> = sats
            .iter()
            .map(|sat| CString::new(sat.to_string()).unwrap())
            .collect();
        let (obs0, _, c_inputs0) = c_inputs(&inputs0, &doppler0, &tokens0);

        // Epoch 1 supplies two satellites, but core selection uses only one at
        // the initial state, so the real refusal is TooFewSatellites { used: 1,
        // required: 4 }. Confirm that direct core outcome before checking C's
        // copied error payload and legacy text.
        let (inputs1, doppler1) = core_inputs(&sp3, &sats[..2]);
        let direct_error = sidereon::solve_spp(
            &sp3,
            &inputs1,
            true,
            sidereon_core::positioning::SolvePolicy::default(),
        )
        .expect_err("two supplied satellites leave only one usable at core selection");
        assert!(matches!(
            direct_error,
            sidereon::Error::Spp(sidereon_core::positioning::SolvePolicyError::Solve(
                sidereon_core::positioning::SppError::TooFewSatellites {
                    used: 1,
                    required: 4,
                }
            ))
        ));
        let tokens1: Vec<CString> = sats[..2]
            .iter()
            .map(|sat| CString::new(sat.to_string()).unwrap())
            .collect();
        let (obs1, _, c_inputs1) = c_inputs(&inputs1, &doppler1, &tokens1);

        let test_policy = SidereonSppSolvePolicy {
            use_validation_options: false,
            validation: SidereonSppValidationOptions {
                max_pdop_enabled: false,
                max_pdop: 0.0,
                min_plausible_radius_m: 0.0,
                max_plausible_radius_m: 0.0,
                max_converged_residual_rms_m: 0.0,
            },
            coarse_search_enabled: false,
            coarse_search_seeds: 0,
        };

        let sp3_handle = SidereonSp3 { inner: sp3 };
        let inputs_v2 = [c_inputs0, c_inputs1];

        unsafe fn run_batch_checks(
            batch_ptr: *mut SidereonSppBatch,
            sp3_handle: &SidereonSp3,
            c_inputs1: &SidereonSppInputsV2,
        ) -> (Vec<u8>, Vec<u8>) {
            // Seed generic TLS with real public refusal FIRST
            let mut seed_sol = ptr::null_mut();
            let status = sidereon_solve_spp(sp3_handle, &c_inputs1.base, &mut seed_sol);
            assert_eq!(status, SidereonStatus::Solve);
            assert!(seed_sol.is_null());

            let mut seed_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut seed_info),
                SidereonStatus::Ok
            );
            assert_eq!(seed_info.family, SidereonEngineErrorFamily::Facade);
            assert!(seed_info.payload_len > 0);
            let mut seed_buf = vec![0u8; seed_info.payload_len];
            let mut seed_written = 0;
            let mut seed_req = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    seed_buf.as_mut_ptr(),
                    seed_buf.len(),
                    &mut seed_written,
                    &mut seed_req,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(seed_written, seed_info.payload_len);

            let verify_generic_tls_intact = || {
                let mut cur_info = SidereonEngineErrorInfo {
                    family: SidereonEngineErrorFamily::None,
                    payload_len: 0,
                };
                assert_eq!(
                    sidereon_last_engine_error_info(&mut cur_info),
                    SidereonStatus::Ok
                );
                assert_eq!(cur_info.family, SidereonEngineErrorFamily::Facade);
                assert_eq!(cur_info.payload_len, seed_info.payload_len);
                let mut cur_buf = vec![0u8; cur_info.payload_len];
                let mut w = 0;
                let mut r = 0;
                assert_eq!(
                    sidereon_last_engine_error_payload(
                        cur_buf.as_mut_ptr(),
                        cur_buf.len(),
                        &mut w,
                        &mut r,
                    ),
                    SidereonStatus::Ok
                );
                assert_eq!(cur_buf, seed_buf);
            };

            // Count
            let mut count = 0;
            assert_eq!(
                sidereon_spp_batch_count(batch_ptr, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 2);
            verify_generic_tls_intact();

            // Epoch ok
            let mut ok = false;
            assert_eq!(
                sidereon_spp_batch_epoch_ok(batch_ptr, 0, &mut ok),
                SidereonStatus::Ok
            );
            assert!(ok);
            verify_generic_tls_intact();

            assert_eq!(
                sidereon_spp_batch_epoch_ok(batch_ptr, 1, &mut ok),
                SidereonStatus::Ok
            );
            assert!(!ok);
            verify_generic_tls_intact();

            assert_eq!(
                sidereon_spp_batch_epoch_ok(batch_ptr, 2, &mut ok),
                SidereonStatus::InvalidArgument
            );
            verify_generic_tls_intact();

            // Solution getter on epoch 0 (success)
            let mut sol_handle = ptr::null_mut();
            assert_eq!(
                sidereon_spp_batch_solution(batch_ptr, 0, &mut sol_handle),
                SidereonStatus::Ok
            );
            assert!(!sol_handle.is_null());
            verify_generic_tls_intact();

            sidereon_spp_solution_free(sol_handle);
            verify_generic_tls_intact();

            // Epoch 0 error info (empty)
            let mut info0 = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Spp,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_spp_batch_error_info(batch_ptr, 0, &mut info0),
                SidereonStatus::Ok
            );
            assert_eq!(info0.family, SidereonEngineErrorFamily::None);
            assert_eq!(info0.payload_len, 0);
            verify_generic_tls_intact();

            // Epoch 0 error payload (empty)
            let mut written = 999;
            let mut required = 999;
            let mut dummy = [0u8; 16];
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    0,
                    dummy.as_mut_ptr(),
                    dummy.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);
            verify_generic_tls_intact();

            // Epoch 0 legacy error (empty)
            assert_eq!(
                sidereon_spp_batch_error(
                    batch_ptr,
                    0,
                    dummy.as_mut_ptr(),
                    dummy.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);
            verify_generic_tls_intact();

            // Solution getter on failing epoch 1
            let mut sol_fail = ptr::null_mut();
            let status = sidereon_spp_batch_solution(batch_ptr, 1, &mut sol_fail);
            assert_eq!(status, SidereonStatus::Solve);
            assert!(sol_fail.is_null());

            // Assert documented failed-solution diagnostic
            let mut legacy_diag_buf = vec![0 as std::os::raw::c_char; 512];
            let diag_needed = crate::sidereon_last_error_message(
                legacy_diag_buf.as_mut_ptr(),
                legacy_diag_buf.len(),
            );
            assert!(diag_needed > 0);
            let legacy_diag_str = std::ffi::CStr::from_ptr(legacy_diag_buf.as_ptr())
                .to_str()
                .unwrap();
            assert_eq!(
                legacy_diag_str,
                "sidereon_spp_batch_solution: epoch 1 did not solve: SPP solve failed: only 1 usable satellites; need at least 4 (3 position + 1 clock per GNSS)"
            );
            verify_generic_tls_intact();

            // Error info getter on epoch 1
            let mut row1_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_spp_batch_error_info(batch_ptr, 1, &mut row1_info),
                SidereonStatus::Ok
            );
            assert_eq!(row1_info.family, SidereonEngineErrorFamily::Spp);
            assert!(row1_info.payload_len > 0);
            let row1_len = row1_info.payload_len;
            verify_generic_tls_intact();

            // Sizing query on epoch 1
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, row1_len);
            verify_generic_tls_intact();

            // Short buffer with sentinel 0xA5 (must remain completely untouched)
            let mut short_buf = vec![0xA5u8; row1_len - 1];
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    1,
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, row1_len);
            assert!(
                short_buf.iter().all(|&b| b == 0xA5),
                "short buffer must remain untouched"
            );
            verify_generic_tls_intact();

            // Full buffer on epoch 1
            let mut full_buf1 = vec![0u8; row1_len];
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    1,
                    full_buf1.as_mut_ptr(),
                    full_buf1.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, row1_len);
            assert_eq!(required, row1_len);
            verify_generic_tls_intact();

            let v1: Value = serde_json::from_slice(&full_buf1).expect("valid JSON");
            assert_eq!(v1["schema_version"], 1);
            assert_eq!(v1["family"], "spp");
            assert!(v1["operation"].as_str().unwrap().contains("epoch 1"));
            assert_eq!(v1["error"]["kind"], "too_few_satellites");
            assert_eq!(v1["error"]["fields"]["used"], 1);
            assert_eq!(v1["error"]["fields"]["required"], 4);

            // Legacy error getter on epoch 1
            let mut legacy_buf = vec![0u8; 256];
            assert_eq!(
                sidereon_spp_batch_error(
                    batch_ptr,
                    1,
                    legacy_buf.as_mut_ptr(),
                    legacy_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert!(written > 0);
            let legacy_msg = std::str::from_utf8(&legacy_buf[..written]).unwrap();
            assert_eq!(
                legacy_msg,
                "SPP solve failed: only 1 usable satellites; need at least 4 (3 position + 1 clock per GNSS)"
            );
            verify_generic_tls_intact();

            // Null checks and invalid index
            assert_eq!(
                sidereon_spp_batch_error_info(ptr::null(), 0, &mut row1_info),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error_info(batch_ptr, 0, ptr::null_mut()),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error_info(batch_ptr, 99, &mut row1_info),
                SidereonStatus::InvalidArgument
            );

            assert_eq!(
                sidereon_spp_batch_error_payload(
                    ptr::null(),
                    1,
                    full_buf1.as_mut_ptr(),
                    full_buf1.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    1,
                    ptr::null_mut(),
                    10,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    batch_ptr,
                    99,
                    full_buf1.as_mut_ptr(),
                    full_buf1.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );

            assert_eq!(
                sidereon_spp_batch_error(
                    ptr::null(),
                    1,
                    legacy_buf.as_mut_ptr(),
                    legacy_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error(
                    batch_ptr,
                    1,
                    ptr::null_mut(),
                    10,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_spp_batch_error(
                    batch_ptr,
                    99,
                    legacy_buf.as_mut_ptr(),
                    legacy_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );

            verify_generic_tls_intact();

            (full_buf1, seed_buf)
        }

        unsafe {
            // 1. Real public serial batch producer
            let mut batch_serial = ptr::null_mut();
            let status = sidereon_solve_spp_batch_serial(
                &sp3_handle,
                inputs_v2.as_ptr(),
                inputs_v2.len(),
                true,
                &test_policy,
                &mut batch_serial,
            );
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!batch_serial.is_null());

            let (full_buf_serial, seed_buf_serial) =
                run_batch_checks(batch_serial, &sp3_handle, &c_inputs1);
            let full_buf_serial_original = full_buf_serial.clone();
            let serial_value: Value = serde_json::from_slice(&full_buf_serial).expect("valid JSON");
            assert_eq!(
                serial_value["operation"],
                "sidereon_solve_spp_batch_serial epoch 1"
            );

            // Free retains generic TLS
            sidereon_spp_batch_free(batch_serial);

            let mut cur_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut cur_info),
                SidereonStatus::Ok
            );
            assert_eq!(cur_info.family, SidereonEngineErrorFamily::Facade);
            assert_eq!(cur_info.payload_len, seed_buf_serial.len());
            let mut cur_buf = vec![0u8; cur_info.payload_len];
            let mut w = 0;
            let mut r = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    cur_buf.as_mut_ptr(),
                    cur_buf.len(),
                    &mut w,
                    &mut r,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(cur_buf, seed_buf_serial);

            // Actual unrelated successful producer reset
            let mut success_sol = ptr::null_mut();
            let status = sidereon_solve_spp(&sp3_handle, &c_inputs0.base, &mut success_sol);
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!success_sol.is_null());
            sidereon_spp_solution_free(success_sol);

            let mut reset_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Spp,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut reset_info),
                SidereonStatus::Ok
            );
            assert_eq!(reset_info.family, SidereonEngineErrorFamily::None);
            assert_eq!(reset_info.payload_len, 0);

            // Owned row bytes unchanged throughout
            let v_after: Value = serde_json::from_slice(&full_buf_serial).expect("valid JSON");
            assert_eq!(v_after["schema_version"], 1);
            assert_eq!(v_after["family"], "spp");
            assert_eq!(
                v_after["operation"],
                "sidereon_solve_spp_batch_serial epoch 1"
            );
            assert_eq!(v_after["error"], serial_value["error"]);
            assert_eq!(v_after["error"]["kind"], "too_few_satellites");
            assert_eq!(full_buf_serial, full_buf_serial_original);
            assert_eq!(v_after["error"]["fields"]["used"], 1);
            assert_eq!(v_after["error"]["fields"]["required"], 4);

            // 2. Real public parallel batch producer
            let mut batch_parallel = ptr::null_mut();
            let status = sidereon_solve_spp_batch_parallel(
                &sp3_handle,
                inputs_v2.as_ptr(),
                inputs_v2.len(),
                true,
                &test_policy,
                &mut batch_parallel,
            );
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!batch_parallel.is_null());

            let (full_buf_parallel, seed_buf_parallel) =
                run_batch_checks(batch_parallel, &sp3_handle, &c_inputs1);
            let full_buf_parallel_original = full_buf_parallel.clone();
            let parallel_value: Value =
                serde_json::from_slice(&full_buf_parallel).expect("valid JSON");
            assert_eq!(
                parallel_value["operation"],
                "sidereon_solve_spp_batch_parallel epoch 1"
            );
            assert_eq!(
                parallel_value["schema_version"],
                serial_value["schema_version"]
            );
            assert_eq!(parallel_value["family"], serial_value["family"]);
            assert_eq!(parallel_value["error"], serial_value["error"]);

            sidereon_spp_batch_free(batch_parallel);

            let mut cur_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut cur_info),
                SidereonStatus::Ok
            );
            assert_eq!(cur_info.family, SidereonEngineErrorFamily::Facade);
            assert_eq!(cur_info.payload_len, seed_buf_parallel.len());
            let mut cur_buf = vec![0u8; cur_info.payload_len];
            let mut w = 0;
            let mut r = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    cur_buf.as_mut_ptr(),
                    cur_buf.len(),
                    &mut w,
                    &mut r,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(cur_buf, seed_buf_parallel);

            // Actual unrelated successful producer reset
            let mut success_sol2 = ptr::null_mut();
            let status = sidereon_solve_spp(&sp3_handle, &c_inputs0.base, &mut success_sol2);
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!success_sol2.is_null());
            sidereon_spp_solution_free(success_sol2);

            let mut reset_info2 = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Spp,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut reset_info2),
                SidereonStatus::Ok
            );
            assert_eq!(reset_info2.family, SidereonEngineErrorFamily::None);
            assert_eq!(reset_info2.payload_len, 0);

            // This producer's complete owned row bytes survive reset.
            assert_eq!(full_buf_parallel, full_buf_parallel_original);

            // 3. Real public v2 serial batch producer
            let mut model_opts = MaybeUninit::<SidereonSppModelOptions>::uninit();
            assert_eq!(
                sidereon_spp_model_options_init(model_opts.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let model_opts = model_opts.assume_init();

            let batch_v2_inputs = [
                SidereonSppBatchInputV2 {
                    inputs: c_inputs0,
                    models: model_opts,
                },
                SidereonSppBatchInputV2 {
                    inputs: c_inputs1,
                    models: model_opts,
                },
            ];

            let mut batch_v2 = ptr::null_mut();
            let status = sidereon_solve_spp_batch_v2_serial(
                &sp3_handle,
                batch_v2_inputs.as_ptr(),
                batch_v2_inputs.len(),
                &mut batch_v2,
            );
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!batch_v2.is_null());

            let (full_buf_v2, seed_buf_v2) = run_batch_checks(batch_v2, &sp3_handle, &c_inputs1);
            let full_buf_v2_original = full_buf_v2.clone();
            let v2_value: Value = serde_json::from_slice(&full_buf_v2).expect("valid JSON");
            assert_eq!(
                v2_value["operation"],
                "sidereon_solve_spp_batch_v2_serial epoch 1"
            );
            assert_eq!(v2_value["schema_version"], serial_value["schema_version"]);
            assert_eq!(v2_value["family"], serial_value["family"]);
            assert_eq!(v2_value["error"], serial_value["error"]);

            sidereon_spp_batch_free(batch_v2);

            let mut cur_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut cur_info),
                SidereonStatus::Ok
            );
            assert_eq!(cur_info.family, SidereonEngineErrorFamily::Facade);
            assert_eq!(cur_info.payload_len, seed_buf_v2.len());
            let mut cur_buf = vec![0u8; cur_info.payload_len];
            let mut w = 0;
            let mut r = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    cur_buf.as_mut_ptr(),
                    cur_buf.len(),
                    &mut w,
                    &mut r,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(cur_buf, seed_buf_v2);

            // Actual unrelated successful producer reset
            let mut success_sol3 = ptr::null_mut();
            let status = sidereon_solve_spp(&sp3_handle, &c_inputs0.base, &mut success_sol3);
            assert_eq!(status, SidereonStatus::Ok);
            assert!(!success_sol3.is_null());
            sidereon_spp_solution_free(success_sol3);

            let mut reset_info3 = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Spp,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut reset_info3),
                SidereonStatus::Ok
            );
            assert_eq!(reset_info3.family, SidereonEngineErrorFamily::None);
            assert_eq!(reset_info3.payload_len, 0);

            // This producer's complete owned row bytes survive reset.
            assert_eq!(full_buf_v2, full_buf_v2_original);

            // 4. Pure mapping control for policy branch (NoCoarseSolution)
            let policy_err = spp_batch_row_error_from_facade(
                "test_policy epoch 0",
                sidereon::Error::Spp(
                    sidereon_core::positioning::SolvePolicyError::NoCoarseSolution,
                ),
            );
            let policy_batch = Box::into_raw(Box::new(SidereonSppBatch {
                inner: vec![Err(policy_err)],
            }));

            let mut policy_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_spp_batch_error_info(policy_batch, 0, &mut policy_info),
                SidereonStatus::Ok
            );
            assert_eq!(policy_info.family, SidereonEngineErrorFamily::SppPolicy);
            assert!(policy_info.payload_len > 0);

            let mut policy_buf = vec![0u8; policy_info.payload_len];
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_spp_batch_error_payload(
                    policy_batch,
                    0,
                    policy_buf.as_mut_ptr(),
                    policy_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, policy_info.payload_len);
            let v_pol: Value = serde_json::from_slice(&policy_buf).expect("valid JSON");
            assert_eq!(v_pol["schema_version"], 1);
            assert_eq!(v_pol["family"], "spp_policy");
            assert_eq!(v_pol["operation"], "test_policy epoch 0");
            assert_eq!(v_pol["error"]["kind"], "no_coarse_solution");

            sidereon_spp_batch_free(policy_batch);
        }

        drop(obs0);
        drop(obs1);
    }
}
