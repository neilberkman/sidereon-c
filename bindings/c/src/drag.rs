use super::*;

/// Validated drag parameters stored on propagation and decay configs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDragParameters {
    /// Drag factor B = C_D * A / m, m^2/kg.
    pub bc_factor_m2_kg: f64,
    /// Space-weather inputs.
    pub space_weather: SidereonSpaceWeather,
    /// Density cutoff altitude, km.
    pub cutoff_altitude_km: f64,
}

/// Build validated drag parameters from drag coefficient, area, and mass.
///
/// Safety: out_drag must point to a SidereonDragParameters.
#[no_mangle]
pub unsafe extern "C" fn sidereon_drag_parameters_from_area_mass(
    cd: f64,
    area_m2: f64,
    mass_kg: f64,
    weather: SidereonSpaceWeather,
    cutoff_altitude_km: f64,
    out_drag: *mut SidereonDragParameters,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_drag_parameters_from_area_mass",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_drag,
                "sidereon_drag_parameters_from_area_mass",
                "out_drag"
            ));
            match DragParameters::from_area_mass(
                cd,
                area_m2,
                mass_kg,
                space_weather_from_c(weather),
                cutoff_altitude_km,
            ) {
                Ok(value) => {
                    *out = drag_parameters_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Propagation,
                        "sidereon_drag_parameters_from_area_mass",
                        crate::orbit_fit::propagation_error_value(&err),
                    );
                    set_last_error(format!("sidereon_drag_parameters_from_area_mass: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Build validated drag parameters from B = C_D * A / m in m^2/kg.
///
/// Safety: out_drag must point to a SidereonDragParameters.
#[no_mangle]
pub unsafe extern "C" fn sidereon_drag_parameters_from_bc_factor(
    bc_factor_m2_kg: f64,
    weather: SidereonSpaceWeather,
    cutoff_altitude_km: f64,
    out_drag: *mut SidereonDragParameters,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_drag_parameters_from_bc_factor",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_drag,
                "sidereon_drag_parameters_from_bc_factor",
                "out_drag"
            ));
            match DragParameters::from_bc_factor_m2_kg(
                bc_factor_m2_kg,
                space_weather_from_c(weather),
                cutoff_altitude_km,
            ) {
                Ok(value) => {
                    *out = drag_parameters_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Propagation,
                        "sidereon_drag_parameters_from_bc_factor",
                        crate::orbit_fit::propagation_error_value(&err),
                    );
                    set_last_error(format!("sidereon_drag_parameters_from_bc_factor: {err}"));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Build validated drag parameters from reciprocal ballistic coefficient.
///
/// Safety: out_drag must point to a SidereonDragParameters.
#[no_mangle]
pub unsafe extern "C" fn sidereon_drag_parameters_from_ballistic_coefficient(
    bc_kg_m2: f64,
    weather: SidereonSpaceWeather,
    cutoff_altitude_km: f64,
    out_drag: *mut SidereonDragParameters,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_drag_parameters_from_ballistic_coefficient",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_drag,
                "sidereon_drag_parameters_from_ballistic_coefficient",
                "out_drag"
            ));
            match DragParameters::from_ballistic_coefficient(
                bc_kg_m2,
                space_weather_from_c(weather),
                cutoff_altitude_km,
            ) {
                Ok(value) => {
                    *out = drag_parameters_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Propagation,
                        "sidereon_drag_parameters_from_ballistic_coefficient",
                        crate::orbit_fit::propagation_error_value(&err),
                    );
                    set_last_error(format!(
                        "sidereon_drag_parameters_from_ballistic_coefficient: {err}"
                    ));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Evaluate atmospheric-drag acceleration for one ECI Cartesian state.
///
/// Safety: drag and state must point to valid structs; out_accel_km_s2 must
/// point to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_drag_force_acceleration(
    drag: *const SidereonDragParameters,
    state: *const SidereonCartesianState,
    out_accel_km_s2: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_drag_force_acceleration",
        SidereonStatus::Panic,
        || {
            c_try!(require_out(
                out_accel_km_s2,
                "sidereon_drag_force_acceleration",
                "out_accel_km_s2"
            ));
            zero_f64_prefix(out_accel_km_s2, 3, 3);
            let drag = c_try!(require_ref(
                drag,
                "sidereon_drag_force_acceleration",
                "drag"
            ));
            let state = c_try!(require_ref(
                state,
                "sidereon_drag_force_acceleration",
                "state"
            ));
            let params = c_try!(drag_parameters_from_c(
                "sidereon_drag_force_acceleration",
                *drag
            ));
            let force = params.to_force();
            match force.acceleration(
                &cartesian_state_from_c(state),
                &PropagationContext::default(),
            ) {
                Ok(accel) => {
                    c_try!(copy_exact_f64s(
                        "sidereon_drag_force_acceleration",
                        "out_accel_km_s2",
                        out_accel_km_s2,
                        3,
                        accel.as_slice(),
                    ));
                    SidereonStatus::Ok
                }
                Err(err @ PropagationError::InvalidInput(_)) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Propagation,
                        "sidereon_drag_force_acceleration",
                        crate::orbit_fit::propagation_error_value(&err),
                    );
                    set_last_error(format!("sidereon_drag_force_acceleration: {err}"));
                    SidereonStatus::InvalidArgument
                }
                Err(err) => {
                    crate::engine_error::record_engine_error(
                        crate::engine_error::SidereonEngineErrorFamily::Propagation,
                        "sidereon_drag_force_acceleration",
                        crate::orbit_fit::propagation_error_value(&err),
                    );
                    set_last_error(format!("sidereon_drag_force_acceleration: {err}"));
                    SidereonStatus::Solve
                }
            }
        },
    )
}

