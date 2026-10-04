use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};

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
    engine_error_operation_boundary(
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

fn atmosphere_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

/// Convert an `AtmosphereError` into a structured JSON error node.
pub(crate) fn atmosphere_error_value(error: &AtmosphereError) -> serde_json::Value {
    use AtmosphereError as E;
    match error {
        E::MissingApArray => atmosphere_node("missing_ap_array", serde_json::json!({})),
        E::NonFiniteInput(field) => atmosphere_node(
            "non_finite_input",
            serde_json::json!({
                "field": *field,
            }),
        ),
        E::OutOfDomain(field) => atmosphere_node(
            "out_of_domain",
            serde_json::json!({
                "field": *field,
            }),
        ),
    }
}

fn map_atmosphere_error(fn_name: &str, err: AtmosphereError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Atmosphere,
        fn_name,
        atmosphere_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::Value;
    use std::ptr;

    #[test]
    fn table_driven_atmosphere_error_mapping() {
        use sidereon_core::astro::atmosphere::AtmosphereError as E;

        // 1. MissingApArray
        let err = E::MissingApArray;
        let val = atmosphere_error_value(&err);
        assert_eq!(val["kind"], "missing_ap_array");
        assert_eq!(val["fields"], serde_json::json!({}));

        // 2. NonFiniteInput named fields
        for field in [
            "alt", "g_lat", "g_long", "sec", "lst", "f107", "f107a", "ap", "ap_array",
        ] {
            let err = E::NonFiniteInput(field);
            let val = atmosphere_error_value(&err);
            assert_eq!(val["kind"], "non_finite_input");
            assert_eq!(val["fields"]["field"], field);
        }

        // 3. OutOfDomain named fields
        for field in ["alt", "f107", "f107a", "ap", "ap_array"] {
            let err = E::OutOfDomain(field);
            let val = atmosphere_error_value(&err);
            assert_eq!(val["kind"], "out_of_domain");
            assert_eq!(val["fields"]["field"], field);
        }
    }

    #[test]
    fn atmosphere_public_producer_control_and_refusals() {
        clear_engine_error();

        unsafe {
            // Seed real existing domain refusal immediately before valid success control
            let mut bad_input = sidereon_atmosphere_input_default();
            bad_input.year = 2024;
            bad_input.doy = 100;
            bad_input.sec = 3600.0;
            bad_input.alt_km = -10.0;

            let mut out = SidereonAtmosphereOutput {
                density_kg_m3: 0.0,
                temperature_k: 0.0,
            };
            assert_eq!(
                sidereon_atmosphere_nrlmsise00(&bad_input, &mut out),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);
            assert!(info.payload_len > 0);

            // Valid success control clears TLS error
            let mut input = sidereon_atmosphere_input_default();
            input.year = 2024;
            input.doy = 100;
            input.sec = 3600.0;
            input.alt_km = 400.0;
            input.lat_deg = 30.0;
            input.lon_deg = -90.0;

            assert_eq!(
                sidereon_atmosphere_nrlmsise00(&input, &mut out),
                SidereonStatus::Ok
            );
            assert!(out.density_kg_m3 > 0.0);
            assert!(out.temperature_k > 0.0);

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Non-producing reader retains clean state
            let _def = sidereon_atmosphere_input_default();
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }

        // Real public refusal 1: out-of-domain altitude (-10 km)
        unsafe {
            let mut input = sidereon_atmosphere_input_default();
            input.year = 2024;
            input.doy = 100;
            input.sec = 3600.0;
            input.alt_km = -10.0;

            let mut out = SidereonAtmosphereOutput {
                density_kg_m3: 0.0,
                temperature_k: 0.0,
            };
            assert_eq!(
                sidereon_atmosphere_nrlmsise00(&input, &mut out),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "atmosphere");
            assert_eq!(payload["operation"], "sidereon_atmosphere_nrlmsise00");
            assert_eq!(payload["error"]["kind"], "out_of_domain");
            assert_eq!(payload["error"]["fields"]["field"], "alt");
        }

        // Real public refusal 2: non-finite input altitude (NaN)
        unsafe {
            let mut input = sidereon_atmosphere_input_default();
            input.year = 2024;
            input.doy = 100;
            input.sec = 3600.0;
            input.alt_km = f64::NAN;

            let mut out = SidereonAtmosphereOutput {
                density_kg_m3: 0.0,
                temperature_k: 0.0,
            };
            assert_eq!(
                sidereon_atmosphere_nrlmsise00(&input, &mut out),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, info.payload_len);
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "atmosphere");
            assert_eq!(payload["operation"], "sidereon_atmosphere_nrlmsise00");
            assert_eq!(payload["error"]["kind"], "non_finite_input");
            assert_eq!(payload["error"]["fields"]["field"], "alt");
        }
    }

    #[test]
    fn atmosphere_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // Seed error via real refusal
            let mut input = sidereon_atmosphere_input_default();
            input.year = 2024;
            input.doy = 100;
            input.sec = 3600.0;
            input.alt_km = -10.0;

            let mut out = SidereonAtmosphereOutput {
                density_kg_m3: 0.0,
                temperature_k: 0.0,
            };
            assert_eq!(
                sidereon_atmosphere_nrlmsise00(&input, &mut out),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Non-producing call retains error: sidereon_atmosphere_input_default
            let _def = sidereon_atmosphere_input_default();
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);
            assert_eq!(info.payload_len, expected_len);

            // Two-pass payload retrieval: Pass 1 query length
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument and retains
            let mut short_buf = vec![0u8; expected_len - 1];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Pass 2: Exact buffer query succeeds
            let mut buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, expected_len);
            assert_eq!(required, expected_len);

            // Slot still retains
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Atmosphere);

            // Producer early argument failure (null pointer) clears slot before checks!
            assert_eq!(
                sidereon_atmosphere_nrlmsise00(ptr::null(), &mut out),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
