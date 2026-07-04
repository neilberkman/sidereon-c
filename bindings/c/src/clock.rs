use super::*;

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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
    Ok(AllanOptions {
        estimators: allan_estimator_set_from_c(options.estimators),
        tau_grid: allan_tau_grid_from_c(fn_name, options)?,
        gap_policy: allan_gap_policy_from_c(fn_name, options.gap_policy)?,
    })
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

fn map_allan_error(fn_name: &str, err: AllanError) -> SidereonStatus {
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
