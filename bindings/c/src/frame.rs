use super::*;

/// Convert a WGS84 geodetic position (latitude/longitude in radians, ellipsoidal
/// height in meters) to an ITRF/ECEF position in meters. Pure value in, value
/// out: no handle is allocated. Wraps the engine's single validated forward
/// converter; no conversion math lives here.
///
/// Safety: geodetic must point to a SidereonGeodetic; out_ecef must point to a
/// SidereonItrfPosition.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodetic_to_ecef(
    geodetic: *const SidereonGeodetic,
    out_ecef: *mut SidereonItrfPosition,
) -> SidereonStatus {
    ffi_boundary("sidereon_geodetic_to_ecef", SidereonStatus::Panic, || {
        let out_ecef = c_try!(require_out(
            out_ecef,
            "sidereon_geodetic_to_ecef",
            "out_ecef"
        ));
        *out_ecef = SidereonItrfPosition {
            x_m: 0.0,
            y_m: 0.0,
            z_m: 0.0,
        };
        let geodetic = c_try!(require_ref(
            geodetic,
            "sidereon_geodetic_to_ecef",
            "geodetic"
        ));
        let wgs84 = c_try!(Wgs84Geodetic::new(
            geodetic.lat_rad,
            geodetic.lon_rad,
            geodetic.height_m
        )
        .map_err(|err| {
            set_last_error(format!("sidereon_geodetic_to_ecef: {err}"));
            SidereonStatus::InvalidArgument
        }));
        let itrf = c_try!(geodetic_to_itrf(wgs84)
            .map_err(|err| map_frame_transform_error("sidereon_geodetic_to_ecef", err)));
        *out_ecef = SidereonItrfPosition {
            x_m: itrf.x_m,
            y_m: itrf.y_m,
            z_m: itrf.z_m,
        };
        SidereonStatus::Ok
    })
}

/// Convert an ITRF/ECEF position in meters to a WGS84 geodetic position
/// (latitude/longitude in radians, ellipsoidal height in meters). Pure value in,
/// value out: no handle is allocated. Wraps the engine's single validated inverse
/// converter; no conversion math lives here.
///
/// Safety: ecef must point to a SidereonItrfPosition; out_geodetic must point to
/// a SidereonGeodetic.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ecef_to_geodetic(
    ecef: *const SidereonItrfPosition,
    out_geodetic: *mut SidereonGeodetic,
) -> SidereonStatus {
    ffi_boundary("sidereon_ecef_to_geodetic", SidereonStatus::Panic, || {
        let out_geodetic = c_try!(require_out(
            out_geodetic,
            "sidereon_ecef_to_geodetic",
            "out_geodetic"
        ));
        *out_geodetic = empty_geodetic();
        let ecef = c_try!(require_ref(ecef, "sidereon_ecef_to_geodetic", "ecef"));
        let itrf = c_try!(
            ItrfPositionM::new(ecef.x_m, ecef.y_m, ecef.z_m).map_err(|err| {
                set_last_error(format!("sidereon_ecef_to_geodetic: {err}"));
                SidereonStatus::InvalidArgument
            })
        );
        let geodetic = c_try!(itrf_to_geodetic(itrf)
            .map_err(|err| map_frame_transform_error("sidereon_ecef_to_geodetic", err)));
        *out_geodetic = geodetic_to_c(&geodetic);
        SidereonStatus::Ok
    })
}

/// GCRS->ITRS rotation matrix, row-major in out_matrix (9 doubles). Delegates to
/// sidereon_core::astro::frames::transforms::gcrs_to_itrs_matrix.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gcrs_to_itrs_matrix(
    ts: *const SidereonTimeScales,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_gcrs_to_itrs_matrix",
        SidereonStatus::Panic,
        || {
            write_frame_matrix(
                "sidereon_frame_gcrs_to_itrs_matrix",
                ts,
                out_matrix,
                ft::gcrs_to_itrs_matrix,
            )
        },
    )
}

