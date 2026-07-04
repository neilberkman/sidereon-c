use super::*;

/// The result of a PPP float solve. Opaque to C. Create with
/// sidereon_solve_ppp_float and release with sidereon_ppp_float_solution_free.
pub struct SidereonPppFloatSolution {
    pub(crate) inner: PppFloatSolutionInner,
}

/// The result of a PPP fixed solve. Opaque to C. Create with
/// sidereon_solve_ppp_fixed and release with sidereon_ppp_fixed_solution_free.
pub struct SidereonPppFixedSolution {
    pub(crate) inner: PppFixedSolutionInner,
}

/// Fixed-size null-terminated PPP ambiguity id storage. Values returned by
/// Sidereon are always null-terminated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppId {
    /// Null-terminated id bytes.
    pub bytes: [c_char; 65],
}

/// Terminal status of a PPP float or fixed least-squares solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppSolveStatus {
    /// State update tolerances were reached.
    StateTolerance = 0,
    /// Maximum iterations were reached.
    MaxIterations = 1,
}

/// Integer ambiguity-fix verdict for a PPP fixed solve.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppIntegerStatus {
    /// The ambiguity search accepted an integer fix.
    Fixed = 0,
    /// The ambiguity search rejected the integer fix.
    NotFixed = 1,
}

/// Civil timestamp for one PPP epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCivilDateTime {
    /// Calendar year.
    pub year: i32,
    /// Calendar month, 1 through 12.
    pub month: u8,
    /// Calendar day, 1 through 31.
    pub day: u8,
    /// Hour of day.
    pub hour: u8,
    /// Minute of hour.
    pub minute: u8,
    /// Seconds of minute.
    pub second: f64,
}

/// One ionosphere-free code/phase observation in a PPP epoch.
#[repr(C)]
pub struct SidereonPppObservation {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// Null-terminated ambiguity id. Split arcs may use ids like G08#2.
    pub ambiguity_id: *const c_char,
    /// Ionosphere-free code observable in meters.
    pub code_m: f64,
    /// Ionosphere-free carrier phase observable in meters.
    pub phase_m: f64,
    /// First raw carrier frequency in Hz, or 0 when not supplied.
    pub freq1_hz: f64,
    /// Second raw carrier frequency in Hz, or 0 when not supplied.
    pub freq2_hz: f64,
}

/// One static PPP epoch.
#[repr(C)]
pub struct SidereonPppEpoch {
    /// Civil timestamp used by the PPP correction model.
    pub civil: SidereonPppCivilDateTime,
    /// Julian date whole-day part.
    pub jd_whole: f64,
    /// Julian date fractional-day part.
    pub jd_fraction: f64,
    /// Receive time as J2000 seconds.
    pub t_rx_j2000_s: f64,
    /// Pointer to observation_count observations.
    pub observations: *const SidereonPppObservation,
    /// Number of observations in this epoch.
    pub observation_count: usize,
}

/// One PPP string-to-f64 map entry, used for ambiguity, wavelength, and offset
/// maps.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppFloatMapEntry {
    /// Null-terminated ambiguity id.
    pub id: *const c_char,
    /// Map value.
    pub value: f64,
}

/// Initial static-arc PPP state.
#[repr(C)]
pub struct SidereonPppFloatState {
    /// Initial receiver ECEF position in meters.
    pub position_m: [f64; 3],
    /// Pointer to clock_count receiver-clock values in meters.
    pub clocks_m: *const f64,
    /// Number of receiver-clock values. Must equal epoch_count.
    pub clock_count: usize,
    /// Pointer to ambiguity_count float ambiguity entries in meters.
    pub ambiguities_m: *const SidereonPppFloatMapEntry,
    /// Number of float ambiguity entries.
    pub ambiguity_count: usize,
    /// Initial zenith troposphere residual in meters.
    pub ztd_m: f64,
}

/// PPP measurement weights. Values are inverse sigmas matching the engine.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppMeasurementWeights {
    /// Code inverse sigma.
    pub code: f64,
    /// Carrier phase inverse sigma.
    pub phase: f64,
    /// Whether to use elevation weighting.
    pub elevation_weighting: bool,
}

/// Maximum number of VMF1 site-wise samples carried in
/// SidereonPppTroposphereOptions.vmf_samples. Mirrors the engine
/// VMF_SITE_MAX_SAMPLES (kept a literal so cbindgen emits a usable array bound;
/// the static assertion below guarantees it tracks the engine constant).
pub const SIDEREON_PPP_VMF_SITE_MAX_SAMPLES: usize = 8;

/// Tropospheric mapping-function selection for a PPP solve. Pass as a uint32_t
/// in SidereonPppTroposphereOptions.mapping.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppTropoMapping {
    /// Niell (1996) climatological mapping; needs no external data. vmf_samples
    /// is ignored.
    Niell = 0,
    /// VMF1 site-wise mapping driven by the vmf_samples a-coefficient series.
    Vmf1 = 1,
}

/// One VMF1 site-wise sample: the a coefficients at a single epoch. Used only
/// when SidereonPppTroposphereOptions.mapping is SIDEREON_PPP_TROPO_MAPPING_VMF1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppVmfSiteSample {
    /// Modified Julian date of the sample (VMF nodes are 00/06/12/18 UT).
    pub mjd: f64,
    /// Hydrostatic a coefficient from the VMF data product.
    pub ah: f64,
    /// Wet a coefficient from the VMF data product.
    pub aw: f64,
}

/// PPP troposphere controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppTroposphereOptions {
    /// Enable troposphere correction.
    pub enabled: bool,
    /// Estimate zenith troposphere residual as a state component.
    pub estimate_ztd: bool,
    /// Surface pressure in hPa.
    pub pressure_hpa: f64,
    /// Surface temperature in kelvin.
    pub temperature_k: f64,
    /// Relative humidity, 0 to 1.
    pub relative_humidity: f64,
    /// Mapping function, a SidereonPppTropoMapping value.
    pub mapping: u32,
    /// Number of valid entries in vmf_samples (1..=SIDEREON_PPP_VMF_SITE_MAX_SAMPLES);
    /// used only when mapping is VMF1. Samples must be strictly increasing in mjd
    /// with positive, finite a coefficients.
    pub vmf_sample_count: usize,
    /// VMF1 site-wise a-coefficient series, ascending in mjd.
    pub vmf_samples: [SidereonPppVmfSiteSample; SIDEREON_PPP_VMF_SITE_MAX_SAMPLES],
}

