use super::*;

// --- Sun/Moon angles + eclipse (sidereon_core::astro::angles / events) -------

/// Eclipse status, mirroring sidereon_core::astro::events::eclipse::EclipseStatus.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonEclipseStatus {
    /// Fully sunlit.
    Sunlit = 0,
    /// Partial shadow.
    Penumbra = 1,
    /// Full shadow.
    Umbra = 2,
}

/// Sun aspect angle in degrees between satellite and Sun directions. Delegates
/// to sidereon_core::astro::angles::sun_angle. Position vectors are in km.
///
/// Safety: sat_pos_km and sun_pos_km must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_angle_deg(
    sat_pos_km: *const f64,
    sun_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_angle_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sun_angle_deg", "out"));
        *out = 0.0;
        let sat = c_try!(read_vec3(
            "sidereon_sun_angle_deg",
            "sat_pos_km",
            sat_pos_km
        ));
        let sun = c_try!(read_vec3(
            "sidereon_sun_angle_deg",
            "sun_pos_km",
            sun_pos_km
        ));
        match sidereon_core::astro::angles::sun_angle(sat, sun) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_sun_angle_deg", err),
        }
    })
}

/// Moon aspect angle in degrees. Delegates to
/// sidereon_core::astro::angles::moon_angle.
///
/// Safety: sat_pos_km and moon_pos_km must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moon_angle_deg(
    sat_pos_km: *const f64,
    moon_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_moon_angle_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_moon_angle_deg", "out"));
        *out = 0.0;
        let sat = c_try!(read_vec3(
            "sidereon_moon_angle_deg",
            "sat_pos_km",
            sat_pos_km
        ));
        let moon = c_try!(read_vec3(
            "sidereon_moon_angle_deg",
            "moon_pos_km",
            moon_pos_km
        ));
        match sidereon_core::astro::angles::moon_angle(sat, moon) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_moon_angle_deg", err),
        }
    })
}

/// Sun elevation in degrees at the satellite. Delegates to
/// sidereon_core::astro::angles::sun_elevation.
///
/// Safety: sat_pos_km and sun_pos_km must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_elevation_deg(
    sat_pos_km: *const f64,
    sun_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_elevation_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sun_elevation_deg", "out"));
        *out = 0.0;
        let sat = c_try!(read_vec3(
            "sidereon_sun_elevation_deg",
            "sat_pos_km",
            sat_pos_km
        ));
        let sun = c_try!(read_vec3(
            "sidereon_sun_elevation_deg",
            "sun_pos_km",
            sun_pos_km
        ));
        match sidereon_core::astro::angles::sun_elevation(sat, sun) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_sun_elevation_deg", err),
        }
    })
}

/// Earth angular radius in degrees as seen from the satellite. Delegates to
/// sidereon_core::astro::angles::earth_angular_radius.
///
/// Safety: sat_pos_km must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_earth_angular_radius_deg(
    sat_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_earth_angular_radius_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_earth_angular_radius_deg", "out"));
            *out = 0.0;
            let sat = c_try!(read_vec3(
                "sidereon_earth_angular_radius_deg",
                "sat_pos_km",
                sat_pos_km
            ));
            match sidereon_core::astro::angles::earth_angular_radius(sat) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_earth_angular_radius_deg", err),
            }
        },
    )
}

/// Fractional solar illumination at the satellite in [0, 1]. Delegates to
/// sidereon_core::astro::events::eclipse::shadow_fraction.
///
/// Safety: sat_pos_km and sun_pos_km must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eclipse_shadow_fraction(
    sat_pos_km: *const f64,
    sun_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_eclipse_shadow_fraction",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_eclipse_shadow_fraction", "out"));
            *out = 0.0;
            let sat = c_try!(read_vec3(
                "sidereon_eclipse_shadow_fraction",
                "sat_pos_km",
                sat_pos_km
            ));
            let sun = c_try!(read_vec3(
                "sidereon_eclipse_shadow_fraction",
                "sun_pos_km",
                sun_pos_km
            ));
            match sidereon_core::astro::events::eclipse::shadow_fraction(sat, sun) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_eclipse_shadow_fraction", err),
            }
        },
    )
}

/// Discrete eclipse status (sunlit / penumbra / umbra). Delegates to
/// sidereon_core::astro::events::eclipse::status.
///
/// Safety: sat_pos_km and sun_pos_km must point to 3 doubles; out must point to
/// a SidereonEclipseStatus.
#[no_mangle]
pub unsafe extern "C" fn sidereon_eclipse_status(
    sat_pos_km: *const f64,
    sun_pos_km: *const f64,
    out: *mut SidereonEclipseStatus,
) -> SidereonStatus {
    ffi_boundary("sidereon_eclipse_status", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_eclipse_status", "out"));
        *out = SidereonEclipseStatus::Sunlit;
        let sat = c_try!(read_vec3(
            "sidereon_eclipse_status",
            "sat_pos_km",
            sat_pos_km
        ));
        let sun = c_try!(read_vec3(
            "sidereon_eclipse_status",
            "sun_pos_km",
            sun_pos_km
        ));
        match sidereon_core::astro::events::eclipse::status(sat, sun) {
            Ok(s) => {
                *out = match s {
                    sidereon_core::astro::events::eclipse::EclipseStatus::Sunlit => {
                        SidereonEclipseStatus::Sunlit
                    }
                    sidereon_core::astro::events::eclipse::EclipseStatus::Penumbra => {
                        SidereonEclipseStatus::Penumbra
                    }
                    sidereon_core::astro::events::eclipse::EclipseStatus::Umbra => {
                        SidereonEclipseStatus::Umbra
                    }
                };
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_eclipse_status", err),
        }
    })
}

// --- Sun/Moon ephemeris (sidereon_core::astro::bodies) -----------------------

/// Geocentric Sun and Moon positions in meters (ECI, mean equator/equinox of
/// date) at a TT epoch expressed in Julian centuries since J2000. Delegates to
/// sidereon_core::astro::bodies::sun_moon_eci.
///
/// Safety: out_sun_m and out_moon_m must each point to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_moon_eci(
    tt_julian_centuries: f64,
    out_sun_m: *mut f64,
    out_moon_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_moon_eci", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_sun_moon_eci",
            "out_sun_m",
            out_sun_m,
            3,
            &[0.0, 0.0, 0.0]
        ));
        c_try!(copy_exact_f64s(
            "sidereon_sun_moon_eci",
            "out_moon_m",
            out_moon_m,
            3,
            &[0.0, 0.0, 0.0]
        ));
        match sidereon_core::astro::bodies::sun_moon_eci(tt_julian_centuries) {
            Ok(sm) => {
                c_try!(copy_exact_f64s(
                    "sidereon_sun_moon_eci",
                    "out_sun_m",
                    out_sun_m,
                    3,
                    &sm.sun
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_sun_moon_eci",
                    "out_moon_m",
                    out_moon_m,
                    3,
                    &sm.moon
                ));
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_sun_moon_eci", err),
        }
    })
}

/// Analytic Sun and Moon positions in Earth-fixed ECEF, metres, for one
/// unix-microsecond UTC epoch. Delegates to
/// sidereon_core::astro::bodies::sun_moon_ecef.
///
/// Safety: out_sun_m and out_moon_m must each point to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_moon_ecef(
    epoch_unix_us: i64,
    out_sun_m: *mut f64,
    out_moon_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_moon_ecef", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_sun_moon_ecef",
            "out_sun_m",
            out_sun_m,
            3,
            &[0.0, 0.0, 0.0]
        ));
        c_try!(copy_exact_f64s(
            "sidereon_sun_moon_ecef",
            "out_moon_m",
            out_moon_m,
            3,
            &[0.0, 0.0, 0.0]
        ));
        let ts = UtcInstant::from_unix_microseconds(epoch_unix_us).time_scales();
        match sidereon_core::astro::bodies::sun_moon_ecef(&ts) {
            Ok(sm) => {
                c_try!(copy_exact_f64s(
                    "sidereon_sun_moon_ecef",
                    "out_sun_m",
                    out_sun_m,
                    3,
                    &sm.sun
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_sun_moon_ecef",
                    "out_moon_m",
                    out_moon_m,
                    3,
                    &sm.moon
                ));
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_sun_moon_ecef", err),
        }
    })
}

