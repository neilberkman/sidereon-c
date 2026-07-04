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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
    ffi_boundary(
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
                    set_last_error(format!("sidereon_drag_force_acceleration: {err}"));
                    SidereonStatus::InvalidArgument
                }
                Err(err) => {
                    set_last_error(format!("sidereon_drag_force_acceleration: {err}"));
                    SidereonStatus::Solve
                }
            }
        },
    )
}
