use super::*;

// ===========================================================================

/// Which mean-element model a reduced-orbit fit/evaluation uses.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonReducedOrbitModel {
    /// Circular orbit (eccentricity fixed at zero); the engine default.
    CircularSecular = 0,
    /// Eccentric orbit via a nonsingular (h, k) parameterization.
    EccentricSecular = 1,
}

/// Reference frame a reduced-orbit position/velocity result is expressed in.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonReducedOrbitFrame {
    /// Inertial GCRS (ECI).
    Gcrs = 0,
    /// Earth-fixed ITRF/IGS ECEF.
    Ecef = 1,
}

/// A UTC calendar instant (year, month, day, hour, minute, fractional second).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCalendarEpoch {
    /// Calendar year.
    pub year: i32,
    /// Calendar month, 1-12.
    pub month: i32,
    /// Calendar day of month, 1-31.
    pub day: i32,
    /// Hour of day, 0-23.
    pub hour: i32,
    /// Minute of hour, 0-59.
    pub minute: i32,
    /// Second of minute, fractional.
    pub second: f64,
}

/// One fit/drift truth sample: a calendar epoch and an ECEF (ITRF) position.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEcefSample {
    /// Sample epoch.
    pub epoch: SidereonCalendarEpoch,
    /// ECEF X, meters.
    pub x_m: f64,
    /// ECEF Y, meters.
    pub y_m: f64,
    /// ECEF Z, meters.
    pub z_m: f64,
}

/// Fitted reduced-orbit mean elements. The `model` field is a
/// SidereonReducedOrbitModel value (cast to uint32_t).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitElements {
    /// Model these elements belong to (SidereonReducedOrbitModel as uint32_t).
    pub model: u32,
    /// Reference epoch t0.
    pub epoch: SidereonCalendarEpoch,
    /// Semi-major axis a, meters.
    pub a_m: f64,
    /// Eccentricity.
    pub e: f64,
    /// Inclination i, radians.
    pub i_rad: f64,
    /// RAAN at t0, radians.
    pub raan_rad: f64,
    /// Fitted nodal regression rate, radians per second.
    pub raan_rate_rad_s: f64,
    /// J2 nodal-regression seed for raan_rate, radians per second.
    pub raan_rate_j2_rad_s: f64,
    /// Argument of latitude at t0, radians.
    pub arg_lat_rad: f64,
    /// Mean motion n, radians per second.
    pub mean_motion_rad_s: f64,
    /// Eccentricity vector component h = e*sin(omega).
    pub h: f64,
    /// Eccentricity vector component k = e*cos(omega).
    pub k: f64,
    /// Argument of perigee omega, radians.
    pub arg_perigee_rad: f64,
}

/// Residual statistics from a reduced-orbit fit, meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitFitStats {
    /// RMS GCRS position residual over the samples, meters.
    pub rms_m: f64,
    /// Maximum GCRS position residual over the samples, meters.
    pub max_m: f64,
    /// Number of samples used in the fit.
    pub n_samples: usize,
}

/// Sampling window and cadence for source-backed reduced-orbit drivers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitSourceSampling {
    /// Inclusive sample window start.
    pub t0: SidereonCalendarEpoch,
    /// Inclusive sample window end.
    pub t1: SidereonCalendarEpoch,
    /// Sampling cadence, seconds.
    pub cadence_s: f64,
}

/// Source-backed reduced-orbit fit options.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitSourceFitOptions {
    /// Sampling window and cadence.
    pub sampling: SidereonReducedOrbitSourceSampling,
    /// Model to fit, as SidereonReducedOrbitModel.
    pub model: u32,
}

/// Source-backed reduced-orbit drift options.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitSourceDriftOptions {
    /// Sampling window and cadence.
    pub sampling: SidereonReducedOrbitSourceSampling,
    /// Threshold used to mark the first crossing, meters.
    pub threshold_m: f64,
}

/// Residual statistics plus source-sampling metadata from a source-backed fit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitSourceFitStats {
    /// Fit residual statistics.
    pub fit: SidereonReducedOrbitFitStats,
    /// Number of samples requested from the source before unavailable epochs
    /// were skipped.
    pub requested_samples: usize,
}

/// One per-epoch drift entry from sidereon_reduced_orbit_drift_report_entries.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitDriftEntry {
    /// Epoch evaluated.
    pub epoch: SidereonCalendarEpoch,
    /// Position error magnitude (model minus truth), meters.
    pub error_m: f64,
}

/// Aggregate drift summary from sidereon_reduced_orbit_drift_report_summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitDriftSummary {
    /// Maximum error over the horizon, meters.
    pub max_m: f64,
    /// RMS error over the horizon, meters.
    pub rms_m: f64,
    /// Whether the error crossed the requested threshold over the horizon.
    pub has_threshold_crossing: bool,
    /// Index of the first threshold-crossing entry within the drift report
    /// entries (sidereon_reduced_orbit_drift_report_entries), whose `epoch` field
    /// is the crossing epoch. Valid only when has_threshold_crossing is true; the
    /// crossing epoch is read from that entry rather than fabricated here.
    pub threshold_index: usize,
}

/// A source-backed reduced-orbit drift report. Opaque to C. Create with
/// sidereon_reduced_orbit_drift and release with
/// sidereon_reduced_orbit_drift_report_free.
pub struct SidereonReducedOrbitDriftReport {
    pub(crate) inner: DriftReport,
    pub(crate) requested_samples: usize,
}

