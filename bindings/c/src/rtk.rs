use super::*;
use sidereon_core::positioning::{
    solve_static_reference_station_rinex, RinexSppOptions, StaticReferenceCarrierRinexOptions,
    StaticReferenceEpochDiagnostic, StaticReferenceFixStatus, StaticReferenceModeReport,
    StaticReferenceModeStatus, StaticReferenceStationError, StaticReferenceStationMode,
    StaticReferenceStationRinexOptions, StaticReferenceStationSolution,
};

/// The result of an RTK float solve. Opaque to C. Create with
/// sidereon_solve_rtk_float and release with sidereon_rtk_float_solution_free.
pub struct SidereonRtkFloatSolution {
    pub(crate) inner: FloatBaselineSolution,
    pub(crate) base_ecef_m: [f64; 3],
}

/// The result of an RTK fixed solve. Opaque to C. Create with
/// sidereon_solve_rtk_fixed and release with sidereon_rtk_fixed_solution_free.
pub struct SidereonRtkFixedSolution {
    pub(crate) inner: ValidatedFixedBaselineSolution,
    pub(crate) base_ecef_m: [f64; 3],
}

/// Fixed-size null-terminated RTK ambiguity id storage. Values returned by
/// Sidereon are always null-terminated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkId {
    /// Null-terminated id bytes.
    pub bytes: [c_char; 65],
}

/// RTK stochastic measurement weighting model.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkStochasticModel {
    /// Simple sigma model, optionally elevation weighted.
    Simple = 0,
    /// RTKLIB-compatible floor-plus-elevation model.
    Rtklib = 1,
}

/// Terminal status of an RTK float or fixed least-squares solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkSolveStatus {
    /// State update tolerances were reached.
    StateTolerance = 0,
    /// Maximum iterations were reached.
    MaxIterations = 1,
}

/// Integer ambiguity-fix verdict for an RTK fixed solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkIntegerStatus {
    /// The ambiguity search accepted an integer fix.
    Fixed = 0,
    /// The ambiguity search rejected the integer fix.
    NotFixed = 1,
}

/// One satellite's base/rover measurements for an RTK epoch.
#[repr(C)]
pub struct SidereonRtkSatMeasurement {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Null-terminated single-difference ambiguity id.
    pub sd_ambiguity_id: *const c_char,
    /// Base receiver code observable in meters.
    pub base_code_m: f64,
    /// Base receiver carrier phase observable in meters.
    pub base_phase_m: f64,
    /// Rover receiver code observable in meters.
    pub rover_code_m: f64,
    /// Rover receiver carrier phase observable in meters.
    pub rover_phase_m: f64,
    /// Satellite transmit-position for the base receiver, ECEF meters.
    pub base_tx_pos: [f64; 3],
    /// Satellite transmit-position for the rover receiver, ECEF meters.
    pub rover_tx_pos: [f64; 3],
    /// Shared receive-time satellite position for weighting, ECEF meters.
    pub pos: [f64; 3],
}

/// One RTK epoch with per-system reference and non-reference satellite rows.
#[repr(C)]
pub struct SidereonRtkEpoch {
    /// Pointer to reference_count reference rows.
    pub references: *const SidereonRtkSatMeasurement,
    /// Number of reference rows.
    pub reference_count: usize,
    /// Pointer to nonref_count non-reference rows.
    pub nonref: *const SidereonRtkSatMeasurement,
    /// Number of non-reference rows.
    pub nonref_count: usize,
    /// Whether velocity_mps is supplied.
    pub has_velocity_mps: bool,
    /// Rover ECEF velocity in meters per second when has_velocity_mps is true.
    pub velocity_mps: [f64; 3],
    /// Elapsed seconds since the previous epoch.
    pub dt_s: f64,
}

/// RTK measurement weighting and correction model.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkMeasurementModel {
    /// Code standard deviation in meters.
    pub code_sigma_m: f64,
    /// Carrier phase standard deviation in meters.
    pub phase_sigma_m: f64,
    /// Apply the engine's RTK Sagnac correction.
    pub sagnac: bool,
    /// One of SidereonRtkStochasticModel_* encoded as uint32_t.
    pub stochastic: u32,
    /// Simple-model elevation weighting flag.
    pub elevation_weighting: bool,
}

/// Iteration controls for an RTK float solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFloatOptions {
    /// Position update tolerance in meters.
    pub position_tol_m: f64,
    /// Ambiguity update tolerance in meters.
    pub ambiguity_tol_m: f64,
    /// Maximum solver iterations.
    pub max_iterations: usize,
}

/// Iteration and integer-search controls for RTK fixed solving.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFixedOptions {
    /// Position update tolerance in meters.
    pub position_tol_m: f64,
    /// Ambiguity update tolerance in meters.
    pub ambiguity_tol_m: f64,
    /// Maximum solver iterations.
    pub max_iterations: usize,
    /// Integer ratio-test threshold.
    pub ratio_threshold: f64,
    /// Enable partial ambiguity resolution.
    pub partial_ambiguity_resolution: bool,
    /// Minimum ambiguities to hold during partial ambiguity resolution.
    pub partial_min_ambiguities: usize,
}

/// Residual-validation controls for RTK fixed solving.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkResidualValidationOptions {
    /// Whether threshold_sigma is supplied.
    pub threshold_sigma_enabled: bool,
    /// Residual rejection threshold in sigma when enabled.
    pub threshold_sigma: f64,
    /// Maximum residual-validation exclusions.
    pub max_exclusions: usize,
}

/// One RTK string-to-f64 map entry, used for wavelength and offset maps.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFloatMapEntry {
    /// Null-terminated ambiguity id.
    pub id: *const c_char,
    /// Map value.
    pub value: f64,
}

/// One RTK ambiguity-id to satellite-token map entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkAmbiguitySatellite {
    /// Null-terminated ambiguity id.
    pub id: *const c_char,
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
}

/// RTK receiver-antenna corrections for the base and rover receivers. Set the
/// config receiver_antenna pointer to NULL to disable this correction.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkReceiverAntennaCorrections {
    /// Base receiver calibration.
    pub base: SidereonReceiverAntennaCalibration,
    /// Rover receiver calibration.
    pub rover: SidereonReceiverAntennaCalibration,
}

/// Complete typed input bundle for an RTK float solve. Initialize model and
/// options with sidereon_rtk_measurement_model_init and
/// sidereon_rtk_float_options_init before overriding fields.
#[repr(C)]
pub struct SidereonRtkFloatConfig {
    /// Pointer to epoch_count epochs.
    pub epochs: *const SidereonRtkEpoch,
    /// Number of epochs.
    pub epoch_count: usize,
    /// Base receiver ECEF position in meters.
    pub base_ecef_m: [f64; 3],
    /// Pointer to ambiguity_id_count null-terminated ambiguity id strings.
    pub ambiguity_ids: *const *const c_char,
    /// Number of ambiguity id strings.
    pub ambiguity_id_count: usize,
    /// RTK measurement model.
    pub model: SidereonRtkMeasurementModel,
    /// Optional receiver-antenna corrections for base/rover; NULL disables them.
    pub receiver_antenna: *const SidereonRtkReceiverAntennaCorrections,
    /// Initial rover-minus-base ECEF baseline in meters.
    pub initial_baseline_m: [f64; 3],
    /// Float solve options.
    pub options: SidereonRtkFloatOptions,
}

/// Complete typed input bundle for an RTK fixed solve. Initialize model and
/// option structs with their init functions before overriding fields.
#[repr(C)]
pub struct SidereonRtkFixedConfig {
    /// Pointer to epoch_count epochs.
    pub epochs: *const SidereonRtkEpoch,
    /// Number of epochs.
    pub epoch_count: usize,
    /// Base receiver ECEF position in meters.
    pub base_ecef_m: [f64; 3],
    /// Pointer to ambiguity_id_count null-terminated ambiguity id strings.
    pub ambiguity_ids: *const *const c_char,
    /// Number of ambiguity id strings.
    pub ambiguity_id_count: usize,
    /// Pointer to ambiguity_satellite_count ambiguity-to-satellite map entries.
    pub ambiguity_satellites: *const SidereonRtkAmbiguitySatellite,
    /// Number of ambiguity-to-satellite map entries.
    pub ambiguity_satellite_count: usize,
    /// Pointer to wavelength_count ambiguity wavelength entries.
    pub wavelengths_m: *const SidereonRtkFloatMapEntry,
    /// Number of wavelength entries.
    pub wavelength_count: usize,
    /// Pointer to offset_count ambiguity offset entries.
    pub offsets_m: *const SidereonRtkFloatMapEntry,
    /// Number of offset entries.
    pub offset_count: usize,
    /// RTK measurement model.
    pub model: SidereonRtkMeasurementModel,
    /// Optional receiver-antenna corrections for base/rover; NULL disables them.
    pub receiver_antenna: *const SidereonRtkReceiverAntennaCorrections,
    /// Float solve options used before integer fixing.
    pub float_options: SidereonRtkFloatOptions,
    /// Fixed solve options.
    pub fixed_options: SidereonRtkFixedOptions,
    /// Residual validation options.
    pub residual_options: SidereonRtkResidualValidationOptions,
    /// Optional array of SidereonGnssSystem_* values encoded as uint32_t.
    pub float_only_systems: *const u32,
    /// Number of float-only system entries.
    pub float_only_system_count: usize,
    /// Initial rover-minus-base ECEF baseline in meters.
    pub initial_baseline_m: [f64; 3],
}

/// One float ambiguity estimate in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkAmbiguity {
    /// Ambiguity id.
    pub id: SidereonRtkId,
    /// Ambiguity estimate in meters.
    pub value_m: f64,
}

/// One fixed integer ambiguity estimate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFixedAmbiguity {
    /// Ambiguity id.
    pub id: SidereonRtkId,
    /// Fixed ambiguity in carrier cycles.
    pub cycles: i64,
    /// Fixed ambiguity in meters after wavelength and offset scaling.
    pub value_m: f64,
}

/// Summary scalars for an RTK float solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFloatMetadata {
    /// Solver iterations.
    pub iterations: usize,
    /// Whether the solver converged.
    pub converged: bool,
    /// Terminal solve status.
    pub status: SidereonRtkSolveStatus,
    /// Code residual RMS in meters.
    pub code_rms_m: f64,
    /// Carrier phase residual RMS in meters.
    pub phase_rms_m: f64,
    /// Weighted residual RMS in meters.
    pub weighted_rms_m: f64,
    /// Number of scalar observations.
    pub n_observations: usize,
    /// Number of float ambiguity estimates.
    pub ambiguity_count: usize,
    /// Number of residual rows.
    pub residual_count: usize,
    /// Number of unique used satellites in residual order.
    pub used_sat_count: usize,
    /// Geometry observability and covariance-validation diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
}

/// Summary scalars and integer-search metadata for an RTK fixed solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkFixedMetadata {
    /// Solver iterations.
    pub iterations: usize,
    /// Whether the fixed re-solve converged.
    pub converged: bool,
    /// Terminal solve status.
    pub status: SidereonRtkSolveStatus,
    /// Code residual RMS in meters.
    pub code_rms_m: f64,
    /// Carrier phase residual RMS in meters.
    pub phase_rms_m: f64,
    /// Weighted residual RMS in meters.
    pub weighted_rms_m: f64,
    /// Number of scalar observations.
    pub n_observations: usize,
    /// Number of ambiguities left float in the fixed solve.
    pub free_ambiguity_count: usize,
    /// Number of fixed integer ambiguities.
    pub fixed_ambiguity_count: usize,
    /// Number of residual rows.
    pub residual_count: usize,
    /// Number of unique used satellites in residual order.
    pub used_sat_count: usize,
    /// Integer ambiguity-fix verdict.
    pub integer_status: SidereonRtkIntegerStatus,
    /// Whether integer_ratio is present.
    pub has_integer_ratio: bool,
    /// Integer ratio when present.
    pub integer_ratio: f64,
    /// Whether integer_best_score is present.
    pub has_integer_best_score: bool,
    /// Best integer-search score when present.
    pub integer_best_score: f64,
    /// Whether integer_second_best_score is present.
    pub has_integer_second_best_score: bool,
    /// Second-best integer-search score when present.
    pub integer_second_best_score: f64,
    /// Number of integer candidates evaluated or reported by the search.
    pub integer_candidates: usize,
    /// Geometry observability and covariance-validation diagnostics from the
    /// float solve used by integer fixing.
    pub geometry_quality: SidereonGeometryQuality,
}

/// Initialize an RTK measurement model with engine binding defaults.
///
/// Safety: out_model must point to a SidereonRtkMeasurementModel.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_measurement_model_init(
    out_model: *mut SidereonRtkMeasurementModel,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_measurement_model_init",
        SidereonStatus::Panic,
        || {
            let out_model = c_try!(require_out(
                out_model,
                "sidereon_rtk_measurement_model_init",
                "out_model"
            ));
            *out_model = default_rtk_measurement_model();
            SidereonStatus::Ok
        },
    )
}

/// Initialize RTK float solve options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonRtkFloatOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_options_init(
    out_options: *mut SidereonRtkFloatOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_rtk_float_options_init",
                "out_options"
            ));
            *out_options = default_rtk_float_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize RTK fixed solve options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonRtkFixedOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_options_init(
    out_options: *mut SidereonRtkFixedOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_rtk_fixed_options_init",
                "out_options"
            ));
            *out_options = default_rtk_fixed_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize RTK residual-validation options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonRtkResidualValidationOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_residual_validation_options_init(
    out_options: *mut SidereonRtkResidualValidationOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_residual_validation_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_rtk_residual_validation_options_init",
                "out_options"
            ));
            *out_options = default_rtk_residual_options();
            SidereonStatus::Ok
        },
    )
}

/// Copy the RTK float rover-minus-base ECEF baseline into out_xyz.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_baseline_ecef(
    sol: *const SidereonRtkFloatSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_solution_baseline_ecef",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_rtk_float_solution_baseline_ecef",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_float_solution_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_float_solution_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the RTK float rover-minus-base baseline in the geocentric local
/// East-North-Up frame at the base receiver.
///
/// Safety: sol must be a live solution handle; out_enu must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_baseline_enu(
    sol: *const SidereonRtkFloatSolution,
    out_enu: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_solution_baseline_enu",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_enu,
                "sidereon_rtk_float_solution_baseline_enu",
                "out_enu"
            ));
            zero_f64_prefix(out_enu, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_float_solution_baseline_enu",
                "solution"
            ));
            let enu = geocentric_enu(sol.base_ecef_m, sol.inner.baseline_m);
            c_try!(copy_exact_f64s(
                "sidereon_rtk_float_solution_baseline_enu",
                "out_enu",
                out_enu,
                len,
                &enu,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy RTK float metadata into *out_metadata.
///
/// Safety: sol must be a live solution handle; out_metadata must point to a
/// SidereonRtkFloatMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_metadata(
    sol: *const SidereonRtkFloatSolution,
    out_metadata: *mut SidereonRtkFloatMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_float_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = SidereonRtkFloatMetadata {
                iterations: 0,
                converged: false,
                status: SidereonRtkSolveStatus::StateTolerance,
                code_rms_m: 0.0,
                phase_rms_m: 0.0,
                weighted_rms_m: 0.0,
                n_observations: 0,
                ambiguity_count: 0,
                residual_count: 0,
                used_sat_count: 0,
                geometry_quality: empty_geometry_quality(),
            };
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_float_solution_metadata",
                "solution"
            ));
            *out_metadata = rtk_float_metadata(&sol.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy float ambiguity estimates in meters. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_ambiguities(
    sol: *const SidereonRtkFloatSolution,
    out: *mut SidereonRtkAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_solution_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_float_solution_ambiguities",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_float_solution_ambiguities",
                "solution"
            ));
            let values = rtk_ambiguities_to_c(&sol.inner.ambiguities_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_float_solution_ambiguities",
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

/// Copy unique used satellite tokens from RTK float residuals. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_used_sat_ids(
    sol: *const SidereonRtkFloatSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_float_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_float_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_float_solution_used_sat_ids",
                "solution"
            ));
            let values = rtk_used_satellite_tokens(&sol.inner.residuals);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_float_solution_used_sat_ids",
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