/// ITRS->GCRS rotation matrix, row-major in out_matrix (9 doubles). Delegates to
/// sidereon_core::astro::frames::transforms::itrs_to_gcrs_matrix.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_itrs_to_gcrs_matrix(
    ts: *const SidereonTimeScales,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_itrs_to_gcrs_matrix",
        SidereonStatus::Panic,
        || {
            write_frame_matrix(
                "sidereon_frame_itrs_to_gcrs_matrix",
                ts,
                out_matrix,
                ft::itrs_to_gcrs_matrix,
            )
        },
    )
}

/// Mean-equator/equinox-of-date->ITRS rotation matrix, row-major in out_matrix (9
/// doubles). Delegates to
/// sidereon_core::astro::frames::transforms::mean_of_date_to_itrs_matrix.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_mean_of_date_to_itrs_matrix(
    ts: *const SidereonTimeScales,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_mean_of_date_to_itrs_matrix",
        SidereonStatus::Panic,
        || {
            write_frame_matrix(
                "sidereon_frame_mean_of_date_to_itrs_matrix",
                ts,
                out_matrix,
                ft::mean_of_date_to_itrs_matrix,
            )
        },
    )
}

/// GCRS->ITRS rotation matrix with explicit polar motion (arcsec), row-major in
/// out_matrix. Delegates to
/// sidereon_core::astro::frames::transforms::gcrs_to_itrs_matrix_with_polar_motion.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gcrs_to_itrs_matrix_with_polar_motion(
    ts: *const SidereonTimeScales,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_gcrs_to_itrs_matrix_with_polar_motion",
        SidereonStatus::Panic,
        || {
            write_frame_matrix_polar(
                "sidereon_frame_gcrs_to_itrs_matrix_with_polar_motion",
                ts,
                xp_arcsec,
                yp_arcsec,
                out_matrix,
                ft::gcrs_to_itrs_matrix_with_polar_motion,
            )
        },
    )
}

/// ITRS->GCRS rotation matrix with explicit polar motion (arcsec), row-major in
/// out_matrix. Delegates to
/// sidereon_core::astro::frames::transforms::itrs_to_gcrs_matrix_with_polar_motion.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_itrs_to_gcrs_matrix_with_polar_motion(
    ts: *const SidereonTimeScales,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_itrs_to_gcrs_matrix_with_polar_motion",
        SidereonStatus::Panic,
        || {
            write_frame_matrix_polar(
                "sidereon_frame_itrs_to_gcrs_matrix_with_polar_motion",
                ts,
                xp_arcsec,
                yp_arcsec,
                out_matrix,
                ft::itrs_to_gcrs_matrix_with_polar_motion,
            )
        },
    )
}

/// Mean-of-date->ITRS rotation matrix with explicit polar motion (arcsec),
/// row-major in out_matrix. Delegates to
/// sidereon_core::astro::frames::transforms::mean_of_date_to_itrs_matrix_with_polar_motion.
///
/// Safety: ts points to a SidereonTimeScales; out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_mean_of_date_to_itrs_matrix_with_polar_motion(
    ts: *const SidereonTimeScales,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_mean_of_date_to_itrs_matrix_with_polar_motion",
        SidereonStatus::Panic,
        || {
            write_frame_matrix_polar(
                "sidereon_frame_mean_of_date_to_itrs_matrix_with_polar_motion",
                ts,
                xp_arcsec,
                yp_arcsec,
                out_matrix,
                ft::mean_of_date_to_itrs_matrix_with_polar_motion,
            )
        },
    )
}