/// Analytic Sun and Moon positions in geocentric ECI, metres, for a batch of
/// unix-microsecond UTC epochs. Delegates to
/// sidereon_core::astro::bodies::sun_moon_eci_at.
///
/// Safety: epochs_unix_us must point to count int64_t values; out_sun_m and
/// out_moon_m must each point to at least 3 * count doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_moon_eci_batch(
    epochs_unix_us: *const i64,
    count: usize,
    out_sun_m: *mut f64,
    sun_len: usize,
    out_moon_m: *mut f64,
    moon_len: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_moon_eci_batch", SidereonStatus::Panic, || {
        sun_moon_epoch_batch(
            "sidereon_sun_moon_eci_batch",
            SunMoonEpochBatchArgs {
                epochs_unix_us,
                count,
                out_sun_m,
                sun_len,
                out_moon_m,
                moon_len,
            },
            sidereon_core::astro::bodies::sun_moon_eci_at,
        )
    })
}

/// Analytic Sun and Moon positions in Earth-fixed ECEF, metres, for a batch of
/// unix-microsecond UTC epochs. Delegates to
/// sidereon_core::astro::bodies::sun_moon_ecef.
///
/// Safety: epochs_unix_us must point to count int64_t values; out_sun_m and
/// out_moon_m must each point to at least 3 * count doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_moon_ecef_batch(
    epochs_unix_us: *const i64,
    count: usize,
    out_sun_m: *mut f64,
    sun_len: usize,
    out_moon_m: *mut f64,
    moon_len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sun_moon_ecef_batch",
        SidereonStatus::Panic,
        || {
            sun_moon_epoch_batch(
                "sidereon_sun_moon_ecef_batch",
                SunMoonEpochBatchArgs {
                    epochs_unix_us,
                    count,
                    out_sun_m,
                    sun_len,
                    out_moon_m,
                    moon_len,
                },
                sidereon_core::astro::bodies::sun_moon_ecef,
            )
        },
    )
}

/// IAU 2000A nutation in longitude (dpsi) and obliquity (deps), radians, at a TT
/// Julian date. Delegates to
/// sidereon_core::astro::frames::nutation::skyfield_iau2000a_radians.
///
/// Safety: out_dpsi_rad and out_deps_rad point to a double each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nutation_iau2000a_radians(
    jd_tt: f64,
    out_dpsi_rad: *mut f64,
    out_deps_rad: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nutation_iau2000a_radians",
        SidereonStatus::Panic,
        || {
            let out_dpsi_rad = c_try!(require_out(
                out_dpsi_rad,
                "sidereon_nutation_iau2000a_radians",
                "out_dpsi_rad"
            ));
            *out_dpsi_rad = 0.0;
            let out_deps_rad = c_try!(require_out(
                out_deps_rad,
                "sidereon_nutation_iau2000a_radians",
                "out_deps_rad"
            ));
            *out_deps_rad = 0.0;
            match ft_nutation::skyfield_iau2000a_radians(jd_tt) {
                Ok((dpsi, deps)) => {
                    *out_dpsi_rad = dpsi;
                    *out_deps_rad = deps;
                    SidereonStatus::Ok
                }
                Err(err) => map_nutation_error("sidereon_nutation_iau2000a_radians", err),
            }
        },
    )
}

/// Mean obliquity of the ecliptic (radians) at a TDB Julian date. Delegates to
/// sidereon_core::astro::frames::nutation::skyfield_mean_obliquity_radians.
///
/// Safety: out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nutation_mean_obliquity_radians(
    jd_tdb: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nutation_mean_obliquity_radians",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_nutation_mean_obliquity_radians",
                "out"
            ));
            *out = 0.0;
            match ft_nutation::skyfield_mean_obliquity_radians(jd_tdb) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => map_nutation_error("sidereon_nutation_mean_obliquity_radians", err),
            }
        },
    )
}

/// The five Delaunay fundamental arguments (radians) at Julian centuries t.
/// Delegates to
/// sidereon_core::astro::frames::nutation::skyfield_fundamental_arguments.
///
/// Safety: out points to 5 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nutation_fundamental_arguments(
    t_julian_centuries: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nutation_fundamental_arguments",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_nutation_fundamental_arguments",
                "out"
            ));
            let out = out as *mut f64;
            for idx in 0..5 {
                *out.add(idx) = 0.0;
            }
            match ft_nutation::skyfield_fundamental_arguments(t_julian_centuries) {
                Ok(v) => {
                    for (idx, value) in v.iter().enumerate() {
                        *out.add(idx) = *value;
                    }
                    SidereonStatus::Ok
                }
                Err(err) => map_nutation_error("sidereon_nutation_fundamental_arguments", err),
            }
        },
    )
}

/// Equation-of-the-equinoxes complementary terms (radians) at a TT Julian date.
/// Delegates to
/// sidereon_core::astro::frames::nutation::skyfield_equation_of_the_equinoxes_complimentary_terms.
///
/// Safety: out points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nutation_equation_of_equinoxes_terms(
    jd_tt: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nutation_equation_of_equinoxes_terms",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_nutation_equation_of_equinoxes_terms",
                "out"
            ));
            *out = 0.0;
            match ft_nutation::skyfield_equation_of_the_equinoxes_complimentary_terms(jd_tt) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_nutation_error("sidereon_nutation_equation_of_equinoxes_terms", err)
                }
            }
        },
    )
}

/// 3x3 nutation rotation matrix from mean obliquity, true obliquity, and the
/// nutation in longitude (psi), all radians, row-major in out_matrix. Delegates
/// to sidereon_core::astro::frames::nutation::build_skyfield_nutation_matrix.
///
/// Safety: out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nutation_matrix(
    mean_obliquity_rad: f64,
    true_obliquity_rad: f64,
    psi_rad: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_nutation_matrix", SidereonStatus::Panic, || {
        let out_matrix = c_try!(require_out(
            out_matrix,
            "sidereon_nutation_matrix",
            "out_matrix"
        ));
        let out_matrix = out_matrix as *mut f64;
        for idx in 0..9 {
            *out_matrix.add(idx) = 0.0;
        }
        match ft_nutation::build_skyfield_nutation_matrix(
            mean_obliquity_rad,
            true_obliquity_rad,
            psi_rad,
        ) {
            Ok(m) => {
                copy_flat9(out_matrix, m);
                SidereonStatus::Ok
            }
            Err(err) => map_nutation_error("sidereon_nutation_matrix", err),
        }
    })
}

/// IAU 2006 precession matrix at a TDB Julian date, row-major in out_matrix.
/// Delegates to
/// sidereon_core::astro::frames::precession::compute_skyfield_precession_matrix.
///
/// Safety: out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precession_matrix(
    jd_tdb: f64,
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_precession_matrix", SidereonStatus::Panic, || {
        let out_matrix = c_try!(require_out(
            out_matrix,
            "sidereon_precession_matrix",
            "out_matrix"
        ));
        let out_matrix = out_matrix as *mut f64;
        for idx in 0..9 {
            *out_matrix.add(idx) = 0.0;
        }
        match ft_precession::compute_skyfield_precession_matrix(jd_tdb) {
            Ok(m) => {
                copy_flat9(out_matrix, m);
                SidereonStatus::Ok
            }
            Err(err) => map_precession_error("sidereon_precession_matrix", err),
        }
    })
}

/// ICRS-to-J2000 frame-bias matrix, row-major in out_matrix (infallible).
/// Delegates to sidereon_core::astro::frames::precession::build_icrs_to_j2000.
///
/// Safety: out_matrix points to 9 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precession_icrs_to_j2000_matrix(
    out_matrix: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precession_icrs_to_j2000_matrix",
        SidereonStatus::Panic,
        || {
            let out_matrix = c_try!(require_out(
                out_matrix,
                "sidereon_precession_icrs_to_j2000_matrix",
                "out_matrix"
            ));
            let out_matrix = out_matrix as *mut f64;
            copy_flat9(out_matrix, ft_precession::build_icrs_to_j2000());
            SidereonStatus::Ok
        },
    )
}