/// Copy the RTK fixed rover-minus-base ECEF baseline into out_xyz.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_fixed_baseline_ecef(
    sol: *const SidereonRtkFixedSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_fixed_baseline_ecef",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_rtk_fixed_solution_fixed_baseline_ecef",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_fixed_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_fixed_solution_fixed_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.fixed_solution.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the RTK fixed rover-minus-base baseline in the geocentric local
/// East-North-Up frame at the base receiver.
///
/// Safety: sol must be a live solution handle; out_enu must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_fixed_baseline_enu(
    sol: *const SidereonRtkFixedSolution,
    out_enu: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_fixed_baseline_enu",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_enu,
                "sidereon_rtk_fixed_solution_fixed_baseline_enu",
                "out_enu"
            ));
            zero_f64_prefix(out_enu, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_fixed_baseline_enu",
                "solution"
            ));
            let enu = geocentric_enu(sol.base_ecef_m, sol.inner.fixed_solution.baseline_m);
            c_try!(copy_exact_f64s(
                "sidereon_rtk_fixed_solution_fixed_baseline_enu",
                "out_enu",
                out_enu,
                len,
                &enu,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the underlying RTK float ECEF baseline used by a fixed solve.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_float_baseline_ecef(
    sol: *const SidereonRtkFixedSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_float_baseline_ecef",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_rtk_fixed_solution_float_baseline_ecef",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_float_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_fixed_solution_float_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.float_solution.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy RTK fixed metadata into *out_metadata.
///
/// Safety: sol must be a live solution handle; out_metadata must point to a
/// SidereonRtkFixedMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_metadata(
    sol: *const SidereonRtkFixedSolution,
    out_metadata: *mut SidereonRtkFixedMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_fixed_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = SidereonRtkFixedMetadata {
                iterations: 0,
                converged: false,
                status: SidereonRtkSolveStatus::StateTolerance,
                code_rms_m: 0.0,
                phase_rms_m: 0.0,
                weighted_rms_m: 0.0,
                n_observations: 0,
                free_ambiguity_count: 0,
                fixed_ambiguity_count: 0,
                residual_count: 0,
                used_sat_count: 0,
                integer_status: SidereonRtkIntegerStatus::NotFixed,
                has_integer_ratio: false,
                integer_ratio: 0.0,
                has_integer_best_score: false,
                integer_best_score: 0.0,
                has_integer_second_best_score: false,
                integer_second_best_score: 0.0,
                integer_candidates: 0,
                geometry_quality: empty_geometry_quality(),
            };
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_metadata",
                "solution"
            ));
            *out_metadata = rtk_fixed_metadata(&sol.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy fixed-solve free ambiguity estimates in meters. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_free_ambiguities(
    sol: *const SidereonRtkFixedSolution,
    out: *mut SidereonRtkAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_free_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_fixed_solution_free_ambiguities",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_free_ambiguities",
                "solution"
            ));
            let values = rtk_ambiguities_to_c(&sol.inner.fixed_solution.free_ambiguities_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_fixed_solution_free_ambiguities",
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

/// Copy fixed integer ambiguities. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_fixed_ambiguities(
    sol: *const SidereonRtkFixedSolution,
    out: *mut SidereonRtkFixedAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_fixed_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_fixed_solution_fixed_ambiguities",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_fixed_ambiguities",
                "solution"
            ));
            let values = c_try!(rtk_fixed_ambiguities_to_c(
                "sidereon_rtk_fixed_solution_fixed_ambiguities",
                &sol.inner
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_fixed_solution_fixed_ambiguities",
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

/// Copy unique used satellite tokens from RTK fixed residuals. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_used_sat_ids(
    sol: *const SidereonRtkFixedSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_fixed_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_fixed_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_rtk_fixed_solution_used_sat_ids",
                "solution"
            ));
            let values = rtk_used_satellite_tokens(&sol.inner.fixed_solution.residuals);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_fixed_solution_used_sat_ids",
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

/// Release an RTK float solution handle. Null is a no-op. A non-null handle
/// must come from sidereon_solve_rtk_float and must be freed exactly once with
/// this function.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_rtk_float.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_float_solution_free(sol: *mut SidereonRtkFloatSolution) {
    ffi_boundary("sidereon_rtk_float_solution_free", (), || {
        free_boxed(sol);
    });
}

/// Release an RTK fixed solution handle. Null is a no-op. A non-null handle
/// must come from sidereon_solve_rtk_fixed and must be freed exactly once with
/// this function.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_rtk_fixed.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_fixed_solution_free(sol: *mut SidereonRtkFixedSolution) {
    ffi_boundary("sidereon_rtk_fixed_solution_free", (), || {
        free_boxed(sol);
    });
}

// --- Sequential RTK baseline arc driver (sidereon_core::rtk_filter::arc) ------

/// One raw single-frequency code/carrier observation at a receiver, mirroring
/// sidereon_core::rtk_filter::RtkArcObservation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcObservation {
    /// Physical satellite id token, e.g. "G05".
    pub sat_id: *const c_char,
    /// Ambiguity-arc id. A clean arc uses the satellite id; a cycle-slip split
    /// carries a distinct id (e.g. "G05#2") so the single-difference key resets.
    pub ambiguity_id: *const c_char,
    /// Code pseudorange (metres).
    pub code_m: f64,
    /// Carrier phase range (metres).
    pub phase_m: f64,
    /// Whether the optional loss-of-lock indicator is present.
    pub has_lli: bool,
    /// Loss-of-lock indicator. Read only when has_lli is true and cycle-slip
    /// preprocessing is enabled.
    pub lli: i64,
}

/// One satellite-id-keyed ECEF position entry (metres) for an RTK arc epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcPositionEntry {
    /// Satellite id token, e.g. "G05".
    pub id: *const c_char,
    /// Satellite ECEF position (metres).
    pub pos: [f64; 3],
}

/// One raw RTK arc epoch, mirroring sidereon_core::rtk_filter::RtkArcEpoch. The
/// per-receiver position arrays default to `satellite_positions` when their count
/// is zero.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcEpoch {
    /// Base-station observations.
    pub base: *const SidereonRtkArcObservation,
    /// Number of base observations.
    pub base_count: usize,
    /// Rover observations.
    pub rover: *const SidereonRtkArcObservation,
    /// Number of rover observations.
    pub rover_count: usize,
    /// Shared receive-time satellite ECEF positions (metres).
    pub satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of shared positions.
    pub satellite_position_count: usize,
    /// Transmit-time base satellite ECEF positions; empty defaults to the shared map.
    pub base_satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of base positions.
    pub base_satellite_position_count: usize,
    /// Transmit-time rover satellite ECEF positions; empty defaults to the shared map.
    pub rover_satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of rover positions.
    pub rover_satellite_position_count: usize,
    /// Whether an optional rover ECEF velocity is present.
    pub has_velocity_mps: bool,
    /// Rover ECEF velocity (metres/second) for the velocity-propagated branch.
    pub velocity_mps: [f64; 3],
    /// Whether an optional epoch time coordinate is present.
    pub has_prediction_time: bool,
    /// Epoch time coordinate (seconds) for prediction-delta computation.
    pub prediction_time_s: f64,
}

/// Reference-satellite selection mode for an RTK arc, mirroring
/// sidereon_core::rtk::BaselineReferenceSelection. Pass in
/// SidereonRtkArcConfig.reference_mode as a uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkArcReferenceMode {
    /// Pick the highest-average-elevation satellite per constellation.
    Auto = 0,
    /// Use one fixed reference satellite (single-system data only).
    Satellite = 1,
    /// Use one fixed reference satellite per constellation.
    PerSystem = 2,
}

/// Cycle-slip preprocessing policy for an RTK arc, mirroring
/// sidereon_core::rtk_filter::CycleSlipPolicy. Pass in
/// SidereonRtkArcPreprocessing.cycle_slip as a uint32_t when
/// has_cycle_slip is true.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkCycleSlipPolicy {
    /// Fail the solve when any slip is detected.
    Error = 0,
    /// Drop any satellite with a detected slip.
    DropSatellite = 1,
    /// Split the affected satellite into a new ambiguity arc.
    SplitArc = 2,
}

/// Receiver side reported for split cycle-slip arcs.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkCycleSlipReceiver {
    /// Base receiver observation.
    Base = 0,
    /// Rover receiver observation.
    Rover = 1,
}

/// One per-constellation reference satellite for the PerSystem mode.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcReferenceEntry {
    /// Constellation, a SidereonGnssSystem value.
    pub system: SidereonGnssSystem,
    /// Reference satellite id token for that constellation, e.g. "G04".
    pub sat_id: *const c_char,
}

/// Per-epoch sequential-update controls for the RTK arc, mirroring
/// sidereon_core::rtk_filter::UpdateOpts. Initialize with
/// sidereon_rtk_arc_update_options_init, then override fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcUpdateOptions {
    /// Hold sigma (metres) applied to newly fixed integer ambiguities.
    pub hold_sigma_m: f64,
    /// Position convergence tolerance (metres).
    pub position_tol_m: f64,
    /// Ambiguity convergence tolerance (metres).
    pub ambiguity_tol_m: f64,
    /// Maximum solve iterations per epoch.
    pub max_iterations: usize,
    /// Kinematic process-noise sigma (metres) for the baseline. 0 is the static
    /// filter; > 0 inflates the baseline covariance between epochs.
    pub process_noise_baseline_sigma_m: f64,
    /// When true, advance the baseline mean by velocity * elapsed seconds; when
    /// false, keep the carried baseline mean fixed.
    pub dynamics_velocity_propagated: bool,
    /// Constellations whose ambiguities never enter the LAMBDA search (float-only
    /// rows), each a SidereonGnssSystem value as a uint32_t.
    pub float_only_systems: *const u32,
    /// Number of float-only systems.
    pub float_only_system_count: usize,
    /// Emit public residual diagnostics in each epoch solution.
    pub report_residuals: bool,
    /// Whether the optional AR commitment arming gate is set.
    pub has_ar_arming_sigma_m: bool,
    /// AR arming gate: attempt the search only once the baseline posterior
    /// standard deviation is at most this many metres.
    pub ar_arming_sigma_m: f64,
    /// LAMBDA integer ratio acceptance threshold.
    pub ratio_threshold: f64,
    /// Optional receiver-antenna PCO/PCV corrections, or NULL.
    pub receiver_antenna: *const SidereonRtkReceiverAntennaCorrections,
}

/// Optional preprocessing for an RTK arc, mirroring
/// sidereon_core::rtk_filter::RtkArcPreprocessing. Zeroed fields disable every
/// stage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcPreprocessing {
    /// Whether cycle-slip preprocessing is enabled.
    pub has_cycle_slip: bool,
    /// Cycle-slip policy, a SidereonRtkCycleSlipPolicy value.
    pub cycle_slip: u32,
    /// Whether Hatch code smoothing is enabled.
    pub has_hatch_window_cap: bool,
    /// Hatch code-smoothing window cap when enabled.
    pub hatch_window_cap: usize,
    /// Whether elevation masking is enabled.
    pub has_elevation_mask_deg: bool,
    /// Elevation mask in degrees when enabled.
    pub elevation_mask_deg: f64,
}

/// Sequential RTK arc driver configuration, mirroring
/// sidereon_core::rtk_filter::RtkArcConfig.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcConfig {
    /// Base-station ECEF position (metres).
    pub base_m: [f64; 3],
    /// Reference-selection mode, a SidereonRtkArcReferenceMode value.
    pub reference_mode: u32,
    /// Fixed reference satellite id token for the Satellite mode (NULL otherwise).
    pub reference_satellite: *const c_char,
    /// Per-constellation references for the PerSystem mode.
    pub reference_per_system: *const SidereonRtkArcReferenceEntry,
    /// Number of per-system references.
    pub reference_per_system_count: usize,
    /// Measurement model (sigmas, Sagnac, stochastic model).
    pub model: SidereonRtkMeasurementModel,
    /// Baseline prior sigma (metres) for the initial information matrix.
    pub baseline_prior_sigma_m: f64,
    /// Ambiguity prior sigma (metres) for each new SD ambiguity column.
    pub ambiguity_prior_sigma_m: f64,
    /// Initial baseline guess (metres, ECEF rover - base).
    pub initial_baseline_m: [f64; 3],
    /// Per-ambiguity carrier wavelengths (metres) for the integer search.
    pub wavelengths_m: *const SidereonRtkFloatMapEntry,
    /// Number of wavelength entries.
    pub wavelength_count: usize,
    /// Per-ambiguity code-to-phase metre offsets for the integer search.
    pub offsets_m: *const SidereonRtkFloatMapEntry,
    /// Number of offset entries.
    pub offset_count: usize,
    /// Per-epoch sequential-update controls.
    pub update_options: SidereonRtkArcUpdateOptions,
    /// Optional preprocessing chained before the arc solve.
    pub preprocessing: SidereonRtkArcPreprocessing,
}

/// One epoch's reported RTK arc solution metadata, mirroring the scalar fields of
/// sidereon_core::rtk_filter::RtkArcEpochSolution. Read the id and ambiguity
/// lists with the sidereon_rtk_arc_solution_epoch_* accessors.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcEpochMetadata {
    /// Ambiguity-conditioned reported baseline for this epoch (metres).
    pub reported_baseline_m: [f64; 3],
    /// Carried float (Kalman posterior) baseline after this epoch (metres).
    pub float_baseline_m: [f64; 3],
    /// Whether any integer ambiguity is held after this epoch.
    pub integer_fixed: bool,
    /// Integer ratio from this epoch's ambiguity search (0 = no search ran).
    pub integer_ratio: f64,
    /// Number of single-difference ambiguity ids newly fixed this epoch.
    pub newly_fixed_count: usize,
    /// Number of held single-difference ambiguity ids after this epoch.
    pub fixed_id_count: usize,
    /// Number of double-difference ambiguity ids fixed this epoch.
    pub fixed_double_difference_count: usize,
    /// Number of satellites used this epoch.
    pub used_satellite_count: usize,
    /// Number of reported single-difference ambiguities.
    pub sd_ambiguity_count: usize,
    /// Number of public residual rows (when residual reporting is enabled).
    pub residual_count: usize,
    /// Whether a LAMBDA search ran this epoch.
    pub has_search: bool,
}

/// Selects which per-epoch single-difference id list a reader returns. Pass as a
/// uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtkArcEpochIdList {
    /// Single-difference ambiguity ids newly fixed this epoch.
    NewlyFixed = 0,
    /// All held single-difference ambiguity ids after this epoch.
    FixedIds = 1,
    /// Double-difference ambiguity ids fixed this epoch.
    FixedDoubleDifferenceIds = 2,
}

/// One per-constellation reference single-difference ambiguity id of an RTK arc
/// solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcReferenceOut {
    /// Constellation letter (e.g. "G"), null-terminated.
    pub system: SidereonRtkId,
    /// Reference single-difference ambiguity id.
    pub reference_id: SidereonRtkId,
}

/// One split cycle-slip arc reported by an RTK arc solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcSplitArc {
    /// Receiver side, a SidereonRtkCycleSlipReceiver value.
    pub receiver: u32,
    /// Physical satellite id token.
    pub satellite_id: SidereonSatelliteToken,
    /// Ambiguity id assigned to this split arc.
    pub ambiguity_id: SidereonRtkId,
    /// First epoch index covered by this arc.
    pub start_epoch_index: usize,
    /// Last epoch index covered by this arc.
    pub end_epoch_index: usize,
    /// Number of epochs covered by this arc.
    pub n_epochs: usize,
}

/// Static RTK arc driver configuration, mirroring
/// sidereon_core::rtk_filter::RtkStaticArcConfig. The arc field carries the raw
/// arc setup; the option fields carry the float, fixed, and residual-validation
/// settings for the single float+fixed baseline solve over the arc.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkStaticArcConfig {
    /// Raw RTK arc setup.
    pub arc: SidereonRtkArcConfig,
    /// Float solve options.
    pub float_options: SidereonRtkFloatOptions,
    /// Fixed solve options.
    pub fixed_options: SidereonRtkFixedOptions,
    /// Residual validation options.
    pub residual_options: SidereonRtkResidualValidationOptions,
}

/// One RINEX code/carrier pair used to build a single-frequency RTK arc.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexSignalPair {
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Null-terminated RINEX code observable, e.g. C1C.
    pub code_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated RINEX carrier observable, e.g. L1C.
    pub phase_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
}

