use super::*;

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
    ffi_boundary("sidereon_rf_fspl", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rf_fspl", "out"));
        *out = 0.0;
        match sidereon_core::astro::rf::fspl(distance_km, frequency_mhz) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_rf_fspl", err),
        }
    })
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
    ffi_boundary("sidereon_rf_eirp", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rf_eirp", "out"));
        *out = 0.0;
        match sidereon_core::astro::rf::eirp(tx_power_dbm, tx_antenna_gain_dbi) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_rf_eirp", err),
        }
    })
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
    ffi_boundary("sidereon_rf_cn0", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rf_cn0", "out"));
        *out = 0.0;
        match sidereon_core::astro::rf::cn0(eirp_dbw, fspl_db, receiver_gt_dbk, other_losses_db) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_rf_cn0", err),
        }
    })
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
    ffi_boundary("sidereon_rf_link_margin", SidereonStatus::Panic, || {
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
            Err(err) => extra_invalid_arg("sidereon_rf_link_margin", err),
        }
    })
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
    ffi_boundary("sidereon_rf_wavelength", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rf_wavelength", "out"));
        *out = 0.0;
        match sidereon_core::astro::rf::wavelength(frequency_hz) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_rf_wavelength", err),
        }
    })
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
    ffi_boundary("sidereon_rf_dish_gain", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_rf_dish_gain", "out"));
        *out = 0.0;
        match sidereon_core::astro::rf::dish_gain(diameter_m, frequency_hz, efficiency) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_rf_dish_gain", err),
        }
    })
}
