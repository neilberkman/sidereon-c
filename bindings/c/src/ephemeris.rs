use super::*;
use crate::engine_error::engine_error_operation_boundary;

/// Numerical Cartesian-state ephemeris. Opaque to C. Create with
/// sidereon_propagate_state and release with sidereon_ephemeris_free.
pub struct SidereonEphemeris {
    pub(crate) times_s: Vec<f64>,
    pub(crate) states: Vec<CartesianState>,
}

/// Numerical propagation force-model selector. Stored in
/// SidereonStatePropagationConfig.force_model as a uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPropagationForceModel {
    /// Point-mass two-body gravity.
    TwoBody = 0,
    /// Two-body gravity plus Earth J2 oblateness.
    TwoBodyJ2 = 1,
    /// Additive force components from SidereonForceModelComponents.
    Composite = 2,
    /// Canonical Earth Phase A force set with optional SRP parameters.
    EarthPhaseA = 3,
    /// Canonical Earth Phase B force set with spherical-harmonic gravity.
    EarthPhaseB = 4,
}

/// Numerical propagation integrator selector. Stored in
/// SidereonStatePropagationConfig.integrator as a uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPropagationIntegrator {
    /// Dormand-Prince 5(4) adaptive integrator.
    Dp54 = 0,
    /// Fixed-step fourth-order Runge-Kutta integrator.
    Rk4 = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGravityTideSystem {
    TideFree = 0,
    ZeroTide = 1,
    MeanTide = 2,
}

/// Cannonball solar-radiation-pressure spacecraft parameters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSolarRadiationPressure {
    /// Reflectivity coefficient C_R.
    pub cr: f64,
    /// Spacecraft area-to-mass ratio A/m, square meters per kilogram.
    pub area_to_mass_m2_kg: f64,
}

/// Additive force components for composite numerical propagation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonForceModelComponents {
    /// Whether to include central two-body gravity.
    pub has_two_body: bool,
    /// Whether two_body_mu_km3_s2 overrides the config-level mu for two-body gravity.
    pub two_body_mu_km3_s2_enabled: bool,
    /// Two-body gravitational parameter in km^3/s^2 when enabled.
    pub two_body_mu_km3_s2: f64,
    /// Whether to include Earth zonal gravity.
    pub has_zonal: bool,
    /// Highest active zonal degree, in the inclusive range 2 through 6.
    pub zonal_max_degree: u32,
    /// Whether to include Earth spherical-harmonic gravity.
    pub has_spherical_harmonic: bool,
    /// Spherical-harmonic maximum degree.
    pub spherical_harmonic_max_degree: u32,
    /// Spherical-harmonic maximum order.
    pub spherical_harmonic_max_order: u32,
    /// Whether to include solid-Earth tide gravity.
    pub has_solid_earth_tide: bool,
    /// Whether to include solid-Earth pole-tide gravity.
    pub has_solid_earth_pole_tide: bool,
    /// Whether to include third-body gravity.
    pub has_third_body: bool,
    /// Include the Sun in third-body gravity.
    pub third_body_sun: bool,
    /// Include the Moon in third-body gravity.
    pub third_body_moon: bool,
    /// Whether to include cannonball solar radiation pressure.
    pub has_solar_radiation_pressure: bool,
    /// Solar-radiation-pressure parameters when enabled.
    pub solar_radiation_pressure: SidereonSolarRadiationPressure,
    /// Whether to include the geocentric Schwarzschild correction.
    pub has_relativity: bool,
}

