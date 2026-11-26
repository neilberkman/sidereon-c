use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use crate::force_models::{
    additional_forces_from_c, configure_radiation, force_context_from_c,
    reject_output_overlaps_forces,
};
use crate::orbit_fit::{orbit_fit_error_value, orbit_residual_stats_to_c};
use crate::rinex_clock::clock_epoch_from_c;
use serde_json::json;
use sidereon_core::astro::forces::SpaceWeatherSource;
use sidereon_core::astro::time::ValidityMode;
use sidereon_core::orbit_determination::{
    self as od, OrbitParameterCovariance, OrbitParameterFit, OrbitParameterFitError,
    OrbitParameterFitOptions,
};

/// Borrowed UTF-8 input for an orbit parameter name.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitParameterName {
    /// UTF-8 bytes.
    pub data: *const u8,
    /// Number of bytes.
    pub len: usize,
}

/// One inertial position observation at a time-scale-tagged epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonInertialPositionSample {
    /// Observation instant, as a split date or exact nanosecond count.
    pub epoch: SidereonClockEpoch,
    /// Observed GCRF position in metres.
    pub position_gcrf_m: [f64; 3],
}

/// Configuration for initial state and force-parameter estimation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitParameterFitOptions {
    /// Built-in model as SidereonPropagationForceModel.
    pub force_model: u32,
    /// Additive components for a composite, or SRP and EGM96 inputs for phase presets.
    pub force_components: SidereonForceModelComponents,
    /// Gravity tide system as SidereonGravityTideSystem.
    pub gravity_tide_system: u32,
    /// Optional solar radiation pressure lighting override.
    pub solar_shadow: *const SidereonRadiationShadowOptions,
    /// Optional Earth albedo and infrared pressure component.
    pub earth_radiation_pressure: *const SidereonEarthRadiationPressureOptions,
    /// Optional central gravity override in km³/s².
    pub mu_km3_s2: *const f64,
    /// Optional drag parameters.
    pub drag: *const SidereonDragParameters,
    /// Optional space-weather table when drag is enabled.
    pub space_weather: *const SidereonSpaceWeatherTable,
    /// Additional force handles in evaluation order.
    pub additional_forces: *const *const SidereonForceModel,
    /// Number of additional force handles.
    pub additional_force_count: usize,
    /// Force parameter names in estimation order.
    pub estimated_parameters: *const SidereonOrbitParameterName,
    /// Number of force parameter names.
    pub estimated_parameter_count: usize,
    /// Integrator and step controls.
    pub integrator_options: SidereonIntegratorOptions,
    /// Optional propagation context.
    pub propagation_context: *const SidereonPropagationContext,
    /// A-priori standard deviation of a position component, m.
    pub observation_sigma_m: f64,
    /// Maximum number of Gauss–Newton iterations.
    pub max_iterations: usize,
    /// Convergence threshold in formal standard deviations.
    pub convergence_threshold: f64,
    /// Maximum normal equation condition number.
    pub max_condition_number: f64,
    /// Residual count below which ledger entries are marked low-sample.
    pub min_ledger_samples: usize,
}

/// Default options for estimating the initial state and force parameters.
#[no_mangle]
pub extern "C" fn sidereon_orbit_parameter_fit_options_default() -> SidereonOrbitParameterFitOptions
{
    let value = OrbitParameterFitOptions::default();
    SidereonOrbitParameterFitOptions {
        force_model: SidereonPropagationForceModel::EarthPhaseA as u32,
        force_components: crate::ephemeris::default_force_model_components(),
        gravity_tide_system: SidereonGravityTideSystem::TideFree as u32,
        solar_shadow: ptr::null(),
        earth_radiation_pressure: ptr::null(),
        mu_km3_s2: ptr::null(),
        drag: ptr::null(),
        space_weather: ptr::null(),
        additional_forces: ptr::null(),
        additional_force_count: 0,
        estimated_parameters: ptr::null(),
        estimated_parameter_count: 0,
        integrator_options: SidereonIntegratorOptions::from_core(
            SidereonPropagationIntegrator::Dp54 as u32,
            &value.integrator_options,
        ),
        propagation_context: ptr::null(),
        observation_sigma_m: value.observation_sigma_m,
        max_iterations: value.max_iterations,
        convergence_threshold: value.convergence_threshold,
        max_condition_number: value.max_condition_number,
        min_ledger_samples: value.min_ledger_samples,
    }
}

