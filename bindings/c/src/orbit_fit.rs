use super::*;
use crate::engine_error::{
    degrade_reason_name, engine_error_operation_boundary, engine_f64, record_engine_error,
    SidereonEngineErrorFamily,
};
use crate::orbit::rtn_frame_error_value;
use crate::reduced::solve_error_value;
use serde_json::{json, Value};

fn orbit_fit_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

fn observability_tier_name(
    tier: sidereon_core::geometry_quality::ObservabilityTier,
) -> &'static str {
    use sidereon_core::geometry_quality::ObservabilityTier as T;
    match tier {
        T::RankDeficient => "rank_deficient",
        T::ZeroRedundancy => "zero_redundancy",
        T::Weak => "weak",
        T::Nominal => "nominal",
    }
}

fn validity_mode_name(mode: sidereon_core::astro::time::ValidityMode) -> &'static str {
    match mode {
        sidereon_core::astro::time::ValidityMode::Strict => "strict",
        sidereon_core::astro::time::ValidityMode::Permissive => "permissive",
    }
}

fn ut1_provider_role_name(role: sidereon_core::ephemeris::Ut1ProviderRole) -> &'static str {
    match role {
        sidereon_core::ephemeris::Ut1ProviderRole::Orientation => "orientation",
        sidereon_core::ephemeris::Ut1ProviderRole::Propagation => "propagation",
    }
}

pub(crate) fn geometry_quality_value(
    quality: &sidereon_core::geometry_quality::GeometryQuality,
) -> Value {
    json!({
        "tier": observability_tier_name(quality.tier),
        "redundancy": quality.redundancy,
        "rank": quality.rank,
        "condition_number": engine_f64(quality.condition_number),
        "gdop": engine_f64(quality.gdop),
        "raim_checkable": quality.raim_checkable,
        "covariance_validated": quality.covariance_validated,
    })
}

pub(crate) fn frame_transform_error_value(
    error: &sidereon_core::astro::frames::transforms::FrameTransformError,
) -> Value {
    use sidereon_core::astro::frames::transforms::FrameTransformError as E;
    match error {
        E::InvalidInput { field, reason } => orbit_fit_node(
            "invalid_input",
            json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::Ut1OutsideCoverage { reason } => orbit_fit_node(
            "ut1_outside_coverage",
            json!({
                "reason": degrade_reason_name(*reason),
            }),
        ),
    }
}

pub(crate) fn propagation_error_value(
    error: &sidereon_core::astro::error::PropagationError,
) -> Value {
    use sidereon_core::astro::error::PropagationError as E;
    match error {
        E::InvalidInput(message) => orbit_fit_node(
            "invalid_input",
            json!({
                "message": message,
            }),
        ),
        E::NumericalFailure(message) => orbit_fit_node(
            "numerical_failure",
            json!({
                "message": message,
            }),
        ),
        E::MaxStepsExceeded => orbit_fit_node("max_steps_exceeded", json!({})),
        E::EventFailure(message) => orbit_fit_node(
            "event_failure",
            json!({
                "message": message,
            }),
        ),
        E::ForceModelFailure(message) => orbit_fit_node(
            "force_model_failure",
            json!({
                "message": message,
            }),
        ),
        E::Ut1OutsideCoverage(reason) => orbit_fit_node(
            "ut1_outside_coverage",
            json!({
                "reason": degrade_reason_name(*reason),
            }),
        ),
    }
}

pub(crate) fn orbit_fit_error_value(error: &sidereon_core::ephemeris::OrbitFitError) -> Value {
    use sidereon_core::ephemeris::OrbitFitError as E;
    match error {
        E::EmptySelection => orbit_fit_node("empty_selection", json!({})),
        E::InvalidOption { field, reason } => orbit_fit_node(
            "invalid_option",
            json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::TooFewSamples {
            satellite,
            got,
            required,
        } => orbit_fit_node(
            "too_few_samples",
            json!({
                "satellite": satellite.to_string(),
                "got": got,
                "required": required,
            }),
        ),
        E::NonMonotonicEpochs { satellite } => orbit_fit_node(
            "non_monotonic_epochs",
            json!({
                "satellite": satellite.to_string(),
            }),
        ),
        E::MixedTimeScales => orbit_fit_node("mixed_time_scales", json!({})),
        E::InvalidEpoch { satellite, reason } => orbit_fit_node(
            "invalid_epoch",
            json!({
                "satellite": satellite.to_string(),
                "reason": reason,
            }),
        ),
        E::InvalidObservation { satellite, reason } => orbit_fit_node(
            "invalid_observation",
            json!({
                "satellite": satellite.to_string(),
                "reason": reason,
            }),
        ),
        E::Frame { satellite, source } => orbit_fit_node(
            "frame",
            json!({
                "satellite": satellite.to_string(),
                "cause": frame_transform_error_value(source),
            }),
        ),
        E::Propagation { satellite, source } => orbit_fit_node(
            "propagation",
            json!({
                "satellite": satellite.to_string(),
                "cause": propagation_error_value(source),
            }),
        ),
        E::LeastSquares { satellite, source } => orbit_fit_node(
            "least_squares",
            json!({
                "satellite": satellite.to_string(),
                "cause": solve_error_value(source),
            }),
        ),
        E::SingularGeometry {
            satellite,
            geometry_quality,
        } => orbit_fit_node(
            "singular_geometry",
            json!({
                "satellite": satellite.to_string(),
                "geometry_quality": geometry_quality_value(geometry_quality),
            }),
        ),
        E::DidNotConverge {
            satellite,
            iterations,
        } => orbit_fit_node(
            "did_not_converge",
            json!({
                "satellite": satellite.to_string(),
                "iterations": iterations,
            }),
        ),
        E::Ut1OutsideCoverage(reason) => orbit_fit_node(
            "ut1_outside_coverage",
            json!({
                "reason": degrade_reason_name(*reason),
            }),
        ),
        E::Ut1ValidityMismatch {
            fit,
            provider,
            provider_mode,
        } => orbit_fit_node(
            "ut1_validity_mismatch",
            json!({
                "fit": validity_mode_name(*fit),
                "provider": ut1_provider_role_name(*provider),
                "provider_mode": validity_mode_name(*provider_mode),
            }),
        ),
        E::RtnFrame { satellite, reason } => orbit_fit_node(
            "rtn_frame",
            json!({
                "satellite": satellite.to_string(),
                "cause": rtn_frame_error_value(reason),
            }),
        ),
    }
}

/// Covariance state for a fitted orbit.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonOrbitFitCovarianceKind {
    /// A finite covariance matrix is present.
    Estimated = 0,
    /// The arc has no positive residual degrees of freedom.
    Unbounded = 1,
}

/// Fitted initial-state covariance for [x, y, z, vx, vy, vz].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitFitCovariance {
    /// Covariance tag, as SidereonOrbitFitCovarianceKind.
    pub kind: u32,
    /// Row-major 6x6 covariance when kind is Estimated.
    pub matrix: [f64; 36],
}

