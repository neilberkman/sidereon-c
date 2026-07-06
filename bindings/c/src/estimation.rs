use super::*;

// --- Quality remainder (sidereon_core::quality) ------------------------------

/// One satellite/elevation entry for sigma and weight maps, mirroring
/// sidereon_core::quality::WeightEntry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonWeightEntry {
    /// Null-terminated satellite token, for example G01.
    pub sat_id: *const c_char,
    /// Topocentric elevation, degrees.
    pub elevation_deg: f64,
    /// Whether cn0_dbhz carries a value (selects the C/N0 model for this entry).
    pub has_cn0: bool,
    /// Carrier-to-noise density, dB-Hz, used when has_cn0 is true.
    pub cn0_dbhz: f64,
}

/// Per-satellite inverse-variance weight for elevation/C-N0 entries. Writes one
/// value/present pair per input entry (see sidereon_sigmas for the contract).
/// Delegates to sidereon_core::quality::weight_vector.
///
/// Safety: entries points to count SidereonWeightEntry; options to a
/// SidereonPseudorangeVarianceOptions; out_values to count doubles; out_present
/// to count bools.
#[no_mangle]
pub unsafe extern "C" fn sidereon_weight_vector(
    entries: *const SidereonWeightEntry,
    count: usize,
    options: *const SidereonPseudorangeVarianceOptions,
    out_values: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_weight_vector", SidereonStatus::Panic, || {
        let out_values = c_try!(require_out(
            out_values,
            "sidereon_weight_vector",
            "out_values"
        ));
        let out_values = out_values as *mut f64;
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_weight_vector",
            "out_present"
        ));
        let out_present = out_present as *mut bool;
        c_try!(validate_element_count::<f64>(
            "sidereon_weight_vector",
            "count",
            count
        ));
        for idx in 0..count {
            *out_values.add(idx) = 0.0;
            *out_present.add(idx) = false;
        }
        let options = c_try!(require_ref(options, "sidereon_weight_vector", "options"));
        let opts = c_try!(pseudorange_variance_options_from_c(
            "sidereon_weight_vector",
            options
        ));
        let entries = c_try!(weight_entries_from_c(
            "sidereon_weight_vector",
            entries,
            count
        ));
        let map = sidereon_core::quality::weight_vector(&entries, opts);
        write_entry_map_positional(&entries, &map, out_values, out_present);
        SidereonStatus::Ok
    })
}

// --- Jacobian-derived covariance / Hessian trace / error ellipse ------------

/// A 2D confidence ellipse from a 2x2 covariance block. Mirrors the core
/// `ErrorEllipse2`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonErrorEllipse2 {
    /// Confidence level used to scale the axes, in `[0, 1)`.
    pub confidence: f64,
    /// Two-DOF chi-square quantile `-2 ln(1 - confidence)`.
    pub chi_square_scale: f64,
    /// Semi-major axis length.
    pub semi_major: f64,
    /// Semi-minor axis length.
    pub semi_minor: f64,
    /// Orientation of the major axis, radians.
    pub orientation_rad: f64,
}

/// Parameter covariance `variance_scale * (J^T J)^-1` from a row-major `m`-by-`n`
/// design (Jacobian) matrix, written row-major (`n * n`) into out. Delegates to
/// the core `normal_covariance` (SVD of J, so the conditioning is `cond(J)`, not
/// `cond(J)^2`). A rank-deficient Jacobian returns SIDEREON_STATUS_SOLVE. Same
/// variable-length output contract as the other array readers (query with out
/// NULL, len 0).
///
/// Safety: jacobian must point to m*n readable doubles (or be NULL when m*n is
/// 0); out must point to at least len writable doubles or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_normal_covariance(
    jacobian: *const f64,
    m: usize,
    n: usize,
    variance_scale: f64,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_normal_covariance", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_normal_covariance",
            out_written,
            out_required
        ));
        let count = c_try!(m.checked_mul(n).ok_or_else(|| {
            set_last_error("sidereon_normal_covariance: m*n overflows".to_string());
            SidereonStatus::InvalidArgument
        }));
        let data = c_try!(require_slice(
            jacobian,
            count,
            "sidereon_normal_covariance",
            "jacobian"
        ));
        let jac = DMatrix::from_row_slice(m, n, data);
        let cov = match core_normal_covariance(&jac, variance_scale) {
            Ok(cov) => cov,
            Err(err) => return map_lsq_error("sidereon_normal_covariance", err),
        };
        // nalgebra stores column-major; transpose-iterate to emit row-major.
        let row_major: Vec<f64> = cov
            .row_iter()
            .flat_map(|r| r.iter().copied().collect::<Vec<_>>())
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_normal_covariance",
            "out",
            &row_major,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Trace of the Gauss-Newton Hessian approximation `J^T J` (the sum of squared
/// column norms) of a row-major `m`-by-`n` Jacobian, written to *out. Delegates
/// to the core `hessian_trace`; forms no inverse.
///
/// Safety: jacobian must point to m*n readable doubles (or be NULL when m*n is
/// 0); out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_hessian_trace(
    jacobian: *const f64,
    m: usize,
    n: usize,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_hessian_trace", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_hessian_trace", "out"));
        *out = 0.0;
        let count = c_try!(m.checked_mul(n).ok_or_else(|| {
            set_last_error("sidereon_hessian_trace: m*n overflows".to_string());
            SidereonStatus::InvalidArgument
        }));
        let data = c_try!(require_slice(
            jacobian,
            count,
            "sidereon_hessian_trace",
            "jacobian"
        ));
        let jac = DMatrix::from_row_slice(m, n, data);
        *out = core_hessian_trace(&jac);
        SidereonStatus::Ok
    })
}

/// Confidence ellipse from a 2x2 covariance block (row-major `[c00, c01, c10,
/// c11]`) at `confidence` in `[0, 1)`, written to *out_ellipse. Delegates to the
/// core `error_ellipse_2x2` (closed-form symmetric 2x2 eigensolve). A
/// non-positive-semidefinite block or out-of-range confidence returns
/// SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: covariance must point to 4 readable doubles; out_ellipse must point
/// to a SidereonErrorEllipse2.
#[no_mangle]
pub unsafe extern "C" fn sidereon_error_ellipse_2x2(
    covariance: *const f64,
    confidence: f64,
    out_ellipse: *mut SidereonErrorEllipse2,
) -> SidereonStatus {
    ffi_boundary("sidereon_error_ellipse_2x2", SidereonStatus::Panic, || {
        let out_ellipse = c_try!(require_out(
            out_ellipse,
            "sidereon_error_ellipse_2x2",
            "out_ellipse"
        ));
        let cov = c_try!(require_slice(
            covariance,
            4,
            "sidereon_error_ellipse_2x2",
            "covariance"
        ));
        let block = [[cov[0], cov[1]], [cov[2], cov[3]]];
        let ellipse = match core_error_ellipse_2x2(block, confidence) {
            Ok(ellipse) => ellipse,
            Err(err) => return map_dop_error("sidereon_error_ellipse_2x2", err),
        };
        *out_ellipse = SidereonErrorEllipse2 {
            confidence: ellipse.confidence,
            chi_square_scale: ellipse.chi_square_scale,
            semi_major: ellipse.semi_major,
            semi_minor: ellipse.semi_minor,
            orientation_rad: ellipse.orientation_rad,
        };
        SidereonStatus::Ok
    })
}

// --- Residual-distribution statistics ---------------------------------------

/// Sample mean, variance, skewness, and kurtosis of a residual set. Mirrors the
/// core `MomentStats`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonResidualMoments {
    /// Arithmetic mean.
    pub mean: f64,
    /// Population (biased) variance, the second central moment.
    pub variance: f64,
    /// Sample skewness (biased or bias-corrected per the request).
    pub skewness: f64,
    /// Sample kurtosis: excess (Gaussian -> 0) when fisher, Pearson otherwise.
    pub kurtosis_excess: f64,
}

/// Jarque-Bera normality test result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonJarqueBera {
    /// Test statistic `n/6 * (S^2 + K^2/4)`.
    pub statistic: f64,
    /// Upper-tail chi-square(2) p-value `exp(-statistic/2)`.
    pub p_value: f64,
}

/// Shapiro-Wilk normality test result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonShapiroWilk {
    /// The `W` statistic in `(0, 1]`.
    pub w: f64,
    /// Upper-tail p-value for the normality null.
    pub p_value: f64,
}

/// Sample skewness of a residual set, written to *out. `bias = true` is the
/// Fisher-Pearson coefficient `g1` (scipy.stats.skew default); `bias = false`
/// applies the sample correction (needs at least three values). Delegates to the
/// core `skewness`.
///
/// Safety: x must point to len readable doubles (or be NULL when len is 0); out
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_residual_skewness(
    x: *const f64,
    len: usize,
    bias: bool,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_residual_skewness", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_residual_skewness", "out"));
        *out = 0.0;
        let x = c_try!(require_slice(x, len, "sidereon_residual_skewness", "x"));
        match core_skewness(x, bias) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => map_normality_error("sidereon_residual_skewness", err),
        }
    })
}