/// Options for building single-frequency RTK arcs from paired RINEX OBS files.
/// Initialize with sidereon_rtk_rinex_arc_options_init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexArcOptions {
    /// Optional array of signal pairs. A zero count uses the GPS C1C/L1C default.
    pub signal_pairs: *const SidereonRtkRinexSignalPair,
    /// Number of signal pairs.
    pub signal_pair_count: usize,
    /// Whether max_epochs carries a value.
    pub has_max_epochs: bool,
    /// Optional cap on base epochs considered, in file order.
    pub max_epochs: usize,
    /// Minimum common satellites with observations and ephemeris in an epoch.
    pub min_common_satellites: usize,
    /// Fill prediction_time_s in generated arc epochs.
    pub include_prediction_time: bool,
}

/// One RINEX dual-frequency code/carrier selection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexDualSignalPair {
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Null-terminated band-1 code observable, e.g. C1C.
    pub code1_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated band-1 carrier observable, e.g. L1C.
    pub phase1_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated band-2 code observable, e.g. C2W.
    pub code2_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Null-terminated band-2 carrier observable, e.g. L2W.
    pub phase2_observable: [c_char; RINEX_OBS_CODE_C_BYTES],
}

/// Options for building dual-frequency RTK arcs from paired RINEX OBS files.
/// Initialize with sidereon_rtk_rinex_dual_arc_options_init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexDualArcOptions {
    /// Optional array of signal pairs. A zero count uses GPS C1C/L1C + C2W/L2W.
    pub signal_pairs: *const SidereonRtkRinexDualSignalPair,
    /// Number of signal pairs.
    pub signal_pair_count: usize,
    /// Whether max_epochs carries a value.
    pub has_max_epochs: bool,
    /// Optional cap on base epochs considered, in file order.
    pub max_epochs: usize,
    /// Minimum common satellites with observations and ephemeris in an epoch.
    pub min_common_satellites: usize,
    /// Fill prediction_time_s in generated arc epochs.
    pub include_prediction_time: bool,
}

/// Static RTK solve config for paired raw RINEX OBS plus SP3. Initialize with
/// sidereon_rtk_rinex_static_baseline_config_init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexStaticBaselineConfig {
    /// Base-station ECEF position (metres).
    pub base_m: [f64; 3],
    /// RINEX arc-build options.
    pub arc_options: SidereonRtkRinexArcOptions,
    /// Reference-selection mode, a SidereonRtkArcReferenceMode value.
    pub reference_mode: u32,
    /// Fixed reference satellite id token for the Satellite mode (NULL otherwise).
    pub reference_satellite: *const c_char,
    /// Per-constellation references for the PerSystem mode.
    pub reference_per_system: *const SidereonRtkArcReferenceEntry,
    /// Number of per-system references.
    pub reference_per_system_count: usize,
    /// Measurement model.
    pub model: SidereonRtkMeasurementModel,
    /// Baseline prior sigma (metres).
    pub baseline_prior_sigma_m: f64,
    /// Ambiguity prior sigma (metres).
    pub ambiguity_prior_sigma_m: f64,
    /// Initial rover-minus-base ECEF baseline (metres).
    pub initial_baseline_m: [f64; 3],
    /// Sequential-update controls used by the static driver.
    pub update_options: SidereonRtkArcUpdateOptions,
    /// Optional preprocessing chained before the static solve.
    pub preprocessing: SidereonRtkArcPreprocessing,
    /// Float solve options.
    pub float_options: SidereonRtkFloatOptions,
    /// Fixed solve options.
    pub fixed_options: SidereonRtkFixedOptions,
    /// Residual validation options.
    pub residual_options: SidereonRtkResidualValidationOptions,
}

/// Static dual-frequency wide-lane fixed RTK solve config for paired raw RINEX
/// OBS plus SP3. Initialize with sidereon_rtk_rinex_wide_lane_fixed_config_init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkRinexWideLaneFixedConfig {
    /// Base-station ECEF position (metres).
    pub base_m: [f64; 3],
    /// RINEX dual-frequency arc-build options.
    pub arc_options: SidereonRtkRinexDualArcOptions,
    /// Reference-selection mode, a SidereonRtkArcReferenceMode value.
    pub reference_mode: u32,
    /// Fixed reference satellite id token for the Satellite mode (NULL otherwise).
    pub reference_satellite: *const c_char,
    /// Per-constellation references for the PerSystem mode.
    pub reference_per_system: *const SidereonRtkArcReferenceEntry,
    /// Number of per-system references.
    pub reference_per_system_count: usize,
    /// Measurement model for the final narrow-lane solve.
    pub model: SidereonRtkMeasurementModel,
    /// Baseline prior sigma (metres).
    pub baseline_prior_sigma_m: f64,
    /// Ambiguity prior sigma (metres).
    pub ambiguity_prior_sigma_m: f64,
    /// Initial rover-minus-base ECEF baseline (metres).
    pub initial_baseline_m: [f64; 3],
    /// Sequential-update controls used by the static driver.
    pub update_options: SidereonRtkArcUpdateOptions,
    /// Float solve options.
    pub float_options: SidereonRtkFloatOptions,
    /// Fixed solve options.
    pub fixed_options: SidereonRtkFixedOptions,
    /// Residual validation options.
    pub residual_options: SidereonRtkResidualValidationOptions,
    /// Apply the core troposphere setup before the ionosphere-free combination.
    pub apply_troposphere: bool,
}

/// One receiver's dual-frequency code/carrier observation for RTK wide-lane and
/// ionosphere-free arc setup.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkDualFrequencyObservation {
    /// Ambiguity id for this satellite arc.
    pub ambiguity_id: *const c_char,
    /// Band-1 pseudorange in metres.
    pub p1_m: f64,
    /// Band-2 pseudorange in metres.
    pub p2_m: f64,
    /// Band-1 carrier phase in cycles.
    pub phi1_cycles: f64,
    /// Band-2 carrier phase in cycles.
    pub phi2_cycles: f64,
    /// Band-1 carrier frequency in Hz.
    pub f1_hz: f64,
    /// Band-2 carrier frequency in Hz.
    pub f2_hz: f64,
    /// Whether lli1 carries a value.
    pub has_lli1: bool,
    /// Band-1 loss-of-lock indicator when has_lli1 is true.
    pub lli1: i64,
    /// Whether lli2 carries a value.
    pub has_lli2: bool,
    /// Band-2 loss-of-lock indicator when has_lli2 is true.
    pub lli2: i64,
}

/// Paired base/rover dual-frequency observation for one satellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkDualFrequencySatelliteObservation {
    /// Physical satellite id token, e.g. "G05".
    pub sat_id: *const c_char,
    /// Base receiver observation.
    pub base: SidereonRtkDualFrequencyObservation,
    /// Rover receiver observation.
    pub rover: SidereonRtkDualFrequencyObservation,
}

/// One dual-frequency RTK arc epoch for wide-lane fixing and ionosphere-free
/// setup.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkDualFrequencyArcEpoch {
    /// Split Julian day whole part.
    pub jd_whole: f64,
    /// Split Julian day fractional part.
    pub jd_fraction: f64,
    /// Optional deterministic epoch sort key. NULL means absent.
    pub epoch_sort_key: *const c_char,
    /// Whether gap_time_s carries a value.
    pub has_gap_time_s: bool,
    /// Comparable gap coordinate in seconds when has_gap_time_s is true.
    pub gap_time_s: f64,
    /// Pointer to observation_count paired satellite observations.
    pub observations: *const SidereonRtkDualFrequencySatelliteObservation,
    /// Number of paired satellite observations.
    pub observation_count: usize,
    /// Shared receive-time satellite ECEF positions (metres).
    pub satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of shared positions.
    pub satellite_position_count: usize,
    /// Transmit-time base satellite ECEF positions; empty defaults to shared.
    pub base_satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of base positions.
    pub base_satellite_position_count: usize,
    /// Transmit-time rover satellite ECEF positions; empty defaults to shared.
    pub rover_satellite_positions: *const SidereonRtkArcPositionEntry,
    /// Number of rover positions.
    pub rover_satellite_position_count: usize,
    /// Whether an optional rover ECEF velocity is present.
    pub has_velocity_mps: bool,
    /// Rover ECEF velocity (metres/second) for downstream prediction.
    pub velocity_mps: [f64; 3],
    /// Whether an optional epoch time coordinate is present.
    pub has_prediction_time: bool,
    /// Epoch time coordinate (seconds) for downstream prediction.
    pub prediction_time_s: f64,
}

/// Dual-frequency cycle-slip preprocessing config for a wide-lane RTK arc.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkDualCycleSlipConfig {
    /// Cycle-slip policy, a SidereonRtkCycleSlipPolicy value.
    pub policy: u32,
    /// Cycle-slip detector options.
    pub options: SidereonCycleSlipOptions,
}

/// Melbourne-Wubbena wide-lane integer estimation controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkWideLaneOptions {
    /// Minimum epochs required per ambiguity fragment.
    pub min_epochs: usize,
    /// Integer acceptance tolerance in cycles.
    pub tolerance_cycles: f64,
    /// Drop short fragments instead of failing.
    pub skip_short_fragments: bool,
}

/// Wide-lane RTK arc driver configuration, mirroring
/// sidereon_core::rtk_filter::RtkWideLaneArcConfig.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkWideLaneArcConfig {
    /// Base-station ECEF position (metres).
    pub base_m: [f64; 3],
    /// Reference-selection mode, a SidereonRtkArcReferenceMode value.
    pub reference_mode: u32,
    /// Fixed reference satellite id token for the Satellite mode (NULL otherwise).
    pub reference_satellite: *const c_char,
    /// Per-constellation references for the PerSystem mode.
    pub reference_per_system: *const SidereonRtkArcReferenceEntry,
    /// Number of per-system references.
    pub reference_per_system_count: usize,
    /// Wide-lane estimation controls.
    pub options: SidereonRtkWideLaneOptions,
    /// Whether cycle_slip carries a value.
    pub has_cycle_slip: bool,
    /// Optional dual-frequency cycle-slip preprocessing.
    pub cycle_slip: SidereonRtkDualCycleSlipConfig,
}

/// Ionosphere-free RTK arc setup configuration, mirroring
/// sidereon_core::rtk_filter::RtkIonosphereFreeArcConfig.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkIonosphereFreeArcConfig {
    /// Base-station ECEF position (metres).
    pub base_m: [f64; 3],
    /// Initial rover-minus-base ECEF baseline (metres).
    pub initial_baseline_m: [f64; 3],
    /// Reference-selection mode, a SidereonRtkArcReferenceMode value.
    pub reference_mode: u32,
    /// Fixed reference satellite id token for the Satellite mode (NULL otherwise).
    pub reference_satellite: *const c_char,
    /// Per-constellation references for the PerSystem mode.
    pub reference_per_system: *const SidereonRtkArcReferenceEntry,
    /// Number of per-system references.
    pub reference_per_system_count: usize,
    /// Apply the core troposphere setup before the ionosphere-free combination.
    pub apply_troposphere: bool,
}

/// One fixed wide-lane ambiguity result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkWideLaneCycle {
    /// Ambiguity id.
    pub id: SidereonRtkId,
    /// Fixed wide-lane cycles.
    pub cycles: i64,
}

/// One RTK ambiguity-id keyed floating value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkMapValue {
    /// Ambiguity id.
    pub id: SidereonRtkId,
    /// Map value.
    pub value: f64,
}

/// One ambiguity-id to satellite-token output row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkAmbiguitySatelliteOut {
    /// Ambiguity id.
    pub id: SidereonRtkId,
    /// Physical satellite id token.
    pub sat_id: SidereonSatelliteToken,
}

/// One output RTK arc observation with fixed-size string storage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcObservationOut {
    /// Physical satellite id token.
    pub sat_id: SidereonSatelliteToken,
    /// Ambiguity id.
    pub ambiguity_id: SidereonRtkId,
    /// Code pseudorange (metres).
    pub code_m: f64,
    /// Carrier phase range (metres).
    pub phase_m: f64,
    /// Whether the optional loss-of-lock indicator is present.
    pub has_lli: bool,
    /// Loss-of-lock indicator when has_lli is true.
    pub lli: i64,
}

/// One output satellite-id keyed ECEF position entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcPositionOut {
    /// Satellite id token.
    pub id: SidereonSatelliteToken,
    /// Satellite ECEF position (metres).
    pub pos: [f64; 3],
}

/// One ionosphere-free output epoch's array lengths and optional fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkArcEpochOutMetadata {
    /// Number of base observations.
    pub base_count: usize,
    /// Number of rover observations.
    pub rover_count: usize,
    /// Number of shared positions.
    pub satellite_position_count: usize,
    /// Number of base positions.
    pub base_satellite_position_count: usize,
    /// Number of rover positions.
    pub rover_satellite_position_count: usize,
    /// Whether velocity_mps carries a value.
    pub has_velocity_mps: bool,
    /// Rover ECEF velocity (metres/second) when present.
    pub velocity_mps: [f64; 3],
    /// Whether prediction_time_s carries a value.
    pub has_prediction_time: bool,
    /// Epoch time coordinate (seconds) when present.
    pub prediction_time_s: f64,
}

/// A solved sequential RTK arc. Opaque to C. Create with sidereon_solve_rtk_arc;
/// read with the sidereon_rtk_arc_solution_* accessors; release with
/// sidereon_rtk_arc_solution_free.
pub struct SidereonRtkArcSolution {
    pub(crate) inner: RtkArcSolution,
}

/// A solved static RTK arc. Opaque to C. Create with
/// sidereon_solve_static_rtk_arc; read with the
/// sidereon_rtk_static_arc_solution_* accessors; release with
/// sidereon_rtk_static_arc_solution_free.
pub struct SidereonRtkStaticArcSolution {
    pub(crate) inner: RtkStaticArcSolution,
}

/// A wide-lane fixed RTK arc. Opaque to C. Create with
/// sidereon_fix_wide_lane_rtk_arc; read with the
/// sidereon_rtk_wide_lane_arc_solution_* accessors; release with
/// sidereon_rtk_wide_lane_arc_solution_free.
pub struct SidereonRtkWideLaneArcSolution {
    pub(crate) inner: RtkWideLaneArcSolution,
}

/// An ionosphere-free RTK arc setup. Opaque to C. Create with
/// sidereon_prepare_ionosphere_free_rtk_arc; read with the
/// sidereon_rtk_ionosphere_free_arc_solution_* accessors; release with
/// sidereon_rtk_ionosphere_free_arc_solution_free.
pub struct SidereonRtkIonosphereFreeArcSolution {
    pub(crate) inner: RtkIonosphereFreeArcSolution,
}

/// A static dual-frequency wide-lane fixed RTK solution built from RINEX OBS.
/// Create with sidereon_solve_wide_lane_fixed_rinex_rtk_baseline; read with the
/// sidereon_rtk_wide_lane_fixed_rinex_solution_* accessors; release with
/// sidereon_rtk_wide_lane_fixed_rinex_solution_free.
pub struct SidereonRtkWideLaneFixedRinexSolution {
    pub(crate) inner: RtkWideLaneFixedStaticArcSolution,
}

/// Metadata for the combined wide-lane fixed RINEX RTK solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtkWideLaneFixedRinexMetadata {
    /// True when at least one wide-lane ambiguity was fixed and used downstream.
    pub wide_lane_fixed: bool,
    /// Number of fixed wide-lane ambiguity rows.
    pub wide_lane_ambiguity_count: usize,
    /// Number of satellites dropped by dual-frequency cycle-slip preprocessing.
    pub dropped_cycle_slip_sat_count: usize,
    /// Number of split cycle-slip arcs.
    pub split_cycle_slip_arc_count: usize,
}

/// Selected solve mode for a static reference-station coordinate.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStaticReferenceStationMode {
    /// Code-DGNSS corrected pseudoranges stacked in a static solve.
    CodeDgnss = 0,
    /// Carrier RTK float baseline added to the reference coordinate.
    CarrierFloat = 1,
    /// Carrier RTK fixed baseline added to the reference coordinate.
    CarrierFixed = 2,
}

/// Fix status label for a static reference-station coordinate.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStaticReferenceFixStatus {
    /// Code-DGNSS solution.
    CodeDgnss = 0,
    /// Carrier RTK float solution.
    CarrierFloat = 1,
    /// Carrier RTK fixed solution.
    CarrierFixed = 2,
}

