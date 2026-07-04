use super::*;

/// Convert a 3x3 RTN covariance matrix to ECI. Delegates to
/// sidereon_core::astro::covariance::rtn_to_eci.
///
/// Safety: cov_rtn and out_cov_eci point to 9 doubles each; r_km and v_km_s
/// point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtn_to_eci_covariance(
    cov_rtn: *const f64,
    r_km: *const f64,
    v_km_s: *const f64,
    out_cov_eci: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtn_to_eci_covariance",
        SidereonStatus::Panic,
        || {
            let cov_rtn = c_try!(read_mat3(
                "sidereon_rtn_to_eci_covariance",
                "cov_rtn",
                cov_rtn
            ));
            let r = c_try!(read_vec3("sidereon_rtn_to_eci_covariance", "r_km", r_km));
            let v = c_try!(read_vec3(
                "sidereon_rtn_to_eci_covariance",
                "v_km_s",
                v_km_s
            ));
            match sidereon_core::astro::covariance::rtn_to_eci(&cov_rtn, r, v) {
                Ok(eci) => {
                    c_try!(copy_exact_f64s(
                        "sidereon_rtn_to_eci_covariance",
                        "out_cov_eci",
                        out_cov_eci,
                        9,
                        &flatten_mat3(eci),
                    ));
                    SidereonStatus::Ok
                }
                Err(err) => {
                    set_last_error(format!("sidereon_rtn_to_eci_covariance: {}", err.message()));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

// --- Lambert transfer (sidereon_core::astro::lambert) ------------------------

/// Battin Lambert boundary-value solve. dm: 0 short-way, 1 long-way. de: 0 low-
/// energy, 1 high-energy. nrev: number of complete revolutions. Writes the
/// departure and arrival transfer velocities (km/s). Delegates to
/// sidereon_core::astro::lambert::battin.
///
/// Safety: r1/r2/v1_km(_s) point to 3 doubles each; out_v1/out_v2 point to 3.
#[no_mangle]
pub unsafe extern "C" fn sidereon_lambert_battin(
    r1_km: *const f64,
    r2_km: *const f64,
    v1_km_s: *const f64,
    dm: u32,
    de: u32,
    nrev: i32,
    dtsec: f64,
    out_v1_km_s: *mut f64,
    out_v2_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_lambert_battin", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_lambert_battin",
            "out_v1_km_s",
            out_v1_km_s,
            3,
            &[0.0, 0.0, 0.0]
        ));
        c_try!(copy_exact_f64s(
            "sidereon_lambert_battin",
            "out_v2_km_s",
            out_v2_km_s,
            3,
            &[0.0, 0.0, 0.0]
        ));
        let r1 = c_try!(read_vec3("sidereon_lambert_battin", "r1_km", r1_km));
        let r2 = c_try!(read_vec3("sidereon_lambert_battin", "r2_km", r2_km));
        let v1 = c_try!(read_vec3("sidereon_lambert_battin", "v1_km_s", v1_km_s));
        let dm = match dm {
            0 => sidereon_core::astro::lambert::DirectionOfMotion::Short,
            1 => sidereon_core::astro::lambert::DirectionOfMotion::Long,
            _ => {
                set_last_error("sidereon_lambert_battin: dm must be 0 or 1".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        let de = match de {
            0 => sidereon_core::astro::lambert::DirectionOfEnergy::Low,
            1 => sidereon_core::astro::lambert::DirectionOfEnergy::High,
            _ => {
                set_last_error("sidereon_lambert_battin: de must be 0 or 1".to_string());
                return SidereonStatus::InvalidArgument;
            }
        };
        match sidereon_core::astro::lambert::battin(&r1, &r2, &v1, dm, de, nrev, dtsec) {
            Ok((vt1, vt2)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_lambert_battin",
                    "out_v1_km_s",
                    out_v1_km_s,
                    3,
                    &vt1
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_lambert_battin",
                    "out_v2_km_s",
                    out_v2_km_s,
                    3,
                    &vt2
                ));
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_lambert_battin", err),
        }
    })
}

/// Broadcast Keplerian orbital elements (SI units; radians; toe_sow in seconds
/// of week), mirroring sidereon_core::ephemeris::KeplerianElements.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonKeplerianElements {
    /// Square root of the semi-major axis (sqrt(m)).
    pub sqrt_a: f64,
    /// Eccentricity (dimensionless).
    pub e: f64,
    /// Mean anomaly at reference time (rad).
    pub m0: f64,
    /// Mean motion difference (rad/s).
    pub delta_n: f64,
    /// Longitude of ascending node at weekly epoch (rad).
    pub omega0: f64,
    /// Inclination at reference time (rad).
    pub i0: f64,
    /// Argument of perigee (rad).
    pub omega: f64,
    /// Rate of right ascension (rad/s).
    pub omega_dot: f64,
    /// Rate of inclination (rad/s).
    pub idot: f64,
    /// Latitude argument cosine correction (rad).
    pub cuc: f64,
    /// Latitude argument sine correction (rad).
    pub cus: f64,
    /// Orbit radius cosine correction (m).
    pub crc: f64,
    /// Orbit radius sine correction (m).
    pub crs: f64,
    /// Inclination cosine correction (rad).
    pub cic: f64,
    /// Inclination sine correction (rad).
    pub cis: f64,
    /// Ephemeris reference time, seconds of week.
    pub toe_sow: f64,
}