/// Solid-earth tide displacement of an ITRF station, metres ECEF. Delegates to
/// sidereon_core::tides::solid_earth_tide.
///
/// Safety: station_ecef_m, sun_ecef_m, moon_ecef_m, and out_m must each point
/// to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solid_earth_tide(
    station_ecef_m: *const f64,
    year: i32,
    month: i32,
    day: i32,
    fhr: f64,
    sun_ecef_m: *const f64,
    moon_ecef_m: *const f64,
    out_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_solid_earth_tide", SidereonStatus::Panic, || {
        let station = c_try!(read_vec3(
            "sidereon_solid_earth_tide",
            "station_ecef_m",
            station_ecef_m
        ));
        let sun = c_try!(read_vec3(
            "sidereon_solid_earth_tide",
            "sun_ecef_m",
            sun_ecef_m
        ));
        let moon = c_try!(read_vec3(
            "sidereon_solid_earth_tide",
            "moon_ecef_m",
            moon_ecef_m
        ));
        match sidereon_core::tides::solid_earth_tide(&station, year, month, day, fhr, &sun, &moon) {
            Ok(displacement) => {
                c_try!(copy_exact_f64s(
                    "sidereon_solid_earth_tide",
                    "out_m",
                    out_m,
                    3,
                    &displacement
                ));
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_solid_earth_tide", err),
        }
    })
}

/// Ocean tide loading displacement of an ITRF station, metres ECEF. Delegates
/// to sidereon_core::tides::ocean_tide_loading.
///
/// Safety: station_ecef_m and out_m must each point to 3 doubles; blq must
/// point to a SidereonOceanLoadingBlq.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ocean_tide_loading(
    station_ecef_m: *const f64,
    year: i32,
    month: i32,
    day: i32,
    fhr: f64,
    blq: *const SidereonOceanLoadingBlq,
    out_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_ocean_tide_loading", SidereonStatus::Panic, || {
        let station = c_try!(read_vec3(
            "sidereon_ocean_tide_loading",
            "station_ecef_m",
            station_ecef_m
        ));
        let blq = c_try!(require_ref(blq, "sidereon_ocean_tide_loading", "blq"));
        let blq = sidereon_core::tides::OceanLoadingBlq {
            amplitude_m: blq.amplitude_m,
            phase_deg: blq.phase_deg,
        };
        match sidereon_core::tides::ocean_tide_loading(&station, year, month, day, fhr, &blq) {
            Ok(displacement) => {
                c_try!(copy_exact_f64s(
                    "sidereon_ocean_tide_loading",
                    "out_m",
                    out_m,
                    3,
                    &displacement
                ));
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_ocean_tide_loading", err),
        }
    })
}

