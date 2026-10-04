use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

// --- Clock stability: Allan-family estimators -------------------------------

/// Sample input kind for Allan-family clock-stability estimators.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAllanSeriesKind {
    /// Phase deviations in seconds.
    PhaseSeconds = 0,
    /// Fractional-frequency samples, dimensionless.
    FractionalFrequency = 1,
    /// Phase deviations in seconds with missing samples.
    PhaseSecondsWithGaps = 2,
    /// Fractional-frequency samples with missing samples.
    FractionalFrequencyWithGaps = 3,
}

/// Averaging-factor grid for Allan-family estimators.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAllanTauGrid {
    /// `m = 1, 2, 4, 8, ...` while the estimator has terms.
    Octave = 0,
    /// Every `m = 1..=m_max` while the estimator has terms.
    All = 1,
    /// Caller-supplied averaging factors.
    Explicit = 2,
}

/// Missing-sample policy for gapped Allan-family inputs.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAllanGapPolicy {
    /// Reject any missing sample.
    Reject = 0,
    /// Exclude estimator terms that cross a missing sample.
    OmitTerms = 1,
}

/// Allan-family estimator selector.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAllanEstimator {
    /// Plain non-overlapping Allan deviation.
    Adev = 0,
    /// Fully overlapping Allan deviation.
    OverlappingAdev = 1,
    /// Modified Allan deviation.
    Mdev = 2,
    /// Overlapping Hadamard deviation.
    Hdev = 3,
    /// Time deviation.
    Tdev = 4,
}

/// One clock-stability sample. For non-gapped series, `has_value` must be true
/// for every row. Phase samples are seconds. Fractional-frequency samples are
/// dimensionless.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAllanSample {
    /// Whether value carries a sample.
    pub has_value: bool,
    /// Phase deviation in seconds or fractional frequency, depending on series.
    pub value: f64,
}

/// Estimators to compute in sidereon_clock_compute_allan_deviations.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAllanEstimatorSet {
    /// Compute plain non-overlapping Allan deviation.
    pub adev: bool,
    /// Compute fully overlapping Allan deviation.
    pub overlapping_adev: bool,
    /// Compute modified Allan deviation.
    pub mdev: bool,
    /// Compute overlapping Hadamard deviation.
    pub hdev: bool,
    /// Compute time deviation.
    pub tdev: bool,
}

/// Options for sidereon_clock_compute_allan_deviations. Initialize with
/// sidereon_clock_allan_options_init. Explicit averaging factors are read only
/// when `tau_grid == SIDEREON_ALLAN_TAU_GRID_EXPLICIT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAllanOptions {
    /// Estimators to compute.
    pub estimators: SidereonAllanEstimatorSet,
    /// Tau grid selector as SidereonAllanTauGrid.
    pub tau_grid: u32,
    /// Gap policy as SidereonAllanGapPolicy.
    pub gap_policy: u32,
    /// Averaging factors for the explicit tau grid.
    pub averaging_factors: *const usize,
    /// Number of explicit averaging factors.
    pub averaging_factor_count: usize,
}

/// One point on an Allan-family estimator curve. `tau_s` is seconds, deviation
/// is in the estimator's natural units, and `n` is the number of terms used.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAllanPoint {
    /// Averaging time, seconds.
    pub tau_s: f64,
    /// Deviation value at tau_s.
    pub deviation: f64,
    /// Number of estimator terms used at tau_s.
    pub n: usize,
}

/// One RINEX receiver-clock phase-deviation sample in seconds.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockPhaseSample {
    /// Whether phase_s carries a receiver-clock phase deviation.
    pub has_phase_s: bool,
    /// Receiver-clock phase deviation, seconds, when present.
    pub phase_s: f64,
}

/// Combined Allan-family estimator curves. Opaque to C. Create with
/// sidereon_clock_compute_allan_deviations and release with
/// sidereon_clock_allan_deviation_curves_free.
pub struct SidereonAllanDeviationCurves {
    pub(crate) inner: CoreAllanDeviationCurves,
}

/// IEEE 1139 fractional-frequency PSD power-law noise type.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPowerLawNoiseType {
    /// Random-walk frequency modulation.
    RandomWalkFM = 0,
    /// Flicker frequency modulation.
    FlickerFM = 1,
    /// White frequency modulation.
    WhiteFM = 2,
    /// Flicker phase modulation.
    FlickerPM = 3,
    /// White phase modulation.
    WhitePM = 4,
}

/// Reason a power-law octave was not classifiable.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPowerLawOctaveFlag {
    /// Too few tau points were present in the octave.
    UnderSampled = 0,
    /// A zero deviation made the slope undefined.
    DegenerateDeviation = 1,
    /// MDEV did not have enough tau points to separate phase-modulation types.
    MissingModifiedAllan = 2,
}

/// Dominant-noise decision class for one tau octave.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPowerLawOctaveDominanceKind {
    /// A single power-law type was identified.
    Dominant = 0,
    /// The octave contains conflicting or off-table slopes.
    Ambiguous = 1,
    /// Required data were absent.
    Flagged = 2,
}

/// Options for IEEE 1139 power-law noise identification.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPowerLawNoiseOptions {
    /// Minimum tau points required before an octave can be classified.
    pub min_points_per_octave: usize,
    /// Maximum absolute slope error allowed for exact-rational noise types.
    pub slope_tolerance: f64,
    /// Maximum robust local-slope scatter before an octave is ambiguous.
    pub scatter_tolerance: f64,
    /// Basic sample interval used by the deviation calculation, seconds.
    pub basic_tau_s: f64,
    /// Upper measurement bandwidth, hertz.
    pub measurement_bandwidth_hz: f64,
}

/// Per-octave power-law classification from ADEV and MDEV slopes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPowerLawOctave {
    /// First tau in the octave, seconds.
    pub tau_start_s: f64,
    /// Last tau used in the octave, seconds.
    pub tau_end_s: f64,
    /// Number of ADEV tau points used for the slope.
    pub point_count: usize,
    /// Whether adev_slope carries a value.
    pub has_adev_slope: bool,
    /// Fitted ADEV log-log slope.
    pub adev_slope: f64,
    /// Whether mdev_slope carries a value.
    pub has_mdev_slope: bool,
    /// Fitted MDEV log-log slope.
    pub mdev_slope: f64,
    /// Whether slope_scatter carries a value.
    pub has_slope_scatter: bool,
    /// Robust scatter of adjacent ADEV slopes.
    pub slope_scatter: f64,
    /// Dominance class, as SidereonPowerLawOctaveDominanceKind.
    pub dominance_kind: u32,
    /// Dominant noise type when dominance_kind is Dominant.
    pub noise_type: u32,
    /// Flag reason when dominance_kind is Flagged.
    pub flag: u32,
}

/// Consecutive tau span supporting one fitted power-law coefficient.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPowerLawNoiseRegion {
    /// Identified noise type.
    pub noise_type: u32,
    /// First tau in the region, seconds.
    pub tau_start_s: f64,
    /// Last tau in the region, seconds.
    pub tau_end_s: f64,
    /// Number of classified octaves merged into this region.
    pub octave_count: usize,
    /// Number of deviation points used in the coefficient fit.
    pub point_count: usize,
    /// Mean local slope from the classification statistic.
    pub mean_slope: f64,
    /// Fitted PSD coefficient.
    pub coefficient: f64,
}

