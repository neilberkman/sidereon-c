use super::*;

// --- Initial orbit determination (sidereon_core::astro::iod) ------------------

/// Gibbs three-position initial orbit determination. Writes the middle-epoch
/// velocity (km/s) to out_v2_km_s and the inter-vector angles (radians) to the
/// out_* scalars. Delegates to sidereon_core::astro::iod::gibbs.
///
/// Safety: r1/r2/r3_km point to 3 doubles each; out_v2_km_s points to 3 doubles;
/// the out angle pointers point to doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iod_gibbs(
    r1_km: *const f64,
    r2_km: *const f64,
    r3_km: *const f64,
    out_v2_km_s: *mut f64,
    out_theta12_rad: *mut f64,
    out_theta23_rad: *mut f64,
    out_coplanar_rad: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_iod_gibbs", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_iod_gibbs",
            "out_v2_km_s",
            out_v2_km_s,
            3,
            &[0.0, 0.0, 0.0]
        ));
        let t12 = c_try!(require_out(
            out_theta12_rad,
            "sidereon_iod_gibbs",
            "out_theta12_rad"
        ));
        *t12 = 0.0;
        let t23 = c_try!(require_out(
            out_theta23_rad,
            "sidereon_iod_gibbs",
            "out_theta23_rad"
        ));
        *t23 = 0.0;
        let copa = c_try!(require_out(
            out_coplanar_rad,
            "sidereon_iod_gibbs",
            "out_coplanar_rad"
        ));
        *copa = 0.0;
        let r1 = c_try!(read_vec3("sidereon_iod_gibbs", "r1_km", r1_km));
        let r2 = c_try!(read_vec3("sidereon_iod_gibbs", "r2_km", r2_km));
        let r3 = c_try!(read_vec3("sidereon_iod_gibbs", "r3_km", r3_km));
        match sidereon_core::astro::iod::gibbs(&r1, &r2, &r3) {
            Ok((v2, theta12, theta23, copa_v)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_iod_gibbs",
                    "out_v2_km_s",
                    out_v2_km_s,
                    3,
                    &v2
                ));
                *t12 = theta12;
                *t23 = theta23;
                *copa = copa_v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_iod_gibbs", err),
        }
    })
}

/// Herrick-Gibbs three-position initial orbit determination, for closely spaced
/// epochs. jd1/jd2/jd3 are Julian days. Delegates to
/// sidereon_core::astro::iod::hgibbs.
///
/// Safety: as sidereon_iod_gibbs, with Julian-day scalars added.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iod_hgibbs(
    r1_km: *const f64,
    r2_km: *const f64,
    r3_km: *const f64,
    jd1: f64,
    jd2: f64,
    jd3: f64,
    out_v2_km_s: *mut f64,
    out_theta12_rad: *mut f64,
    out_theta23_rad: *mut f64,
    out_coplanar_rad: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_iod_hgibbs", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_iod_hgibbs",
            "out_v2_km_s",
            out_v2_km_s,
            3,
            &[0.0, 0.0, 0.0]
        ));
        let t12 = c_try!(require_out(
            out_theta12_rad,
            "sidereon_iod_hgibbs",
            "out_theta12_rad"
        ));
        *t12 = 0.0;
        let t23 = c_try!(require_out(
            out_theta23_rad,
            "sidereon_iod_hgibbs",
            "out_theta23_rad"
        ));
        *t23 = 0.0;
        let copa = c_try!(require_out(
            out_coplanar_rad,
            "sidereon_iod_hgibbs",
            "out_coplanar_rad"
        ));
        *copa = 0.0;
        let r1 = c_try!(read_vec3("sidereon_iod_hgibbs", "r1_km", r1_km));
        let r2 = c_try!(read_vec3("sidereon_iod_hgibbs", "r2_km", r2_km));
        let r3 = c_try!(read_vec3("sidereon_iod_hgibbs", "r3_km", r3_km));
        match sidereon_core::astro::iod::hgibbs(&r1, &r2, &r3, jd1, jd2, jd3) {
            Ok((v2, theta12, theta23, copa_v)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_iod_hgibbs",
                    "out_v2_km_s",
                    out_v2_km_s,
                    3,
                    &v2
                ));
                *t12 = theta12;
                *t23 = theta23;
                *copa = copa_v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_iod_hgibbs", err),
        }
    })
}

// --- Angles-only IOD (sidereon_core::astro::iod) -----------------------------

/// Gauss angles-only initial orbit determination. From three topocentric
/// right-ascension/declination observations (radians), their split Julian dates,
/// and the three site ECI position vectors (3x3, row i = observation i, km),
/// recover the middle observation's ECI position (km) and velocity (km/s).
/// Delegates to sidereon_core::astro::iod::gauss_angles.
///
/// Safety: decl/rtasc/jd/jdf point to 3 doubles each; rseci_km points to 9
/// doubles (row-major, row i = site i); out_position_km and out_velocity_km_s
/// point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iod_gauss_angles(
    decl_rad: *const f64,
    rtasc_rad: *const f64,
    jd: *const f64,
    jdf: *const f64,
    rseci_km: *const f64,
    out_position_km: *mut f64,
    out_velocity_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_iod_gauss_angles", SidereonStatus::Panic, || {
        let out_position_km = c_try!(require_out(
            out_position_km,
            "sidereon_iod_gauss_angles",
            "out_position_km"
        ));
        let out_position_km = out_position_km as *mut f64;
        let out_velocity_km_s = c_try!(require_out(
            out_velocity_km_s,
            "sidereon_iod_gauss_angles",
            "out_velocity_km_s"
        ));
        let out_velocity_km_s = out_velocity_km_s as *mut f64;
        for idx in 0..3 {
            *out_position_km.add(idx) = 0.0;
            *out_velocity_km_s.add(idx) = 0.0;
        }
        let decl = c_try!(read_vec3("sidereon_iod_gauss_angles", "decl_rad", decl_rad));
        let rtasc = c_try!(read_vec3(
            "sidereon_iod_gauss_angles",
            "rtasc_rad",
            rtasc_rad
        ));
        let jd = c_try!(read_vec3("sidereon_iod_gauss_angles", "jd", jd));
        let jdf = c_try!(read_vec3("sidereon_iod_gauss_angles", "jdf", jdf));
        let rseci = c_try!(read_mat3("sidereon_iod_gauss_angles", "rseci_km", rseci_km));
        match sidereon_core::astro::iod::gauss_angles(&decl, &rtasc, &jd, &jdf, &rseci) {
            Ok((position, velocity)) => {
                copy_vec3(out_position_km, position);
                copy_vec3(out_velocity_km_s, velocity);
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_iod_gauss_angles", err),
        }
    })
}
