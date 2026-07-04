use super::*;

/// Orbital decay estimate controls. Initialize with sidereon_decay_config_init.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDecayConfig {
    /// Gravity model under drag, one of SidereonPropagationForceModel.
    pub force_model: u32,
    /// One of SidereonPropagationIntegrator.
    pub integrator: u32,
    /// Absolute tolerance.
    pub abs_tol: f64,
    /// Relative tolerance.
    pub rel_tol: f64,
    /// Initial integration step in seconds.
    pub initial_step_s: f64,
    /// Minimum integration step in seconds.
    pub min_step_s: f64,
    /// Maximum integration step in seconds.
    pub max_step_s: f64,
    /// Maximum internal integrator steps.
    pub max_steps: u32,
    /// Whether mu_km3_s2 overrides the selected model.
    pub mu_km3_s2_enabled: bool,
    /// Optional gravitational parameter override, km^3/s^2.
    pub mu_km3_s2: f64,
    /// Validated drag parameters.
    pub drag: SidereonDragParameters,
    /// Reentry threshold altitude, km.
    pub reentry_altitude_km: f64,
    /// Coarse scan step, s.
    pub scan_step_s: f64,
    /// Bisection time tolerance, s.
    pub crossing_tolerance_s: f64,
    /// Maximum elapsed scan horizon, s.
    pub max_duration_s: f64,
    /// Maximum coarse scan samples.
    pub max_scan_samples: u32,
}

/// Result of a drag-decay estimate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDecayEstimate {
    /// Seconds from the initial epoch to reentry.
    pub time_to_decay_s: f64,
    /// State at reentry.
    pub reentry_state: SidereonCartesianState,
    /// Geodetic altitude at the reported state, km.
    pub reentry_altitude_km: f64,
}

/// Initialize decay-estimate controls with core defaults.
///
/// Safety: out_config must point to a SidereonDecayConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_decay_config_init(
    out_config: *mut SidereonDecayConfig,
) -> SidereonStatus {
    ffi_boundary("sidereon_decay_config_init", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_config,
            "sidereon_decay_config_init",
            "out_config"
        ));
        *out = default_decay_config();
        SidereonStatus::Ok
    })
}

/// Estimate time to reentry using drag-perturbed numerical propagation.
///
/// Safety: initial and config must point to valid structs; out_estimate must
/// point to a SidereonDecayEstimate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_estimate_decay(
    initial: *const SidereonCartesianState,
    config: *const SidereonDecayConfig,
    out_estimate: *mut SidereonDecayEstimate,
) -> SidereonStatus {
    ffi_boundary("sidereon_estimate_decay", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_estimate,
            "sidereon_estimate_decay",
            "out_estimate"
        ));
        *out = SidereonDecayEstimate {
            time_to_decay_s: 0.0,
            reentry_state: SidereonCartesianState {
                epoch_s: 0.0,
                position_km: [0.0; 3],
                velocity_km_s: [0.0; 3],
            },
            reentry_altitude_km: 0.0,
        };
        let initial = c_try!(require_ref(initial, "sidereon_estimate_decay", "initial"));
        let config = c_try!(require_ref(config, "sidereon_estimate_decay", "config"));
        let config = c_try!(decay_config_from_c("sidereon_estimate_decay", config));
        match estimate_decay(cartesian_state_from_c(initial), &config) {
            Ok(value) => {
                *out = decay_estimate_to_c(value);
                SidereonStatus::Ok
            }
            Err(err) => map_decay_error("sidereon_estimate_decay", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_estimate_decay_with_space_weather_table(
    initial: *const SidereonCartesianState,
    config: *const SidereonDecayConfig,
    table: *const SidereonSpaceWeatherTable,
    out_estimate: *mut SidereonDecayEstimate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_estimate_decay_with_space_weather_table",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_estimate,
                "sidereon_estimate_decay_with_space_weather_table",
                "out_estimate"
            ));
            *out = SidereonDecayEstimate {
                time_to_decay_s: 0.0,
                reentry_state: SidereonCartesianState {
                    epoch_s: 0.0,
                    position_km: [0.0; 3],
                    velocity_km_s: [0.0; 3],
                },
                reentry_altitude_km: 0.0,
            };
            let initial = c_try!(require_ref(
                initial,
                "sidereon_estimate_decay_with_space_weather_table",
                "initial"
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_estimate_decay_with_space_weather_table",
                "config"
            ));
            let config = c_try!(decay_config_from_c(
                "sidereon_estimate_decay_with_space_weather_table",
                config
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_estimate_decay_with_space_weather_table",
                "table"
            ));
            let source = SpaceWeatherSource::Table(table.inner.clone());
            match sidereon_core::astro::propagator::estimate_decay_with_source(
                cartesian_state_from_c(initial),
                &config,
                &source,
            ) {
                Ok(value) => {
                    *out = decay_estimate_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_decay_error("sidereon_estimate_decay_with_space_weather_table", err)
                }
            }
        },
    )
}

