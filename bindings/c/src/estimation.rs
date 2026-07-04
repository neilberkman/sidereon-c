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