/// The full intermediate substrate of a broadcast orbit evaluation, mirroring
/// sidereon_core::ephemeris::OrbitState. Every field is exposed for parity.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitState {
    /// Semi-major axis (m).
    pub a: f64,
    /// Computed mean motion (rad/s).
    pub n0: f64,
    /// Corrected mean motion (rad/s).
    pub n: f64,
    /// Time from ephemeris reference epoch, half-week folded (s).
    pub tk: f64,
    /// Mean anomaly (rad).
    pub mk: f64,
    /// Eccentric anomaly (rad).
    pub eccentric_anomaly: f64,
    /// Number of Kepler iterations.
    pub kepler_iterations: usize,
    /// sin(E).
    pub sin_e: f64,
    /// cos(E).
    pub cos_e: f64,
    /// True anomaly (rad).
    pub nu: f64,
    /// Argument of latitude before correction (rad).
    pub phi: f64,
    /// sin(2*phi).
    pub s2: f64,
    /// cos(2*phi).
    pub c2: f64,
    /// Argument-of-latitude correction (rad).
    pub du: f64,
    /// Radius correction (m).
    pub dr: f64,
    /// Inclination correction (rad).
    pub di: f64,
    /// Corrected argument of latitude (rad).
    pub u: f64,
    /// Corrected radius (m).
    pub r: f64,
    /// Corrected inclination (rad).
    pub i: f64,
    /// Orbital-plane x (m).
    pub xp: f64,
    /// Orbital-plane y (m).
    pub yp: f64,
    /// Corrected longitude of ascending node (rad).
    pub omega_k: f64,
    /// ECEF x (m).
    pub x_m: f64,
    /// ECEF y (m).
    pub y_m: f64,
    /// ECEF z (m).
    pub z_m: f64,
}

// --- Classical orbital elements (sidereon_core::astro::elements) -------------

/// Geometric classification of a two-body orbit, mirroring
/// sidereon_core::astro::elements::OrbitType. Pass as a uint32_t in
/// SidereonClassicalElements.orbit_type.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonOrbitType {
    /// Eccentric and inclined: all six classical elements are defined.
    EllipticalInclined = 0,
    /// Eccentric but equatorial: lonper replaces the ascending node.
    EllipticalEquatorial = 1,
    /// Circular but inclined: arglat replaces the argument of perigee.
    CircularInclined = 2,
    /// Circular and equatorial: truelon replaces node and argument of perigee.
    CircularEquatorial = 3,
}

/// Classical (Keplerian) orbital elements, mirroring
/// sidereon_core::astro::elements::ClassicalElements. Angles are radians;
/// undefined auxiliary angles are reported as NaN. `orbit_type` is a
/// SidereonOrbitType value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClassicalElements {
    /// Semi-latus rectum p = h^2 / mu (km).
    pub p: f64,
    /// Semi-major axis a (km); INFINITY for a parabolic orbit.
    pub a: f64,
    /// Eccentricity.
    pub ecc: f64,
    /// Inclination in [0, pi] (rad).
    pub incl: f64,
    /// Right ascension of the ascending node (rad); NaN for equatorial orbits.
    pub raan: f64,
    /// Argument of perigee (rad); NaN for circular orbits.
    pub argp: f64,
    /// True anomaly (rad); NaN for circular orbits.
    pub nu: f64,
    /// Argument of latitude (rad); NaN unless circular inclined.
    pub arglat: f64,
    /// True longitude (rad); NaN unless circular equatorial.
    pub truelon: f64,
    /// Longitude of perigee (rad); NaN unless elliptical equatorial.
    pub lonper: f64,
    /// Geometric classification, a SidereonOrbitType value.
    pub orbit_type: u32,
}

