use super::*;

// ===========================================================================

/// Number of entries in an NRLMSISE-00 Ap-history array (matches the core
/// `ApArray` length).
pub const SIDEREON_ATMOSPHERE_AP_ARRAY_LEN: usize = 7;

/// Inputs to the NRLMSISE-00 neutral-atmosphere evaluation.
///
/// When `has_lst` is false the core derives local apparent solar time from `sec`
/// and `lon_deg`; when true, `lst` (hours) is used verbatim. When `has_ap_array`
/// is false the scalar `ap` drives the daily magnetic forcing (switch 9 off);
/// when true, `ap_array` supplies the Ap history for Ap-history mode. Source the
/// quiet-Sun defaults for `f107`, `f107a`, and `ap` from
/// sidereon_atmosphere_input_default.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAtmosphereInput {
    /// Calendar year.
    pub year: i32,
    /// Day of year, 1-366.
    pub doy: i32,
    /// Seconds into the UTC day.
    pub sec: f64,
    /// Geodetic altitude, kilometers (valid in [0, 1000]).
    pub alt_km: f64,
    /// Geodetic latitude, degrees.
    pub lat_deg: f64,
    /// Geodetic longitude, degrees.
    pub lon_deg: f64,
    /// Whether `lst` carries a caller-supplied local solar time. When false the
    /// core derives it from `sec` and `lon_deg`.
    pub has_lst: bool,
    /// Local apparent solar time, hours. Used only when has_lst is true.
    pub lst: f64,
    /// Daily F10.7 solar flux.
    pub f107: f64,
    /// 81-day average F10.7 solar flux.
    pub f107a: f64,
    /// Daily magnetic index Ap. Used when has_ap_array is false.
    pub ap: f64,
    /// Whether `ap_array` carries an Ap history (selects Ap-history mode).
    pub has_ap_array: bool,
    /// Ap history, used only when has_ap_array is true.
    pub ap_array: [f64; SIDEREON_ATMOSPHERE_AP_ARRAY_LEN],
}

/// An NRLMSISE-00 input prefilled with the engine's reference quiet-Sun
/// geomagnetic defaults (f107, f107a, ap from
/// sidereon_core::astro::atmosphere::{DEFAULT_F107, DEFAULT_F107A, DEFAULT_AP}),
/// has_lst and has_ap_array cleared, and the epoch/location fields zeroed for the
/// caller to fill.
#[no_mangle]
pub extern "C" fn sidereon_atmosphere_input_default() -> SidereonAtmosphereInput {
    SidereonAtmosphereInput {
        year: 0,
        doy: 0,
        sec: 0.0,
        alt_km: 0.0,
        lat_deg: 0.0,
        lon_deg: 0.0,
        has_lst: false,
        lst: 0.0,
        f107: DEFAULT_F107,
        f107a: DEFAULT_F107A,
        ap: DEFAULT_AP,
        has_ap_array: false,
        ap_array: [0.0; SIDEREON_ATMOSPHERE_AP_ARRAY_LEN],
    }
}

/// NRLMSISE-00 output.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonAtmosphereOutput {
    /// Total mass density, kilograms per cubic meter.
    pub density_kg_m3: f64,
    /// Temperature at the requested altitude, kelvin.
    pub temperature_k: f64,
}

/// Evaluate NRLMSISE-00 total mass density and temperature at a geodetic point
/// and epoch. Delegates to
/// sidereon_core::astro::atmosphere::nrlmsise00_with_lst, which derives the local
/// solar time in core when has_lst is false (or uses the supplied lst when
/// true). The Ap history is passed through when has_ap_array is set, otherwise
/// the scalar ap drives the daily forcing.
///
/// Safety: input must point to a SidereonAtmosphereInput; out must point to a
/// SidereonAtmosphereOutput.
#[no_mangle]
pub unsafe extern "C" fn sidereon_atmosphere_nrlmsise00(
    input: *const SidereonAtmosphereInput,
    out: *mut SidereonAtmosphereOutput,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_atmosphere_nrlmsise00",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_atmosphere_nrlmsise00", "out"));
            let input = c_try!(require_ref(
                input,
                "sidereon_atmosphere_nrlmsise00",
                "input"
            ));
            let lst = input.has_lst.then_some(input.lst);
            let ap_array: Option<ApArray> = input.has_ap_array.then_some(input.ap_array);
            let core_input = NrlmsiseInput {
                year: input.year,
                doy: input.doy,
                sec: input.sec,
                alt: input.alt_km,
                g_lat: input.lat_deg,
                g_long: input.lon_deg,
                lst: 0.0,
                f107a: input.f107a,
                f107: input.f107,
                ap: input.ap,
                ap_array,
            };
            let output = match nrlmsise00_with_lst(&core_input, lst) {
                Ok(output) => output,
                Err(err) => return map_atmosphere_error("sidereon_atmosphere_nrlmsise00", err),
            };
            *out = SidereonAtmosphereOutput {
                density_kg_m3: output.density(),
                temperature_k: output.temperature_alt(),
            };
            SidereonStatus::Ok
        },
    )
}

// ============================================================================
// Capability-parity additions: thin extern-C wrappers over existing core fns.
// Every function here marshals C input into the engine type, calls the cited
// sidereon-core entry point, and copies the result back. No modeling lives here.

fn map_atmosphere_error(fn_name: &str, err: AtmosphereError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
