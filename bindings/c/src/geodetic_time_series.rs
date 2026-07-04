use super::*;

const MAX_GEODETIC_STATION_ID_BYTES: usize = 64;
const GEODETIC_STATION_ID_C_BYTES: usize = MAX_GEODETIC_STATION_ID_BYTES + 1;

/// Coordinate frame for geodetic time-series samples.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeodeticTimeSeriesFrame {
    /// Local east, north, up meters.
    Enu = 0,
    /// ITRF/ECEF meters, differenced from a geodetic reference.
    Ecef = 1,
}

/// Strength flag for a time-series velocity estimate.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeodeticTimeSeriesQuality {
    /// The span and selected pairs support the requested estimator.
    Nominal = 0,
    /// The estimate is usable but has less than three dominant periods of span.
    ShortSpan = 1,
}

/// Trajectory robust loss selector. Values match SidereonTrlsLoss.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeodeticTrajectoryLoss {
    /// Ordinary least squares.
    Linear = 0,
    /// Soft-L1 robust loss.
    SoftL1 = 1,
    /// Huber robust loss.
    Huber = 2,
    /// Cauchy robust loss.
    Cauchy = 3,
    /// Arctangent robust loss.
    Arctan = 4,
}

/// Trajectory term kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeodeticTrajectoryTermKind {
    /// Position at the reference epoch.
    Position = 0,
    /// Linear velocity.
    Velocity = 1,
    /// Annual sine coefficient.
    AnnualSin = 2,
    /// Annual cosine coefficient.
    AnnualCos = 3,
    /// Semiannual sine coefficient.
    SemiannualSin = 4,
    /// Semiannual cosine coefficient.
    SemiannualCos = 5,
    /// Heaviside offset coefficient.
    Offset = 6,
}

/// Heuristic used for step detection.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGeodeticStepDetectionHeuristic {
    /// Difference of detrended pre-event and post-event medians.
    DetrendedSlidingMedian = 0,
}

/// Fixed-size station id token.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticStationId {
    /// Null-terminated UTF-8 station id bytes.
    pub bytes: [c_char; 65],
}

/// One position sample in a station time series.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticPositionSample {
    /// Epoch expressed as a decimal year.
    pub epoch_year: f64,
    /// Position vector in meters, interpreted by the series frame.
    pub position_m: [f64; 3],
    /// Whether covariance_m2 carries a row-major 3x3 covariance.
    pub has_covariance_m2: bool,
    /// Row-major coordinate covariance in square meters.
    pub covariance_m2: [f64; 9],
}

/// Borrowed position-series descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticPositionSeries {
    /// Frame selector, as SidereonGeodeticTimeSeriesFrame.
    pub frame: u32,
    /// ECEF reference when frame is Ecef.
    pub reference: SidereonGeodetic,
    /// Position samples.
    pub samples: *const SidereonGeodeticPositionSample,
    /// Number of position samples.
    pub sample_count: usize,
}

/// MIDAS velocity-estimator options.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMidasOptions {
    /// Dominant period used for pair selection, years.
    pub dominant_period_years: f64,
    /// Allowed absolute period difference, years.
    pub period_tolerance_years: f64,
    /// Minimum retained pair count for each component.
    pub min_pairs: usize,
}

/// MIDAS diagnostics for one ENU component.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMidasComponentStats {
    /// Pair slopes selected before trimming.
    pub pair_count: usize,
    /// Pair slopes retained after trimming.
    pub retained_pair_count: usize,
    /// Robust standard deviation of retained slopes, meters per year.
    pub slope_sigma_m_per_yr: f64,
    /// Effective independent slope count.
    pub effective_pair_count: f64,
}

/// Robust ENU velocity estimate from MIDAS.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMidasVelocity {
    /// Velocity components [east, north, up], meters per year.
    pub rate_enu_m_per_yr: [f64; 3],
    /// One-sigma uncertainties [east, north, up], meters per year.
    pub sigma_enu_m_per_yr: [f64; 3],
    /// Row-major diagonal ENU velocity covariance.
    pub covariance_enu_m2_per_yr2: [f64; 9],
    /// Per-component MIDAS slope statistics.
    pub component_stats: [SidereonMidasComponentStats; 3],
    /// Number of accepted position samples.
    pub sample_count: usize,
    /// Series span, years.
    pub span_years: f64,
    /// SidereonGeodeticTimeSeriesQuality value.
    pub quality: u32,
}

/// Linear trajectory model shape.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticTrajectoryModel {
    /// Whether reference_epoch_year overrides the mean sample epoch.
    pub has_reference_epoch_year: bool,
    /// Reference epoch for the position parameter.
    pub reference_epoch_year: f64,
    /// Include annual sine and cosine terms.
    pub include_annual: bool,
    /// Include semiannual sine and cosine terms.
    pub include_semiannual: bool,
    /// Known offset epochs, decimal years.
    pub offset_epochs_year: *const f64,
    /// Number of known offset epochs.
    pub offset_count: usize,
}