/// Solid-earth pole tide displacement of an ITRF station, metres ECEF.
/// Delegates to sidereon_core::tides::solid_earth_pole_tide.
///
/// Safety: station_ecef_m and out_m must each point to 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_solid_earth_pole_tide(
    station_ecef_m: *const f64,
    year: i32,
    month: i32,
    day: i32,
    fhr: f64,
    xp_arcsec: f64,
    yp_arcsec: f64,
    out_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_solid_earth_pole_tide",
        SidereonStatus::Panic,
        || {
            let station = c_try!(read_vec3(
                "sidereon_solid_earth_pole_tide",
                "station_ecef_m",
                station_ecef_m
            ));
            match sidereon_core::tides::solid_earth_pole_tide(
                &station, year, month, day, fhr, xp_arcsec, yp_arcsec,
            ) {
                Ok(displacement) => {
                    c_try!(copy_exact_f64s(
                        "sidereon_solid_earth_pole_tide",
                        "out_m",
                        out_m,
                        3,
                        &displacement
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_solid_earth_pole_tide", err),
            }
        },
    )
}

// --- General body observing (sidereon_core::astro::bodies::observe) ----------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEquatorial {
    pub right_ascension_deg: f64,
    pub right_ascension_hours: f64,
    pub declination_deg: f64,
    pub distance_km: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonHorizontal {
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub range_km: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEcliptic {
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_km: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBodyObservation {
    pub astrometric: SidereonEquatorial,
    pub apparent_icrs: SidereonEquatorial,
    pub apparent: SidereonEquatorial,
    pub horizontal: SidereonHorizontal,
    pub hour_angle_deg: f64,
    pub hour_angle_hours: f64,
    pub ecliptic: SidereonEcliptic,
    pub reduced: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRefraction {
    pub pressure_mbar: f64,
    pub temperature_c: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObserveOptions {
    pub has_polar_motion: bool,
    pub xp_rad: f64,
    pub yp_rad: f64,
    pub has_refraction: bool,
    pub refraction: SidereonRefraction,
    pub deflection: bool,
    pub aberration: bool,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonObserveTargetKind {
    Sun = 0,
    Moon = 1,
    Spk = 2,
    BarycentricState = 3,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observe_options_init(
    out: *mut SidereonObserveOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observe_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_observe_options_init", "out"));
            let defaults = sidereon_core::astro::bodies::ObserveOptions::default();
            *out = SidereonObserveOptions {
                has_polar_motion: false,
                xp_rad: 0.0,
                yp_rad: 0.0,
                has_refraction: false,
                refraction: SidereonRefraction {
                    pressure_mbar: 0.0,
                    temperature_c: 0.0,
                },
                deflection: defaults.deflection,
                aberration: defaults.aberration,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observe_spk_body(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    spk: *const SidereonSpk,
    naif_id: i32,
    out: *mut SidereonBodyObservation,
) -> SidereonStatus {
    ffi_boundary("sidereon_observe_spk_body", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_observe_spk_body", "out"));
        let station = c_try!(require_ref(station, "sidereon_observe_spk_body", "station"));
        let spk = c_try!(require_ref(spk, "sidereon_observe_spk_body", "spk"));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        match sidereon_core::astro::bodies::observe_spk_body(
            &station_to_core(station),
            time,
            &spk.inner,
            naif_id,
        ) {
            Ok(obs) => {
                *out = body_observation_to_c(obs);
                SidereonStatus::Ok
            }
            Err(err) => map_observe_error("sidereon_observe_spk_body", err),
        }
    })
}

// --- Almanac events (sidereon_core::astro::almanac) -------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSeasonKind {
    MarchEquinox = 0,
    JuneSolstice = 1,
    SeptemberEquinox = 2,
    DecemberSolstice = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSeasonEvent {
    pub time_unix_us: i64,
    pub kind: SidereonSeasonKind,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonMoonPhaseKind {
    New = 0,
    FirstQuarter = 1,
    Full = 2,
    LastQuarter = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMoonPhaseEvent {
    pub time_unix_us: i64,
    pub kind: SidereonMoonPhaseKind,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPlanet {
    Mercury = 0,
    Venus = 1,
    Mars = 2,
    Jupiter = 3,
    Saturn = 4,
    Uranus = 5,
    Neptune = 6,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPlanetaryEventKind {
    Conjunction = 0,
    Opposition = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPlanetaryEvent {
    pub time_unix_us: i64,
    pub planet: SidereonPlanet,
    pub kind: SidereonPlanetaryEventKind,
    pub elongation_deg: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonCulminationKind {
    Upper = 0,
    Lower = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTransitBodyKind {
    Sun = 0,
    Moon = 1,
    Planet = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMeridianTransit {
    pub time_unix_us: i64,
    pub kind: SidereonCulminationKind,
    pub altitude_deg: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonAlmanacEclipseKind {
    LunarPenumbral = 0,
    LunarPartial = 1,
    LunarTotal = 2,
    SolarPartial = 3,
    SolarAnnular = 4,
    SolarTotal = 5,
    SolarHybrid = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAlmanacEclipseEvent {
    pub time_maximum_unix_us: i64,
    pub kind: SidereonAlmanacEclipseKind,
    pub magnitude: f64,
    pub moon_latitude_deg: f64,
    pub gamma: f64,
    pub uncertain: bool,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_almanac_seasons(
    spk: *const SidereonSpk,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonSeasonEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_almanac_seasons", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_almanac_seasons",
            out_written,
            out_required
        ));
        let source = c_try!(almanac_source_from_c("sidereon_almanac_seasons", spk));
        match sidereon_core::astro::almanac::seasons(
            source,
            UtcInstant::from_unix_microseconds(start_unix_us),
            UtcInstant::from_unix_microseconds(end_unix_us),
            step_seconds,
            time_tolerance_seconds,
        ) {
            Ok(events) => {
                let rows: Vec<SidereonSeasonEvent> = events
                    .into_iter()
                    .map(|event| SidereonSeasonEvent {
                        time_unix_us: event.time.unix_microseconds(),
                        kind: season_kind_to_c(event.kind),
                    })
                    .collect();
                c_try!(copy_prefix_to_c(
                    "sidereon_almanac_seasons",
                    "out",
                    &rows,
                    out,
                    len,
                    out_written,
                    out_required,
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_almanac_error("sidereon_almanac_seasons", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_almanac_moon_phases(
    spk: *const SidereonSpk,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonMoonPhaseEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_almanac_moon_phases",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_almanac_moon_phases",
                out_written,
                out_required
            ));
            let source = c_try!(almanac_source_from_c("sidereon_almanac_moon_phases", spk));
            match sidereon_core::astro::almanac::moon_phases(
                source,
                UtcInstant::from_unix_microseconds(start_unix_us),
                UtcInstant::from_unix_microseconds(end_unix_us),
                step_seconds,
                time_tolerance_seconds,
            ) {
                Ok(events) => {
                    let rows: Vec<SidereonMoonPhaseEvent> = events
                        .into_iter()
                        .map(|event| SidereonMoonPhaseEvent {
                            time_unix_us: event.time.unix_microseconds(),
                            kind: moon_phase_kind_to_c(event.kind),
                        })
                        .collect();
                    c_try!(copy_prefix_to_c(
                        "sidereon_almanac_moon_phases",
                        "out",
                        &rows,
                        out,
                        len,
                        out_written,
                        out_required,
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => map_almanac_error("sidereon_almanac_moon_phases", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_almanac_planetary_events(
    spk: *const SidereonSpk,
    planet: u32,
    kind: u32,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonPlanetaryEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_almanac_planetary_events",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_almanac_planetary_events",
                out_written,
                out_required
            ));
            let source = c_try!(almanac_source_from_c(
                "sidereon_almanac_planetary_events",
                spk
            ));
            let planet = c_try!(planet_from_c("sidereon_almanac_planetary_events", planet));
            let kind = c_try!(planetary_kind_from_c(
                "sidereon_almanac_planetary_events",
                kind
            ));
            match sidereon_core::astro::almanac::planetary_events(
                source,
                planet,
                kind,
                UtcInstant::from_unix_microseconds(start_unix_us),
                UtcInstant::from_unix_microseconds(end_unix_us),
                step_seconds,
                time_tolerance_seconds,
            ) {
                Ok(events) => {
                    let rows: Vec<SidereonPlanetaryEvent> = events
                        .into_iter()
                        .map(|event| SidereonPlanetaryEvent {
                            time_unix_us: event.time.unix_microseconds(),
                            planet: planet_to_c(event.planet),
                            kind: match event.kind {
                                sidereon_core::astro::almanac::PlanetaryEventKind::Conjunction => {
                                    SidereonPlanetaryEventKind::Conjunction
                                }
                                sidereon_core::astro::almanac::PlanetaryEventKind::Opposition => {
                                    SidereonPlanetaryEventKind::Opposition
                                }
                                _ => SidereonPlanetaryEventKind::Conjunction,
                            },
                            elongation_deg: event.elongation_deg,
                        })
                        .collect();
                    c_try!(copy_prefix_to_c(
                        "sidereon_almanac_planetary_events",
                        "out",
                        &rows,
                        out,
                        len,
                        out_written,
                        out_required,
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => map_almanac_error("sidereon_almanac_planetary_events", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_almanac_meridian_transits(
    spk: *const SidereonSpk,
    body_kind: u32,
    planet: u32,
    station: *const SidereonGeodeticStation,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonMeridianTransit,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_almanac_meridian_transits",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_almanac_meridian_transits",
                out_written,
                out_required
            ));
            let source = c_try!(almanac_source_from_c(
                "sidereon_almanac_meridian_transits",
                spk
            ));
            let body_kind = match SidereonTransitBodyKind::try_from(body_kind) {
                Ok(value) => value,
                Err(()) => {
                    set_last_error(
                        "sidereon_almanac_meridian_transits: invalid body_kind".to_string(),
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            let body = match body_kind {
                SidereonTransitBodyKind::Sun => sidereon_core::astro::almanac::TransitBody::Sun,
                SidereonTransitBodyKind::Moon => sidereon_core::astro::almanac::TransitBody::Moon,
                SidereonTransitBodyKind::Planet => {
                    sidereon_core::astro::almanac::TransitBody::Planet(c_try!(planet_from_c(
                        "sidereon_almanac_meridian_transits",
                        planet
                    )))
                }
            };
            let station = c_try!(require_ref(
                station,
                "sidereon_almanac_meridian_transits",
                "station"
            ));
            match sidereon_core::astro::almanac::meridian_transits(
                source,
                body,
                &station_to_core(station),
                UtcInstant::from_unix_microseconds(start_unix_us),
                UtcInstant::from_unix_microseconds(end_unix_us),
                step_seconds,
                time_tolerance_seconds,
            ) {
                Ok(events) => {
                    let rows: Vec<SidereonMeridianTransit> = events
                        .into_iter()
                        .map(|event| SidereonMeridianTransit {
                            time_unix_us: event.time.unix_microseconds(),
                            kind: match event.kind {
                                sidereon_core::astro::almanac::CulminationKind::Upper => {
                                    SidereonCulminationKind::Upper
                                }
                                sidereon_core::astro::almanac::CulminationKind::Lower => {
                                    SidereonCulminationKind::Lower
                                }
                                _ => SidereonCulminationKind::Upper,
                            },
                            altitude_deg: event.altitude_deg,
                        })
                        .collect();
                    c_try!(copy_prefix_to_c(
                        "sidereon_almanac_meridian_transits",
                        "out",
                        &rows,
                        out,
                        len,
                        out_written,
                        out_required,
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => map_almanac_error("sidereon_almanac_meridian_transits", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_almanac_lunar_solar_eclipses(
    spk: *const SidereonSpk,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonAlmanacEclipseEvent,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_almanac_lunar_solar_eclipses",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_almanac_lunar_solar_eclipses",
                out_written,
                out_required
            ));
            let source = c_try!(almanac_source_from_c(
                "sidereon_almanac_lunar_solar_eclipses",
                spk
            ));
            match sidereon_core::astro::almanac::lunar_solar_eclipses(
                source,
                UtcInstant::from_unix_microseconds(start_unix_us),
                UtcInstant::from_unix_microseconds(end_unix_us),
                step_seconds,
                time_tolerance_seconds,
            ) {
                Ok(events) => {
                    let rows: Vec<SidereonAlmanacEclipseEvent> = events
                        .into_iter()
                        .map(|event| SidereonAlmanacEclipseEvent {
                            time_maximum_unix_us: event.time_maximum.unix_microseconds(),
                            kind: almanac_eclipse_kind_to_c(event.kind),
                            magnitude: event.magnitude,
                            moon_latitude_deg: event.moon_latitude_deg,
                            gamma: event.gamma,
                            uncertain: event.uncertain,
                        })
                        .collect();
                    c_try!(copy_prefix_to_c(
                        "sidereon_almanac_lunar_solar_eclipses",
                        "out",
                        &rows,
                        out,
                        len,
                        out_written,
                        out_required,
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => map_almanac_error("sidereon_almanac_lunar_solar_eclipses", err),
            }
        },
    )
}

// --- Observational-astronomy geometry (sidereon_core::astro::observation) -----

/// A surface point as geocentric latitude/longitude (degrees), mirroring
/// sidereon_core::astro::observation::SurfacePoint.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSurfacePoint {
    /// Geocentric latitude, degrees on [-90, 90].
    pub latitude_deg: f64,
    /// Longitude, degrees on (-180, 180].
    pub longitude_deg: f64,
}

/// Sub-solar point: the geographic point where the Sun is at the zenith.
/// `sun_ecef` is the geocentric Sun position in an Earth-fixed frame (only its
/// direction matters). Delegates to
/// sidereon_core::astro::observation::sub_solar_point.
///
/// Safety: sun_ecef points to 3 doubles; out points to a SidereonSurfacePoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sub_solar_point(
    sun_ecef: *const f64,
    out: *mut SidereonSurfacePoint,
) -> SidereonStatus {
    ffi_boundary("sidereon_sub_solar_point", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sub_solar_point", "out"));
        let sun = c_try!(read_vec3("sidereon_sub_solar_point", "sun_ecef", sun_ecef));
        match sub_solar_point(sun) {
            Ok(point) => {
                *out = surface_point_to_c(point);
                SidereonStatus::Ok
            }
            Err(err) => map_observation_error("sidereon_sub_solar_point", err),
        }
    })
}

/// Latitude (degrees) of the day-night terminator at a query longitude, given
/// the sub-solar point. Delegates to
/// sidereon_core::astro::observation::terminator_latitude_deg.
///
/// Safety: out_latitude_deg points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_terminator_latitude_deg(
    sub_solar_latitude_deg: f64,
    sub_solar_longitude_deg: f64,
    longitude_deg: f64,
    out_latitude_deg: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_terminator_latitude_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_latitude_deg,
                "sidereon_terminator_latitude_deg",
                "out_latitude_deg"
            ));
            *out = 0.0;
            let sub_solar = SurfacePoint {
                latitude_deg: sub_solar_latitude_deg,
                longitude_deg: sub_solar_longitude_deg,
            };
            match terminator_latitude_deg(sub_solar, longitude_deg) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_observation_error("sidereon_terminator_latitude_deg", err),
            }
        },
    )
}

/// Parallactic angle (degrees) of a target at a station. Delegates to
/// sidereon_core::astro::observation::parallactic_angle_deg.
///
/// Safety: out_angle_deg points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parallactic_angle_deg(
    observer_latitude_deg: f64,
    hour_angle_deg: f64,
    declination_deg: f64,
    out_angle_deg: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parallactic_angle_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_angle_deg,
                "sidereon_parallactic_angle_deg",
                "out_angle_deg"
            ));
            *out = 0.0;
            match parallactic_angle_deg(observer_latitude_deg, hour_angle_deg, declination_deg) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_observation_error("sidereon_parallactic_angle_deg", err),
            }
        },
    )
}

/// Sub-observer point (planetary central meridian) on a rotating body, from the
/// observer position relative to the body center (inertial frame) and the body's
/// IAU orientation (degrees). Delegates to
/// sidereon_core::astro::observation::sub_observer_point.
///
/// Safety: observer_from_body points to 3 doubles; out points to a
/// SidereonSurfacePoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sub_observer_point(
    observer_from_body: *const f64,
    pole_ra_deg: f64,
    pole_dec_deg: f64,
    prime_meridian_deg: f64,
    out: *mut SidereonSurfacePoint,
) -> SidereonStatus {
    ffi_boundary("sidereon_sub_observer_point", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sub_observer_point", "out"));
        let observer = c_try!(read_vec3(
            "sidereon_sub_observer_point",
            "observer_from_body",
            observer_from_body
        ));
        match sub_observer_point(observer, pole_ra_deg, pole_dec_deg, prime_meridian_deg) {
            Ok(point) => {
                *out = surface_point_to_c(point);
                SidereonStatus::Ok
            }
            Err(err) => map_observation_error("sidereon_sub_observer_point", err),
        }
    })
}

/// Topocentric look angle of a body (Sun or Moon) from a ground site.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonBodyAzEl {
    /// Azimuth, degrees clockwise from north on [0, 360).
    pub azimuth_deg: f64,
    /// Elevation above the local horizon, degrees on [-90, 90].
    pub elevation_deg: f64,
    /// Slant range from the site to the body, kilometers.
    pub range_km: f64,
}

/// The Moon's illuminated state as seen from a ground site.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMoonIllumination {
    /// Sunlit fraction of the lunar disk on [0, 1] (0 = new, 1 = full).
    pub illuminated_fraction: f64,
    /// Sun-Moon-observer phase angle, degrees on [0, 180] (0 = full).
    pub phase_angle_deg: f64,
}

/// Direction of a Moon elevation threshold crossing.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonMoonRiseSetKind {
    /// The Moon crossed upward through the threshold (moonrise).
    Rising = 0,
    /// The Moon crossed downward through the threshold (moonset).
    Setting = 1,
}

/// One Moon elevation threshold crossing (moonrise / moonset).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMoonElevationCrossing {
    /// Refined crossing instant, Unix microseconds.
    pub time_unix_us: i64,
    /// Crossing direction.
    pub kind: SidereonMoonRiseSetKind,
    /// Topocentric Moon elevation at the refined instant, degrees.
    pub elevation_deg: f64,
}

/// Options for the Moon rise/set finder. Pass NULL for the engine defaults
/// (-0.833 deg threshold, 300 s scan step, 1 s refinement tolerance).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMoonElevationOptions {
    /// Topocentric Moon (disk-center) elevation threshold, degrees.
    pub elevation_threshold_deg: f64,
    /// Uniform event-finder scan step, seconds.
    pub step_seconds: f64,
    /// Crossing-time refinement tolerance, seconds.
    pub time_tolerance_seconds: f64,
}

/// Kind of a Moon meridian transit (culmination).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonMoonTransitKind {
    /// Upper culmination (azimuth through due south, highest in the sky).
    Upper = 0,
    /// Lower culmination (azimuth through due north, lowest in the sky).
    Lower = 1,
}

/// One Moon meridian transit (culmination).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMoonTransit {
    /// Refined transit instant, Unix microseconds.
    pub time_unix_us: i64,
    /// Upper or lower culmination.
    pub kind: SidereonMoonTransitKind,
    /// Topocentric Moon elevation at the refined instant, degrees.
    pub elevation_deg: f64,
}

/// Topocentric azimuth/elevation/range of the Sun from a ground site at an
/// instant, written to *out. Delegates to the core `sun_az_el`.
///
/// Safety: station must point to a SidereonGeodeticStation; out must point to a
/// SidereonBodyAzEl.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sun_az_el(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    out: *mut SidereonBodyAzEl,
) -> SidereonStatus {
    ffi_boundary("sidereon_sun_az_el", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sun_az_el", "out"));
        *out = SidereonBodyAzEl {
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            range_km: 0.0,
        };
        let station = c_try!(require_ref(station, "sidereon_sun_az_el", "station"));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        match core_sun_az_el(&station_to_core(station), time) {
            Ok(azel) => {
                *out = body_az_el_to_c(azel);
                SidereonStatus::Ok
            }
            Err(err) => map_body_observation_error("sidereon_sun_az_el", err),
        }
    })
}

/// Topocentric azimuth/elevation/range of the Moon from a ground site at an
/// instant, written to *out (includes topocentric parallax). Delegates to the
/// core `moon_az_el`.
///
/// Safety: station must point to a SidereonGeodeticStation; out must point to a
/// SidereonBodyAzEl.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moon_az_el(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    out: *mut SidereonBodyAzEl,
) -> SidereonStatus {
    ffi_boundary("sidereon_moon_az_el", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_moon_az_el", "out"));
        *out = SidereonBodyAzEl {
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            range_km: 0.0,
        };
        let station = c_try!(require_ref(station, "sidereon_moon_az_el", "station"));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        match core_moon_az_el(&station_to_core(station), time) {
            Ok(azel) => {
                *out = body_az_el_to_c(azel);
                SidereonStatus::Ok
            }
            Err(err) => map_body_observation_error("sidereon_moon_az_el", err),
        }
    })
}

/// Illuminated fraction and phase angle of the Moon as seen from a ground site
/// at an instant, written to *out. Delegates to the core `moon_illumination`.
///
/// Safety: station must point to a SidereonGeodeticStation; out must point to a
/// SidereonMoonIllumination.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moon_illumination(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    out: *mut SidereonMoonIllumination,
) -> SidereonStatus {
    ffi_boundary("sidereon_moon_illumination", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_moon_illumination", "out"));
        *out = SidereonMoonIllumination {
            illuminated_fraction: 0.0,
            phase_angle_deg: 0.0,
        };
        let station = c_try!(require_ref(
            station,
            "sidereon_moon_illumination",
            "station"
        ));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        match core_moon_illumination(&station_to_core(station), time) {
            Ok(illum) => {
                *out = SidereonMoonIllumination {
                    illuminated_fraction: illum.illuminated_fraction,
                    phase_angle_deg: illum.phase_angle_deg,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_body_observation_error("sidereon_moon_illumination", err),
        }
    })
}

/// Topocentric geometric Moon (disk-center) elevation at a station and instant,
/// degrees, written to *out (includes topocentric parallax). Routes through the
/// core `moon_az_el` and returns its elevation, so a bad station maps to
/// SIDEREON_STATUS_INVALID_ARGUMENT rather than tripping the internal `expect`
/// in the core `moon_elevation_deg` convenience wrapper.
///
/// Safety: station must point to a SidereonGeodeticStation; out must point to a
/// double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moon_elevation_deg(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_moon_elevation_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_moon_elevation_deg", "out"));
        *out = 0.0;
        let station = c_try!(require_ref(
            station,
            "sidereon_moon_elevation_deg",
            "station"
        ));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        match core_moon_az_el(&station_to_core(station), time) {
            Ok(azel) => {
                *out = azel.elevation_deg;
                SidereonStatus::Ok
            }
            Err(err) => map_body_observation_error("sidereon_moon_elevation_deg", err),
        }
    })
}

/// Populate *out_options with the engine's default Moon rise/set finder options
/// (-0.833 deg threshold, 300 s scan step, 1 s refinement tolerance).
///
/// Safety: out_options must point to a SidereonMoonElevationOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moon_elevation_options_init(
    out_options: *mut SidereonMoonElevationOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_moon_elevation_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_moon_elevation_options_init",
                "out_options"
            ));
            let defaults = CoreMoonElevationOptions::default();
            *out_options = SidereonMoonElevationOptions {
                elevation_threshold_deg: defaults.elevation_threshold_deg,
                step_seconds: defaults.step_seconds,
                time_tolerance_seconds: defaults.time_tolerance_seconds,
            };
            SidereonStatus::Ok
        },
    )
}

/// Find Moon elevation threshold crossings (moonrise / moonset) for a station
/// over a UTC window. Delegates to the core `find_moon_elevation_crossings`.
/// options may be NULL for the engine defaults. Variable-length output contract:
/// pass out NULL with len 0 to query the count via *out_required.
///
/// Safety: station must point to a SidereonGeodeticStation; options must be NULL
/// or point to a SidereonMoonElevationOptions; out must point to at least len
/// writable SidereonMoonElevationCrossing or be NULL when len is 0; out_written
/// and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_find_moon_elevation_crossings(
    station: *const SidereonGeodeticStation,
    start_unix_us: i64,
    end_unix_us: i64,
    options: *const SidereonMoonElevationOptions,
    out: *mut SidereonMoonElevationCrossing,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_find_moon_elevation_crossings",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_find_moon_elevation_crossings",
                out_written,
                out_required
            ));
            let station = c_try!(require_ref(
                station,
                "sidereon_find_moon_elevation_crossings",
                "station"
            ));
            let opts = c_try!(moon_elevation_options_from_c(
                "sidereon_find_moon_elevation_crossings",
                options
            ));
            let crossings = match core_find_moon_elevation_crossings(
                &station_to_core(station),
                UtcInstant::from_unix_microseconds(start_unix_us),
                UtcInstant::from_unix_microseconds(end_unix_us),
                opts,
            ) {
                Ok(crossings) => crossings,
                Err(err) => {
                    return map_event_finder_error("sidereon_find_moon_elevation_crossings", err)
                }
            };
            let values: Vec<SidereonMoonElevationCrossing> = crossings
                .iter()
                .map(|c| SidereonMoonElevationCrossing {
                    time_unix_us: c.time.unix_microseconds(),
                    kind: match c.kind {
                        MoonElevationCrossingKind::Rising => SidereonMoonRiseSetKind::Rising,
                        MoonElevationCrossingKind::Setting => SidereonMoonRiseSetKind::Setting,
                    },
                    elevation_deg: c.elevation_deg,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_find_moon_elevation_crossings",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Find Moon meridian transits (upper and lower culminations) for a station over
/// a UTC window. Delegates to the core `find_moon_transits`. Variable-length
/// output contract: pass out NULL with len 0 to query the count via
/// *out_required.
///
/// Safety: station must point to a SidereonGeodeticStation; out must point to at
/// least len writable SidereonMoonTransit or be NULL when len is 0; out_written
/// and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_find_moon_transits(
    station: *const SidereonGeodeticStation,
    start_unix_us: i64,
    end_unix_us: i64,
    step_seconds: f64,
    time_tolerance_seconds: f64,
    out: *mut SidereonMoonTransit,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_find_moon_transits", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_find_moon_transits",
            out_written,
            out_required
        ));
        let station = c_try!(require_ref(
            station,
            "sidereon_find_moon_transits",
            "station"
        ));
        let transits = match core_find_moon_transits(
            &station_to_core(station),
            UtcInstant::from_unix_microseconds(start_unix_us),
            UtcInstant::from_unix_microseconds(end_unix_us),
            step_seconds,
            time_tolerance_seconds,
        ) {
            Ok(transits) => transits,
            Err(err) => return map_event_finder_error("sidereon_find_moon_transits", err),
        };
        let values: Vec<SidereonMoonTransit> = transits
            .iter()
            .map(|t| SidereonMoonTransit {
                time_unix_us: t.time.unix_microseconds(),
                kind: match t.kind {
                    MoonTransitKind::Upper => SidereonMoonTransitKind::Upper,
                    MoonTransitKind::Lower => SidereonMoonTransitKind::Lower,
                },
                elevation_deg: t.elevation_deg,
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_find_moon_transits",
            "out",
            &values,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_observe(
    station: *const SidereonGeodeticStation,
    time_unix_us: i64,
    target_kind: u32,
    spk: *const SidereonSpk,
    naif_id: i32,
    barycentric_position_km: *const f64,
    barycentric_velocity_km_s: *const f64,
    options: *const SidereonObserveOptions,
    out: *mut SidereonBodyObservation,
) -> SidereonStatus {
    ffi_boundary("sidereon_observe", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_observe", "out"));
        let station = c_try!(require_ref(station, "sidereon_observe", "station"));
        let station = station_to_core(station);
        let options = c_try!(observe_options_from_c(options));
        let time = UtcInstant::from_unix_microseconds(time_unix_us);
        let target_kind = match SidereonObserveTargetKind::try_from(target_kind) {
            Ok(kind) => kind,
            Err(()) => {
                set_last_error("sidereon_observe: invalid target_kind".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        let result = match target_kind {
            SidereonObserveTargetKind::Sun => sidereon_core::astro::bodies::observe(
                &station,
                time,
                sidereon_core::astro::bodies::Target::Sun,
                options,
            ),
            SidereonObserveTargetKind::Moon => sidereon_core::astro::bodies::observe(
                &station,
                time,
                sidereon_core::astro::bodies::Target::Moon,
                options,
            ),
            SidereonObserveTargetKind::Spk => {
                let spk = c_try!(require_ref(spk, "sidereon_observe", "spk"));
                sidereon_core::astro::bodies::observe(
                    &station,
                    time,
                    sidereon_core::astro::bodies::Target::Spk {
                        kernel: &spk.inner,
                        naif_id,
                    },
                    options,
                )
            }
            SidereonObserveTargetKind::BarycentricState => {
                let spk = c_try!(require_ref(spk, "sidereon_observe", "spk"));
                let position_km = c_try!(read_vec3(
                    "sidereon_observe",
                    "barycentric_position_km",
                    barycentric_position_km
                ));
                let velocity_km_s = c_try!(read_vec3(
                    "sidereon_observe",
                    "barycentric_velocity_km_s",
                    barycentric_velocity_km_s
                ));
                sidereon_core::astro::bodies::observe(
                    &station,
                    time,
                    sidereon_core::astro::bodies::Target::BarycentricState {
                        kernel: &spk.inner,
                        position_km,
                        velocity_km_s,
                    },
                    options,
                )
            }
        };
        match result {
            Ok(obs) => {
                *out = body_observation_to_c(obs);
                SidereonStatus::Ok
            }
            Err(err) => map_observe_error("sidereon_observe", err),
        }
    })
}

// --- Ground-observer Sun/Moon geometry --------------------------------------

/// A geodetic ground station for the Sun/Moon observation helpers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodeticStation {
    /// Geodetic latitude, degrees (positive north).
    pub latitude_deg: f64,
    /// Geodetic longitude, degrees (positive east).
    pub longitude_deg: f64,
    /// Height above the ellipsoid, kilometers.
    pub altitude_km: f64,
}

/// On-sky angular separation in degrees between two direction vectors. Each
/// vector must point to three finite doubles and must be non-zero.
///
/// Safety: a and b must point to three doubles; out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_angular_separation_deg(
    a: *const f64,
    b: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_angular_separation_deg",
        SidereonStatus::Panic,
        || {
            angle_scalar_vec3_2(
                "sidereon_angular_separation_deg",
                a,
                b,
                out,
                sidereon_core::astro::angles::angular_separation,
            )
        },
    )
}

/// On-sky angular separation in degrees between two coordinate pairs. Inputs are
/// `(lon_deg, lat_deg)` pairs, which also correspond to `(RA, Dec)` in the
/// astronomy convention. The second component must be in [-90, 90].
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_angular_separation_coords_deg(
    a_lon_deg: f64,
    a_lat_deg: f64,
    b_lon_deg: f64,
    b_lat_deg: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_angular_separation_coords_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_angular_separation_coords_deg",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::astro::angles::angular_separation_coords(
                (a_lon_deg, a_lat_deg),
                (b_lon_deg, b_lat_deg),
            ) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_angular_separation_coords_deg", err),
            }
        },
    )
}