unsafe fn options_from_c(
    operation: &str,
    input: &SidereonOrbitParameterFitOptions,
) -> Result<OrbitParameterFitOptions, SidereonStatus> {
    let mut config = crate::ephemeris::default_state_propagation_config();
    config.force_model = input.force_model;
    config.force_components = input.force_components;
    if let Some(mu) = input.mu_km3_s2.as_ref() {
        config.mu_km3_s2_enabled = true;
        config.mu_km3_s2 = *mu;
    }
    let mut options = OrbitParameterFitOptions::default();
    let tide_system = gravity_tide_system_from_c(operation, input.gravity_tide_system)?;
    options.force_model =
        propagation_force_model_kind_from_c(operation, &config, tide_system, false)?;
    configure_radiation(
        operation,
        &mut options.force_model,
        input.solar_shadow,
        input.earth_radiation_pressure,
    )?;
    options.drag = input
        .drag
        .as_ref()
        .map(|drag| drag_parameters_from_c(operation, *drag))
        .transpose()?;
    options.space_weather = input
        .space_weather
        .as_ref()
        .map(|table| SpaceWeatherSource::Table(table.inner.clone()));
    options.additional_forces = additional_forces_from_c(
        operation,
        input.additional_forces,
        input.additional_force_count,
    )?;
    options.estimated_parameters = require_slice(
        input.estimated_parameters,
        input.estimated_parameter_count,
        operation,
        "estimated_parameters",
    )?
    .iter()
    .map(|name| text_bytes_from_c(operation, name.data, name.len).map(str::to_owned))
    .collect::<Result<_, _>>()?;
    options.integrator =
        propagation_integrator_from_c(operation, input.integrator_options.integrator)?;
    options.integrator_options = input.integrator_options.to_core();
    options.propagation_context = force_context_from_c(
        input.propagation_context,
        propagation_context_from_c(&config),
    );
    options.observation_sigma_m = input.observation_sigma_m;
    options.max_iterations = input.max_iterations;
    options.convergence_threshold = input.convergence_threshold;
    options.max_condition_number = input.max_condition_number;
    options.min_ledger_samples = input.min_ledger_samples;
    Ok(options)
}

