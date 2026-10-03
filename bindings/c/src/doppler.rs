use super::*;

fn doppler_invalid_arg(
    operation: &str,
    error: sidereon_core::astro::doppler::DopplerError,
) -> SidereonStatus {
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::Doppler,
        operation,
        crate::engine_error::doppler_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

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
    crate::engine_error::engine_error_operation_boundary(
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
            let ts = c_try!(c_try!(require_ref(
                ts,
                "sidereon_doppler_range_rate_and_ratio",
                "ts"
            ))
            .to_core("sidereon_doppler_range_rate_and_ratio"));
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
                Err(err) => doppler_invalid_arg("sidereon_doppler_range_rate_and_ratio", err),
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_doppler_shift",
        SidereonStatus::Panic,
        || {
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
            let ts = c_try!(c_try!(require_ref(ts, "sidereon_doppler_shift", "ts"))
                .to_core("sidereon_doppler_shift"));
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
                Err(err) => doppler_invalid_arg("sidereon_doppler_shift", err),
            }
        },
    )
}

#[cfg(test)]
mod engine_error_tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};
    use std::ffi::CStr;

    fn assert_doppler_contract(
        operation: &str,
        refusal: impl Fn() -> SidereonStatus,
        success: impl FnOnce() -> SidereonStatus,
        early_null: impl FnOnce() -> SidereonStatus,
    ) {
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        let (info, payload) = snapshot_engine_error_for_test().expect("typed Doppler refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Doppler);
        assert_eq!(info.payload_len, payload.len());
        let value: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "family": "doppler",
                "operation": operation,
                "error": {
                    "kind": "frame_transform",
                    "fields": {
                        "cause": {
                            "kind": "invalid_input",
                            "fields": {"field": "latitude_deg", "reason": "must be finite"}
                        }
                    }
                }
            })
        );

        let mut message = vec![0 as std::os::raw::c_char; 256];
        let needed =
            unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert!(needed > 0);
        let message = unsafe { CStr::from_ptr(message.as_ptr()) }
            .to_str()
            .expect("legacy UTF-8");
        assert_eq!(
            message,
            format!(
                "{operation}: doppler frame transform failed: invalid frame transform latitude_deg: must be finite"
            )
        );

        assert_eq!(success(), SidereonStatus::Ok);
        assert!(snapshot_engine_error_for_test().is_none());
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(early_null(), SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());
    }

    #[test]
    fn both_doppler_producers_record_nested_frame_transform_errors_and_clear_tls() {
        let mut ts = std::mem::MaybeUninit::<SidereonTimeScales>::uninit();
        assert_eq!(
            unsafe { sidereon_timescales_from_utc(2024, 1, 1, 0, 0, 0.0, ts.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let ts = unsafe { ts.assume_init() };
        let position = [7000.0, 0.0, 0.0];
        let velocity = [0.0, 7.5, 0.0];

        assert_doppler_contract(
            "sidereon_doppler_range_rate_and_ratio",
            || {
                let mut out = SidereonDopplerRangeRate {
                    range_rate_km_s: 0.0,
                    doppler_ratio: 0.0,
                };
                unsafe {
                    sidereon_doppler_range_rate_and_ratio(
                        position.as_ptr(),
                        velocity.as_ptr(),
                        f64::NAN,
                        0.0,
                        0.0,
                        &ts,
                        &mut out,
                    )
                }
            },
            || {
                let mut out = SidereonDopplerRangeRate {
                    range_rate_km_s: 0.0,
                    doppler_ratio: 0.0,
                };
                unsafe {
                    sidereon_doppler_range_rate_and_ratio(
                        position.as_ptr(),
                        velocity.as_ptr(),
                        0.0,
                        0.0,
                        0.0,
                        &ts,
                        &mut out,
                    )
                }
            },
            || unsafe {
                sidereon_doppler_range_rate_and_ratio(
                    position.as_ptr(),
                    velocity.as_ptr(),
                    0.0,
                    0.0,
                    0.0,
                    &ts,
                    ptr::null_mut(),
                )
            },
        );

        assert_doppler_contract(
            "sidereon_doppler_shift",
            || {
                let mut out = SidereonDopplerShift {
                    range_rate_km_s: 0.0,
                    doppler_hz: 0.0,
                    doppler_ratio: 0.0,
                };
                unsafe {
                    sidereon_doppler_shift(
                        position.as_ptr(),
                        velocity.as_ptr(),
                        f64::NAN,
                        0.0,
                        0.0,
                        &ts,
                        1575.42e6,
                        &mut out,
                    )
                }
            },
            || {
                let mut out = SidereonDopplerShift {
                    range_rate_km_s: 0.0,
                    doppler_hz: 0.0,
                    doppler_ratio: 0.0,
                };
                unsafe {
                    sidereon_doppler_shift(
                        position.as_ptr(),
                        velocity.as_ptr(),
                        0.0,
                        0.0,
                        0.0,
                        &ts,
                        1575.42e6,
                        &mut out,
                    )
                }
            },
            || unsafe {
                sidereon_doppler_shift(
                    position.as_ptr(),
                    velocity.as_ptr(),
                    0.0,
                    0.0,
                    0.0,
                    &ts,
                    1575.42e6,
                    ptr::null_mut(),
                )
            },
        );
    }
}
