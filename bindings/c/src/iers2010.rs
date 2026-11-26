use super::*;
use crate::engine_error::engine_error_operation_boundary;
use sidereon_core::astro::frames::{Iers2010EarthOrientationProvider, Iers2010EopSample};
use sidereon_core::astro::time::ValidityMode;

/// Owned IERS 2010 orientation provider. Release with
/// sidereon_iers2010_earth_orientation_provider_free.
pub struct SidereonIers2010EarthOrientationProvider {
    pub(crate) inner: Arc<Iers2010EarthOrientationProvider>,
}

/// A daily IERS Earth orientation sample. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidereonIers2010EopSample {
    /// UTC modified Julian date.
    pub mjd_utc: i32,
    /// Pole X in arcseconds.
    pub xp_arcsec: f64,
    /// Pole Y in arcseconds.
    pub yp_arcsec: f64,
    /// UT1 minus UTC in seconds.
    pub ut1_minus_utc_s: f64,
    /// Excess length of day in seconds.
    pub lod_s: f64,
    /// Celestial pole X correction in arcseconds.
    pub dx_arcsec: f64,
    /// Celestial pole Y correction in arcseconds.
    pub dy_arcsec: f64,
    /// Whether both pole rates are supplied.
    pub has_pole_rates: bool,
    /// Pole X and Y rates in radians per second when supplied.
    pub pole_rates_rad_s: [f64; 2],
}

impl From<SidereonIers2010EopSample> for Iers2010EopSample {
    fn from(sample: SidereonIers2010EopSample) -> Self {
        Self {
            mjd_utc: sample.mjd_utc,
            xp_arcsec: sample.xp_arcsec,
            yp_arcsec: sample.yp_arcsec,
            ut1_minus_utc_s: sample.ut1_minus_utc_s,
            lod_s: sample.lod_s,
            dx_arcsec: sample.dx_arcsec,
            dy_arcsec: sample.dy_arcsec,
            pole_rates_rad_s: sample.has_pole_rates.then_some(sample.pole_rates_rad_s),
        }
    }
}

/// Earth orientation at one instant. Matrices use row-major order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEarthOrientation {
    /// Time scales at the instant and UT1 coverage status.
    pub time_scales: SidereonTimeScales,
    /// Polar motion X and Y in radians.
    pub polar_motion_rad: [f64; 2],
    /// Earth spin vector in ITRF, in radians per second.
    pub earth_rotation_vector_itrf_rad_s: [f64; 3],
    /// Rotation from GCRF to ITRF.
    pub gcrf_to_itrf_matrix: [[f64; 3]; 3],
    /// Rotation from ITRF to GCRF.
    pub itrf_to_gcrf_matrix: [[f64; 3]; 3],
    /// Time derivative of the GCRF to ITRF rotation.
    pub gcrf_to_itrf_rotation_rate_matrix: [[f64; 3]; 3],
    /// Time derivative of the ITRF to GCRF rotation.
    pub itrf_to_gcrf_rotation_rate_matrix: [[f64; 3]; 3],
    /// Whether an IERS 2010 rotation quaternion is available.
    pub has_iers2010_quaternion: bool,
    /// IERS 2010 rotation quaternion in scalar-first order when available.
    pub iers2010_quaternion: [f64; 4],
}

fn orientation_to_c(
    value: &sidereon_core::astro::frames::EarthOrientation,
) -> SidereonEarthOrientation {
    let pole = value.polar_motion();
    let quaternion = value.iers2010_quaternion();
    SidereonEarthOrientation {
        time_scales: SidereonTimeScales::from_core(&value.time_scales()),
        polar_motion_rad: [pole.xp_rad, pole.yp_rad],
        earth_rotation_vector_itrf_rad_s: value.earth_rotation_vector_itrf_rad_s(),
        gcrf_to_itrf_matrix: value.gcrf_to_itrf_matrix(),
        itrf_to_gcrf_matrix: value.itrf_to_gcrf_matrix(),
        gcrf_to_itrf_rotation_rate_matrix: value.gcrf_to_itrf_rotation_rate_matrix(),
        itrf_to_gcrf_rotation_rate_matrix: value.itrf_to_gcrf_rotation_rate_matrix(),
        has_iers2010_quaternion: quaternion.is_some(),
        iers2010_quaternion: quaternion.unwrap_or_default(),
    }
}