/// IERS polar-motion matrix W from xp/yp (arcseconds), row-major in out_matrix.
/// Delegates to sidereon_core::astro::frames::transforms::polar_motion_matrix.
///
/// Safety: out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_polar_motion_matrix(
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_polar_motion_matrix",
        SidereonStatus::Panic,
        || {
            let out_matrix = c_try!(require_out(
                out_matrix,
                "sidereon_frame_polar_motion_matrix",
                "out_matrix"
            ));
            let out_matrix = out_matrix as *mut f64;
            for idx in 0..9 {
                *out_matrix.add(idx) = 0.0;
            }
            let pole = c_try!(polar_motion_from_arcsec(
                "sidereon_frame_polar_motion_matrix",
                xp_arcsec,
                yp_arcsec
            ));
            match ft::polar_motion_matrix(pole) {
                Ok(m) => {
                    copy_flat9(out_matrix, m);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_transform_error("sidereon_frame_polar_motion_matrix", err),
            }
        },
    )
}

/// Greenwich Mean Sidereal Time (radians) for the given time scales.
/// Delegates to
/// sidereon_core::astro::frames::transforms::greenwich_mean_sidereal_time_radians.
///
/// Safety: ts points to a SidereonTimeScales; out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gmst_radians(
    ts: *const SidereonTimeScales,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frame_gmst_radians", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_frame_gmst_radians", "out"));
        *out = 0.0;
        let ts = c_try!(require_ref(ts, "sidereon_frame_gmst_radians", "ts")).to_core();
        match ft::greenwich_mean_sidereal_time_radians(&ts) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => map_frame_transform_error("sidereon_frame_gmst_radians", err),
        }
    })
}

/// Greenwich Apparent Sidereal Time (radians) for the given time scales.
/// Delegates to
/// sidereon_core::astro::frames::transforms::greenwich_apparent_sidereal_time_radians.
///
/// Safety: ts points to a SidereonTimeScales; out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gast_radians(
    ts: *const SidereonTimeScales,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frame_gast_radians", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_frame_gast_radians", "out"));
        *out = 0.0;
        let ts = c_try!(require_ref(ts, "sidereon_frame_gast_radians", "ts")).to_core();
        match ft::greenwich_apparent_sidereal_time_radians(&ts) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => map_frame_transform_error("sidereon_frame_gast_radians", err),
        }
    })
}

/// Standard (non-FMA) 3x3 matrix times 3-vector. The matrix is row-major in r
/// (9 doubles), the vector in p (3 doubles), the product in out (3 doubles).
/// Delegates to sidereon_core::astro::frames::transforms::mat3_vec3_mul.
///
/// Safety: r points to 9 doubles; p and out point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_mat3_vec3_mul(
    r: *const f64,
    p: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_mat3_vec3_mul",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_frame_mat3_vec3_mul", "out"));
            let out = out as *mut f64;
            for idx in 0..3 {
                *out.add(idx) = 0.0;
            }
            let r = c_try!(read_mat3("sidereon_frame_mat3_vec3_mul", "r", r));
            let p = c_try!(read_vec3("sidereon_frame_mat3_vec3_mul", "p", p));
            match ft::mat3_vec3_mul(&r, &p) {
                Ok(v) => {
                    copy_vec3(out, v);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_transform_error("sidereon_frame_mat3_vec3_mul", err),
            }
        },
    )
}