/// Trajectory least-squares controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticTrajectoryFitOptions {
    /// Robust loss selector, as SidereonGeodeticTrajectoryLoss.
    pub loss: u32,
    /// Robust loss scale, meters.
    pub f_scale_m: f64,
    /// Whether max_nfev overrides the solver default.
    pub has_max_nfev: bool,
    /// Maximum residual evaluations when enabled.
    pub max_nfev: usize,
}

/// Summary of one trajectory fit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticTrajectorySummary {
    /// Reference epoch used by the fit.
    pub reference_epoch_year: f64,
    /// Number of terms per ENU component.
    pub term_count: usize,
    /// Square covariance dimension.
    pub covariance_dim: usize,
    /// Root-mean-square residuals [east, north, up], meters.
    pub residual_rms_enu_m: [f64; 3],
    /// Design observability diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
    /// Trust-region termination status.
    pub status: i32,
    /// Residual evaluations used by the solver.
    pub nfev: usize,
    /// Jacobian evaluations used by the solver.
    pub njev: usize,
    /// Final least-squares cost.
    pub cost: f64,
    /// Infinity norm of the final gradient.
    pub optimality: f64,
}

/// One trajectory term descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticTrajectoryTerm {
    /// Term kind, as SidereonGeodeticTrajectoryTermKind.
    pub kind: u32,
    /// Offset index when kind is Offset.
    pub offset_index: usize,
    /// Offset epoch when kind is Offset.
    pub epoch_year: f64,
}

/// Fitted trajectory coefficients for one ENU component.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticTrajectoryComponent {
    /// Position at the reference epoch, meters.
    pub position_m: f64,
    /// Linear velocity, meters per year.
    pub velocity_m_per_yr: f64,
    /// Whether annual_sin_m is present.
    pub has_annual_sin_m: bool,
    /// Annual sine coefficient, meters.
    pub annual_sin_m: f64,
    /// Whether annual_cos_m is present.
    pub has_annual_cos_m: bool,
    /// Annual cosine coefficient, meters.
    pub annual_cos_m: f64,
    /// Whether semiannual_sin_m is present.
    pub has_semiannual_sin_m: bool,
    /// Semiannual sine coefficient, meters.
    pub semiannual_sin_m: f64,
    /// Whether semiannual_cos_m is present.
    pub has_semiannual_cos_m: bool,
    /// Semiannual cosine coefficient, meters.
    pub semiannual_cos_m: f64,
    /// Number of offset coefficients available through the offset accessor.
    pub offset_count: usize,
}

/// Step-detection controls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticStepDetectionOptions {
    /// Half-window around a candidate epoch, years.
    pub window_years: f64,
    /// Minimum normalized offset score to report.
    pub score_threshold: f64,
    /// Minimum three-dimensional offset norm, meters.
    pub min_offset_m: f64,
    /// Minimum sample count on each side.
    pub min_samples_each_side: usize,
    /// Minimum separation between retained candidates, years.
    pub min_separation_years: f64,
    /// MIDAS controls used for detrending.
    pub midas: SidereonMidasOptions,
}

/// Candidate displacement step.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticStepCandidate {
    /// Candidate epoch, decimal years.
    pub epoch_year: f64,
    /// Estimated ENU offset, meters, after minus before.
    pub offset_enu_m: [f64; 3],
    /// Robust normalized offset score.
    pub score: f64,
    /// Number of samples before the candidate.
    pub before_count: usize,
    /// Number of samples after the candidate.
    pub after_count: usize,
    /// Heuristic selector.
    pub heuristic: u32,
}

/// Network field frame and common-mode control.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticNetworkFrame {
    /// Geodetic origin defining the output ENU frame.
    pub origin: SidereonGeodetic,
    /// Remove unweighted mean velocity across stations.
    pub remove_common_mode: bool,
}

/// One station input for network_field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticNetworkStation {
    /// Null-terminated station id.
    pub id: *const c_char,
    /// Station reference position used for local ENU rotation.
    pub reference: SidereonGeodetic,
    /// Station position time series.
    pub series: SidereonGeodeticPositionSeries,
}

/// One station motion in a network field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticStationMotion {
    /// Station id.
    pub id: SidereonGeodeticStationId,
    /// Velocity after optional common-mode removal.
    pub rate_enu_m_per_yr: [f64; 3],
    /// Velocity before optional common-mode removal.
    pub raw_rate_enu_m_per_yr: [f64; 3],
    /// One-sigma uncertainty in the network frame.
    pub sigma_enu_m_per_yr: [f64; 3],
    /// Station-local MIDAS velocity before rotation.
    pub local_velocity: SidereonMidasVelocity,
}