fn parameter_fit_error_value(err: &OrbitParameterFitError) -> serde_json::Value {
    use OrbitParameterFitError as E;
    let weakest = |terms: &[(String, f64)]| {
        terms
            .iter()
            .map(|(name, value)| json!([name, engine_f64(*value)]))
            .collect::<Vec<_>>()
    };
    let (kind, fields) = match err {
        E::Fit(cause) => ("fit", json!({"cause":orbit_fit_error_value(cause)})),
        E::InvalidParameter { name, reason } => {
            ("invalid_parameter", json!({"name":name,"reason":reason}))
        }
        E::UnobservableParameter { satellite, name } => (
            "unobservable_parameter",
            json!({"satellite":satellite.to_string(),"name":name}),
        ),
        E::SingularNormalEquations {
            satellite,
            condition_number,
            weakest: terms,
        } => (
            "singular_normal_equations",
            json!({
                "satellite":satellite.to_string(), "condition_number":engine_f64(*condition_number), "weakest":weakest(terms),
            }),
        ),
        E::IllConditionedNormalEquations {
            satellite,
            condition_number,
            limit,
            weakest: terms,
        } => (
            "ill_conditioned_normal_equations",
            json!({
                "satellite":satellite.to_string(), "condition_number":engine_f64(*condition_number),
                "limit":engine_f64(*limit), "weakest":weakest(terms),
            }),
        ),
        E::NumericalFailure {
            satellite,
            quantity,
        } => (
            "numerical_failure",
            json!({"satellite":satellite.to_string(),"quantity":quantity}),
        ),
        E::DidNotConverge {
            satellite,
            iterations,
            last_step_sigmas,
        } => (
            "did_not_converge",
            json!({
                "satellite":satellite.to_string(), "iterations":iterations, "last_step_sigmas":engine_f64(*last_step_sigmas),
            }),
        ),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_parameter_fit_error(operation: &str, err: OrbitParameterFitError) -> SidereonStatus {
    use OrbitParameterFitError as E;
    let status = match &err {
        E::Fit(cause) => crate::orbit_fit::orbit_fit_status(cause),
        E::InvalidParameter { .. } => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    };
    record_engine_error(
        SidereonEngineErrorFamily::OrbitParameterFit,
        operation,
        parameter_fit_error_value(&err),
    );
    set_last_error(format!("{operation}: {err}"));
    status
}

/// Owned orbit parameter fit. Release with sidereon_orbit_parameter_fit_free.
pub struct SidereonOrbitParameterFit {
    inner: OrbitParameterFit,
    degraded: SidereonUt1Degradation,
}

unsafe fn write_fit(
    operation: &str,
    out: *mut *mut SidereonOrbitParameterFit,
    options: *const SidereonOrbitParameterFitOptions,
    guard: impl FnOnce() -> Result<(), SidereonStatus>,
    fit: impl FnOnce() -> Result<(OrbitParameterFit, SidereonUt1Degradation), SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(guard());
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", options, "options"
        ));
        if let Some(options) = options.as_ref() {
            c_try!(reject_output_overlaps_handle(
                operation,
                out,
                "out",
                options.space_weather,
                "space_weather"
            ));
            c_try!(reject_output_overlaps_handle(
                operation,
                out,
                "out",
                options.propagation_context,
                "propagation_context"
            ));
            if let Some(context) = options.propagation_context.as_ref() {
                c_try!(reject_output_overlaps_handle(
                    operation,
                    out,
                    "out",
                    context.de_ephemeris,
                    "de_ephemeris"
                ));
                c_try!(reject_output_overlaps_handle(
                    operation,
                    out,
                    "out",
                    context.earth_orientation,
                    "earth_orientation"
                ));
            }
            c_try!(reject_output_overlaps_forces(
                operation,
                out,
                "out",
                options.additional_forces,
                options.additional_force_count
            ));
        }
        out.write(ptr::null_mut());
        let (inner, degraded) = c_try!(fit());
        out.write(Box::into_raw(Box::new(SidereonOrbitParameterFit {
            inner,
            degraded,
        })));
        SidereonStatus::Ok
    })
}

fn validity_mode(permissive: bool) -> ValidityMode {
    if permissive {
        ValidityMode::Permissive
    } else {
        ValidityMode::Strict
    }
}