/// Fit a reduced-orbit model to ECEF/ITRF samples. Delegates to
/// sidereon_core::orbit::fit_with_model. `scale` is a SidereonTimeScale and
/// `model` is a SidereonReducedOrbitModel (both cast to uint32_t). On success
/// writes the fitted elements to *out_elements and the residual statistics to
/// *out_stats.
///
/// Safety: samples must point to count SidereonEcefSample; out_elements must
/// point to a SidereonReducedOrbitElements; out_stats must point to a
/// SidereonReducedOrbitFitStats.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit(
    samples: *const SidereonEcefSample,
    count: usize,
    scale: u32,
    model: u32,
    out_elements: *mut SidereonReducedOrbitElements,
    out_stats: *mut SidereonReducedOrbitFitStats,
) -> SidereonStatus {
    ffi_boundary("sidereon_reduced_orbit_fit", SidereonStatus::Panic, || {
        let out_elements = c_try!(require_out(
            out_elements,
            "sidereon_reduced_orbit_fit",
            "out_elements"
        ));
        let out_stats = c_try!(require_out(
            out_stats,
            "sidereon_reduced_orbit_fit",
            "out_stats"
        ));
        let raw = c_try!(require_slice(
            samples,
            count,
            "sidereon_reduced_orbit_fit",
            "samples"
        ));
        let scale = c_try!(time_scale_from_c_code(
            "sidereon_reduced_orbit_fit",
            "scale",
            scale
        ));
        let model = c_try!(reduced_orbit_model_from_c(
            "sidereon_reduced_orbit_fit",
            "model",
            model
        ));
        let parsed: Vec<EcefSample> = raw.iter().map(ecef_sample_from_c).collect();
        let orbit: ReducedOrbit = match reduced_orbit_fit_core(&parsed, scale, model) {
            Ok(orbit) => orbit,
            Err(err) => return map_reduced_orbit_error("sidereon_reduced_orbit_fit", err),
        };
        *out_elements = reduced_orbit_elements_to_c(&orbit.elements);
        *out_stats = SidereonReducedOrbitFitStats {
            rms_m: orbit.stats.rms_m,
            max_m: orbit.stats.max_m,
            n_samples: orbit.stats.n_samples,
        };
        SidereonStatus::Ok
    })
}

/// Fit a reduced-orbit model by sampling one satellite from an SP3 product.
/// Delegates to sidereon_core::orbit::fit_reduced_orbit_source.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated satellite
/// token; options, out_elements, and out_stats must point to their documented
/// structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit_sp3_source(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonReducedOrbitSourceFitOptions,
    out_elements: *mut SidereonReducedOrbitElements,
    out_stats: *mut SidereonReducedOrbitSourceFitStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_fit_sp3_source",
        SidereonStatus::Panic,
        || {
            let out_elements = c_try!(require_out(
                out_elements,
                "sidereon_reduced_orbit_fit_sp3_source",
                "out_elements"
            ));
            let out_stats = c_try!(require_out(
                out_stats,
                "sidereon_reduced_orbit_fit_sp3_source",
                "out_stats"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_reduced_orbit_fit_sp3_source",
                "sp3"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_reduced_orbit_fit_sp3_source",
                sat_id
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_fit_sp3_source",
                "options"
            ));
            let options = c_try!(reduced_orbit_source_fit_options_from_c(
                "sidereon_reduced_orbit_fit_sp3_source",
                options
            ));
            let source = ReducedOrbitSource::Sp3 {
                product: &sp3.inner,
                satellite: sat,
            };
            let fit = match reduced_orbit_fit_source_core(source, options) {
                Ok(fit) => fit,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_fit_sp3_source",
                        err,
                    )
                }
            };
            *out_elements = reduced_orbit_elements_to_c(&fit.orbit.elements);
            *out_stats = SidereonReducedOrbitSourceFitStats {
                fit: SidereonReducedOrbitFitStats {
                    rms_m: fit.orbit.stats.rms_m,
                    max_m: fit.orbit.stats.max_m,
                    n_samples: fit.orbit.stats.n_samples,
                },
                requested_samples: fit.requested_samples,
            };
            SidereonStatus::Ok
        },
    )
}

/// Fit a reduced-orbit model by sampling a TLE/SGP4 source in UTC. Delegates to
/// sidereon_core::orbit::fit_reduced_orbit_source.
///
/// Safety: tle must be a live handle; options, out_elements, and out_stats must
/// point to their documented structs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit_tle_source(
    tle: *const SidereonTle,
    options: *const SidereonReducedOrbitSourceFitOptions,
    out_elements: *mut SidereonReducedOrbitElements,
    out_stats: *mut SidereonReducedOrbitSourceFitStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_fit_tle_source",
        SidereonStatus::Panic,
        || {
            let out_elements = c_try!(require_out(
                out_elements,
                "sidereon_reduced_orbit_fit_tle_source",
                "out_elements"
            ));
            let out_stats = c_try!(require_out(
                out_stats,
                "sidereon_reduced_orbit_fit_tle_source",
                "out_stats"
            ));
            let tle = c_try!(require_ref(
                tle,
                "sidereon_reduced_orbit_fit_tle_source",
                "tle"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_fit_tle_source",
                "options"
            ));
            let options = c_try!(reduced_orbit_source_fit_options_from_c(
                "sidereon_reduced_orbit_fit_tle_source",
                options
            ));
            let source = ReducedOrbitSource::Sgp4 {
                satellite: &tle.satellite,
            };
            let fit = match reduced_orbit_fit_source_core(source, options) {
                Ok(fit) => fit,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_fit_tle_source",
                        err,
                    )
                }
            };
            *out_elements = reduced_orbit_elements_to_c(&fit.orbit.elements);
            *out_stats = SidereonReducedOrbitSourceFitStats {
                fit: SidereonReducedOrbitFitStats {
                    rms_m: fit.orbit.stats.rms_m,
                    max_m: fit.orbit.stats.max_m,
                    n_samples: fit.orbit.stats.n_samples,
                },
                requested_samples: fit.requested_samples,
            };
            SidereonStatus::Ok
        },
    )
}