/// TEME position/velocity (km, km/s) to GCRS. skyfield_compat selects the
/// AU-scaled FMA path (true) or the direct km path (false). Delegates to
/// sidereon_core::astro::frames::transforms::teme_to_gcrs_compute.
///
/// Safety: position_km and velocity_km_s point to 3 doubles each; ts points to a
/// SidereonTimeScales; out_position_km and out_velocity_km_s point to 3 doubles
/// each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_teme_to_gcrs(
    position_km: *const f64,
    velocity_km_s: *const f64,
    ts: *const SidereonTimeScales,
    skyfield_compat: bool,
    out_position_km: *mut f64,
    out_velocity_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frame_teme_to_gcrs", SidereonStatus::Panic, || {
        let out_position_km = c_try!(require_out(
            out_position_km,
            "sidereon_frame_teme_to_gcrs",
            "out_position_km"
        ));
        let out_position_km = out_position_km as *mut f64;
        let out_velocity_km_s = c_try!(require_out(
            out_velocity_km_s,
            "sidereon_frame_teme_to_gcrs",
            "out_velocity_km_s"
        ));
        let out_velocity_km_s = out_velocity_km_s as *mut f64;
        for idx in 0..3 {
            *out_position_km.add(idx) = 0.0;
            *out_velocity_km_s.add(idx) = 0.0;
        }
        let position_km = c_try!(read_vec3(
            "sidereon_frame_teme_to_gcrs",
            "position_km",
            position_km
        ));
        let velocity_km_s = c_try!(read_vec3(
            "sidereon_frame_teme_to_gcrs",
            "velocity_km_s",
            velocity_km_s
        ));
        let ts = c_try!(require_ref(ts, "sidereon_frame_teme_to_gcrs", "ts")).to_core();
        let state = ft::TemeStateKm {
            position_km,
            velocity_km_s,
        };
        match ft::teme_to_gcrs_compute(&state, &ts, skyfield_compat) {
            Ok((p, v)) => {
                copy_vec3(out_position_km, [p.0, p.1, p.2]);
                copy_vec3(out_velocity_km_s, [v.0, v.1, v.2]);
                SidereonStatus::Ok
            }
            Err(err) => map_frame_transform_error("sidereon_frame_teme_to_gcrs", err),
        }
    })
}

/// GCRS position (km) to ITRS (ECEF, km). Delegates to
/// sidereon_core::astro::frames::transforms::gcrs_to_itrs_compute.
///
/// Safety: position_km and out_position_km point to 3 doubles; ts points to a
/// SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gcrs_to_itrs(
    position_km: *const f64,
    ts: *const SidereonTimeScales,
    skyfield_compat: bool,
    out_position_km: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frame_gcrs_to_itrs", SidereonStatus::Panic, || {
        let out_position_km = c_try!(require_out(
            out_position_km,
            "sidereon_frame_gcrs_to_itrs",
            "out_position_km"
        ));
        let out_position_km = out_position_km as *mut f64;
        for idx in 0..3 {
            *out_position_km.add(idx) = 0.0;
        }
        let p = c_try!(read_vec3(
            "sidereon_frame_gcrs_to_itrs",
            "position_km",
            position_km
        ));
        let ts = c_try!(require_ref(ts, "sidereon_frame_gcrs_to_itrs", "ts")).to_core();
        match ft::gcrs_to_itrs_compute(p[0], p[1], p[2], &ts, skyfield_compat) {
            Ok((x, y, z)) => {
                copy_vec3(out_position_km, [x, y, z]);
                SidereonStatus::Ok
            }
            Err(err) => map_frame_transform_error("sidereon_frame_gcrs_to_itrs", err),
        }
    })
}

/// GCRS position (km) to ITRS (ECEF, km) with explicit polar motion (arcsec).
/// Delegates to
/// sidereon_core::astro::frames::transforms::gcrs_to_itrs_compute_with_polar_motion.
///
/// Safety: position_km and out_position_km point to 3 doubles; ts points to a
/// SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gcrs_to_itrs_with_polar_motion(
    position_km: *const f64,
    ts: *const SidereonTimeScales,
    skyfield_compat: bool,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_position_km: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_gcrs_to_itrs_with_polar_motion",
        SidereonStatus::Panic,
        || {
            let out_position_km = c_try!(require_out(
                out_position_km,
                "sidereon_frame_gcrs_to_itrs_with_polar_motion",
                "out_position_km"
            ));
            let out_position_km = out_position_km as *mut f64;
            for idx in 0..3 {
                *out_position_km.add(idx) = 0.0;
            }
            let p = c_try!(read_vec3(
                "sidereon_frame_gcrs_to_itrs_with_polar_motion",
                "position_km",
                position_km
            ));
            let ts = c_try!(require_ref(
                ts,
                "sidereon_frame_gcrs_to_itrs_with_polar_motion",
                "ts"
            ))
            .to_core();
            let pole = c_try!(polar_motion_from_arcsec(
                "sidereon_frame_gcrs_to_itrs_with_polar_motion",
                xp_arcsec,
                yp_arcsec
            ));
            match ft::gcrs_to_itrs_compute_with_polar_motion(
                p[0],
                p[1],
                p[2],
                &ts,
                skyfield_compat,
                pole,
            ) {
                Ok((x, y, z)) => {
                    copy_vec3(out_position_km, [x, y, z]);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_frame_transform_error("sidereon_frame_gcrs_to_itrs_with_polar_motion", err)
                }
            }
        },
    )
}