/// Initial-state fit result for one satellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitFitSolution {
    /// Satellite fitted by this solution.
    pub satellite: SidereonSatelliteToken,
    /// Estimated inertial initial state.
    pub initial_state: SidereonCartesianState,
    /// Fitted state covariance tag and matrix.
    pub covariance: SidereonOrbitFitCovariance,
    /// Singular-value geometry diagnostics.
    pub geometry_quality: SidereonGeometryQuality,
    /// Three-dimensional RMS residual of the seeded state, meters.
    pub seed_rms_3d_m: f64,
    /// Three-dimensional RMS residual of the fitted state, meters.
    pub fit_rms_3d_m: f64,
    /// Accepted nonlinear least-squares iterations.
    pub iterations: usize,
}

/// Arc span covered by a residual ledger.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitArcSpan {
    /// Time scale shared by residual epochs, as SidereonTimeScale.
    pub time_scale: u32,
    /// First residual epoch, seconds since J2000 in time_scale.
    pub start_j2000_s: f64,
    /// Last residual epoch, seconds since J2000 in time_scale.
    pub end_j2000_s: f64,
    /// Arc duration, seconds.
    pub duration_s: f64,
}

/// RTN residual RMS summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitResidualStats {
    /// Radial RMS residual, meters.
    pub radial_rms_m: f64,
    /// Along-track RMS residual, meters.
    pub along_rms_m: f64,
    /// Cross-track RMS residual, meters.
    pub cross_rms_m: f64,
    /// Three-dimensional RMS residual, meters.
    pub rms_3d_m: f64,
    /// Number of residual epochs.
    pub n: usize,
    /// Whether n is below the configured ledger minimum.
    pub low_sample_count: bool,
}

/// Per-satellite residual ledger entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitSatelliteResidualEntry {
    /// Satellite for this ledger entry.
    pub satellite: SidereonSatelliteToken,
    /// Residual statistics.
    pub stats: SidereonOrbitResidualStats,
}

/// Per-constellation residual ledger entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitConstellationResidualEntry {
    /// GNSS system for this ledger entry.
    pub system: u32,
    /// Residual statistics.
    pub stats: SidereonOrbitResidualStats,
}

/// Options controlling a precise-orbit fit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitFitOptions {
    /// Force model selector, as SidereonPropagationForceModel.
    pub force_model: u32,
    /// Additive force components for Composite, and optional SRP for EarthPhaseA.
    pub force_components: SidereonForceModelComponents,
    /// Whether mu_km3_s2 overrides the engine default in legacy and composite paths.
    pub mu_km3_s2_enabled: bool,
    /// Gravitational parameter in km^3/s^2 when enabled.
    pub mu_km3_s2: f64,
    /// Integrator selector, as SidereonPropagationIntegrator.
    pub integrator: u32,
    /// Absolute propagation tolerance.
    pub abs_tol: f64,
    /// Relative propagation tolerance.
    pub rel_tol: f64,
    /// Initial integration step, seconds.
    pub initial_step_s: f64,
    /// Minimum integration step, seconds.
    pub min_step_s: f64,
    /// Maximum integration step, seconds.
    pub max_step_s: f64,
    /// Maximum integration steps.
    pub max_steps: u32,
    /// Nonlinear solve gradient tolerance.
    pub solver_gtol: f64,
    /// Nonlinear solve cost tolerance.
    pub solver_ftol: f64,
    /// Nonlinear solve step tolerance.
    pub solver_xtol: f64,
    /// Maximum nonlinear residual evaluations.
    pub solver_max_nfev: usize,
    /// Minimum residual count before a ledger entry is not marked low-n.
    pub min_ledger_samples: usize,
    /// Whether drag is layered on the selected force model.
    pub has_drag: bool,
    /// Drag parameters when has_drag is true.
    pub drag: SidereonDragParameters,
}

/// Precise-orbit fit report. Opaque to C. Create with
/// sidereon_fit_sp3_precise_orbit or sidereon_fit_precise_ephemeris_sample_orbit
/// and release with sidereon_orbit_fit_report_free.
pub struct SidereonOrbitFitReport {
    pub(crate) inner: sidereon_core::ephemeris::OrbitFitReport,
}

/// Initialize orbit-fit options with core defaults.
///
/// Safety: out_options must point to SidereonOrbitFitOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_options_init(
    out_options: *mut SidereonOrbitFitOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orbit_fit_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_orbit_fit_options_init",
                "out_options"
            ));
            let defaults = sidereon_core::ephemeris::OrbitFitOptions::default();
            let drag = DragParameters::from_bc_factor_m2_kg(
                0.01,
                SpaceWeather::default(),
                DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
            )
            .expect("default orbit-fit drag parameters are valid");
            *out = SidereonOrbitFitOptions {
                force_model: SidereonPropagationForceModel::EarthPhaseA as u32,
                force_components: orbit_fit_default_force_components(),
                mu_km3_s2_enabled: false,
                mu_km3_s2: MU_EARTH,
                integrator: match defaults.integrator {
                    IntegratorKind::Dp54 => SidereonPropagationIntegrator::Dp54 as u32,
                    IntegratorKind::Rk4 => SidereonPropagationIntegrator::Rk4 as u32,
                },
                abs_tol: defaults.integrator_options.abs_tol,
                rel_tol: defaults.integrator_options.rel_tol,
                initial_step_s: defaults.integrator_options.initial_step,
                min_step_s: defaults.integrator_options.min_step,
                max_step_s: defaults.integrator_options.max_step,
                max_steps: defaults.integrator_options.max_steps,
                solver_gtol: defaults.solver_options.gtol,
                solver_ftol: defaults.solver_options.ftol,
                solver_xtol: defaults.solver_options.xtol,
                solver_max_nfev: defaults.solver_options.max_nfev,
                min_ledger_samples: defaults.min_ledger_samples,
                has_drag: false,
                drag: drag_parameters_to_c(drag),
            };
            SidereonStatus::Ok
        },
    )
}