/// Fitted trajectory handle. Create with sidereon_geodetic_fit_trajectory and
/// release with sidereon_geodetic_trajectory_free.
pub struct SidereonGeodeticTrajectory {
    pub(crate) inner: sidereon_core::geodetic_time_series::Trajectory,
}

/// Network motion field handle. Create with sidereon_geodetic_network_field and
/// release with sidereon_geodetic_motion_field_free.
pub struct SidereonGeodeticMotionField {
    pub(crate) inner: sidereon_core::geodetic_time_series::MotionField,
}

/// Initialize MIDAS options with core defaults.
///
/// Safety: out_options must point to SidereonMidasOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_midas_options_init(
    out_options: *mut SidereonMidasOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_midas_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_geodetic_midas_options_init",
                "out_options"
            ));
            *out = midas_options_to_c(sidereon_core::geodetic_time_series::MidasOptions::default());
            SidereonStatus::Ok
        },
    )
}

/// Estimate station velocity with MIDAS.
///
/// Safety: series must point to a series descriptor; options may be NULL for
/// defaults; out_velocity must point to a SidereonMidasVelocity.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_velocity_midas(
    series: *const SidereonGeodeticPositionSeries,
    options: *const SidereonMidasOptions,
    out_velocity: *mut SidereonMidasVelocity,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_velocity_midas",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_velocity,
                "sidereon_geodetic_velocity_midas",
                "out_velocity"
            ));
            *out = empty_midas_velocity();
            let series = c_try!(require_ref(
                series,
                "sidereon_geodetic_velocity_midas",
                "series"
            ));
            let samples = c_try!(geodetic_samples_from_c(
                "sidereon_geodetic_velocity_midas",
                series,
            ));
            let frame = c_try!(position_frame_from_c(
                "sidereon_geodetic_velocity_midas",
                series,
            ));
            let core_series = sidereon_core::geodetic_time_series::PositionSeries {
                frame,
                samples: &samples,
            };
            let options = c_try!(midas_options_from_c(
                "sidereon_geodetic_velocity_midas",
                options,
            ));
            match sidereon_core::geodetic_time_series::velocity_midas(&core_series, options) {
                Ok(velocity) => {
                    *out = midas_velocity_to_c(&velocity);
                    SidereonStatus::Ok
                }
                Err(err) => map_geodetic_time_series_error("sidereon_geodetic_velocity_midas", err),
            }
        },
    )
}

/// Initialize trajectory fit options with core defaults.
///
/// Safety: out_options must point to SidereonGeodeticTrajectoryFitOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_fit_options_init(
    out_options: *mut SidereonGeodeticTrajectoryFitOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_fit_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_geodetic_trajectory_fit_options_init",
                "out_options"
            ));
            let options = sidereon_core::geodetic_time_series::TrajectoryFitOptions::default();
            *out = SidereonGeodeticTrajectoryFitOptions {
                loss: SidereonGeodeticTrajectoryLoss::Linear as u32,
                f_scale_m: options.f_scale_m,
                has_max_nfev: options.max_nfev.is_some(),
                max_nfev: options.max_nfev.unwrap_or(0),
            };
            SidereonStatus::Ok
        },
    )
}

/// Fit a linear geodetic trajectory model. On success writes a trajectory
/// handle to *out_trajectory.
///
/// Safety: series and model must point to valid structs; options may be NULL
/// for defaults; out_trajectory must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_fit_trajectory(
    series: *const SidereonGeodeticPositionSeries,
    model: *const SidereonGeodeticTrajectoryModel,
    options: *const SidereonGeodeticTrajectoryFitOptions,
    out_trajectory: *mut *mut SidereonGeodeticTrajectory,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_fit_trajectory",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_trajectory,
                "sidereon_geodetic_fit_trajectory",
                "out_trajectory"
            ));
            *out = ptr::null_mut();
            let series = c_try!(require_ref(
                series,
                "sidereon_geodetic_fit_trajectory",
                "series"
            ));
            let model = c_try!(require_ref(
                model,
                "sidereon_geodetic_fit_trajectory",
                "model"
            ));
            let samples = c_try!(geodetic_samples_from_c(
                "sidereon_geodetic_fit_trajectory",
                series,
            ));
            let frame = c_try!(position_frame_from_c(
                "sidereon_geodetic_fit_trajectory",
                series,
            ));
            let core_series = sidereon_core::geodetic_time_series::PositionSeries {
                frame,
                samples: &samples,
            };
            let model = c_try!(trajectory_model_from_c(
                "sidereon_geodetic_fit_trajectory",
                model,
            ));
            let options = c_try!(trajectory_fit_options_from_c(
                "sidereon_geodetic_fit_trajectory",
                options,
            ));
            match sidereon_core::geodetic_time_series::fit_trajectory(&core_series, &model, options)
            {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonGeodeticTrajectory { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_geodetic_time_series_error("sidereon_geodetic_fit_trajectory", err),
            }
        },
    )
}

