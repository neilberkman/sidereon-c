use super::*;

/// Range-rate and Doppler-ratio output for one satellite-ground link.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDopplerRangeRate {
    /// Range rate in km/s. Positive means receding from the station.
    pub range_rate_km_s: f64,
    /// Dimensionless Doppler ratio. Positive means approaching the station.
    pub doppler_ratio: f64,
}

/// Carrier Doppler-shift output for one satellite-ground link.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDopplerShift {
    /// Range rate in km/s. Positive means receding from the station.
    pub range_rate_km_s: f64,
    /// Doppler shift in hertz.
    pub doppler_hz: f64,
    /// Dimensionless Doppler ratio. Positive means approaching the station.
    pub doppler_ratio: f64,
}

/// Compute range rate and Doppler ratio from a GCRS satellite state. Delegates
/// to sidereon_core::astro::doppler::range_rate_and_ratio.
///
/// Safety: gcrs_position_km and gcrs_velocity_km_s point to 3 doubles each; ts
/// points to a SidereonTimeScales; out points to a SidereonDopplerRangeRate.
#[no_mangle]
pub unsafe extern "C" fn sidereon_doppler_range_rate_and_ratio(
    gcrs_position_km: *const f64,
    gcrs_velocity_km_s: *const f64,
    station_lat_deg: f64,
    station_lon_deg: f64,
    station_alt_km: f64,
    ts: *const SidereonTimeScales,
    out: *mut SidereonDopplerRangeRate,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_doppler_range_rate_and_ratio",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_doppler_range_rate_and_ratio",
                "out"
            ));
            *out = SidereonDopplerRangeRate {
                range_rate_km_s: 0.0,
                doppler_ratio: 0.0,
            };
            let position = c_try!(read_vec3(
                "sidereon_doppler_range_rate_and_ratio",
                "gcrs_position_km",
                gcrs_position_km
            ));
            let velocity = c_try!(read_vec3(
                "sidereon_doppler_range_rate_and_ratio",
                "gcrs_velocity_km_s",
                gcrs_velocity_km_s
            ));
            let ts = c_try!(require_ref(
                ts,
                "sidereon_doppler_range_rate_and_ratio",
                "ts"
            ))
            .to_core();
            match sidereon_core::astro::doppler::range_rate_and_ratio(
                position,
                velocity,
                station_lat_deg,
                station_lon_deg,
                station_alt_km,
                &ts,
            ) {
                Ok((range_rate_km_s, doppler_ratio)) => {
                    *out = SidereonDopplerRangeRate {
                        range_rate_km_s,
                        doppler_ratio,
                    };
                    SidereonStatus::Ok
                }
                Err(err) => extra_invalid_arg("sidereon_doppler_range_rate_and_ratio", err),
            }
        },
    )
}

/// Compute range rate, Doppler ratio, and carrier Doppler shift. Delegates to
/// sidereon_core::astro::doppler::doppler_shift.
///
/// Safety: gcrs_position_km and gcrs_velocity_km_s point to 3 doubles each; ts
/// points to a SidereonTimeScales; out points to a SidereonDopplerShift.
#[no_mangle]
pub unsafe extern "C" fn sidereon_doppler_shift(
    gcrs_position_km: *const f64,
    gcrs_velocity_km_s: *const f64,
    station_lat_deg: f64,
    station_lon_deg: f64,
    station_alt_km: f64,
    ts: *const SidereonTimeScales,
    frequency_hz: f64,
    out: *mut SidereonDopplerShift,
) -> SidereonStatus {
    ffi_boundary("sidereon_doppler_shift", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_doppler_shift", "out"));
        *out = SidereonDopplerShift {
            range_rate_km_s: 0.0,
            doppler_hz: 0.0,
            doppler_ratio: 0.0,
        };
        let position = c_try!(read_vec3(
            "sidereon_doppler_shift",
            "gcrs_position_km",
            gcrs_position_km
        ));
        let velocity = c_try!(read_vec3(
            "sidereon_doppler_shift",
            "gcrs_velocity_km_s",
            gcrs_velocity_km_s
        ));
        let ts = c_try!(require_ref(ts, "sidereon_doppler_shift", "ts")).to_core();
        match sidereon_core::astro::doppler::doppler_shift(
            position,
            velocity,
            station_lat_deg,
            station_lon_deg,
            station_alt_km,
            &ts,
            frequency_hz,
        ) {
            Ok(shift) => {
                *out = SidereonDopplerShift {
                    range_rate_km_s: shift.range_rate_km_s,
                    doppler_hz: shift.doppler_hz,
                    doppler_ratio: shift.doppler_ratio,
                };
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_doppler_shift", err),
        }
    })
}
