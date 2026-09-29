use super::*;
use crate::covariance::covariance_propagation_error_value;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

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
    const FN: &str = "sidereon_estimate_decay";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_estimate, FN, "out_estimate"));
        *out = SidereonDecayEstimate {
            time_to_decay_s: 0.0,
            reentry_state: SidereonCartesianState {
                epoch_s: 0.0,
                position_km: [0.0; 3],
                velocity_km_s: [0.0; 3],
            },
            reentry_altitude_km: 0.0,
        };
        let initial = c_try!(require_ref(initial, FN, "initial"));
        let config = c_try!(require_ref(config, FN, "config"));
        let config = c_try!(decay_config_from_c(FN, config));
        match estimate_decay(cartesian_state_from_c(initial), &config) {
            Ok(value) => {
                *out = decay_estimate_to_c(value);
                SidereonStatus::Ok
            }
            Err(err) => map_decay_error(FN, err),
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
    const FN: &str = "sidereon_estimate_decay_with_space_weather_table";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_estimate, FN, "out_estimate"));
        *out = SidereonDecayEstimate {
            time_to_decay_s: 0.0,
            reentry_state: SidereonCartesianState {
                epoch_s: 0.0,
                position_km: [0.0; 3],
                velocity_km_s: [0.0; 3],
            },
            reentry_altitude_km: 0.0,
        };
        let initial = c_try!(require_ref(initial, FN, "initial"));
        let config = c_try!(require_ref(config, FN, "config"));
        let config = c_try!(decay_config_from_c(FN, config));
        let table = c_try!(require_ref(table, FN, "table"));
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
            Err(err) => map_decay_error(FN, err),
        }
    })
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
    let mut options = IntegratorOptions::default();
    options.abs_tol = config.abs_tol;
    options.rel_tol = config.rel_tol;
    options.initial_step = config.initial_step_s;
    options.min_step = config.min_step_s;
    options.max_step = config.max_step_s;
    options.max_steps = config.max_steps;
    options.dense_output = false;
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

pub fn decay_node(kind: &str, fields: Value) -> Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

pub fn decay_error_value(error: &DecayError) -> Value {
    match error {
        DecayError::Propagation(source) => decay_node(
            "propagation",
            json!({
                "cause": covariance_propagation_error_value(source),
            }),
        ),
        DecayError::InvalidConfig(field) => decay_node(
            "invalid_config",
            json!({
                "field": field,
                "reason": field,
            }),
        ),
        DecayError::NoDecayWithinHorizon { horizon_s } => decay_node(
            "no_decay_within_horizon",
            json!({
                "horizon_s": engine_f64(*horizon_s),
            }),
        ),
        DecayError::ScanBudgetExhausted { scanned_s, samples } => decay_node(
            "scan_budget_exhausted",
            json!({
                "scanned_s": engine_f64(*scanned_s),
                "samples": samples,
            }),
        ),
    }
}