/// Evaluate the reduced-orbit position at one epoch in the requested frame.
/// Delegates to sidereon_core::orbit::position. `scale` is a SidereonTimeScale
/// and `frame` is a SidereonReducedOrbitFrame (cast to uint32_t). Writes the
/// ECEF/GCRS position (meters) to out_xyz.
///
/// Safety: elements must point to a SidereonReducedOrbitElements; epoch must
/// point to a SidereonCalendarEpoch; out_xyz must point to len writable doubles
/// (len must be at least 3).
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_position(
    elements: *const SidereonReducedOrbitElements,
    epoch: *const SidereonCalendarEpoch,
    scale: u32,
    frame: u32,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_position",
        SidereonStatus::Panic,
        || {
            let elements = c_try!(require_ref(
                elements,
                "sidereon_reduced_orbit_position",
                "elements"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_reduced_orbit_position",
                "epoch"
            ));
            let scale = c_try!(time_scale_from_c_code(
                "sidereon_reduced_orbit_position",
                "scale",
                scale
            ));
            let frame = c_try!(reduced_orbit_frame_from_c(
                "sidereon_reduced_orbit_position",
                "frame",
                frame
            ));
            let elements = c_try!(reduced_orbit_elements_from_c(
                "sidereon_reduced_orbit_position",
                elements
            ));
            let position = match reduced_orbit_position_core(
                &elements,
                calendar_epoch_from_c(epoch),
                scale,
                frame,
            ) {
                Ok(position) => position,
                Err(err) => return map_reduced_orbit_error("sidereon_reduced_orbit_position", err),
            };
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_position",
                "out_xyz",
                out_xyz,
                len,
                &position
            ));
            SidereonStatus::Ok
        },
    )
}

/// Evaluate the reduced-orbit position and velocity at one epoch in the
/// requested frame. Delegates to sidereon_core::orbit::position_velocity. Writes
/// position (meters) to out_pos and velocity (meters per second) to out_vel.
/// ECEF velocity includes the Earth-rotation transport term.
///
/// Safety: elements must point to a SidereonReducedOrbitElements; epoch must
/// point to a SidereonCalendarEpoch; out_pos and out_vel must each point to
/// three writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_position_velocity(
    elements: *const SidereonReducedOrbitElements,
    epoch: *const SidereonCalendarEpoch,
    scale: u32,
    frame: u32,
    out_pos: *mut f64,
    out_vel: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_position_velocity",
        SidereonStatus::Panic,
        || {
            let elements = c_try!(require_ref(
                elements,
                "sidereon_reduced_orbit_position_velocity",
                "elements"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_reduced_orbit_position_velocity",
                "epoch"
            ));
            let scale = c_try!(time_scale_from_c_code(
                "sidereon_reduced_orbit_position_velocity",
                "scale",
                scale
            ));
            let frame = c_try!(reduced_orbit_frame_from_c(
                "sidereon_reduced_orbit_position_velocity",
                "frame",
                frame
            ));
            let elements = c_try!(reduced_orbit_elements_from_c(
                "sidereon_reduced_orbit_position_velocity",
                elements
            ));
            let (position, velocity) = match reduced_orbit_position_velocity_core(
                &elements,
                calendar_epoch_from_c(epoch),
                scale,
                frame,
            ) {
                Ok(pair) => pair,
                Err(err) => {
                    return map_reduced_orbit_error("sidereon_reduced_orbit_position_velocity", err)
                }
            };
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_position_velocity",
                "out_pos",
                out_pos,
                3,
                &position
            ));
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_position_velocity",
                "out_vel",
                out_vel,
                3,
                &velocity
            ));
            SidereonStatus::Ok
        },
    )
}

/// Evaluate model-vs-truth drift over a horizon of truth samples. Delegates to
/// sidereon_core::orbit::drift. On success writes a newly owned report handle to
/// *out_report; release it with sidereon_reduced_orbit_drift_report_free. Read
/// the per-epoch errors with sidereon_reduced_orbit_drift_report_entries and the
/// aggregate with sidereon_reduced_orbit_drift_report_summary.
///
/// Safety: elements must point to a SidereonReducedOrbitElements; truth must
/// point to count SidereonEcefSample; out_report must point to storage for a
/// SidereonReducedOrbitDriftReport*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift(
    elements: *const SidereonReducedOrbitElements,
    truth: *const SidereonEcefSample,
    count: usize,
    scale: u32,
    threshold_m: f64,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_drift",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let elements = c_try!(require_ref(
                elements,
                "sidereon_reduced_orbit_drift",
                "elements"
            ));
            let raw = c_try!(require_slice(
                truth,
                count,
                "sidereon_reduced_orbit_drift",
                "truth"
            ));
            let scale = c_try!(time_scale_from_c_code(
                "sidereon_reduced_orbit_drift",
                "scale",
                scale
            ));
            let elements = c_try!(reduced_orbit_elements_from_c(
                "sidereon_reduced_orbit_drift",
                elements
            ));
            let truth: Vec<EcefSample> = raw.iter().map(ecef_sample_from_c).collect();
            let report = match reduced_orbit_drift_core(&elements, &truth, scale, threshold_m) {
                Ok(report) => report,
                Err(err) => return map_reduced_orbit_error("sidereon_reduced_orbit_drift", err),
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: report,
                    requested_samples: truth.len(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Evaluate reduced-orbit drift by sampling one satellite from an SP3 product.
/// Delegates to sidereon_core::orbit::drift_reduced_orbit_source. On success
/// writes a newly owned drift report handle.
///
/// Safety: elements must point to a SidereonReducedOrbitElements; sp3 must be a
/// live handle; sat_id must be a null-terminated satellite token; options must
/// point to SidereonReducedOrbitSourceDriftOptions; out_report must point to
/// storage for a SidereonReducedOrbitDriftReport*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_sp3_source(
    elements: *const SidereonReducedOrbitElements,
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonReducedOrbitSourceDriftOptions,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift_sp3_source",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_drift_sp3_source",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let elements = c_try!(require_ref(
                elements,
                "sidereon_reduced_orbit_drift_sp3_source",
                "elements"
            ));
            let elements = c_try!(reduced_orbit_elements_from_c(
                "sidereon_reduced_orbit_drift_sp3_source",
                elements
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_reduced_orbit_drift_sp3_source",
                "sp3"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_reduced_orbit_drift_sp3_source",
                sat_id
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_drift_sp3_source",
                "options"
            ));
            let options = reduced_orbit_source_drift_options_from_c(options);
            let source = ReducedOrbitSource::Sp3 {
                product: &sp3.inner,
                satellite: sat,
            };
            let drift = match reduced_orbit_drift_source_core(&elements, source, options) {
                Ok(drift) => drift,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_drift_sp3_source",
                        err,
                    )
                }
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: drift.report,
                    requested_samples: drift.requested_samples,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Evaluate reduced-orbit drift by sampling a TLE/SGP4 source in UTC. Delegates
/// to sidereon_core::orbit::drift_reduced_orbit_source. On success writes a newly
/// owned drift report handle.
///
/// Safety: elements must point to a SidereonReducedOrbitElements; tle must be a
/// live handle; options must point to SidereonReducedOrbitSourceDriftOptions;
/// out_report must point to storage for a SidereonReducedOrbitDriftReport*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_tle_source(
    elements: *const SidereonReducedOrbitElements,
    tle: *const SidereonTle,
    options: *const SidereonReducedOrbitSourceDriftOptions,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift_tle_source",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_drift_tle_source",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let elements = c_try!(require_ref(
                elements,
                "sidereon_reduced_orbit_drift_tle_source",
                "elements"
            ));
            let elements = c_try!(reduced_orbit_elements_from_c(
                "sidereon_reduced_orbit_drift_tle_source",
                elements
            ));
            let tle = c_try!(require_ref(
                tle,
                "sidereon_reduced_orbit_drift_tle_source",
                "tle"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_drift_tle_source",
                "options"
            ));
            let options = reduced_orbit_source_drift_options_from_c(options);
            let source = ReducedOrbitSource::Sgp4 {
                satellite: &tle.satellite,
            };
            let drift = match reduced_orbit_drift_source_core(&elements, source, options) {
                Ok(drift) => drift,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_drift_tle_source",
                        err,
                    )
                }
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: drift.report,
                    requested_samples: drift.requested_samples,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-epoch drift entries. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: report must be a live handle from sidereon_reduced_orbit_drift; out