/// PPP iteration and convergence controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppFloatOptions {
    /// Maximum solver iterations.
    pub max_iterations: usize,
    /// Position update tolerance in meters.
    pub position_tolerance_m: f64,
    /// Clock update tolerance in meters.
    pub clock_tolerance_m: f64,
    /// Ambiguity update tolerance in meters.
    pub ambiguity_tolerance_m: f64,
    /// Zenith troposphere update tolerance in meters.
    pub ztd_tolerance_m: f64,
}

/// PPP receiver-antenna correction options for the ionosphere-free frequency
/// pair. Set SidereonPppRangeCorrections.receiver_antenna to NULL to disable.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppReceiverAntennaOptions {
    /// Null-terminated label for the first frequency calibration.
    pub freq1_label: *const c_char,
    /// First carrier frequency in Hz.
    pub freq1_hz: f64,
    /// First frequency receiver-antenna calibration.
    pub freq1: SidereonReceiverAntennaCalibration,
    /// Null-terminated label for the second frequency calibration.
    pub freq2_label: *const c_char,
    /// Second carrier frequency in Hz.
    pub freq2_hz: f64,
    /// Second frequency receiver-antenna calibration.
    pub freq2: SidereonReceiverAntennaCalibration,
}

/// One precise satellite-clock sample. Values are keyed by satellite and GPS
/// seconds, and the clock value is seconds.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppSatelliteClockRecord {
    /// Null-terminated satellite token, for example G08.
    pub sat_id: *const c_char,
    /// GPS seconds for this clock sample.
    pub gps_seconds: f64,
    /// Satellite clock value in seconds.
    pub clock_s: f64,
}

/// PPP range-correction controls. Tractable corrections are applied directly:
/// receiver_antenna, sat_clock_relativity, and satellite_clock_records. Tide,
/// phase windup, and satellite ANTEX corrections require precomputed tables
/// that this C ABI cannot yet represent; setting those flags returns
/// SIDEREON_STATUS_INVALID_ARGUMENT rather than silently ignoring them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppRangeCorrections {
    /// Optional receiver-antenna options, or NULL to disable.
    pub receiver_antenna: *const SidereonPppReceiverAntennaOptions,
    /// Enable relativistic satellite-clock correction.
    pub sat_clock_relativity: bool,
    /// Pointer to satellite_clock_record_count satellite-clock samples.
    pub satellite_clock_records: *const SidereonPppSatelliteClockRecord,
    /// Number of satellite-clock samples.
    pub satellite_clock_record_count: usize,
    /// Unsupported in this C ABI; true returns InvalidArgument.
    pub solid_earth_tide: bool,
    /// Unsupported in this C ABI; true returns InvalidArgument.
    pub phase_windup: bool,
    /// Unsupported in this C ABI; true returns InvalidArgument.
    pub satellite_antenna: bool,
}

/// Complete typed input bundle for a PPP float solve.
#[repr(C)]
pub struct SidereonPppFloatConfig {
    /// Pointer to epoch_count epochs.
    pub epochs: *const SidereonPppEpoch,
    /// Number of epochs.
    pub epoch_count: usize,
    /// Initial PPP state.
    pub initial_state: SidereonPppFloatState,
    /// Measurement weights.
    pub weights: SidereonPppMeasurementWeights,
    /// Troposphere controls.
    pub tropo: SidereonPppTroposphereOptions,
    /// Range-correction controls.
    pub corrections: SidereonPppRangeCorrections,
    /// Float solve options.
    pub options: SidereonPppFloatOptions,
    /// Enable residual screening.
    pub residual_screen: bool,
}

/// Integer ambiguity controls for PPP fixed solving.
#[repr(C)]
pub struct SidereonPppFixedAmbiguityOptions {
    /// Pointer to wavelength_count ambiguity wavelength entries.
    pub wavelengths_m: *const SidereonPppFloatMapEntry,
    /// Number of wavelength entries.
    pub wavelength_count: usize,
    /// Pointer to offset_count ambiguity offset entries.
    pub offsets_m: *const SidereonPppFloatMapEntry,
    /// Number of offset entries.
    pub offset_count: usize,
    /// Integer ratio-test threshold.
    pub ratio_threshold: f64,
}

/// Complete typed input bundle for a PPP fixed solve.
#[repr(C)]
pub struct SidereonPppFixedConfig {
    /// Pointer to epoch_count epochs.
    pub epochs: *const SidereonPppEpoch,
    /// Number of epochs.
    pub epoch_count: usize,
    /// Measurement weights.
    pub weights: SidereonPppMeasurementWeights,
    /// Troposphere controls.
    pub tropo: SidereonPppTroposphereOptions,
    /// Range-correction controls.
    pub corrections: SidereonPppRangeCorrections,
    /// Fixed re-solve options.
    pub options: SidereonPppFloatOptions,
    /// Integer ambiguity controls.
    pub ambiguity: SidereonPppFixedAmbiguityOptions,
}

/// One PPP float ambiguity estimate in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppAmbiguity {
    /// Ambiguity id.
    pub id: SidereonPppId,
    /// Ambiguity estimate in meters.
    pub value_m: f64,
}

/// One PPP fixed integer ambiguity estimate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppFixedAmbiguity {
    /// Ambiguity id.
    pub id: SidereonPppId,
    /// Fixed ambiguity in carrier cycles.
    pub cycles: i64,
    /// Fixed ambiguity in meters after wavelength and offset scaling.
    pub value_m: f64,
}

/// Summary scalars for a PPP float solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppFloatMetadata {
    /// Solver iterations.
    pub iterations: usize,
    /// Whether the solver converged.
    pub converged: bool,
    /// Terminal solve status.
    pub status: SidereonPppSolveStatus,
    /// Whether ztd_residual_m is present.
    pub has_ztd_residual_m: bool,
    /// Zenith troposphere residual in meters when present.
    pub ztd_residual_m: f64,
    /// Code residual RMS in meters.
    pub code_rms_m: f64,
    /// Carrier phase residual RMS in meters.
    pub phase_rms_m: f64,
    /// Weighted residual RMS in meters.
    pub weighted_rms_m: f64,
    /// Number of float ambiguity estimates.
    pub ambiguity_count: usize,
    /// Number of residual rows.
    pub residual_count: usize,
    /// Number of used satellite or ambiguity ids.
    pub used_sat_count: usize,
}