/// Status for one enabled static reference-station mode.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStaticReferenceModeStatus {
    /// The mode solved.
    Solved = 0,
    /// The mode failed.
    Failed = 1,
}

/// Static reference-station RINEX solve config. Initialize with
/// sidereon_static_reference_station_rinex_config_init. The carrier field uses
/// the same options as sidereon_solve_static_rinex_rtk_baseline; its base_m is
/// ignored and reference_position_m is used as the known reference coordinate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticReferenceStationRinexConfig {
    /// Known reference-station ECEF coordinate in metres.
    pub reference_position_m: [f64; 3],
    /// Enable the code-DGNSS static mode.
    pub enable_code_dgnss: bool,
    /// Enable the carrier RTK static mode.
    pub enable_carrier_rtk: bool,
    /// Include geodetic coordinates in the selected result.
    pub with_geodetic: bool,
    /// Carrier RTK options, used only when enable_carrier_rtk is true.
    pub carrier: SidereonRtkRinexStaticBaselineConfig,
}

/// Static reference-station solve metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticReferenceStationMetadata {
    /// Selected solve mode, a SidereonStaticReferenceStationMode value.
    pub mode: u32,
    /// Reported fix status, a SidereonStaticReferenceFixStatus value.
    pub fix_status: u32,
    /// Whether geodetic carries a value.
    pub has_geodetic: bool,
    /// Geodetic coordinate when requested.
    pub geodetic: SidereonGeodetic,
    /// Baseline length, rover minus reference, metres.
    pub baseline_m: f64,
    /// Whether a code-DGNSS nested solution is present.
    pub has_code_solution: bool,
    /// Whether a carrier RTK nested solution is present.
    pub has_carrier_solution: bool,
    /// Number of selected-mode diagnostic rows.
    pub diagnostic_count: usize,
    /// Number of per-mode attempt reports.
    pub mode_report_count: usize,
    /// Carrier integer status when a carrier solution is present.
    pub carrier_integer_status: SidereonRtkIntegerStatus,
    /// Whether carrier_integer_ratio carries a value.
    pub has_carrier_integer_ratio: bool,
    /// Carrier integer ratio when present.
    pub carrier_integer_ratio: f64,
    /// Number of code-DGNSS diagnostic rows when present.
    pub code_diagnostic_count: usize,
    /// Number of carrier diagnostic rows when present.
    pub carrier_diagnostic_count: usize,
}

/// Per-epoch diagnostic row from a static reference-station solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticReferenceEpochDiagnostic {
    /// Mode that produced this diagnostic row.
    pub mode: u32,
    /// Epoch index in the assembled mode input.
    pub epoch_index: usize,
    /// Number of used satellites.
    pub used_satellite_count: usize,
    /// Number of rejected satellites.
    pub rejected_satellite_count: usize,
    /// Whether code_residual_rms_m carries a value.
    pub has_code_residual_rms_m: bool,
    /// Code residual RMS in metres.
    pub code_residual_rms_m: f64,
    /// Whether phase_residual_rms_m carries a value.
    pub has_phase_residual_rms_m: bool,
    /// Carrier residual RMS in metres.
    pub phase_residual_rms_m: f64,
    /// Whether residual_rms_m carries a value.
    pub has_residual_rms_m: bool,
    /// Total unweighted residual RMS in metres.
    pub residual_rms_m: f64,
}

/// Per-mode attempt report from a static reference-station solve.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStaticReferenceModeReport {
    /// Attempted mode.
    pub mode: u32,
    /// Attempt status, a SidereonStaticReferenceModeStatus value.
    pub status: u32,
    /// Number of solved epochs.
    pub used_epochs: usize,
    /// Number of skipped raw RINEX epochs.
    pub skipped_epochs: usize,
    /// Number of measurements used by the final solve.
    pub used_measurements: usize,
    /// Whether a failure string exists for this mode.
    pub has_error: bool,
}

/// A solved static reference-station coordinate. Create with
/// sidereon_solve_static_reference_station_rinex; read with
/// sidereon_static_reference_station_solution_* accessors; release with
/// sidereon_static_reference_station_solution_free.
pub struct SidereonStaticReferenceStationSolution {
    pub(crate) inner: StaticReferenceStationSolution,
}

/// Initialize SidereonRtkArcUpdateOptions with the engine RTK defaults.
///
/// Safety: options must point to a writable SidereonRtkArcUpdateOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_update_options_init(
    options: *mut SidereonRtkArcUpdateOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_update_options_init",
        SidereonStatus::Panic,
        || {
            let options = c_try!(require_out(
                options,
                "sidereon_rtk_arc_update_options_init",
                "options"
            ));
            *options = SidereonRtkArcUpdateOptions {
                hold_sigma_m: RTK_AMBIGUITY_TOL_M,
                position_tol_m: RTK_POSITION_TOL_M,
                ambiguity_tol_m: RTK_AMBIGUITY_TOL_M,
                max_iterations: RTK_MAX_ITERATIONS,
                process_noise_baseline_sigma_m: 0.0,
                dynamics_velocity_propagated: false,
                float_only_systems: ptr::null(),
                float_only_system_count: 0,
                report_residuals: false,
                has_ar_arming_sigma_m: false,
                ar_arming_sigma_m: 0.0,
                ratio_threshold: RTK_RATIO_THRESHOLD,
                receiver_antenna: ptr::null(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Initialize RINEX single-frequency RTK arc options with GPS C1C/L1C defaults.
///
/// Safety: options must point to a writable SidereonRtkRinexArcOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_rinex_arc_options_init(
    options: *mut SidereonRtkRinexArcOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_rinex_arc_options_init",
        SidereonStatus::Panic,
        || {
            let options = c_try!(require_out(
                options,
                "sidereon_rtk_rinex_arc_options_init",
                "options"
            ));
            *options = default_rtk_rinex_arc_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize RINEX dual-frequency RTK arc options with GPS L1/L2 defaults.
///
/// Safety: options must point to a writable SidereonRtkRinexDualArcOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_rinex_dual_arc_options_init(
    options: *mut SidereonRtkRinexDualArcOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_rinex_dual_arc_options_init",
        SidereonStatus::Panic,
        || {
            let options = c_try!(require_out(
                options,
                "sidereon_rtk_rinex_dual_arc_options_init",
                "options"
            ));
            *options = default_rtk_rinex_dual_arc_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize static RINEX RTK baseline config with engine defaults.
///
/// Safety: config must point to a writable SidereonRtkRinexStaticBaselineConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_rinex_static_baseline_config_init(
    config: *mut SidereonRtkRinexStaticBaselineConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_rinex_static_baseline_config_init",
        SidereonStatus::Panic,
        || {
            let config = c_try!(require_out(
                config,
                "sidereon_rtk_rinex_static_baseline_config_init",
                "config"
            ));
            *config = default_rtk_rinex_static_baseline_config();
            SidereonStatus::Ok
        },
    )
}

/// Initialize dual-frequency wide-lane fixed RINEX RTK config with defaults.
///
/// Safety: config must point to a writable SidereonRtkRinexWideLaneFixedConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_rinex_wide_lane_fixed_config_init(
    config: *mut SidereonRtkRinexWideLaneFixedConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_rinex_wide_lane_fixed_config_init",
        SidereonStatus::Panic,
        || {
            let config = c_try!(require_out(
                config,
                "sidereon_rtk_rinex_wide_lane_fixed_config_init",
                "config"
            ));
            *config = default_rtk_rinex_wide_lane_fixed_config();
            SidereonStatus::Ok
        },
    )
}

/// Initialize static reference-station RINEX config with engine defaults.
///
/// Safety: config must point to a writable
/// SidereonStaticReferenceStationRinexConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_rinex_config_init(
    config: *mut SidereonStaticReferenceStationRinexConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_rinex_config_init",
        SidereonStatus::Panic,
        || {
            let config = c_try!(require_out(
                config,
                "sidereon_static_reference_station_rinex_config_init",
                "config"
            ));
            *config = default_static_reference_station_rinex_config();
            SidereonStatus::Ok
        },
    )
}

/// Copy the static-arc float rover-minus-base ECEF baseline into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_float_baseline_ecef(
    solution: *const SidereonRtkStaticArcSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_float_baseline_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_float_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_static_arc_solution_float_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.float_solution.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the static-arc fixed rover-minus-base ECEF baseline into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_fixed_baseline_ecef(
    solution: *const SidereonRtkStaticArcSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_fixed_baseline_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_fixed_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_static_arc_solution_fixed_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.fixed_solution.fixed_solution.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc float metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to a
/// SidereonRtkFloatMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_float_metadata(
    solution: *const SidereonRtkStaticArcSolution,
    out_metadata: *mut SidereonRtkFloatMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_float_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_static_arc_solution_float_metadata",
                "out_metadata"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_float_metadata",
                "solution"
            ));
            *out_metadata = rtk_float_metadata(&solution.inner.float_solution);
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc fixed metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to a
/// SidereonRtkFixedMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_fixed_metadata(
    solution: *const SidereonRtkStaticArcSolution,
    out_metadata: *mut SidereonRtkFixedMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_fixed_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_static_arc_solution_fixed_metadata",
                "out_metadata"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_fixed_metadata",
                "solution"
            ));
            *out_metadata = rtk_fixed_metadata(&solution.inner.fixed_solution);
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc geometry observability and covariance-validation diagnostics
/// into *out_geometry_quality.
///
/// Safety: solution is a live handle; out_geometry_quality points to a
/// SidereonGeometryQuality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_geometry_quality(
    solution: *const SidereonRtkStaticArcSolution,
    out_geometry_quality: *mut SidereonGeometryQuality,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_geometry_quality",
        SidereonStatus::Panic,
        || {
            let out_geometry_quality = c_try!(require_out(
                out_geometry_quality,
                "sidereon_rtk_static_arc_solution_geometry_quality",
                "out_geometry_quality"
            ));
            *out_geometry_quality = empty_geometry_quality();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_geometry_quality",
                "solution"
            ));
            *out_geometry_quality = geometry_quality_to_c(&solution.inner.geometry_quality);
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc float ambiguity estimates in metres. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkAmbiguity or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_float_ambiguities(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_float_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_float_ambiguities",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_float_ambiguities",
                "solution"
            ));
            let rows = rtk_ambiguities_to_c(&solution.inner.float_solution.ambiguities_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_float_ambiguities",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc fixed-solve free ambiguity estimates in metres. Variable-
/// length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkAmbiguity or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_fixed_free_ambiguities(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_fixed_free_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_fixed_free_ambiguities",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_fixed_free_ambiguities",
                "solution"
            ));
            let rows = rtk_ambiguities_to_c(
                &solution
                    .inner
                    .fixed_solution
                    .fixed_solution
                    .free_ambiguities_m,
            );
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_fixed_free_ambiguities",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc fixed integer ambiguities. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkFixedAmbiguity or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_fixed_ambiguities(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkFixedAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_fixed_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_fixed_ambiguities",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_fixed_ambiguities",
                "solution"
            ));
            let rows = c_try!(rtk_fixed_ambiguities_to_c(
                "sidereon_rtk_static_arc_solution_fixed_ambiguities",
                &solution.inner.fixed_solution,
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_fixed_ambiguities",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc ambiguity ids. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkId or NULL
/// when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_ambiguity_ids(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkId,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_ambiguity_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_ambiguity_ids",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_ambiguity_ids",
                "solution"
            ));
            let rows: Vec<SidereonRtkId> = solution
                .inner
                .ambiguity_ids
                .iter()
                .map(|id| rtk_id_token(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_ambiguity_ids",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc ambiguity-id to satellite-token rows. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkAmbiguitySatelliteOut or NULL when 0; out_written and out_required
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_ambiguity_satellites(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkAmbiguitySatelliteOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_ambiguity_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_ambiguity_satellites",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_ambiguity_satellites",
                "solution"
            ));
            let rows = rtk_ambiguity_satellites_to_c(&solution.inner.ambiguity_satellites);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_ambiguity_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc references. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcReferenceOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_references(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkArcReferenceOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_references",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_references",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_references",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcReferenceOut> = solution
                .inner
                .references
                .iter()
                .map(|(system, reference_id)| SidereonRtkArcReferenceOut {
                    system: rtk_id_token(system),
                    reference_id: rtk_id_token(reference_id),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_references",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc satellites dropped by preprocessing. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_dropped_sats(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_dropped_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_dropped_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_dropped_sats",
                "solution"
            ));
            let rows: Vec<SidereonSatelliteToken> = solution
                .inner
                .dropped_sats
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_dropped_sats",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc split cycle-slip metadata. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkArcSplitArc
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_split_cycle_slip_arcs(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonRtkArcSplitArc,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_split_cycle_slip_arcs",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_split_cycle_slip_arcs",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_split_cycle_slip_arcs",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcSplitArc> = solution
                .inner
                .split_cycle_slip_arcs
                .iter()
                .map(rtk_arc_split_arc_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_split_cycle_slip_arcs",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static-arc satellites masked by elevation preprocessing. Variable-length
/// output contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_elevation_masked_sats(
    solution: *const SidereonRtkStaticArcSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_static_arc_solution_elevation_masked_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_static_arc_solution_elevation_masked_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_static_arc_solution_elevation_masked_sats",
                "solution"
            ));
            let rows: Vec<SidereonSatelliteToken> = solution
                .inner
                .elevation_masked_sats
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_static_arc_solution_elevation_masked_sats",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Number of prepared epochs in the wide-lane arc solution.
///
/// Safety: solution is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_epoch_count(
    solution: *const SidereonRtkWideLaneArcSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rtk_wide_lane_arc_solution_epoch_count",
                "out_count"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_epoch_count",
                "solution"
            ));
            *out_count = solution.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane geometry observability and covariance-validation diagnostics