/// Sample kurtosis of a residual set, written to *out. `fisher = true` returns
/// excess kurtosis (Gaussian -> 0, scipy default); `fisher = false` the Pearson
/// kurtosis. `bias = false` applies the sample correction (needs at least four
/// values). Delegates to the core `kurtosis`.
///
/// Safety: x must point to len readable doubles (or be NULL when len is 0); out
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_residual_kurtosis(
    x: *const f64,
    len: usize,
    fisher: bool,
    bias: bool,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_residual_kurtosis", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_residual_kurtosis", "out"));
        *out = 0.0;
        let x = c_try!(require_slice(x, len, "sidereon_residual_kurtosis", "x"));
        match core_kurtosis(x, fisher, bias) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => map_normality_error("sidereon_residual_kurtosis", err),
        }
    })
}

/// Mean, variance, skewness, and kurtosis of a residual set in one pass, written
/// to *out_moments. `fisher`/`bias` select the kurtosis convention and the
/// bias correction as in sidereon_residual_skewness/kurtosis. Delegates to the
/// core `moments`.
///
/// Safety: x must point to len readable doubles (or be NULL when len is 0);
/// out_moments must point to a SidereonResidualMoments.
#[no_mangle]
pub unsafe extern "C" fn sidereon_residual_moments(
    x: *const f64,
    len: usize,
    fisher: bool,
    bias: bool,
    out_moments: *mut SidereonResidualMoments,
) -> SidereonStatus {
    ffi_boundary("sidereon_residual_moments", SidereonStatus::Panic, || {
        let out_moments = c_try!(require_out(
            out_moments,
            "sidereon_residual_moments",
            "out_moments"
        ));
        *out_moments = SidereonResidualMoments {
            mean: 0.0,
            variance: 0.0,
            skewness: 0.0,
            kurtosis_excess: 0.0,
        };
        let x = c_try!(require_slice(x, len, "sidereon_residual_moments", "x"));
        match core_moments(x, fisher, bias) {
            Ok(stats) => {
                *out_moments = SidereonResidualMoments {
                    mean: stats.mean,
                    variance: stats.variance,
                    skewness: stats.skewness,
                    kurtosis_excess: stats.kurtosis_excess,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_normality_error("sidereon_residual_moments", err),
        }
    })
}

/// Jarque-Bera normality test on a residual set, written to *out. Uses the
/// biased skewness and excess kurtosis (scipy.stats.jarque_bera). Needs at least
/// two values. Delegates to the core `jarque_bera`.
///
/// Safety: x must point to len readable doubles (or be NULL when len is 0); out
/// must point to a SidereonJarqueBera.
#[no_mangle]
pub unsafe extern "C" fn sidereon_residual_jarque_bera(
    x: *const f64,
    len: usize,
    out: *mut SidereonJarqueBera,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_residual_jarque_bera",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_residual_jarque_bera", "out"));
            *out = SidereonJarqueBera {
                statistic: 0.0,
                p_value: 0.0,
            };
            let x = c_try!(require_slice(x, len, "sidereon_residual_jarque_bera", "x"));
            match core_jarque_bera(x) {
                Ok(jb) => {
                    *out = SidereonJarqueBera {
                        statistic: jb.statistic,
                        p_value: jb.p_value,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => map_normality_error("sidereon_residual_jarque_bera", err),
            }
        },
    )
}

/// Shapiro-Wilk W normality test on a residual set, written to *out (Royston AS
/// R94, the scipy.stats.shapiro algorithm). Needs at least three values; all
/// equal values return SIDEREON_STATUS_INVALID_ARGUMENT. Delegates to the core
/// `shapiro_wilk`.
///
/// Safety: x must point to len readable doubles (or be NULL when len is 0); out
/// must point to a SidereonShapiroWilk.
#[no_mangle]
pub unsafe extern "C" fn sidereon_residual_shapiro_wilk(
    x: *const f64,
    len: usize,
    out: *mut SidereonShapiroWilk,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_residual_shapiro_wilk",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_residual_shapiro_wilk", "out"));
            *out = SidereonShapiroWilk {
                w: 0.0,
                p_value: 0.0,
            };
            let x = c_try!(require_slice(x, len, "sidereon_residual_shapiro_wilk", "x"));
            match core_shapiro_wilk(x) {
                Ok(sw) => {
                    *out = SidereonShapiroWilk {
                        w: sw.w,
                        p_value: sw.p_value,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => map_normality_error("sidereon_residual_shapiro_wilk", err),
            }
        },
    )
}

/// MAD Gaussian consistency factor used by sidereon_mad_spread.
pub const SIDEREON_MAD_GAUSSIAN_CONSISTENCY: f64 = core_estimation::MAD_GAUSSIAN_CONSISTENCY;

/// State of one scalar level and rate alpha-beta channel.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAlphaBetaState {
    /// Level estimate, in caller-chosen units.
    pub level: f64,
    /// Rate estimate, in caller-chosen units per second.
    pub rate: f64,
}

/// Alpha-beta gain set for one scalar channel.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAlphaBetaGains {
    /// Level gain alpha.
    pub alpha: f64,
    /// Rate gain beta. The update applies beta * innovation / dt.
    pub beta: f64,
}

/// One alpha-beta predict and update result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAlphaBetaStep {
    /// Predicted state before applying the measurement.
    pub predicted: SidereonAlphaBetaState,
    /// Updated state after applying the measurement.
    pub updated: SidereonAlphaBetaState,
    /// Measurement innovation, measurement minus predicted level.
    pub innovation: f64,
}

/// Steady-state scalar constant-velocity Kalman gain set.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScalarKalmanGains {
    /// Position gain Kx.
    pub position_gain: f64,
    /// Rate gain Kv, in 1 / dt units.
    pub rate_gain: f64,
}

/// Result of a normalized innovation squared gate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNisGate {
    /// Normalized innovation squared statistic.
    pub nis: f64,
    /// Chi-square gate threshold.
    pub threshold: f64,
    /// True when nis is less than or equal to threshold.
    pub in_gate: bool,
    /// Measurement degrees of freedom.
    pub dof: usize,
}

/// Cartesian frame for no-IMU track filtering.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTrackCoordinateFrame {
    /// Earth-Centered-Earth-Fixed position in metres.
    Ecef = 0,
    /// Local East-North-Up position in metres.
    Enu = 1,
    /// Caller-defined fixed Cartesian axes in metres.
    CallerDefinedCartesian = 2,
}

/// Opaque no-IMU track filter config. Create with
/// sidereon_track_filter_config_from_position or
/// sidereon_track_filter_config_from_position_velocity and release with
/// sidereon_track_filter_config_free.
pub struct SidereonTrackFilterConfig {
    pub(crate) inner: core_estimation::TrackFilterConfig,
}

/// Opaque stateful no-IMU track filter. Create with sidereon_track_filter_new
/// or sidereon_track_filter_new_from_position and release with
/// sidereon_track_filter_free.
pub struct SidereonTrackFilter {
    pub(crate) inner: core_estimation::TrackFilter,
}

/// Opaque RTS history builder for recorded track filtering. Create with
/// sidereon_track_rts_history_builder_new or
/// sidereon_track_rts_history_builder_from_filter and release with
/// sidereon_track_rts_history_builder_free.
pub struct SidereonTrackRtsHistoryBuilder {
    pub(crate) inner: core_estimation::TrackRtsHistoryBuilder,
}

/// Opaque finished RTS history. Create with
/// sidereon_track_rts_history_builder_finish and release with
/// sidereon_track_rts_history_free.
pub struct SidereonTrackRtsHistory {
    pub(crate) inner: core_estimation::TrackRtsHistory,
}

/// Opaque fixed-interval RTS smoothed track. Create with sidereon_smooth_track_rts
/// and release with sidereon_smoothed_track_free.
pub struct SidereonSmoothedTrack {
    pub(crate) inner: core_estimation::SmoothedTrack,
}

/// Track state metadata. Position, velocity, state-vector, and covariance arrays
/// are copied with the corresponding state reader functions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackState {
    /// One of SidereonTrackCoordinateFrame encoded as uint32_t.
    pub frame: u32,
    /// Epoch seconds in the caller's monotonic time base.
    pub t_s: f64,
    /// Position dimension.
    pub dimension: usize,
    /// Full state dimension, position plus velocity.
    pub state_dimension: usize,
}

/// Prediction report from a no-IMU track filter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackPrediction {
    /// Propagation step, seconds.
    pub dt_s: f64,
    /// Predicted state metadata after the propagation.
    pub predicted: SidereonTrackState,
}

/// Innovation report for a pending or applied track update.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackInnovation {
    /// Measurement dimension.
    pub dimension: usize,
    /// Normalized innovation squared.
    pub nis: f64,
}

/// Covariance-weighted track update report.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackUpdate {
    /// State before applying the correction.
    pub predicted: SidereonTrackState,
    /// State after applying the correction.
    pub updated: SidereonTrackState,
    /// Innovation statistics for the correction.
    pub innovation: SidereonTrackInnovation,
}

/// Gated track update report.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackGatedUpdate {
    /// NIS gate result.
    pub gate: SidereonNisGate,
    /// True when update contains an accepted correction.
    pub has_update: bool,
    /// Accepted correction report when has_update is true; zeroed otherwise.
    pub update: SidereonTrackUpdate,
    /// Current filter state metadata after the gated operation.
    pub state: SidereonTrackState,
}