/// must point to at least len writable SidereonReducedOrbitDriftEntry or be NULL
/// when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_report_entries(
    report: *const SidereonReducedOrbitDriftReport,
    out: *mut SidereonReducedOrbitDriftEntry,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift_report_entries",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_reduced_orbit_drift_report_entries",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_reduced_orbit_drift_report_entries",
                "report"
            ));
            let values: Vec<SidereonReducedOrbitDriftEntry> = report
                .inner
                .per_epoch
                .iter()
                .map(|entry: &DriftEntry| SidereonReducedOrbitDriftEntry {
                    epoch: calendar_epoch_to_c(entry.epoch),
                    error_m: entry.error_m,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_reduced_orbit_drift_report_entries",
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

/// Read the aggregate drift summary (max, RMS, and the index of the first
/// threshold-crossing entry if any).
///
/// Safety: report must be a live handle from sidereon_reduced_orbit_drift;
/// out_summary must point to a SidereonReducedOrbitDriftSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_report_summary(
    report: *const SidereonReducedOrbitDriftReport,
    out_summary: *mut SidereonReducedOrbitDriftSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift_report_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_reduced_orbit_drift_report_summary",
                "out_summary"
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_reduced_orbit_drift_report_summary",
                "report"
            ));
            let threshold_index = report.inner.threshold_index;
            *out_summary = SidereonReducedOrbitDriftSummary {
                max_m: report.inner.max_m,
                rms_m: report.inner.rms_m,
                has_threshold_crossing: threshold_index.is_some(),
                threshold_index: threshold_index.unwrap_or(0),
            };
            SidereonStatus::Ok
        },
    )
}

/// Write the number of samples requested from the source or input set that
/// produced this drift report.
///
/// Safety: report must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_report_requested_samples(
    report: *const SidereonReducedOrbitDriftReport,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_drift_report_requested_samples",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_reduced_orbit_drift_report_requested_samples",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_reduced_orbit_drift_report_requested_samples",
                "report"
            ));
            *out_count = report.requested_samples;
            SidereonStatus::Ok
        },
    )
}

/// Release a reduced-orbit drift report handle from sidereon_reduced_orbit_drift.
/// Passing NULL is a no-op.
///
/// Safety: report must be NULL or a live handle from sidereon_reduced_orbit_drift
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_drift_report_free(
    report: *mut SidereonReducedOrbitDriftReport,
) {
    ffi_boundary("sidereon_reduced_orbit_drift_report_free", (), || {
        free_boxed(report);
    });
}

/// A fitted piecewise reduced-orbit model. Opaque to C. Create with
/// sidereon_reduced_orbit_fit_piecewise and release with
/// sidereon_reduced_orbit_piecewise_free.
pub struct SidereonReducedOrbitPiecewise {
    pub(crate) inner: ReducedOrbitPiecewise,
    pub(crate) scale: TimeScale,
}

/// Summary metadata for a fitted piecewise reduced-orbit model.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitPiecewiseInfo {
    /// Model fitted in every segment (SidereonReducedOrbitModel as uint32_t).
    pub model: u32,
    /// Time scale used for fit and evaluation (SidereonTimeScale as uint32_t).
    pub scale: u32,
    /// Advertised coverage start.
    pub t0: SidereonCalendarEpoch,
    /// Advertised coverage end, inclusive on the final segment.
    pub t1: SidereonCalendarEpoch,
    /// Rounded segment length used to tile the window, seconds.
    pub segment_s: i64,
    /// Number of fitted segments.
    pub n_segments: usize,
}

/// Source-backed piecewise fit metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitPiecewiseSourceFitStats {
    /// Number of samples requested from the source before unavailable epochs
    /// were skipped.
    pub requested_samples: usize,
    /// Number of source samples used across all fitted segments.
    pub used_samples: usize,
}

/// One fitted piecewise segment and its reduced-orbit fit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonReducedOrbitPiecewiseSegment {
    /// Inclusive segment start.
    pub t0: SidereonCalendarEpoch,
    /// Segment end, exclusive except for the final segment.
    pub t1: SidereonCalendarEpoch,
    /// Fitted mean elements for this segment.
    pub elements: SidereonReducedOrbitElements,
    /// Fit residual statistics for this segment.
    pub stats: SidereonReducedOrbitFitStats,
}