/// Copy a trajectory summary.
///
/// Safety: trajectory must be a live handle; out_summary must point to a
/// SidereonGeodeticTrajectorySummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_summary(
    trajectory: *const SidereonGeodeticTrajectory,
    out_summary: *mut SidereonGeodeticTrajectorySummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_geodetic_trajectory_summary",
                "out_summary"
            ));
            *out = empty_trajectory_summary();
            let trajectory = c_try!(require_ref(
                trajectory,
                "sidereon_geodetic_trajectory_summary",
                "trajectory"
            ));
            *out = trajectory_summary_to_c(&trajectory.inner);
            SidereonStatus::Ok
        },
    )
}

/// Copy the three ENU trajectory component summaries.
///
/// Safety: out_components must point to at least 3 component structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_components(
    trajectory: *const SidereonGeodeticTrajectory,
    out_components: *mut SidereonGeodeticTrajectoryComponent,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_components",
        SidereonStatus::Panic,
        || {
            c_try!(require_out_array(
                out_components,
                3,
                "sidereon_geodetic_trajectory_components",
                "out_components"
            ));
            let trajectory = c_try!(require_ref(
                trajectory,
                "sidereon_geodetic_trajectory_components",
                "trajectory"
            ));
            for axis in 0..3 {
                *out_components.add(axis) =
                    trajectory_component_to_c(&trajectory.inner.components[axis]);
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy trajectory terms. Uses the variable-length output contract.
///
/// Safety: trajectory must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_terms(
    trajectory: *const SidereonGeodeticTrajectory,
    out: *mut SidereonGeodeticTrajectoryTerm,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_terms",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geodetic_trajectory_terms",
                out_written,
                out_required
            ));
            let trajectory = c_try!(require_ref(
                trajectory,
                "sidereon_geodetic_trajectory_terms",
                "trajectory"
            ));
            let terms: Vec<_> = trajectory
                .inner
                .terms
                .iter()
                .map(trajectory_term_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_geodetic_trajectory_terms",
                "out",
                &terms,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one ENU component's offset coefficients. Uses the variable-length
/// output contract.
///
/// Safety: trajectory must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_offsets(
    trajectory: *const SidereonGeodeticTrajectory,
    axis: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_offsets",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geodetic_trajectory_offsets",
                out_written,
                out_required
            ));
            if axis >= 3 {
                set_last_error(
                    "sidereon_geodetic_trajectory_offsets: axis must be 0, 1, or 2".to_string(),
                );
                return SidereonStatus::InvalidArgument;
            }
            let trajectory = c_try!(require_ref(
                trajectory,
                "sidereon_geodetic_trajectory_offsets",
                "trajectory"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_geodetic_trajectory_offsets",
                "out",
                &trajectory.inner.components[axis].offsets_m,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the flattened row-major trajectory parameter covariance.
///
/// Safety: trajectory must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_parameter_covariance(
    trajectory: *const SidereonGeodeticTrajectory,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_trajectory_parameter_covariance",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geodetic_trajectory_parameter_covariance",
                out_written,
                out_required
            ));
            let trajectory = c_try!(require_ref(
                trajectory,
                "sidereon_geodetic_trajectory_parameter_covariance",
                "trajectory"
            ));
            let flat = flatten_vec_matrix(&trajectory.inner.parameter_covariance);
            c_try!(copy_prefix_to_c(
                "sidereon_geodetic_trajectory_parameter_covariance",
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

/// Release a trajectory handle. Null is a no-op.
///
/// Safety: trajectory must be NULL or a live handle from
/// sidereon_geodetic_fit_trajectory.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_trajectory_free(
    trajectory: *mut SidereonGeodeticTrajectory,
) {
    ffi_boundary("sidereon_geodetic_trajectory_free", (), || {
        free_boxed(trajectory);
    });
}

/// Initialize step-detection options with core defaults.
///
/// Safety: out_options must point to SidereonGeodeticStepDetectionOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_step_detection_options_init(
    out_options: *mut SidereonGeodeticStepDetectionOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_step_detection_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_geodetic_step_detection_options_init",
                "out_options"
            ));
            *out = step_options_to_c(
                sidereon_core::geodetic_time_series::StepDetectionOptions::default(),
            );
            SidereonStatus::Ok
        },
    )
}