/// Phase angle in degrees (Sun-satellite-observer). Delegates to
/// sidereon_core::astro::angles::phase_angle.
///
/// Safety: each pointer must point to 3 doubles; out to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_phase_angle_deg(
    sat_pos_km: *const f64,
    sun_pos_km: *const f64,
    observer_pos_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_phase_angle_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_phase_angle_deg", "out"));
        *out = 0.0;
        let sat = c_try!(read_vec3(
            "sidereon_phase_angle_deg",
            "sat_pos_km",
            sat_pos_km
        ));
        let sun = c_try!(read_vec3(
            "sidereon_phase_angle_deg",
            "sun_pos_km",
            sun_pos_km
        ));
        let obs = c_try!(read_vec3(
            "sidereon_phase_angle_deg",
            "observer_pos_km",
            observer_pos_km
        ));
        match sidereon_core::astro::angles::phase_angle(sat, sun, obs) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_phase_angle_deg", err),
        }
    })
}

/// Position angle in degrees, in [0, 360), of the `to` coordinate as seen from
/// `from`, measured from North through East. Inputs are `(lon_deg, lat_deg)`
/// pairs, which also correspond to `(RA, Dec)` in the astronomy convention.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_position_angle_deg(
    from_lon_deg: f64,
    from_lat_deg: f64,
    to_lon_deg: f64,
    to_lat_deg: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_position_angle_deg", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_position_angle_deg", "out"));
        *out = 0.0;
        match sidereon_core::astro::angles::position_angle(
            (from_lon_deg, from_lat_deg),
            (to_lon_deg, to_lat_deg),
        ) {
            Ok(value) => {
                *out = value;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_position_angle_deg", err),
        }
    })
}

