use super::*;

fn rf_invalid_arg(operation: &str, error: sidereon_core::astro::rf::RfError) -> SidereonStatus {
    crate::engine_error::record_engine_error(
        crate::engine_error::SidereonEngineErrorFamily::Rf,
        operation,
        crate::engine_error::rf_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

// --- RF link budget (sidereon_core::astro::rf) ------------------------------

/// A radio-frequency link budget, mirroring sidereon_core::astro::rf::LinkBudget.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLinkBudget {
    /// Effective isotropic radiated power, dBW.
    pub eirp_dbw: f64,
    /// Free-space path loss, dB.
    pub fspl_db: f64,
    /// Receiver gain-over-temperature, dB/K.
    pub receiver_gt_dbk: f64,
    /// Other losses, dB.
    pub other_losses_db: f64,
    /// Required carrier-to-noise density, dB-Hz.
    pub required_cn0_dbhz: f64,
}

/// Free-space path loss in dB. Delegates to sidereon_core::astro::rf::fspl.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_fspl(
    distance_km: f64,
    frequency_mhz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_fspl",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_fspl", "out"));
            *out = 0.0;
            match sidereon_core::astro::rf::fspl(distance_km, frequency_mhz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_fspl", err),
            }
        },
    )
}

/// Effective isotropic radiated power in dBW. Delegates to
/// sidereon_core::astro::rf::eirp.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_eirp(
    tx_power_dbm: f64,
    tx_antenna_gain_dbi: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_eirp",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_eirp", "out"));
            *out = 0.0;
            match sidereon_core::astro::rf::eirp(tx_power_dbm, tx_antenna_gain_dbi) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_eirp", err),
            }
        },
    )
}

/// Received carrier-to-noise density in dB-Hz. Delegates to
/// sidereon_core::astro::rf::cn0.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_cn0(
    eirp_dbw: f64,
    fspl_db: f64,
    receiver_gt_dbk: f64,
    other_losses_db: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_cn0",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_cn0", "out"));
            *out = 0.0;
            match sidereon_core::astro::rf::cn0(eirp_dbw, fspl_db, receiver_gt_dbk, other_losses_db)
            {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_cn0", err),
            }
        },
    )
}

/// Link margin in dB (received minus required C/N0). Delegates to
/// sidereon_core::astro::rf::link_margin.
///
/// Safety: budget must point to a SidereonLinkBudget; out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_link_margin(
    budget: *const SidereonLinkBudget,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_link_margin",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_link_margin", "out"));
            *out = 0.0;
            let budget = c_try!(require_ref(budget, "sidereon_rf_link_margin", "budget"));
            let inner = sidereon_core::astro::rf::LinkBudget {
                eirp_dbw: budget.eirp_dbw,
                fspl_db: budget.fspl_db,
                receiver_gt_dbk: budget.receiver_gt_dbk,
                other_losses_db: budget.other_losses_db,
                required_cn0_dbhz: budget.required_cn0_dbhz,
            };
            match sidereon_core::astro::rf::link_margin(&inner) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_link_margin", err),
            }
        },
    )
}

/// Wavelength in meters for a frequency in Hz. Delegates to
/// sidereon_core::astro::rf::wavelength.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_wavelength(
    frequency_hz: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_wavelength",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_wavelength", "out"));
            *out = 0.0;
            match sidereon_core::astro::rf::wavelength(frequency_hz) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_wavelength", err),
            }
        },
    )
}

/// Parabolic-dish antenna gain in dBi. Delegates to
/// sidereon_core::astro::rf::dish_gain.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rf_dish_gain(
    diameter_m: f64,
    frequency_hz: f64,
    efficiency: f64,
    out: *mut f64,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_rf_dish_gain",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rf_dish_gain", "out"));
            *out = 0.0;
            match sidereon_core::astro::rf::dish_gain(diameter_m, frequency_hz, efficiency) {
                Ok(v) => {
                    *out = v;
                    SidereonStatus::Ok
                }
                Err(err) => rf_invalid_arg("sidereon_rf_dish_gain", err),
            }
        },
    )
}