/// Fit one satellite from a parsed SP3 product. On success writes a report
/// handle to *out_report.
///
/// Safety: sp3 must be live; sat_id must be a null-terminated satellite token;
/// options may be NULL for defaults; out_report must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_sp3_precise_orbit(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonOrbitFitOptions,
    out_report: *mut *mut SidereonOrbitFitReport,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_fit_sp3_precise_orbit",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_report,
                "sidereon_fit_sp3_precise_orbit",
                "out_report"
            ));
            *out = ptr::null_mut();
            let sp3 = c_try!(require_ref(sp3, "sidereon_fit_sp3_precise_orbit", "sp3"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_fit_sp3_precise_orbit",
                sat_id,
            ));
            let options = c_try!(orbit_fit_options_from_c(
                "sidereon_fit_sp3_precise_orbit",
                options,
            ));
            match sidereon_core::ephemeris::fit_sp3_precise_orbit(&sp3.inner, sat, &options) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonOrbitFitReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_orbit_fit_error("sidereon_fit_sp3_precise_orbit", err),
            }
        },
    )
}

/// Fit one satellite from a parsed ECEF SP3 product. On success writes a report
/// handle to *out_report.
///
/// The Earth-orientation provider is constructed internally with zero polar
/// motion.
///
/// Safety: sp3 must be live; sat_id must be a null-terminated satellite token;
/// options may be NULL for defaults; out_report must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_sp3_ecef_precise_orbit(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    options: *const SidereonOrbitFitOptions,
    out_report: *mut *mut SidereonOrbitFitReport,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_fit_sp3_ecef_precise_orbit",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_report,
                "sidereon_fit_sp3_ecef_precise_orbit",
                "out_report"
            ));
            *out = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fit_sp3_ecef_precise_orbit",
                "sp3"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_fit_sp3_ecef_precise_orbit",
                sat_id,
            ));
            let options = c_try!(orbit_fit_options_from_c(
                "sidereon_fit_sp3_ecef_precise_orbit",
                options,
            ));
            let orientation = TdbEarthOrientationProvider::new();
            match sidereon_core::ephemeris::fit_sp3_ecef_precise_orbit(
                &sp3.inner,
                sat,
                &orientation,
                &options,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonOrbitFitReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_orbit_fit_error("sidereon_fit_sp3_ecef_precise_orbit", err),
            }
        },
    )
}

/// Fit selected satellites from a parsed ECEF SP3 product.
///
/// Safety: sp3 must be live; satellites must point to satellite_count fixed
/// tokens; options may be NULL; out_report must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_sp3_ecef_precise_orbits(
    sp3: *const SidereonSp3,
    satellites: *const SidereonSatelliteToken,
    satellite_count: usize,
    options: *const SidereonOrbitFitOptions,
    out_report: *mut *mut SidereonOrbitFitReport,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_fit_sp3_ecef_precise_orbits",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_report,
                "sidereon_fit_sp3_ecef_precise_orbits",
                "out_report"
            ));
            *out = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fit_sp3_ecef_precise_orbits",
                "sp3"
            ));
            let satellites = c_try!(satellite_tokens_from_c(
                "sidereon_fit_sp3_ecef_precise_orbits",
                satellites,
                satellite_count
            ));
            let options = c_try!(orbit_fit_options_from_c(
                "sidereon_fit_sp3_ecef_precise_orbits",
                options,
            ));
            let orientation = TdbEarthOrientationProvider::new();
            match sidereon_core::ephemeris::fit_sp3_ecef_precise_orbits(
                &sp3.inner,
                &satellites,
                &orientation,
                &options,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonOrbitFitReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_orbit_fit_error("sidereon_fit_sp3_ecef_precise_orbits", err),
            }
        },
    )
}

/// Fit every satellite from a parsed ECEF SP3 product.
///
/// Safety: sp3 must be live; options may be NULL; out_report must point to
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_all_sp3_ecef_precise_orbits(
    sp3: *const SidereonSp3,
    options: *const SidereonOrbitFitOptions,
    out_report: *mut *mut SidereonOrbitFitReport,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_fit_all_sp3_ecef_precise_orbits",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_report,
                "sidereon_fit_all_sp3_ecef_precise_orbits",
                "out_report"
            ));
            *out = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_fit_all_sp3_ecef_precise_orbits",
                "sp3"
            ));
            let options = c_try!(orbit_fit_options_from_c(
                "sidereon_fit_all_sp3_ecef_precise_orbits",
                options,
            ));
            let orientation = TdbEarthOrientationProvider::new();
            match sidereon_core::ephemeris::fit_all_sp3_ecef_precise_orbits(
                &sp3.inner,
                &orientation,
                &options,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonOrbitFitReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_orbit_fit_error("sidereon_fit_all_sp3_ecef_precise_orbits", err),
            }
        },
    )
}

/// Fit one satellite from canonical precise-ephemeris samples. On success
/// writes a report handle to *out_report.
///
/// Safety: samples points to count sample structs; sat_id must be a
/// null-terminated satellite token; options may be NULL; out_report must point
/// to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_precise_ephemeris_sample_orbit(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    sat_id: *const c_char,
    options: *const SidereonOrbitFitOptions,
    out_report: *mut *mut SidereonOrbitFitReport,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_fit_precise_ephemeris_sample_orbit",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_report,
                "sidereon_fit_precise_ephemeris_sample_orbit",
                "out_report"
            ));
            *out = ptr::null_mut();
            let raw = c_try!(require_slice(
                samples,
                count,
                "sidereon_fit_precise_ephemeris_sample_orbit",
                "samples"
            ));
            let mut parsed = Vec::with_capacity(raw.len());
            for sample in raw {
                parsed.push(c_try!(crate::precise::precise_sample_from_c(
                    "sidereon_fit_precise_ephemeris_sample_orbit",
                    sample,
                )));
            }
            let sat = c_try!(parse_satellite_token(
                "sidereon_fit_precise_ephemeris_sample_orbit",
                sat_id,
            ));
            let options = c_try!(orbit_fit_options_from_c(
                "sidereon_fit_precise_ephemeris_sample_orbit",
                options,
            ));
            match sidereon_core::ephemeris::fit_precise_ephemeris_sample_orbit(
                &parsed, sat, &options,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonOrbitFitReport { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_orbit_fit_error("sidereon_fit_precise_ephemeris_sample_orbit", err),
            }
        },
    )
}