/// Power-law noise fit. Opaque to C. Create with
/// sidereon_clock_fit_power_law_noise and release with
/// sidereon_clock_power_law_noise_fit_free.
pub struct SidereonPowerLawNoiseFit {
    pub(crate) inner: CorePowerLawNoiseFit,
}

/// Initialize Allan-family options with the core defaults: standard estimators,
/// octave tau grid, and gap rejection.
///
/// Safety: out_options must point to a SidereonAllanOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_allan_options_init(
    out_options: *mut SidereonAllanOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_allan_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_clock_allan_options_init",
                "out_options"
            ));
            let defaults = AllanOptions::default();
            *out = SidereonAllanOptions {
                estimators: SidereonAllanEstimatorSet {
                    adev: defaults.estimators.adev,
                    overlapping_adev: defaults.estimators.overlapping_adev,
                    mdev: defaults.estimators.mdev,
                    hdev: defaults.estimators.hdev,
                    tdev: defaults.estimators.tdev,
                },
                tau_grid: SidereonAllanTauGrid::Octave as u32,
                gap_policy: SidereonAllanGapPolicy::Reject as u32,
                averaging_factors: ptr::null(),
                averaging_factor_count: 0,
            };
            SidereonStatus::Ok
        },
    )
}

/// Compute selected Allan-family estimator curves from phase seconds or
/// fractional-frequency samples. `tau0_s` is the sample interval in seconds.
/// On success writes a handle to *out_curves; release it with
/// sidereon_clock_allan_deviation_curves_free.
///
/// Safety: samples points to count SidereonAllanSample entries, or NULL when
/// count is zero; options may be NULL for defaults; out_curves points to a
/// SidereonAllanDeviationCurves*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_compute_allan_deviations(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    options: *const SidereonAllanOptions,
    out_curves: *mut *mut SidereonAllanDeviationCurves,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_compute_allan_deviations",
        SidereonStatus::Panic,
        || {
            let out_curves = c_try!(require_out(
                out_curves,
                "sidereon_clock_compute_allan_deviations",
                "out_curves"
            ));
            *out_curves = ptr::null_mut();
            let storage = c_try!(allan_series_from_c(
                "sidereon_clock_compute_allan_deviations",
                samples,
                count,
                series_kind,
            ));
            let options = c_try!(allan_options_from_c(
                "sidereon_clock_compute_allan_deviations",
                options,
            ));
            let input = AllanInput {
                series: storage.as_series(),
                tau0_s,
                options,
            };
            match core_compute_allan_deviations(&input) {
                Ok(inner) => {
                    write_boxed_handle(out_curves, SidereonAllanDeviationCurves { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_allan_error("sidereon_clock_compute_allan_deviations", err),
            }
        },
    )
}

/// Report whether a combined Allan-family result contains a curve for the
/// requested estimator.
///
/// Safety: curves must be a live handle; out_present must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_allan_curve_present(
    curves: *const SidereonAllanDeviationCurves,
    estimator: u32,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_allan_curve_present",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_present,
                "sidereon_clock_allan_curve_present",
                "out_present"
            ));
            *out = false;
            let curves = c_try!(require_ref(
                curves,
                "sidereon_clock_allan_curve_present",
                "curves"
            ));
            let estimator = c_try!(allan_estimator_from_c(
                "sidereon_clock_allan_curve_present",
                estimator,
            ));
            *out = allan_curve_for_estimator(&curves.inner, estimator).is_some();
            SidereonStatus::Ok
        },
    )
}