/// One finished RTS history epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrackRtsEpoch {
    /// Epoch seconds.
    pub t_s: f64,
    /// Prediction carried by the epoch.
    pub predicted: SidereonTrackState,
    /// Update carried by the epoch.
    pub updated: SidereonTrackState,
    /// Whether transition_from_previous is present for this epoch.
    pub has_transition_from_previous: bool,
}

/// One smoothed RTS output epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSmoothedTrackEpoch {
    /// Epoch seconds.
    pub t_s: f64,
    /// Smoothed state metadata.
    pub state: SidereonTrackState,
    /// Whether rts_gain_to_next is present for this epoch.
    pub has_rts_gain_to_next: bool,
}

/// Return the MAD Gaussian consistency factor, 1 / Phi^-1(3/4).
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mad_gaussian_consistency(out: *mut f64) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mad_gaussian_consistency",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_mad_gaussian_consistency", "out"));
            *out = core_estimation::MAD_GAUSSIAN_CONSISTENCY;
            SidereonStatus::Ok
        },
    )
}

/// Compute steady-state alpha-beta gains from a positive dimensionless tracking
/// index.
///
/// Safety: out_gains must point to a SidereonAlphaBetaGains.
#[no_mangle]
pub unsafe extern "C" fn sidereon_alpha_beta_steady_state_gains(
    tracking_index: f64,
    out_gains: *mut SidereonAlphaBetaGains,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_alpha_beta_steady_state_gains",
        SidereonStatus::Panic,
        || {
            let out_gains = c_try!(require_out(
                out_gains,
                "sidereon_alpha_beta_steady_state_gains",
                "out_gains"
            ));
            *out_gains = SidereonAlphaBetaGains {
                alpha: 0.0,
                beta: 0.0,
            };
            match core_estimation::alpha_beta_steady_state_gains(tracking_index) {
                Ok(gains) => {
                    *out_gains = alpha_beta_gains_to_c(gains);
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_alpha_beta_steady_state_gains", err),
            }
        },
    )
}

/// Run one scalar alpha-beta predict and measurement update.
///
/// Safety: state, gains, and out_step must point to live structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_alpha_beta_filter_step(
    state: *const SidereonAlphaBetaState,
    measurement: f64,
    dt: f64,
    gains: *const SidereonAlphaBetaGains,
    out_step: *mut SidereonAlphaBetaStep,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_alpha_beta_filter_step",
        SidereonStatus::Panic,
        || {
            let state = c_try!(require_ref(
                state,
                "sidereon_alpha_beta_filter_step",
                "state"
            ));
            let gains = c_try!(require_ref(
                gains,
                "sidereon_alpha_beta_filter_step",
                "gains"
            ));
            let out_step = c_try!(require_out(
                out_step,
                "sidereon_alpha_beta_filter_step",
                "out_step"
            ));
            *out_step = SidereonAlphaBetaStep {
                predicted: SidereonAlphaBetaState {
                    level: 0.0,
                    rate: 0.0,
                },
                updated: SidereonAlphaBetaState {
                    level: 0.0,
                    rate: 0.0,
                },
                innovation: 0.0,
            };
            match core_estimation::alpha_beta_filter_step(
                alpha_beta_state_from_c(*state),
                measurement,
                dt,
                alpha_beta_gains_from_c(*gains),
            ) {
                Ok(step) => {
                    *out_step = SidereonAlphaBetaStep {
                        predicted: alpha_beta_state_to_c(step.predicted),
                        updated: alpha_beta_state_to_c(step.updated),
                        innovation: step.innovation,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_alpha_beta_filter_step", err),
            }
        },
    )
}

/// Compute steady-state gains for a scalar constant-velocity Kalman filter.
///
/// Safety: out_gains must point to a SidereonScalarKalmanGains.
#[no_mangle]
pub unsafe extern "C" fn sidereon_kalman_cv_steady_state_gains(
    tracking_index: f64,
    dt: f64,
    measurement_variance: f64,
    out_gains: *mut SidereonScalarKalmanGains,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_kalman_cv_steady_state_gains",
        SidereonStatus::Panic,
        || {
            let out_gains = c_try!(require_out(
                out_gains,
                "sidereon_kalman_cv_steady_state_gains",
                "out_gains"
            ));
            *out_gains = SidereonScalarKalmanGains {
                position_gain: 0.0,
                rate_gain: 0.0,
            };
            match core_estimation::kalman_cv_steady_state_gains(
                tracking_index,
                dt,
                measurement_variance,
            ) {
                Ok(gains) => {
                    *out_gains = scalar_kalman_gains_to_c(gains);
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_kalman_cv_steady_state_gains", err),
            }
        },
    )
}

/// Scalar normalized innovation, innovation divided by sqrt(variance).
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_normalized_innovation(
    innovation: f64,
    innovation_variance: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_normalized_innovation",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_normalized_innovation", "out"));
            *out = 0.0;
            match core_estimation::normalized_innovation(innovation, innovation_variance) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_normalized_innovation", err),
            }
        },
    )
}

/// Expected NIS value for a measurement dimension.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nis_expected_value(dof: usize, out: *mut f64) -> SidereonStatus {
    ffi_boundary("sidereon_nis_expected_value", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_nis_expected_value", "out"));
        *out = 0.0;
        match core_estimation::nis_expected_value(dof) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_nis_expected_value", err),
        }
    })
}

/// Chi-square NIS gate threshold for a confidence in (0, 1) and positive DOF.
///
/// Safety: out_threshold must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nis_gate_threshold(
    dof: usize,
    confidence: f64,
    out_threshold: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_nis_gate_threshold", SidereonStatus::Panic, || {
        let out_threshold = c_try!(require_out(
            out_threshold,
            "sidereon_nis_gate_threshold",
            "out_threshold"
        ));
        *out_threshold = 0.0;
        match core_estimation::nis_gate_threshold(dof, confidence) {
            Ok(value) => {
                *out_threshold = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_nis_gate_threshold", err),
        }
    })
}

/// Test one scalar innovation against a chi-square NIS gate.
///
/// Safety: out_gate must point to a SidereonNisGate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nis_gate_test(
    innovation: f64,
    innovation_variance: f64,
    dof: usize,
    confidence: f64,
    out_gate: *mut SidereonNisGate,
) -> SidereonStatus {
    ffi_boundary("sidereon_nis_gate_test", SidereonStatus::Panic, || {
        let out_gate = c_try!(require_out(out_gate, "sidereon_nis_gate_test", "out_gate"));
        *out_gate = SidereonNisGate {
            nis: 0.0,
            threshold: 0.0,
            in_gate: false,
            dof: 0,
        };
        match core_estimation::nis_gate_test(innovation, innovation_variance, dof, confidence) {
            Ok(gate) => {
                *out_gate = nis_gate_to_c(gate);
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_nis_gate_test", err),
        }
    })
}

/// Build a no-IMU track filter config from a position fix and uncertain zero
/// initial velocity. `position_covariance_m2` is row-major dimension-by-dimension.
///
/// Safety: initial_position_m points to dimension doubles; position_covariance_m2
/// points to position_covariance_len doubles; out_config points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_config_from_position(
    frame: u32,
    initial_t_s: f64,
    initial_position_m: *const f64,
    dimension: usize,
    position_covariance_m2: *const f64,
    position_covariance_len: usize,
    initial_velocity_variance_m2_s2: f64,
    acceleration_variance_spectral_density_m2_s3: f64,
    out_config: *mut *mut SidereonTrackFilterConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_config_from_position",
        SidereonStatus::Panic,
        || {
            let out_config = c_try!(require_out(
                out_config,
                "sidereon_track_filter_config_from_position",
                "out_config"
            ));
            *out_config = ptr::null_mut();
            let frame = c_try!(track_frame_from_c(
                "sidereon_track_filter_config_from_position",
                frame
            ));
            let position = c_try!(track_vector_from_c(
                "sidereon_track_filter_config_from_position",
                "initial_position_m",
                initial_position_m,
                dimension
            ));
            let covariance = c_try!(track_matrix_from_c(
                "sidereon_track_filter_config_from_position",
                "position_covariance_m2",
                position_covariance_m2,
                position_covariance_len,
                dimension,
                dimension
            ));
            let inner = match core_estimation::TrackFilterConfig::from_position(
                frame,
                initial_t_s,
                position,
                covariance,
                initial_velocity_variance_m2_s2,
                acceleration_variance_spectral_density_m2_s3,
            ) {
                Ok(config) => config,
                Err(err) => {
                    return map_track_error("sidereon_track_filter_config_from_position", err);
                }
            };
            write_boxed_handle(out_config, SidereonTrackFilterConfig { inner });
            SidereonStatus::Ok
        },
    )
}