/// Fit the initial state and force parameters to inertial position observations.
///
/// Safety: all input pointers are readable for their declared lengths and
/// contain live handles. satellite is NUL-terminated. initial_state can be NULL.
/// samples has count entries.
/// out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_inertial_orbit_parameters(
    samples: *const SidereonInertialPositionSample,
    count: usize,
    satellite: *const c_char,
    initial_state: *const SidereonCartesianState,
    options: *const SidereonOrbitParameterFitOptions,
    out: *mut *mut SidereonOrbitParameterFit,
) -> SidereonStatus {
    const OP: &str = "sidereon_fit_inertial_orbit_parameters";
    write_fit(
        OP,
        out,
        options,
        || Ok(()),
        || {
            let satellite = parse_satellite_token(OP, satellite)?;
            let initial_state = initial_state.as_ref().map(cartesian_state_from_c);
            let options = options_from_c(OP, require_ref(options, OP, "options")?)?;

            let rows = require_slice(samples, count, OP, "samples")?;
            let samples = rows
                .iter()
                .map(|row| {
                    Ok(od::InertialPositionSample {
                        epoch: clock_epoch_from_c(OP, "epoch", &row.epoch)?,
                        position_gcrf_m: row.position_gcrf_m,
                    })
                })
                .collect::<Result<Vec<_>, SidereonStatus>>()?;
            let fit =
                od::fit_inertial_orbit_parameters(&samples, satellite, initial_state, &options)
                    .map_err(|err| map_parameter_fit_error(OP, err))?;
            Ok((fit, SidereonUt1Degradation::None))
        },
    )
}

/// Fit the initial state and force parameters to inertial position observations.
///
/// Safety: all input pointers are readable for their declared lengths and
/// contain live handles. satellite is NUL-terminated. initial_state can be NULL.
/// samples has count entries.
/// out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_inertial_orbit_parameters_with_validity(
    samples: *const SidereonInertialPositionSample,
    count: usize,
    satellite: *const c_char,
    initial_state: *const SidereonCartesianState,
    options: *const SidereonOrbitParameterFitOptions,
    permissive: bool,
    out: *mut *mut SidereonOrbitParameterFit,
) -> SidereonStatus {
    const OP: &str = "sidereon_fit_inertial_orbit_parameters_with_validity";
    write_fit(
        OP,
        out,
        options,
        || Ok(()),
        || {
            let satellite = parse_satellite_token(OP, satellite)?;
            let initial_state = initial_state.as_ref().map(cartesian_state_from_c);
            let options = options_from_c(OP, require_ref(options, OP, "options")?)?;
            let mode = validity_mode(permissive);
            let rows = require_slice(samples, count, OP, "samples")?;
            let samples = rows
                .iter()
                .map(|row| {
                    Ok(od::InertialPositionSample {
                        epoch: clock_epoch_from_c(OP, "epoch", &row.epoch)?,
                        position_gcrf_m: row.position_gcrf_m,
                    })
                })
                .collect::<Result<Vec<_>, SidereonStatus>>()?;
            let fit = od::fit_inertial_orbit_parameters_with_validity(
                &samples,
                satellite,
                initial_state,
                &options,
                mode,
            )
            .map_err(|err| map_parameter_fit_error(OP, err))?;
            Ok((fit.value, SidereonUt1Degradation::from_core(fit.degraded)))
        },
    )
}

/// Fit the initial state and force parameters to SP3 positions rotated into GCRF.
///
/// Safety: all input pointers are readable for their declared lengths and
/// contain live handles. satellite is NUL-terminated. initial_state can be NULL.
/// provider can be NULL to use embedded Earth orientation.
/// out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_sp3_ecef_orbit_parameters(
    product: *const SidereonSp3,
    provider: *const SidereonIers2010EarthOrientationProvider,
    satellite: *const c_char,
    initial_state: *const SidereonCartesianState,
    options: *const SidereonOrbitParameterFitOptions,
    out: *mut *mut SidereonOrbitParameterFit,
) -> SidereonStatus {
    const OP: &str = "sidereon_fit_sp3_ecef_orbit_parameters";
    write_fit(
        OP,
        out,
        options,
        || {
            reject_output_overlaps_handle(OP, out, "out", product, "product")?;
            reject_output_overlaps_handle(OP, out, "out", provider, "provider")
        },
        || {
            let satellite = parse_satellite_token(OP, satellite)?;
            let initial_state = initial_state.as_ref().map(cartesian_state_from_c);
            let options = options_from_c(OP, require_ref(options, OP, "options")?)?;
            let mode = ValidityMode::Strict;
            let product = require_ref(product, OP, "product")?;
            let embedded = TdbEarthOrientationProvider::new().with_validity(mode);
            let orientation: &dyn sidereon_core::astro::frames::EarthOrientationProvider = provider
                .as_ref()
                .map_or(&embedded as _, |provider| provider.inner.as_ref());
            let fit = od::fit_sp3_ecef_orbit_parameters(
                &product.inner,
                satellite,
                orientation,
                initial_state,
                &options,
            )
            .map_err(|err| map_parameter_fit_error(OP, err))?;
            Ok((fit, SidereonUt1Degradation::None))
        },
    )
}