pub fn map_decay_error(fn_name: &str, err: DecayError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Decay,
        fn_name,
        decay_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        DecayError::InvalidConfig(_) => SidereonStatus::InvalidArgument,
        DecayError::Propagation(_)
        | DecayError::NoDecayWithinHorizon { .. }
        | DecayError::ScanBudgetExhausted { .. } => SidereonStatus::Solve,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorInfo,
    };
    use sidereon_core::astro::error::PropagationError;
    use sidereon_core::astro::forces::drag::DragParameters;
    use sidereon_core::astro::forces::SpaceWeather;
    use std::ptr;

    #[test]
    fn table_driven_decay_error_mapping() {
        clear_engine_error();

        let cases: Vec<(DecayError, &'static str, SidereonStatus)> = vec![
            (
                DecayError::Propagation(PropagationError::NumericalFailure(
                    "underflow".to_string(),
                )),
                "propagation",
                SidereonStatus::Solve,
            ),
            (
                DecayError::Propagation(PropagationError::InvalidInput("bad state".to_string())),
                "propagation",
                SidereonStatus::Solve,
            ),
            (
                DecayError::Propagation(PropagationError::Ut1OutsideCoverage(
                    sidereon_core::astro::time::DegradeReason::AfterCoverage,
                )),
                "propagation",
                SidereonStatus::Solve,
            ),
            (
                DecayError::InvalidConfig("scan_step_s"),
                "invalid_config",
                SidereonStatus::InvalidArgument,
            ),
            (
                DecayError::NoDecayWithinHorizon { horizon_s: 7200.0 },
                "no_decay_within_horizon",
                SidereonStatus::Solve,
            ),
            (
                DecayError::ScanBudgetExhausted {
                    scanned_s: 1800.0,
                    samples: 30,
                },
                "scan_budget_exhausted",
                SidereonStatus::Solve,
            ),
        ];

        for (err, expected_kind, expected_status) in cases {
            let val = decay_error_value(&err);
            assert_eq!(val["kind"], json!(expected_kind));
            match &err {
                DecayError::Propagation(source) => {
                    assert_eq!(
                        val["fields"]["cause"],
                        crate::covariance::covariance_propagation_error_value(source)
                    );
                }
                DecayError::InvalidConfig(field) => {
                    assert_eq!(val["fields"]["field"], json!(field));
                    assert_eq!(val["fields"]["reason"], json!(field));
                }
                DecayError::NoDecayWithinHorizon { horizon_s } => {
                    assert_eq!(val["fields"]["horizon_s"]["decimal"], json!("7200"));
                    assert_eq!(
                        val["fields"]["horizon_s"]["bits_hex"],
                        json!(format!("{:016x}", horizon_s.to_bits()))
                    );
                }
                DecayError::ScanBudgetExhausted { scanned_s, samples } => {
                    assert_eq!(val["fields"]["scanned_s"]["decimal"], json!("1800"));
                    assert_eq!(
                        val["fields"]["scanned_s"]["bits_hex"],
                        json!(format!("{:016x}", scanned_s.to_bits()))
                    );
                    assert_eq!(val["fields"]["samples"], json!(samples));
                }
            }

            let status = map_decay_error("test_decay_op", err);
            assert_eq!(status, expected_status);

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                unsafe { sidereon_last_engine_error_info(&mut info) },
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Decay);
            assert!(info.payload_len > 0);
            clear_engine_error();
        }
    }

    fn assert_retained_error(expected_family: SidereonEngineErrorFamily, expected_payload: &[u8]) {
        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, expected_family);
        assert_eq!(info.payload_len, expected_payload.len());
        let mut buf = vec![0u8; expected_payload.len()];
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, expected_payload.len());
        assert_eq!(required, expected_payload.len());
        assert_eq!(buf, expected_payload);
    }

    #[test]
    fn decay_estimation_real_public_refusal_and_valid_control() {
        clear_engine_error();

        let config = SidereonDecayConfig {
            force_model: SidereonPropagationForceModel::TwoBodyJ2 as u32,
            integrator: SidereonPropagationIntegrator::Dp54 as u32,
            abs_tol: 1e-8,
            rel_tol: 1e-8,
            initial_step_s: 60.0,
            min_step_s: 0.1,
            max_step_s: 120.0,
            max_steps: 1000,
            mu_km3_s2_enabled: false,
            mu_km3_s2: MU_EARTH,
            drag: drag_parameters_to_c(
                DragParameters::from_bc_factor_m2_kg(
                    0.01,
                    SpaceWeather::default(),
                    DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
                )
                .unwrap(),
            ),
            reentry_altitude_km: 100.0,
            scan_step_s: 60.0,
            crossing_tolerance_s: 1.0,
            max_duration_s: 86400.0,
            max_scan_samples: 1000,
        };

        // 1. Valid control: initial state at radius 6400 km (altitude ~21.8 km <= reentry 100 km)
        // Returns immediately with 0s to decay, no propagation needed
        let initial_low = SidereonCartesianState {
            epoch_s: 0.0,
            position_km: [6400.0, 0.0, 0.0],
            velocity_km_s: [0.0, 7.5, 0.0],
        };
        let mut estimate = SidereonDecayEstimate {
            time_to_decay_s: -1.0,
            reentry_state: initial_low,
            reentry_altitude_km: -1.0,
        };
        let status = unsafe { sidereon_estimate_decay(&initial_low, &config, &mut estimate) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(estimate.time_to_decay_s, 0.0);

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Decay,
            payload_len: 999,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 2. Real bounded public refusal: invalid config (scan_step_s <= 0)
        let mut bad_config = config;
        bad_config.scan_step_s = -10.0;
        let status = unsafe { sidereon_estimate_decay(&initial_low, &bad_config, &mut estimate) };
        assert_eq!(status, SidereonStatus::InvalidArgument);

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Decay);
        assert!(info.payload_len > 0);
        let expected_len = info.payload_len;

        // Two-pass payload retrieval: Pass 1 query length
        let mut payload_written = 999usize;
        let mut payload_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    ptr::null_mut(),
                    0,
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(payload_written, 0);
        assert_eq!(payload_required, expected_len);

        // Short-buffer check: InvalidArgument, 0 written, required set, no copy
        let mut short_buf = vec![0u8; expected_len - 1];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(payload_written, 0);
        assert_eq!(payload_required, expected_len);

        // Pass 2: full exact buffer retrieval
        let mut payload_buf = vec![0u8; expected_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    payload_buf.as_mut_ptr(),
                    payload_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(payload_written, expected_len);
        assert_eq!(payload_required, expected_len);

        let parsed: serde_json::Value =
            serde_json::from_slice(&payload_buf).expect("valid JSON payload");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["family"], "decay");
        assert_eq!(parsed["operation"], "sidereon_estimate_decay");
        assert_eq!(parsed["error"]["kind"], "invalid_config");
        assert_eq!(parsed["error"]["fields"]["field"], "scan_step_s");

        // 3. Real bounded public refusal: NoDecayWithinHorizon
        let initial_high = SidereonCartesianState {
            epoch_s: 0.0,
            position_km: [7000.0, 0.0, 0.0],
            velocity_km_s: [0.0, 7.5, 0.0],
        };
        let mut horizon_config = config;
        horizon_config.max_duration_s = 1.0;
        horizon_config.scan_step_s = 2.0;
        let status =
            unsafe { sidereon_estimate_decay(&initial_high, &horizon_config, &mut estimate) };
        assert_eq!(status, SidereonStatus::Solve);

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Decay);
        let mut horizon_buf = vec![0u8; info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    horizon_buf.as_mut_ptr(),
                    horizon_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        let horizon_parsed: serde_json::Value =
            serde_json::from_slice(&horizon_buf).expect("valid JSON payload");
        assert_eq!(horizon_parsed["error"]["kind"], "no_decay_within_horizon");
        assert_eq!(
            horizon_parsed["error"]["fields"]["horizon_s"]["decimal"],
            json!("1")
        );

        // 4. Real bounded public refusal: ScanBudgetExhausted
        let mut budget_config = config;
        budget_config.max_duration_s = 100.0;
        budget_config.scan_step_s = 1.0;
        budget_config.max_scan_samples = 1;
        let status =
            unsafe { sidereon_estimate_decay(&initial_high, &budget_config, &mut estimate) };
        assert_eq!(status, SidereonStatus::Solve);

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Decay);
        let mut budget_buf = vec![0u8; info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    budget_buf.as_mut_ptr(),
                    budget_buf.len(),
                    &mut payload_written,
                    &mut payload_required,
                )
            },
            SidereonStatus::Ok
        );
        let budget_parsed: serde_json::Value =
            serde_json::from_slice(&budget_buf).expect("valid JSON payload");
        assert_eq!(budget_parsed["error"]["kind"], "scan_budget_exhausted");
        assert_eq!(budget_parsed["error"]["fields"]["samples"], json!(1));

        // 5. Early-null reset
        let status = unsafe { sidereon_estimate_decay(&initial_low, &config, ptr::null_mut()) };
        assert_eq!(status, SidereonStatus::NullPointer);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 6. Active error retained through sidereon_decay_config_init
        let _ = unsafe { sidereon_estimate_decay(&initial_low, &bad_config, &mut estimate) };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Decay);

        let mut init_cfg: SidereonDecayConfig = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { sidereon_decay_config_init(&mut init_cfg) },
            SidereonStatus::Ok
        );
        assert_retained_error(SidereonEngineErrorFamily::Decay, &payload_buf);

        // 7. Successful-producer invalidation: seeded refusal cleared by short valid producer
        let status = unsafe { sidereon_estimate_decay(&initial_low, &config, &mut estimate) };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(estimate.time_to_decay_s, 0.0);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        clear_engine_error();
    }
}