/// Summary scalars and integer-search metadata for a PPP fixed solution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppFixedMetadata {
    /// Solver iterations.
    pub iterations: usize,
    /// Whether the fixed re-solve converged.
    pub converged: bool,
    /// Terminal solve status.
    pub status: SidereonPppSolveStatus,
    /// Whether ztd_residual_m is present.
    pub has_ztd_residual_m: bool,
    /// Zenith troposphere residual in meters when present.
    pub ztd_residual_m: f64,
    /// Code residual RMS in meters.
    pub code_rms_m: f64,
    /// Carrier phase residual RMS in meters.
    pub phase_rms_m: f64,
    /// Weighted residual RMS in meters.
    pub weighted_rms_m: f64,
    /// Number of fixed integer ambiguities.
    pub fixed_ambiguity_count: usize,
    /// Number of residual rows.
    pub residual_count: usize,
    /// Number of used satellite or ambiguity ids.
    pub used_sat_count: usize,
    /// Integer ambiguity-fix verdict.
    pub integer_status: SidereonPppIntegerStatus,
    /// Integer ratio.
    pub integer_ratio: f64,
    /// Best integer-search score.
    pub integer_best_score: f64,
    /// Whether integer_second_best_score is present.
    pub has_integer_second_best_score: bool,
    /// Second-best integer-search score when present.
    pub integer_second_best_score: f64,
    /// Number of integer candidates evaluated by the search.
    pub integer_candidates: usize,
}

/// Initialize PPP measurement weights with engine binding defaults.
///
/// Safety: out_weights must point to a SidereonPppMeasurementWeights.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_measurement_weights_init(
    out_weights: *mut SidereonPppMeasurementWeights,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_measurement_weights_init",
        SidereonStatus::Panic,
        || {
            let out_weights = c_try!(require_out(
                out_weights,
                "sidereon_ppp_measurement_weights_init",
                "out_weights"
            ));
            *out_weights = default_ppp_measurement_weights();
            SidereonStatus::Ok
        },
    )
}

/// Initialize PPP troposphere options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonPppTroposphereOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_troposphere_options_init(
    out_options: *mut SidereonPppTroposphereOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_troposphere_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_ppp_troposphere_options_init",
                "out_options"
            ));
            *out_options = default_ppp_troposphere_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize PPP float solve options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonPppFloatOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_options_init(
    out_options: *mut SidereonPppFloatOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_ppp_float_options_init",
                "out_options"
            ));
            *out_options = default_ppp_float_options();
            SidereonStatus::Ok
        },
    )
}

/// Initialize PPP range corrections as disabled.
///
/// Safety: out_corrections must point to a SidereonPppRangeCorrections.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_range_corrections_init(
    out_corrections: *mut SidereonPppRangeCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_range_corrections_init",
        SidereonStatus::Panic,
        || {
            let out_corrections = c_try!(require_out(
                out_corrections,
                "sidereon_ppp_range_corrections_init",
                "out_corrections"
            ));
            *out_corrections = default_ppp_range_corrections();
            SidereonStatus::Ok
        },
    )
}

/// Initialize PPP fixed ambiguity options with engine binding defaults.
///
/// Safety: out_options must point to a SidereonPppFixedAmbiguityOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_ambiguity_options_init(
    out_options: *mut SidereonPppFixedAmbiguityOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_ambiguity_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_ppp_fixed_ambiguity_options_init",
                "out_options"
            ));
            *out_options = default_ppp_fixed_ambiguity_options();
            SidereonStatus::Ok
        },
    )
}

/// Copy the PPP float ECEF position into out_xyz.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_position(
    sol: *const SidereonPppFloatSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_solution_position",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_ppp_float_solution_position",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_float_solution_position",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_ppp_float_solution_position",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.position_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy PPP float metadata into *out_metadata.
///
/// Safety: sol must be a live solution handle; out_metadata must point to a
/// SidereonPppFloatMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_metadata(
    sol: *const SidereonPppFloatSolution,
    out_metadata: *mut SidereonPppFloatMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_ppp_float_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = SidereonPppFloatMetadata {
                iterations: 0,
                converged: false,
                status: SidereonPppSolveStatus::StateTolerance,
                has_ztd_residual_m: false,
                ztd_residual_m: 0.0,
                code_rms_m: 0.0,
                phase_rms_m: 0.0,
                weighted_rms_m: 0.0,
                ambiguity_count: 0,
                residual_count: 0,
                used_sat_count: 0,
            };
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_float_solution_metadata",
                "solution"
            ));
            *out_metadata = ppp_float_metadata(&sol.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy PPP float ambiguity estimates in meters. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_ambiguities(
    sol: *const SidereonPppFloatSolution,
    out: *mut SidereonPppAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_solution_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_float_solution_ambiguities",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_float_solution_ambiguities",
                "solution"
            ));
            let values = ppp_ambiguities_to_c(&sol.inner.ambiguities_m);
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_float_solution_ambiguities",
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

/// Copy used PPP ids from a PPP float solution into 65-byte SidereonPppId
/// tokens. Uses the variable-length output contract documented at the top of
/// the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_used_ids(
    sol: *const SidereonPppFloatSolution,
    out: *mut SidereonPppId,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_solution_used_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_float_solution_used_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_float_solution_used_ids",
                "solution"
            ));
            let values = c_try!(ppp_used_id_tokens(
                "sidereon_ppp_float_solution_used_ids",
                &sol.inner.used_sats
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_float_solution_used_ids",
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

/// Copy used satellite ids from a PPP float solution into 17-byte satellite
/// tokens. This legacy accessor returns InvalidArgument if any used PPP id is
/// longer than SidereonSatelliteToken can represent; use
/// sidereon_ppp_float_solution_used_ids for full-width PPP ambiguity ids. Uses
/// the variable-length output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_used_sat_ids(
    sol: *const SidereonPppFloatSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_float_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_float_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_float_solution_used_sat_ids",
                "solution"
            ));
            let values = c_try!(ppp_used_satellite_tokens(
                "sidereon_ppp_float_solution_used_sat_ids",
                &sol.inner.used_sats
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_float_solution_used_sat_ids",
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

/// Copy the PPP fixed ECEF position into out_xyz.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_position(
    sol: *const SidereonPppFixedSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_position",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_ppp_fixed_solution_position",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_position",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_ppp_fixed_solution_position",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.position_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the embedded PPP float ECEF position from a fixed solution into
/// out_xyz.
///
/// Safety: sol must be a live solution handle; out_xyz must point to at least
/// len writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_float_position(
    sol: *const SidereonPppFixedSolution,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_float_position",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_xyz,
                "sidereon_ppp_fixed_solution_float_position",
                "out_xyz"
            ));
            zero_f64_prefix(out_xyz, len, 3);
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_float_position",
                "solution"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_ppp_fixed_solution_float_position",
                "out_xyz",
                out_xyz,
                len,
                &sol.inner.float_solution.position_m,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy PPP fixed metadata into *out_metadata.