/// Copy one curve from a combined Allan-family result. Missing curves copy zero
/// points and return OK. Uses the variable-length output contract.
///
/// Safety: curves must be a live handle; out points to len SidereonAllanPoint
/// entries or NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_allan_curve(
    curves: *const SidereonAllanDeviationCurves,
    estimator: u32,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_clock_allan_curve", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_clock_allan_curve",
            out_written,
            out_required
        ));
        let curves = c_try!(require_ref(curves, "sidereon_clock_allan_curve", "curves"));
        let estimator = c_try!(allan_estimator_from_c(
            "sidereon_clock_allan_curve",
            estimator,
        ));
        let points = allan_curve_for_estimator(&curves.inner, estimator)
            .map(allan_points)
            .unwrap_or_default();
        c_try!(copy_prefix_to_c(
            "sidereon_clock_allan_curve",
            "out",
            &points,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release combined Allan-family curves. Passing NULL is a no-op.
///
/// Safety: curves must be NULL or a live handle from
/// sidereon_clock_compute_allan_deviations.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_allan_deviation_curves_free(
    curves: *mut SidereonAllanDeviationCurves,
) {
    ffi_boundary("sidereon_clock_allan_deviation_curves_free", (), || {
        free_boxed(curves);
    });
}

/// Plain non-overlapping Allan deviation for explicit averaging factors. Each
/// output point has tau in seconds.
///
/// Safety: samples and averaging_factors point to their counts; out follows the
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_allan_deviation(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_allan_deviation",
        SidereonStatus::Panic,
        || {
            allan_explicit_common(
                "sidereon_clock_allan_deviation",
                samples,
                count,
                series_kind,
                tau0_s,
                averaging_factors,
                averaging_factor_count,
                CoreAllanEstimator::Adev,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Fully overlapping Allan deviation for explicit averaging factors. Each
/// output point has tau in seconds.
///
/// Safety: samples and averaging_factors point to their counts; out follows the
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_overlapping_adev(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_overlapping_adev",
        SidereonStatus::Panic,
        || {
            allan_explicit_common(
                "sidereon_clock_overlapping_adev",
                samples,
                count,
                series_kind,
                tau0_s,
                averaging_factors,
                averaging_factor_count,
                CoreAllanEstimator::OverlappingAdev,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Modified Allan deviation for explicit averaging factors. Each output point
/// has tau in seconds.
///
/// Safety: samples and averaging_factors point to their counts; out follows the
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_modified_adev(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_modified_adev",
        SidereonStatus::Panic,
        || {
            allan_explicit_common(
                "sidereon_clock_modified_adev",
                samples,
                count,
                series_kind,
                tau0_s,
                averaging_factors,
                averaging_factor_count,
                CoreAllanEstimator::Mdev,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Overlapping Hadamard deviation for explicit averaging factors. Each output
/// point has tau in seconds.
///
/// Safety: samples and averaging_factors point to their counts; out follows the
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_hadamard_deviation(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_hadamard_deviation",
        SidereonStatus::Panic,
        || {
            allan_explicit_common(
                "sidereon_clock_hadamard_deviation",
                samples,
                count,
                series_kind,
                tau0_s,
                averaging_factors,
                averaging_factor_count,
                CoreAllanEstimator::Hdev,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Time deviation for explicit averaging factors. Tau and deviation are seconds.
///
/// Safety: samples and averaging_factors point to their counts; out follows the
/// variable-length output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_time_deviation(
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_time_deviation",
        SidereonStatus::Panic,
        || {
            allan_explicit_common(
                "sidereon_clock_time_deviation",
                samples,
                count,
                series_kind,
                tau0_s,
                averaging_factors,
                averaging_factor_count,
                CoreAllanEstimator::Tdev,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Initialize power-law noise options from a sample interval and bandwidth.
///
/// Safety: out_options must point to SidereonPowerLawNoiseOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_options_init(
    basic_tau_s: f64,
    measurement_bandwidth_hz: f64,
    out_options: *mut SidereonPowerLawNoiseOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_power_law_noise_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_clock_power_law_noise_options_init",
                "out_options"
            ));
            *out = power_law_options_to_c(PowerLawNoiseOptions::new(
                basic_tau_s,
                measurement_bandwidth_hz,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Return exact ADEV and MDEV log-log slopes for a power-law noise type.
///
/// Safety: out_adev_slope, out_mdev_slope, and out_variance_tau_exponent must
/// point to writable scalars and their output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_slopes(
    noise_type: u32,
    out_adev_slope: *mut f64,
    out_mdev_slope: *mut f64,
    out_variance_tau_exponent: *mut i32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_power_law_noise_slopes",
        SidereonStatus::Panic,
        || {
            if !out_adev_slope.is_null()
                && !out_mdev_slope.is_null()
                && !out_variance_tau_exponent.is_null()
            {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_clock_power_law_noise_slopes",
                            out_adev_slope,
                            1,
                            "out_adev_slope"
                        )),
                        "out_adev_slope",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_clock_power_law_noise_slopes",
                            out_mdev_slope,
                            1,
                            "out_mdev_slope"
                        )),
                        "out_mdev_slope",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_clock_power_law_noise_slopes",
                            out_variance_tau_exponent,
                            1,
                            "out_variance_tau_exponent"
                        )),
                        "out_variance_tau_exponent",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_clock_power_law_noise_slopes",
                    &outputs
                ));
            }
            let out_adev = c_try!(require_out(
                out_adev_slope,
                "sidereon_clock_power_law_noise_slopes",
                "out_adev_slope"
            ));
            let out_mdev = c_try!(require_out(
                out_mdev_slope,
                "sidereon_clock_power_law_noise_slopes",
                "out_mdev_slope"
            ));
            let out_exp = c_try!(require_out(
                out_variance_tau_exponent,
                "sidereon_clock_power_law_noise_slopes",
                "out_variance_tau_exponent"
            ));
            *out_adev = 0.0;
            *out_mdev = 0.0;
            *out_exp = 0;
            let noise_type = c_try!(power_law_noise_type_from_c(
                "sidereon_clock_power_law_noise_slopes",
                noise_type,
            ));
            *out_adev = core_allan_deviation_power_law_slope(noise_type);
            *out_mdev = core_modified_allan_deviation_power_law_slope(noise_type);
            *out_exp = core_allan_variance_power_law_tau_exponent(noise_type);
            SidereonStatus::Ok
        },
    )
}

/// Identify power-law clock noise from supplied ADEV and MDEV curves. On
/// success writes a handle to *out_fit.
///
/// Safety: adev_points and mdev_points point to their counts; options may be
/// NULL for sampled-at-Nyquist defaults from the first ADEV tau; out_fit must
/// point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_fit_power_law_noise(
    adev_points: *const SidereonAllanPoint,
    adev_count: usize,
    mdev_points: *const SidereonAllanPoint,
    mdev_count: usize,
    options: *const SidereonPowerLawNoiseOptions,
    out_fit: *mut *mut SidereonPowerLawNoiseFit,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_clock_fit_power_law_noise",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_fit,
                "sidereon_clock_fit_power_law_noise",
                "out_fit"
            ));
            *out = ptr::null_mut();
            let adev = c_try!(allan_result_from_points(
                "sidereon_clock_fit_power_law_noise",
                "adev_points",
                adev_points,
                adev_count,
            ));
            let mdev = c_try!(allan_result_from_points(
                "sidereon_clock_fit_power_law_noise",
                "mdev_points",
                mdev_points,
                mdev_count,
            ));
            let options = c_try!(power_law_options_from_c(
                "sidereon_clock_fit_power_law_noise",
                options,
                &adev,
            ));
            match core_fit_power_law_noise(&adev, &mdev, options) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonPowerLawNoiseFit { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_power_law_noise_error("sidereon_clock_fit_power_law_noise", err),
            }
        },
    )
}

/// Copy PSD coefficients [h_-2, h_-1, h_0, h_1, h_2].
///
/// Safety: fit must be live; out_coefficients must point to 5 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_fit_coefficients(
    fit: *const SidereonPowerLawNoiseFit,
    out_coefficients: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_power_law_noise_fit_coefficients",
        SidereonStatus::Panic,
        || {
            let fit = c_try!(require_ref(
                fit,
                "sidereon_clock_power_law_noise_fit_coefficients",
                "fit"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_clock_power_law_noise_fit_coefficients",
                "out_coefficients",
                out_coefficients,
                5,
                &fit.inner.coefficients,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy per-octave power-law decisions. Uses the variable-length output
/// contract.
///
/// Safety: fit must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_fit_octaves(
    fit: *const SidereonPowerLawNoiseFit,
    out: *mut SidereonPowerLawOctave,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_power_law_noise_fit_octaves",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_clock_power_law_noise_fit_octaves",
                out_written,
                out_required
            ));
            let fit = c_try!(require_ref(
                fit,
                "sidereon_clock_power_law_noise_fit_octaves",
                "fit"
            ));
            let octaves: Vec<_> = fit
                .inner
                .dominant_per_octave
                .iter()
                .map(power_law_octave_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_clock_power_law_noise_fit_octaves",
                "out",
                &octaves,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy fitted power-law regions. Uses the variable-length output contract.
///
/// Safety: fit must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_fit_regions(
    fit: *const SidereonPowerLawNoiseFit,
    out: *mut SidereonPowerLawNoiseRegion,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_clock_power_law_noise_fit_regions",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_clock_power_law_noise_fit_regions",
                out_written,
                out_required
            ));
            let fit = c_try!(require_ref(
                fit,
                "sidereon_clock_power_law_noise_fit_regions",
                "fit"
            ));
            let regions: Vec<_> = fit
                .inner
                .regions
                .iter()
                .map(power_law_region_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_clock_power_law_noise_fit_regions",
                "out",
                &regions,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a power-law noise fit handle. Null is a no-op.
///
/// Safety: fit must be NULL or a live handle from
/// sidereon_clock_fit_power_law_noise.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_power_law_noise_fit_free(
    fit: *mut SidereonPowerLawNoiseFit,
) {
    ffi_boundary("sidereon_clock_power_law_noise_fit_free", (), || {
        free_boxed(fit);
    });
}

/// Broadcast satellite-clock polynomial about toc_sow, mirroring
/// sidereon_core::ephemeris::ClockPolynomial.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockPolynomial {
    /// Clock bias (s).
    pub af0: f64,
    /// Clock drift (s/s).
    pub af1: f64,
    /// Clock drift rate (s/s^2).
    pub af2: f64,
    /// Clock reference time, seconds of week.
    pub toc_sow: f64,
}