unsafe fn sun_moon_epoch_batch(
    fn_name: &str,
    args: SunMoonEpochBatchArgs,
    compute: impl Fn(
        &sidereon_core::astro::time::scales::TimeScales,
    ) -> Result<
        sidereon_core::astro::bodies::SunMoon,
        sidereon_core::astro::bodies::SunMoonError,
    >,
) -> SidereonStatus {
    let epochs = c_try!(require_slice(
        args.epochs_unix_us,
        args.count,
        fn_name,
        "epochs_unix_us"
    ));
    let flat_len = c_try!(checked_epoch_vec3_output_len(fn_name, epochs.len()));
    let mut sun = Vec::with_capacity(flat_len);
    let mut moon = Vec::with_capacity(flat_len);
    for &epoch_unix_us in epochs {
        let ts = UtcInstant::from_unix_microseconds(epoch_unix_us).time_scales();
        let sm = match compute(&ts) {
            Ok(sm) => sm,
            Err(err) => return extra_invalid_arg(fn_name, err),
        };
        sun.extend_from_slice(&sm.sun);
        moon.extend_from_slice(&sm.moon);
    }
    c_try!(copy_exact_f64s(
        fn_name,
        "out_sun_m",
        args.out_sun_m,
        args.sun_len,
        &sun
    ));
    c_try!(copy_exact_f64s(
        fn_name,
        "out_moon_m",
        args.out_moon_m,
        args.moon_len,
        &moon
    ));
    SidereonStatus::Ok
}