/// Build a no-IMU track filter config from position, velocity, and full
/// covariance. `initial_covariance` is row-major over `[position, velocity]`.
///
/// Safety: initial_position_m and initial_velocity_m_s point to dimension
/// doubles; initial_covariance points to initial_covariance_len doubles;
/// out_config points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_config_from_position_velocity(
    frame: u32,
    initial_t_s: f64,
    initial_position_m: *const f64,
    initial_velocity_m_s: *const f64,
    dimension: usize,
    initial_covariance: *const f64,
    initial_covariance_len: usize,
    acceleration_variance_spectral_density_m2_s3: f64,
    out_config: *mut *mut SidereonTrackFilterConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_config_from_position_velocity",
        SidereonStatus::Panic,
        || {
            let out_config = c_try!(require_out(
                out_config,
                "sidereon_track_filter_config_from_position_velocity",
                "out_config"
            ));
            *out_config = ptr::null_mut();
            let frame = c_try!(track_frame_from_c(
                "sidereon_track_filter_config_from_position_velocity",
                frame
            ));
            let position = c_try!(track_vector_from_c(
                "sidereon_track_filter_config_from_position_velocity",
                "initial_position_m",
                initial_position_m,
                dimension
            ));
            let velocity = c_try!(track_vector_from_c(
                "sidereon_track_filter_config_from_position_velocity",
                "initial_velocity_m_s",
                initial_velocity_m_s,
                dimension
            ));
            let state_dimension = c_try!(track_state_dimension(
                "sidereon_track_filter_config_from_position_velocity",
                dimension
            ));
            let covariance = c_try!(track_matrix_from_c(
                "sidereon_track_filter_config_from_position_velocity",
                "initial_covariance",
                initial_covariance,
                initial_covariance_len,
                state_dimension,
                state_dimension
            ));
            let inner = match core_estimation::TrackFilterConfig::from_position_velocity(
                frame,
                initial_t_s,
                position,
                velocity,
                covariance,
                acceleration_variance_spectral_density_m2_s3,
            ) {
                Ok(config) => config,
                Err(err) => {
                    return map_track_error(
                        "sidereon_track_filter_config_from_position_velocity",
                        err,
                    );
                }
            };
            write_boxed_handle(out_config, SidereonTrackFilterConfig { inner });
            SidereonStatus::Ok
        },
    )
}

/// Copy config dimension.
///
/// Safety: config must be live; out_dimension must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_config_dimension(
    config: *const SidereonTrackFilterConfig,
    out_dimension: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_config_dimension",
        SidereonStatus::Panic,
        || {
            let config = c_try!(require_ref(
                config,
                "sidereon_track_filter_config_dimension",
                "config"
            ));
            let out_dimension = c_try!(require_out(
                out_dimension,
                "sidereon_track_filter_config_dimension",
                "out_dimension"
            ));
            *out_dimension = config.inner.dimension();
            SidereonStatus::Ok
        },
    )
}

/// Copy config frame selector.
///
/// Safety: config must be live; out_frame must point to uint32_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_config_frame(
    config: *const SidereonTrackFilterConfig,
    out_frame: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_config_frame",
        SidereonStatus::Panic,
        || {
            let config = c_try!(require_ref(
                config,
                "sidereon_track_filter_config_frame",
                "config"
            ));
            let out_frame = c_try!(require_out(
                out_frame,
                "sidereon_track_filter_config_frame",
                "out_frame"
            ));
            *out_frame = track_frame_to_c(config.inner.frame);
            SidereonStatus::Ok
        },
    )
}

/// Release a track filter config handle. Passing NULL is a no-op.
///
/// Safety: config must be NULL or a live handle from a config constructor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_config_free(config: *mut SidereonTrackFilterConfig) {
    ffi_boundary("sidereon_track_filter_config_free", (), || {
        free_boxed(config);
    });
}

/// Build a stateful no-IMU track filter from a config.
///
/// Safety: config must be live; out_filter points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_new(
    config: *const SidereonTrackFilterConfig,
    out_filter: *mut *mut SidereonTrackFilter,
) -> SidereonStatus {
    ffi_boundary("sidereon_track_filter_new", SidereonStatus::Panic, || {
        let config = c_try!(require_ref(config, "sidereon_track_filter_new", "config"));
        let out_filter = c_try!(require_out(
            out_filter,
            "sidereon_track_filter_new",
            "out_filter"
        ));
        *out_filter = ptr::null_mut();
        match core_estimation::TrackFilter::new(config.inner.clone()) {
            Ok(inner) => {
                write_boxed_handle(out_filter, SidereonTrackFilter { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_track_error("sidereon_track_filter_new", err),
        }
    })
}

/// Build a stateful no-IMU track filter directly from a position fix and
/// uncertain zero initial velocity.
///
/// Safety: same pointer contract as sidereon_track_filter_config_from_position;
/// out_filter points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_new_from_position(
    frame: u32,
    initial_t_s: f64,
    initial_position_m: *const f64,
    dimension: usize,
    position_covariance_m2: *const f64,
    position_covariance_len: usize,
    initial_velocity_variance_m2_s2: f64,
    acceleration_variance_spectral_density_m2_s3: f64,
    out_filter: *mut *mut SidereonTrackFilter,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_new_from_position",
        SidereonStatus::Panic,
        || {
            let out_filter = c_try!(require_out(
                out_filter,
                "sidereon_track_filter_new_from_position",
                "out_filter"
            ));
            *out_filter = ptr::null_mut();
            let mut config: *mut SidereonTrackFilterConfig = ptr::null_mut();
            let status = sidereon_track_filter_config_from_position(
                frame,
                initial_t_s,
                initial_position_m,
                dimension,
                position_covariance_m2,
                position_covariance_len,
                initial_velocity_variance_m2_s2,
                acceleration_variance_spectral_density_m2_s3,
                &mut config,
            );
            if status != SidereonStatus::Ok {
                return status;
            }
            let status = sidereon_track_filter_new(config, out_filter);
            sidereon_track_filter_config_free(config);
            status
        },
    )
}

/// Release a track filter handle. Passing NULL is a no-op.
///
/// Safety: filter must be NULL or a live handle from a filter constructor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_free(filter: *mut SidereonTrackFilter) {
    ffi_boundary("sidereon_track_filter_free", (), || {
        free_boxed(filter);
    });
}

/// Copy current filter state metadata.
///
/// Safety: filter must be live; out_state points to a SidereonTrackState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_state(
    filter: *const SidereonTrackFilter,
    out_state: *mut SidereonTrackState,
) -> SidereonStatus {
    ffi_boundary("sidereon_track_filter_state", SidereonStatus::Panic, || {
        let filter = c_try!(require_ref(filter, "sidereon_track_filter_state", "filter"));
        let out_state = c_try!(require_out(
            out_state,
            "sidereon_track_filter_state",
            "out_state"
        ));
        *out_state = track_state_to_c(filter.inner.state());
        SidereonStatus::Ok
    })
}

/// Copy the current filter position vector in metres.
///
/// Safety: filter must be live; out follows the standard variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_position_m(
    filter: *const SidereonTrackFilter,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_position_m",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_position_m",
                "filter"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_track_filter_position_m",
                "out",
                &filter.inner.state().position_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the current filter velocity vector in metres per second.
///
/// Safety: filter must be live; out follows the standard variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_velocity_m_s(
    filter: *const SidereonTrackFilter,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_velocity_m_s",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_velocity_m_s",
                "filter"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_track_filter_velocity_m_s",
                "out",
                &filter.inner.state().velocity_m_s,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the current filter state vector `[position, velocity]`.
///
/// Safety: filter must be live; out follows the standard variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_state_vector(
    filter: *const SidereonTrackFilter,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_state_vector",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_state_vector",
                "filter"
            ));
            let values = filter.inner.state().state_vector();
            c_try!(copy_prefix_to_c(
                "sidereon_track_filter_state_vector",
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

/// Copy the current row-major filter covariance over `[position, velocity]`.
///
/// Safety: filter must be live; out follows the standard variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_covariance(
    filter: *const SidereonTrackFilter,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_covariance",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_covariance",
                "filter"
            ));
            let flat = flatten_matrix(&filter.inner.state().covariance);
            c_try!(copy_prefix_to_c(
                "sidereon_track_filter_covariance",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Predict the filter state by dt_s seconds.
///
/// Safety: filter must be live; out_prediction points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_predict(
    filter: *mut SidereonTrackFilter,
    dt_s: f64,
    out_prediction: *mut SidereonTrackPrediction,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_predict",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_predict",
                "filter"
            ));
            let out_prediction = c_try!(require_out(
                out_prediction,
                "sidereon_track_filter_predict",
                "out_prediction"
            ));
            *out_prediction = empty_track_prediction();
            match filter.inner.predict(dt_s) {
                Ok(prediction) => {
                    *out_prediction = track_prediction_to_c(&prediction);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_predict", err),
            }
        },
    )
}

/// Predict the filter state and append the prediction to an RTS history builder.
///
/// Safety: filter and history must be live; out_prediction points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_predict_recorded(
    filter: *mut SidereonTrackFilter,
    dt_s: f64,
    history: *mut SidereonTrackRtsHistoryBuilder,
    out_prediction: *mut SidereonTrackPrediction,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_predict_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_predict_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_track_filter_predict_recorded",
                "history"
            ));
            let out_prediction = c_try!(require_out(
                out_prediction,
                "sidereon_track_filter_predict_recorded",
                "out_prediction"
            ));
            *out_prediction = empty_track_prediction();
            match filter.inner.predict_recorded(dt_s, &mut history.inner) {
                Ok(prediction) => {
                    *out_prediction = track_prediction_to_c(&prediction);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_predict_recorded", err),
            }
        },
    )
}