/// Fit the initial state and force parameters to SP3 positions rotated into GCRF.
///
/// Safety: all input pointers are readable for their declared lengths and
/// contain live handles. satellite is NUL-terminated. initial_state can be NULL.
/// provider can be NULL to use embedded Earth orientation.
/// out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_fit_sp3_ecef_orbit_parameters_with_validity(
    product: *const SidereonSp3,
    provider: *const SidereonIers2010EarthOrientationProvider,
    satellite: *const c_char,
    initial_state: *const SidereonCartesianState,
    options: *const SidereonOrbitParameterFitOptions,
    permissive: bool,
    out: *mut *mut SidereonOrbitParameterFit,
) -> SidereonStatus {
    const OP: &str = "sidereon_fit_sp3_ecef_orbit_parameters_with_validity";
    write_fit(
        OP,
        out,
        options,
        || {
            reject_output_overlaps_handle(OP, out, "out", product, "product")?;
            reject_output_overlaps_handle(OP, out, "out", provider, "provider")
        },
        || {
            let satellite = parse_satellite_token(OP, satellite)?;
            let initial_state = initial_state.as_ref().map(cartesian_state_from_c);
            let options = options_from_c(OP, require_ref(options, OP, "options")?)?;
            let mode = validity_mode(permissive);
            let product = require_ref(product, OP, "product")?;
            let embedded = TdbEarthOrientationProvider::new().with_validity(mode);
            let orientation: &dyn sidereon_core::astro::frames::EarthOrientationProvider = provider
                .as_ref()
                .map_or(&embedded as _, |provider| provider.inner.as_ref());
            let fit = od::fit_sp3_ecef_orbit_parameters_with_validity(
                &product.inner,
                satellite,
                orientation,
                initial_state,
                &options,
                mode,
            )
            .map_err(|err| map_parameter_fit_error(OP, err))?;
            Ok((fit.value, SidereonUt1Degradation::from_core(fit.degraded)))
        },
    )
}

/// Release a force-parameter fit result.
///
/// Safety: fit is NULL or a live owned result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_free(fit: *mut SidereonOrbitParameterFit) {
    ffi_boundary("sidereon_orbit_parameter_fit_free", (), || free_boxed(fit));
}

/// Summary of a force-parameter fit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitParameterFitSummary {
    /// Fitted satellite.
    pub satellite: SidereonSatelliteToken,
    /// Estimated initial state at the first observation.
    pub initial_state: SidereonCartesianState,
    /// Number of estimated force parameters.
    pub force_parameter_count: usize,
    /// Number of initial-state and force parameters.
    pub parameter_count: usize,
    /// Post-fit residual statistics.
    pub residual_stats: SidereonOrbitResidualStats,
    /// Initial guess three-dimensional RMS residual, m.
    pub seed_rms_3d_m: f64,
    /// Iteration count.
    pub iterations: usize,
    /// Column-scaled normal equation condition number.
    pub condition_number: f64,
    /// First UT1 degradation, if any.
    pub ut1_degraded: SidereonUt1Degradation,
}