/// ITRS (ECEF, km) position to GCRS (km). Delegates to
/// sidereon_core::astro::frames::transforms::itrs_to_gcrs_compute.
///
/// Safety: position_km and out_position_km point to 3 doubles; ts points to a
/// SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_itrs_to_gcrs(
    position_km: *const f64,
    ts: *const SidereonTimeScales,
    out_position_km: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_frame_itrs_to_gcrs", SidereonStatus::Panic, || {
        let out_position_km = c_try!(require_out(
            out_position_km,
            "sidereon_frame_itrs_to_gcrs",
            "out_position_km"
        ));
        let out_position_km = out_position_km as *mut f64;
        for idx in 0..3 {
            *out_position_km.add(idx) = 0.0;
        }
        let p = c_try!(read_vec3(
            "sidereon_frame_itrs_to_gcrs",
            "position_km",
            position_km
        ));
        let ts = c_try!(require_ref(ts, "sidereon_frame_itrs_to_gcrs", "ts")).to_core();
        match ft::itrs_to_gcrs_compute(p[0], p[1], p[2], &ts) {
            Ok((x, y, z)) => {
                copy_vec3(out_position_km, [x, y, z]);
                SidereonStatus::Ok
            }
            Err(err) => map_frame_transform_error("sidereon_frame_itrs_to_gcrs", err),
        }
    })
}

/// ITRS (ECEF, km) to GCRS (km) with explicit polar motion (arcsec). Delegates
/// to
/// sidereon_core::astro::frames::transforms::itrs_to_gcrs_compute_with_polar_motion.
///
/// Safety: position_km and out_position_km point to 3 doubles; ts points to a
/// SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_itrs_to_gcrs_with_polar_motion(
    position_km: *const f64,
    ts: *const SidereonTimeScales,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_position_km: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_itrs_to_gcrs_with_polar_motion",
        SidereonStatus::Panic,
        || {
            let out_position_km = c_try!(require_out(
                out_position_km,
                "sidereon_frame_itrs_to_gcrs_with_polar_motion",
                "out_position_km"
            ));
            let out_position_km = out_position_km as *mut f64;
            for idx in 0..3 {
                *out_position_km.add(idx) = 0.0;
            }
            let p = c_try!(read_vec3(
                "sidereon_frame_itrs_to_gcrs_with_polar_motion",
                "position_km",
                position_km
            ));
            let ts = c_try!(require_ref(
                ts,
                "sidereon_frame_itrs_to_gcrs_with_polar_motion",
                "ts"
            ))
            .to_core();
            let pole = c_try!(polar_motion_from_arcsec(
                "sidereon_frame_itrs_to_gcrs_with_polar_motion",
                xp_arcsec,
                yp_arcsec
            ));
            match ft::itrs_to_gcrs_compute_with_polar_motion(p[0], p[1], p[2], &ts, pole) {
                Ok((x, y, z)) => {
                    copy_vec3(out_position_km, [x, y, z]);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_frame_transform_error("sidereon_frame_itrs_to_gcrs_with_polar_motion", err)
                }
            }
        },
    )
}

