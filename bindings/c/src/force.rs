use super::*;

/// Two-body point-mass acceleration in km/s^2. Delegates to
/// sidereon_core::astro::forces::TwoBodyGravity::acceleration.
///
/// Safety: position_km and velocity_km_s point to 3 doubles each; out_accel
/// points to 3 writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_twobody_acceleration(
    position_km: *const f64,
    velocity_km_s: *const f64,
    out_accel: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_force_twobody_acceleration",
        SidereonStatus::Panic,
        || {
            let position_km = c_try!(read_vec3(
                "sidereon_force_twobody_acceleration",
                "position_km",
                position_km
            ));
            let velocity_km_s = c_try!(read_vec3(
                "sidereon_force_twobody_acceleration",
                "velocity_km_s",
                velocity_km_s
            ));
            let accel = c_try!(force_acceleration(
                "sidereon_force_twobody_acceleration",
                &TwoBodyGravity::default(),
                position_km,
                velocity_km_s,
            ));
            c_try!(copy_exact_f64s(
                "sidereon_force_twobody_acceleration",
                "out_accel",
                out_accel,
                3,
                &accel,
            ));
            SidereonStatus::Ok
        },
    )
}

/// J2 oblateness perturbing acceleration in km/s^2. Delegates to
/// sidereon_core::astro::forces::J2Gravity::acceleration.
///
/// Safety: position_km and velocity_km_s point to 3 doubles each; out_accel
/// points to 3 writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_force_j2_acceleration(
    position_km: *const f64,
    velocity_km_s: *const f64,
    out_accel: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_force_j2_acceleration",
        SidereonStatus::Panic,
        || {
            let position_km = c_try!(read_vec3(
                "sidereon_force_j2_acceleration",
                "position_km",
                position_km
            ));
            let velocity_km_s = c_try!(read_vec3(
                "sidereon_force_j2_acceleration",
                "velocity_km_s",
                velocity_km_s
            ));
            let accel = c_try!(force_acceleration(
                "sidereon_force_j2_acceleration",
                &J2Gravity::default(),
                position_km,
                velocity_km_s,
            ));
            c_try!(copy_exact_f64s(
                "sidereon_force_j2_acceleration",
                "out_accel",
                out_accel,
                3,
                &accel,
            ));
            SidereonStatus::Ok
        },
    )
}

// --- Astro force models, Doppler, covariance, and public time metadata -------

fn force_acceleration(
    fn_name: &str,
    force: &dyn ForceModel,
    position_km: [f64; 3],
    velocity_km_s: [f64; 3],
) -> Result<[f64; 3], SidereonStatus> {
    let state = CartesianState::new(0.0, position_km, velocity_km_s);
    match force.acceleration(&state, &PropagationContext::default()) {
        Ok(accel) => Ok([accel.x, accel.y, accel.z]),
        Err(err) => {
            crate::engine_error::record_engine_error(
                crate::engine_error::SidereonEngineErrorFamily::Propagation,
                fn_name,
                crate::orbit_fit::propagation_error_value(&err),
            );
            set_last_error(format!("{fn_name}: {err}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

#[cfg(test)]
mod propagation_error_tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, snapshot_engine_error_for_test, SidereonEngineErrorFamily,
    };

    #[test]
    fn force_producers_record_full_failures_succeed_and_clear_before_null_outputs() {
        type ForceFn = unsafe extern "C" fn(*const f64, *const f64, *mut f64) -> SidereonStatus;
        let zero = [0.0; 3];
        let position = [7000.0, 10.0, 20.0];
        let velocity = [0.0, 7.5, 1.0];
        let mut output = [0.0; 3];

        for (operation, call) in [
            (
                "sidereon_force_twobody_acceleration",
                sidereon_force_twobody_acceleration as ForceFn,
            ),
            (
                "sidereon_force_j2_acceleration",
                sidereon_force_j2_acceleration as ForceFn,
            ),
        ] {
            clear_engine_error();
            assert_eq!(
                unsafe { call(zero.as_ptr(), velocity.as_ptr(), output.as_mut_ptr()) },
                SidereonStatus::InvalidArgument
            );
            let (info, payload) = snapshot_engine_error_for_test().expect("force refusal payload");
            assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
            assert!(payload.contains(&format!("\"operation\":\"{operation}\"")));
            assert!(payload.contains("\"kind\":\"numerical_failure\""));
            assert!(payload.contains("Zero position magnitude"));

            let mut message = vec![0 as std::ffi::c_char; 160];
            unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
            let message = unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }.to_string_lossy();
            assert_eq!(
                message,
                format!("{operation}: Numerical failure: Zero position magnitude")
            );

            assert_eq!(
                unsafe { call(position.as_ptr(), velocity.as_ptr(), output.as_mut_ptr()) },
                SidereonStatus::Ok
            );
            assert!(snapshot_engine_error_for_test().is_none());

            assert_eq!(
                unsafe { call(zero.as_ptr(), velocity.as_ptr(), output.as_mut_ptr()) },
                SidereonStatus::InvalidArgument
            );
            assert!(snapshot_engine_error_for_test().is_some());

            assert_eq!(
                unsafe { call(position.as_ptr(), velocity.as_ptr(), ptr::null_mut()) },
                SidereonStatus::NullPointer
            );
            assert!(snapshot_engine_error_for_test().is_none());
        }
        clear_engine_error();
    }
}
