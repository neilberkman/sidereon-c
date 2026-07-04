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
    ffi_boundary(
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
    ffi_boundary(
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
            set_last_error(format!("{fn_name}: {err}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
