use super::*;

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
    ffi_boundary(
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
    ffi_boundary(
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
    let force_model = propagation_force_model_kind_from_c(fn_name, &propagation_config)?;
    let drag = if options.has_drag {
        Some(drag_parameters_from_c(fn_name, options.drag)?)
    } else {
        None
    };
    Ok(sidereon_core::ephemeris::OrbitFitOptions {
        force_model,
        integrator: propagation_integrator_from_c(fn_name, options.integrator)?,
        integrator_options: IntegratorOptions {
            abs_tol: options.abs_tol,
            rel_tol: options.rel_tol,
            initial_step: options.initial_step_s,
            min_step: options.min_step_s,
            max_step: options.max_step_s,
            max_steps: options.max_steps,
            dense_output: false,
        },
        solver_options: sidereon_core::astro::math::least_squares::SolveOptions {
            gtol: options.solver_gtol,
            ftol: options.solver_ftol,
            xtol: options.solver_xtol,
            max_nfev: options.solver_max_nfev,
        },
        linear_solve:
            sidereon_core::astro::math::least_squares::TrustRegionSolve::OwnedGaussianFirstTie,
        geometry_thresholds: sidereon_core::geometry_quality::GeometryQualityThresholds::default(),
        min_ledger_samples: options.min_ledger_samples,
        drag,
        space_weather: None,
    })
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
    };
    set_last_error(format!("{fn_name}: {err}"));
    status
}