/// Copy fitted initial-state solutions. Uses the variable-length output
/// contract.
///
/// Safety: report must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_report_fits(
    report: *const SidereonOrbitFitReport,
    out: *mut SidereonOrbitFitSolution,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orbit_fit_report_fits",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_orbit_fit_report_fits",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_orbit_fit_report_fits",
                "report"
            ));
            let fits: Vec<_> = report
                .inner
                .fits
                .values()
                .map(orbit_fit_solution_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_orbit_fit_report_fits",
                "out",
                &fits,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy per-satellite residual ledger entries. Uses the variable-length output
/// contract.
///
/// Safety: report must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_report_satellite_ledger(
    report: *const SidereonOrbitFitReport,
    out: *mut SidereonOrbitSatelliteResidualEntry,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orbit_fit_report_satellite_ledger",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_orbit_fit_report_satellite_ledger",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_orbit_fit_report_satellite_ledger",
                "report"
            ));
            let entries: Vec<_> = report
                .inner
                .ledger
                .per_sat
                .iter()
                .map(|(&satellite, &stats)| SidereonOrbitSatelliteResidualEntry {
                    satellite: satellite_token(satellite),
                    stats: orbit_residual_stats_to_c(stats),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_orbit_fit_report_satellite_ledger",
                "out",
                &entries,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy per-constellation residual ledger entries. Uses the variable-length
/// output contract.
///
/// Safety: report must be live; out may be NULL only when len is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_report_constellation_ledger(
    report: *const SidereonOrbitFitReport,
    out: *mut SidereonOrbitConstellationResidualEntry,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orbit_fit_report_constellation_ledger",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_orbit_fit_report_constellation_ledger",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_orbit_fit_report_constellation_ledger",
                "report"
            ));
            let entries: Vec<_> = report
                .inner
                .ledger
                .per_constellation
                .iter()
                .map(
                    |(&system, &stats)| SidereonOrbitConstellationResidualEntry {
                        system: gnss_system_to_c(system) as u32,
                        stats: orbit_residual_stats_to_c(stats),
                    },
                )
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_orbit_fit_report_constellation_ledger",
                "out",
                &entries,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the residual ledger arc span.
///
/// Safety: report must be live; out_span must point to SidereonOrbitArcSpan.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_report_arc_span(
    report: *const SidereonOrbitFitReport,
    out_span: *mut SidereonOrbitArcSpan,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orbit_fit_report_arc_span",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_span,
                "sidereon_orbit_fit_report_arc_span",
                "out_span"
            ));
            *out = SidereonOrbitArcSpan {
                time_scale: SidereonTimeScale::Utc as u32,
                start_j2000_s: 0.0,
                end_j2000_s: 0.0,
                duration_s: 0.0,
            };
            let report = c_try!(require_ref(
                report,
                "sidereon_orbit_fit_report_arc_span",
                "report"
            ));
            *out = orbit_arc_span_to_c(report.inner.ledger.arc_span);
            SidereonStatus::Ok
        },
    )
}

/// Release an orbit-fit report handle. Null is a no-op.
///
/// Safety: report must be NULL or a live handle from an orbit-fit function.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_fit_report_free(report: *mut SidereonOrbitFitReport) {
    ffi_boundary("sidereon_orbit_fit_report_free", (), || {
        free_boxed(report);
    });
}