/// Read an orbit parameter fit summary.
///
/// Safety: fit is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_summary(
    fit: *const SidereonOrbitParameterFit,
    out: *mut SidereonOrbitParameterFitSummary,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_summary";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", fit, "fit"));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let result = &fit.inner;
        out.write(SidereonOrbitParameterFitSummary {
            satellite: satellite_token_from_text(&result.satellite.to_string()),
            initial_state: cartesian_state_to_c(&result.initial_state),
            force_parameter_count: result.force_parameters.len(),
            parameter_count: result.parameter_names.len(),
            residual_stats: orbit_residual_stats_to_c(result.residual_stats),
            seed_rms_3d_m: result.seed_rms_3d_m,
            iterations: result.iterations,
            condition_number: result.condition_number,
            ut1_degraded: fit.degraded,
        });
        SidereonStatus::Ok
    })
}

/// One post-fit RTN residual. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonOrbitRtnResidual {
    /// Observation epoch in TDB and elapsed TT since the estimated initial state.
    pub variational_epoch: SidereonVariationalEpoch,
    /// Radial residual in metres.
    pub radial_m: f64,
    /// Along-track residual in metres.
    pub along_m: f64,
    /// Cross-track residual in metres.
    pub cross_m: f64,
}

/// Copy the full residual list in observation order.
/// NULL and zero capacity query the required count.
///
/// Safety: fit is live. All outputs are writable and disjoint from each other
/// and the handle. out has capacity records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_residuals(
    fit: *const SidereonOrbitParameterFit,
    out: *mut SidereonOrbitRtnResidual,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_residuals";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            fit,
            "fit"
        ));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let rows: Vec<_> = fit
            .inner
            .residuals
            .iter()
            .map(|row| SidereonOrbitRtnResidual {
                variational_epoch: row.variational_epoch().into(),
                radial_m: row.radial_m,
                along_m: row.along_m,
                cross_m: row.cross_m,
            })
            .collect();
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &rows,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Metadata for the scaled covariance and correlation matrices.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonOrbitParameterCovariance {
    /// Whether estimated covariance and correlation matrices are available.
    pub estimated: bool,
    /// Number of rows and columns in each matrix, when estimated.
    pub dimension: usize,
    /// A-posteriori variance factor, when estimated.
    pub variance_factor: f64,
    /// Residual degrees of freedom.
    pub degrees_of_freedom: usize,
}

/// Read orbit parameter covariance metadata.
///
/// Safety: fit is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_covariance(
    fit: *const SidereonOrbitParameterFit,
    out: *mut SidereonOrbitParameterCovariance,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_covariance";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", fit, "fit"));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        out.write(match &fit.inner.covariance {
            OrbitParameterCovariance::Estimated {
                matrix,
                variance_factor,
                degrees_of_freedom,
                ..
            } => SidereonOrbitParameterCovariance {
                estimated: true,
                dimension: matrix.nrows(),
                variance_factor: *variance_factor,
                degrees_of_freedom: *degrees_of_freedom,
            },
            OrbitParameterCovariance::Unbounded => SidereonOrbitParameterCovariance::default(),
        });
        SidereonStatus::Ok
    })
}

/// Copy the scaled covariance or correlation matrix in row-major order.
/// Unbounded covariance has zero entries. NULL and zero capacity query the full length.
///
/// Safety: fit is live. Outputs are writable and disjoint from each other
/// and the handle. out has capacity doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_covariance_matrix(
    fit: *const SidereonOrbitParameterFit,
    correlation: bool,
    out: *mut f64,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_covariance_matrix";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            fit,
            "fit"
        ));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let values = match &fit.inner.covariance {
            OrbitParameterCovariance::Estimated {
                matrix,
                correlation: correlations,
                ..
            } => {
                let matrix = if correlation { correlations } else { matrix };
                let columns = matrix.ncols();
                (0..matrix.len())
                    .map(|i| matrix[(i / columns, i % columns)])
                    .collect()
            }
            OrbitParameterCovariance::Unbounded => Vec::new(),
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &values,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Estimated force parameter with its formal uncertainty. For value equality
/// of the estimate, compare the name, initial value, and estimated value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonEstimatedForceParameter {
    /// Whether an estimate is present.
    pub present: bool,
    /// Configured value before fitting.
    pub initial_value: f64,
    /// Estimated value.
    pub value: f64,
    /// Formal standard deviation, when covariance is available.
    pub sigma: SidereonOptionalOrbitScalar,
}

