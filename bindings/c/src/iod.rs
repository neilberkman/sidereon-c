use super::*;

fn map_iod_error(fn_name: &str, error: sidereon_core::astro::iod::IodError) -> SidereonStatus {
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::Iod,
        fn_name,
        crate::engine_error::iod_error_value(&error),
    );
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

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
            Err(err) => map_iod_error("sidereon_iod_gibbs", err),
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
            Err(err) => map_iod_error("sidereon_iod_hgibbs", err),
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
/// point to disjoint 3-double ranges. Inputs are copied before outputs are
/// initialized, so an input array may be used as an output array.
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
        if !out_position_km.is_null() && !out_velocity_km_s.is_null() {
            let outputs = [
                Some((
                    c_try!(super::checked_output_range(
                        "sidereon_iod_gauss_angles",
                        out_position_km,
                        3,
                        "out_position_km"
                    )),
                    "out_position_km",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        "sidereon_iod_gauss_angles",
                        out_velocity_km_s,
                        3,
                        "out_velocity_km_s"
                    )),
                    "out_velocity_km_s",
                )),
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                "sidereon_iod_gauss_angles",
                &outputs
            ));
        }
        // Snapshot every input before output initialization to preserve
        // documented in-place input/output use; defer errors to retain the
        // existing output-zeroing behavior.
        let inputs_result = (|| {
            let decl = read_vec3("sidereon_iod_gauss_angles", "decl_rad", decl_rad)?;
            let rtasc = read_vec3("sidereon_iod_gauss_angles", "rtasc_rad", rtasc_rad)?;
            let jd = read_vec3("sidereon_iod_gauss_angles", "jd", jd)?;
            let jdf = read_vec3("sidereon_iod_gauss_angles", "jdf", jdf)?;
            let rseci = read_mat3("sidereon_iod_gauss_angles", "rseci_km", rseci_km)?;
            Ok((decl, rtasc, jd, jdf, rseci))
        })();
        let out_position_km = c_try!(require_out(
            out_position_km,
            "sidereon_iod_gauss_angles",
            "out_position_km"
        ));
        let out_velocity_km_s = c_try!(require_out(
            out_velocity_km_s,
            "sidereon_iod_gauss_angles",
            "out_velocity_km_s"
        ));
        for idx in 0..3 {
            *out_position_km.add(idx) = 0.0;
            *out_velocity_km_s.add(idx) = 0.0;
        }
        let (decl, rtasc, jd, jdf, rseci) = c_try!(inputs_result);
        match sidereon_core::astro::iod::gauss_angles(&decl, &rtasc, &jd, &jdf, &rseci) {
            Ok((position, velocity)) => {
                copy_vec3(out_position_km, position);
                copy_vec3(out_velocity_km_s, velocity);
                SidereonStatus::Ok
            }
            Err(err) => map_iod_error("sidereon_iod_gauss_angles", err),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::sidereon_last_error_message;
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::ptr;

    fn assert_last_iod_error(operation: &str, kind: &str, message: &str) {
        unsafe {
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Iod);

            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            let mut payload = vec![0; required];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, required);
            assert_eq!(required, info.payload_len);
            let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "iod");
            assert_eq!(payload["operation"], operation);
            assert_eq!(payload["error"]["kind"], kind);
            assert_eq!(payload["error"]["fields"], serde_json::json!({}));
            let len = sidereon_last_error_message(ptr::null_mut(), 0);
            let mut text = vec![0 as c_char; len + 1];
            sidereon_last_error_message(text.as_mut_ptr(), text.len());
            let text = CStr::from_ptr(text.as_ptr()).to_string_lossy();
            assert_eq!(text, format!("{operation}: {message}"));
        }
    }

    #[test]
    fn public_iod_producers_return_typed_errors_for_degenerate_inputs() {
        let zero = [0.0; 3];
        let first = [0.0, 0.0, 6378.1363];
        let middle = [0.0, -4464.696, -5102.509];
        let last = [0.0, 5740.323, 3189.068];
        let (mut velocity, mut theta12, mut theta23, mut coplanarity) = ([0.0; 3], 0.0, 0.0, 0.0);

        assert_eq!(
            unsafe {
                sidereon_iod_gibbs(
                    zero.as_ptr(),
                    middle.as_ptr(),
                    last.as_ptr(),
                    velocity.as_mut_ptr(),
                    &mut theta12,
                    &mut theta23,
                    &mut coplanarity,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_last_iod_error(
            "sidereon_iod_gibbs",
            "zero_vector",
            "position vector has near-zero magnitude",
        );

        assert_eq!(
            unsafe {
                sidereon_iod_hgibbs(
                    first.as_ptr(),
                    middle.as_ptr(),
                    last.as_ptr(),
                    1.0,
                    1.0,
                    1.0,
                    velocity.as_mut_ptr(),
                    &mut theta12,
                    &mut theta23,
                    &mut coplanarity,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_last_iod_error(
            "sidereon_iod_hgibbs",
            "invalid_time_geometry",
            "observation times are equal or near-equal",
        );

        let angles = [0.0; 3];
        let jd = [2456159.5; 3];
        let jdf = [0.1, 0.2, 0.3];
        let sites = [
            4054.881, 2748.195, 4074.237, 3956.224, 2888.232, 4074.364, 3905.073, 2956.935,
            4074.430,
        ];
        let mut position = [0.0; 3];
        assert_eq!(
            unsafe {
                sidereon_iod_gauss_angles(
                    angles.as_ptr(),
                    angles.as_ptr(),
                    jd.as_ptr(),
                    jdf.as_ptr(),
                    sites.as_ptr(),
                    position.as_mut_ptr(),
                    velocity.as_mut_ptr(),
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_last_iod_error(
            "sidereon_iod_gauss_angles",
            "determinant_too_small",
            "line-of-sight determinant too small",
        );
    }

    #[test]
    fn public_gibbs_producer_retains_the_vallado_reference_result() {
        let r1 = [0.0, 0.0, 6378.1363];
        let r2 = [0.0, -4464.696, -5102.509];
        let r3 = [0.0, 5740.323, 3189.068];
        let (mut velocity, mut theta12, mut theta23, mut coplanarity) = ([0.0; 3], 0.0, 0.0, 0.0);
        assert_eq!(
            unsafe {
                sidereon_iod_gibbs(
                    r1.as_ptr(),
                    r2.as_ptr(),
                    r3.as_ptr(),
                    velocity.as_mut_ptr(),
                    &mut theta12,
                    &mut theta23,
                    &mut coplanarity,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(velocity, [0.0, 5.5311472050176125, -5.191806413494606]);
        assert!((theta12.to_degrees() - 138.81407085944375).abs() < 1e-9);
        assert!((theta23.to_degrees() - 160.24053069723146).abs() < 1e-9);
        assert!(coplanarity.abs() < 1e-9);
    }
}