/// Compute a position-only innovation without updating the filter. The
/// innovation and covariance buffers are exact-size outputs: dimension and
/// dimension*dimension doubles.
///
/// Safety: filter must be live; position_m and covariance_m2 point to readable
/// arrays; innovation, innovation_covariance, and out_report point to writable
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_position_innovation(
    filter: *const SidereonTrackFilter,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
    innovation: *mut f64,
    innovation_len: usize,
    innovation_covariance: *mut f64,
    innovation_covariance_len: usize,
    out_report: *mut SidereonTrackInnovation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_position_innovation",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_position_innovation",
                "filter"
            ));
            let position = c_try!(track_vector_from_c(
                "sidereon_track_filter_position_innovation",
                "position_m",
                position_m,
                dimension
            ));
            let covariance = c_try!(track_matrix_from_c(
                "sidereon_track_filter_position_innovation",
                "covariance_m2",
                covariance_m2,
                covariance_len,
                dimension,
                dimension
            ));
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_track_filter_position_innovation",
                "out_report"
            ));
            *out_report = empty_track_innovation();
            match filter.inner.position_innovation(&position, &covariance) {
                Ok(report) => {
                    c_try!(copy_exact_f64s(
                        "sidereon_track_filter_position_innovation",
                        "innovation",
                        innovation,
                        innovation_len,
                        &report.innovation,
                    ));
                    let flat_covariance = flatten_matrix(&report.innovation_covariance);
                    c_try!(copy_exact_f64s(
                        "sidereon_track_filter_position_innovation",
                        "innovation_covariance",
                        innovation_covariance,
                        innovation_covariance_len,
                        &flat_covariance,
                    ));
                    *out_report = track_innovation_to_c(&report);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_position_innovation", err),
            }
        },
    )
}

/// Compute a full-state innovation without updating the filter. The innovation
/// and covariance buffers are exact-size outputs: state_dimension and
/// state_dimension*state_dimension doubles.
///
/// Safety: filter must be live; state and covariance point to readable arrays;
/// innovation, innovation_covariance, and out_report point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_state_innovation(
    filter: *const SidereonTrackFilter,
    state: *const f64,
    state_len: usize,
    covariance: *const f64,
    covariance_len: usize,
    innovation: *mut f64,
    innovation_len: usize,
    innovation_covariance: *mut f64,
    innovation_covariance_len: usize,
    out_report: *mut SidereonTrackInnovation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_state_innovation",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_state_innovation",
                "filter"
            ));
            let expected_state_len = filter.inner.state().state_dimension();
            if state_len != expected_state_len {
                set_last_error(format!(
                    "sidereon_track_filter_state_innovation: state needs {expected_state_len} doubles"
                ));
                return SidereonStatus::InvalidArgument;
            }
            let state = c_try!(track_vector_from_c(
                "sidereon_track_filter_state_innovation",
                "state",
                state,
                state_len
            ));
            let covariance = c_try!(track_matrix_from_c(
                "sidereon_track_filter_state_innovation",
                "covariance",
                covariance,
                covariance_len,
                expected_state_len,
                expected_state_len
            ));
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_track_filter_state_innovation",
                "out_report"
            ));
            *out_report = empty_track_innovation();
            match filter.inner.state_innovation(&state, &covariance) {
                Ok(report) => {
                    c_try!(copy_exact_f64s(
                        "sidereon_track_filter_state_innovation",
                        "innovation",
                        innovation,
                        innovation_len,
                        &report.innovation,
                    ));
                    let flat_covariance = flatten_matrix(&report.innovation_covariance);
                    c_try!(copy_exact_f64s(
                        "sidereon_track_filter_state_innovation",
                        "innovation_covariance",
                        innovation_covariance,
                        innovation_covariance_len,
                        &flat_covariance,
                    ));
                    *out_report = track_innovation_to_c(&report);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_state_innovation", err),
            }
        },
    )
}

/// Apply a position-only track update.
///
/// Safety: filter must be live; position_m and covariance_m2 point to readable
/// arrays; out_update points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_update_position(
    filter: *mut SidereonTrackFilter,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
    out_update: *mut SidereonTrackUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_update_position",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_update_position",
                "filter"
            ));
            let (position, covariance) = c_try!(track_position_observation_from_c(
                "sidereon_track_filter_update_position",
                position_m,
                dimension,
                covariance_m2,
                covariance_len
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_track_filter_update_position",
                "out_update"
            ));
            *out_update = empty_track_update();
            match filter.inner.update_position(&position, &covariance) {
                Ok(update) => {
                    *out_update = track_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_update_position", err),
            }
        },
    )
}

/// Apply a full-state track update.
///
/// Safety: filter must be live; state and covariance point to readable arrays;
/// out_update points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_update_state(
    filter: *mut SidereonTrackFilter,
    state: *const f64,
    state_len: usize,
    covariance: *const f64,
    covariance_len: usize,
    out_update: *mut SidereonTrackUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_update_state",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_update_state",
                "filter"
            ));
            let expected_state_len = filter.inner.state().state_dimension();
            if state_len != expected_state_len {
                set_last_error(format!(
                    "sidereon_track_filter_update_state: state needs {expected_state_len} doubles"
                ));
                return SidereonStatus::InvalidArgument;
            }
            let state = c_try!(track_vector_from_c(
                "sidereon_track_filter_update_state",
                "state",
                state,
                state_len
            ));
            let covariance = c_try!(track_matrix_from_c(
                "sidereon_track_filter_update_state",
                "covariance",
                covariance,
                covariance_len,
                expected_state_len,
                expected_state_len
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_track_filter_update_state",
                "out_update"
            ));
            *out_update = empty_track_update();
            match filter.inner.update_state(&state, &covariance) {
                Ok(update) => {
                    *out_update = track_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_update_state", err),
            }
        },
    )
}

/// Apply a NIS-gated position-only track update.
///
/// Safety: filter must be live; position_m and covariance_m2 point to readable
/// arrays; out_update points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_update_position_gated(
    filter: *mut SidereonTrackFilter,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
    confidence: f64,
    out_update: *mut SidereonTrackGatedUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_update_position_gated",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_update_position_gated",
                "filter"
            ));
            let (position, covariance) = c_try!(track_position_observation_from_c(
                "sidereon_track_filter_update_position_gated",
                position_m,
                dimension,
                covariance_m2,
                covariance_len
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_track_filter_update_position_gated",
                "out_update"
            ));
            *out_update = empty_track_gated_update();
            match filter
                .inner
                .update_position_gated(&position, &covariance, confidence)
            {
                Ok(update) => {
                    *out_update = track_gated_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_update_position_gated", err),
            }
        },
    )
}

/// Apply a position-only track update and record the epoch for RTS smoothing.
///
/// Safety: filter and history must be live; position_m and covariance_m2 point
/// to readable arrays; out_update points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_update_position_recorded(
    filter: *mut SidereonTrackFilter,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
    history: *mut SidereonTrackRtsHistoryBuilder,
    out_update: *mut SidereonTrackUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_update_position_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_update_position_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_track_filter_update_position_recorded",
                "history"
            ));
            let (position, covariance) = c_try!(track_position_observation_from_c(
                "sidereon_track_filter_update_position_recorded",
                position_m,
                dimension,
                covariance_m2,
                covariance_len
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_track_filter_update_position_recorded",
                "out_update"
            ));
            *out_update = empty_track_update();
            match filter
                .inner
                .update_position_recorded(&position, &covariance, &mut history.inner)
            {
                Ok(update) => {
                    *out_update = track_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_filter_update_position_recorded", err),
            }
        },
    )
}

/// Apply a NIS-gated position-only track update and record accepted or rejected
/// epochs for RTS smoothing.
///
/// Safety: filter and history must be live; position_m and covariance_m2 point
/// to readable arrays; out_update points to a report.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_update_position_gated_recorded(
    filter: *mut SidereonTrackFilter,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
    confidence: f64,
    history: *mut SidereonTrackRtsHistoryBuilder,
    out_update: *mut SidereonTrackGatedUpdate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_update_position_gated_recorded",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_mut(
                filter,
                "sidereon_track_filter_update_position_gated_recorded",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_track_filter_update_position_gated_recorded",
                "history"
            ));
            let (position, covariance) = c_try!(track_position_observation_from_c(
                "sidereon_track_filter_update_position_gated_recorded",
                position_m,
                dimension,
                covariance_m2,
                covariance_len
            ));
            let out_update = c_try!(require_out(
                out_update,
                "sidereon_track_filter_update_position_gated_recorded",
                "out_update"
            ));
            *out_update = empty_track_gated_update();
            match filter.inner.update_position_gated_recorded(
                &position,
                &covariance,
                confidence,
                &mut history.inner,
            ) {
                Ok(update) => {
                    *out_update = track_gated_update_to_c(&update);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_track_error("sidereon_track_filter_update_position_gated_recorded", err)
                }
            }
        },
    )
}