// --- Nutation / precession (sidereon_core::astro::frames) --------------------

fn map_nutation_error(fn_name: &str, err: ft_nutation::NutationError) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

fn map_precession_error(fn_name: &str, err: ft_precession::PrecessionError) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

fn body_observation_to_c(
    value: sidereon_core::astro::bodies::Observation,
) -> SidereonBodyObservation {
    SidereonBodyObservation {
        astrometric: equatorial_to_c(value.astrometric),
        apparent_icrs: equatorial_to_c(value.apparent_icrs),
        apparent: equatorial_to_c(value.apparent),
        horizontal: horizontal_to_c(value.horizontal),
        hour_angle_deg: value.hour_angle_deg,
        hour_angle_hours: value.hour_angle_hours,
        ecliptic: ecliptic_to_c(value.ecliptic),
        reduced: value.reduced,
    }
}

fn observe_options_from_c(
    options: *const SidereonObserveOptions,
) -> Result<sidereon_core::astro::bodies::ObserveOptions, SidereonStatus> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(sidereon_core::astro::bodies::ObserveOptions::default());
    };
    Ok(sidereon_core::astro::bodies::ObserveOptions {
        polar_motion: options.has_polar_motion.then_some(
            sidereon_core::astro::frames::transforms::PolarMotion {
                xp_rad: options.xp_rad,
                yp_rad: options.yp_rad,
            },
        ),
        refraction: options
            .has_refraction
            .then_some(sidereon_core::astro::bodies::Refraction {
                pressure_mbar: options.refraction.pressure_mbar,
                temperature_c: options.refraction.temperature_c,
            }),
        deflection: options.deflection,
        aberration: options.aberration,
    })
}

fn map_observe_error(
    fn_name: &str,
    err: sidereon_core::astro::bodies::ObserveError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::Solve
}