///
/// Safety: sol must be a live solution handle; out_metadata must point to a
/// SidereonPppFixedMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_metadata(
    sol: *const SidereonPppFixedSolution,
    out_metadata: *mut SidereonPppFixedMetadata,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_metadata",
        SidereonStatus::Panic,
        || {
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_ppp_fixed_solution_metadata",
                "out_metadata"
            ));
            *out_metadata = SidereonPppFixedMetadata {
                iterations: 0,
                converged: false,
                status: SidereonPppSolveStatus::StateTolerance,
                has_ztd_residual_m: false,
                ztd_residual_m: 0.0,
                code_rms_m: 0.0,
                phase_rms_m: 0.0,
                weighted_rms_m: 0.0,
                fixed_ambiguity_count: 0,
                residual_count: 0,
                used_sat_count: 0,
                integer_status: SidereonPppIntegerStatus::NotFixed,
                integer_ratio: 0.0,
                integer_best_score: 0.0,
                has_integer_second_best_score: false,
                integer_second_best_score: 0.0,
                integer_candidates: 0,
            };
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_metadata",
                "solution"
            ));
            *out_metadata = ppp_fixed_metadata(&sol.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy PPP fixed integer ambiguities. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_fixed_ambiguities(
    sol: *const SidereonPppFixedSolution,
    out: *mut SidereonPppFixedAmbiguity,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_fixed_ambiguities",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_fixed_solution_fixed_ambiguities",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_fixed_ambiguities",
                "solution"
            ));
            let values = c_try!(ppp_fixed_ambiguities_to_c(
                "sidereon_ppp_fixed_solution_fixed_ambiguities",
                &sol.inner
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_fixed_solution_fixed_ambiguities",
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

/// Copy used PPP ids from a PPP fixed solution into 65-byte SidereonPppId
/// tokens. Uses the variable-length output contract documented at the top of
/// the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_used_ids(
    sol: *const SidereonPppFixedSolution,
    out: *mut SidereonPppId,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_used_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_fixed_solution_used_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_used_ids",
                "solution"
            ));
            let values = c_try!(ppp_used_id_tokens(
                "sidereon_ppp_fixed_solution_used_ids",
                &sol.inner.used_sats
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_fixed_solution_used_ids",
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

/// Copy used satellite ids from a PPP fixed solution into 17-byte satellite
/// tokens. This legacy accessor returns InvalidArgument if any used PPP id is
/// longer than SidereonSatelliteToken can represent; use
/// sidereon_ppp_fixed_solution_used_ids for full-width PPP ambiguity ids. Uses
/// the variable-length output contract documented at the top of the header.
///
/// Safety: sol must be a live solution handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_used_sat_ids(
    sol: *const SidereonPppFixedSolution,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_fixed_solution_used_sat_ids",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_fixed_solution_used_sat_ids",
                out_written,
                out_required
            ));
            let sol = c_try!(require_ref(
                sol,
                "sidereon_ppp_fixed_solution_used_sat_ids",
                "solution"
            ));
            let values = c_try!(ppp_used_satellite_tokens(
                "sidereon_ppp_fixed_solution_used_sat_ids",
                &sol.inner.used_sats
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_fixed_solution_used_sat_ids",
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

/// Release a PPP float solution handle. Null is a no-op. A non-null handle must
/// come from sidereon_solve_ppp_float and must be freed exactly once with this
/// function.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_ppp_float.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_solution_free(sol: *mut SidereonPppFloatSolution) {
    ffi_boundary("sidereon_ppp_float_solution_free", (), || {
        free_boxed(sol);
    });
}

/// Release a PPP fixed solution handle. Null is a no-op. A non-null handle must
/// come from sidereon_solve_ppp_fixed and must be freed exactly once with this
/// function.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_ppp_fixed.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_fixed_solution_free(sol: *mut SidereonPppFixedSolution) {
    ffi_boundary("sidereon_ppp_fixed_solution_free", (), || {
        free_boxed(sol);
    });
}

/// Number of ocean-loading tidal constituents in a BLQ block (matches
/// sidereon_core::ppp_corrections::NUM_OCEAN_CONSTITUENTS).
pub const SIDEREON_PPP_OCEAN_CONSTITUENTS: usize = 11;

/// A UTC-like civil calendar instant, mirroring
/// sidereon_core::ppp_corrections::CivilDateTime.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCivilDateTime {
    /// Calendar year.
    pub year: i32,
    /// Month (1-12).
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Hour (0-23).
    pub hour: u8,
    /// Minute (0-59).
    pub minute: u8,
    /// Second (fractional).
    pub second: f64,
}

/// One satellite observation row for the PPP correction precompute, mirroring
/// sidereon_core::ppp_corrections::PppCorrectionObservation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCorrectionObservation {
    /// Null-terminated satellite token, for example G01.
    pub sat_id: *const c_char,
    /// Band-1 carrier frequency, Hz.
    pub freq1_hz: f64,
    /// Band-2 carrier frequency, Hz.
    pub freq2_hz: f64,
    /// Whether glonass_channel is present.
    pub has_glonass_channel: bool,
    /// GLONASS FDMA frequency channel, used when has_glonass_channel.
    pub glonass_channel: i8,
}

/// One receiver epoch and its visible satellite rows, mirroring
/// sidereon_core::ppp_corrections::PppCorrectionEpoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCorrectionEpoch {
    /// Civil UTC epoch.
    pub epoch: SidereonCivilDateTime,
    /// Receiver clock epoch in J2000 seconds.
    pub t_rx_j2000_s: f64,
    /// Pointer to observation_count SidereonPppCorrectionObservation.
    pub observations: *const SidereonPppCorrectionObservation,
    /// Number of observation rows.
    pub observation_count: usize,
}

/// Solid-Earth pole-tide options, mirroring
/// sidereon_core::ppp_corrections::PoleTideOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPoleTideOptions {
    /// IERS polar motion x of the date (arcsec).
    pub xp_arcsec: f64,
    /// IERS polar motion y of the date (arcsec).
    pub yp_arcsec: f64,
}