/// Record the filter's current predicted state without applying a correction.
///
/// Safety: filter and history must be live.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_filter_record_prediction_only(
    filter: *const SidereonTrackFilter,
    history: *mut SidereonTrackRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_filter_record_prediction_only",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_filter_record_prediction_only",
                "filter"
            ));
            let history = c_try!(require_mut(
                history,
                "sidereon_track_filter_record_prediction_only",
                "history"
            ));
            match filter.inner.record_prediction_only(&mut history.inner) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_track_error("sidereon_track_filter_record_prediction_only", err),
            }
        },
    )
}

/// Create an empty RTS history builder.
///
/// Safety: out_history points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_builder_new(
    out_history: *mut *mut SidereonTrackRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_builder_new",
        SidereonStatus::Panic,
        || {
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_track_rts_history_builder_new",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            write_boxed_handle(
                out_history,
                SidereonTrackRtsHistoryBuilder {
                    inner: core_estimation::TrackRtsHistoryBuilder::empty(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Create an RTS history builder from a filter's current state.
///
/// Safety: filter must be live; out_history points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_builder_from_filter(
    filter: *const SidereonTrackFilter,
    out_history: *mut *mut SidereonTrackRtsHistoryBuilder,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_builder_from_filter",
        SidereonStatus::Panic,
        || {
            let filter = c_try!(require_ref(
                filter,
                "sidereon_track_rts_history_builder_from_filter",
                "filter"
            ));
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_track_rts_history_builder_from_filter",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            match core_estimation::TrackRtsHistoryBuilder::from_filter(&filter.inner) {
                Ok(inner) => {
                    write_boxed_handle(out_history, SidereonTrackRtsHistoryBuilder { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_rts_history_builder_from_filter", err),
            }
        },
    )
}

/// Finish an RTS history builder into a newly owned history handle.
///
/// Safety: history must be live; out_history points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_builder_finish(
    history: *const SidereonTrackRtsHistoryBuilder,
    out_history: *mut *mut SidereonTrackRtsHistory,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_builder_finish",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_builder_finish",
                "history"
            ));
            let out_history = c_try!(require_out(
                out_history,
                "sidereon_track_rts_history_builder_finish",
                "out_history"
            ));
            *out_history = ptr::null_mut();
            match history.inner.clone().finish() {
                Ok(inner) => {
                    write_boxed_handle(out_history, SidereonTrackRtsHistory { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_track_error("sidereon_track_rts_history_builder_finish", err),
            }
        },
    )
}

/// Release an RTS history builder. Passing NULL is a no-op.
///
/// Safety: history must be NULL or a live builder handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_builder_free(
    history: *mut SidereonTrackRtsHistoryBuilder,
) {
    ffi_boundary("sidereon_track_rts_history_builder_free", (), || {
        free_boxed(history);
    });
}

/// Copy finished history epoch count.
///
/// Safety: history must be live; out_count points to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_epoch_count(
    history: *const SidereonTrackRtsHistory,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_epoch_count",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_epoch_count",
                "history"
            ));
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_track_rts_history_epoch_count",
                "out_count"
            ));
            *out_count = history.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one finished history epoch summary.
///
/// Safety: history must be live; out_epoch points to a SidereonTrackRtsEpoch.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_epoch(
    history: *const SidereonTrackRtsHistory,
    index: usize,
    out_epoch: *mut SidereonTrackRtsEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_epoch",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_epoch",
                "history"
            ));
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_track_rts_history_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_track_rts_epoch();
            let epoch = c_try!(track_history_epoch(
                "sidereon_track_rts_history_epoch",
                history,
                index
            ));
            *out_epoch = track_rts_epoch_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Copy one history epoch's predicted position vector.
///
/// Safety: history must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_epoch_predicted_position_m(
    history: *const SidereonTrackRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_epoch_predicted_position_m",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_epoch_predicted_position_m",
                "history"
            ));
            let epoch = c_try!(track_history_epoch(
                "sidereon_track_rts_history_epoch_predicted_position_m",
                history,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_track_rts_history_epoch_predicted_position_m",
                "out",
                &epoch.predicted.position_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one history epoch's updated position vector.
///
/// Safety: history must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_epoch_updated_position_m(
    history: *const SidereonTrackRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_epoch_updated_position_m",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_epoch_updated_position_m",
                "history"
            ));
            let epoch = c_try!(track_history_epoch(
                "sidereon_track_rts_history_epoch_updated_position_m",
                history,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_track_rts_history_epoch_updated_position_m",
                "out",
                &epoch.updated.position_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one history epoch's transition from the previous epoch. When the epoch
/// has no transition, required is zero.
///
/// Safety: history must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_epoch_transition_from_previous(
    history: *const SidereonTrackRtsHistory,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_track_rts_history_epoch_transition_from_previous",
        SidereonStatus::Panic,
        || {
            let history = c_try!(require_ref(
                history,
                "sidereon_track_rts_history_epoch_transition_from_previous",
                "history"
            ));
            let epoch = c_try!(track_history_epoch(
                "sidereon_track_rts_history_epoch_transition_from_previous",
                history,
                index
            ));
            let flat = epoch
                .transition_from_previous
                .as_ref()
                .map(|matrix| flatten_matrix(matrix))
                .unwrap_or_default();
            c_try!(copy_prefix_to_c(
                "sidereon_track_rts_history_epoch_transition_from_previous",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a finished RTS history handle. Passing NULL is a no-op.
///
/// Safety: history must be NULL or a live history handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_track_rts_history_free(history: *mut SidereonTrackRtsHistory) {
    ffi_boundary("sidereon_track_rts_history_free", (), || {
        free_boxed(history);
    });
}

/// Smooth a finished RTS history with the fixed-interval RTS smoother.
///
/// Safety: history must be live; out_smoothed points to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smooth_track_rts(
    history: *const SidereonTrackRtsHistory,
    out_smoothed: *mut *mut SidereonSmoothedTrack,
) -> SidereonStatus {
    ffi_boundary("sidereon_smooth_track_rts", SidereonStatus::Panic, || {
        let history = c_try!(require_ref(history, "sidereon_smooth_track_rts", "history"));
        let out_smoothed = c_try!(require_out(
            out_smoothed,
            "sidereon_smooth_track_rts",
            "out_smoothed"
        ));
        *out_smoothed = ptr::null_mut();
        match core_estimation::smooth_track_rts(&history.inner) {
            Ok(inner) => {
                write_boxed_handle(out_smoothed, SidereonSmoothedTrack { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_track_error("sidereon_smooth_track_rts", err),
        }
    })
}

/// Copy smoothed track epoch count.
///
/// Safety: smoothed must be live; out_count points to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_epoch_count(
    smoothed: *const SidereonSmoothedTrack,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_track_epoch_count",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_track_epoch_count",
                "smoothed"
            ));
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_smoothed_track_epoch_count",
                "out_count"
            ));
            *out_count = smoothed.inner.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one smoothed track epoch summary.
///
/// Safety: smoothed must be live; out_epoch points to a SidereonSmoothedTrackEpoch.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_epoch(
    smoothed: *const SidereonSmoothedTrack,
    index: usize,
    out_epoch: *mut SidereonSmoothedTrackEpoch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_track_epoch",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_track_epoch",
                "smoothed"
            ));
            let out_epoch = c_try!(require_out(
                out_epoch,
                "sidereon_smoothed_track_epoch",
                "out_epoch"
            ));
            *out_epoch = empty_smoothed_track_epoch();
            let epoch = c_try!(smoothed_track_epoch(
                "sidereon_smoothed_track_epoch",
                smoothed,
                index
            ));
            *out_epoch = smoothed_track_epoch_to_c(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Copy one smoothed epoch's position vector.
///
/// Safety: smoothed must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_epoch_position_m(
    smoothed: *const SidereonSmoothedTrack,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_track_epoch_position_m",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_track_epoch_position_m",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_track_epoch(
                "sidereon_smoothed_track_epoch_position_m",
                smoothed,
                index
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_track_epoch_position_m",
                "out",
                &epoch.state.position_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one smoothed epoch's covariance over `[position, velocity]`.
///
/// Safety: smoothed must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_epoch_covariance(
    smoothed: *const SidereonSmoothedTrack,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_track_epoch_covariance",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_track_epoch_covariance",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_track_epoch(
                "sidereon_smoothed_track_epoch_covariance",
                smoothed,
                index
            ));
            let flat = flatten_matrix(&epoch.state.covariance);
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_track_epoch_covariance",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one smoothed epoch's RTS gain to the next epoch. When the epoch has no
/// next gain, required is zero.
///
/// Safety: smoothed must be live; out follows the standard variable-length
/// output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_epoch_rts_gain_to_next(
    smoothed: *const SidereonSmoothedTrack,
    index: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_smoothed_track_epoch_rts_gain_to_next",
        SidereonStatus::Panic,
        || {
            let smoothed = c_try!(require_ref(
                smoothed,
                "sidereon_smoothed_track_epoch_rts_gain_to_next",
                "smoothed"
            ));
            let epoch = c_try!(smoothed_track_epoch(
                "sidereon_smoothed_track_epoch_rts_gain_to_next",
                smoothed,
                index
            ));
            let flat = epoch
                .rts_gain_to_next
                .as_ref()
                .map(|matrix| flatten_matrix(matrix))
                .unwrap_or_default();
            c_try!(copy_prefix_to_c(
                "sidereon_smoothed_track_epoch_rts_gain_to_next",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a smoothed track handle. Passing NULL is a no-op.
///
/// Safety: smoothed must be NULL or a live smoothed-track handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_smoothed_track_free(smoothed: *mut SidereonSmoothedTrack) {
    ffi_boundary("sidereon_smoothed_track_free", (), || {
        free_boxed(smoothed);
    });
}

/// Median absolute deviation spread estimate with Gaussian consistency scaling.
///
/// Safety: values must point to count doubles or be NULL when count is 0; out
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mad_spread(
    values: *const f64,
    count: usize,
    scale_floor: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_mad_spread", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_mad_spread", "out"));
        *out = 0.0;
        let values = c_try!(require_slice(
            values,
            count,
            "sidereon_mad_spread",
            "values"
        ));
        match core_estimation::mad_spread(values, scale_floor) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_mad_spread", err),
        }
    })
}