/// ITRS/ECEF (km) to WGS84 geodetic. out_geodetic receives
/// (latitude_deg, longitude_deg, altitude_km). Delegates to
/// sidereon_core::astro::frames::transforms::itrs_to_geodetic_compute.
///
/// Safety: position_km and out_geodetic point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_itrs_to_geodetic(
    position_km: *const f64,
    out_geodetic: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_itrs_to_geodetic",
        SidereonStatus::Panic,
        || {
            let out_geodetic = c_try!(require_out(
                out_geodetic,
                "sidereon_frame_itrs_to_geodetic",
                "out_geodetic"
            ));
            let out_geodetic = out_geodetic as *mut f64;
            for idx in 0..3 {
                *out_geodetic.add(idx) = 0.0;
            }
            let p = c_try!(read_vec3(
                "sidereon_frame_itrs_to_geodetic",
                "position_km",
                position_km
            ));
            match ft::itrs_to_geodetic_compute(p[0], p[1], p[2]) {
                Ok((lat, lon, alt)) => {
                    copy_vec3(out_geodetic, [lat, lon, alt]);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_transform_error("sidereon_frame_itrs_to_geodetic", err),
            }
        },
    )
}

/// WGS84 geodetic (lat_deg, lon_deg, alt_km) to ITRS/ECEF (km). Delegates to
/// sidereon_core::astro::frames::transforms::geodetic_to_itrs.
///
/// Safety: out_position_km points to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_geodetic_to_itrs(
    lat_deg: f64,
    lon_deg: f64,
    alt_km: f64,
    out_position_km: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_geodetic_to_itrs",
        SidereonStatus::Panic,
        || {
            let out_position_km = c_try!(require_out(
                out_position_km,
                "sidereon_frame_geodetic_to_itrs",
                "out_position_km"
            ));
            let out_position_km = out_position_km as *mut f64;
            for idx in 0..3 {
                *out_position_km.add(idx) = 0.0;
            }
            match ft::geodetic_to_itrs(lat_deg, lon_deg, alt_km) {
                Ok((x, y, z)) => {
                    copy_vec3(out_position_km, [x, y, z]);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_transform_error("sidereon_frame_geodetic_to_itrs", err),
            }
        },
    )
}

/// ECEF (meters) to geodetic via the PROJ-compatible closed form. out_geodetic
/// receives (longitude_deg, latitude_deg, altitude_m). Delegates to
/// sidereon_core::astro::frames::transforms::geodetic_from_ecef_proj.
///
/// Safety: ecef_m and out_geodetic point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_geodetic_from_ecef_proj(
    ecef_m: *const f64,
    out_geodetic: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_geodetic_from_ecef_proj",
        SidereonStatus::Panic,
        || {
            let out_geodetic = c_try!(require_out(
                out_geodetic,
                "sidereon_frame_geodetic_from_ecef_proj",
                "out_geodetic"
            ));
            let out_geodetic = out_geodetic as *mut f64;
            for idx in 0..3 {
                *out_geodetic.add(idx) = 0.0;
            }
            let p = c_try!(read_vec3(
                "sidereon_frame_geodetic_from_ecef_proj",
                "ecef_m",
                ecef_m
            ));
            match ft::geodetic_from_ecef_proj(p[0], p[1], p[2]) {
                Ok(g) => {
                    copy_vec3(out_geodetic, g);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_frame_transform_error("sidereon_frame_geodetic_from_ecef_proj", err)
                }
            }
        },
    )
}