fn estimated_force_parameter_to_c(
    fit: &OrbitParameterFit,
    value: &od::EstimatedForceParameter,
) -> SidereonEstimatedForceParameter {
    SidereonEstimatedForceParameter {
        present: true,
        initial_value: value.initial_value,
        value: value.value,
        sigma: fit.sigma(&value.name).into(),
    }
}

/// Read an estimated force parameter by name.
///
/// Safety: fit is live, name's bytes are readable and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_force_parameter(
    fit: *const SidereonOrbitParameterFit,
    name: SidereonOrbitParameterName,
    out: *mut SidereonEstimatedForceParameter,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_force_parameter";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", fit, "fit"));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let name = c_try!(text_bytes_from_c(OP, name.data, name.len));
        let estimate = fit
            .inner
            .force_parameter(name)
            .map(|value| estimated_force_parameter_to_c(&fit.inner, value));
        out.write(estimate.unwrap_or_default());
        SidereonStatus::Ok
    })
}

/// Read an estimated force parameter by row index in the fit's force parameter
/// list. The UTF-8 name has no terminator. NULL and zero capacity query the
/// full byte count. The row count is available in the fit summary.
///
/// Safety: fit is live. Outputs are writable and disjoint from each other
/// and the handle. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_force_parameter_at(
    fit: *const SidereonOrbitParameterFit,
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_parameter: *mut SidereonEstimatedForceParameter,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_force_parameter_at";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out_parameter = c_try!(require_out(out_parameter, OP, "out_parameter"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out_parameter,
            "out_parameter",
            fit,
            "fit"
        ));
        let result = &c_try!(require_ref(fit, OP, "fit")).inner;
        let row = result.force_parameters.get(index);
        let name_bytes = row.map_or(&[][..], |row| row.name.as_bytes());
        let name_count = if capacity >= name_bytes.len() {
            name_bytes.len()
        } else {
            0
        };
        c_try!(reject_outputs_overlapping_handle(
            OP,
            &[
                (out_name.cast(), 1, name_count, "out_name"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required"),
            ],
            out_parameter,
            "out_parameter",
        ));
        c_try!(init_handle_copy_counts(
            OP,
            out_name,
            name_count,
            out_written,
            out_required,
            fit,
            "fit"
        ));
        let Some(row) = row else {
            return extra_invalid_arg(
                OP,
                format_args!("force parameter index {index} out of range"),
            );
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name_bytes,
            out_name,
            capacity,
            out_written,
            out_required
        ));
        out_parameter.write(estimated_force_parameter_to_c(result, row));
        SidereonStatus::Ok
    })
}

/// Compare two force-parameter fits by value.
///
/// Safety: both fits are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_equal(
    left: *const SidereonOrbitParameterFit,
    right: *const SidereonOrbitParameterFit,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_equal";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", left, "left"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", right, "right"
        ));
        let left = c_try!(require_ref(left, OP, "left"));
        let right = c_try!(require_ref(right, OP, "right"));
        out.write(left.inner == right.inner);
        SidereonStatus::Ok
    })
}