/// Ocean-loading BLQ coefficients, mirroring
/// sidereon_core::ppp_corrections::OceanLoadingBlq. Both arrays are
/// [3][SIDEREON_PPP_OCEAN_CONSTITUENTS]: row 0 radial, 1 west, 2 south.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOceanLoadingBlq {
    /// Constituent amplitudes (m).
    pub amplitude_m: [[f64; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
    /// Constituent Greenwich phase lags (degrees, positive lag).
    pub phase_deg: [[f64; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
}

/// One non-azimuthal PCV sample pair (the two-tuple in
/// SatelliteAntennaFrequency::noazi_pcv_m).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNoaziPcvSample {
    /// First tuple element (zenith-angle node, degrees).
    pub a: f64,
    /// Second tuple element (PCV value, meters).
    pub b: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCodeBiasSystemPair {
    /// GNSS system code, one of SidereonGnssSystem.
    pub system: u32,
    /// First RINEX observable code.
    pub obs1: *const c_char,
    /// Second RINEX observable code.
    pub obs2: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCodeBiasSatellitePair {
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
    /// First RINEX observable code.
    pub obs1: *const c_char,
    /// Second RINEX observable code.
    pub obs2: *const c_char,
}

/// PPP correction precompute switches, mirroring
/// sidereon_core::ppp_corrections::PppCorrectionsOptions. The has_* flags gate
/// each optional sub-structure.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppCorrectionsOptions {
    /// Enable the solid-Earth tide.
    pub solid_earth_tide: bool,
    /// Whether pole_tide is enabled.
    pub has_pole_tide: bool,
    /// Pole-tide options when has_pole_tide.
    pub pole_tide: SidereonPoleTideOptions,
    /// Whether ocean_loading is enabled.
    pub has_ocean_loading: bool,
    /// Ocean-loading BLQ block when has_ocean_loading.
    pub ocean_loading: SidereonOceanLoadingBlq,
    /// Enable carrier phase wind-up.
    pub phase_windup: bool,
    /// Whether satellite_antenna is enabled.
    pub has_satellite_antenna: bool,
    /// Pointer to a SidereonSatelliteAntennaOptions when has_satellite_antenna.
    pub satellite_antenna: *const SidereonSatelliteAntennaOptions,
    /// Whether code_bias is enabled.
    pub has_code_bias: bool,
    /// Parsed Bias-SINEX or DCB product when has_code_bias.
    pub code_bias: *const SidereonBiasSet,
    /// System default observable pairs.
    pub code_bias_system_pairs: *const SidereonPppCodeBiasSystemPair,
    /// Number of system default observable pairs.
    pub code_bias_system_pair_count: usize,
    /// Per-satellite observable-pair overrides.
    pub code_bias_satellite_pairs: *const SidereonPppCodeBiasSatellitePair,
    /// Number of per-satellite observable-pair overrides.
    pub code_bias_satellite_pair_count: usize,
    /// Whether code_bias_clock_reference_pairs overrides the product metadata.
    pub has_code_bias_clock_reference: bool,
    /// Optional clock-reference observable pairs by system.
    pub code_bias_clock_reference_pairs: *const SidereonPppCodeBiasSystemPair,
    /// Number of clock-reference observable pairs.
    pub code_bias_clock_reference_pair_count: usize,
}

/// An epoch-indexed vector correction, mirroring
/// sidereon_core::ppp_corrections::EpochVectorCorrection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEpochVectorCorrection {
    /// Index into the input epoch slice.
    pub epoch_index: usize,
    /// Correction vector, meters (ECEF).
    pub vector_m: [f64; 3],
}

/// An epoch-indexed per-satellite scalar correction, mirroring
/// sidereon_core::ppp_corrections::SatScalarCorrection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatScalarCorrection {
    /// Null-terminated satellite token.
    pub sat_id: [c_char; 17],
    /// Index into the input epoch slice.
    pub epoch_index: usize,
    /// Correction value, meters.
    pub value_m: f64,
}

/// An epoch-indexed per-satellite vector correction, mirroring
/// sidereon_core::ppp_corrections::SatVectorCorrection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatVectorCorrection {
    /// Null-terminated satellite token.
    pub sat_id: [c_char; 17],
    /// Index into the input epoch slice.
    pub epoch_index: usize,
    /// Correction vector, meters (ECEF).
    pub vector_m: [f64; 3],
}

/// Precomputed PPP correction tables. Opaque to C. Create with
/// sidereon_ppp_corrections_build and release with
/// sidereon_ppp_corrections_free.
pub struct SidereonPppCorrections {
    pub(crate) inner: PppCorrectionsInner,
}

/// Build static PPP correction tables for a precise-orbit (SP3) arc. On success
/// writes a newly owned corrections handle; release it with
/// sidereon_ppp_corrections_free. Delegates to
/// sidereon_core::ppp_corrections::build.
///
/// Safety: sp3 is a live handle; epochs points to epoch_count
/// SidereonPppCorrectionEpoch (each with its own observation pointer);
/// receiver_ecef_m points to 3 doubles; options points to a
/// SidereonPppCorrectionsOptions; out points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_build(
    sp3: *const SidereonSp3,
    epochs: *const SidereonPppCorrectionEpoch,
    epoch_count: usize,
    receiver_ecef_m: *const f64,
    options: *const SidereonPppCorrectionsOptions,
    out: *mut *mut SidereonPppCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_build",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_ppp_corrections_build", "out"));
            *out = ptr::null_mut();
            let sp3 = c_try!(require_ref(sp3, "sidereon_ppp_corrections_build", "sp3"));
            let receiver = c_try!(read_vec3(
                "sidereon_ppp_corrections_build",
                "receiver_ecef_m",
                receiver_ecef_m
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_ppp_corrections_build",
                "options"
            ));
            let epochs = c_try!(ppp_corr_epochs_from_c(
                "sidereon_ppp_corrections_build",
                epochs,
                epoch_count
            ));
            let opts = c_try!(ppp_corrections_options_from_c(
                "sidereon_ppp_corrections_build",
                options
            ));
            match sidereon_core::ppp_corrections::build(&sp3.inner, &epochs, receiver, &opts) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonPppCorrections { inner });
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_ppp_corrections_build", err),
            }
        },
    )
}

/// Release a PPP corrections handle.
///
/// Safety: corrections must be a handle from sidereon_ppp_corrections_build or
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_free(corrections: *mut SidereonPppCorrections) {
    free_boxed(corrections);
}

// cbindgen does not expand macros, so the reader entry points are written out
// explicitly and delegate to these shared bodies.