#[cfg(test)]
mod propagation_error_tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, snapshot_engine_error_for_test, SidereonEngineErrorFamily,
    };

    fn weather() -> SidereonSpaceWeather {
        SidereonSpaceWeather {
            f107: 150.0,
            f107a: 150.0,
            ap: 4.0,
        }
    }

    fn error_payload(operation: &str, expected_message: &str) -> String {
        let (info, payload) = snapshot_engine_error_for_test().expect("drag refusal payload");
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(payload.contains(&format!("\"operation\":\"{operation}\"")));
        assert!(payload.contains("\"kind\":\"invalid_input\""));
        assert!(payload.contains(expected_message));
        payload
    }

    fn assert_early_null_clears(call: impl FnOnce() -> SidereonStatus) {
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(call(), SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());
    }

    fn seed_real_drag_refusal(params: &mut SidereonDragParameters) {
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_area_mass(0.0, 1.0, 10.0, weather(), 100.0, params)
            },
            SidereonStatus::InvalidArgument
        );
        assert!(snapshot_engine_error_for_test().is_some());
    }

    #[test]
    fn drag_producers_record_real_refusals_and_reset_on_success_and_early_null() {
        clear_engine_error();
        let mut params = SidereonDragParameters {
            bc_factor_m2_kg: 0.0,
            space_weather: weather(),
            cutoff_altitude_km: 100.0,
        };

        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_area_mass(
                    1.0,
                    0.0,
                    10.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_parameters_from_area_mass",
            "area_m2 not positive",
        );
        let mut message = vec![0 as std::ffi::c_char; 160];
        unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }.to_string_lossy(),
            "sidereon_drag_parameters_from_area_mass: Invalid input: area_m2 not positive"
        );
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_area_mass(
                    0.0,
                    1.0,
                    10.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::InvalidArgument
        );
        error_payload("sidereon_drag_parameters_from_area_mass", "cd not positive");
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_area_mass(
                    1.0,
                    1.0,
                    0.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_parameters_from_area_mass",
            "mass_kg not positive",
        );
        assert_early_null_clears(|| unsafe {
            sidereon_drag_parameters_from_area_mass(
                1.0,
                1.0,
                10.0,
                weather(),
                100.0,
                ptr::null_mut(),
            )
        });
        seed_real_drag_refusal(&mut params);
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_area_mass(
                    1.0,
                    1.0,
                    10.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        assert_eq!(
            unsafe { sidereon_drag_parameters_from_bc_factor(0.0, weather(), 100.0, &mut params,) },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_parameters_from_bc_factor",
            "bc_factor_m2_kg not positive",
        );
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_bc_factor(
                    0.01,
                    SidereonSpaceWeather {
                        ap: -1.0,
                        ..weather()
                    },
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::InvalidArgument
        );
        error_payload("sidereon_drag_parameters_from_bc_factor", "ap negative");
        assert_eq!(
            unsafe { sidereon_drag_parameters_from_bc_factor(0.01, weather(), -1.0, &mut params) },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_parameters_from_bc_factor",
            "cutoff_altitude_km out of domain",
        );
        assert_early_null_clears(|| unsafe {
            sidereon_drag_parameters_from_bc_factor(0.01, weather(), 100.0, ptr::null_mut())
        });
        seed_real_drag_refusal(&mut params);
        assert_eq!(
            unsafe { sidereon_drag_parameters_from_bc_factor(0.01, weather(), 100.0, &mut params) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_ballistic_coefficient(
                    0.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_parameters_from_ballistic_coefficient",
            "bc_kg_m2 not positive",
        );
        assert_early_null_clears(|| unsafe {
            sidereon_drag_parameters_from_ballistic_coefficient(
                100.0,
                weather(),
                100.0,
                ptr::null_mut(),
            )
        });
        seed_real_drag_refusal(&mut params);
        assert_eq!(
            unsafe {
                sidereon_drag_parameters_from_ballistic_coefficient(
                    100.0,
                    weather(),
                    100.0,
                    &mut params,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let valid_state = SidereonCartesianState {
            epoch_s: 0.0,
            position_km: [6578.137, 0.0, 0.0],
            velocity_km_s: [0.0, 7.7, 0.0],
        };
        let invalid_state = SidereonCartesianState {
            epoch_s: f64::NAN,
            ..valid_state
        };
        let mut accel = [0.0; 3];
        assert_eq!(
            unsafe {
                sidereon_drag_force_acceleration(&params, &invalid_state, accel.as_mut_ptr())
            },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_force_acceleration",
            "epoch_tdb_seconds not finite",
        );
        let invalid_params = SidereonDragParameters {
            bc_factor_m2_kg: 0.0,
            ..params
        };
        assert_eq!(
            unsafe {
                sidereon_drag_force_acceleration(&invalid_params, &valid_state, accel.as_mut_ptr())
            },
            SidereonStatus::InvalidArgument
        );
        error_payload(
            "sidereon_drag_force_acceleration",
            "bc_factor_m2_kg not positive",
        );
        assert_early_null_clears(|| unsafe {
            sidereon_drag_force_acceleration(&params, &valid_state, ptr::null_mut())
        });
        seed_real_drag_refusal(&mut params);
        assert_eq!(
            unsafe { sidereon_drag_force_acceleration(&params, &valid_state, accel.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let zero_position = SidereonCartesianState {
            position_km: [0.0; 3],
            ..valid_state
        };
        assert_eq!(
            unsafe {
                sidereon_drag_force_acceleration(&params, &zero_position, accel.as_mut_ptr())
            },
            SidereonStatus::Solve
        );
        let (info, payload) = snapshot_engine_error_for_test().expect("numerical drag refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(payload.contains("\"kind\":\"numerical_failure\""));
        assert!(payload.contains("Zero position magnitude"));
        clear_engine_error();
    }
}