/// into *out_geometry_quality.
///
/// Safety: solution is a live handle; out_geometry_quality points to a
/// SidereonGeometryQuality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_geometry_quality(
    solution: *const SidereonRtkWideLaneArcSolution,
    out_geometry_quality: *mut SidereonGeometryQuality,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_geometry_quality",
        SidereonStatus::Panic,
        || {
            let out_geometry_quality = c_try!(require_out(
                out_geometry_quality,
                "sidereon_rtk_wide_lane_arc_solution_geometry_quality",
                "out_geometry_quality"
            ));
            *out_geometry_quality = empty_geometry_quality();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_geometry_quality",
                "solution"
            ));
            *out_geometry_quality = geometry_quality_to_c(&solution.inner.geometry_quality);
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane fixed cycles. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkWideLaneCycle or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles(
    solution: *const SidereonRtkWideLaneArcSolution,
    out: *mut SidereonRtkWideLaneCycle,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles",
                "solution"
            ));
            let rows = rtk_wide_lane_cycles_to_c(&solution.inner.wide_lane_cycles);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_wide_lane_arc_solution_wide_lane_cycles",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane arc references. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcReferenceOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_references(
    solution: *const SidereonRtkWideLaneArcSolution,
    out: *mut SidereonRtkArcReferenceOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_references",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_wide_lane_arc_solution_references",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_references",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcReferenceOut> = solution
                .inner
                .references
                .iter()
                .map(|(system, reference_id)| SidereonRtkArcReferenceOut {
                    system: rtk_id_token(system),
                    reference_id: rtk_id_token(reference_id),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_wide_lane_arc_solution_references",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane satellites dropped by preprocessing. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_dropped_sats(
    solution: *const SidereonRtkWideLaneArcSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_dropped_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_wide_lane_arc_solution_dropped_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_dropped_sats",
                "solution"
            ));
            let rows: Vec<SidereonSatelliteToken> = solution
                .inner
                .dropped_sats
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_wide_lane_arc_solution_dropped_sats",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane split cycle-slip metadata. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkArcSplitArc
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_split_cycle_slip_arcs(
    solution: *const SidereonRtkWideLaneArcSolution,
    out: *mut SidereonRtkArcSplitArc,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_arc_solution_split_cycle_slip_arcs",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_wide_lane_arc_solution_split_cycle_slip_arcs",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_arc_solution_split_cycle_slip_arcs",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcSplitArc> = solution
                .inner
                .split_cycle_slip_arcs
                .iter()
                .map(rtk_arc_split_arc_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_wide_lane_arc_solution_split_cycle_slip_arcs",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Number of converted epochs in the ionosphere-free arc solution.
///
/// Safety: solution is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_count(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_count",
                "out_count"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_count",
                "solution"
            ));
            *out_count = solution.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy ionosphere-free arc references. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcReferenceOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_references(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    out: *mut SidereonRtkArcReferenceOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_references",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_ionosphere_free_arc_solution_references",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_ionosphere_free_arc_solution_references",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcReferenceOut> = solution
                .inner
                .references
                .iter()
                .map(|(system, reference_id)| SidereonRtkArcReferenceOut {
                    system: rtk_id_token(system),
                    reference_id: rtk_id_token(reference_id),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_ionosphere_free_arc_solution_references",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy ionosphere-free carrier wavelengths. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkMapValue or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    out: *mut SidereonRtkMapValue,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m",
                "solution"
            ));
            let rows = rtk_map_values_to_c(&solution.inner.wavelengths_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_ionosphere_free_arc_solution_wavelengths_m",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy ionosphere-free code-to-phase offsets. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkMapValue or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_offsets_m(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    out: *mut SidereonRtkMapValue,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_offsets_m",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_ionosphere_free_arc_solution_offsets_m",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_ionosphere_free_arc_solution_offsets_m",
                "solution"
            ));
            let rows = rtk_map_values_to_c(&solution.inner.offsets_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_ionosphere_free_arc_solution_offsets_m",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one ionosphere-free output epoch's counts and optional fields into *out.
///
/// Safety: solution is a live handle; out points to
/// SidereonRtkArcEpochOutMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_metadata(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcEpochOutMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_metadata",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_metadata",
                "out"
            ));
            let epoch = c_try!(rtk_ionosphere_free_epoch_at(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_metadata",
                solution,
                index,
            ));
            *out = SidereonRtkArcEpochOutMetadata {
                base_count: epoch.base.len(),
                rover_count: epoch.rover.len(),
                satellite_position_count: epoch.satellite_positions_m.len(),
                base_satellite_position_count: epoch.base_satellite_positions_m.len(),
                rover_satellite_position_count: epoch.rover_satellite_positions_m.len(),
                has_velocity_mps: epoch.velocity_mps.is_some(),
                velocity_mps: epoch.velocity_mps.unwrap_or([0.0; 3]),
                has_prediction_time: epoch.prediction_time_s.is_some(),
                prediction_time_s: epoch.prediction_time_s.unwrap_or(0.0),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy one ionosphere-free output epoch's base observations. Variable-length
/// output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcObservationOut or NULL when 0; out_written and out_required
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_base_observations(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcObservationOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_base_observations",
        SidereonStatus::Panic,
        || {
            copy_ionosphere_free_epoch_observations(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_base_observations",
                solution,
                index,
                false,
                RtkVariableOut {
                    out,
                    len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy one ionosphere-free output epoch's rover observations. Variable-length
/// output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcObservationOut or NULL when 0; out_written and out_required
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_observations(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcObservationOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_observations",
        SidereonStatus::Panic,
        || {
            copy_ionosphere_free_epoch_observations(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_observations",
                solution,
                index,
                true,
                RtkVariableOut {
                    out,
                    len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy one ionosphere-free output epoch's shared satellite positions. Variable-
/// length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcPositionOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_satellite_positions(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcPositionOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_satellite_positions",
        SidereonStatus::Panic,
        || {
            copy_ionosphere_free_epoch_positions(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_satellite_positions",
                solution,
                index,
                0,
                RtkVariableOut {
                    out,
                    len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy one ionosphere-free output epoch's base satellite positions. Variable-
/// length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcPositionOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_base_satellite_positions(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcPositionOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_base_satellite_positions",
        SidereonStatus::Panic,
        || {
            copy_ionosphere_free_epoch_positions(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_base_satellite_positions",
                solution,
                index,
                1,
                RtkVariableOut {
                    out,
                    len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy one ionosphere-free output epoch's rover satellite positions. Variable-
/// length output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonRtkArcPositionOut or NULL when 0; out_written and out_required point
/// to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_satellite_positions(
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    out: *mut SidereonRtkArcPositionOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_satellite_positions",
        SidereonStatus::Panic,
        || {
            copy_ionosphere_free_epoch_positions(
                "sidereon_rtk_ionosphere_free_arc_solution_epoch_rover_satellite_positions",
                solution,
                index,
                2,
                RtkVariableOut {
                    out,
                    len,
                    out_written,
                    out_required,
                },
            )
        },
    )
}

/// Copy the combined wide-lane fixed RINEX RTK float baseline into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_float_baseline_ecef(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_float_baseline_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_float_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_wide_lane_fixed_rinex_solution_float_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.solution.float_solution.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the combined wide-lane fixed RINEX RTK fixed baseline into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_baseline_ecef(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_baseline_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution
                    .inner
                    .solution
                    .fixed_solution
                    .fixed_solution
                    .baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy combined wide-lane fixed RINEX RTK float metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to a
/// SidereonRtkFloatMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_float_metadata(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out_metadata: *mut SidereonRtkFloatMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_float_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_float_metadata",
                "out_metadata"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_float_metadata",
                "solution"
            ));
            *out_metadata = rtk_float_metadata(&solution.inner.solution.float_solution);
            SidereonStatus::Ok
        },
    )
}

/// Copy combined wide-lane fixed RINEX RTK fixed metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to a
/// SidereonRtkFixedMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_metadata(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out_metadata: *mut SidereonRtkFixedMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_metadata",
                "out_metadata"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_fixed_metadata",
                "solution"
            ));
            *out_metadata = rtk_fixed_metadata(&solution.inner.solution.fixed_solution);
            SidereonStatus::Ok
        },
    )
}

/// Copy combined wide-lane fixed RINEX RTK metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to
/// SidereonRtkWideLaneFixedRinexMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_metadata(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out_metadata: *mut SidereonRtkWideLaneFixedRinexMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_metadata",
                "out_metadata"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_metadata",
                "solution"
            ));
            *out_metadata = rtk_wide_lane_fixed_rinex_metadata(&solution.inner.metadata);
            SidereonStatus::Ok
        },
    )
}

/// Copy wide-lane fixed ambiguity cycles from the combined RINEX RTK solution.
/// Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkWideLaneCycle
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles(
    solution: *const SidereonRtkWideLaneFixedRinexSolution,
    out: *mut SidereonRtkWideLaneCycle,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles",
                "solution"
            ));
            let rows =
                rtk_wide_lane_cycles_to_c(&solution.inner.metadata.wide_lane_ambiguities_cycles);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_wide_lane_fixed_rinex_solution_wide_lane_cycles",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static reference-station metadata into *out_metadata.
///
/// Safety: solution is a live handle; out_metadata points to
/// SidereonStaticReferenceStationMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_metadata(
    solution: *const SidereonStaticReferenceStationSolution,
    out_metadata: *mut SidereonStaticReferenceStationMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_static_reference_station_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = empty_static_reference_metadata();
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_metadata",
                "solution"
            ));
            *out_metadata = static_reference_metadata(&solution.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the selected static reference-station ECEF coordinate into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_position_ecef(
    solution: *const SidereonStaticReferenceStationSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_position_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_position_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_static_reference_station_solution_position_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.position.as_array(),
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the selected rover-minus-reference ECEF baseline into out_xyz.
///
/// Safety: solution is a live handle; out_xyz points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_baseline_ecef(
    solution: *const SidereonStaticReferenceStationSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_baseline_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_baseline_ecef",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_static_reference_station_solution_baseline_ecef",
                "out_xyz",
                out_xyz,
                len,
                &solution.inner.baseline_vector_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the selected ECEF covariance into out_cov in row-major order.
///
/// Safety: solution is a live handle; out_cov points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_covariance_ecef(
    solution: *const SidereonStaticReferenceStationSolution,
    out_cov: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_covariance_ecef",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_covariance_ecef",
                "solution"
            ));
            let values = flatten_mat3(solution.inner.covariance.position_ecef_m2);
            c_try!(copy_exact_f64s(
                "sidereon_static_reference_station_solution_covariance_ecef",
                "out_cov",
                out_cov,
                len,
                &values,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the selected ENU covariance into out_cov in row-major order.
///
/// Safety: solution is a live handle; out_cov points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_covariance_enu(
    solution: *const SidereonStaticReferenceStationSolution,
    out_cov: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_covariance_enu",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_covariance_enu",
                "solution"
            ));
            let values = flatten_mat3(solution.inner.covariance.position_enu_m2);
            c_try!(copy_exact_f64s(
                "sidereon_static_reference_station_solution_covariance_enu",
                "out_cov",
                out_cov,
                len,
                &values,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy selected-mode static reference-station diagnostic rows. Variable-length
/// output contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonStaticReferenceEpochDiagnostic or NULL when 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_diagnostics(
    solution: *const SidereonStaticReferenceStationSolution,
    out: *mut SidereonStaticReferenceEpochDiagnostic,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_diagnostics",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_static_reference_station_solution_diagnostics",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_diagnostics",
                "solution"
            ));
            let rows: Vec<SidereonStaticReferenceEpochDiagnostic> = solution
                .inner
                .diagnostics
                .iter()
                .map(static_reference_diagnostic_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_reference_station_solution_diagnostics",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy static reference-station per-mode reports. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len
/// SidereonStaticReferenceModeReport or NULL when 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_mode_reports(
    solution: *const SidereonStaticReferenceStationSolution,
    out: *mut SidereonStaticReferenceModeReport,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_static_reference_station_solution_mode_reports",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_static_reference_station_solution_mode_reports",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_static_reference_station_solution_mode_reports",
                "solution"
            ));
            let rows: Vec<SidereonStaticReferenceModeReport> = solution
                .inner
                .mode_reports
                .iter()
                .map(static_reference_mode_report_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_static_reference_station_solution_mode_reports",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a static RTK arc solution handle. Passing NULL is a no-op.
///
/// Safety: solution is a handle from sidereon_solve_static_rtk_arc or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_static_arc_solution_free(
    solution: *mut SidereonRtkStaticArcSolution,
) {
    free_boxed(solution);
}

/// Release a wide-lane RTK arc solution handle. Passing NULL is a no-op.
///
/// Safety: solution is a handle from sidereon_fix_wide_lane_rtk_arc or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_arc_solution_free(
    solution: *mut SidereonRtkWideLaneArcSolution,
) {
    free_boxed(solution);
}

/// Release an ionosphere-free RTK arc solution handle. Passing NULL is a no-op.
///
/// Safety: solution is a handle from sidereon_prepare_ionosphere_free_rtk_arc or
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_ionosphere_free_arc_solution_free(
    solution: *mut SidereonRtkIonosphereFreeArcSolution,
) {
    free_boxed(solution);
}

/// Release a combined wide-lane fixed RINEX RTK solution handle. Passing NULL is
/// a no-op.
///
/// Safety: solution is a handle from
/// sidereon_solve_wide_lane_fixed_rinex_rtk_baseline or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_wide_lane_fixed_rinex_solution_free(
    solution: *mut SidereonRtkWideLaneFixedRinexSolution,
) {
    free_boxed(solution);
}

/// Release a static reference-station solution handle. Passing NULL is a no-op.
///
/// Safety: solution is a handle from
/// sidereon_solve_static_reference_station_rinex or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_static_reference_station_solution_free(
    solution: *mut SidereonStaticReferenceStationSolution,
) {
    free_boxed(solution);
}

/// Number of per-epoch solutions in an RTK arc solution.
///
/// Safety: solution is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_epoch_count(
    solution: *const SidereonRtkArcSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rtk_arc_solution_epoch_count",
                "out_count"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_epoch_count",
                "solution"
            ));
            *out_count = solution.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one epoch's reported solution metadata into *out.
///
/// Safety: solution is a live handle; out points to a SidereonRtkArcEpochMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_epoch_metadata(
    solution: *const SidereonRtkArcSolution,
    index: usize,
    out: *mut SidereonRtkArcEpochMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_epoch_metadata",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtk_arc_solution_epoch_metadata",
                "out"
            ));
            let epoch = c_try!(rtk_arc_epoch_at(
                "sidereon_rtk_arc_solution_epoch_metadata",
                solution,
                index
            ));
            *out = SidereonRtkArcEpochMetadata {
                reported_baseline_m: epoch.reported_baseline_m,
                float_baseline_m: epoch.float_baseline_m,
                integer_fixed: epoch.integer_fixed,
                integer_ratio: epoch.integer_ratio,
                newly_fixed_count: epoch.newly_fixed.len(),
                fixed_id_count: epoch.fixed_ids.len(),
                fixed_double_difference_count: epoch.fixed_double_difference_ids.len(),
                used_satellite_count: epoch.used_satellite_ids.len(),
                sd_ambiguity_count: epoch.sd_ambiguities_m.len(),
                residual_count: epoch.residuals.len(),
                has_search: epoch.search.is_some(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy one epoch's used satellite id tokens into out. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_epoch_used_satellites(
    solution: *const SidereonRtkArcSolution,
    index: usize,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_epoch_used_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_epoch_used_satellites",
                out_written,
                out_required
            ));
            let epoch = c_try!(rtk_arc_epoch_at(
                "sidereon_rtk_arc_solution_epoch_used_satellites",
                solution,
                index
            ));
            let rows: Vec<SidereonSatelliteToken> = epoch
                .used_satellite_ids
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_epoch_used_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one epoch's reported single-difference ambiguities (id, metres) in column
/// order into out. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkAmbiguity or
/// NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_epoch_sd_ambiguities(
    solution: *const SidereonRtkArcSolution,
    index: usize,
    out: *mut SidereonRtkAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_epoch_sd_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_epoch_sd_ambiguities",
                out_written,
                out_required
            ));
            let epoch = c_try!(rtk_arc_epoch_at(
                "sidereon_rtk_arc_solution_epoch_sd_ambiguities",
                solution,
                index
            ));
            let rows = rtk_ambiguities_to_c(&epoch.sd_ambiguities_m);
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_epoch_sd_ambiguities",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one of an epoch's single-difference id lists (selected by `which`, a
/// SidereonRtkArcEpochIdList value) into out. Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkId or NULL
/// when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_epoch_string_ids(
    solution: *const SidereonRtkArcSolution,
    index: usize,
    which: u32,
    out: *mut SidereonRtkId,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_epoch_string_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_epoch_string_ids",
                out_written,
                out_required
            ));
            let epoch = c_try!(rtk_arc_epoch_at(
                "sidereon_rtk_arc_solution_epoch_string_ids",
                solution,
                index
            ));
            let source = match which {
                v if v == SidereonRtkArcEpochIdList::NewlyFixed as u32 => &epoch.newly_fixed,
                v if v == SidereonRtkArcEpochIdList::FixedIds as u32 => &epoch.fixed_ids,
                v if v == SidereonRtkArcEpochIdList::FixedDoubleDifferenceIds as u32 => {
                    &epoch.fixed_double_difference_ids
                }
                _ => {
                    set_last_error(
                        "sidereon_rtk_arc_solution_epoch_string_ids: invalid id list code"
                            .to_owned(),
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            let rows: Vec<SidereonRtkId> = source.iter().map(|id| rtk_id_token(id)).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_epoch_string_ids",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-constellation reference single-difference ambiguity ids into out.
/// Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkArcReferenceOut
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_references(
    solution: *const SidereonRtkArcSolution,
    out: *mut SidereonRtkArcReferenceOut,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_references",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_references",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_references",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcReferenceOut> = solution
                .inner
                .references
                .iter()
                .map(|(system, reference_id)| SidereonRtkArcReferenceOut {
                    system: rtk_id_token(system),
                    reference_id: rtk_id_token(reference_id),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_references",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the satellites dropped by cycle-slip preprocessing into out.
/// Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_dropped_sats(
    solution: *const SidereonRtkArcSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_dropped_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_dropped_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_dropped_sats",
                "solution"
            ));
            let rows: Vec<SidereonSatelliteToken> = solution
                .inner
                .dropped_sats
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_dropped_sats",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the split cycle-slip arc metadata into out. Variable-length output
/// contract.
///
/// Safety: solution is a live handle; out points to len SidereonRtkArcSplitArc
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_split_cycle_slip_arcs(
    solution: *const SidereonRtkArcSolution,
    out: *mut SidereonRtkArcSplitArc,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_split_cycle_slip_arcs",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_split_cycle_slip_arcs",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_split_cycle_slip_arcs",
                "solution"
            ));
            let rows: Vec<SidereonRtkArcSplitArc> = solution
                .inner
                .split_cycle_slip_arcs
                .iter()
                .map(rtk_arc_split_arc_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_split_cycle_slip_arcs",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the satellites masked by elevation preprocessing into out.
/// Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len SidereonSatelliteToken
/// or NULL when 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_elevation_masked_sats(
    solution: *const SidereonRtkArcSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_elevation_masked_sats",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_elevation_masked_sats",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_elevation_masked_sats",
                "solution"
            ));
            let rows: Vec<SidereonSatelliteToken> = solution
                .inner
                .elevation_masked_sats
                .iter()
                .map(|id| satellite_token_from_text(id))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_elevation_masked_sats",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the final posterior measurement covariance into out as row-major doubles.
/// Variable-length output contract.
///
/// Safety: solution is a live handle; out points to len doubles or NULL when 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_measurement_covariance(
    solution: *const SidereonRtkArcSolution,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_measurement_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtk_arc_solution_measurement_covariance",
                out_written,
                out_required
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_measurement_covariance",
                "solution"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_rtk_arc_solution_measurement_covariance",
                "out",
                &solution.inner.measurement_covariance,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the final carried filter-state baseline (metres) into out_baseline.
///
/// Safety: solution is a live handle; out_baseline points to at least len doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_final_baseline(
    solution: *const SidereonRtkArcSolution,
    out_baseline: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_final_baseline",
        SidereonStatus::Panic,
        || {
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_final_baseline",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_rtk_arc_solution_final_baseline",
                "out_baseline",
                out_baseline,
                len,
                &solution.inner.final_state.baseline_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Number of epochs incorporated into the final carried filter state.
///
/// Safety: solution is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_final_epoch_count(
    solution: *const SidereonRtkArcSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtk_arc_solution_final_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rtk_arc_solution_final_epoch_count",
                "out_count"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_rtk_arc_solution_final_epoch_count",
                "solution"
            ));
            *out_count = solution.inner.final_state.epoch_count;
            SidereonStatus::Ok
        },
    )
}