/// Fit a piecewise reduced-orbit model over [t0, t1], tiled by segment_s
/// seconds. Delegates to sidereon_core::orbit::fit_piecewise. The returned
/// handle stores the fitted time scale for later evaluation.
///
/// Safety: samples must point to count SidereonEcefSample; t0 and t1 must point
/// to SidereonCalendarEpoch values; out must point to storage for a
/// SidereonReducedOrbitPiecewise*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit_piecewise(
    samples: *const SidereonEcefSample,
    count: usize,
    scale: u32,
    model: u32,
    t0: *const SidereonCalendarEpoch,
    t1: *const SidereonCalendarEpoch,
    segment_s: i64,
    out: *mut *mut SidereonReducedOrbitPiecewise,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_fit_piecewise",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_reduced_orbit_fit_piecewise",
                "out"
            ));
            *out = ptr::null_mut();
            let raw = c_try!(require_slice(
                samples,
                count,
                "sidereon_reduced_orbit_fit_piecewise",
                "samples"
            ));
            let scale = c_try!(time_scale_from_c_code(
                "sidereon_reduced_orbit_fit_piecewise",
                "scale",
                scale
            ));
            let model = c_try!(reduced_orbit_model_from_c(
                "sidereon_reduced_orbit_fit_piecewise",
                "model",
                model
            ));
            let t0 = c_try!(require_ref(
                t0,
                "sidereon_reduced_orbit_fit_piecewise",
                "t0"
            ));
            let t1 = c_try!(require_ref(
                t1,
                "sidereon_reduced_orbit_fit_piecewise",
                "t1"
            ));
            let samples: Vec<EcefSample> = raw.iter().map(ecef_sample_from_c).collect();
            let inner = match reduced_orbit_fit_piecewise_core(
                &samples,
                scale,
                model,
                calendar_epoch_from_c(t0),
                calendar_epoch_from_c(t1),
                segment_s,
            ) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_piecewise_orbit_error("sidereon_reduced_orbit_fit_piecewise", err)
                }
            };
            write_boxed_handle(out, SidereonReducedOrbitPiecewise { inner, scale });
            SidereonStatus::Ok
        },
    )
}

/// Read piecewise model metadata.
///
/// Safety: piecewise must be a live handle; out_info must point to a
/// SidereonReducedOrbitPiecewiseInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_info(
    piecewise: *const SidereonReducedOrbitPiecewise,
    out_info: *mut SidereonReducedOrbitPiecewiseInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_info",
        SidereonStatus::Panic,
        || {
            let out_info = c_try!(require_out(
                out_info,
                "sidereon_reduced_orbit_piecewise_info",
                "out_info"
            ));
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_info",
                "piecewise"
            ));
            *out_info = reduced_orbit_piecewise_info_to_c(piecewise);
            SidereonStatus::Ok
        },
    )
}

/// Copy fitted piecewise segment metadata. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: piecewise must be a live handle; out must point to len
/// SidereonReducedOrbitPiecewiseSegment entries or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_segments(
    piecewise: *const SidereonReducedOrbitPiecewise,
    out: *mut SidereonReducedOrbitPiecewiseSegment,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_segments",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_reduced_orbit_piecewise_segments",
                out_written,
                out_required
            ));
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_segments",
                "piecewise"
            ));
            let segments: Vec<SidereonReducedOrbitPiecewiseSegment> = piecewise
                .inner
                .segments
                .iter()
                .map(reduced_orbit_piecewise_segment_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_reduced_orbit_piecewise_segments",
                "out",
                &segments,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Select the segment covering epoch. Interior boundaries resolve to the later
/// segment; the exact end of the final segment resolves to that final segment.
///
/// Safety: piecewise and epoch must be live pointers; out_index and out_segment
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_select_segment(
    piecewise: *const SidereonReducedOrbitPiecewise,
    epoch: *const SidereonCalendarEpoch,
    out_index: *mut usize,
    out_segment: *mut SidereonReducedOrbitPiecewiseSegment,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_select_segment",
        SidereonStatus::Panic,
        || {
            let out_index = c_try!(require_out(
                out_index,
                "sidereon_reduced_orbit_piecewise_select_segment",
                "out_index"
            ));
            *out_index = 0;
            let out_segment = c_try!(require_out(
                out_segment,
                "sidereon_reduced_orbit_piecewise_select_segment",
                "out_segment"
            ));
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_select_segment",
                "piecewise"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_reduced_orbit_piecewise_select_segment",
                "epoch"
            ));
            let selected = match reduced_orbit_select_piecewise_segment_core(
                &piecewise.inner,
                calendar_epoch_from_c(epoch),
            ) {
                Ok(selected) => selected,
                Err(err) => {
                    return map_piecewise_orbit_error(
                        "sidereon_reduced_orbit_piecewise_select_segment",
                        err,
                    )
                }
            };
            let index = piecewise
                .inner
                .segments
                .iter()
                .position(|segment| std::ptr::eq(segment, selected))
                .unwrap_or(0);
            *out_index = index;
            *out_segment = reduced_orbit_piecewise_segment_to_c(selected);
            SidereonStatus::Ok
        },
    )
}

/// Evaluate a piecewise reduced-orbit position in the requested frame. Delegates
/// to sidereon_core::orbit::piecewise_position.
///
/// Safety: piecewise and epoch must be live pointers; out_xyz must point to len
/// writable doubles (len must be at least 3).
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_position(
    piecewise: *const SidereonReducedOrbitPiecewise,
    epoch: *const SidereonCalendarEpoch,
    frame: u32,
    out_xyz: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_position",
        SidereonStatus::Panic,
        || {
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_position",
                "piecewise"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_reduced_orbit_piecewise_position",
                "epoch"
            ));
            let frame = c_try!(reduced_orbit_frame_from_c(
                "sidereon_reduced_orbit_piecewise_position",
                "frame",
                frame
            ));
            let position = match reduced_orbit_piecewise_position_core(
                &piecewise.inner,
                calendar_epoch_from_c(epoch),
                piecewise.scale,
                frame,
            ) {
                Ok(position) => position,
                Err(err) => {
                    return map_piecewise_orbit_error(
                        "sidereon_reduced_orbit_piecewise_position",
                        err,
                    )
                }
            };
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_piecewise_position",
                "out_xyz",
                out_xyz,
                len,
                &position
            ));
            SidereonStatus::Ok
        },
    )
}