unsafe fn orbit_fit_options_from_c(
    fn_name: &str,
    options: *const SidereonOrbitFitOptions,
) -> Result<sidereon_core::ephemeris::OrbitFitOptions, SidereonStatus> {
    let owned_default;
    let options = if options.is_null() {
        owned_default = default_orbit_fit_options_c();
        &owned_default
    } else {
        require_ref(options, fn_name, "options")?
    };
    if options.initial_step_s <= 0.0 {
        set_last_error(format!("{fn_name}: initial_step_s must be positive"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let propagation_config = SidereonStatePropagationConfig {
        epoch_s: 0.0,
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
        force_model: options.force_model,
        integrator: options.integrator,
        abs_tol: options.abs_tol,
        rel_tol: options.rel_tol,
        initial_step_s: options.initial_step_s,
        min_step_s: options.min_step_s,
        max_step_s: options.max_step_s,
        max_steps: options.max_steps,
        mu_km3_s2_enabled: options.mu_km3_s2_enabled,
        mu_km3_s2: options.mu_km3_s2,
        has_drag: false,
        drag: options.drag,
        force_components: options.force_components,
    };
    let force_model = propagation_force_model_kind_from_c(
        fn_name,
        &propagation_config,
        CoreTideSystem::TideFree,
    )?;
    let drag = if options.has_drag {
        Some(drag_parameters_from_c(fn_name, options.drag)?)
    } else {
        None
    };
    let mut integrator_options = IntegratorOptions::default();
    integrator_options.abs_tol = options.abs_tol;
    integrator_options.rel_tol = options.rel_tol;
    integrator_options.initial_step = options.initial_step_s;
    integrator_options.min_step = options.min_step_s;
    integrator_options.max_step = options.max_step_s;
    integrator_options.max_steps = options.max_steps;
    integrator_options.dense_output = false;

    let mut solver_options = sidereon_core::astro::math::least_squares::SolveOptions::default();
    solver_options.gtol = options.solver_gtol;
    solver_options.ftol = options.solver_ftol;
    solver_options.xtol = options.solver_xtol;
    solver_options.max_nfev = options.solver_max_nfev;

    let mut o = sidereon_core::ephemeris::OrbitFitOptions::default();
    o.force_model = force_model;
    o.integrator = propagation_integrator_from_c(fn_name, options.integrator)?;
    o.integrator_options = integrator_options;
    o.solver_options = solver_options;
    o.linear_solve =
        sidereon_core::astro::math::least_squares::TrustRegionSolve::OwnedGaussianFirstTie;
    o.geometry_thresholds = sidereon_core::geometry_quality::GeometryQualityThresholds::default();
    o.min_ledger_samples = options.min_ledger_samples;
    o.drag = drag;
    o.space_weather = None;
    o.propagation_context = propagation_context_from_c(&propagation_config);
    Ok(o)
}

fn default_orbit_fit_options_c() -> SidereonOrbitFitOptions {
    let defaults = sidereon_core::ephemeris::OrbitFitOptions::default();
    let drag = DragParameters::from_bc_factor_m2_kg(
        0.01,
        SpaceWeather::default(),
        DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
    )
    .expect("default orbit-fit drag parameters are valid");
    SidereonOrbitFitOptions {
        force_model: SidereonPropagationForceModel::EarthPhaseA as u32,
        force_components: orbit_fit_default_force_components(),
        mu_km3_s2_enabled: false,
        mu_km3_s2: MU_EARTH,
        integrator: match defaults.integrator {
            IntegratorKind::Dp54 => SidereonPropagationIntegrator::Dp54 as u32,
            IntegratorKind::Rk4 => SidereonPropagationIntegrator::Rk4 as u32,
        },
        abs_tol: defaults.integrator_options.abs_tol,
        rel_tol: defaults.integrator_options.rel_tol,
        initial_step_s: defaults.integrator_options.initial_step,
        min_step_s: defaults.integrator_options.min_step,
        max_step_s: defaults.integrator_options.max_step,
        max_steps: defaults.integrator_options.max_steps,
        solver_gtol: defaults.solver_options.gtol,
        solver_ftol: defaults.solver_options.ftol,
        solver_xtol: defaults.solver_options.xtol,
        solver_max_nfev: defaults.solver_options.max_nfev,
        min_ledger_samples: defaults.min_ledger_samples,
        has_drag: false,
        drag: drag_parameters_to_c(drag),
    }
}

fn orbit_fit_default_force_components() -> SidereonForceModelComponents {
    SidereonForceModelComponents {
        has_two_body: true,
        two_body_mu_km3_s2_enabled: false,
        two_body_mu_km3_s2: MU_EARTH,
        has_zonal: false,
        zonal_max_degree: 6,
        has_spherical_harmonic: false,
        spherical_harmonic_max_degree: 8,
        spherical_harmonic_max_order: 8,
        has_solid_earth_tide: false,
        has_solid_earth_pole_tide: false,
        has_third_body: false,
        third_body_sun: true,
        third_body_moon: true,
        has_solar_radiation_pressure: false,
        solar_radiation_pressure: SidereonSolarRadiationPressure {
            cr: 1.0,
            area_to_mass_m2_kg: 0.01,
        },
        has_relativity: false,
    }
}

unsafe fn satellite_tokens_from_c(
    fn_name: &str,
    satellites: *const SidereonSatelliteToken,
    satellite_count: usize,
) -> Result<Vec<GnssSatelliteId>, SidereonStatus> {
    let raw = require_slice(satellites, satellite_count, fn_name, "satellites")?;
    let mut parsed = Vec::with_capacity(raw.len());
    for (idx, token) in raw.iter().enumerate() {
        let text = fixed_c_array_to_string(fn_name, &format!("satellites[{idx}]"), &token.bytes)?;
        parsed.push(GnssSatelliteId::from_str(&text).map_err(|_| {
            set_last_error(format!("{fn_name}: invalid satellite token: {text}"));
            SidereonStatus::InvalidToken
        })?);
    }
    Ok(parsed)
}

fn orbit_fit_solution_to_c(
    solution: &sidereon_core::ephemeris::OrbitFitSolution,
) -> SidereonOrbitFitSolution {
    SidereonOrbitFitSolution {
        satellite: satellite_token(solution.satellite),
        initial_state: cartesian_state_to_c(&solution.initial_state),
        covariance: orbit_fit_covariance_to_c(&solution.covariance),
        geometry_quality: geometry_quality_to_c(&solution.geometry_quality),
        seed_rms_3d_m: solution.seed_rms_3d_m,
        fit_rms_3d_m: solution.fit_rms_3d_m,
        iterations: solution.iterations,
    }
}

fn orbit_fit_covariance_to_c(
    covariance: &sidereon_core::ephemeris::OrbitFitCovariance,
) -> SidereonOrbitFitCovariance {
    match covariance {
        sidereon_core::ephemeris::OrbitFitCovariance::Estimated { matrix } => {
            SidereonOrbitFitCovariance {
                kind: SidereonOrbitFitCovarianceKind::Estimated as u32,
                matrix: flatten_mat6(**matrix),
            }
        }
        sidereon_core::ephemeris::OrbitFitCovariance::Unbounded => SidereonOrbitFitCovariance {
            kind: SidereonOrbitFitCovarianceKind::Unbounded as u32,
            matrix: [f64::NAN; 36],
        },
    }
}

fn orbit_residual_stats_to_c(
    stats: sidereon_core::ephemeris::OrbitResidualStats,
) -> SidereonOrbitResidualStats {
    SidereonOrbitResidualStats {
        radial_rms_m: stats.radial_rms_m,
        along_rms_m: stats.along_rms_m,
        cross_rms_m: stats.cross_rms_m,
        rms_3d_m: stats.rms_3d_m,
        n: stats.n,
        low_sample_count: stats.low_sample_count,
    }
}

fn orbit_arc_span_to_c(span: sidereon_core::ephemeris::OrbitArcSpan) -> SidereonOrbitArcSpan {
    SidereonOrbitArcSpan {
        time_scale: time_scale_to_c_code(span.time_scale),
        start_j2000_s: span.start_j2000_s,
        end_j2000_s: span.end_j2000_s,
        duration_s: span.duration_s,
    }
}

fn flatten_mat6(matrix: [[f64; 6]; 6]) -> [f64; 36] {
    let mut out = [0.0; 36];
    for row in 0..6 {
        for col in 0..6 {
            out[row * 6 + col] = matrix[row][col];
        }
    }
    out
}

fn map_orbit_fit_error(
    fn_name: &str,
    err: sidereon_core::ephemeris::OrbitFitError,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::OrbitFit,
        fn_name,
        orbit_fit_error_value(&err),
    );
    let status = match err {
        sidereon_core::ephemeris::OrbitFitError::EmptySelection
        | sidereon_core::ephemeris::OrbitFitError::InvalidOption { .. }
        | sidereon_core::ephemeris::OrbitFitError::TooFewSamples { .. }
        | sidereon_core::ephemeris::OrbitFitError::NonMonotonicEpochs { .. }
        | sidereon_core::ephemeris::OrbitFitError::MixedTimeScales
        | sidereon_core::ephemeris::OrbitFitError::InvalidEpoch { .. }
        | sidereon_core::ephemeris::OrbitFitError::InvalidObservation { .. }
        | sidereon_core::ephemeris::OrbitFitError::Frame { .. } => SidereonStatus::InvalidArgument,
        sidereon_core::ephemeris::OrbitFitError::Propagation { .. }
        | sidereon_core::ephemeris::OrbitFitError::LeastSquares { .. }
        | sidereon_core::ephemeris::OrbitFitError::SingularGeometry { .. }
        | sidereon_core::ephemeris::OrbitFitError::DidNotConverge { .. }
        | sidereon_core::ephemeris::OrbitFitError::RtnFrame { .. } => SidereonStatus::Solve,
        sidereon_core::ephemeris::OrbitFitError::Ut1OutsideCoverage(_) => {
            SidereonStatus::Ut1OutsideCoverage
        }
        sidereon_core::ephemeris::OrbitFitError::Ut1ValidityMismatch { .. } => {
            SidereonStatus::InvalidArgument
        }
    };
    set_last_error(format!("{fn_name}: {err}"));
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use sidereon_core::astro::covariance::RtnFrameError;
    use sidereon_core::astro::error::PropagationError;
    use sidereon_core::astro::frames::transforms::FrameTransformError;
    use sidereon_core::astro::math::least_squares::SolveError;
    use sidereon_core::astro::time::{DegradeReason, ValidityMode};
    use sidereon_core::ephemeris::{OrbitFitError as OFE, Ut1ProviderRole};
    use sidereon_core::geometry_quality::{GeometryQuality, ObservabilityTier};
    use sidereon_core::GnssSatelliteId;
    use std::ptr;
    use std::str::FromStr;

    const IGS_SP3_BYTES: &[u8] =
        include_bytes!("../tests/fixtures/sp3/IGS0OPSFIN_20261200945_02H30M_15M_ORB.SP3");

    #[test]
    fn test_orbit_fit_error_mapping_and_nested_payloads() {
        let sat1 = GnssSatelliteId::from_str("G01").expect("valid satellite token");
        let sat2 = GnssSatelliteId::from_str("G02").expect("valid satellite token");
        let sat3 = GnssSatelliteId::from_str("G03").expect("valid satellite token");
        let sat4 = GnssSatelliteId::from_str("G04").expect("valid satellite token");
        let sat5 = GnssSatelliteId::from_str("G05").expect("valid satellite token");
        let sat6 = GnssSatelliteId::from_str("G06").expect("valid satellite token");
        let sat7 = GnssSatelliteId::from_str("G07").expect("valid satellite token");
        let sat8 = GnssSatelliteId::from_str("G08").expect("valid satellite token");
        let sat9 = GnssSatelliteId::from_str("G09").expect("valid satellite token");
        let sat10 = GnssSatelliteId::from_str("G10").expect("valid satellite token");
        let sat11 = GnssSatelliteId::from_str("G11").expect("valid satellite token");
        let sat12 = GnssSatelliteId::from_str("G12").expect("valid satellite token");
        let sat13 = GnssSatelliteId::from_str("G13").expect("valid satellite token");
        let sat14 = GnssSatelliteId::from_str("G14").expect("valid satellite token");
        let sat15 = GnssSatelliteId::from_str("G15").expect("valid satellite token");
        let sat16 = GnssSatelliteId::from_str("G16").expect("valid satellite token");
        let sat17 = GnssSatelliteId::from_str("G17").expect("valid satellite token");

        // 1. EmptySelection
        let val = orbit_fit_error_value(&OFE::EmptySelection);
        assert_eq!(val["kind"], "empty_selection");
        assert_eq!(val["fields"], json!({}));

        // 2. InvalidOption
        let val = orbit_fit_error_value(&OFE::InvalidOption {
            field: "abs_tol",
            reason: "must be positive",
        });
        assert_eq!(val["kind"], "invalid_option");
        assert_eq!(val["fields"]["field"], "abs_tol");
        assert_eq!(val["fields"]["reason"], "must be positive");

        // 3. TooFewSamples
        let val = orbit_fit_error_value(&OFE::TooFewSamples {
            satellite: sat1,
            got: 1,
            required: 4,
        });
        assert_eq!(val["kind"], "too_few_samples");
        assert_eq!(val["fields"]["satellite"], "G01");
        assert_eq!(val["fields"]["got"], 1);
        assert_eq!(val["fields"]["required"], 4);

        // 4. NonMonotonicEpochs
        let val = orbit_fit_error_value(&OFE::NonMonotonicEpochs { satellite: sat2 });
        assert_eq!(val["kind"], "non_monotonic_epochs");
        assert_eq!(val["fields"]["satellite"], "G02");

        // 5. MixedTimeScales
        let val = orbit_fit_error_value(&OFE::MixedTimeScales);
        assert_eq!(val["kind"], "mixed_time_scales");
        assert_eq!(val["fields"], json!({}));

        // 6. InvalidEpoch
        let val = orbit_fit_error_value(&OFE::InvalidEpoch {
            satellite: sat3,
            reason: "out of time table bounds".to_string(),
        });
        assert_eq!(val["kind"], "invalid_epoch");
        assert_eq!(val["fields"]["satellite"], "G03");
        assert_eq!(val["fields"]["reason"], "out of time table bounds");

        // 7. InvalidObservation
        let val = orbit_fit_error_value(&OFE::InvalidObservation {
            satellite: sat4,
            reason: "non-finite position component",
        });
        assert_eq!(val["kind"], "invalid_observation");
        assert_eq!(val["fields"]["satellite"], "G04");
        assert_eq!(val["fields"]["reason"], "non-finite position component");

        // 8. Frame (nested FrameTransformError)
        let val = orbit_fit_error_value(&OFE::Frame {
            satellite: sat5,
            source: FrameTransformError::InvalidInput {
                field: "epoch",
                reason: "components must be finite",
            },
        });
        assert_eq!(val["kind"], "frame");
        assert_eq!(val["fields"]["satellite"], "G05");
        assert_eq!(val["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(val["fields"]["cause"]["fields"]["field"], "epoch");
        assert_eq!(
            val["fields"]["cause"]["fields"]["reason"],
            "components must be finite"
        );

        let val_frame_ut1 = orbit_fit_error_value(&OFE::Frame {
            satellite: sat6,
            source: FrameTransformError::Ut1OutsideCoverage {
                reason: DegradeReason::BeforeCoverage,
            },
        });
        assert_eq!(val_frame_ut1["kind"], "frame");
        assert_eq!(val_frame_ut1["fields"]["satellite"], "G06");
        assert_eq!(
            val_frame_ut1["fields"]["cause"]["kind"],
            "ut1_outside_coverage"
        );
        assert_eq!(
            val_frame_ut1["fields"]["cause"]["fields"]["reason"],
            "before_coverage"
        );

        // 9. Propagation (nested PropagationError)
        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat7,
            source: PropagationError::MaxStepsExceeded,
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G07");
        assert_eq!(val["fields"]["cause"]["kind"], "max_steps_exceeded");
        assert_eq!(val["fields"]["cause"]["fields"], json!({}));

        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat8,
            source: PropagationError::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G08");
        assert_eq!(val["fields"]["cause"]["kind"], "ut1_outside_coverage");
        assert_eq!(val["fields"]["cause"]["fields"]["reason"], "after_coverage");

        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat9,
            source: PropagationError::InvalidInput("initial step zero".to_string()),
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G09");
        assert_eq!(val["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(
            val["fields"]["cause"]["fields"]["message"],
            "initial step zero"
        );

        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat10,
            source: PropagationError::NumericalFailure("state divergence".to_string()),
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G10");
        assert_eq!(val["fields"]["cause"]["kind"], "numerical_failure");
        assert_eq!(
            val["fields"]["cause"]["fields"]["message"],
            "state divergence"
        );

        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat11,
            source: PropagationError::EventFailure("root search missed".to_string()),
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G11");
        assert_eq!(val["fields"]["cause"]["kind"], "event_failure");
        assert_eq!(
            val["fields"]["cause"]["fields"]["message"],
            "root search missed"
        );

        let val = orbit_fit_error_value(&OFE::Propagation {
            satellite: sat12,
            source: PropagationError::ForceModelFailure("atmosphere table expired".to_string()),
        });
        assert_eq!(val["kind"], "propagation");
        assert_eq!(val["fields"]["satellite"], "G12");
        assert_eq!(val["fields"]["cause"]["kind"], "force_model_failure");
        assert_eq!(
            val["fields"]["cause"]["fields"]["message"],
            "atmosphere table expired"
        );

        // 10. LeastSquares (nested SolveError)
        let val = orbit_fit_error_value(&OFE::LeastSquares {
            satellite: sat13,
            source: SolveError::SingularJacobian,
        });
        assert_eq!(val["kind"], "least_squares");
        assert_eq!(val["fields"]["satellite"], "G13");
        assert_eq!(val["fields"]["cause"]["kind"], "singular_jacobian");
        assert_eq!(val["fields"]["cause"]["fields"], json!({}));

        let val = orbit_fit_error_value(&OFE::LeastSquares {
            satellite: sat14,
            source: SolveError::InvalidInput {
                field: "y",
                reason: "contains NaN",
            },
        });
        assert_eq!(val["kind"], "least_squares");
        assert_eq!(val["fields"]["satellite"], "G14");
        assert_eq!(val["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(val["fields"]["cause"]["fields"]["field"], "y");
        assert_eq!(val["fields"]["cause"]["fields"]["reason"], "contains NaN");

        // 11. SingularGeometry (nested GeometryQuality)
        let cond_val = 1.25e9_f64;
        let gdop_val = 25.5_f64;
        let quality = GeometryQuality {
            tier: ObservabilityTier::RankDeficient,
            redundancy: -2,
            rank: 4,
            condition_number: cond_val,
            gdop: gdop_val,
            raim_checkable: false,
            covariance_validated: false,
        };
        let val = orbit_fit_error_value(&OFE::SingularGeometry {
            satellite: sat15,
            geometry_quality: quality,
        });
        assert_eq!(val["kind"], "singular_geometry");
        assert_eq!(val["fields"]["satellite"], "G15");
        let gq = &val["fields"]["geometry_quality"];
        assert_eq!(gq["tier"], "rank_deficient");
        assert_eq!(gq["redundancy"], -2);
        assert_eq!(gq["rank"], 4);
        assert_eq!(gq["condition_number"]["decimal"], cond_val.to_string());
        assert_eq!(
            gq["condition_number"]["bits_hex"],
            format!("{:016x}", cond_val.to_bits())
        );
        assert_eq!(gq["gdop"]["decimal"], gdop_val.to_string());
        assert_eq!(
            gq["gdop"]["bits_hex"],
            format!("{:016x}", gdop_val.to_bits())
        );
        assert_eq!(gq["raim_checkable"], false);
        assert_eq!(gq["covariance_validated"], false);

        // 12. DidNotConverge
        let val = orbit_fit_error_value(&OFE::DidNotConverge {
            satellite: sat16,
            iterations: 50,
        });
        assert_eq!(val["kind"], "did_not_converge");
        assert_eq!(val["fields"]["satellite"], "G16");
        assert_eq!(val["fields"]["iterations"], 50);

        // 13. Ut1OutsideCoverage
        let val = orbit_fit_error_value(&OFE::Ut1OutsideCoverage(DegradeReason::AfterCoverage));
        assert_eq!(val["kind"], "ut1_outside_coverage");
        assert_eq!(val["fields"]["reason"], "after_coverage");

        // 14. Ut1ValidityMismatch
        let val = orbit_fit_error_value(&OFE::Ut1ValidityMismatch {
            fit: ValidityMode::Strict,
            provider: Ut1ProviderRole::Orientation,
            provider_mode: ValidityMode::Permissive,
        });
        assert_eq!(val["kind"], "ut1_validity_mismatch");
        assert_eq!(val["fields"]["fit"], "strict");
        assert_eq!(val["fields"]["provider"], "orientation");
        assert_eq!(val["fields"]["provider_mode"], "permissive");

        // 15. RtnFrame (nested RtnFrameError)
        let val = orbit_fit_error_value(&OFE::RtnFrame {
            satellite: sat17,
            reason: RtnFrameError::ZeroPosition,
        });
        assert_eq!(val["kind"], "rtn_frame");
        assert_eq!(val["fields"]["satellite"], "G17");
        assert_eq!(val["fields"]["cause"]["kind"], "zero_position");
        assert_eq!(val["fields"]["cause"]["fields"], json!({}));

        let val_rtn_parallel = orbit_fit_error_value(&OFE::RtnFrame {
            satellite: sat17,
            reason: RtnFrameError::ParallelPositionVelocity,
        });
        assert_eq!(
            val_rtn_parallel["fields"]["cause"]["kind"],
            "parallel_position_velocity"
        );

        let val_rtn_inv = orbit_fit_error_value(&OFE::RtnFrame {
            satellite: sat17,
            reason: RtnFrameError::InvalidInput {
                field: "cov_rtn",
                reason: "matrix must be symmetric",
            },
        });
        assert_eq!(val_rtn_inv["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(val_rtn_inv["fields"]["cause"]["fields"]["field"], "cov_rtn");
        assert_eq!(
            val_rtn_inv["fields"]["cause"]["fields"]["reason"],
            "matrix must be symmetric"
        );
    }

    #[test]
    fn test_orbit_fit_public_control_and_refusal_lifecycle() {
        unsafe {
            let mut sp3: *mut SidereonSp3 = ptr::null_mut();
            assert_eq!(
                crate::sp3::sidereon_sp3_load(
                    IGS_SP3_BYTES.as_ptr(),
                    IGS_SP3_BYTES.len(),
                    &mut sp3
                ),
                SidereonStatus::Ok
            );
            assert!(!sp3.is_null());

            let mut options: SidereonOrbitFitOptions = std::mem::zeroed();
            assert_eq!(
                sidereon_orbit_fit_options_init(&mut options),
                SidereonStatus::Ok
            );
            options.force_model = SidereonPropagationForceModel::TwoBody as u32;

            let mut report: *mut SidereonOrbitFitReport = ptr::null_mut();

            // 1. Valid control: fit G01 with bounded two-body model from IGS SP3 (production default max_nfev=500)
            let fit_status =
                sidereon_fit_sp3_ecef_precise_orbit(sp3, c"G01".as_ptr(), &options, &mut report);
            if fit_status != SidereonStatus::Ok {
                let mut info = SidereonEngineErrorInfo {
                    family: SidereonEngineErrorFamily::None,
                    payload_len: 0,
                };
                let _ = sidereon_last_engine_error_info(&mut info);
                let payload_str = if info.payload_len > 0 {
                    let mut pbuf = vec![0u8; info.payload_len];
                    let mut written = 0;
                    let mut required = 0;
                    let _ = sidereon_last_engine_error_payload(
                        pbuf.as_mut_ptr(),
                        pbuf.len(),
                        &mut written,
                        &mut required,
                    );
                    String::from_utf8_lossy(&pbuf).into_owned()
                } else {
                    String::new()
                };
                let mut err_msg_buf = [0 as c_char; 512];
                let len =
                    crate::sidereon_last_error_message(err_msg_buf.as_mut_ptr(), err_msg_buf.len());
                let msg = if len > 0 {
                    std::ffi::CStr::from_ptr(err_msg_buf.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                };
                panic!(
                    "sidereon_fit_sp3_ecef_precise_orbit failed: status={:?}, message={:?}, engine_family={:?}, payload={}",
                    fit_status, msg, info.family, payload_str
                );
            }
            assert_eq!(fit_status, SidereonStatus::Ok);
            assert!(!report.is_null());

            // Verify engine error info is initially clear
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // 2. Real public refusal while valid report handle remains alive:
            // fit sample orbit with zero samples
            let mut bad_report: *mut SidereonOrbitFitReport = ptr::null_mut();
            assert_eq!(
                sidereon_fit_precise_ephemeris_sample_orbit(
                    ptr::null(),
                    0,
                    c"G01".as_ptr(),
                    &options,
                    &mut bad_report,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(bad_report.is_null());

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert!(info.payload_len > 0);
            let expected_len = info.payload_len;

            // Two-pass payload check: short buffer returns InvalidArgument, writes 0, leaves payload intact
            let mut short_buf = [0u8; 4];
            let mut written_short = 999;
            let mut required_short = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written_short,
                    &mut required_short,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written_short, 0);
            assert_eq!(required_short, expected_len);

            // Full buffer retrieves payload and captures owned snapshot
            let mut captured_payload = vec![0u8; expected_len];
            let mut written_full = 0;
            let mut required_full = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(
                    captured_payload.as_mut_ptr(),
                    captured_payload.len(),
                    &mut written_full,
                    &mut required_full,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written_full, expected_len);
            assert_eq!(required_full, expected_len);

            let payload: serde_json::Value =
                serde_json::from_slice(&captured_payload).expect("valid json engine error payload");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "orbit_fit");
            assert_eq!(
                payload["operation"],
                "sidereon_fit_precise_ephemeris_sample_orbit"
            );
            assert_eq!(payload["error"]["kind"], "too_few_samples");
            assert_eq!(payload["error"]["fields"]["satellite"], "G01");
            assert_eq!(payload["error"]["fields"]["got"], 0);
            assert_eq!(payload["error"]["fields"]["required"], 2);

            // 3. Active payload retention across LIVE report getters:
            // Check that report getters succeed and retain the active engine error and payload
            let mut fits = [std::mem::zeroed::<SidereonOrbitFitSolution>(); 2];
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_orbit_fit_report_fits(
                    report,
                    fits.as_mut_ptr(),
                    fits.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 1);
            assert_eq!(required, 1);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert_eq!(info.payload_len, expected_len);
            let mut verify_buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    verify_buf.as_mut_ptr(),
                    verify_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(verify_buf, captured_payload);

            let mut sat_entries = [std::mem::zeroed::<SidereonOrbitSatelliteResidualEntry>(); 2];
            assert_eq!(
                sidereon_orbit_fit_report_satellite_ledger(
                    report,
                    sat_entries.as_mut_ptr(),
                    sat_entries.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 1);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert_eq!(info.payload_len, expected_len);
            assert_eq!(
                sidereon_last_engine_error_payload(
                    verify_buf.as_mut_ptr(),
                    verify_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(verify_buf, captured_payload);

            let mut const_entries =
                [std::mem::zeroed::<SidereonOrbitConstellationResidualEntry>(); 2];
            assert_eq!(
                sidereon_orbit_fit_report_constellation_ledger(
                    report,
                    const_entries.as_mut_ptr(),
                    const_entries.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 1);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert_eq!(info.payload_len, expected_len);
            assert_eq!(
                sidereon_last_engine_error_payload(
                    verify_buf.as_mut_ptr(),
                    verify_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(verify_buf, captured_payload);

            let mut span = std::mem::zeroed::<SidereonOrbitArcSpan>();
            assert_eq!(
                sidereon_orbit_fit_report_arc_span(report, &mut span),
                SidereonStatus::Ok
            );
            assert!(span.duration_s > 0.0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert_eq!(info.payload_len, expected_len);
            assert_eq!(
                sidereon_last_engine_error_payload(
                    verify_buf.as_mut_ptr(),
                    verify_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(verify_buf, captured_payload);

            // Free the LIVE report handle: retains active error and payload
            sidereon_orbit_fit_report_free(report);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);
            assert_eq!(info.payload_len, expected_len);
            assert_eq!(
                sidereon_last_engine_error_payload(
                    verify_buf.as_mut_ptr(),
                    verify_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(verify_buf, captured_payload);

            // Early null reset: calling with invalid out_report resets error details
            assert_eq!(
                sidereon_fit_sp3_ecef_precise_orbit(
                    sp3,
                    c"G01".as_ptr(),
                    &options,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Success reset: re-trigger refusal then run valid control to verify reset to None
            assert_eq!(
                sidereon_fit_precise_ephemeris_sample_orbit(
                    ptr::null(),
                    0,
                    c"G01".as_ptr(),
                    &options,
                    &mut bad_report,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::OrbitFit);

            let mut success_report: *mut SidereonOrbitFitReport = ptr::null_mut();
            assert_eq!(
                sidereon_fit_sp3_ecef_precise_orbit(
                    sp3,
                    c"G01".as_ptr(),
                    &options,
                    &mut success_report,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            sidereon_orbit_fit_report_free(success_report);
            crate::sp3::sidereon_sp3_free(sp3);
        }
    }
}