/// Release an RTK arc solution handle. Passing NULL is a no-op.
///
/// Safety: solution is a handle from sidereon_solve_rtk_arc or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtk_arc_solution_free(solution: *mut SidereonRtkArcSolution) {
    free_boxed(solution);
}

// --- RTCM 3 from-scratch message construction (sidereon_core::rtcm) -----------
//
// The C binding already decodes RTCM 3 into a SidereonRtcmMessages handle and
// encodes a held message back to a body/frame. These builders create the same
// handle from caller-supplied fields, so the existing
// sidereon_rtcm_message_encode / sidereon_rtcm_message_to_frame and the typed
// accessors close the construct -> encode -> decode loop. Each builder wraps one
// constructed sidereon_core::rtcm::Message in a single-element list.

/// Solve one static RTK baseline directly from parsed RINEX OBS plus SP3. On
/// success writes a static-arc solution handle to *out_solution. Release it with
/// sidereon_rtk_static_arc_solution_free.
///
/// Safety: sp3, base_obs, rover_obs, and config must be live handles/pointers;
/// out_solution must point to storage for a SidereonRtkStaticArcSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_static_rinex_rtk_baseline(
    sp3: *const SidereonSp3,
    base_obs: *const SidereonRinexObs,
    rover_obs: *const SidereonRinexObs,
    config: *const SidereonRtkRinexStaticBaselineConfig,
    out_solution: *mut *mut SidereonRtkStaticArcSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_static_rinex_rtk_baseline",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_static_rinex_rtk_baseline",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_static_rinex_rtk_baseline",
                "sp3"
            ));
            let base_obs = c_try!(require_ref(
                base_obs,
                "sidereon_solve_static_rinex_rtk_baseline",
                "base_obs"
            ));
            let rover_obs = c_try!(require_ref(
                rover_obs,
                "sidereon_solve_static_rinex_rtk_baseline",
                "rover_obs"
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_solve_static_rinex_rtk_baseline",
                "config"
            ));
            let options = c_try!(rtk_rinex_arc_options_from_c(
                "sidereon_solve_static_rinex_rtk_baseline",
                &config.arc_options,
            ));
            let arc = match build_rinex_rtk_arc(
                &sp3.inner,
                &base_obs.inner,
                &rover_obs.inner,
                &options,
            ) {
                Ok(arc) => arc,
                Err(err) => {
                    return map_rtk_rinex_arc_error(
                        "sidereon_solve_static_rinex_rtk_baseline",
                        &err,
                    )
                }
            };
            let core_config = c_try!(rtk_rinex_static_config_from_c(
                "sidereon_solve_static_rinex_rtk_baseline",
                config,
                arc.wavelengths_m.clone(),
                arc.offsets_m.clone(),
            ));
            match solve_static_rtk_arc(&arc.epochs, &core_config) {
                Ok(inner) => {
                    write_boxed_handle(out_solution, SidereonRtkStaticArcSolution { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_rtk_static_arc_error("sidereon_solve_static_rinex_rtk_baseline", &err)
                }
            }
        },
    )
}

/// Solve a multi-epoch static reference-station coordinate from parsed RINEX
/// OBS plus SP3. Release the result with
/// sidereon_static_reference_station_solution_free.
///
/// Safety: sp3, reference_obs, rover_obs, and config must be live
/// handles/pointers; out_solution must point to storage for a
/// SidereonStaticReferenceStationSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_static_reference_station_rinex(
    sp3: *const SidereonSp3,
    reference_obs: *const SidereonRinexObs,
    rover_obs: *const SidereonRinexObs,
    config: *const SidereonStaticReferenceStationRinexConfig,
    out_solution: *mut *mut SidereonStaticReferenceStationSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_static_reference_station_rinex",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_static_reference_station_rinex",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_static_reference_station_rinex",
                "sp3"
            ));
            let reference_obs = c_try!(require_ref(
                reference_obs,
                "sidereon_solve_static_reference_station_rinex",
                "reference_obs"
            ));
            let rover_obs = c_try!(require_ref(
                rover_obs,
                "sidereon_solve_static_reference_station_rinex",
                "rover_obs"
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_solve_static_reference_station_rinex",
                "config"
            ));
            let code_options = if config.enable_code_dgnss {
                match RinexSppOptions::default_for(&rover_obs.inner) {
                    Ok(options) => Some(options),
                    Err(err) => {
                        set_last_error(format!(
                            "sidereon_solve_static_reference_station_rinex: {err}"
                        ));
                        return SidereonStatus::InvalidArgument;
                    }
                }
            } else {
                None
            };
            let carrier_options = if config.enable_carrier_rtk {
                Some(c_try!(static_reference_carrier_options_from_c(
                    "sidereon_solve_static_reference_station_rinex",
                    config,
                )))
            } else {
                None
            };
            let options = StaticReferenceStationRinexOptions {
                code_options,
                carrier_options,
                with_geodetic: config.with_geodetic,
            };
            match solve_static_reference_station_rinex(
                &sp3.inner,
                &reference_obs.inner,
                &rover_obs.inner,
                config.reference_position_m,
                &options,
            ) {
                Ok(inner) => {
                    write_boxed_handle(
                        out_solution,
                        SidereonStaticReferenceStationSolution { inner },
                    );
                    SidereonStatus::Ok
                }
                Err(err) => map_static_reference_error(
                    "sidereon_solve_static_reference_station_rinex",
                    &err,
                ),
            }
        },
    )
}

/// Solve one static dual-frequency wide-lane fixed RTK baseline directly from
/// parsed RINEX OBS plus SP3. Release the result with
/// sidereon_rtk_wide_lane_fixed_rinex_solution_free.
///
/// Safety: sp3, base_obs, rover_obs, and config must be live handles/pointers;
/// out_solution must point to storage for a SidereonRtkWideLaneFixedRinexSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_wide_lane_fixed_rinex_rtk_baseline(
    sp3: *const SidereonSp3,
    base_obs: *const SidereonRinexObs,
    rover_obs: *const SidereonRinexObs,
    config: *const SidereonRtkRinexWideLaneFixedConfig,
    out_solution: *mut *mut SidereonRtkWideLaneFixedRinexSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                "sp3"
            ));
            let base_obs = c_try!(require_ref(
                base_obs,
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                "base_obs"
            ));
            let rover_obs = c_try!(require_ref(
                rover_obs,
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                "rover_obs"
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                "config"
            ));
            let options = c_try!(rtk_rinex_dual_arc_options_from_c(
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                &config.arc_options,
            ));
            let arc = match build_dual_frequency_rinex_rtk_arc(
                &sp3.inner,
                &base_obs.inner,
                &rover_obs.inner,
                &options,
            ) {
                Ok(arc) => arc,
                Err(err) => {
                    return map_rtk_rinex_arc_error(
                        "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                        &err,
                    )
                }
            };
            let core_config = c_try!(rtk_rinex_wide_lane_fixed_config_from_c(
                "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                config,
            ));
            match solve_wide_lane_fixed_rtk_arc(&arc.epochs, &core_config) {
                Ok(RtkWideLaneFixedArcSolution::Static(inner)) => {
                    write_boxed_handle(
                        out_solution,
                        SidereonRtkWideLaneFixedRinexSolution { inner },
                    );
                    SidereonStatus::Ok
                }
                Ok(RtkWideLaneFixedArcSolution::Sequential(_)) => {
                    set_last_error(
                        "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline: expected static solution"
                            .to_string(),
                    );
                    SidereonStatus::InvalidArgument
                }
                Err(err) => map_rtk_wide_lane_fixed_arc_error(
                    "sidereon_solve_wide_lane_fixed_rinex_rtk_baseline",
                    &err,
                ),
            }
        },
    )
}

/// Fix Melbourne-Wubbena wide-lane ambiguities over a dual-frequency RTK arc. On
/// success writes a newly owned solution handle to *out_solution. Release it
/// with sidereon_rtk_wide_lane_arc_solution_free. Delegates to
/// sidereon_core::rtk_filter::fix_wide_lane_rtk_arc.
///
/// Safety: epochs points to epoch_count SidereonRtkDualFrequencyArcEpoch (or
/// NULL when 0); config points to a SidereonRtkWideLaneArcConfig; out_solution to
/// a SidereonRtkWideLaneArcSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fix_wide_lane_rtk_arc(
    epochs: *const SidereonRtkDualFrequencyArcEpoch,
    epoch_count: usize,
    config: *const SidereonRtkWideLaneArcConfig,
    out_solution: *mut *mut SidereonRtkWideLaneArcSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_fix_wide_lane_rtk_arc",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_fix_wide_lane_rtk_arc",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let config = c_try!(require_ref(
                config,
                "sidereon_fix_wide_lane_rtk_arc",
                "config"
            ));
            let core_config = c_try!(rtk_wide_lane_arc_config_from_c(
                "sidereon_fix_wide_lane_rtk_arc",
                config
            ));
            let core_epochs = c_try!(rtk_dual_frequency_arc_epochs_from_c(
                "sidereon_fix_wide_lane_rtk_arc",
                epochs,
                epoch_count,
            ));
            match fix_wide_lane_rtk_arc(&core_epochs, &core_config) {
                Ok(inner) => {
                    write_boxed_handle(out_solution, SidereonRtkWideLaneArcSolution { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_rtk_wide_lane_arc_error("sidereon_fix_wide_lane_rtk_arc", &err),
            }
        },
    )
}

/// Prepare an ionosphere-free single-frequency RTK arc from dual-frequency input
/// and fixed wide-lane cycles. On success writes a newly owned solution handle to
/// *out_solution. Release it with sidereon_rtk_ionosphere_free_arc_solution_free.
/// Delegates to sidereon_core::rtk_filter::prepare_ionosphere_free_rtk_arc.
///
/// Safety: epochs points to epoch_count SidereonRtkDualFrequencyArcEpoch (or
/// NULL when 0); wide_lane_cycles points to wide_lane_cycle_count rows (or NULL
/// when 0); config points to a SidereonRtkIonosphereFreeArcConfig; out_solution
/// to a SidereonRtkIonosphereFreeArcSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_prepare_ionosphere_free_rtk_arc(
    epochs: *const SidereonRtkDualFrequencyArcEpoch,
    epoch_count: usize,
    wide_lane_cycles: *const SidereonRtkWideLaneCycle,
    wide_lane_cycle_count: usize,
    config: *const SidereonRtkIonosphereFreeArcConfig,
    out_solution: *mut *mut SidereonRtkIonosphereFreeArcSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_prepare_ionosphere_free_rtk_arc",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_prepare_ionosphere_free_rtk_arc",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let config = c_try!(require_ref(
                config,
                "sidereon_prepare_ionosphere_free_rtk_arc",
                "config"
            ));
            let core_config = c_try!(rtk_ionosphere_free_arc_config_from_c(
                "sidereon_prepare_ionosphere_free_rtk_arc",
                config,
            ));
            let core_epochs = c_try!(rtk_dual_frequency_arc_epochs_from_c(
                "sidereon_prepare_ionosphere_free_rtk_arc",
                epochs,
                epoch_count,
            ));
            let core_cycles = c_try!(rtk_wide_lane_cycles_from_c(
                "sidereon_prepare_ionosphere_free_rtk_arc",
                wide_lane_cycles,
                wide_lane_cycle_count,
            ));
            match prepare_ionosphere_free_rtk_arc(&core_epochs, &core_cycles, &core_config) {
                Ok(inner) => {
                    write_boxed_handle(
                        out_solution,
                        SidereonRtkIonosphereFreeArcSolution { inner },
                    );
                    SidereonStatus::Ok
                }
                Err(err) => map_rtk_ionosphere_free_arc_error(
                    "sidereon_prepare_ionosphere_free_rtk_arc",
                    &err,
                ),
            }
        },
    )
}

fn rtk_fixed_metadata(solution: &ValidatedFixedBaselineSolution) -> SidereonRtkFixedMetadata {
    rtk_fixed_metadata_from_solution(
        &solution.fixed_solution,
        &solution.float_solution.geometry_quality,
    )
}

fn rtk_ambiguities_to_c(values: &[(String, f64)]) -> Vec<SidereonRtkAmbiguity> {
    values
        .iter()
        .map(|(id, value_m)| SidereonRtkAmbiguity {
            id: rtk_id_token(id),
            value_m: *value_m,
        })
        .collect()
}

fn rtk_fixed_ambiguities_to_c(
    fn_name: &str,
    solution: &ValidatedFixedBaselineSolution,
) -> Result<Vec<SidereonRtkFixedAmbiguity>, SidereonStatus> {
    let meters = solution
        .fixed_solution
        .fixed_ambiguities_m
        .iter()
        .map(|(id, value_m)| (id.as_str(), *value_m))
        .collect::<BTreeMap<_, _>>();
    rtk_fixed_ambiguity_rows_to_c(
        fn_name,
        &solution.fixed_solution.fixed_ambiguities_cycles,
        &meters,
    )
}

fn geocentric_enu(base_ecef_m: [f64; 3], baseline_ecef_m: [f64; 3]) -> [f64; 3] {
    let (north, east, up) = geocentric_neu_basis(base_ecef_m);
    [
        dot3(baseline_ecef_m, east),
        dot3(baseline_ecef_m, north),
        dot3(baseline_ecef_m, up),
    ]
}

fn default_rtk_measurement_model() -> SidereonRtkMeasurementModel {
    SidereonRtkMeasurementModel {
        code_sigma_m: RTK_CODE_SIGMA_M,
        phase_sigma_m: RTK_PHASE_SIGMA_M,
        sagnac: true,
        stochastic: SidereonRtkStochasticModel::Simple as u32,
        elevation_weighting: false,
    }
}

fn default_rtk_float_options() -> SidereonRtkFloatOptions {
    SidereonRtkFloatOptions {
        position_tol_m: RTK_POSITION_TOL_M,
        ambiguity_tol_m: RTK_AMBIGUITY_TOL_M,
        max_iterations: RTK_MAX_ITERATIONS,
    }
}

