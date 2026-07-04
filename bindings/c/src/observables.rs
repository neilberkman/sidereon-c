use super::*;

// ===========================================================================

/// Predictor options. Initialize with sidereon_observables_options_init for the
/// engine defaults (L1 carrier, light-time and Sagnac corrections on).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonObservablesOptions {
    /// Carrier frequency in hertz used for the Doppler conversion.
    pub carrier_hz: f64,
    /// Apply fixed-point light-time correction in the geometry substrate.
    pub light_time: bool,
    /// Apply Earth-rotation Sagnac correction in the geometry substrate.
    pub sagnac: bool,
}

/// One satellite's predicted observables at one epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPredictedObservables {
    /// Geometric range, meters.
    pub geometric_range_m: f64,
    /// Range rate (positive receding), meters per second.
    pub range_rate_m_s: f64,
    /// Doppler shift at options.carrier_hz, hertz.
    pub doppler_hz: f64,
    /// Whether sat_clock_s is present.
    pub has_sat_clock_s: bool,
    /// Satellite clock offset, seconds, when present.
    pub sat_clock_s: f64,
    /// Topocentric elevation, degrees.
    pub elevation_deg: f64,
    /// Topocentric azimuth, degrees in [0, 360).
    pub azimuth_deg: f64,
    /// Transmit-time offset from the receive epoch, microseconds.
    pub transmit_offset_us: i64,
    /// Transmit time, seconds since J2000.
    pub transmit_time_j2000_s: f64,
    /// ECEF line-of-sight unit vector (receiver toward satellite).
    pub los_unit: [f64; 3],
    /// Satellite ECEF position, meters.
    pub sat_pos_ecef_m: [f64; 3],
    /// Satellite ECEF velocity, meters per second.
    pub sat_velocity_m_s: [f64; 3],
}

/// Populate *out_options with the engine's default predictor options (L1
/// carrier, light-time and Sagnac corrections on).
///
/// Safety: out_options must point to a SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observables_options_init(
    out_options: *mut SidereonObservablesOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observables_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_observables_options_init",
                "out_options"
            ));
            let defaults = PredictOptions::default();
            *out_options = SidereonObservablesOptions {
                carrier_hz: defaults.carrier_hz,
                light_time: defaults.light_time,
                sagnac: defaults.sagnac,
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy the observable-state missing-position sentinel into out. The sentinel is
/// three NaN components and is also written for every failed batch element.
///
/// Safety: out_position_ecef_m must point to at least len doubles; len must be
/// at least 3.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_state_missing_position_ecef_m(
    out_position_ecef_m: *mut f64,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observable_state_missing_position_ecef_m",
        SidereonStatus::Panic,
        || {
            c_try!(copy_exact_f64s(
                "sidereon_observable_state_missing_position_ecef_m",
                "out_position_ecef_m",
                out_position_ecef_m,
                len,
                &OBSERVABLE_STATE_MISSING_POSITION_ECEF_M,
            ));
            SidereonStatus::Ok
        },
    )
}