fn default_decay_config() -> SidereonDecayConfig {
    let drag = DragParameters::from_bc_factor_m2_kg(
        0.01,
        SpaceWeather::default(),
        DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
    )
    .expect("default drag parameters are valid");
    let core = DecayConfig::new(drag);
    SidereonDecayConfig {
        force_model: SidereonPropagationForceModel::TwoBodyJ2 as u32,
        integrator: SidereonPropagationIntegrator::Dp54 as u32,
        abs_tol: core.options.abs_tol,
        rel_tol: core.options.rel_tol,
        initial_step_s: core.options.initial_step,
        min_step_s: core.options.min_step,
        max_step_s: core.options.max_step,
        max_steps: core.options.max_steps,
        mu_km3_s2_enabled: core.mu_km3_s2.is_some(),
        mu_km3_s2: core.mu_km3_s2.unwrap_or(MU_EARTH),
        drag: drag_parameters_to_c(core.drag),
        reentry_altitude_km: core.reentry_altitude_km,
        scan_step_s: core.scan_step_s,
        crossing_tolerance_s: core.crossing_tolerance_s,
        max_duration_s: core.max_duration_s,
        max_scan_samples: core.max_scan_samples,
    }
}

fn decay_config_from_c(
    fn_name: &str,
    config: &SidereonDecayConfig,
) -> Result<DecayConfig, SidereonStatus> {
    let drag = drag_parameters_from_c(fn_name, config.drag)?;
    let options = IntegratorOptions {
        abs_tol: config.abs_tol,
        rel_tol: config.rel_tol,
        initial_step: config.initial_step_s,
        min_step: config.min_step_s,
        max_step: config.max_step_s,
        max_steps: config.max_steps,
        dense_output: false,
    };
    Ok(DecayConfig::new(drag)
        .with_force_model(propagation_force_model_from_c(fn_name, config.force_model)?)
        .with_mu_km3_s2(config.mu_km3_s2_enabled.then_some(config.mu_km3_s2))
        .with_integrator(propagation_integrator_from_c(fn_name, config.integrator)?)
        .with_options(options)
        .with_reentry_altitude_km(config.reentry_altitude_km)
        .with_scan_step_s(config.scan_step_s)
        .with_crossing_tolerance_s(config.crossing_tolerance_s)
        .with_max_duration_s(config.max_duration_s)
        .with_max_scan_samples(config.max_scan_samples))
}

fn decay_estimate_to_c(value: DecayEstimate) -> SidereonDecayEstimate {
    SidereonDecayEstimate {
        time_to_decay_s: value.time_to_decay_s,
        reentry_state: cartesian_state_to_c(&value.reentry_state),
        reentry_altitude_km: value.reentry_altitude_km,
    }
}

fn map_decay_error(fn_name: &str, err: DecayError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        DecayError::InvalidConfig(_) => SidereonStatus::InvalidArgument,
        DecayError::Propagation(_)
        | DecayError::NoDecayWithinHorizon { .. }
        | DecayError::ScanBudgetExhausted { .. } => SidereonStatus::Solve,
    }
}