/// Build an IERS 2010 orientation provider from daily samples.
///
/// Safety: samples points to count entries. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_new(
    samples: *const SidereonIers2010EopSample,
    count: usize,
    out: *mut *mut SidereonIers2010EarthOrientationProvider,
) -> SidereonStatus {
    const OP: &str = "sidereon_iers2010_earth_orientation_provider_new";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(ptr::null_mut());
        let samples = c_try!(require_slice(samples, count, OP, "samples"));
        let provider = c_try!(Iers2010EarthOrientationProvider::new(
            samples.iter().copied().map(Into::into).collect()
        )
        .map_err(|err| map_frame_transform_error(OP, err)));
        out.write(Box::into_raw(Box::new(
            SidereonIers2010EarthOrientationProvider {
                inner: Arc::new(provider),
            },
        )));
        SidereonStatus::Ok
    })
}

/// Copy an orientation provider with the selected UT1 validity policy.
///
/// Safety: provider is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_with_validity(
    provider: *const SidereonIers2010EarthOrientationProvider,
    permissive: bool,
    out: *mut *mut SidereonIers2010EarthOrientationProvider,
) -> SidereonStatus {
    const OP: &str = "sidereon_iers2010_earth_orientation_provider_with_validity";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", provider, "provider"
        ));
        out.write(ptr::null_mut());
        let provider = c_try!(require_ref(provider, OP, "provider"));
        let validity = if permissive {
            ValidityMode::Permissive
        } else {
            ValidityMode::Strict
        };
        let inner = Arc::new((*provider.inner).clone().with_validity(validity));
        out.write(Box::into_raw(Box::new(
            SidereonIers2010EarthOrientationProvider { inner },
        )));
        SidereonStatus::Ok
    })
}

/// Compare two IERS 2010 providers by value.
///
/// Safety: both handles are live and out points to disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_equal(
    left: *const SidereonIers2010EarthOrientationProvider,
    right: *const SidereonIers2010EarthOrientationProvider,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_iers2010_earth_orientation_provider_equal";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", left, "left"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", right, "right"
        ));
        let left = c_try!(require_ref(left, OP, "left"));
        let right = c_try!(require_ref(right, OP, "right"));
        out.write(left.inner == right.inner);
        SidereonStatus::Ok
    })
}

/// Evaluate the IERS 2010 orientation at a UTC MJD and seconds within that day.
///
/// Safety: provider is live and out is disjoint writable orientation storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_orientation_at_utc_mjd(
    provider: *const SidereonIers2010EarthOrientationProvider,
    mjd_utc: i32,
    seconds_in_day: f64,
    out: *mut SidereonEarthOrientation,
) -> SidereonStatus {
    const OP: &str = "sidereon_iers2010_earth_orientation_provider_orientation_at_utc_mjd";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", provider, "provider"
        ));
        let provider = c_try!(require_ref(provider, OP, "provider"));
        let value = c_try!(provider
            .inner
            .orientation_at_utc_mjd(mjd_utc, seconds_in_day)
            .map_err(|err| map_frame_transform_error(OP, err)));
        out.write(orientation_to_c(&value));
        SidereonStatus::Ok
    })
}

/// Release an IERS 2010 orientation provider.
///
/// Safety: provider is NULL or a live owned provider.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_free(
    provider: *mut SidereonIers2010EarthOrientationProvider,
) {
    ffi_boundary(
        "sidereon_iers2010_earth_orientation_provider_free",
        (),
        || free_boxed(provider),
    );
}

/// Evaluate IERS 2010 orientation at absolute TDB seconds past J2000.
///
/// Safety: provider is live and out is disjoint writable orientation storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_iers2010_earth_orientation_provider_orientation_at_tdb_seconds(
    provider: *const SidereonIers2010EarthOrientationProvider,
    epoch_tdb_seconds: f64,
    out: *mut SidereonEarthOrientation,
) -> SidereonStatus {
    use sidereon_core::astro::frames::EarthOrientationProvider;
    const OP: &str = "sidereon_iers2010_earth_orientation_provider_orientation_at_tdb_seconds";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", provider, "provider"
        ));
        let provider = c_try!(require_ref(provider, OP, "provider"));
        let value = c_try!(provider
            .inner
            .orientation_at_tdb_seconds(epoch_tdb_seconds)
            .map_err(|err| map_frame_transform_error(OP, err)));
        out.write(orientation_to_c(&value));
        SidereonStatus::Ok
    })
}