fn default_rtk_fixed_options() -> SidereonRtkFixedOptions {
    SidereonRtkFixedOptions {
        position_tol_m: RTK_POSITION_TOL_M,
        ambiguity_tol_m: RTK_AMBIGUITY_TOL_M,
        max_iterations: RTK_MAX_ITERATIONS,
        ratio_threshold: RTK_RATIO_THRESHOLD,
        partial_ambiguity_resolution: false,
        partial_min_ambiguities: RTK_PARTIAL_MIN_AMBIGUITIES,
    }
}

fn default_rtk_residual_options() -> SidereonRtkResidualValidationOptions {
    SidereonRtkResidualValidationOptions {
        threshold_sigma_enabled: false,
        threshold_sigma: 0.0,
        max_exclusions: 0,
    }
}

fn default_rtk_rinex_arc_options() -> SidereonRtkRinexArcOptions {
    SidereonRtkRinexArcOptions {
        signal_pairs: ptr::null(),
        signal_pair_count: 0,
        has_max_epochs: false,
        max_epochs: 0,
        min_common_satellites: 4,
        include_prediction_time: true,
    }
}

fn default_rtk_rinex_dual_arc_options() -> SidereonRtkRinexDualArcOptions {
    SidereonRtkRinexDualArcOptions {
        signal_pairs: ptr::null(),
        signal_pair_count: 0,
        has_max_epochs: false,
        max_epochs: 0,
        min_common_satellites: 4,
        include_prediction_time: true,
    }
}

fn default_rtk_arc_update_options_value() -> SidereonRtkArcUpdateOptions {
    SidereonRtkArcUpdateOptions {
        hold_sigma_m: RTK_AMBIGUITY_TOL_M,
        position_tol_m: RTK_POSITION_TOL_M,
        ambiguity_tol_m: RTK_AMBIGUITY_TOL_M,
        max_iterations: RTK_MAX_ITERATIONS,
        process_noise_baseline_sigma_m: 0.0,
        dynamics_velocity_propagated: false,
        float_only_systems: ptr::null(),
        float_only_system_count: 0,
        report_residuals: false,
        has_ar_arming_sigma_m: false,
        ar_arming_sigma_m: 0.0,
        ratio_threshold: RTK_RATIO_THRESHOLD,
        receiver_antenna: ptr::null(),
    }
}

fn default_rtk_arc_preprocessing() -> SidereonRtkArcPreprocessing {
    SidereonRtkArcPreprocessing {
        has_cycle_slip: false,
        cycle_slip: SidereonRtkCycleSlipPolicy::Error as u32,
        has_hatch_window_cap: false,
        hatch_window_cap: 0,
        has_elevation_mask_deg: false,
        elevation_mask_deg: 0.0,
    }
}

fn default_rtk_rinex_static_baseline_config() -> SidereonRtkRinexStaticBaselineConfig {
    SidereonRtkRinexStaticBaselineConfig {
        base_m: [0.0; 3],
        arc_options: default_rtk_rinex_arc_options(),
        reference_mode: SidereonRtkArcReferenceMode::Auto as u32,
        reference_satellite: ptr::null(),
        reference_per_system: ptr::null(),
        reference_per_system_count: 0,
        model: default_rtk_measurement_model(),
        baseline_prior_sigma_m: 30.0,
        ambiguity_prior_sigma_m: 30.0,
        initial_baseline_m: [0.0; 3],
        update_options: default_rtk_arc_update_options_value(),
        preprocessing: default_rtk_arc_preprocessing(),
        float_options: default_rtk_float_options(),
        fixed_options: default_rtk_fixed_options(),
        residual_options: default_rtk_residual_options(),
    }
}

fn default_rtk_rinex_wide_lane_fixed_config() -> SidereonRtkRinexWideLaneFixedConfig {
    SidereonRtkRinexWideLaneFixedConfig {
        base_m: [0.0; 3],
        arc_options: default_rtk_rinex_dual_arc_options(),
        reference_mode: SidereonRtkArcReferenceMode::Auto as u32,
        reference_satellite: ptr::null(),
        reference_per_system: ptr::null(),
        reference_per_system_count: 0,
        model: default_rtk_measurement_model(),
        baseline_prior_sigma_m: 30.0,
        ambiguity_prior_sigma_m: 30.0,
        initial_baseline_m: [0.0; 3],
        update_options: default_rtk_arc_update_options_value(),
        float_options: default_rtk_float_options(),
        fixed_options: default_rtk_fixed_options(),
        residual_options: default_rtk_residual_options(),
        apply_troposphere: true,
    }
}

fn default_static_reference_station_rinex_config() -> SidereonStaticReferenceStationRinexConfig {
    SidereonStaticReferenceStationRinexConfig {
        reference_position_m: [0.0; 3],
        enable_code_dgnss: true,
        enable_carrier_rtk: true,
        with_geodetic: true,
        carrier: default_rtk_rinex_static_baseline_config(),
    }
}

fn rtk_float_options_from_c(options: &SidereonRtkFloatOptions) -> FloatSolveOpts {
    FloatSolveOpts {
        position_tol_m: options.position_tol_m,
        ambiguity_tol_m: options.ambiguity_tol_m,
        max_iterations: options.max_iterations,
    }
}

fn rtk_fixed_options_from_c(options: &SidereonRtkFixedOptions) -> FixedSolveOpts {
    FixedSolveOpts {
        position_tol_m: options.position_tol_m,
        ambiguity_tol_m: options.ambiguity_tol_m,
        max_iterations: options.max_iterations,
        ratio_threshold: options.ratio_threshold,
        partial_ambiguity_resolution: options.partial_ambiguity_resolution,
        partial_min_ambiguities: options.partial_min_ambiguities,
    }
}

fn rtk_residual_options_from_c(
    options: &SidereonRtkResidualValidationOptions,
) -> ResidualValidationOpts {
    ResidualValidationOpts {
        threshold_sigma: options
            .threshold_sigma_enabled
            .then_some(options.threshold_sigma),
        max_exclusions: options.max_exclusions,
    }
}

fn rtk_validated_fixed_options_from_c(
    config: &SidereonRtkRinexStaticBaselineConfig,
) -> ValidatedFixedSolveOpts {
    ValidatedFixedSolveOpts {
        float: rtk_float_options_from_c(&config.float_options),
        fixed: rtk_fixed_options_from_c(&config.fixed_options),
        residual: rtk_residual_options_from_c(&config.residual_options),
    }
}

fn rtk_validated_fixed_options_from_wide_lane_c(
    config: &SidereonRtkRinexWideLaneFixedConfig,
) -> ValidatedFixedSolveOpts {
    ValidatedFixedSolveOpts {
        float: rtk_float_options_from_c(&config.float_options),
        fixed: rtk_fixed_options_from_c(&config.fixed_options),
        residual: rtk_residual_options_from_c(&config.residual_options),
    }
}

unsafe fn rtk_rinex_arc_options_from_c(
    fn_name: &str,
    options: &SidereonRtkRinexArcOptions,
) -> Result<RtkRinexArcOptions, SidereonStatus> {
    let mut out = if options.signal_pair_count == 0 {
        RtkRinexArcOptions::gps_l1_c()
    } else {
        let raw = require_slice(
            options.signal_pairs,
            options.signal_pair_count,
            fn_name,
            "arc_options.signal_pairs",
        )?;
        let mut pairs = Vec::with_capacity(raw.len());
        for (idx, pair) in raw.iter().enumerate() {
            pairs.push(RtkRinexSignalPair {
                system: gnss_system_from_c_code(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].system"),
                    pair.system,
                )?,
                code_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].code_observable"),
                    &pair.code_observable,
                )?,
                phase_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].phase_observable"),
                    &pair.phase_observable,
                )?,
            });
        }
        RtkRinexArcOptions {
            signal_pairs: pairs,
            max_epochs: None,
            min_common_satellites: options.min_common_satellites,
            include_prediction_time: options.include_prediction_time,
        }
    };
    out.max_epochs = options.has_max_epochs.then_some(options.max_epochs);
    out.min_common_satellites = options.min_common_satellites;
    out.include_prediction_time = options.include_prediction_time;
    Ok(out)
}

unsafe fn rtk_rinex_dual_arc_options_from_c(
    fn_name: &str,
    options: &SidereonRtkRinexDualArcOptions,
) -> Result<RtkRinexDualArcOptions, SidereonStatus> {
    let mut out = if options.signal_pair_count == 0 {
        RtkRinexDualArcOptions::gps_l1_l2_cw()
    } else {
        let raw = require_slice(
            options.signal_pairs,
            options.signal_pair_count,
            fn_name,
            "arc_options.signal_pairs",
        )?;
        let mut pairs = Vec::with_capacity(raw.len());
        for (idx, pair) in raw.iter().enumerate() {
            pairs.push(RtkRinexDualSignalPair {
                system: gnss_system_from_c_code(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].system"),
                    pair.system,
                )?,
                code1_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].code1_observable"),
                    &pair.code1_observable,
                )?,
                phase1_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].phase1_observable"),
                    &pair.phase1_observable,
                )?,
                code2_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].code2_observable"),
                    &pair.code2_observable,
                )?,
                phase2_observable: fixed_c_token_to_string(
                    fn_name,
                    &format!("arc_options.signal_pairs[{idx}].phase2_observable"),
                    &pair.phase2_observable,
                )?,
            });
        }
        RtkRinexDualArcOptions {
            signal_pairs: pairs,
            max_epochs: None,
            min_common_satellites: options.min_common_satellites,
            include_prediction_time: options.include_prediction_time,
        }
    };
    out.max_epochs = options.has_max_epochs.then_some(options.max_epochs);
    out.min_common_satellites = options.min_common_satellites;
    out.include_prediction_time = options.include_prediction_time;
    Ok(out)
}

unsafe fn rtk_rinex_static_config_from_c(
    fn_name: &str,
    config: &SidereonRtkRinexStaticBaselineConfig,
    wavelengths_m: BTreeMap<String, f64>,
    offsets_m: BTreeMap<String, f64>,
) -> Result<RtkStaticArcConfig, SidereonStatus> {
    Ok(RtkStaticArcConfig {
        arc: RtkArcConfig {
            base_m: config.base_m,
            reference: rtk_reference_selection_from_c(
                fn_name,
                config.reference_mode,
                config.reference_satellite,
                config.reference_per_system,
                config.reference_per_system_count,
            )?,
            model: rtk_model_from_c(fn_name, &config.model)?,
            baseline_prior_sigma_m: config.baseline_prior_sigma_m,
            ambiguity_prior_sigma_m: config.ambiguity_prior_sigma_m,
            initial_baseline_m: config.initial_baseline_m,
            wavelengths_m,
            offsets_m,
            update_opts: rtk_arc_update_opts_from_c(fn_name, &config.update_options)?,
            preprocessing: rtk_arc_preprocessing_from_c(fn_name, &config.preprocessing)?,
        },
        opts: rtk_validated_fixed_options_from_c(config),
    })
}

unsafe fn static_reference_carrier_options_from_c(
    fn_name: &str,
    config: &SidereonStaticReferenceStationRinexConfig,
) -> Result<StaticReferenceCarrierRinexOptions, SidereonStatus> {
    let mut carrier = config.carrier;
    carrier.base_m = config.reference_position_m;
    Ok(StaticReferenceCarrierRinexOptions {
        arc_options: rtk_rinex_arc_options_from_c(fn_name, &carrier.arc_options)?,
        static_config: rtk_rinex_static_config_from_c(
            fn_name,
            &carrier,
            BTreeMap::new(),
            BTreeMap::new(),
        )?,
    })
}

unsafe fn rtk_rinex_wide_lane_static_config_from_c(
    fn_name: &str,
    config: &SidereonRtkRinexWideLaneFixedConfig,
    reference: BaselineReferenceSelection,
) -> Result<RtkStaticArcConfig, SidereonStatus> {
    Ok(RtkStaticArcConfig {
        arc: RtkArcConfig {
            base_m: config.base_m,
            reference,
            model: rtk_model_from_c(fn_name, &config.model)?,
            baseline_prior_sigma_m: config.baseline_prior_sigma_m,
            ambiguity_prior_sigma_m: config.ambiguity_prior_sigma_m,
            initial_baseline_m: config.initial_baseline_m,
            wavelengths_m: BTreeMap::new(),
            offsets_m: BTreeMap::new(),
            update_opts: rtk_arc_update_opts_from_c(fn_name, &config.update_options)?,
            preprocessing: RtkArcPreprocessing::default(),
        },
        opts: rtk_validated_fixed_options_from_wide_lane_c(config),
    })
}

unsafe fn rtk_rinex_wide_lane_fixed_config_from_c(
    fn_name: &str,
    config: &SidereonRtkRinexWideLaneFixedConfig,
) -> Result<RtkWideLaneFixedArcConfig, SidereonStatus> {
    let reference = rtk_reference_selection_from_c(
        fn_name,
        config.reference_mode,
        config.reference_satellite,
        config.reference_per_system,
        config.reference_per_system_count,
    )?;
    Ok(RtkWideLaneFixedArcConfig {
        wide_lane: RtkWideLaneArcConfig {
            base_m: config.base_m,
            reference: reference.clone(),
            options: WideLaneOptions {
                min_epochs: 2,
                tolerance_cycles: 0.5,
                skip_short_fragments: false,
            },
            cycle_slip: Some(RtkDualCycleSlipConfig {
                policy: CycleSlipPolicy::DropSatellite,
                options: sidereon_core::carrier_phase::CycleSlipOptions::default(),
            }),
        },
        ionosphere_free: RtkIonosphereFreeArcConfig {
            base_m: config.base_m,
            initial_baseline_m: config.initial_baseline_m,
            reference: reference.clone(),
            apply_troposphere: config.apply_troposphere,
        },
        solve: RtkWideLaneFixedArcSolveConfig::Static(rtk_rinex_wide_lane_static_config_from_c(
            fn_name, config, reference,
        )?),
    })
}

unsafe fn rtk_dual_frequency_arc_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonRtkDualFrequencyArcEpoch,
    epoch_count: usize,
) -> Result<Vec<RtkDualFrequencyArcEpoch>, SidereonStatus> {
    let raw_epochs = require_slice(epochs, epoch_count, fn_name, "epochs")?;
    validate_element_count::<RtkDualFrequencyArcEpoch>(fn_name, "epoch_count", raw_epochs.len())?;
    let mut out = Vec::with_capacity(raw_epochs.len());
    for (idx, epoch) in raw_epochs.iter().enumerate() {
        let observations = rtk_dual_frequency_satellite_observations_from_c(
            fn_name,
            epoch.observations,
            epoch.observation_count,
            &format!("epochs[{idx}].observations"),
        )?;
        let satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.satellite_positions,
            epoch.satellite_position_count,
            &format!("epochs[{idx}].satellite_positions"),
        )?;
        let base_satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.base_satellite_positions,
            epoch.base_satellite_position_count,
            &format!("epochs[{idx}].base_satellite_positions"),
        )?;
        let rover_satellite_positions_m = rtk_arc_positions_from_c(
            fn_name,
            epoch.rover_satellite_positions,
            epoch.rover_satellite_position_count,
            &format!("epochs[{idx}].rover_satellite_positions"),
        )?;
        out.push(RtkDualFrequencyArcEpoch {
            jd_whole: epoch.jd_whole,
            jd_fraction: epoch.jd_fraction,
            epoch_sort_key: optional_bounded_c_string(
                fn_name,
                &format!("epochs[{idx}].epoch_sort_key"),
                epoch.epoch_sort_key,
                MAX_RTK_ID_BYTES,
            )?,
            gap_time_s: epoch.has_gap_time_s.then_some(epoch.gap_time_s),
            observations,
            satellite_positions_m,
            base_satellite_positions_m,
            rover_satellite_positions_m,
            velocity_mps: epoch.has_velocity_mps.then_some(epoch.velocity_mps),
            prediction_time_s: epoch.has_prediction_time.then_some(epoch.prediction_time_s),
        });
    }
    Ok(out)
}