/// Convert an inertial Cartesian state to classical orbital elements. `r_km` and
/// `v_km_s` are the ECI position (km) and velocity (km/s); `mu_km3_s2` is the
/// gravitational parameter (km^3/s^2). Writes *out on success. Delegates to
/// sidereon_core::astro::elements::rv2coe.
///
/// Safety: r_km and v_km_s point to 3 doubles each; out points to a
/// SidereonClassicalElements.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rv2coe(
    r_km: *const f64,
    v_km_s: *const f64,
    mu_km3_s2: f64,
    out: *mut SidereonClassicalElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_rv2coe", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rv2coe", "out"));
        let r = c_try!(read_vec3("sidereon_rv2coe", "r_km", r_km));
        let v = c_try!(read_vec3("sidereon_rv2coe", "v_km_s", v_km_s));
        match rv2coe(r, v, mu_km3_s2) {
            Ok(coe) => {
                *out = classical_elements_to_c(&coe);
                SidereonStatus::Ok
            }
            Err(err) => map_elements_error("sidereon_rv2coe", err),
        }
    })
}

/// Convert classical orbital elements to an inertial Cartesian state. Writes the
/// position (km) to out_r_km and velocity (km/s) to out_v_km_s. Delegates to
/// sidereon_core::astro::elements::coe2rv.
///
/// Safety: coe points to a SidereonClassicalElements; out_r_km and out_v_km_s
/// point to 3 doubles each.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coe2rv(
    coe: *const SidereonClassicalElements,
    mu_km3_s2: f64,
    out_r_km: *mut f64,
    out_v_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_coe2rv", SidereonStatus::Panic, || {
        c_try!(copy_exact_f64s(
            "sidereon_coe2rv",
            "out_r_km",
            out_r_km,
            3,
            &[0.0, 0.0, 0.0]
        ));
        c_try!(copy_exact_f64s(
            "sidereon_coe2rv",
            "out_v_km_s",
            out_v_km_s,
            3,
            &[0.0, 0.0, 0.0]
        ));
        let coe = c_try!(require_ref(coe, "sidereon_coe2rv", "coe"));
        let elements = c_try!(classical_elements_from_c("sidereon_coe2rv", coe));
        match coe2rv(&elements, mu_km3_s2) {
            Ok((r, v)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_coe2rv",
                    "out_r_km",
                    out_r_km,
                    3,
                    &r
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_coe2rv",
                    "out_v_km_s",
                    out_v_km_s,
                    3,
                    &v
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_elements_error("sidereon_coe2rv", err),
        }
    })
}