/// EWMA update with alpha in [0, 1].
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ewma_update(
    previous: f64,
    sample: f64,
    alpha: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_ewma_update", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_ewma_update", "out"));
        *out = 0.0;
        match core_estimation::ewma_update(previous, sample, alpha) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_ewma_update", err),
        }
    })
}

/// EWMA update with alpha = 1 / 2^shift.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ewma_update_power_of_two(
    previous: f64,
    sample: f64,
    shift: u32,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ewma_update_power_of_two",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_ewma_update_power_of_two", "out"));
            *out = 0.0;
            match core_estimation::ewma_update_power_of_two(previous, sample, shift) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_ewma_update_power_of_two", err),
            }
        },
    )
}

/// CA-CFAR threshold multiplier from searched-cell count and target false-alarm
/// probability.
///
/// Safety: out_multiplier must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cfar_ca_multiplier_from_pfa(
    searched_cells: usize,
    false_alarm_probability: f64,
    out_multiplier: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cfar_ca_multiplier_from_pfa",
        SidereonStatus::Panic,
        || {
            let out_multiplier = c_try!(require_out(
                out_multiplier,
                "sidereon_cfar_ca_multiplier_from_pfa",
                "out_multiplier"
            ));
            *out_multiplier = 0.0;
            match core_estimation::cfar_ca_multiplier_from_pfa(
                searched_cells,
                false_alarm_probability,
            ) {
                Ok(value) => {
                    *out_multiplier = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_cfar_ca_multiplier_from_pfa", err),
            }
        },
    )
}

/// CA-CFAR false-alarm probability from a threshold multiplier.
///
/// Safety: out_pfa must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cfar_ca_pfa_from_multiplier(
    searched_cells: usize,
    multiplier: f64,
    out_pfa: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cfar_ca_pfa_from_multiplier",
        SidereonStatus::Panic,
        || {
            let out_pfa = c_try!(require_out(
                out_pfa,
                "sidereon_cfar_ca_pfa_from_multiplier",
                "out_pfa"
            ));
            *out_pfa = 0.0;
            match core_estimation::cfar_ca_pfa_from_multiplier(searched_cells, multiplier) {
                Ok(value) => {
                    *out_pfa = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_cfar_ca_pfa_from_multiplier", err),
            }
        },
    )
}

/// CA-CFAR absolute threshold from mean noise level and target false-alarm
/// probability.
///
/// Safety: out_threshold must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cfar_ca_threshold(
    searched_cells: usize,
    false_alarm_probability: f64,
    noise_level: f64,
    out_threshold: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_cfar_ca_threshold", SidereonStatus::Panic, || {
        let out_threshold = c_try!(require_out(
            out_threshold,
            "sidereon_cfar_ca_threshold",
            "out_threshold"
        ));
        *out_threshold = 0.0;
        match core_estimation::cfar_ca_threshold(
            searched_cells,
            false_alarm_probability,
            noise_level,
        ) {
            Ok(value) => {
                *out_threshold = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_cfar_ca_threshold", err),
        }
    })
}

/// CA-CFAR false-alarm probability from an absolute threshold and mean noise
/// level.
///
/// Safety: out_pfa must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_cfar_ca_false_alarm_probability(
    searched_cells: usize,
    threshold: f64,
    noise_level: f64,
    out_pfa: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_cfar_ca_false_alarm_probability",
        SidereonStatus::Panic,
        || {
            let out_pfa = c_try!(require_out(
                out_pfa,
                "sidereon_cfar_ca_false_alarm_probability",
                "out_pfa"
            ));
            *out_pfa = 0.0;
            match core_estimation::cfar_ca_false_alarm_probability(
                searched_cells,
                threshold,
                noise_level,
            ) {
                Ok(value) => {
                    *out_pfa = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_primitive_error("sidereon_cfar_ca_false_alarm_probability", err),
            }
        },
    )
}

/// Scalar normalized innovation squared statistic.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nis(
    innovation: f64,
    innovation_variance: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_nis", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_nis", "out"));
        *out = 0.0;
        match core_estimation::nis_statistic(innovation, innovation_variance) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(err) => map_primitive_error("sidereon_nis", err),
        }
    })
}

/// Inverse chi-square quantile for probability p and k degrees of freedom.
/// Delegates to sidereon_core::quality::chi2_inv.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_chi2_inv(p: f64, k: usize, out: *mut f64) -> SidereonStatus {
    ffi_boundary("sidereon_chi2_inv", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_chi2_inv", "out"));
        *out = 0.0;
        match sidereon_core::quality::chi2_inv(p, k) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_chi2_inv", err),
        }
    })
}

/// Per-satellite measurement sigma (meters) for elevation/C-N0 entries. Writes
/// one value/present pair per input entry, aligned to entries by index; an entry
/// whose variance is not computable is reported present=false. Delegates to
/// sidereon_core::quality::sigmas.
///
/// Safety: entries points to count SidereonWeightEntry; options to a
/// SidereonPseudorangeVarianceOptions; out_values to count doubles; out_present
/// to count bools.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sigmas(
    entries: *const SidereonWeightEntry,
    count: usize,
    options: *const SidereonPseudorangeVarianceOptions,
    out_values: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_sigmas", SidereonStatus::Panic, || {
        let out_values = c_try!(require_out(out_values, "sidereon_sigmas", "out_values"));
        let out_values = out_values as *mut f64;
        let out_present = c_try!(require_out(out_present, "sidereon_sigmas", "out_present"));
        let out_present = out_present as *mut bool;
        c_try!(validate_element_count::<f64>(
            "sidereon_sigmas",
            "count",
            count
        ));
        for idx in 0..count {
            *out_values.add(idx) = 0.0;
            *out_present.add(idx) = false;
        }
        let options = c_try!(require_ref(options, "sidereon_sigmas", "options"));
        let opts = c_try!(pseudorange_variance_options_from_c(
            "sidereon_sigmas",
            options
        ));
        let entries = c_try!(weight_entries_from_c("sidereon_sigmas", entries, count));
        let map = sidereon_core::quality::sigmas(&entries, opts);
        write_entry_map_positional(&entries, &map, out_values, out_present);
        SidereonStatus::Ok
    })
}

unsafe fn weight_entries_from_c(
    fn_name: &str,
    entries: *const SidereonWeightEntry,
    count: usize,
) -> Result<Vec<sidereon_core::quality::WeightEntry>, SidereonStatus> {
    let rows = require_slice(entries, count, fn_name, "entries")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        out.push(sidereon_core::quality::WeightEntry {
            satellite_id: sat.to_string(),
            elevation_deg: row.elevation_deg,
            cn0_dbhz: row.has_cn0.then_some(row.cn0_dbhz),
        });
    }
    Ok(out)
}

// Write a per-input-entry value/present pair from a token-keyed map. Entries the
// map dropped (variance not computable) are reported present=false.

unsafe fn write_entry_map_positional(
    entries: &[sidereon_core::quality::WeightEntry],
    map: &BTreeMap<String, f64>,
    out_values: *mut f64,
    out_present: *mut bool,
) {
    for (idx, entry) in entries.iter().enumerate() {
        match map.get(&entry.satellite_id) {
            Some(value) => {
                *out_values.add(idx) = *value;
                *out_present.add(idx) = true;
            }
            None => {
                *out_values.add(idx) = 0.0;
                *out_present.add(idx) = false;
            }
        }
    }
}