/// The satellite clock offset, split into components, mirroring
/// sidereon_core::ephemeris::ClockOffset.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockOffset {
    /// Polynomial term (s).
    pub dt_clock_poly_s: f64,
    /// Relativistic eccentricity term (s).
    pub dt_rel_s: f64,
    /// Group delay subtracted for the single-frequency user (s).
    pub tgd_s: f64,
    /// Total satellite clock offset (s).
    pub dt_clock_total_s: f64,
}

fn allan_estimator_from_c(
    fn_name: &str,
    estimator: u32,
) -> Result<CoreAllanEstimator, SidereonStatus> {
    match estimator {
        value if value == SidereonAllanEstimator::Adev as u32 => Ok(CoreAllanEstimator::Adev),
        value if value == SidereonAllanEstimator::OverlappingAdev as u32 => {
            Ok(CoreAllanEstimator::OverlappingAdev)
        }
        value if value == SidereonAllanEstimator::Mdev as u32 => Ok(CoreAllanEstimator::Mdev),
        value if value == SidereonAllanEstimator::Hdev as u32 => Ok(CoreAllanEstimator::Hdev),
        value if value == SidereonAllanEstimator::Tdev as u32 => Ok(CoreAllanEstimator::Tdev),
        _ => {
            set_last_error(format!("{fn_name}: invalid Allan estimator"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn allan_options_from_c(
    fn_name: &str,
    options: *const SidereonAllanOptions,
) -> Result<AllanOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(AllanOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    let mut o = AllanOptions::default();
    o.estimators = allan_estimator_set_from_c(options.estimators);
    o.tau_grid = allan_tau_grid_from_c(fn_name, options)?;
    o.gap_policy = allan_gap_policy_from_c(fn_name, options.gap_policy)?;
    Ok(o)
}

fn allan_curve_for_estimator(
    curves: &CoreAllanDeviationCurves,
    estimator: CoreAllanEstimator,
) -> Option<&CoreAllanResult> {
    match estimator {
        CoreAllanEstimator::Adev => curves.adev.as_ref(),
        CoreAllanEstimator::OverlappingAdev => curves.overlapping_adev.as_ref(),
        CoreAllanEstimator::Mdev => curves.mdev.as_ref(),
        CoreAllanEstimator::Hdev => curves.hdev.as_ref(),
        CoreAllanEstimator::Tdev => curves.tdev.as_ref(),
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn allan_explicit_common(
    fn_name: &str,
    samples: *const SidereonAllanSample,
    count: usize,
    series_kind: u32,
    tau0_s: f64,
    averaging_factors: *const usize,
    averaging_factor_count: usize,
    estimator: CoreAllanEstimator,
    out: *mut SidereonAllanPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let storage = c_try!(allan_series_from_c(fn_name, samples, count, series_kind));
    let factors = c_try!(require_slice(
        averaging_factors,
        averaging_factor_count,
        fn_name,
        "averaging_factors"
    ));
    let result = match estimator {
        CoreAllanEstimator::Adev => core_allan_deviation(storage.as_series(), tau0_s, factors),
        CoreAllanEstimator::OverlappingAdev => {
            core_overlapping_adev(storage.as_series(), tau0_s, factors)
        }
        CoreAllanEstimator::Mdev => core_modified_adev(storage.as_series(), tau0_s, factors),
        CoreAllanEstimator::Hdev => core_hadamard_deviation(storage.as_series(), tau0_s, factors),
        CoreAllanEstimator::Tdev => core_time_deviation(storage.as_series(), tau0_s, factors),
    };
    let result = match result {
        Ok(result) => result,
        Err(err) => return map_allan_error(fn_name, err),
    };
    let points = allan_points(&result);
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &points,
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

impl SidereonClockPolynomial {
    pub(crate) fn to_core(self) -> CoreClockPolynomial {
        CoreClockPolynomial {
            af0: self.af0,
            af1: self.af1,
            af2: self.af2,
            toc_sow: self.toc_sow,
        }
    }
}

impl SidereonClockOffset {
    pub(crate) fn from_core(c: &CoreClockOffset) -> Self {
        Self {
            dt_clock_poly_s: c.dt_clock_poly_s,
            dt_rel_s: c.dt_rel_s,
            tgd_s: c.tgd_s,
            dt_clock_total_s: c.dt_clock_total_s,
        }
    }

    pub(crate) const ZERO: Self = Self {
        dt_clock_poly_s: 0.0,
        dt_rel_s: 0.0,
        tgd_s: 0.0,
        dt_clock_total_s: 0.0,
    };
}

fn allan_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn allan_estimator_name(estimator: CoreAllanEstimator) -> &'static str {
    match estimator {
        CoreAllanEstimator::Adev => "adev",
        CoreAllanEstimator::OverlappingAdev => "overlapping_adev",
        CoreAllanEstimator::Mdev => "mdev",
        CoreAllanEstimator::Hdev => "hdev",
        CoreAllanEstimator::Tdev => "tdev",
    }
}

pub(crate) fn allan_error_value(error: &AllanError) -> Value {
    match error {
        AllanError::EmptySeries => allan_node("empty_series", json!({})),
        AllanError::InvalidTau0 { tau0_s } => allan_node(
            "invalid_tau0",
            json!({
                "tau0_s": engine_f64(*tau0_s),
            }),
        ),
        AllanError::NoEstimators => allan_node("no_estimators", json!({})),
        AllanError::EmptyTauGrid => allan_node("empty_tau_grid", json!({})),
        AllanError::InvalidAveragingFactor { averaging_factor } => allan_node(
            "invalid_averaging_factor",
            json!({
                "averaging_factor": averaging_factor,
            }),
        ),
        AllanError::TooFewSamples {
            estimator,
            averaging_factor,
            available_phase_samples,
        } => allan_node(
            "too_few_samples",
            json!({
                "estimator": allan_estimator_name(*estimator),
                "averaging_factor": averaging_factor,
                "available_phase_samples": available_phase_samples,
            }),
        ),
        AllanError::NonFiniteSample { index } => allan_node(
            "non_finite_sample",
            json!({
                "index": index,
            }),
        ),
        AllanError::Gap { index } => allan_node(
            "gap",
            json!({
                "index": index,
            }),
        ),
        AllanError::NonFiniteTau {
            estimator,
            averaging_factor,
        } => allan_node(
            "non_finite_tau",
            json!({
                "estimator": allan_estimator_name(*estimator),
                "averaging_factor": averaging_factor,
            }),
        ),
        AllanError::NonFiniteDeviation {
            estimator,
            averaging_factor,
        } => allan_node(
            "non_finite_deviation",
            json!({
                "estimator": allan_estimator_name(*estimator),
                "averaging_factor": averaging_factor,
            }),
        ),
    }
}

fn map_allan_error(fn_name: &str, err: AllanError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Allan,
        fn_name,
        allan_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn allan_tau_grid_from_c(
    fn_name: &str,
    options: &SidereonAllanOptions,
) -> Result<TauGrid, SidereonStatus> {
    match options.tau_grid {
        value if value == SidereonAllanTauGrid::Octave as u32 => Ok(TauGrid::Octave),
        value if value == SidereonAllanTauGrid::All as u32 => Ok(TauGrid::All),
        value if value == SidereonAllanTauGrid::Explicit as u32 => {
            let factors = unsafe {
                require_slice(
                    options.averaging_factors,
                    options.averaging_factor_count,
                    fn_name,
                    "options.averaging_factors",
                )?
            };
            Ok(TauGrid::Explicit(factors.to_vec()))
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid Allan tau grid"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn allan_gap_policy_from_c(fn_name: &str, policy: u32) -> Result<GapPolicy, SidereonStatus> {
    match policy {
        value if value == SidereonAllanGapPolicy::Reject as u32 => Ok(GapPolicy::Reject),
        value if value == SidereonAllanGapPolicy::OmitTerms as u32 => Ok(GapPolicy::OmitTerms),
        _ => {
            set_last_error(format!("{fn_name}: invalid Allan gap policy"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn allan_estimator_set_from_c(set: SidereonAllanEstimatorSet) -> AllanEstimatorSet {
    AllanEstimatorSet {
        adev: set.adev,
        overlapping_adev: set.overlapping_adev,
        mdev: set.mdev,
        hdev: set.hdev,
        tdev: set.tdev,
    }
}

unsafe fn allan_series_from_c(
    fn_name: &str,
    samples: *const SidereonAllanSample,
    count: usize,
    kind: u32,
) -> Result<AllanSeriesStorage, SidereonStatus> {
    let kind = allan_series_kind_from_c(fn_name, kind)?;
    let raw = require_slice(samples, count, fn_name, "samples")?;
    match kind {
        SidereonAllanSeriesKind::PhaseSeconds => {
            let mut values = Vec::with_capacity(raw.len());
            for (idx, sample) in raw.iter().enumerate() {
                if !sample.has_value {
                    set_last_error(format!("{fn_name}: samples[{idx}] is missing"));
                    return Err(SidereonStatus::InvalidArgument);
                }
                values.push(sample.value);
            }
            Ok(AllanSeriesStorage::PhaseSeconds(values))
        }
        SidereonAllanSeriesKind::FractionalFrequency => {
            let mut values = Vec::with_capacity(raw.len());
            for (idx, sample) in raw.iter().enumerate() {
                if !sample.has_value {
                    set_last_error(format!("{fn_name}: samples[{idx}] is missing"));
                    return Err(SidereonStatus::InvalidArgument);
                }
                values.push(sample.value);
            }
            Ok(AllanSeriesStorage::FractionalFrequency(values))
        }
        SidereonAllanSeriesKind::PhaseSecondsWithGaps => {
            Ok(AllanSeriesStorage::PhaseSecondsWithGaps(
                raw.iter()
                    .map(|sample| sample.has_value.then_some(sample.value))
                    .collect(),
            ))
        }
        SidereonAllanSeriesKind::FractionalFrequencyWithGaps => {
            Ok(AllanSeriesStorage::FractionalFrequencyWithGaps(
                raw.iter()
                    .map(|sample| sample.has_value.then_some(sample.value))
                    .collect(),
            ))
        }
    }
}

fn allan_points(result: &CoreAllanResult) -> Vec<SidereonAllanPoint> {
    result
        .tau_s
        .iter()
        .zip(&result.deviation)
        .zip(&result.n)
        .map(|((&tau_s, &deviation), &n)| SidereonAllanPoint {
            tau_s,
            deviation,
            n,
        })
        .collect()
}

unsafe fn allan_result_from_points(
    fn_name: &str,
    arg_name: &str,
    points: *const SidereonAllanPoint,
    count: usize,
) -> Result<CoreAllanResult, SidereonStatus> {
    let raw = require_slice(points, count, fn_name, arg_name)?;
    let mut result = CoreAllanResult {
        tau_s: Vec::with_capacity(raw.len()),
        deviation: Vec::with_capacity(raw.len()),
        n: Vec::with_capacity(raw.len()),
    };
    for point in raw {
        result.tau_s.push(point.tau_s);
        result.deviation.push(point.deviation);
        result.n.push(point.n);
    }
    Ok(result)
}

unsafe fn power_law_options_from_c(
    fn_name: &str,
    options: *const SidereonPowerLawNoiseOptions,
    adev: &CoreAllanResult,
) -> Result<PowerLawNoiseOptions, SidereonStatus> {
    if options.is_null() {
        let basic_tau_s = adev.tau_s.first().copied().unwrap_or(1.0);
        return Ok(PowerLawNoiseOptions::sampled_at_nyquist(basic_tau_s));
    }
    let options = require_ref(options, fn_name, "options")?;
    let mut o = PowerLawNoiseOptions::new(options.basic_tau_s, options.measurement_bandwidth_hz);
    o.min_points_per_octave = options.min_points_per_octave;
    o.slope_tolerance = options.slope_tolerance;
    o.scatter_tolerance = options.scatter_tolerance;
    o.basic_tau_s = options.basic_tau_s;
    o.measurement_bandwidth_hz = options.measurement_bandwidth_hz;
    Ok(o)
}

fn power_law_options_to_c(options: PowerLawNoiseOptions) -> SidereonPowerLawNoiseOptions {
    SidereonPowerLawNoiseOptions {
        min_points_per_octave: options.min_points_per_octave,
        slope_tolerance: options.slope_tolerance,
        scatter_tolerance: options.scatter_tolerance,
        basic_tau_s: options.basic_tau_s,
        measurement_bandwidth_hz: options.measurement_bandwidth_hz,
    }
}

fn power_law_noise_type_from_c(
    fn_name: &str,
    value: u32,
) -> Result<PowerLawNoiseType, SidereonStatus> {
    match value {
        x if x == SidereonPowerLawNoiseType::RandomWalkFM as u32 => {
            Ok(PowerLawNoiseType::RandomWalkFM)
        }
        x if x == SidereonPowerLawNoiseType::FlickerFM as u32 => Ok(PowerLawNoiseType::FlickerFM),
        x if x == SidereonPowerLawNoiseType::WhiteFM as u32 => Ok(PowerLawNoiseType::WhiteFM),
        x if x == SidereonPowerLawNoiseType::FlickerPM as u32 => Ok(PowerLawNoiseType::FlickerPM),
        x if x == SidereonPowerLawNoiseType::WhitePM as u32 => Ok(PowerLawNoiseType::WhitePM),
        _ => {
            set_last_error(format!("{fn_name}: invalid power-law noise type"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn power_law_noise_type_to_c(value: PowerLawNoiseType) -> u32 {
    match value {
        PowerLawNoiseType::RandomWalkFM => SidereonPowerLawNoiseType::RandomWalkFM as u32,
        PowerLawNoiseType::FlickerFM => SidereonPowerLawNoiseType::FlickerFM as u32,
        PowerLawNoiseType::WhiteFM => SidereonPowerLawNoiseType::WhiteFM as u32,
        PowerLawNoiseType::FlickerPM => SidereonPowerLawNoiseType::FlickerPM as u32,
        PowerLawNoiseType::WhitePM => SidereonPowerLawNoiseType::WhitePM as u32,
    }
}

fn power_law_flag_to_c(value: PowerLawOctaveFlag) -> u32 {
    match value {
        PowerLawOctaveFlag::UnderSampled => SidereonPowerLawOctaveFlag::UnderSampled as u32,
        PowerLawOctaveFlag::DegenerateDeviation => {
            SidereonPowerLawOctaveFlag::DegenerateDeviation as u32
        }
        PowerLawOctaveFlag::MissingModifiedAllan => {
            SidereonPowerLawOctaveFlag::MissingModifiedAllan as u32
        }
    }
}

fn power_law_octave_to_c(octave: &PowerLawOctave) -> SidereonPowerLawOctave {
    let (dominance_kind, noise_type, flag) = match octave.dominance {
        PowerLawOctaveDominance::Dominant(noise_type) => (
            SidereonPowerLawOctaveDominanceKind::Dominant as u32,
            power_law_noise_type_to_c(noise_type),
            SidereonPowerLawOctaveFlag::UnderSampled as u32,
        ),
        PowerLawOctaveDominance::Ambiguous => (
            SidereonPowerLawOctaveDominanceKind::Ambiguous as u32,
            SidereonPowerLawNoiseType::RandomWalkFM as u32,
            SidereonPowerLawOctaveFlag::UnderSampled as u32,
        ),
        PowerLawOctaveDominance::Flagged(flag) => (
            SidereonPowerLawOctaveDominanceKind::Flagged as u32,
            SidereonPowerLawNoiseType::RandomWalkFM as u32,
            power_law_flag_to_c(flag),
        ),
    };
    SidereonPowerLawOctave {
        tau_start_s: octave.tau_start_s,
        tau_end_s: octave.tau_end_s,
        point_count: octave.point_count,
        has_adev_slope: octave.adev_slope.is_some(),
        adev_slope: octave.adev_slope.unwrap_or(0.0),
        has_mdev_slope: octave.mdev_slope.is_some(),
        mdev_slope: octave.mdev_slope.unwrap_or(0.0),
        has_slope_scatter: octave.slope_scatter.is_some(),
        slope_scatter: octave.slope_scatter.unwrap_or(0.0),
        dominance_kind,
        noise_type,
        flag,
    }
}

fn power_law_region_to_c(region: &PowerLawNoiseRegion) -> SidereonPowerLawNoiseRegion {
    SidereonPowerLawNoiseRegion {
        noise_type: power_law_noise_type_to_c(region.noise_type),
        tau_start_s: region.tau_start_s,
        tau_end_s: region.tau_end_s,
        octave_count: region.octave_count,
        point_count: region.point_count,
        mean_slope: region.mean_slope,
        coefficient: region.coefficient,
    }
}

fn power_law_noise_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn power_law_noise_error_value(error: &PowerLawNoiseError) -> Value {
    match error {
        PowerLawNoiseError::InvalidOptions { field, reason } => power_law_noise_node(
            "invalid_options",
            json!({
                "field": field,
                "reason": reason,
            }),
        ),
        PowerLawNoiseError::InvalidCurve { curve, reason } => power_law_noise_node(
            "invalid_curve",
            json!({
                "curve": curve,
                "reason": reason,
            }),
        ),
    }
}

fn map_power_law_noise_error(fn_name: &str, err: PowerLawNoiseError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PowerLawNoise,
        fn_name,
        power_law_noise_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, engine_f64, sidereon_last_engine_error_info,
        sidereon_last_engine_error_payload, SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use serde_json::Value;
    use std::ptr;

    #[test]
    fn test_allan_error_variants_payload_table() {
        let cases = vec![
            (AllanError::EmptySeries, "empty_series", json!({})),
            (
                AllanError::InvalidTau0 { tau0_s: 0.0 },
                "invalid_tau0",
                json!({
                    "tau0_s": engine_f64(0.0),
                }),
            ),
            (
                AllanError::InvalidTau0 { tau0_s: -0.0 },
                "invalid_tau0",
                json!({
                    "tau0_s": engine_f64(-0.0),
                }),
            ),
            (
                AllanError::InvalidTau0 { tau0_s: f64::NAN },
                "invalid_tau0",
                json!({
                    "tau0_s": engine_f64(f64::NAN),
                }),
            ),
            (
                AllanError::InvalidTau0 {
                    tau0_s: f64::INFINITY,
                },
                "invalid_tau0",
                json!({
                    "tau0_s": engine_f64(f64::INFINITY),
                }),
            ),
            (
                AllanError::InvalidTau0 {
                    tau0_s: f64::NEG_INFINITY,
                },
                "invalid_tau0",
                json!({
                    "tau0_s": engine_f64(f64::NEG_INFINITY),
                }),
            ),
            (AllanError::NoEstimators, "no_estimators", json!({})),
            (AllanError::EmptyTauGrid, "empty_tau_grid", json!({})),
            (
                AllanError::InvalidAveragingFactor {
                    averaging_factor: 0,
                },
                "invalid_averaging_factor",
                json!({
                    "averaging_factor": 0,
                }),
            ),
            (
                AllanError::TooFewSamples {
                    estimator: CoreAllanEstimator::Adev,
                    averaging_factor: 2,
                    available_phase_samples: 3,
                },
                "too_few_samples",
                json!({
                    "estimator": "adev",
                    "averaging_factor": 2,
                    "available_phase_samples": 3,
                }),
            ),
            (
                AllanError::TooFewSamples {
                    estimator: CoreAllanEstimator::OverlappingAdev,
                    averaging_factor: 4,
                    available_phase_samples: 5,
                },
                "too_few_samples",
                json!({
                    "estimator": "overlapping_adev",
                    "averaging_factor": 4,
                    "available_phase_samples": 5,
                }),
            ),
            (
                AllanError::TooFewSamples {
                    estimator: CoreAllanEstimator::Mdev,
                    averaging_factor: 8,
                    available_phase_samples: 9,
                },
                "too_few_samples",
                json!({
                    "estimator": "mdev",
                    "averaging_factor": 8,
                    "available_phase_samples": 9,
                }),
            ),
            (
                AllanError::TooFewSamples {
                    estimator: CoreAllanEstimator::Hdev,
                    averaging_factor: 16,
                    available_phase_samples: 17,
                },
                "too_few_samples",
                json!({
                    "estimator": "hdev",
                    "averaging_factor": 16,
                    "available_phase_samples": 17,
                }),
            ),
            (
                AllanError::TooFewSamples {
                    estimator: CoreAllanEstimator::Tdev,
                    averaging_factor: 32,
                    available_phase_samples: 33,
                },
                "too_few_samples",
                json!({
                    "estimator": "tdev",
                    "averaging_factor": 32,
                    "available_phase_samples": 33,
                }),
            ),
            (
                AllanError::NonFiniteSample { index: 42 },
                "non_finite_sample",
                json!({
                    "index": 42,
                }),
            ),
            (
                AllanError::Gap { index: 7 },
                "gap",
                json!({
                    "index": 7,
                }),
            ),
            (
                AllanError::NonFiniteTau {
                    estimator: CoreAllanEstimator::Adev,
                    averaging_factor: 12,
                },
                "non_finite_tau",
                json!({
                    "estimator": "adev",
                    "averaging_factor": 12,
                }),
            ),
            (
                AllanError::NonFiniteDeviation {
                    estimator: CoreAllanEstimator::Mdev,
                    averaging_factor: 24,
                },
                "non_finite_deviation",
                json!({
                    "estimator": "mdev",
                    "averaging_factor": 24,
                }),
            ),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let val = allan_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            assert_eq!(val["fields"], expected_fields);
        }
    }

    #[test]
    fn test_power_law_noise_error_variants_payload_table() {
        let cases = vec![
            (
                PowerLawNoiseError::InvalidOptions {
                    field: "min_points_per_octave",
                    reason: "must be at least 2",
                },
                "invalid_options",
                json!({
                    "field": "min_points_per_octave",
                    "reason": "must be at least 2",
                }),
            ),
            (
                PowerLawNoiseError::InvalidCurve {
                    curve: "ADEV",
                    reason: "tau values must be strictly increasing",
                },
                "invalid_curve",
                json!({
                    "curve": "ADEV",
                    "reason": "tau values must be strictly increasing",
                }),
            ),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let val = power_law_noise_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            assert_eq!(val["fields"], expected_fields);
        }
    }

    #[test]
    fn test_allan_producer_valid_and_refusal_controls() {
        clear_engine_error();

        // 1. Valid producer control: compute allan deviations with valid samples
        let sample_data: Vec<SidereonAllanSample> = (0..20)
            .map(|i| SidereonAllanSample {
                has_value: true,
                value: i as f64 * 1e-9,
            })
            .collect();
        let mut curves_handle: *mut SidereonAllanDeviationCurves = ptr::null_mut();
        let status = unsafe {
            sidereon_clock_compute_allan_deviations(
                sample_data.as_ptr(),
                sample_data.len(),
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                &mut curves_handle,
            )
        };
        if status != SidereonStatus::Ok {
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            let _ = unsafe { sidereon_last_engine_error_info(&mut info) };
            let mut err_msg_buf = [0 as c_char; 512];
            let len = unsafe {
                crate::sidereon_last_error_message(err_msg_buf.as_mut_ptr(), err_msg_buf.len())
            };
            let msg = if len > 0 {
                unsafe {
                    std::ffi::CStr::from_ptr(err_msg_buf.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                }
            } else {
                String::new()
            };
            panic!(
                "sidereon_clock_compute_allan_deviations failed: status={:?}, message={:?}, family={:?}",
                status, msg, info.family
            );
        }
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!curves_handle.is_null());

        // Check TLS is None on success
        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Unknown,
            payload_len: 12345,
        };
        let info_status = unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info_status, SidereonStatus::Ok);
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // Non-producing inspectors and readers retain TLS.
        // Default standard options compute OverlappingAdev (overlapping_adev: true, adev: false).
        let mut present = false;
        let p_status = unsafe {
            sidereon_clock_allan_curve_present(
                curves_handle,
                SidereonAllanEstimator::OverlappingAdev as u32,
                &mut present,
            )
        };
        assert_eq!(p_status, SidereonStatus::Ok);
        assert!(present);

        let mut adev_present = true;
        let adev_p_status = unsafe {
            sidereon_clock_allan_curve_present(
                curves_handle,
                SidereonAllanEstimator::Adev as u32,
                &mut adev_present,
            )
        };
        assert_eq!(adev_p_status, SidereonStatus::Ok);
        assert!(!adev_present);

        let mut written = 0usize;
        let mut required = 0usize;
        let c_status = unsafe {
            sidereon_clock_allan_curve(
                curves_handle,
                SidereonAllanEstimator::OverlappingAdev as u32,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(c_status, SidereonStatus::Ok);
        assert!(required > 0);

        unsafe { sidereon_clock_allan_deviation_curves_free(curves_handle) };

        // 2. Real public refusal control: empty series
        let mut bad_curves: *mut SidereonAllanDeviationCurves = ptr::null_mut();
        let status_empty = unsafe {
            sidereon_clock_compute_allan_deviations(
                ptr::null(),
                0,
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                &mut bad_curves,
            )
        };
        assert_eq!(status_empty, SidereonStatus::InvalidArgument);
        assert!(bad_curves.is_null());

        let info_status = unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info_status, SidereonStatus::Ok);
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);
        assert!(info.payload_len > 0);

        let mut buf = vec![0u8; info.payload_len];
        let p_status = unsafe {
            sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(p_status, SidereonStatus::Ok);
        assert_eq!(written, info.payload_len);
        let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "allan");
        assert_eq!(
            payload["operation"],
            "sidereon_clock_compute_allan_deviations"
        );
        assert_eq!(payload["error"]["kind"], "empty_series");

        // 3. Real public refusal control: TooFewSamples via sidereon_clock_allan_deviation
        let small_samples = [
            SidereonAllanSample {
                has_value: true,
                value: 0.0,
            },
            SidereonAllanSample {
                has_value: true,
                value: 1e-9,
            },
            SidereonAllanSample {
                has_value: true,
                value: 2e-9,
            },
        ];
        let factor = 10usize;
        let mut out_pts = [SidereonAllanPoint {
            tau_s: 0.0,
            deviation: 0.0,
            n: 0,
        }; 2];
        let dev_status = unsafe {
            sidereon_clock_allan_deviation(
                small_samples.as_ptr(),
                small_samples.len(),
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                &factor,
                1,
                out_pts.as_mut_ptr(),
                out_pts.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(dev_status, SidereonStatus::InvalidArgument);
        let info_status = unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info_status, SidereonStatus::Ok);
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);

        let mut buf = vec![0u8; info.payload_len];
        unsafe {
            sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            )
        };
        let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
        assert_eq!(payload["family"], "allan");
        assert_eq!(payload["operation"], "sidereon_clock_allan_deviation");
        assert_eq!(payload["error"]["kind"], "too_few_samples");
        assert_eq!(payload["error"]["fields"]["estimator"], "adev");
        assert_eq!(payload["error"]["fields"]["averaging_factor"], 10);
        assert_eq!(payload["error"]["fields"]["available_phase_samples"], 3);
    }

    #[test]
    fn test_power_law_producer_valid_and_refusal_controls() {
        clear_engine_error();

        // Points for ADEV and MDEV
        let adev_points = [
            SidereonAllanPoint {
                tau_s: 1.0,
                deviation: 1e-12,
                n: 100,
            },
            SidereonAllanPoint {
                tau_s: 2.0,
                deviation: 1e-12,
                n: 100,
            },
            SidereonAllanPoint {
                tau_s: 4.0,
                deviation: 1e-12,
                n: 100,
            },
        ];
        let mdev_points = [
            SidereonAllanPoint {
                tau_s: 1.0,
                deviation: 1e-12,
                n: 100,
            },
            SidereonAllanPoint {
                tau_s: 2.0,
                deviation: 1e-12,
                n: 100,
            },
            SidereonAllanPoint {
                tau_s: 4.0,
                deviation: 1e-12,
                n: 100,
            },
        ];

        let mut options = SidereonPowerLawNoiseOptions {
            min_points_per_octave: 2,
            slope_tolerance: 0.1,
            scatter_tolerance: 0.2,
            basic_tau_s: 1.0,
            measurement_bandwidth_hz: 10.0,
        };

        // 1. Valid producer control: fit power law noise
        let mut fit_handle: *mut SidereonPowerLawNoiseFit = ptr::null_mut();
        let status = unsafe {
            sidereon_clock_fit_power_law_noise(
                adev_points.as_ptr(),
                adev_points.len(),
                mdev_points.as_ptr(),
                mdev_points.len(),
                &options,
                &mut fit_handle,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!fit_handle.is_null());

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Unknown,
            payload_len: 999,
        };
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // Non-producing getters and free
        let mut coeffs = [0.0; 5];
        let c_status = unsafe {
            sidereon_clock_power_law_noise_fit_coefficients(fit_handle, coeffs.as_mut_ptr())
        };
        assert_eq!(c_status, SidereonStatus::Ok);

        let mut written = 0usize;
        let mut required = 0usize;
        let oct_status = unsafe {
            sidereon_clock_power_law_noise_fit_octaves(
                fit_handle,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(oct_status, SidereonStatus::Ok);

        unsafe { sidereon_clock_power_law_noise_fit_free(fit_handle) };

        // 2. Real public refusal control: invalid options
        options.min_points_per_octave = 1; // core requires >= 2
        let mut bad_fit: *mut SidereonPowerLawNoiseFit = ptr::null_mut();
        let status_bad_opt = unsafe {
            sidereon_clock_fit_power_law_noise(
                adev_points.as_ptr(),
                adev_points.len(),
                mdev_points.as_ptr(),
                mdev_points.len(),
                &options,
                &mut bad_fit,
            )
        };
        assert_eq!(status_bad_opt, SidereonStatus::InvalidArgument);
        assert!(bad_fit.is_null());

        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::PowerLawNoise);
        assert!(info.payload_len > 0);

        let mut buf = vec![0u8; info.payload_len];
        unsafe {
            sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            )
        };
        let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "power_law_noise");
        assert_eq!(payload["operation"], "sidereon_clock_fit_power_law_noise");
        assert_eq!(payload["error"]["kind"], "invalid_options");
        assert_eq!(payload["error"]["fields"]["field"], "min_points_per_octave");
        assert_eq!(payload["error"]["fields"]["reason"], "must be at least 2");

        // 3. Real public refusal control: invalid curve (non-increasing tau)
        let decreasing_adev = [
            SidereonAllanPoint {
                tau_s: 2.0,
                deviation: 1e-12,
                n: 100,
            },
            SidereonAllanPoint {
                tau_s: 1.0,
                deviation: 1e-12,
                n: 100,
            },
        ];
        options.min_points_per_octave = 2;
        let status_bad_curve = unsafe {
            sidereon_clock_fit_power_law_noise(
                decreasing_adev.as_ptr(),
                decreasing_adev.len(),
                mdev_points.as_ptr(),
                mdev_points.len(),
                &options,
                &mut bad_fit,
            )
        };
        assert_eq!(status_bad_curve, SidereonStatus::InvalidArgument);
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::PowerLawNoise);

        let mut buf = vec![0u8; info.payload_len];
        unsafe {
            sidereon_last_engine_error_payload(
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
                &mut required,
            )
        };
        let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
        assert_eq!(payload["error"]["kind"], "invalid_curve");
        assert_eq!(payload["error"]["fields"]["curve"], "ADEV");
        assert_eq!(
            payload["error"]["fields"]["reason"],
            "tau values must be strictly increasing"
        );
    }

    #[test]
    fn test_clock_engine_error_lifecycle_buffer_sizing_and_clearing() {
        clear_engine_error();

        // Cause an Allan refusal
        let mut curves: *mut SidereonAllanDeviationCurves = ptr::null_mut();
        let status = unsafe {
            sidereon_clock_compute_allan_deviations(
                ptr::null(),
                0,
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                &mut curves,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Unknown,
            payload_len: 0,
        };
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);
        let needed = info.payload_len;
        assert!(needed > 0);

        // 1. Query size with null buffer
        let mut written = 999;
        let mut required = 0;
        let q_status = unsafe {
            sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
        };
        assert_eq!(q_status, SidereonStatus::Ok);
        assert_eq!(written, 0);
        assert_eq!(required, needed);

        // 2. Undersized buffer returns InvalidArgument, writes zero, reports full required,
        // does not copy bytes, and retains error
        let mut short_buf = vec![0xAAu8; needed / 2];
        let short_len = short_buf.len();
        let u_status = unsafe {
            sidereon_last_engine_error_payload(
                short_buf.as_mut_ptr(),
                short_len,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(u_status, SidereonStatus::InvalidArgument);
        assert_eq!(written, 0);
        assert_eq!(required, needed);
        assert!(short_buf.iter().all(|&b| b == 0xAA));

        // Error is still retained!
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);
        assert_eq!(info.payload_len, needed);

        // 3. Exact copy succeeds and retains error
        let mut full_buf = vec![0u8; needed];
        let f_status = unsafe {
            sidereon_last_engine_error_payload(
                full_buf.as_mut_ptr(),
                needed,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(f_status, SidereonStatus::Ok);
        assert_eq!(written, needed);
        assert_eq!(required, needed);

        // Error is still retained!
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);

        // 4. Non-producing readers/inspectors/frees retain error
        let mut adev_slope = 0.0;
        let mut mdev_slope = 0.0;
        let mut exp = 0;
        let slope_status = unsafe {
            sidereon_clock_power_law_noise_slopes(
                SidereonPowerLawNoiseType::WhiteFM as u32,
                &mut adev_slope,
                &mut mdev_slope,
                &mut exp,
            )
        };
        assert_eq!(slope_status, SidereonStatus::Ok);
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);

        unsafe { sidereon_clock_allan_deviation_curves_free(ptr::null_mut()) };
        unsafe { sidereon_clock_power_law_noise_fit_free(ptr::null_mut()) };
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);

        // 5. Early argument validation failure clears TLS
        let null_arg_status = unsafe {
            sidereon_clock_compute_allan_deviations(
                ptr::null(),
                0,
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                ptr::null_mut(), // null out_curves
            )
        };
        assert_eq!(null_arg_status, SidereonStatus::NullPointer);
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 6. Successful producer clears TLS
        // Re-introduce an error first
        let _ = unsafe {
            sidereon_clock_compute_allan_deviations(
                ptr::null(),
                0,
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                &mut curves,
            )
        };
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::Allan);

        // Now run a successful producer
        let samples: Vec<SidereonAllanSample> = (0..20)
            .map(|i| SidereonAllanSample {
                has_value: true,
                value: i as f64 * 1e-9,
            })
            .collect();
        let succ_status = unsafe {
            sidereon_clock_compute_allan_deviations(
                samples.as_ptr(),
                samples.len(),
                SidereonAllanSeriesKind::PhaseSeconds as u32,
                1.0,
                ptr::null(),
                &mut curves,
            )
        };
        assert_eq!(succ_status, SidereonStatus::Ok);
        unsafe { sidereon_last_engine_error_info(&mut info) };
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        unsafe { sidereon_clock_allan_deviation_curves_free(curves) };
    }
}
