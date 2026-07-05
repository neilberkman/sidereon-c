use super::*;

/// Geodesic inverse solution on WGS84.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodesicInverseResult {
    /// Ellipsoidal distance in meters.
    pub distance_m: f64,
    /// Forward azimuth at the first point, degrees clockwise from north.
    pub initial_azimuth_deg: f64,
    /// Forward azimuth at the second point, degrees clockwise from north.
    pub final_azimuth_deg: f64,
}

/// Geodesic direct solution on WGS84.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodesicDirectResult {
    /// Destination latitude in degrees.
    pub latitude_deg: f64,
    /// Destination longitude in degrees.
    pub longitude_deg: f64,
    /// Forward azimuth at the destination, degrees clockwise from north.
    pub final_azimuth_deg: f64,
}

/// Solve the WGS84 Karney inverse geodesic from point 1 to point 2.
///
/// Safety: out_result must point to writable SidereonGeodesicInverseResult
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodesic_inverse(
    lat1_deg: f64,
    lon1_deg: f64,
    lat2_deg: f64,
    lon2_deg: f64,
    out_result: *mut SidereonGeodesicInverseResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_geodesic_inverse", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_result,
            "sidereon_geodesic_inverse",
            "out_result"
        ));
        *out = SidereonGeodesicInverseResult {
            distance_m: 0.0,
            initial_azimuth_deg: 0.0,
            final_azimuth_deg: 0.0,
        };
        match sidereon_core::geodesic::geodesic_inverse(lat1_deg, lon1_deg, lat2_deg, lon2_deg) {
            Ok((distance_m, initial_azimuth_deg, final_azimuth_deg)) => {
                *out = SidereonGeodesicInverseResult {
                    distance_m,
                    initial_azimuth_deg,
                    final_azimuth_deg,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_geodesic_error("sidereon_geodesic_inverse", err),
        }
    })
}

/// Solve the WGS84 Karney direct geodesic from a point, azimuth, and distance.
///
/// Safety: out_result must point to writable SidereonGeodesicDirectResult
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geodesic_direct(
    lat1_deg: f64,
    lon1_deg: f64,
    initial_azimuth_deg: f64,
    distance_m: f64,
    out_result: *mut SidereonGeodesicDirectResult,
) -> SidereonStatus {
    ffi_boundary("sidereon_geodesic_direct", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_result,
            "sidereon_geodesic_direct",
            "out_result"
        ));
        *out = SidereonGeodesicDirectResult {
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            final_azimuth_deg: 0.0,
        };
        match sidereon_core::geodesic::geodesic_direct(
            lat1_deg,
            lon1_deg,
            initial_azimuth_deg,
            distance_m,
        ) {
            Ok((latitude_deg, longitude_deg, final_azimuth_deg)) => {
                *out = SidereonGeodesicDirectResult {
                    latitude_deg,
                    longitude_deg,
                    final_azimuth_deg,
                };
                SidereonStatus::Ok
            }
            Err(err) => map_geodesic_error("sidereon_geodesic_direct", err),
        }
    })
}

fn map_geodesic_error(
    fn_name: &str,
    err: sidereon_core::geodesic::GeodesicError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