#[cfg(test)]
mod engine_error_tests {
    use super::*;
    use crate::engine_error::{snapshot_engine_error_for_test, SidereonEngineErrorFamily};
    use std::ffi::CStr;

    fn assert_rf_producer_contract(
        operation: &str,
        field: &str,
        reason: &str,
        refusal: impl Fn() -> SidereonStatus,
        success: impl FnOnce() -> SidereonStatus,
        early_null: impl FnOnce() -> SidereonStatus,
    ) {
        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        let (info, payload) = snapshot_engine_error_for_test().expect("typed RF refusal");
        assert_eq!(info.family, SidereonEngineErrorFamily::Rf);
        assert_eq!(info.payload_len, payload.len());
        let value: serde_json::Value = serde_json::from_str(&payload).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "family": "rf",
                "operation": operation,
                "error": {
                    "kind": "invalid_input",
                    "fields": {"field": field, "reason": reason}
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
            format!("{operation}: invalid RF input {field}: {reason}")
        );

        assert_eq!(success(), SidereonStatus::Ok);
        assert!(snapshot_engine_error_for_test().is_none());

        assert_eq!(refusal(), SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());
        assert_eq!(early_null(), SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());
    }

    #[test]
    fn all_rf_producers_record_typed_errors_and_clear_them_on_success_or_early_null() {
        assert_rf_producer_contract(
            "sidereon_rf_fspl",
            "distance_km",
            "not positive",
            || unsafe { sidereon_rf_fspl(0.0, 1616.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_fspl(1200.0, 1616.0, &mut out) }
            },
            || unsafe { sidereon_rf_fspl(1200.0, 1616.0, ptr::null_mut()) },
        );
        assert_rf_producer_contract(
            "sidereon_rf_eirp",
            "tx_power_dbm",
            "not finite",
            || unsafe { sidereon_rf_eirp(f64::NAN, 3.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_eirp(27.0, 3.0, &mut out) }
            },
            || unsafe { sidereon_rf_eirp(27.0, 3.0, ptr::null_mut()) },
        );
        assert_rf_producer_contract(
            "sidereon_rf_cn0",
            "eirp_dbw",
            "not finite",
            || unsafe { sidereon_rf_cn0(f64::NAN, 165.0, -12.0, 3.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_cn0(0.0, 165.0, -12.0, 3.0, &mut out) }
            },
            || unsafe { sidereon_rf_cn0(0.0, 165.0, -12.0, 3.0, ptr::null_mut()) },
        );
        let bad_budget = SidereonLinkBudget {
            eirp_dbw: f64::NAN,
            fspl_db: 165.0,
            receiver_gt_dbk: -12.0,
            other_losses_db: 3.0,
            required_cn0_dbhz: 35.0,
        };
        let good_budget = SidereonLinkBudget {
            eirp_dbw: 0.0,
            ..bad_budget
        };
        assert_rf_producer_contract(
            "sidereon_rf_link_margin",
            "eirp_dbw",
            "not finite",
            || unsafe { sidereon_rf_link_margin(&bad_budget, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_link_margin(&good_budget, &mut out) }
            },
            || unsafe { sidereon_rf_link_margin(&good_budget, ptr::null_mut()) },
        );
        assert_rf_producer_contract(
            "sidereon_rf_wavelength",
            "frequency_hz",
            "not positive",
            || unsafe { sidereon_rf_wavelength(0.0, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_wavelength(1616.0e6, &mut out) }
            },
            || unsafe { sidereon_rf_wavelength(1616.0e6, ptr::null_mut()) },
        );
        assert_rf_producer_contract(
            "sidereon_rf_dish_gain",
            "diameter_m",
            "not positive",
            || unsafe { sidereon_rf_dish_gain(0.0, 1616.0e6, 0.55, &mut 0.0) },
            || {
                let mut out = 0.0;
                unsafe { sidereon_rf_dish_gain(1.0, 1616.0e6, 0.55, &mut out) }
            },
            || unsafe { sidereon_rf_dish_gain(1.0, 1616.0e6, 0.55, ptr::null_mut()) },
        );
    }
}