/// Detect candidate displacement steps. Uses the variable-length output
/// contract.
///
/// Safety: series must point to a valid series; out may be NULL only when len
/// is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_detect_steps(
    series: *const SidereonGeodeticPositionSeries,
    options: *const SidereonGeodeticStepDetectionOptions,
    out: *mut SidereonGeodeticStepCandidate,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_detect_steps",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geodetic_detect_steps",
                out_written,
                out_required
            ));
            let series = c_try!(require_ref(
                series,
                "sidereon_geodetic_detect_steps",
                "series"
            ));
            let samples = c_try!(geodetic_samples_from_c(
                "sidereon_geodetic_detect_steps",
                series,
            ));
            let frame = c_try!(position_frame_from_c(
                "sidereon_geodetic_detect_steps",
                series,
            ));
            let core_series = sidereon_core::geodetic_time_series::PositionSeries {
                frame,
                samples: &samples,
            };
            let options = c_try!(step_options_from_c(
                "sidereon_geodetic_detect_steps",
                options,
            ));
            let candidates =
                match sidereon_core::geodetic_time_series::detect_steps(&core_series, options) {
                    Ok(candidates) => candidates,
                    Err(err) => {
                        return map_geodetic_time_series_error(
                            "sidereon_geodetic_detect_steps",
                            err,
                        )
                    }
                };
            let values: Vec<_> = candidates.iter().map(step_candidate_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_geodetic_detect_steps",
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

/// Estimate a network motion field. On success writes a handle to *out_field.
///
/// Safety: stations points to station_count entries; out_field must point to
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_network_field(
    stations: *const SidereonGeodeticNetworkStation,
    station_count: usize,
    frame: SidereonGeodeticNetworkFrame,
    out_field: *mut *mut SidereonGeodeticMotionField,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_network_field",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_field,
                "sidereon_geodetic_network_field",
                "out_field"
            ));
            *out = ptr::null_mut();
            let raw = c_try!(require_slice(
                stations,
                station_count,
                "sidereon_geodetic_network_field",
                "stations"
            ));
            let parsed = c_try!(network_stations_from_c(
                "sidereon_geodetic_network_field",
                raw,
            ));
            let core_stations: Vec<_> = parsed
                .iter()
                .map(
                    |station| sidereon_core::geodetic_time_series::NetworkStation {
                        id: &station.id,
                        reference: station.reference,
                        series: sidereon_core::geodetic_time_series::PositionSeries {
                            frame: station.frame,
                            samples: &station.samples,
                        },
                    },
                )
                .collect();
            let frame = sidereon_core::geodetic_time_series::NetworkFrame {
                origin: c_try!(geodetic_to_wgs84(
                    "sidereon_geodetic_network_field",
                    "frame.origin",
                    frame.origin,
                )),
                remove_common_mode: frame.remove_common_mode,
            };
            match sidereon_core::geodetic_time_series::network_field(&core_stations, frame) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonGeodeticMotionField { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_geodetic_time_series_error("sidereon_geodetic_network_field", err),
            }
        },
    )
}