fn map_almanac_error(
    fn_name: &str,
    err: sidereon_core::astro::almanac::AlmanacError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        sidereon_core::astro::almanac::AlmanacError::InvalidInput { .. }
        | sidereon_core::astro::almanac::AlmanacError::Finder(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn planet_from_c(
    fn_name: &str,
    value: u32,
) -> Result<sidereon_core::astro::almanac::Planet, SidereonStatus> {
    match SidereonPlanet::try_from(value) {
        Ok(SidereonPlanet::Mercury) => Ok(sidereon_core::astro::almanac::Planet::Mercury),
        Ok(SidereonPlanet::Venus) => Ok(sidereon_core::astro::almanac::Planet::Venus),
        Ok(SidereonPlanet::Mars) => Ok(sidereon_core::astro::almanac::Planet::Mars),
        Ok(SidereonPlanet::Jupiter) => Ok(sidereon_core::astro::almanac::Planet::Jupiter),
        Ok(SidereonPlanet::Saturn) => Ok(sidereon_core::astro::almanac::Planet::Saturn),
        Ok(SidereonPlanet::Uranus) => Ok(sidereon_core::astro::almanac::Planet::Uranus),
        Ok(SidereonPlanet::Neptune) => Ok(sidereon_core::astro::almanac::Planet::Neptune),
        Err(()) => {
            set_last_error(format!("{fn_name}: invalid planet {value}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn planet_to_c(value: sidereon_core::astro::almanac::Planet) -> SidereonPlanet {
    match value {
        sidereon_core::astro::almanac::Planet::Mercury => SidereonPlanet::Mercury,
        sidereon_core::astro::almanac::Planet::Venus => SidereonPlanet::Venus,
        sidereon_core::astro::almanac::Planet::Mars => SidereonPlanet::Mars,
        sidereon_core::astro::almanac::Planet::Jupiter => SidereonPlanet::Jupiter,
        sidereon_core::astro::almanac::Planet::Saturn => SidereonPlanet::Saturn,
        sidereon_core::astro::almanac::Planet::Uranus => SidereonPlanet::Uranus,
        sidereon_core::astro::almanac::Planet::Neptune => SidereonPlanet::Neptune,
        _ => SidereonPlanet::Mercury,
    }
}

fn planetary_kind_from_c(
    fn_name: &str,
    value: u32,
) -> Result<sidereon_core::astro::almanac::PlanetaryEventKind, SidereonStatus> {
    match SidereonPlanetaryEventKind::try_from(value) {
        Ok(SidereonPlanetaryEventKind::Conjunction) => {
            Ok(sidereon_core::astro::almanac::PlanetaryEventKind::Conjunction)
        }
        Ok(SidereonPlanetaryEventKind::Opposition) => {
            Ok(sidereon_core::astro::almanac::PlanetaryEventKind::Opposition)
        }
        Err(()) => {
            set_last_error(format!("{fn_name}: invalid planetary event kind {value}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn season_kind_to_c(value: sidereon_core::astro::almanac::SeasonKind) -> SidereonSeasonKind {
    match value {
        sidereon_core::astro::almanac::SeasonKind::MarchEquinox => SidereonSeasonKind::MarchEquinox,
        sidereon_core::astro::almanac::SeasonKind::JuneSolstice => SidereonSeasonKind::JuneSolstice,
        sidereon_core::astro::almanac::SeasonKind::SeptemberEquinox => {
            SidereonSeasonKind::SeptemberEquinox
        }
        sidereon_core::astro::almanac::SeasonKind::DecemberSolstice => {
            SidereonSeasonKind::DecemberSolstice
        }
        _ => SidereonSeasonKind::MarchEquinox,
    }
}

fn moon_phase_kind_to_c(
    value: sidereon_core::astro::almanac::MoonPhaseKind,
) -> SidereonMoonPhaseKind {
    match value {
        sidereon_core::astro::almanac::MoonPhaseKind::New => SidereonMoonPhaseKind::New,
        sidereon_core::astro::almanac::MoonPhaseKind::FirstQuarter => {
            SidereonMoonPhaseKind::FirstQuarter
        }
        sidereon_core::astro::almanac::MoonPhaseKind::Full => SidereonMoonPhaseKind::Full,
        sidereon_core::astro::almanac::MoonPhaseKind::LastQuarter => {
            SidereonMoonPhaseKind::LastQuarter
        }
        _ => SidereonMoonPhaseKind::New,
    }
}

fn almanac_eclipse_kind_to_c(
    value: sidereon_core::astro::almanac::EclipseKind,
) -> SidereonAlmanacEclipseKind {
    match value {
        sidereon_core::astro::almanac::EclipseKind::LunarPenumbral => {
            SidereonAlmanacEclipseKind::LunarPenumbral
        }
        sidereon_core::astro::almanac::EclipseKind::LunarPartial => {
            SidereonAlmanacEclipseKind::LunarPartial
        }
        sidereon_core::astro::almanac::EclipseKind::LunarTotal => {
            SidereonAlmanacEclipseKind::LunarTotal
        }
        sidereon_core::astro::almanac::EclipseKind::SolarPartial => {
            SidereonAlmanacEclipseKind::SolarPartial
        }
        sidereon_core::astro::almanac::EclipseKind::SolarAnnular => {
            SidereonAlmanacEclipseKind::SolarAnnular
        }
        sidereon_core::astro::almanac::EclipseKind::SolarTotal => {
            SidereonAlmanacEclipseKind::SolarTotal
        }
        sidereon_core::astro::almanac::EclipseKind::SolarHybrid => {
            SidereonAlmanacEclipseKind::SolarHybrid
        }
        _ => SidereonAlmanacEclipseKind::LunarPenumbral,
    }
}

fn surface_point_to_c(point: SurfacePoint) -> SidereonSurfacePoint {
    SidereonSurfacePoint {
        latitude_deg: point.latitude_deg,
        longitude_deg: point.longitude_deg,
    }
}

fn station_to_core(station: &SidereonGeodeticStation) -> GeodeticStationKm {
    GeodeticStationKm {
        latitude_deg: station.latitude_deg,
        longitude_deg: station.longitude_deg,
        altitude_km: station.altitude_km,
    }
}

fn body_az_el_to_c(azel: sidereon_core::astro::bodies::BodyAzEl) -> SidereonBodyAzEl {
    SidereonBodyAzEl {
        azimuth_deg: azel.azimuth_deg,
        elevation_deg: azel.elevation_deg,
        range_km: azel.range_km,
    }
}

/// Map a ground-observer body error to a status code. A station/frame input
/// failure reports SIDEREON_STATUS_INVALID_ARGUMENT; an ephemeris or
/// phase-angle geometry failure reports SIDEREON_STATUS_SOLVE.
fn map_body_observation_error(fn_name: &str, err: BodyObservationError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        BodyObservationError::FrameTransform(_) => SidereonStatus::InvalidArgument,
        BodyObservationError::Ephemeris(_) | BodyObservationError::Angle(_) => {
            SidereonStatus::Solve
        }
    }
}

/// Map an event-finder error to a status code. Its only cause is an invalid
/// window/cadence input, reported as SIDEREON_STATUS_INVALID_ARGUMENT.
fn map_event_finder_error(fn_name: &str, err: EventFinderError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

unsafe fn moon_elevation_options_from_c(
    fn_name: &str,
    options: *const SidereonMoonElevationOptions,
) -> Result<CoreMoonElevationOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(CoreMoonElevationOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    Ok(CoreMoonElevationOptions {
        elevation_threshold_deg: options.elevation_threshold_deg,
        step_seconds: options.step_seconds,
        time_tolerance_seconds: options.time_tolerance_seconds,
    })
}

fn checked_epoch_vec3_output_len(fn_name: &str, count: usize) -> Result<usize, SidereonStatus> {
    if count == 0 {
        set_last_error(format!("{fn_name}: epochs_unix_us must not be empty"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let len = count.checked_mul(3).ok_or_else(|| {
        set_last_error(format!("{fn_name}: epoch count is too large"));
        SidereonStatus::InvalidArgument
    })?;
    validate_element_count::<f64>(fn_name, "epoch_count", len)?;
    Ok(len)
}

fn equatorial_to_c(value: sidereon_core::astro::bodies::Equatorial) -> SidereonEquatorial {
    SidereonEquatorial {
        right_ascension_deg: value.right_ascension_deg,
        right_ascension_hours: value.right_ascension_hours,
        declination_deg: value.declination_deg,
        distance_km: value.distance_km,
    }
}

fn horizontal_to_c(value: sidereon_core::astro::bodies::Horizontal) -> SidereonHorizontal {
    SidereonHorizontal {
        azimuth_deg: value.azimuth_deg,
        elevation_deg: value.elevation_deg,
        range_km: value.range_km,
    }
}

fn ecliptic_to_c(value: sidereon_core::astro::bodies::Ecliptic) -> SidereonEcliptic {
    SidereonEcliptic {
        longitude_deg: value.longitude_deg,
        latitude_deg: value.latitude_deg,
        distance_km: value.distance_km,
    }
}