/// Copy a parameter name in covariance order. The UTF-8 name has no terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: fit is live. All outputs are writable and disjoint from each other
/// and the handle. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_parameter_name(
    fit: *const SidereonOrbitParameterFit,
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_parameter_name";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out_name,
            capacity,
            out_written,
            out_required,
            fit,
            "fit"
        ));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let Some(name) = fit.inner.parameter_names.get(index) else {
            return extra_invalid_arg(OP, format_args!("parameter index {index} out of range"));
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Number of names for the initial-state parameters.
#[no_mangle]
pub extern "C" fn sidereon_state_parameter_name_count() -> usize {
    od::STATE_PARAMETER_NAMES.len()
}

/// Copy an initial-state parameter name in covariance order. The UTF-8 name
/// has no terminator. NULL and zero capacity query the full byte count.
///
/// Safety: all outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_state_parameter_name(
    index: usize,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_state_parameter_name";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let Some(name) = od::STATE_PARAMETER_NAMES.get(index) else {
            return extra_invalid_arg(
                OP,
                format_args!("state parameter index {index} out of range"),
            );
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name.as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Read the standard deviation of a parameter from the scaled covariance.
///
/// Safety: fit is live, name bytes are readable and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_sigma(
    fit: *const SidereonOrbitParameterFit,
    name: SidereonOrbitParameterName,
    out: *mut SidereonOptionalOrbitScalar,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_sigma";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", fit, "fit"));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let name = c_try!(text_bytes_from_c(OP, name.data, name.len));
        out.write(fit.inner.sigma(name).into());
        SidereonStatus::Ok
    })
}

/// Read the correlation between two named parameters.
///
/// Safety: fit is live, name bytes are readable and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_parameter_fit_correlation(
    fit: *const SidereonOrbitParameterFit,
    first: SidereonOrbitParameterName,
    second: SidereonOrbitParameterName,
    out: *mut SidereonOptionalOrbitScalar,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_parameter_fit_correlation";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", fit, "fit"));
        let fit = c_try!(require_ref(fit, OP, "fit"));
        let first = c_try!(text_bytes_from_c(OP, first.data, first.len));
        let second = c_try!(text_bytes_from_c(OP, second.data, second.len));
        out.write(fit.inner.correlation(first, second).into());
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn orbit_parameter_fit_error_details() {
        use OrbitParameterFitError as E;
        let cases = [
            (
                E::Fit(sidereon_core::ephemeris::OrbitFitError::EmptySelection),
                "fit",
                json!({"cause":{"kind":"empty_selection","fields":{}}}),
            ),
            (
                E::InvalidParameter {
                    name: String::from("sample"),
                    reason: "sample",
                },
                "invalid_parameter",
                json!({"name":"sample", "reason":"sample"}),
            ),
            (
                E::UnobservableParameter {
                    satellite: "G03".parse().unwrap(),
                    name: String::from("sample"),
                },
                "unobservable_parameter",
                json!({"satellite":"G03", "name":"sample"}),
            ),
            (
                E::SingularNormalEquations {
                    satellite: "G03".parse().unwrap(),
                    condition_number: 0.5,
                    weakest: vec![(String::from("sample"), 0.5)],
                },
                "singular_normal_equations",
                json!({"satellite":"G03", "condition_number":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "weakest":[["sample",{"decimal":"0.5","bits_hex":"3fe0000000000000"}]]}),
            ),
            (
                E::IllConditionedNormalEquations {
                    satellite: "G03".parse().unwrap(),
                    condition_number: 0.5,
                    limit: 0.5,
                    weakest: vec![(String::from("sample"), 0.5)],
                },
                "ill_conditioned_normal_equations",
                json!({"satellite":"G03", "condition_number":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "limit":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "weakest":[["sample",{"decimal":"0.5","bits_hex":"3fe0000000000000"}]]}),
            ),
            (
                E::NumericalFailure {
                    satellite: "G03".parse().unwrap(),
                    quantity: "sample",
                },
                "numerical_failure",
                json!({"satellite":"G03", "quantity":"sample"}),
            ),
            (
                E::DidNotConverge {
                    satellite: "G03".parse().unwrap(),
                    iterations: 3,
                    last_step_sigmas: 0.5,
                },
                "did_not_converge",
                json!({"satellite":"G03", "iterations":3, "last_step_sigmas":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(
                parameter_fit_error_value(&error),
                json!({"kind":kind,"fields":fields})
            );
        }
    }
}