/// Numerical state propagation controls. Initialize with
/// sidereon_state_propagation_config_init before overriding fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStatePropagationConfig {
    /// Initial epoch in absolute TDB seconds.
    pub epoch_s: f64,
    /// Initial ECI position in kilometers.
    pub position_km: [f64; 3],
    /// Initial ECI velocity in kilometers per second.
    pub velocity_km_s: [f64; 3],
    /// One of SidereonPropagationForceModel_* encoded as uint32_t.
    pub force_model: u32,
    /// One of SidereonPropagationIntegrator_* encoded as uint32_t.
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
    /// Maximum integration steps.
    pub max_steps: u32,
    /// Whether mu_km3_s2 overrides the engine default Earth value.
    pub mu_km3_s2_enabled: bool,
    /// Gravitational parameter in km^3/s^2 when enabled.
    pub mu_km3_s2: f64,
    /// Whether drag is enabled.
    pub has_drag: bool,
    /// Drag parameters when has_drag is true.
    pub drag: SidereonDragParameters,
    /// Additive force components used when force_model is Composite or the SRP
    /// component is requested for EarthPhaseA.
    pub force_components: SidereonForceModelComponents,
}

/// Initialize numerical state propagation config with engine defaults.
///
/// Safety: out_config must point to a SidereonStatePropagationConfig.
#[no_mangle]
pub unsafe extern "C" fn sidereon_state_propagation_config_init(
    out_config: *mut SidereonStatePropagationConfig,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_state_propagation_config_init",
        SidereonStatus::Panic,
        || {
            let out_config = c_try!(require_out(
                out_config,
                "sidereon_state_propagation_config_init",
                "out_config"
            ));
            *out_config = default_state_propagation_config();
            SidereonStatus::Ok
        },
    )
}

/// Numerically propagate an ECI Cartesian state and sample it at times_s. On
/// success writes a newly owned ephemeris handle to *out_ephemeris. Release it
/// with sidereon_ephemeris_free.
///
/// Safety: config must point to a SidereonStatePropagationConfig; times_s must
/// point to time_count doubles; out_ephemeris must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_state(
    config: *const SidereonStatePropagationConfig,
    times_s: *const f64,
    time_count: usize,
    out_ephemeris: *mut *mut SidereonEphemeris,
) -> SidereonStatus {
    const FN: &str = "sidereon_propagate_state";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_ephemeris = c_try!(require_out(out_ephemeris, FN, "out_ephemeris"));
        *out_ephemeris = ptr::null_mut();
        let config = c_try!(require_ref(config, FN, "config"));
        let times = c_try!(times_from_c(FN, times_s, time_count,));
        let propagator = c_try!(state_propagator_from_c(FN, config));
        let context = propagation_context_from_c(config);
        let states = match propagator.ephemeris_with_context(times, &context) {
            Ok(states) => states,
            Err(err) => {
                return map_state_propagation_error(FN, err);
            }
        };
        write_boxed_handle(
            out_ephemeris,
            SidereonEphemeris {
                times_s: times.to_vec(),
                states,
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_state_with_tide_system(
    config: *const SidereonStatePropagationConfig,
    gravity_tide_system: u32,
    times_s: *const f64,
    time_count: usize,
    out_ephemeris: *mut *mut SidereonEphemeris,
) -> SidereonStatus {
    const FN: &str = "sidereon_propagate_state_with_tide_system";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        let out_ephemeris = c_try!(require_out(out_ephemeris, FN, "out_ephemeris"));
        *out_ephemeris = ptr::null_mut();
        let config = c_try!(require_ref(config, FN, "config"));
        let tide_system = c_try!(super::gravity_tide_system_from_c(FN, gravity_tide_system));
        let times = c_try!(times_from_c(FN, times_s, time_count));
        let propagator = c_try!(state_propagator_from_c_with_tide_system(
            FN,
            config,
            tide_system
        ));
        let context = propagation_context_from_c(config);
        let states = match propagator.ephemeris_with_context(times, &context) {
            Ok(states) => states,
            Err(err) => {
                return map_state_propagation_error(FN, err);
            }
        };
        write_boxed_handle(
            out_ephemeris,
            SidereonEphemeris {
                times_s: times.to_vec(),
                states,
            },
        );
        SidereonStatus::Ok
    })
}

/// Write the number of epochs in a numerical ephemeris to *out_count.
///
/// Safety: ephemeris must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ephemeris_epoch_count(
    ephemeris: *const SidereonEphemeris,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ephemeris_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ephemeris_epoch_count",
                "out_count"
            ));
            *out_count = 0;
            let ephemeris = c_try!(require_ref(
                ephemeris,
                "sidereon_ephemeris_epoch_count",
                "ephemeris"
            ));
            *out_count = ephemeris.states.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy output epochs from a numerical ephemeris. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: ephemeris must be a live handle; out must point to at least len