/// Evaluate piecewise reduced-orbit position and velocity in the requested
/// frame. Delegates to sidereon_core::orbit::piecewise_position_velocity.
///
/// Safety: piecewise and epoch must be live pointers; out_pos and out_vel must
/// each point to three writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_position_velocity(
    piecewise: *const SidereonReducedOrbitPiecewise,
    epoch: *const SidereonCalendarEpoch,
    frame: u32,
    out_pos: *mut f64,
    out_vel: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_position_velocity",
        SidereonStatus::Panic,
        || {
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_position_velocity",
                "piecewise"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_reduced_orbit_piecewise_position_velocity",
                "epoch"
            ));
            let frame = c_try!(reduced_orbit_frame_from_c(
                "sidereon_reduced_orbit_piecewise_position_velocity",
                "frame",
                frame
            ));
            let (position, velocity) = match reduced_orbit_piecewise_position_velocity_core(
                &piecewise.inner,
                calendar_epoch_from_c(epoch),
                piecewise.scale,
                frame,
            ) {
                Ok(pair) => pair,
                Err(err) => {
                    return map_piecewise_orbit_error(
                        "sidereon_reduced_orbit_piecewise_position_velocity",
                        err,
                    )
                }
            };
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_piecewise_position_velocity",
                "out_pos",
                out_pos,
                3,
                &position
            ));
            c_try!(copy_exact_f64s(
                "sidereon_reduced_orbit_piecewise_position_velocity",
                "out_vel",
                out_vel,
                3,
                &velocity
            ));
            SidereonStatus::Ok
        },
    )
}

/// Evaluate piecewise model-vs-truth drift. Truth samples outside model coverage
/// are skipped by the core. On success writes a drift report handle; release it
/// with sidereon_reduced_orbit_drift_report_free.
///
/// Safety: piecewise must be a live handle; truth must point to count
/// SidereonEcefSample; out_report must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_drift(
    piecewise: *const SidereonReducedOrbitPiecewise,
    truth: *const SidereonEcefSample,
    count: usize,
    threshold_m: f64,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_drift",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_piecewise_drift",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_drift",
                "piecewise"
            ));
            let raw = c_try!(require_slice(
                truth,
                count,
                "sidereon_reduced_orbit_piecewise_drift",
                "truth"
            ));
            let truth: Vec<EcefSample> = raw.iter().map(ecef_sample_from_c).collect();
            let report = match reduced_orbit_piecewise_drift_core(
                &piecewise.inner,
                &truth,
                piecewise.scale,
                threshold_m,
            ) {
                Ok(report) => report,
                Err(err) => {
                    return map_piecewise_orbit_error("sidereon_reduced_orbit_piecewise_drift", err)
                }
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: report,
                    requested_samples: truth.len(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Fit a piecewise reduced-orbit model by sampling one satellite from an SP3
/// product. Delegates to sidereon_core::orbit::fit_piecewise_reduced_orbit_source.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated
/// satellite token; options, out_piecewise, and out_stats must point to their
/// documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit_piecewise_sp3_source(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonReducedOrbitSourceFitOptions,
    segment_s: f64,
    out_piecewise: *mut *mut SidereonReducedOrbitPiecewise,
    out_stats: *mut SidereonReducedOrbitPiecewiseSourceFitStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_fit_piecewise_sp3_source",
        SidereonStatus::Panic,
        || {
            let out_piecewise = c_try!(require_out(
                out_piecewise,
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                "out_piecewise"
            ));
            *out_piecewise = ptr::null_mut();
            let out_stats = c_try!(require_out(
                out_stats,
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                "out_stats"
            ));
            *out_stats = SidereonReducedOrbitPiecewiseSourceFitStats {
                requested_samples: 0,
                used_samples: 0,
            };
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                "sp3"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                sat_id
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                "options"
            ));
            let options = c_try!(reduced_orbit_piecewise_source_fit_options_from_c(
                "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                options,
                segment_s,
            ));
            let source = ReducedOrbitSource::Sp3 {
                product: &sp3.inner,
                satellite: sat,
            };
            let fit = match reduced_orbit_piecewise_fit_source_core(source, options) {
                Ok(fit) => fit,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_fit_piecewise_sp3_source",
                        err,
                    )
                }
            };
            *out_stats = SidereonReducedOrbitPiecewiseSourceFitStats {
                requested_samples: fit.requested_samples,
                used_samples: reduced_orbit_piecewise_used_samples(&fit.orbit),
            };
            write_boxed_handle(
                out_piecewise,
                SidereonReducedOrbitPiecewise {
                    inner: fit.orbit,
                    scale: sp3.inner.header.time_scale,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Fit a piecewise reduced-orbit model by sampling a TLE/SGP4 source in UTC.
/// Delegates to sidereon_core::orbit::fit_piecewise_reduced_orbit_source.
///
/// Safety: tle must be a live handle; options, out_piecewise, and out_stats
/// must point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_fit_piecewise_tle_source(
    tle: *const SidereonTle,
    options: *const SidereonReducedOrbitSourceFitOptions,
    segment_s: f64,
    out_piecewise: *mut *mut SidereonReducedOrbitPiecewise,
    out_stats: *mut SidereonReducedOrbitPiecewiseSourceFitStats,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_fit_piecewise_tle_source",
        SidereonStatus::Panic,
        || {
            let out_piecewise = c_try!(require_out(
                out_piecewise,
                "sidereon_reduced_orbit_fit_piecewise_tle_source",
                "out_piecewise"
            ));
            *out_piecewise = ptr::null_mut();
            let out_stats = c_try!(require_out(
                out_stats,
                "sidereon_reduced_orbit_fit_piecewise_tle_source",
                "out_stats"
            ));
            *out_stats = SidereonReducedOrbitPiecewiseSourceFitStats {
                requested_samples: 0,
                used_samples: 0,
            };
            let tle = c_try!(require_ref(
                tle,
                "sidereon_reduced_orbit_fit_piecewise_tle_source",
                "tle"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_fit_piecewise_tle_source",
                "options"
            ));
            let options = c_try!(reduced_orbit_piecewise_source_fit_options_from_c(
                "sidereon_reduced_orbit_fit_piecewise_tle_source",
                options,
                segment_s,
            ));
            let source = ReducedOrbitSource::Sgp4 {
                satellite: &tle.satellite,
            };
            let fit = match reduced_orbit_piecewise_fit_source_core(source, options) {
                Ok(fit) => fit,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_fit_piecewise_tle_source",
                        err,
                    )
                }
            };
            *out_stats = SidereonReducedOrbitPiecewiseSourceFitStats {
                requested_samples: fit.requested_samples,
                used_samples: reduced_orbit_piecewise_used_samples(&fit.orbit),
            };
            write_boxed_handle(
                out_piecewise,
                SidereonReducedOrbitPiecewise {
                    inner: fit.orbit,
                    scale: TimeScale::Utc,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Evaluate a piecewise model's drift by sampling one satellite from an SP3
/// product. Delegates to
/// sidereon_core::orbit::drift_piecewise_reduced_orbit_source.
///
/// Safety: piecewise and sp3 must be live handles; sat_id must be a
/// null-terminated satellite token; options and out_report must point to their
/// documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_drift_sp3_source(
    piecewise: *const SidereonReducedOrbitPiecewise,
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonReducedOrbitSourceDriftOptions,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_drift_sp3_source",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                "piecewise"
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                "sp3"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                sat_id
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                "options"
            ));
            let options = reduced_orbit_source_drift_options_from_c(options);
            let source = ReducedOrbitSource::Sp3 {
                product: &sp3.inner,
                satellite: sat,
            };
            let drift = match reduced_orbit_piecewise_drift_source_core(
                &piecewise.inner,
                source,
                options,
            ) {
                Ok(drift) => drift,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_piecewise_drift_sp3_source",
                        err,
                    )
                }
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: drift.report,
                    requested_samples: drift.requested_samples,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Evaluate a piecewise model's drift by sampling a TLE/SGP4 source in UTC.