/// Copy the solid-Earth tide correction table (epoch-indexed vectors, meters
/// ECEF). Variable-length output contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonEpochVectorCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_tide(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonEpochVectorCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_tide",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_tide",
                "corrections"
            ));
            ppp_corrections_emit_vector(
                "sidereon_ppp_corrections_tide",
                &corrections.inner.tide,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the solid-Earth pole-tide correction table. Variable-length output
/// contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonEpochVectorCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_pole_tide(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonEpochVectorCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_pole_tide",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_pole_tide",
                "corrections"
            ));
            ppp_corrections_emit_vector(
                "sidereon_ppp_corrections_pole_tide",
                &corrections.inner.pole_tide,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the ocean-tide-loading correction table. Variable-length output
/// contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonEpochVectorCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_ocean_loading(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonEpochVectorCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_ocean_loading",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_ocean_loading",
                "corrections"
            ));
            ppp_corrections_emit_vector(
                "sidereon_ppp_corrections_ocean_loading",
                &corrections.inner.ocean_loading,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the carrier phase wind-up correction table (per-satellite scalars,
/// meters). Variable-length output contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonSatScalarCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_windup(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonSatScalarCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_windup",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_windup",
                "corrections"
            ));
            ppp_corrections_emit_scalar(
                "sidereon_ppp_corrections_windup",
                &corrections.inner.windup_m,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the per-satellite antenna phase-center-variation correction table
/// (scalars, meters). Variable-length output contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonSatScalarCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_sat_pcv(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonSatScalarCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_sat_pcv",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_sat_pcv",
                "corrections"
            ));
            ppp_corrections_emit_scalar(
                "sidereon_ppp_corrections_sat_pcv",
                &corrections.inner.sat_pcv_m,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the per-satellite code-bias correction table (scalars, meters).
/// Variable-length output contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonSatScalarCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_code_bias(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonSatScalarCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_code_bias",
        SidereonStatus::Panic,
        || {
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_code_bias",
                "corrections"
            ));
            ppp_corrections_emit_scalar(
                "sidereon_ppp_corrections_code_bias",
                &corrections.inner.code_bias_m,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Copy the epoch-indexed per-satellite antenna phase-center-offset vector table.
/// Variable-length output contract.
///
/// Safety: corrections is a live handle; out points to len
/// SidereonSatVectorCorrection or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_corrections_sat_pco_ecef(
    corrections: *const SidereonPppCorrections,
    out: *mut SidereonSatVectorCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_corrections_sat_pco_ecef",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ppp_corrections_sat_pco_ecef",
                out_written,
                out_required
            ));
            let corrections = c_try!(require_ref(
                corrections,
                "sidereon_ppp_corrections_sat_pco_ecef",
                "corrections"
            ));
            let mapped: Vec<SidereonSatVectorCorrection> = corrections
                .inner
                .sat_pco_ecef
                .iter()
                .map(|c| {
                    let mut sat_id = [0 as c_char; 17];
                    write_sat_token_buf(&mut sat_id, &c.sat);
                    SidereonSatVectorCorrection {
                        sat_id,
                        epoch_index: c.epoch_index,
                        vector_m: c.vector_m,
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_ppp_corrections_sat_pco_ecef",
                "out",
                &mapped,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

// --- PPP auto-initialization driver (sidereon_core::precise_positioning) ------

/// SPP-seeded auto-initialization policy for the raw-epochs PPP driver, mirroring
/// sidereon_core::precise_positioning::PppAutoInitOptions. Initialize with
/// sidereon_ppp_auto_init_options_init, then override fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPppAutoInitOptions {
    /// When true, the explicit position/clock seed below bypasses the SPP/mean
    /// auto-init stages entirely.
    pub has_initial_guess: bool,
    /// Explicit static receiver position seed (ECEF metres), used when
    /// has_initial_guess is true.
    pub initial_guess_position_m: [f64; 3],
    /// Explicit receiver clock seed (metres), duplicated across every epoch.
    pub initial_guess_clock_m: f64,
    /// SPP cold-start guess [x_m, y_m, z_m, b_m] for every per-epoch seed solve.
    pub spp_initial_guess: [f64; 4],
    /// Apply the troposphere correction in the SPP seed solve. The ionosphere is
    /// always off in the seed.
    pub spp_troposphere: bool,
    /// SPP seed surface pressure (hPa).
    pub spp_pressure_hpa: f64,
    /// SPP seed surface temperature (K).
    pub spp_temperature_k: f64,
    /// SPP seed surface relative humidity, fraction in [0, 1].
    pub spp_relative_humidity: f64,
}

/// Initialize SidereonPppAutoInitOptions with the engine defaults (no explicit
/// guess, an all-zero SPP cold start, the troposphere off, standard surface
/// meteorology).
///
/// Safety: options must point to a writable SidereonPppAutoInitOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_auto_init_options_init(
    options: *mut SidereonPppAutoInitOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ppp_auto_init_options_init",
        SidereonStatus::Panic,
        || {
            let options = c_try!(require_out(
                options,
                "sidereon_ppp_auto_init_options_init",
                "options"
            ));
            let defaults = PppAutoInitOptions::default();
            *options = SidereonPppAutoInitOptions {
                has_initial_guess: defaults.initial_guess.is_some(),
                initial_guess_position_m: defaults
                    .initial_guess
                    .map(|g| g.position_m)
                    .unwrap_or([0.0; 3]),
                initial_guess_clock_m: defaults.initial_guess.map(|g| g.clock_m).unwrap_or(0.0),
                spp_initial_guess: defaults.spp_initial_guess,
                spp_troposphere: defaults.spp_troposphere,
                spp_pressure_hpa: defaults.spp_met.pressure_hpa,
                spp_temperature_k: defaults.spp_met.temperature_k,
                spp_relative_humidity: defaults.spp_met.relative_humidity,
            };
            SidereonStatus::Ok
        },
    )
}

fn ppp_float_metadata(solution: &PppFloatSolutionInner) -> SidereonPppFloatMetadata {
    SidereonPppFloatMetadata {
        iterations: solution.iterations,
        converged: solution.converged,
        status: ppp_solve_status_to_c(solution.status),
        has_ztd_residual_m: solution.ztd_residual_m.is_some(),
        ztd_residual_m: solution.ztd_residual_m.unwrap_or(0.0),
        code_rms_m: solution.code_rms_m,
        phase_rms_m: solution.phase_rms_m,
        weighted_rms_m: solution.weighted_rms_m,
        ambiguity_count: solution.ambiguities_m.len(),
        residual_count: solution.residuals_m.len(),
        used_sat_count: solution.used_sats.len(),
    }
}

fn ppp_fixed_metadata(solution: &PppFixedSolutionInner) -> SidereonPppFixedMetadata {
    SidereonPppFixedMetadata {
        iterations: solution.iterations,
        converged: solution.converged,
        status: ppp_solve_status_to_c(solution.status),
        has_ztd_residual_m: solution.ztd_residual_m.is_some(),
        ztd_residual_m: solution.ztd_residual_m.unwrap_or(0.0),
        code_rms_m: solution.code_rms_m,
        phase_rms_m: solution.phase_rms_m,
        weighted_rms_m: solution.weighted_rms_m,
        fixed_ambiguity_count: solution.fixed_ambiguities_cycles.len(),
        residual_count: solution.residuals_m.len(),
        used_sat_count: solution.used_sats.len(),
        integer_status: ppp_integer_status_to_c(solution.integer.integer_status),
        integer_ratio: solution.integer.integer_ratio,
        integer_best_score: solution.integer.integer_best_score,
        has_integer_second_best_score: solution.integer.integer_second_best_score.is_some(),
        integer_second_best_score: solution.integer.integer_second_best_score.unwrap_or(0.0),
        integer_candidates: solution.integer.integer_candidates,
    }
}