/// Map a residual-distribution error to a status code. All causes (non-finite
/// value, too few samples, zero variance/range) are input conditions and report
/// SIDEREON_STATUS_INVALID_ARGUMENT.
fn map_normality_error(fn_name: &str, err: NormalityError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_primitive_error(fn_name: &str, err: core_estimation::PrimitiveError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn alpha_beta_state_from_c(state: SidereonAlphaBetaState) -> core_estimation::AlphaBetaState {
    core_estimation::AlphaBetaState {
        level: state.level,
        rate: state.rate,
    }
}

fn alpha_beta_state_to_c(state: core_estimation::AlphaBetaState) -> SidereonAlphaBetaState {
    SidereonAlphaBetaState {
        level: state.level,
        rate: state.rate,
    }
}

fn alpha_beta_gains_from_c(gains: SidereonAlphaBetaGains) -> core_estimation::AlphaBetaGains {
    core_estimation::AlphaBetaGains {
        alpha: gains.alpha,
        beta: gains.beta,
    }
}

fn alpha_beta_gains_to_c(gains: core_estimation::AlphaBetaGains) -> SidereonAlphaBetaGains {
    SidereonAlphaBetaGains {
        alpha: gains.alpha,
        beta: gains.beta,
    }
}

fn scalar_kalman_gains_to_c(
    gains: core_estimation::ScalarKalmanGains,
) -> SidereonScalarKalmanGains {
    SidereonScalarKalmanGains {
        position_gain: gains.position_gain,
        rate_gain: gains.rate_gain,
    }
}

fn nis_gate_to_c(gate: sidereon_core::estimation::primitives::NisGate) -> SidereonNisGate {
    SidereonNisGate {
        nis: gate.nis,
        threshold: gate.threshold,
        in_gate: gate.in_gate,
        dof: gate.dof,
    }
}

fn track_frame_from_c(
    fn_name: &str,
    frame: u32,
) -> Result<core_estimation::TrackCoordinateFrame, SidereonStatus> {
    match frame {
        value if value == SidereonTrackCoordinateFrame::Ecef as u32 => {
            Ok(core_estimation::TrackCoordinateFrame::Ecef)
        }
        value if value == SidereonTrackCoordinateFrame::Enu as u32 => {
            Ok(core_estimation::TrackCoordinateFrame::Enu)
        }
        value if value == SidereonTrackCoordinateFrame::CallerDefinedCartesian as u32 => {
            Ok(core_estimation::TrackCoordinateFrame::CallerDefinedCartesian)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid track frame selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn track_frame_to_c(frame: core_estimation::TrackCoordinateFrame) -> u32 {
    match frame {
        core_estimation::TrackCoordinateFrame::Ecef => SidereonTrackCoordinateFrame::Ecef as u32,
        core_estimation::TrackCoordinateFrame::Enu => SidereonTrackCoordinateFrame::Enu as u32,
        core_estimation::TrackCoordinateFrame::CallerDefinedCartesian => {
            SidereonTrackCoordinateFrame::CallerDefinedCartesian as u32
        }
    }
}

fn track_state_dimension(fn_name: &str, dimension: usize) -> Result<usize, SidereonStatus> {
    dimension.checked_mul(2).ok_or_else(|| {
        set_last_error(format!("{fn_name}: dimension is too large"));
        SidereonStatus::InvalidArgument
    })
}

fn track_matrix_expected_len(
    fn_name: &str,
    rows: usize,
    cols: usize,
) -> Result<usize, SidereonStatus> {
    rows.checked_mul(cols).ok_or_else(|| {
        set_last_error(format!("{fn_name}: matrix dimensions are too large"));
        SidereonStatus::InvalidArgument
    })
}

unsafe fn track_vector_from_c(
    fn_name: &str,
    arg_name: &str,
    ptr: *const f64,
    len: usize,
) -> Result<Vec<f64>, SidereonStatus> {
    let values = require_slice(ptr, len, fn_name, arg_name)?;
    Ok(values.to_vec())
}

unsafe fn track_matrix_from_c(
    fn_name: &str,
    arg_name: &str,
    ptr: *const f64,
    len: usize,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<f64>>, SidereonStatus> {
    let expected = track_matrix_expected_len(fn_name, rows, cols)?;
    if len != expected {
        set_last_error(format!(
            "{fn_name}: {arg_name} needs {expected} row-major doubles"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let values = require_slice(ptr, len, fn_name, arg_name)?;
    if cols == 0 {
        return Ok(Vec::new());
    }
    Ok(values.chunks(cols).map(|row| row.to_vec()).collect())
}

unsafe fn track_position_observation_from_c(
    fn_name: &str,
    position_m: *const f64,
    dimension: usize,
    covariance_m2: *const f64,
    covariance_len: usize,
) -> Result<(Vec<f64>, Vec<Vec<f64>>), SidereonStatus> {
    let position = track_vector_from_c(fn_name, "position_m", position_m, dimension)?;
    let covariance = track_matrix_from_c(
        fn_name,
        "covariance_m2",
        covariance_m2,
        covariance_len,
        dimension,
        dimension,
    )?;
    Ok((position, covariance))
}

fn flatten_matrix(matrix: &[Vec<f64>]) -> Vec<f64> {
    matrix.iter().flat_map(|row| row.iter().copied()).collect()
}

fn track_state_to_c(state: &core_estimation::TrackState) -> SidereonTrackState {
    SidereonTrackState {
        frame: track_frame_to_c(state.frame),
        t_s: state.t_s,
        dimension: state.dimension(),
        state_dimension: state.state_dimension(),
    }
}

fn track_prediction_to_c(prediction: &core_estimation::TrackPrediction) -> SidereonTrackPrediction {
    SidereonTrackPrediction {
        dt_s: prediction.dt_s,
        predicted: track_state_to_c(&prediction.predicted),
    }
}

fn track_innovation_to_c(innovation: &core_estimation::TrackInnovation) -> SidereonTrackInnovation {
    SidereonTrackInnovation {
        dimension: innovation.innovation.len(),
        nis: innovation.nis,
    }
}

fn track_update_to_c(update: &core_estimation::TrackUpdate) -> SidereonTrackUpdate {
    SidereonTrackUpdate {
        predicted: track_state_to_c(&update.predicted),
        updated: track_state_to_c(&update.updated),
        innovation: track_innovation_to_c(&update.innovation),
    }
}

fn track_gated_update_to_c(update: &core_estimation::TrackGatedUpdate) -> SidereonTrackGatedUpdate {
    SidereonTrackGatedUpdate {
        gate: nis_gate_to_c(update.gate),
        has_update: update.update.is_some(),
        update: update
            .update
            .as_ref()
            .map(track_update_to_c)
            .unwrap_or_else(empty_track_update),
        state: track_state_to_c(&update.state),
    }
}

fn track_rts_epoch_to_c(epoch: &core_estimation::TrackRtsEpoch) -> SidereonTrackRtsEpoch {
    SidereonTrackRtsEpoch {
        t_s: epoch.t_s,
        predicted: track_state_to_c(&epoch.predicted),
        updated: track_state_to_c(&epoch.updated),
        has_transition_from_previous: epoch.transition_from_previous.is_some(),
    }
}

fn smoothed_track_epoch_to_c(
    epoch: &core_estimation::SmoothedTrackEpoch,
) -> SidereonSmoothedTrackEpoch {
    SidereonSmoothedTrackEpoch {
        t_s: epoch.t_s,
        state: track_state_to_c(&epoch.state),
        has_rts_gain_to_next: epoch.rts_gain_to_next.is_some(),
    }
}

fn empty_track_state() -> SidereonTrackState {
    SidereonTrackState {
        frame: SidereonTrackCoordinateFrame::CallerDefinedCartesian as u32,
        t_s: 0.0,
        dimension: 0,
        state_dimension: 0,
    }
}

fn empty_track_prediction() -> SidereonTrackPrediction {
    SidereonTrackPrediction {
        dt_s: 0.0,
        predicted: empty_track_state(),
    }
}

fn empty_track_innovation() -> SidereonTrackInnovation {
    SidereonTrackInnovation {
        dimension: 0,
        nis: 0.0,
    }
}

fn empty_track_update() -> SidereonTrackUpdate {
    SidereonTrackUpdate {
        predicted: empty_track_state(),
        updated: empty_track_state(),
        innovation: empty_track_innovation(),
    }
}

fn empty_track_gated_update() -> SidereonTrackGatedUpdate {
    SidereonTrackGatedUpdate {
        gate: SidereonNisGate {
            nis: 0.0,
            threshold: 0.0,
            in_gate: false,
            dof: 0,
        },
        has_update: false,
        update: empty_track_update(),
        state: empty_track_state(),
    }
}

fn empty_track_rts_epoch() -> SidereonTrackRtsEpoch {
    SidereonTrackRtsEpoch {
        t_s: 0.0,
        predicted: empty_track_state(),
        updated: empty_track_state(),
        has_transition_from_previous: false,
    }
}

fn empty_smoothed_track_epoch() -> SidereonSmoothedTrackEpoch {
    SidereonSmoothedTrackEpoch {
        t_s: 0.0,
        state: empty_track_state(),
        has_rts_gain_to_next: false,
    }
}

fn track_history_epoch<'a>(
    fn_name: &str,
    history: &'a SidereonTrackRtsHistory,
    index: usize,
) -> Result<&'a core_estimation::TrackRtsEpoch, SidereonStatus> {
    history.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: epoch index out of range"));
        SidereonStatus::InvalidArgument
    })
}

fn smoothed_track_epoch<'a>(
    fn_name: &str,
    smoothed: &'a SidereonSmoothedTrack,
    index: usize,
) -> Result<&'a core_estimation::SmoothedTrackEpoch, SidereonStatus> {
    smoothed.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: epoch index out of range"));
        SidereonStatus::InvalidArgument
    })
}

fn map_track_error(fn_name: &str, err: core_estimation::TrackError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