/// Copy the common-mode velocity removed from a motion field.
///
/// Safety: field must be live; out_common_mode must point to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_motion_field_common_mode(
    field: *const SidereonGeodeticMotionField,
    out_common_mode: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_motion_field_common_mode",
        SidereonStatus::Panic,
        || {
            let field = c_try!(require_ref(
                field,
                "sidereon_geodetic_motion_field_common_mode",
                "field"
            ));
            c_try!(copy_exact_f64s(
                "sidereon_geodetic_motion_field_common_mode",
                "out_common_mode",
                out_common_mode,
                3,
                &field.inner.common_mode_enu_m_per_yr,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy station motions from a motion field. Uses the variable-length output
/// contract.
///
/// Safety: field must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_motion_field_stations(
    field: *const SidereonGeodeticMotionField,
    out: *mut SidereonGeodeticStationMotion,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geodetic_motion_field_stations",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geodetic_motion_field_stations",
                out_written,
                out_required
            ));
            let field = c_try!(require_ref(
                field,
                "sidereon_geodetic_motion_field_stations",
                "field"
            ));
            let stations: Vec<_> = field
                .inner
                .stations
                .iter()
                .map(station_motion_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_geodetic_motion_field_stations",
                "out",
                &stations,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a motion field handle. Null is a no-op.
///
/// Safety: field must be NULL or a live handle from
/// sidereon_geodetic_network_field.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_motion_field_free(
    field: *mut SidereonGeodeticMotionField,
) {
    ffi_boundary("sidereon_geodetic_motion_field_free", (), || {
        free_boxed(field);
    });
}

unsafe fn geodetic_samples_from_c(
    fn_name: &str,
    series: &SidereonGeodeticPositionSeries,
) -> Result<Vec<sidereon_core::geodetic_time_series::PositionSample>, SidereonStatus> {
    let raw = require_slice(
        series.samples,
        series.sample_count,
        fn_name,
        "series.samples",
    )?;
    let mut samples = Vec::with_capacity(raw.len());
    for (idx, sample) in raw.iter().enumerate() {
        let covariance_m2 = if sample.has_covariance_m2 {
            Some(mat3_from_row_major(sample.covariance_m2))
        } else {
            None
        };
        samples.push(sidereon_core::geodetic_time_series::PositionSample {
            epoch_year: sample.epoch_year,
            position_m: sample.position_m,
            covariance_m2,
        });
        validate_element_count::<sidereon_core::geodetic_time_series::PositionSample>(
            fn_name,
            &format!("series.samples[{idx}]"),
            1,
        )?;
    }
    Ok(samples)
}

fn position_frame_from_c(
    fn_name: &str,
    series: &SidereonGeodeticPositionSeries,
) -> Result<sidereon_core::geodetic_time_series::PositionFrame, SidereonStatus> {
    match series.frame {
        value if value == SidereonGeodeticTimeSeriesFrame::Enu as u32 => {
            Ok(sidereon_core::geodetic_time_series::PositionFrame::Enu)
        }
        value if value == SidereonGeodeticTimeSeriesFrame::Ecef as u32 => {
            Ok(sidereon_core::geodetic_time_series::PositionFrame::Ecef {
                reference: geodetic_to_wgs84(fn_name, "series.reference", series.reference)?,
            })
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid geodetic time-series frame"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn midas_options_from_c(
    fn_name: &str,
    options: *const SidereonMidasOptions,
) -> Result<sidereon_core::geodetic_time_series::MidasOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(sidereon_core::geodetic_time_series::MidasOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    Ok(sidereon_core::geodetic_time_series::MidasOptions {
        dominant_period_years: options.dominant_period_years,
        period_tolerance_years: options.period_tolerance_years,
        min_pairs: options.min_pairs,
    })
}

fn midas_options_to_c(
    options: sidereon_core::geodetic_time_series::MidasOptions,
) -> SidereonMidasOptions {
    SidereonMidasOptions {
        dominant_period_years: options.dominant_period_years,
        period_tolerance_years: options.period_tolerance_years,
        min_pairs: options.min_pairs,
    }
}

fn midas_velocity_to_c(
    velocity: &sidereon_core::geodetic_time_series::Velocity,
) -> SidereonMidasVelocity {
    SidereonMidasVelocity {
        rate_enu_m_per_yr: velocity.rate_enu_m_per_yr,
        sigma_enu_m_per_yr: velocity.sigma_enu_m_per_yr,
        covariance_enu_m2_per_yr2: flatten_mat3(velocity.covariance_enu_m2_per_yr2),
        component_stats: [
            midas_stats_to_c(velocity.component_stats[0]),
            midas_stats_to_c(velocity.component_stats[1]),
            midas_stats_to_c(velocity.component_stats[2]),
        ],
        sample_count: velocity.sample_count,
        span_years: velocity.span_years,
        quality: match velocity.quality {
            sidereon_core::geodetic_time_series::TimeSeriesQuality::Nominal => {
                SidereonGeodeticTimeSeriesQuality::Nominal as u32
            }
            sidereon_core::geodetic_time_series::TimeSeriesQuality::ShortSpan => {
                SidereonGeodeticTimeSeriesQuality::ShortSpan as u32
            }
        },
    }
}

fn empty_midas_velocity() -> SidereonMidasVelocity {
    SidereonMidasVelocity {
        rate_enu_m_per_yr: [0.0; 3],
        sigma_enu_m_per_yr: [0.0; 3],
        covariance_enu_m2_per_yr2: [0.0; 9],
        component_stats: [SidereonMidasComponentStats {
            pair_count: 0,
            retained_pair_count: 0,
            slope_sigma_m_per_yr: 0.0,
            effective_pair_count: 0.0,
        }; 3],
        sample_count: 0,
        span_years: 0.0,
        quality: SidereonGeodeticTimeSeriesQuality::Nominal as u32,
    }
}

fn midas_stats_to_c(
    stats: sidereon_core::geodetic_time_series::MidasComponentStats,
) -> SidereonMidasComponentStats {
    SidereonMidasComponentStats {
        pair_count: stats.pair_count,
        retained_pair_count: stats.retained_pair_count,
        slope_sigma_m_per_yr: stats.slope_sigma_m_per_yr,
        effective_pair_count: stats.effective_pair_count,
    }
}

unsafe fn trajectory_model_from_c(
    fn_name: &str,
    model: &SidereonGeodeticTrajectoryModel,
) -> Result<sidereon_core::geodetic_time_series::TrajectoryModel, SidereonStatus> {
    let offsets = require_slice(
        model.offset_epochs_year,
        model.offset_count,
        fn_name,
        "model.offset_epochs_year",
    )?;
    Ok(sidereon_core::geodetic_time_series::TrajectoryModel {
        reference_epoch_year: model
            .has_reference_epoch_year
            .then_some(model.reference_epoch_year),
        include_annual: model.include_annual,
        include_semiannual: model.include_semiannual,
        offset_epochs_year: offsets.to_vec(),
    })
}

unsafe fn trajectory_fit_options_from_c(
    fn_name: &str,
    options: *const SidereonGeodeticTrajectoryFitOptions,
) -> Result<sidereon_core::geodetic_time_series::TrajectoryFitOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(sidereon_core::geodetic_time_series::TrajectoryFitOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    Ok(sidereon_core::geodetic_time_series::TrajectoryFitOptions {
        loss: geodetic_loss_from_c(fn_name, options.loss)?,
        f_scale_m: options.f_scale_m,
        max_nfev: options.has_max_nfev.then_some(options.max_nfev),
    })
}

fn geodetic_loss_from_c(
    fn_name: &str,
    loss: u32,
) -> Result<sidereon_core::geodetic_time_series::Loss, SidereonStatus> {
    match loss {
        value if value == SidereonGeodeticTrajectoryLoss::Linear as u32 => {
            Ok(sidereon_core::geodetic_time_series::Loss::Linear)
        }
        value if value == SidereonGeodeticTrajectoryLoss::SoftL1 as u32 => {
            Ok(sidereon_core::geodetic_time_series::Loss::SoftL1)
        }
        value if value == SidereonGeodeticTrajectoryLoss::Huber as u32 => {
            Ok(sidereon_core::geodetic_time_series::Loss::Huber)
        }
        value if value == SidereonGeodeticTrajectoryLoss::Cauchy as u32 => {
            Ok(sidereon_core::geodetic_time_series::Loss::Cauchy)
        }
        value if value == SidereonGeodeticTrajectoryLoss::Arctan as u32 => {
            Ok(sidereon_core::geodetic_time_series::Loss::Arctan)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid trajectory loss"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn trajectory_summary_to_c(
    trajectory: &sidereon_core::geodetic_time_series::Trajectory,
) -> SidereonGeodeticTrajectorySummary {
    let covariance_dim = trajectory.parameter_covariance.len();
    SidereonGeodeticTrajectorySummary {
        reference_epoch_year: trajectory.reference_epoch_year,
        term_count: trajectory.terms.len(),
        covariance_dim,
        residual_rms_enu_m: trajectory.residual_rms_enu_m,
        geometry_quality: geometry_quality_to_c(&trajectory.geometry_quality),
        status: trajectory.status,
        nfev: trajectory.nfev,
        njev: trajectory.njev,
        cost: trajectory.cost,
        optimality: trajectory.optimality,
    }
}

fn empty_trajectory_summary() -> SidereonGeodeticTrajectorySummary {
    SidereonGeodeticTrajectorySummary {
        reference_epoch_year: 0.0,
        term_count: 0,
        covariance_dim: 0,
        residual_rms_enu_m: [0.0; 3],
        geometry_quality: empty_geometry_quality(),
        status: 0,
        nfev: 0,
        njev: 0,
        cost: 0.0,
        optimality: 0.0,
    }
}

fn trajectory_component_to_c(
    component: &sidereon_core::geodetic_time_series::TrajectoryComponent,
) -> SidereonGeodeticTrajectoryComponent {
    SidereonGeodeticTrajectoryComponent {
        position_m: component.position_m,
        velocity_m_per_yr: component.velocity_m_per_yr,
        has_annual_sin_m: component.annual_sin_m.is_some(),
        annual_sin_m: component.annual_sin_m.unwrap_or(0.0),
        has_annual_cos_m: component.annual_cos_m.is_some(),
        annual_cos_m: component.annual_cos_m.unwrap_or(0.0),
        has_semiannual_sin_m: component.semiannual_sin_m.is_some(),
        semiannual_sin_m: component.semiannual_sin_m.unwrap_or(0.0),
        has_semiannual_cos_m: component.semiannual_cos_m.is_some(),
        semiannual_cos_m: component.semiannual_cos_m.unwrap_or(0.0),
        offset_count: component.offsets_m.len(),
    }
}

fn trajectory_term_to_c(
    term: &sidereon_core::geodetic_time_series::TrajectoryTerm,
) -> SidereonGeodeticTrajectoryTerm {
    match *term {
        sidereon_core::geodetic_time_series::TrajectoryTerm::Position => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::Position)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::Velocity => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::Velocity)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::AnnualSin => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::AnnualSin)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::AnnualCos => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::AnnualCos)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::SemiannualSin => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::SemiannualSin)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::SemiannualCos => {
            trajectory_term_simple(SidereonGeodeticTrajectoryTermKind::SemiannualCos)
        }
        sidereon_core::geodetic_time_series::TrajectoryTerm::Offset { index, epoch_year } => {
            SidereonGeodeticTrajectoryTerm {
                kind: SidereonGeodeticTrajectoryTermKind::Offset as u32,
                offset_index: index,
                epoch_year,
            }
        }
    }
}