// --- Anomaly conversions (sidereon_core::astro::anomaly) --------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonKeplerSolution {
    pub anomaly_rad: f64,
    pub iterations: usize,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_mean_to_eccentric_anomaly(
    mean_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mean_to_eccentric_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_mean_to_eccentric_anomaly",
                mean_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::mean_to_eccentric,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_eccentric_to_mean_anomaly(
    eccentric_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_eccentric_to_mean_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_eccentric_to_mean_anomaly",
                eccentric_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::eccentric_to_mean,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_eccentric_to_true_anomaly(
    eccentric_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_eccentric_to_true_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_eccentric_to_true_anomaly",
                eccentric_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::eccentric_to_true,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_true_to_eccentric_anomaly(
    true_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_true_to_eccentric_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_true_to_eccentric_anomaly",
                true_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::true_to_eccentric,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_mean_to_true_anomaly(
    mean_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mean_to_true_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_mean_to_true_anomaly",
                mean_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::mean_to_true,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_true_to_mean_anomaly(
    true_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_true_to_mean_anomaly",
        SidereonStatus::Panic,
        || {
            anomaly_scalar(
                "sidereon_true_to_mean_anomaly",
                true_anomaly_rad,
                eccentricity,
                out,
                sidereon_core::astro::anomaly::true_to_mean,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_solve_kepler(
    mean_anomaly_rad: f64,
    eccentricity: f64,
    out: *mut SidereonKeplerSolution,
) -> SidereonStatus {
    ffi_boundary("sidereon_solve_kepler", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_solve_kepler", "out"));
        *out = SidereonKeplerSolution {
            anomaly_rad: 0.0,
            iterations: 0,
        };
        match sidereon_core::astro::anomaly::solve_kepler(mean_anomaly_rad, eccentricity) {
            Ok(solution) => {
                *out = SidereonKeplerSolution {
                    anomaly_rad: solution.anomaly,
                    iterations: solution.iterations,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_anomaly_error("sidereon_solve_kepler", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_kepler(
    coe: *const SidereonClassicalElements,
    mu_km3_s2: f64,
    dt_s: f64,
    out: *mut SidereonClassicalElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_propagate_kepler", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_propagate_kepler", "out"));
        let coe = c_try!(require_ref(coe, "sidereon_propagate_kepler", "coe"));
        let elements = c_try!(classical_elements_from_c("sidereon_propagate_kepler", coe));
        match sidereon_core::astro::anomaly::propagate_kepler(&elements, mu_km3_s2, dt_s) {
            Ok(next) => {
                *out = classical_elements_to_c(&next);
                SidereonStatus::Ok
            }
            Err(err) => map_anomaly_error("sidereon_propagate_kepler", err),
        }
    })
}

// --- Equinoctial elements (sidereon_core::astro::equinoctial) ---------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRetrogradeFactor {
    Prograde = 0,
    Retrograde = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEquinoctialElements {
    pub a: f64,
    pub h: f64,
    pub k: f64,
    pub p: f64,
    pub q: f64,
    pub lambda: f64,
    pub retrograde: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonModifiedEquinoctialElements {
    pub p: f64,
    pub f: f64,
    pub g: f64,
    pub h: f64,
    pub k: f64,
    pub l: f64,
    pub retrograde: u32,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_coe2eq(
    coe: *const SidereonClassicalElements,
    retrograde: u32,
    out: *mut SidereonEquinoctialElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_coe2eq", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_coe2eq", "out"));
        let coe = c_try!(require_ref(coe, "sidereon_coe2eq", "coe"));
        let coe = c_try!(classical_elements_from_c("sidereon_coe2eq", coe));
        let factor = c_try!(retrograde_factor_from_c("sidereon_coe2eq", retrograde));
        match sidereon_core::astro::equinoctial::coe2eq(&coe, factor) {
            Ok(eq) => {
                *out = equinoctial_to_c(eq);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_coe2eq", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_eq2coe(
    eq: *const SidereonEquinoctialElements,
    out: *mut SidereonClassicalElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_eq2coe", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_eq2coe", "out"));
        let eq = c_try!(require_ref(eq, "sidereon_eq2coe", "eq"));
        let eq = c_try!(equinoctial_from_c("sidereon_eq2coe", eq));
        match sidereon_core::astro::equinoctial::eq2coe(&eq) {
            Ok(coe) => {
                *out = classical_elements_to_c(&coe);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_eq2coe", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_coe2mee(
    coe: *const SidereonClassicalElements,
    retrograde: u32,
    out: *mut SidereonModifiedEquinoctialElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_coe2mee", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_coe2mee", "out"));
        let coe = c_try!(require_ref(coe, "sidereon_coe2mee", "coe"));
        let coe = c_try!(classical_elements_from_c("sidereon_coe2mee", coe));
        let factor = c_try!(retrograde_factor_from_c("sidereon_coe2mee", retrograde));
        match sidereon_core::astro::equinoctial::coe2mee(&coe, factor) {
            Ok(mee) => {
                *out = modified_equinoctial_to_c(mee);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_coe2mee", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_mee2coe(
    mee: *const SidereonModifiedEquinoctialElements,
    out: *mut SidereonClassicalElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_mee2coe", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_mee2coe", "out"));
        let mee = c_try!(require_ref(mee, "sidereon_mee2coe", "mee"));
        let mee = c_try!(modified_equinoctial_from_c("sidereon_mee2coe", mee));
        match sidereon_core::astro::equinoctial::mee2coe(&mee) {
            Ok(coe) => {
                *out = classical_elements_to_c(&coe);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_mee2coe", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rv2eq(
    r_km: *const f64,
    v_km_s: *const f64,
    mu_km3_s2: f64,
    retrograde: u32,
    out: *mut SidereonEquinoctialElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_rv2eq", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rv2eq", "out"));
        let r = c_try!(read_vec3("sidereon_rv2eq", "r_km", r_km));
        let v = c_try!(read_vec3("sidereon_rv2eq", "v_km_s", v_km_s));
        let factor = c_try!(retrograde_factor_from_c("sidereon_rv2eq", retrograde));
        match sidereon_core::astro::equinoctial::rv2eq(r, v, mu_km3_s2, factor) {
            Ok(eq) => {
                *out = equinoctial_to_c(eq);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_rv2eq", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_eq2rv(
    eq: *const SidereonEquinoctialElements,
    mu_km3_s2: f64,
    out_r_km: *mut f64,
    out_v_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_eq2rv", SidereonStatus::Panic, || {
        let eq = c_try!(require_ref(eq, "sidereon_eq2rv", "eq"));
        let eq = c_try!(equinoctial_from_c("sidereon_eq2rv", eq));
        match sidereon_core::astro::equinoctial::eq2rv(&eq, mu_km3_s2) {
            Ok((r, v)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_eq2rv",
                    "out_r_km",
                    out_r_km,
                    3,
                    &r
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_eq2rv",
                    "out_v_km_s",
                    out_v_km_s,
                    3,
                    &v
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_eq2rv", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rv2mee(
    r_km: *const f64,
    v_km_s: *const f64,
    mu_km3_s2: f64,
    retrograde: u32,
    out: *mut SidereonModifiedEquinoctialElements,
) -> SidereonStatus {
    ffi_boundary("sidereon_rv2mee", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rv2mee", "out"));
        let r = c_try!(read_vec3("sidereon_rv2mee", "r_km", r_km));
        let v = c_try!(read_vec3("sidereon_rv2mee", "v_km_s", v_km_s));
        let factor = c_try!(retrograde_factor_from_c("sidereon_rv2mee", retrograde));
        match sidereon_core::astro::equinoctial::rv2mee(r, v, mu_km3_s2, factor) {
            Ok(mee) => {
                *out = modified_equinoctial_to_c(mee);
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_rv2mee", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_mee2rv(
    mee: *const SidereonModifiedEquinoctialElements,
    mu_km3_s2: f64,
    out_r_km: *mut f64,
    out_v_km_s: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_mee2rv", SidereonStatus::Panic, || {
        let mee = c_try!(require_ref(mee, "sidereon_mee2rv", "mee"));
        let mee = c_try!(modified_equinoctial_from_c("sidereon_mee2rv", mee));
        match sidereon_core::astro::equinoctial::mee2rv(&mee, mu_km3_s2) {
            Ok((r, v)) => {
                c_try!(copy_exact_f64s(
                    "sidereon_mee2rv",
                    "out_r_km",
                    out_r_km,
                    3,
                    &r
                ));
                c_try!(copy_exact_f64s(
                    "sidereon_mee2rv",
                    "out_v_km_s",
                    out_v_km_s,
                    3,
                    &v
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_equinoctial_error("sidereon_mee2rv", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_beta_angle_deg(
    orbit_normal: *const f64,
    sun: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_beta_angle_deg", SidereonStatus::Panic, || {
        angle_scalar_vec3_2(
            "sidereon_beta_angle_deg",
            orbit_normal,
            sun,
            out,
            sidereon_core::astro::angles::beta_angle,
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_beta_angle_from_state_deg(
    r_km: *const f64,
    v_km_s: *const f64,
    sun_km: *const f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_beta_angle_from_state_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_beta_angle_from_state_deg",
                "out"
            ));
            *out = 0.0;
            let r = c_try!(read_vec3(
                "sidereon_beta_angle_from_state_deg",
                "r_km",
                r_km
            ));
            let v = c_try!(read_vec3(
                "sidereon_beta_angle_from_state_deg",
                "v_km_s",
                v_km_s
            ));
            let sun = c_try!(read_vec3(
                "sidereon_beta_angle_from_state_deg",
                "sun_km",
                sun_km
            ));
            match sidereon_core::astro::angles::beta_angle_from_state(r, v, sun) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_beta_angle_from_state_deg", err),
            }
        },
    )
}

// --- Relative frames and CW motion (sidereon_core::astro::relative) ----------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRelativeFrame {
    Rsw = 0,
    Rtn = 1,
    Ric = 2,
    Lvlh = 3,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_absolute_from_relative(
    chief: *const SidereonCartesianState,
    rel: *const SidereonCartesianState,
    out: *mut SidereonCartesianState,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_absolute_from_relative",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_absolute_from_relative", "out"));
            let chief = c_try!(require_ref(
                chief,
                "sidereon_absolute_from_relative",
                "chief"
            ));
            let rel = c_try!(require_ref(rel, "sidereon_absolute_from_relative", "rel"));
            match sidereon_core::astro::relative::absolute_from_relative(
                &cartesian_state_from_c(chief),
                &cartesian_state_from_c(rel),
            ) {
                Ok(state) => {
                    *out = cartesian_state_to_c(&state);
                    SidereonStatus::Ok
                }
                Err(err) => map_relative_error("sidereon_absolute_from_relative", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_cw_stm(
    mean_motion_rad_s: f64,
    dt_s: f64,
    out_row_major: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_cw_stm", SidereonStatus::Panic, || {
        match sidereon_core::astro::relative::cw_stm(mean_motion_rad_s, dt_s) {
            Ok(matrix) => {
                let mut flat = [0.0_f64; 36];
                for row in 0..6 {
                    for col in 0..6 {
                        flat[row * 6 + col] = matrix[row][col];
                    }
                }
                c_try!(copy_exact_f64s(
                    "sidereon_cw_stm",
                    "out_row_major",
                    out_row_major,
                    len,
                    &flat
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_relative_error("sidereon_cw_stm", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_cw_propagate(
    rel: *const SidereonCartesianState,
    mean_motion_rad_s: f64,
    dt_s: f64,
    out: *mut SidereonCartesianState,
) -> SidereonStatus {
    ffi_boundary("sidereon_cw_propagate", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_cw_propagate", "out"));
        let rel = c_try!(require_ref(rel, "sidereon_cw_propagate", "rel"));
        match sidereon_core::astro::relative::cw_propagate(
            &cartesian_state_from_c(rel),
            mean_motion_rad_s,
            dt_s,
        ) {
            Ok(state) => {
                *out = cartesian_state_to_c(&state);
                SidereonStatus::Ok
            }
            Err(err) => map_relative_error("sidereon_cw_propagate", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_relative_mean_motion_circular(
    radius_km: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_relative_mean_motion_circular",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_relative_mean_motion_circular",
                "out"
            ));
            *out = 0.0;
            match sidereon_core::astro::relative::mean_motion_circular(radius_km) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_relative_error("sidereon_relative_mean_motion_circular", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_relative_mean_motion_from_state(
    chief: *const SidereonCartesianState,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_relative_mean_motion_from_state",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_relative_mean_motion_from_state",
                "out"
            ));
            *out = 0.0;
            let chief = c_try!(require_ref(
                chief,
                "sidereon_relative_mean_motion_from_state",
                "chief"
            ));
            match sidereon_core::astro::relative::mean_motion_from_state(&cartesian_state_from_c(
                chief,
            )) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_relative_error("sidereon_relative_mean_motion_from_state", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_relative_rotation(
    frame: u32,
    chief: *const SidereonCartesianState,
    out_row_major: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_relative_rotation", SidereonStatus::Panic, || {
        let frame = c_try!(relative_frame_from_c("sidereon_relative_rotation", frame));
        let chief = c_try!(require_ref(chief, "sidereon_relative_rotation", "chief"));
        let chief = cartesian_state_from_c(chief);
        let rotation = match frame {
            SidereonRelativeFrame::Rsw => {
                sidereon_core::astro::relative::rsw_to_inertial_rotation(&chief)
            }
            SidereonRelativeFrame::Rtn => {
                sidereon_core::astro::relative::rtn_to_inertial_rotation(&chief)
            }
            SidereonRelativeFrame::Ric => {
                sidereon_core::astro::relative::ric_to_inertial_rotation(&chief)
            }
            SidereonRelativeFrame::Lvlh => {
                sidereon_core::astro::relative::lvlh_to_inertial_rotation(&chief)
            }
        };
        match rotation {
            Ok(matrix) => {
                let flat = [
                    matrix[0][0],
                    matrix[0][1],
                    matrix[0][2],
                    matrix[1][0],
                    matrix[1][1],
                    matrix[1][2],
                    matrix[2][0],
                    matrix[2][1],
                    matrix[2][2],
                ];
                c_try!(copy_exact_f64s(
                    "sidereon_relative_rotation",
                    "out_row_major",
                    out_row_major,
                    len,
                    &flat
                ));
                SidereonStatus::Ok
            }
            Err(err) => map_relative_error("sidereon_relative_rotation", err),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_relative_state(
    chief: *const SidereonCartesianState,
    deputy: *const SidereonCartesianState,
    out: *mut SidereonCartesianState,
) -> SidereonStatus {
    ffi_boundary("sidereon_relative_state", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_relative_state", "out"));
        let chief = c_try!(require_ref(chief, "sidereon_relative_state", "chief"));
        let deputy = c_try!(require_ref(deputy, "sidereon_relative_state", "deputy"));
        match sidereon_core::astro::relative::relative_state(
            &cartesian_state_from_c(chief),
            &cartesian_state_from_c(deputy),
        ) {
            Ok(state) => {
                *out = cartesian_state_to_c(&state);
                SidereonStatus::Ok
            }
            Err(err) => map_relative_error("sidereon_relative_state", err),
        }
    })
}

impl SidereonKeplerianElements {
    pub(crate) fn to_core(self) -> CoreKeplerianElements {
        CoreKeplerianElements {
            sqrt_a: self.sqrt_a,
            e: self.e,
            m0: self.m0,
            delta_n: self.delta_n,
            omega0: self.omega0,
            i0: self.i0,
            omega: self.omega,
            omega_dot: self.omega_dot,
            idot: self.idot,
            cuc: self.cuc,
            cus: self.cus,
            crc: self.crc,
            crs: self.crs,
            cic: self.cic,
            cis: self.cis,
            toe_sow: self.toe_sow,
        }
    }
}

impl SidereonOrbitState {
    pub(crate) fn from_core(o: &CoreOrbitState) -> Self {
        Self {
            a: o.a,
            n0: o.n0,
            n: o.n,
            tk: o.tk,
            mk: o.mk,
            eccentric_anomaly: o.eccentric_anomaly,
            kepler_iterations: o.kepler_iterations,
            sin_e: o.sin_e,
            cos_e: o.cos_e,
            nu: o.nu,
            phi: o.phi,
            s2: o.s2,
            c2: o.c2,
            du: o.du,
            dr: o.dr,
            di: o.di,
            u: o.u,
            r: o.r,
            i: o.i,
            xp: o.xp,
            yp: o.yp,
            omega_k: o.omega_k,
            x_m: o.x_m,
            y_m: o.y_m,
            z_m: o.z_m,
        }
    }

    pub(crate) const ZERO: Self = Self {
        a: 0.0,
        n0: 0.0,
        n: 0.0,
        tk: 0.0,
        mk: 0.0,
        eccentric_anomaly: 0.0,
        kepler_iterations: 0,
        sin_e: 0.0,
        cos_e: 0.0,
        nu: 0.0,
        phi: 0.0,
        s2: 0.0,
        c2: 0.0,
        du: 0.0,
        dr: 0.0,
        di: 0.0,
        u: 0.0,
        r: 0.0,
        i: 0.0,
        xp: 0.0,
        yp: 0.0,
        omega_k: 0.0,
        x_m: 0.0,
        y_m: 0.0,
        z_m: 0.0,
    };
}

fn classical_elements_to_c(coe: &ClassicalElements) -> SidereonClassicalElements {
    SidereonClassicalElements {
        p: coe.p,
        a: coe.a,
        ecc: coe.ecc,
        incl: coe.incl,
        raan: coe.raan,
        argp: coe.argp,
        nu: coe.nu,
        arglat: coe.arglat,
        truelon: coe.truelon,
        lonper: coe.lonper,
        orbit_type: orbit_type_to_code(coe.orbit_type),
    }
}

fn classical_elements_from_c(
    fn_name: &str,
    coe: &SidereonClassicalElements,
) -> Result<ClassicalElements, SidereonStatus> {
    Ok(ClassicalElements {
        p: coe.p,
        a: coe.a,
        ecc: coe.ecc,
        incl: coe.incl,
        raan: coe.raan,
        argp: coe.argp,
        nu: coe.nu,
        arglat: coe.arglat,
        truelon: coe.truelon,
        lonper: coe.lonper,
        orbit_type: orbit_type_from_code(fn_name, coe.orbit_type)?,
    })
}

fn map_elements_error(fn_name: &str, err: ElementsError) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

unsafe fn anomaly_scalar(
    fn_name: &str,
    a: f64,
    ecc: f64,
    out: *mut f64,
    f: fn(f64, f64) -> Result<f64, sidereon_core::astro::anomaly::AnomalyError>,
) -> SidereonStatus {
    let out = c_try!(require_out(out, fn_name, "out"));
    *out = 0.0;
    match f(a, ecc) {
        Ok(value) => {
            *out = value;
            SidereonStatus::Ok
        }
        Err(err) => map_anomaly_error(fn_name, err),
    }
}

fn equinoctial_to_c(
    eq: sidereon_core::astro::equinoctial::EquinoctialElements,
) -> SidereonEquinoctialElements {
    SidereonEquinoctialElements {
        a: eq.a,
        h: eq.h,
        k: eq.k,
        p: eq.p,
        q: eq.q,
        lambda: eq.lambda,
        retrograde: retrograde_factor_to_c(eq.retrograde),
    }
}

fn equinoctial_from_c(
    fn_name: &str,
    eq: &SidereonEquinoctialElements,
) -> Result<sidereon_core::astro::equinoctial::EquinoctialElements, SidereonStatus> {
    Ok(sidereon_core::astro::equinoctial::EquinoctialElements {
        a: eq.a,
        h: eq.h,
        k: eq.k,
        p: eq.p,
        q: eq.q,
        lambda: eq.lambda,
        retrograde: retrograde_factor_from_c(fn_name, eq.retrograde)?,
    })
}

fn modified_equinoctial_to_c(
    mee: sidereon_core::astro::equinoctial::ModifiedEquinoctialElements,
) -> SidereonModifiedEquinoctialElements {
    SidereonModifiedEquinoctialElements {
        p: mee.p,
        f: mee.f,
        g: mee.g,
        h: mee.h,
        k: mee.k,
        l: mee.l,
        retrograde: retrograde_factor_to_c(mee.retrograde),
    }
}

fn modified_equinoctial_from_c(
    fn_name: &str,
    mee: &SidereonModifiedEquinoctialElements,
) -> Result<sidereon_core::astro::equinoctial::ModifiedEquinoctialElements, SidereonStatus> {
    Ok(
        sidereon_core::astro::equinoctial::ModifiedEquinoctialElements {
            p: mee.p,
            f: mee.f,
            g: mee.g,
            h: mee.h,
            k: mee.k,
            l: mee.l,
            retrograde: retrograde_factor_from_c(fn_name, mee.retrograde)?,
        },
    )
}

fn map_equinoctial_error(
    fn_name: &str,
    err: sidereon_core::astro::equinoctial::EquinoctialError,
) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

fn relative_frame_from_c(
    fn_name: &str,
    value: u32,
) -> Result<SidereonRelativeFrame, SidereonStatus> {
    SidereonRelativeFrame::try_from(value).map_err(|_| {
        set_last_error(format!("{fn_name}: invalid relative frame {value}"));
        SidereonStatus::InvalidArgument
    })
}

fn map_relative_error(
    fn_name: &str,
    err: sidereon_core::astro::covariance::RtnFrameError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err:?}"));
    SidereonStatus::InvalidArgument
}

fn orbit_type_to_code(orbit_type: OrbitType) -> u32 {
    (match orbit_type {
        OrbitType::EllipticalInclined => SidereonOrbitType::EllipticalInclined,
        OrbitType::EllipticalEquatorial => SidereonOrbitType::EllipticalEquatorial,
        OrbitType::CircularInclined => SidereonOrbitType::CircularInclined,
        OrbitType::CircularEquatorial => SidereonOrbitType::CircularEquatorial,
    }) as u32
}

fn orbit_type_from_code(fn_name: &str, value: u32) -> Result<OrbitType, SidereonStatus> {
    match value {
        v if v == SidereonOrbitType::EllipticalInclined as u32 => Ok(OrbitType::EllipticalInclined),
        v if v == SidereonOrbitType::EllipticalEquatorial as u32 => {
            Ok(OrbitType::EllipticalEquatorial)
        }
        v if v == SidereonOrbitType::CircularInclined as u32 => Ok(OrbitType::CircularInclined),
        v if v == SidereonOrbitType::CircularEquatorial as u32 => Ok(OrbitType::CircularEquatorial),
        _ => {
            set_last_error(format!("{fn_name}: invalid orbit_type code {value}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn map_anomaly_error(
    fn_name: &str,
    err: sidereon_core::astro::anomaly::AnomalyError,
) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

fn retrograde_factor_from_c(
    fn_name: &str,
    value: u32,
) -> Result<sidereon_core::astro::equinoctial::RetrogradeFactor, SidereonStatus> {
    match SidereonRetrogradeFactor::try_from(value) {
        Ok(SidereonRetrogradeFactor::Prograde) => {
            Ok(sidereon_core::astro::equinoctial::RetrogradeFactor::Prograde)
        }
        Ok(SidereonRetrogradeFactor::Retrograde) => {
            Ok(sidereon_core::astro::equinoctial::RetrogradeFactor::Retrograde)
        }
        Err(()) => {
            set_last_error(format!("{fn_name}: invalid retrograde factor {value}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn retrograde_factor_to_c(value: sidereon_core::astro::equinoctial::RetrogradeFactor) -> u32 {
    match value {
        sidereon_core::astro::equinoctial::RetrogradeFactor::Prograde => {
            SidereonRetrogradeFactor::Prograde as u32
        }
        sidereon_core::astro::equinoctial::RetrogradeFactor::Retrograde => {
            SidereonRetrogradeFactor::Retrograde as u32
        }
    }
}