fn ppp_ambiguities_to_c(values: &BTreeMap<String, f64>) -> Vec<SidereonPppAmbiguity> {
    values
        .iter()
        .map(|(id, value_m)| SidereonPppAmbiguity {
            id: ppp_id_token(id),
            value_m: *value_m,
        })
        .collect()
}

fn ppp_fixed_ambiguities_to_c(
    fn_name: &str,
    solution: &PppFixedSolutionInner,
) -> Result<Vec<SidereonPppFixedAmbiguity>, SidereonStatus> {
    ppp_fixed_ambiguity_rows_to_c(
        fn_name,
        &solution.fixed_ambiguities_cycles,
        &solution.fixed_ambiguities_m,
    )
}

fn ppp_used_satellite_tokens(
    fn_name: &str,
    values: &[String],
) -> Result<Vec<SidereonSatelliteToken>, SidereonStatus> {
    validate_element_count::<SidereonSatelliteToken>(fn_name, "used_sats", values.len())?;
    let mut out = Vec::with_capacity(values.len());
    for (idx, sat) in values.iter().enumerate() {
        if sat.len() > MAX_SATELLITE_TOKEN_BYTES {
            set_last_error(format!(
                "{fn_name}: used_sats[{idx}] is {} bytes; use the SidereonPppId accessor for PPP ids longer than {MAX_SATELLITE_TOKEN_BYTES} bytes",
                sat.len()
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        out.push(satellite_token_from_text(sat));
    }
    Ok(out)
}

fn ppp_used_id_tokens(
    fn_name: &str,
    values: &[String],
) -> Result<Vec<SidereonPppId>, SidereonStatus> {
    validate_element_count::<SidereonPppId>(fn_name, "used_sats", values.len())?;
    let mut out = Vec::with_capacity(values.len());
    for (idx, id) in values.iter().enumerate() {
        if id.len() > MAX_PPP_ID_BYTES {
            set_last_error(format!(
                "{fn_name}: used_sats[{idx}] is {} bytes; maximum PPP id length is {MAX_PPP_ID_BYTES} bytes",
                id.len()
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        out.push(ppp_id_token(id));
    }
    Ok(out)
}

fn default_ppp_measurement_weights() -> SidereonPppMeasurementWeights {
    SidereonPppMeasurementWeights {
        code: 1.0,
        phase: 100.0,
        elevation_weighting: false,
    }
}

fn default_ppp_troposphere_options() -> SidereonPppTroposphereOptions {
    SidereonPppTroposphereOptions {
        enabled: false,
        estimate_ztd: false,
        pressure_hpa: 1013.25,
        temperature_k: 288.15,
        relative_humidity: 0.5,
        mapping: SidereonPppTropoMapping::Niell as u32,
        vmf_sample_count: 0,
        vmf_samples: [SidereonPppVmfSiteSample {
            mjd: 0.0,
            ah: 0.0,
            aw: 0.0,
        }; SIDEREON_PPP_VMF_SITE_MAX_SAMPLES],
    }
}

fn default_ppp_float_options() -> SidereonPppFloatOptions {
    SidereonPppFloatOptions {
        max_iterations: ppp_defaults::MAX_ITERATIONS,
        position_tolerance_m: ppp_defaults::POSITION_TOLERANCE_M,
        clock_tolerance_m: ppp_defaults::CLOCK_TOLERANCE_M,
        ambiguity_tolerance_m: ppp_defaults::AMBIGUITY_TOLERANCE_M,
        ztd_tolerance_m: ppp_defaults::ZTD_TOLERANCE_M,
    }
}

fn default_ppp_range_corrections() -> SidereonPppRangeCorrections {
    SidereonPppRangeCorrections {
        receiver_antenna: ptr::null(),
        sat_clock_relativity: false,
        satellite_clock_records: ptr::null(),
        satellite_clock_record_count: 0,
        solid_earth_tide: false,
        phase_windup: false,
        satellite_antenna: false,
    }
}

fn default_ppp_fixed_ambiguity_options() -> SidereonPppFixedAmbiguityOptions {
    SidereonPppFixedAmbiguityOptions {
        wavelengths_m: ptr::null(),
        wavelength_count: 0,
        offsets_m: ptr::null(),
        offset_count: 0,
        ratio_threshold: ppp_defaults::RATIO_THRESHOLD,
    }
}

impl SidereonCivilDateTime {
    pub(crate) fn to_core(self) -> CivilDateTime {
        CivilDateTime {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
        }
    }
}

unsafe fn ppp_corr_epochs_from_c(
    fn_name: &str,
    epochs: *const SidereonPppCorrectionEpoch,
    count: usize,
) -> Result<Vec<PppCorrectionEpoch>, SidereonStatus> {
    let rows = require_slice(epochs, count, fn_name, "epochs")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let observations =
            ppp_corr_observations_from_c(fn_name, row.observations, row.observation_count)?;
        out.push(PppCorrectionEpoch {
            epoch: row.epoch.to_core(),
            t_rx_j2000_s: row.t_rx_j2000_s,
            observations,
        });
    }
    Ok(out)
}

unsafe fn ppp_corrections_options_from_c(
    fn_name: &str,
    options: &SidereonPppCorrectionsOptions,
) -> Result<PppCorrectionsOptions, SidereonStatus> {
    let pole_tide = options.has_pole_tide.then_some(PppPoleTideOptions {
        xp_arcsec: options.pole_tide.xp_arcsec,
        yp_arcsec: options.pole_tide.yp_arcsec,
    });
    let ocean_loading = options.has_ocean_loading.then_some(PppOceanLoadingBlq {
        amplitude_m: options.ocean_loading.amplitude_m,
        phase_deg: options.ocean_loading.phase_deg,
    });
    let satellite_antenna = if options.has_satellite_antenna {
        Some(ppp_satellite_antenna_options_from_c(
            fn_name,
            options.satellite_antenna,
        )?)
    } else {
        None
    };
    let code_bias = if options.has_code_bias {
        let bias_set = require_ref(options.code_bias, fn_name, "code_bias")?;
        let clock_reference = if options.has_code_bias_clock_reference {
            Some(ClockReferenceObservables {
                per_system: ppp_code_bias_system_pairs_from_c(
                    fn_name,
                    "code_bias_clock_reference_pairs",
                    options.code_bias_clock_reference_pairs,
                    options.code_bias_clock_reference_pair_count,
                )?,
            })
        } else {
            None
        };
        Some(PppCodeBiasOptions {
            bias_set: bias_set.inner.clone(),
            used_observables_per_sat: ppp_code_bias_satellite_pairs_from_c(
                fn_name,
                options.code_bias_satellite_pairs,
                options.code_bias_satellite_pair_count,
            )?,
            used_observables_default: ppp_code_bias_system_pairs_from_c(
                fn_name,
                "code_bias_system_pairs",
                options.code_bias_system_pairs,
                options.code_bias_system_pair_count,
            )?,
            clock_reference,
        })
    } else {
        None
    };
    Ok(PppCorrectionsOptions {
        solid_earth_tide: options.solid_earth_tide,
        pole_tide,
        ocean_loading,
        phase_windup: options.phase_windup,
        satellite_antenna,
        code_bias,
    })
}

unsafe fn ppp_corrections_emit_vector(
    fn_name: &str,
    table: &[sidereon_core::ppp_corrections::EpochVectorCorrection],
    out: *mut SidereonEpochVectorCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let mapped: Vec<SidereonEpochVectorCorrection> = table
        .iter()
        .map(|c| SidereonEpochVectorCorrection {
            epoch_index: c.epoch_index,
            vector_m: c.vector_m,
        })
        .collect();
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &mapped,
        out,
        len,
        out_written,
        out_required
    ));
    SidereonStatus::Ok
}

unsafe fn ppp_corrections_emit_scalar(
    fn_name: &str,
    table: &[sidereon_core::ppp_corrections::SatScalarCorrection],
    out: *mut SidereonSatScalarCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let mapped: Vec<SidereonSatScalarCorrection> = table
        .iter()
        .map(|c| {
            let mut sat_id = [0 as c_char; 17];
            write_sat_token_buf(&mut sat_id, &c.sat);
            SidereonSatScalarCorrection {
                sat_id,
                epoch_index: c.epoch_index,
                value_m: c.value_m,
            }
        })
        .collect();
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &mapped,
        out,
        len,
        out_written,
        out_required
    ));
    SidereonStatus::Ok
}

fn ppp_solve_status_to_c(status: PppFloatStatus) -> SidereonPppSolveStatus {
    match status {
        PppFloatStatus::StateTolerance => SidereonPppSolveStatus::StateTolerance,
        PppFloatStatus::MaxIterations => SidereonPppSolveStatus::MaxIterations,
    }
}

fn ppp_integer_status_to_c(status: PppIntegerStatusInner) -> SidereonPppIntegerStatus {
    match status {
        PppIntegerStatusInner::Fixed => SidereonPppIntegerStatus::Fixed,
        PppIntegerStatusInner::NotFixed => SidereonPppIntegerStatus::NotFixed,
    }
}

fn write_sat_token_buf(buf: &mut [c_char; 17], sat: &GnssSatelliteId) {
    let s = sat.to_string();
    let bytes = s.as_bytes();
    let n = bytes.len().min(SATELLITE_TOKEN_C_BYTES - 1);
    for slot in buf.iter_mut() {
        *slot = 0;
    }
    for (slot, b) in buf.iter_mut().zip(bytes.iter().take(n)) {
        *slot = *b as c_char;
    }
    buf[n] = 0;
}

unsafe fn ppp_corr_observations_from_c(
    fn_name: &str,
    obs: *const SidereonPppCorrectionObservation,
    count: usize,
) -> Result<Vec<PppCorrectionObservation>, SidereonStatus> {
    let rows = require_slice(obs, count, fn_name, "observations")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        out.push(PppCorrectionObservation {
            sat,
            freq1_hz: row.freq1_hz,
            freq2_hz: row.freq2_hz,
            glonass_channel: row.has_glonass_channel.then_some(row.glonass_channel),
        });
    }
    Ok(out)
}