/// writable doubles or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ephemeris_times_s(
    ephemeris: *const SidereonEphemeris,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ephemeris_times_s", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ephemeris_times_s",
            out_written,
            out_required
        ));
        let ephemeris = c_try!(require_ref(
            ephemeris,
            "sidereon_ephemeris_times_s",
            "ephemeris"
        ));
        c_try!(copy_prefix_to_c(
            "sidereon_ephemeris_times_s",
            "out",
            &ephemeris.times_s,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy ECI Cartesian states from a numerical ephemeris. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: ephemeris must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ephemeris_states(
    ephemeris: *const SidereonEphemeris,
    out: *mut SidereonCartesianState,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ephemeris_states", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ephemeris_states",
            out_written,
            out_required
        ));
        let ephemeris = c_try!(require_ref(
            ephemeris,
            "sidereon_ephemeris_states",
            "ephemeris"
        ));
        let states: Vec<SidereonCartesianState> =
            ephemeris.states.iter().map(cartesian_state_to_c).collect();
        c_try!(copy_prefix_to_c(
            "sidereon_ephemeris_states",
            "out",
            &states,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release a numerical ephemeris handle. Null is a no-op. A non-null handle
/// must come from sidereon_propagate_state and must be freed exactly once with
/// this function.
///
/// Safety: ephemeris must be NULL or a live handle from
/// sidereon_propagate_state. Passing a handle after it has already been freed
/// is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ephemeris_free(ephemeris: *mut SidereonEphemeris) {
    ffi_boundary("sidereon_ephemeris_free", (), || {
        free_boxed(ephemeris);
    });
}

fn map_state_propagation_error(fn_name: &str, err: PropagationError) -> SidereonStatus {
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::Propagation,
        fn_name,
        crate::covariance::covariance_propagation_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::Solve
}

fn default_state_propagation_config() -> SidereonStatePropagationConfig {
    let options = IntegratorOptions::default();
    let drag = DragParameters::from_bc_factor_m2_kg(
        0.01,
        SpaceWeather::default(),
        DragForce::DEFAULT_REENTRY_ALTITUDE_KM,
    )
    .expect("default drag parameters are valid");
    SidereonStatePropagationConfig {
        epoch_s: 0.0,
        position_km: [0.0; 3],
        velocity_km_s: [0.0; 3],
        force_model: SidereonPropagationForceModel::TwoBody as u32,
        integrator: SidereonPropagationIntegrator::Dp54 as u32,
        abs_tol: options.abs_tol,
        rel_tol: options.rel_tol,
        initial_step_s: options.initial_step,
        min_step_s: options.min_step,
        max_step_s: options.max_step,
        max_steps: options.max_steps,
        mu_km3_s2_enabled: false,
        mu_km3_s2: MU_EARTH,
        has_drag: false,
        drag: drag_parameters_to_c(drag),
        force_components: default_force_model_components(),
    }
}

fn default_force_model_components() -> SidereonForceModelComponents {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use std::ptr;

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
    fn state_propagation_real_public_refusal_and_valid_control() {
        clear_engine_error();

        let mut config: SidereonStatePropagationConfig = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { sidereon_state_propagation_config_init(&mut config) },
            SidereonStatus::Ok
        );
        config.epoch_s = 0.0;
        config.position_km = [7000.0, 0.0, 0.0];
        config.velocity_km_s = [0.0, 7.5, 0.0];
        config.force_model = SidereonPropagationForceModel::TwoBody as u32;
        config.integrator = SidereonPropagationIntegrator::Dp54 as u32;
        config.initial_step_s = 60.0;
        config.min_step_s = 0.1;
        config.max_step_s = 120.0;
        config.abs_tol = 1e-8;
        config.rel_tol = 1e-8;

        // 1. Valid control: propagate state with monotonic epochs
        let good_epochs = [0.0, 10.0];
        let mut ephem: *mut SidereonEphemeris = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_state(&config, good_epochs.as_ptr(), good_epochs.len(), &mut ephem)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!ephem.is_null());

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Propagation,
            payload_len: 999,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 2. Real bounded public refusal: non-finite epoch
        let bad_epochs = [f64::NAN];
        let mut bad_ephem: *mut SidereonEphemeris = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_state(
                &config,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                &mut bad_ephem,
            )
        };
        assert_eq!(status, SidereonStatus::Solve);
        assert!(bad_ephem.is_null());

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
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
        assert_eq!(parsed["family"], "propagation");
        assert_eq!(parsed["operation"], "sidereon_propagate_state");
        assert_eq!(parsed["error"]["kind"], "invalid_input");
        assert_eq!(
            parsed["error"]["fields"]["message"],
            "epochs_tdb_seconds not finite"
        );
        assert_eq!(
            parsed["error"]["fields"]["reason"],
            "epochs_tdb_seconds not finite"
        );

        // 3. Inspection getters and handle retention
        let mut count = 0usize;
        assert_eq!(
            unsafe { sidereon_ephemeris_epoch_count(ephem, &mut count) },
            SidereonStatus::Ok
        );
        assert_eq!(count, 2);
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        let mut times_out = [0.0; 2];
        let mut times_written = 0usize;
        let mut times_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_ephemeris_times_s(
                    ephem,
                    times_out.as_mut_ptr(),
                    times_out.len(),
                    &mut times_written,
                    &mut times_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(times_written, 2);
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        let mut states_out = [SidereonCartesianState {
            epoch_s: 0.0,
            position_km: [0.0; 3],
            velocity_km_s: [0.0; 3],
        }; 2];
        let mut states_written = 0usize;
        let mut states_required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_ephemeris_states(
                    ephem,
                    states_out.as_mut_ptr(),
                    states_out.len(),
                    &mut states_written,
                    &mut states_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(states_written, 2);
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        // Config init retains error
        let mut dummy_config: SidereonStatePropagationConfig = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { sidereon_state_propagation_config_init(&mut dummy_config) },
            SidereonStatus::Ok
        );
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        // Free ephemeris retains error
        unsafe { sidereon_ephemeris_free(ephem) };
        assert_retained_error(SidereonEngineErrorFamily::Propagation, &payload_buf);

        // 4. Successful-producer invalidation: seeded refusal cleared by short valid producer
        let mut seeded_ephem: *mut SidereonEphemeris = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_state(
                &config,
                good_epochs.as_ptr(),
                good_epochs.len(),
                &mut seeded_ephem,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!seeded_ephem.is_null());
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);
        unsafe { sidereon_ephemeris_free(seeded_ephem) };

        // 5. Early-null reset
        let mut dummy_ephem: *mut SidereonEphemeris = ptr::null_mut();
        let status = unsafe {
            sidereon_propagate_state(
                &config,
                bad_epochs.as_ptr(),
                bad_epochs.len(),
                &mut dummy_ephem,
            )
        };
        assert_eq!(status, SidereonStatus::Solve);
        assert!(dummy_ephem.is_null());
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(info.payload_len > 0);

        let status = unsafe {
            sidereon_propagate_state(
                &config,
                good_epochs.as_ptr(),
                good_epochs.len(),
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        clear_engine_error();
    }
}