fn trajectory_term_simple(
    kind: SidereonGeodeticTrajectoryTermKind,
) -> SidereonGeodeticTrajectoryTerm {
    SidereonGeodeticTrajectoryTerm {
        kind: kind as u32,
        offset_index: 0,
        epoch_year: 0.0,
    }
}

unsafe fn step_options_from_c(
    fn_name: &str,
    options: *const SidereonGeodeticStepDetectionOptions,
) -> Result<sidereon_core::geodetic_time_series::StepDetectionOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(sidereon_core::geodetic_time_series::StepDetectionOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    Ok(sidereon_core::geodetic_time_series::StepDetectionOptions {
        window_years: options.window_years,
        score_threshold: options.score_threshold,
        min_offset_m: options.min_offset_m,
        min_samples_each_side: options.min_samples_each_side,
        min_separation_years: options.min_separation_years,
        midas: sidereon_core::geodetic_time_series::MidasOptions {
            dominant_period_years: options.midas.dominant_period_years,
            period_tolerance_years: options.midas.period_tolerance_years,
            min_pairs: options.midas.min_pairs,
        },
    })
}

fn step_options_to_c(
    options: sidereon_core::geodetic_time_series::StepDetectionOptions,
) -> SidereonGeodeticStepDetectionOptions {
    SidereonGeodeticStepDetectionOptions {
        window_years: options.window_years,
        score_threshold: options.score_threshold,
        min_offset_m: options.min_offset_m,
        min_samples_each_side: options.min_samples_each_side,
        min_separation_years: options.min_separation_years,
        midas: midas_options_to_c(options.midas),
    }
}