/// Topocentric az/el/range from a ground station to a satellite GCRS position.
/// out_topocentric receives (azimuth_deg, elevation_deg, range_km). Delegates to
/// sidereon_core::astro::frames::transforms::gcrs_to_topocentric_compute.
///
/// Safety: sat_gcrs_km and out_topocentric point to 3 doubles each; ts points to
/// a SidereonTimeScales.
#[no_mangle]
pub unsafe extern "C" fn sidereon_frame_gcrs_to_topocentric(
    sat_gcrs_km: *const f64,
    station_lat_deg: f64,
    station_lon_deg: f64,
    station_alt_km: f64,
    ts: *const SidereonTimeScales,
    skyfield_compat: bool,
    out_topocentric: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_frame_gcrs_to_topocentric",
        SidereonStatus::Panic,
        || {
            let out_topocentric = c_try!(require_out(
                out_topocentric,
                "sidereon_frame_gcrs_to_topocentric",
                "out_topocentric"
            ));
            let out_topocentric = out_topocentric as *mut f64;
            for idx in 0..3 {
                *out_topocentric.add(idx) = 0.0;
            }
            let sat_gcrs_km = c_try!(read_vec3(
                "sidereon_frame_gcrs_to_topocentric",
                "sat_gcrs_km",
                sat_gcrs_km
            ));
            let ts = c_try!(require_ref(ts, "sidereon_frame_gcrs_to_topocentric", "ts")).to_core();
            let station = ft::GeodeticStationKm {
                latitude_deg: station_lat_deg,
                longitude_deg: station_lon_deg,
                altitude_km: station_alt_km,
            };
            match ft::gcrs_to_topocentric_compute(sat_gcrs_km, &station, &ts, skyfield_compat) {
                Ok((az, el, range)) => {
                    copy_vec3(out_topocentric, [az, el, range]);
                    SidereonStatus::Ok
                }
                Err(err) => map_frame_transform_error("sidereon_frame_gcrs_to_topocentric", err),
            }
        },
    )
}

// --- DOP with explicit ENU convention ---------------------------------------

/// The local ENU convention used for the DOP horizontal/vertical split.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonEnuConvention {
    /// Geodetic-ellipsoid-normal ENU (the GNSS-standard default, RTKLIB
    /// `xyz2enu`); identical to sidereon_dop.
    GeodeticNormal = 0,
    /// Geocentric-radial ENU whose up is `position / |position|`; changes only
    /// HDOP/VDOP (GDOP/PDOP/TDOP are identical).
    GeocentricRadial = 1,
}

unsafe fn write_frame_matrix(
    fn_name: &str,
    ts: *const SidereonTimeScales,
    out_matrix: *mut f64,
    compute: impl FnOnce(&CoreTimeScales) -> Result<[[f64; 3]; 3], FrameTransformError>,
) -> SidereonStatus {
    let out_matrix = c_try!(require_out(out_matrix, fn_name, "out_matrix"));
    let out_matrix = out_matrix as *mut f64;
    for idx in 0..9 {
        *out_matrix.add(idx) = 0.0;
    }
    let ts = c_try!(require_ref(ts, fn_name, "ts")).to_core();
    match compute(&ts) {
        Ok(m) => {
            copy_flat9(out_matrix, m);
            SidereonStatus::Ok
        }
        Err(err) => map_frame_transform_error(fn_name, err),
    }
}

unsafe fn write_frame_matrix_polar(
    fn_name: &str,
    ts: *const SidereonTimeScales,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_matrix: *mut f64,
    compute: impl FnOnce(&CoreTimeScales, ft::PolarMotion) -> Result<[[f64; 3]; 3], FrameTransformError>,
) -> SidereonStatus {
    let out_matrix = c_try!(require_out(out_matrix, fn_name, "out_matrix"));
    let out_matrix = out_matrix as *mut f64;
    for idx in 0..9 {
        *out_matrix.add(idx) = 0.0;
    }
    let ts = c_try!(require_ref(ts, fn_name, "ts")).to_core();
    let pole = c_try!(polar_motion_from_arcsec(fn_name, xp_arcsec, yp_arcsec));
    match compute(&ts, pole) {
        Ok(m) => {
            copy_flat9(out_matrix, m);
            SidereonStatus::Ok
        }
        Err(err) => map_frame_transform_error(fn_name, err),
    }
}

unsafe fn polar_motion_from_arcsec(
    fn_name: &str,
    xp_arcsec: f64,
    yp_arcsec: f64,
) -> Result<ft::PolarMotion, SidereonStatus> {
    ft::PolarMotion::from_arcseconds(xp_arcsec, yp_arcsec)
        .map_err(|err| map_frame_transform_error(fn_name, err))
}