unsafe fn ppp_satellite_antenna_options_from_c(
    fn_name: &str,
    options: *const SidereonSatelliteAntennaOptions,
) -> Result<PppSatelliteAntennaOptions, SidereonStatus> {
    let options = require_ref(options, fn_name, "satellite_antenna")?;
    Ok(PppSatelliteAntennaOptions {
        freq1_label: parse_bounded_c_string(
            fn_name,
            "satellite_antenna.freq1_label",
            options.freq1_label,
            MAX_PPP_SAT_ANTENNA_LABEL_BYTES,
        )?,
        freq1_hz: options.freq1_hz,
        freq2_label: parse_bounded_c_string(
            fn_name,
            "satellite_antenna.freq2_label",
            options.freq2_label,
            MAX_PPP_SAT_ANTENNA_LABEL_BYTES,
        )?,
        freq2_hz: options.freq2_hz,
        antennas: ppp_satellite_antennas_from_c(fn_name, options.antennas, options.antenna_count)?,
    })
}

unsafe fn ppp_code_bias_system_pairs_from_c(
    fn_name: &str,
    arg_name: &str,
    pairs: *const SidereonPppCodeBiasSystemPair,
    count: usize,
) -> Result<BTreeMap<GnssSystem, (String, String)>, SidereonStatus> {
    let rows = require_slice(pairs, count, fn_name, arg_name)?;
    let mut out = BTreeMap::new();
    for (idx, row) in rows.iter().enumerate() {
        let system =
            gnss_system_from_c_code(fn_name, &format!("{arg_name}[{idx}].system"), row.system)?;
        let obs1 = parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}[{idx}].obs1"),
            row.obs1,
            MAX_BIAS_OBS_BYTES,
        )?;
        let obs2 = parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}[{idx}].obs2"),
            row.obs2,
            MAX_BIAS_OBS_BYTES,
        )?;
        if out.insert(system, (obs1, obs2)).is_some() {
            set_last_error(format!(
                "{fn_name}: duplicate {arg_name} system at index {idx}"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    }
    Ok(out)
}

unsafe fn ppp_code_bias_satellite_pairs_from_c(
    fn_name: &str,
    pairs: *const SidereonPppCodeBiasSatellitePair,
    count: usize,
) -> Result<BTreeMap<GnssSatelliteId, (String, String)>, SidereonStatus> {
    let rows = require_slice(pairs, count, fn_name, "code_bias_satellite_pairs")?;
    let mut out = BTreeMap::new();
    for (idx, row) in rows.iter().enumerate() {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        let obs1 = parse_bounded_c_string(
            fn_name,
            &format!("code_bias_satellite_pairs[{idx}].obs1"),
            row.obs1,
            MAX_BIAS_OBS_BYTES,
        )?;
        let obs2 = parse_bounded_c_string(
            fn_name,
            &format!("code_bias_satellite_pairs[{idx}].obs2"),
            row.obs2,
            MAX_BIAS_OBS_BYTES,
        )?;
        if out.insert(sat, (obs1, obs2)).is_some() {
            set_last_error(format!(
                "{fn_name}: duplicate code_bias_satellite_pairs satellite at index {idx}"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    }
    Ok(out)
}