fn step_candidate_to_c(
    candidate: &sidereon_core::geodetic_time_series::StepCandidate,
) -> SidereonGeodeticStepCandidate {
    SidereonGeodeticStepCandidate {
        epoch_year: candidate.epoch_year,
        offset_enu_m: candidate.offset_enu_m,
        score: candidate.score,
        before_count: candidate.before_count,
        after_count: candidate.after_count,
        heuristic: SidereonGeodeticStepDetectionHeuristic::DetrendedSlidingMedian as u32,
    }
}

struct ParsedNetworkStation {
    id: String,
    reference: Wgs84Geodetic,
    frame: sidereon_core::geodetic_time_series::PositionFrame,
    samples: Vec<sidereon_core::geodetic_time_series::PositionSample>,
}

unsafe fn network_stations_from_c(
    fn_name: &str,
    raw: &[SidereonGeodeticNetworkStation],
) -> Result<Vec<ParsedNetworkStation>, SidereonStatus> {
    let mut parsed = Vec::with_capacity(raw.len());
    for (idx, station) in raw.iter().enumerate() {
        let id = parse_bounded_c_string(
            fn_name,
            &format!("stations[{idx}].id"),
            station.id,
            MAX_GEODETIC_STATION_ID_BYTES,
        )?;
        let reference = geodetic_to_wgs84(
            fn_name,
            &format!("stations[{idx}].reference"),
            station.reference,
        )?;
        let samples = geodetic_samples_from_c(fn_name, &station.series)?;
        let frame = position_frame_from_c(fn_name, &station.series)?;
        parsed.push(ParsedNetworkStation {
            id,
            reference,
            frame,
            samples,
        });
    }
    Ok(parsed)
}

fn station_motion_to_c(
    motion: &sidereon_core::geodetic_time_series::StationMotion,
) -> SidereonGeodeticStationMotion {
    SidereonGeodeticStationMotion {
        id: geodetic_station_id_token(&motion.id),
        rate_enu_m_per_yr: motion.rate_enu_m_per_yr,
        raw_rate_enu_m_per_yr: motion.raw_rate_enu_m_per_yr,
        sigma_enu_m_per_yr: motion.sigma_enu_m_per_yr,
        local_velocity: midas_velocity_to_c(&motion.local_velocity),
    }
}

fn geodetic_station_id_token(text: &str) -> SidereonGeodeticStationId {
    let mut token = SidereonGeodeticStationId {
        bytes: [0; GEODETIC_STATION_ID_C_BYTES],
    };
    for (idx, byte) in text.bytes().take(MAX_GEODETIC_STATION_ID_BYTES).enumerate() {
        token.bytes[idx] = byte as c_char;
    }
    token
}

fn flatten_vec_matrix(matrix: &[Vec<f64>]) -> Vec<f64> {
    let mut out = Vec::new();
    for row in matrix {
        out.extend_from_slice(row);
    }
    out
}

fn mat3_from_row_major(values: [f64; 9]) -> [[f64; 3]; 3] {
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn map_geodetic_time_series_error(
    fn_name: &str,
    err: sidereon_core::geodetic_time_series::GeodeticTimeSeriesError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