/// Delegates to sidereon_core::orbit::drift_piecewise_reduced_orbit_source.
///
/// Safety: piecewise and tle must be live handles; options and out_report must
/// point to their documented storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_drift_tle_source(
    piecewise: *const SidereonReducedOrbitPiecewise,
    tle: *const SidereonTle,
    options: *const SidereonReducedOrbitSourceDriftOptions,
    out_report: *mut *mut SidereonReducedOrbitDriftReport,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_reduced_orbit_piecewise_drift_tle_source",
        SidereonStatus::Panic,
        || {
            let out_report = c_try!(require_out(
                out_report,
                "sidereon_reduced_orbit_piecewise_drift_tle_source",
                "out_report"
            ));
            *out_report = ptr::null_mut();
            let piecewise = c_try!(require_ref(
                piecewise,
                "sidereon_reduced_orbit_piecewise_drift_tle_source",
                "piecewise"
            ));
            let tle = c_try!(require_ref(
                tle,
                "sidereon_reduced_orbit_piecewise_drift_tle_source",
                "tle"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_reduced_orbit_piecewise_drift_tle_source",
                "options"
            ));
            let options = reduced_orbit_source_drift_options_from_c(options);
            let source = ReducedOrbitSource::Sgp4 {
                satellite: &tle.satellite,
            };
            let drift = match reduced_orbit_piecewise_drift_source_core(
                &piecewise.inner,
                source,
                options,
            ) {
                Ok(drift) => drift,
                Err(err) => {
                    return map_reduced_orbit_source_error(
                        "sidereon_reduced_orbit_piecewise_drift_tle_source",
                        err,
                    )
                }
            };
            write_boxed_handle(
                out_report,
                SidereonReducedOrbitDriftReport {
                    inner: drift.report,
                    requested_samples: drift.requested_samples,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a piecewise reduced-orbit handle. Passing NULL is a no-op.
///
/// Safety: piecewise must be NULL or a live handle from
/// sidereon_reduced_orbit_fit_piecewise that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_reduced_orbit_piecewise_free(
    piecewise: *mut SidereonReducedOrbitPiecewise,
) {
    ffi_boundary("sidereon_reduced_orbit_piecewise_free", (), || {
        free_boxed(piecewise);
    });
}

// ===========================================================================
// NRLMSISE-00 neutral-atmosphere density. Delegates to
// sidereon_core::astro::atmosphere::nrlmsise00_with_lst.

fn map_reduced_orbit_source_error(fn_name: &str, err: ReducedOrbitSourceError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        ReducedOrbitSourceError::InvalidWindow
        | ReducedOrbitSourceError::InvalidCadence
        | ReducedOrbitSourceError::InvalidSegment
        | ReducedOrbitSourceError::Reduced(ReducedOrbitError::InvalidInput { .. }) => {
            SidereonStatus::InvalidArgument
        }
        _ => SidereonStatus::Solve,
    }
}

fn reduced_orbit_source_fit_options_from_c(
    fn_name: &str,
    options: &SidereonReducedOrbitSourceFitOptions,
) -> Result<ReducedOrbitSourceFitOptions, SidereonStatus> {
    Ok(ReducedOrbitSourceFitOptions {
        sampling: reduced_orbit_source_sampling_from_c(&options.sampling),
        model: reduced_orbit_model_from_c(fn_name, "options.model", options.model)?,
    })
}

fn reduced_orbit_source_drift_options_from_c(
    options: &SidereonReducedOrbitSourceDriftOptions,
) -> ReducedOrbitSourceDriftOptions {
    ReducedOrbitSourceDriftOptions {
        sampling: reduced_orbit_source_sampling_from_c(&options.sampling),
        threshold_m: options.threshold_m,
    }
}

fn reduced_orbit_piecewise_source_fit_options_from_c(
    fn_name: &str,
    options: &SidereonReducedOrbitSourceFitOptions,
    segment_s: f64,
) -> Result<PiecewiseOrbitSourceFitOptions, SidereonStatus> {
    Ok(PiecewiseOrbitSourceFitOptions {
        sampling: reduced_orbit_source_sampling_from_c(&options.sampling),
        model: reduced_orbit_model_from_c(fn_name, "options.model", options.model)?,
        segment_s,
    })
}

fn reduced_orbit_piecewise_used_samples(piecewise: &ReducedOrbitPiecewise) -> usize {
    piecewise
        .segments
        .iter()
        .map(|segment| segment.orbit.stats.n_samples)
        .sum()
}

fn reduced_orbit_frame_from_c(
    fn_name: &str,
    arg_name: &str,
    frame: u32,
) -> Result<ReducedOrbitFrameInner, SidereonStatus> {
    match frame {
        value if value == SidereonReducedOrbitFrame::Gcrs as u32 => {
            Ok(ReducedOrbitFrameInner::Gcrs)
        }
        value if value == SidereonReducedOrbitFrame::Ecef as u32 => {
            Ok(ReducedOrbitFrameInner::Ecef)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} reduced-orbit frame"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn reduced_orbit_elements_from_c(
    fn_name: &str,
    elements: &SidereonReducedOrbitElements,
) -> Result<ReducedOrbitElements, SidereonStatus> {
    let model = reduced_orbit_model_from_c(fn_name, "elements.model", elements.model)?;
    Ok(ReducedOrbitElements {
        model,
        epoch: calendar_epoch_from_c(&elements.epoch),
        a_m: elements.a_m,
        e: elements.e,
        i_rad: elements.i_rad,
        raan_rad: elements.raan_rad,
        raan_rate_rad_s: elements.raan_rate_rad_s,
        raan_rate_j2_rad_s: elements.raan_rate_j2_rad_s,
        arg_lat_rad: elements.arg_lat_rad,
        mean_motion_rad_s: elements.mean_motion_rad_s,
        h: elements.h,
        k: elements.k,
        arg_perigee_rad: elements.arg_perigee_rad,
    })
}

fn ecef_sample_from_c(sample: &SidereonEcefSample) -> EcefSample {
    EcefSample::new(
        calendar_epoch_from_c(&sample.epoch),
        sample.x_m,
        sample.y_m,
        sample.z_m,
    )
}

fn map_piecewise_orbit_error(fn_name: &str, err: PiecewiseOrbitError) -> SidereonStatus {
    match err {
        PiecewiseOrbitError::InvalidSegment => {
            set_last_error(format!(
                "{fn_name}: piecewise segment length is missing, non-positive, or rounds below one second"
            ));
            SidereonStatus::InvalidArgument
        }
        PiecewiseOrbitError::OutOfRange => {
            set_last_error(format!(
                "{fn_name}: query epoch is outside the piecewise model coverage"
            ));
            SidereonStatus::InvalidArgument
        }
        PiecewiseOrbitError::TooFewSamples { got, required } => {
            set_last_error(format!(
                "{fn_name}: piecewise fit needs at least {required} samples, got {got}"
            ));
            SidereonStatus::Solve
        }
        PiecewiseOrbitError::Reduced(inner) => map_reduced_orbit_error(fn_name, inner),
    }
}

fn reduced_orbit_piecewise_info_to_c(
    piecewise: &SidereonReducedOrbitPiecewise,
) -> SidereonReducedOrbitPiecewiseInfo {
    SidereonReducedOrbitPiecewiseInfo {
        model: reduced_orbit_model_to_c(piecewise.inner.model),
        scale: time_scale_to_c_code(piecewise.scale),
        t0: calendar_epoch_to_c(piecewise.inner.t0),
        t1: calendar_epoch_to_c(piecewise.inner.t1),
        segment_s: piecewise.inner.segment_s,
        n_segments: piecewise.inner.segments.len(),
    }
}

fn reduced_orbit_piecewise_segment_to_c(
    segment: &sidereon_core::orbit::PiecewiseSegment,
) -> SidereonReducedOrbitPiecewiseSegment {
    SidereonReducedOrbitPiecewiseSegment {
        t0: calendar_epoch_to_c(segment.t0),
        t1: calendar_epoch_to_c(segment.t1),
        elements: reduced_orbit_elements_to_c(&segment.orbit.elements),
        stats: SidereonReducedOrbitFitStats {
            rms_m: segment.orbit.stats.rms_m,
            max_m: segment.orbit.stats.max_m,
            n_samples: segment.orbit.stats.n_samples,
        },
    }
}

fn map_reduced_orbit_error(fn_name: &str, err: ReducedOrbitError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        ReducedOrbitError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn reduced_orbit_source_sampling_from_c(
    sampling: &SidereonReducedOrbitSourceSampling,
) -> ReducedOrbitSourceSampling {
    ReducedOrbitSourceSampling::new(
        calendar_epoch_from_c(&sampling.t0),
        calendar_epoch_from_c(&sampling.t1),
        sampling.cadence_s,
    )
}

fn reduced_orbit_model_from_c(
    fn_name: &str,
    arg_name: &str,
    model: u32,
) -> Result<ReducedOrbitModelInner, SidereonStatus> {
    match model {
        value if value == SidereonReducedOrbitModel::CircularSecular as u32 => {
            Ok(ReducedOrbitModelInner::CircularSecular)
        }
        value if value == SidereonReducedOrbitModel::EccentricSecular as u32 => {
            Ok(ReducedOrbitModelInner::EccentricSecular)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} reduced-orbit model"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn reduced_orbit_elements_to_c(elements: &ReducedOrbitElements) -> SidereonReducedOrbitElements {
    SidereonReducedOrbitElements {
        model: reduced_orbit_model_to_c(elements.model),
        epoch: calendar_epoch_to_c(elements.epoch),
        a_m: elements.a_m,
        e: elements.e,
        i_rad: elements.i_rad,
        raan_rad: elements.raan_rad,
        raan_rate_rad_s: elements.raan_rate_rad_s,
        raan_rate_j2_rad_s: elements.raan_rate_j2_rad_s,
        arg_lat_rad: elements.arg_lat_rad,
        mean_motion_rad_s: elements.mean_motion_rad_s,
        h: elements.h,
        k: elements.k,
        arg_perigee_rad: elements.arg_perigee_rad,
    }
}