unsafe fn rtk_wide_lane_arc_config_from_c(
    fn_name: &str,
    config: &SidereonRtkWideLaneArcConfig,
) -> Result<RtkWideLaneArcConfig, SidereonStatus> {
    Ok(RtkWideLaneArcConfig {
        base_m: config.base_m,
        reference: rtk_reference_selection_from_c(
            fn_name,
            config.reference_mode,
            config.reference_satellite,
            config.reference_per_system,
            config.reference_per_system_count,
        )?,
        options: WideLaneOptions {
            min_epochs: config.options.min_epochs,
            tolerance_cycles: config.options.tolerance_cycles,
            skip_short_fragments: config.options.skip_short_fragments,
        },
        cycle_slip: if config.has_cycle_slip {
            Some(rtk_dual_cycle_slip_config_from_c(
                fn_name,
                &config.cycle_slip,
            )?)
        } else {
            None
        },
    })
}

unsafe fn rtk_ionosphere_free_arc_config_from_c(
    fn_name: &str,
    config: &SidereonRtkIonosphereFreeArcConfig,
) -> Result<RtkIonosphereFreeArcConfig, SidereonStatus> {
    Ok(RtkIonosphereFreeArcConfig {
        base_m: config.base_m,
        initial_baseline_m: config.initial_baseline_m,
        reference: rtk_reference_selection_from_c(
            fn_name,
            config.reference_mode,
            config.reference_satellite,
            config.reference_per_system,
            config.reference_per_system_count,
        )?,
        apply_troposphere: config.apply_troposphere,
    })
}

unsafe fn rtk_wide_lane_cycles_from_c(
    fn_name: &str,
    cycles: *const SidereonRtkWideLaneCycle,
    cycle_count: usize,
) -> Result<BTreeMap<String, i64>, SidereonStatus> {
    let raw = require_slice(cycles, cycle_count, fn_name, "wide_lane_cycles")?;
    validate_element_count::<SidereonRtkWideLaneCycle>(fn_name, "wide_lane_cycles", raw.len())?;
    let mut out = BTreeMap::new();
    for (idx, row) in raw.iter().enumerate() {
        let id = rtk_id_from_token(fn_name, &format!("wide_lane_cycles[{idx}].id"), &row.id)?;
        insert_unique_string_key(fn_name, "wide_lane_cycles", idx, &mut out, id, row.cycles)?;
    }
    Ok(out)
}

fn rtk_map_values_to_c(values: &BTreeMap<String, f64>) -> Vec<SidereonRtkMapValue> {
    values
        .iter()
        .map(|(id, value)| SidereonRtkMapValue {
            id: rtk_id_token(id),
            value: *value,
        })
        .collect()
}

fn rtk_wide_lane_cycles_to_c(values: &BTreeMap<String, i64>) -> Vec<SidereonRtkWideLaneCycle> {
    values
        .iter()
        .map(|(id, cycles)| SidereonRtkWideLaneCycle {
            id: rtk_id_token(id),
            cycles: *cycles,
        })
        .collect()
}

fn rtk_ambiguity_satellites_to_c(
    values: &BTreeMap<String, String>,
) -> Vec<SidereonRtkAmbiguitySatelliteOut> {
    values
        .iter()
        .map(|(id, sat_id)| SidereonRtkAmbiguitySatelliteOut {
            id: rtk_id_token(id),
            sat_id: satellite_token_from_text(sat_id),
        })
        .collect()
}

fn static_reference_mode_to_c(
    mode: StaticReferenceStationMode,
) -> SidereonStaticReferenceStationMode {
    match mode {
        StaticReferenceStationMode::CodeDgnss => SidereonStaticReferenceStationMode::CodeDgnss,
        StaticReferenceStationMode::CarrierFloat => {
            SidereonStaticReferenceStationMode::CarrierFloat
        }
        StaticReferenceStationMode::CarrierFixed => {
            SidereonStaticReferenceStationMode::CarrierFixed
        }
    }
}

fn static_reference_fix_status_to_c(
    status: StaticReferenceFixStatus,
) -> SidereonStaticReferenceFixStatus {
    match status {
        StaticReferenceFixStatus::CodeDgnss => SidereonStaticReferenceFixStatus::CodeDgnss,
        StaticReferenceFixStatus::CarrierFloat => SidereonStaticReferenceFixStatus::CarrierFloat,
        StaticReferenceFixStatus::CarrierFixed => SidereonStaticReferenceFixStatus::CarrierFixed,
    }
}

fn static_reference_mode_status_to_c(
    status: StaticReferenceModeStatus,
) -> SidereonStaticReferenceModeStatus {
    match status {
        StaticReferenceModeStatus::Solved => SidereonStaticReferenceModeStatus::Solved,
        StaticReferenceModeStatus::Failed => SidereonStaticReferenceModeStatus::Failed,
    }
}

fn empty_static_reference_metadata() -> SidereonStaticReferenceStationMetadata {
    SidereonStaticReferenceStationMetadata {
        mode: SidereonStaticReferenceStationMode::CodeDgnss as u32,
        fix_status: SidereonStaticReferenceFixStatus::CodeDgnss as u32,
        has_geodetic: false,
        geodetic: empty_geodetic(),
        baseline_m: 0.0,
        has_code_solution: false,
        has_carrier_solution: false,
        diagnostic_count: 0,
        mode_report_count: 0,
        carrier_integer_status: SidereonRtkIntegerStatus::NotFixed,
        has_carrier_integer_ratio: false,
        carrier_integer_ratio: f64::NAN,
        code_diagnostic_count: 0,
        carrier_diagnostic_count: 0,
    }
}

fn static_reference_metadata(
    solution: &StaticReferenceStationSolution,
) -> SidereonStaticReferenceStationMetadata {
    let carrier = solution.carrier_solution.as_ref();
    SidereonStaticReferenceStationMetadata {
        mode: static_reference_mode_to_c(solution.mode) as u32,
        fix_status: static_reference_fix_status_to_c(solution.fix_status) as u32,
        has_geodetic: solution.geodetic.is_some(),
        geodetic: solution
            .geodetic
            .as_ref()
            .map(geodetic_to_c)
            .unwrap_or_else(empty_geodetic),
        baseline_m: solution.baseline_m,
        has_code_solution: solution.code_solution.is_some(),
        has_carrier_solution: carrier.is_some(),
        diagnostic_count: solution.diagnostics.len(),
        mode_report_count: solution.mode_reports.len(),
        carrier_integer_status: carrier
            .map(|inner| rtk_integer_status_to_c(inner.integer_status))
            .unwrap_or(SidereonRtkIntegerStatus::NotFixed),
        has_carrier_integer_ratio: carrier.and_then(|inner| inner.integer_ratio).is_some(),
        carrier_integer_ratio: none_to_nan(carrier.and_then(|inner| inner.integer_ratio)),
        code_diagnostic_count: solution
            .code_solution
            .as_ref()
            .map_or(0, |inner| inner.diagnostics.len()),
        carrier_diagnostic_count: carrier.map_or(0, |inner| inner.diagnostics.len()),
    }
}

fn static_reference_diagnostic_to_c(
    diagnostic: &StaticReferenceEpochDiagnostic,
) -> SidereonStaticReferenceEpochDiagnostic {
    SidereonStaticReferenceEpochDiagnostic {
        mode: static_reference_mode_to_c(diagnostic.mode) as u32,
        epoch_index: diagnostic.epoch_index,
        used_satellite_count: diagnostic.used_satellites.len(),
        rejected_satellite_count: diagnostic.rejected_satellite_count,
        has_code_residual_rms_m: diagnostic.code_residual_rms_m.is_some(),
        code_residual_rms_m: none_to_nan(diagnostic.code_residual_rms_m),
        has_phase_residual_rms_m: diagnostic.phase_residual_rms_m.is_some(),
        phase_residual_rms_m: none_to_nan(diagnostic.phase_residual_rms_m),
        has_residual_rms_m: diagnostic.residual_rms_m.is_some(),
        residual_rms_m: none_to_nan(diagnostic.residual_rms_m),
    }
}

fn static_reference_mode_report_to_c(
    report: &StaticReferenceModeReport,
) -> SidereonStaticReferenceModeReport {
    SidereonStaticReferenceModeReport {
        mode: static_reference_mode_to_c(report.mode) as u32,
        status: static_reference_mode_status_to_c(report.status) as u32,
        used_epochs: report.used_epochs,
        skipped_epochs: report.skipped_epochs,
        used_measurements: report.used_measurements,
        has_error: report.error.is_some(),
    }
}

fn map_static_reference_error(fn_name: &str, err: &StaticReferenceStationError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        StaticReferenceStationError::InvalidInput { .. }
        | StaticReferenceStationError::NoEnabledModes => SidereonStatus::InvalidArgument,
        StaticReferenceStationError::AllModesFailed { .. } => SidereonStatus::Solve,
    }
}

fn map_rtk_wide_lane_arc_error(fn_name: &str, err: &RtkWideLaneArcError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        RtkWideLaneArcError::EmptyEpochs
        | RtkWideLaneArcError::WideLane(WideLaneError::InvalidInput { .. }) => {
            SidereonStatus::InvalidArgument
        }
        _ => SidereonStatus::Solve,
    }
}

fn map_rtk_rinex_arc_error(
    fn_name: &str,
    err: &sidereon_core::rtk_filter::RtkRinexArcError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::rtk_filter::RtkRinexArcError::InvalidInput { .. }
        | sidereon_core::rtk_filter::RtkRinexArcError::NoSignalPairs
        | sidereon_core::rtk_filter::RtkRinexArcError::NoUsableEpochs
        | sidereon_core::rtk_filter::RtkRinexArcError::MissingFrequency { .. } => {
            SidereonStatus::InvalidArgument
        }
        sidereon_core::rtk_filter::RtkRinexArcError::Observation(_) => {
            SidereonStatus::InvalidArgument
        }
        sidereon_core::rtk_filter::RtkRinexArcError::Ephemeris { .. } => SidereonStatus::Solve,
    }
}

fn map_rtk_static_arc_error(fn_name: &str, err: &RtkStaticArcError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::Solve
}

fn map_rtk_wide_lane_fixed_arc_error(
    fn_name: &str,
    err: &RtkWideLaneFixedArcError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        RtkWideLaneFixedArcError::UnsupportedMultiGnss => SidereonStatus::InvalidArgument,
        RtkWideLaneFixedArcError::WideLane(inner) => map_rtk_wide_lane_arc_error(fn_name, inner),
        RtkWideLaneFixedArcError::IonosphereFree(inner) => {
            map_rtk_ionosphere_free_arc_error(fn_name, inner)
        }
        _ => SidereonStatus::Solve,
    }
}

fn map_rtk_ionosphere_free_arc_error(
    fn_name: &str,
    err: &RtkIonosphereFreeArcError,
) -> SidereonStatus {
    use sidereon_core::rtk_filter::IonosphereFreeBaselineError;
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        RtkIonosphereFreeArcError::EmptyEpochs
        | RtkIonosphereFreeArcError::IonosphereFree(
            IonosphereFreeBaselineError::InvalidInput { .. }
            | IonosphereFreeBaselineError::NoEpochs,
        ) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn rtk_wide_lane_fixed_rinex_metadata(
    metadata: &RtkWideLaneFixedArcMetadata,
) -> SidereonRtkWideLaneFixedRinexMetadata {
    SidereonRtkWideLaneFixedRinexMetadata {
        wide_lane_fixed: metadata.wide_lane_fixed,
        wide_lane_ambiguity_count: metadata.wide_lane_ambiguities_cycles.len(),
        dropped_cycle_slip_sat_count: metadata.dropped_cycle_slip_sats.len(),
        split_cycle_slip_arc_count: metadata.split_cycle_slip_arcs.len(),
    }
}

fn rtk_arc_split_arc_to_c(arc: &CycleSlipSplitArc) -> SidereonRtkArcSplitArc {
    SidereonRtkArcSplitArc {
        receiver: rtk_cycle_slip_receiver_to_c(arc.receiver) as u32,
        satellite_id: satellite_token_from_text(&arc.satellite_id),
        ambiguity_id: rtk_id_token(&arc.ambiguity_id),
        start_epoch_index: arc.start_epoch_index,
        end_epoch_index: arc.end_epoch_index,
        n_epochs: arc.n_epochs,
    }
}

unsafe fn copy_ionosphere_free_epoch_observations(
    fn_name: &str,
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    rover: bool,
    output: RtkVariableOut<SidereonRtkArcObservationOut>,
) -> SidereonStatus {
    c_try!(init_copy_counts(
        fn_name,
        output.out_written,
        output.out_required
    ));
    let epoch = c_try!(rtk_ionosphere_free_epoch_at(fn_name, solution, index));
    let source = if rover { &epoch.rover } else { &epoch.base };
    let rows = rtk_arc_observations_to_c(source);
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &rows,
        output.out,
        output.len,
        output.out_written,
        output.out_required,
    ));
    SidereonStatus::Ok
}

unsafe fn copy_ionosphere_free_epoch_positions(
    fn_name: &str,
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
    which: u32,
    output: RtkVariableOut<SidereonRtkArcPositionOut>,
) -> SidereonStatus {
    c_try!(init_copy_counts(
        fn_name,
        output.out_written,
        output.out_required
    ));
    let epoch = c_try!(rtk_ionosphere_free_epoch_at(fn_name, solution, index));
    let source = match which {
        0 => &epoch.satellite_positions_m,
        1 => &epoch.base_satellite_positions_m,
        _ => &epoch.rover_satellite_positions_m,
    };
    let rows = rtk_arc_positions_to_c(source);
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &rows,
        output.out,
        output.len,
        output.out_written,
        output.out_required,
    ));
    SidereonStatus::Ok
}

unsafe fn rtk_dual_frequency_satellite_observations_from_c(
    fn_name: &str,
    ptr: *const SidereonRtkDualFrequencySatelliteObservation,
    count: usize,
    arg_name: &str,
) -> Result<Vec<RtkDualFrequencySatelliteObservation>, SidereonStatus> {
    let raw = require_slice(ptr, count, fn_name, arg_name)?;
    validate_element_count::<RtkDualFrequencySatelliteObservation>(fn_name, arg_name, raw.len())?;
    let mut out = Vec::with_capacity(raw.len());
    for (idx, observation) in raw.iter().enumerate() {
        let satellite_id = parse_satellite_token(fn_name, observation.sat_id)?.to_string();
        let base = rtk_dual_frequency_observation_from_c(
            fn_name,
            &format!("{arg_name}[{idx}].base"),
            &observation.base,
        )?;
        let rover = rtk_dual_frequency_observation_from_c(
            fn_name,
            &format!("{arg_name}[{idx}].rover"),
            &observation.rover,
        )?;
        out.push(RtkDualFrequencySatelliteObservation {
            satellite_id,
            base,
            rover,
        });
    }
    Ok(out)
}

unsafe fn rtk_dual_cycle_slip_config_from_c(
    fn_name: &str,
    config: &SidereonRtkDualCycleSlipConfig,
) -> Result<RtkDualCycleSlipConfig, SidereonStatus> {
    Ok(RtkDualCycleSlipConfig {
        policy: rtk_cycle_slip_policy_from_c(fn_name, config.policy)?,
        options: cycle_slip_options_from_c(&config.options),
    })
}

fn rtk_id_from_token(
    fn_name: &str,
    arg_name: &str,
    token: &SidereonRtkId,
) -> Result<String, SidereonStatus> {
    fixed_c_token_to_string(fn_name, arg_name, &token.bytes)
}

fn rtk_arc_observations_to_c(values: &[RtkArcObservation]) -> Vec<SidereonRtkArcObservationOut> {
    values
        .iter()
        .map(|observation| SidereonRtkArcObservationOut {
            sat_id: satellite_token_from_text(&observation.satellite_id),
            ambiguity_id: rtk_id_token(&observation.ambiguity_id),
            code_m: observation.code_m,
            phase_m: observation.phase_m,
            has_lli: observation.lli.is_some(),
            lli: observation.lli.unwrap_or(0),
        })
        .collect()
}

fn rtk_arc_positions_to_c(values: &BTreeMap<String, [f64; 3]>) -> Vec<SidereonRtkArcPositionOut> {
    values
        .iter()
        .map(|(id, pos)| SidereonRtkArcPositionOut {
            id: satellite_token_from_text(id),
            pos: *pos,
        })
        .collect()
}

fn rtk_cycle_slip_receiver_to_c(receiver: CycleSlipReceiver) -> SidereonRtkCycleSlipReceiver {
    match receiver {
        CycleSlipReceiver::Base => SidereonRtkCycleSlipReceiver::Base,
        CycleSlipReceiver::Rover => SidereonRtkCycleSlipReceiver::Rover,
    }
}
