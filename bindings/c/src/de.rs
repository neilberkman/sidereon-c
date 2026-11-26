use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::de::{DeBody, DeEphemeris, DeError, DeState};
use std::sync::Arc;

/// Owned JPL DE binary ephemeris. Release with sidereon_de_ephemeris_free.
pub struct SidereonDeEphemeris {
    pub(crate) inner: Arc<DeEphemeris>,
}

/// Body whose mass parameter can be read from a DE header.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonDeBody {
    /// Sun.
    Sun = 0,
    /// Mercury.
    Mercury = 1,
    /// Venus.
    Venus = 2,
    /// Earth–Moon barycentre.
    EarthMoon = 3,
    /// Earth.
    Earth = 4,
    /// Moon.
    Moon = 5,
    /// Mars system.
    Mars = 6,
    /// Jupiter system.
    Jupiter = 7,
    /// Saturn system.
    Saturn = 8,
    /// Uranus system.
    Uranus = 9,
    /// Neptune system.
    Neptune = 10,
    /// Pluto system.
    Pluto = 11,
    /// Solar system.
    SolarSystem = 12,
}

/// Geocentric state in GCRF. Compare the fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonDeState {
    /// Position in metres.
    pub position_m: [f64; 3],
    /// Velocity in metres per second.
    pub velocity_m_s: [f64; 3],
}

impl From<DeState> for SidereonDeState {
    fn from(state: DeState) -> Self {
        Self {
            position_m: state.position_m,
            velocity_m_s: state.velocity_m_s,
        }
    }
}

/// Simultaneous geocentric Sun and Moon states in GCRF. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonDeSunMoon {
    /// Sun state.
    pub sun: SidereonDeState,
    /// Moon state.
    pub moon: SidereonDeState,
}

/// Coverage interval in absolute TDB seconds past J2000.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonDeCoverage {
    /// First covered epoch, inclusive.
    pub start_tdb_seconds: f64,
    /// Last covered epoch, inclusive.
    pub end_tdb_seconds: f64,
}

fn de_error_value(error: &DeError) -> serde_json::Value {
    let (kind, fields) = match error {
        DeError::Io(cause) => (
            "io",
            json!({
                "message": cause.to_string(),
                "raw_os_error": cause.raw_os_error(),
            }),
        ),
        DeError::InvalidFile(reason) => ("invalid_file", json!({"reason":reason})),
        DeError::NonFiniteEpoch => ("non_finite_epoch", json!({})),
        DeError::OutsideCoverage { epoch_tdb_seconds } => (
            "outside_coverage",
            json!({"epoch_tdb_seconds":engine_f64(*epoch_tdb_seconds)}),
        ),
        DeError::NonFiniteState => ("non_finite_state", json!({})),
        _ => ("unknown", json!({})),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_de_error(operation: &str, error: DeError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::De,
        operation,
        de_error_value(&error),
    );
    set_last_error(format!("{operation}: {error}"));
    SidereonStatus::Solve
}

/// Read a JPL DE ephemeris from a file.
///
/// Safety: path is a NUL-terminated UTF-8 string and out is writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_load(
    path: *const c_char,
    out: *mut *mut SidereonDeEphemeris,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_load";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(ptr::null_mut());
        let path = c_try!(parse_c_string_allow_empty(OP, "path", path));
        let inner = c_try!(DeEphemeris::load(path).map_err(|err| map_de_error(OP, err)));
        out.write(Box::into_raw(Box::new(SidereonDeEphemeris {
            inner: Arc::new(inner),
        })));
        SidereonStatus::Ok
    })
}

/// Copy and parse a JPL DE ephemeris from bytes.
///
/// Safety: data is readable for len bytes and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_from_bytes(
    data: *const u8,
    len: usize,
    out: *mut *mut SidereonDeEphemeris,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_from_bytes";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(ptr::null_mut());
        let bytes = c_try!(require_slice(data, len, OP, "data"));
        let inner = c_try!(DeEphemeris::from_bytes(bytes).map_err(|err| map_de_error(OP, err)));
        out.write(Box::into_raw(Box::new(SidereonDeEphemeris {
            inner: Arc::new(inner),
        })));
        SidereonStatus::Ok
    })
}

/// Release a DE ephemeris.
///
/// Safety: ephemeris is NULL or a live owned DE ephemeris handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_free(ephemeris: *mut SidereonDeEphemeris) {
    ffi_boundary("sidereon_de_ephemeris_free", (), || free_boxed(ephemeris));
}

unsafe fn de_query<T: Copy + Default>(
    operation: &str,
    ephemeris: *const SidereonDeEphemeris,
    out: *mut T,
    read: impl FnOnce(&DeEphemeris) -> Result<T, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation,
            out,
            "out",
            ephemeris,
            "ephemeris"
        ));
        out.write(T::default());
        let ephemeris = c_try!(require_ref(ephemeris, operation, "ephemeris"));
        out.write(c_try!(read(&ephemeris.inner)));
        SidereonStatus::Ok
    })
}

/// Read the DE version number.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_version(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut u32,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_version",
        ephemeris,
        out,
        |ephemeris| Ok(ephemeris.version()),
    )
}

/// Read the DE record size in bytes.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_record_size(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut usize,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_record_size",
        ephemeris,
        out,
        |ephemeris| Ok(ephemeris.record_size()),
    )
}

/// Read the DE coverage endpoints.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_coverage_tdb_seconds(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut SidereonDeCoverage,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_coverage_tdb_seconds",
        ephemeris,
        out,
        |ephemeris| {
            Ok({
                let (start_tdb_seconds, end_tdb_seconds) = ephemeris.coverage_tdb_seconds();
                SidereonDeCoverage {
                    start_tdb_seconds,
                    end_tdb_seconds,
                }
            })
        },
    )
}

/// Read the astronomical unit in metres.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_astronomical_unit_m(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut f64,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_astronomical_unit_m",
        ephemeris,
        out,
        |ephemeris| Ok(ephemeris.astronomical_unit_m()),
    )
}

/// Read the Earth to Moon mass ratio.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_earth_moon_mass_ratio(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut f64,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_earth_moon_mass_ratio",
        ephemeris,
        out,
        |ephemeris| Ok(ephemeris.earth_moon_mass_ratio()),
    )
}

/// Count the constants in the DE header.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_constant_count(
    ephemeris: *const SidereonDeEphemeris,
    out: *mut usize,
) -> SidereonStatus {
    de_query(
        "sidereon_de_ephemeris_constant_count",
        ephemeris,
        out,
        |ephemeris| Ok(ephemeris.constants().len()),
    )
}

/// Evaluate geocentric GCRF positions in metres.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_sun_moon_positions_at_tdb_seconds(
    ephemeris: *const SidereonDeEphemeris,
    epoch_tdb_seconds: f64,
    out: *mut SidereonSunMoon,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_sun_moon_positions_at_tdb_seconds";
    de_query(OP, ephemeris, out, |ephemeris| {
        let value = ephemeris
            .sun_moon_positions_at_tdb_seconds(epoch_tdb_seconds)
            .map_err(|err| map_de_error(OP, err))?;
        Ok(SidereonSunMoon {
            sun_m: value.sun,
            moon_m: value.moon,
        })
    })
}

/// Evaluate geocentric GCRF positions and velocities.
///
/// Safety: ephemeris is a live handle and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_sun_moon_at_tdb_seconds(
    ephemeris: *const SidereonDeEphemeris,
    epoch_tdb_seconds: f64,
    out: *mut SidereonDeSunMoon,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_sun_moon_at_tdb_seconds";
    de_query(OP, ephemeris, out, |ephemeris| {
        let value = ephemeris
            .sun_moon_at_tdb_seconds(epoch_tdb_seconds)
            .map_err(|err| map_de_error(OP, err))?;
        Ok(SidereonDeSunMoon {
            sun: value.sun.into(),
            moon: value.moon.into(),
        })
    })
}

/// Read a header constant in lexicographic name order. The name is UTF-8
/// without a terminator. NULL and zero capacity query the full byte count.
///
/// Safety: ephemeris is live. All outputs are writable and disjoint from
/// each other and from the handle. out_name has name_capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_constant(
    ephemeris: *const SidereonDeEphemeris,
    index: usize,
    out_name: *mut u8,
    name_capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_value: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_constant";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        c_try!(check_named_value_outputs(
            OP,
            out_name,
            0,
            out_written,
            out_required,
            out_value
        ));
        c_try!(init_handle_copy_counts(
            OP,
            out_name,
            name_capacity,
            out_written,
            out_required,
            ephemeris,
            "ephemeris"
        ));
        let out_value = c_try!(require_out(out_value, OP, "out_value"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out_value,
            "out_value",
            ephemeris,
            "ephemeris"
        ));
        let ephemeris = c_try!(require_ref(ephemeris, OP, "ephemeris"));
        let Some((name, &value)) = ephemeris.inner.constants().iter().nth(index) else {
            set_last_error(format!("{OP}: constant index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        if name_capacity >= name.len() {
            c_try!(check_named_value_outputs(
                OP,
                out_name,
                name.len(),
                out_written,
                out_required,
                out_value
            ));
        }
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            name.as_bytes(),
            out_name,
            name_capacity,
            out_written,
            out_required
        ));
        out_value.write(value);
        SidereonStatus::Ok
    })
}

/// A DE header mass parameter that may be absent.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonDeGm {
    /// Whether the header provides the parameter.
    pub present: bool,
    /// Gravitational mass parameter in m³/s², or zero when absent.
    pub value_m3_s2: f64,
}

/// Read the gravitational mass parameter for a body in m³/s².
///
/// Safety: ephemeris is live and out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_de_ephemeris_gm_m3_s2(
    ephemeris: *const SidereonDeEphemeris,
    body: u32,
    out: *mut SidereonDeGm,
) -> SidereonStatus {
    const OP: &str = "sidereon_de_ephemeris_gm_m3_s2";
    de_query(OP, ephemeris, out, |ephemeris| {
        let body = match body {
            0 => DeBody::Sun,
            1 => DeBody::Mercury,
            2 => DeBody::Venus,
            3 => DeBody::EarthMoon,
            4 => DeBody::Earth,
            5 => DeBody::Moon,
            6 => DeBody::Mars,
            7 => DeBody::Jupiter,
            8 => DeBody::Saturn,
            9 => DeBody::Uranus,
            10 => DeBody::Neptune,
            11 => DeBody::Pluto,
            12 => DeBody::SolarSystem,
            _ => {
                set_last_error(format!("{OP}: invalid DE body {body}"));
                return Err(SidereonStatus::InvalidArgument);
            }
        };
        Ok(ephemeris
            .gm_m3_s2(body)
            .map_or_else(SidereonDeGm::default, |value_m3_s2| SidereonDeGm {
                present: true,
                value_m3_s2,
            }))
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn de_error_details() {
        use DeError as E;
        let cases = [
            (
                E::Io(std::io::Error::other("sample")),
                "io",
                json!({"message":"sample","raw_os_error":null}),
            ),
            (
                E::InvalidFile("sample"),
                "invalid_file",
                json!({"reason":"sample"}),
            ),
            (E::NonFiniteEpoch, "non_finite_epoch", json!({})),
            (
                E::OutsideCoverage {
                    epoch_tdb_seconds: 0.5,
                },
                "outside_coverage",
                json!({"epoch_tdb_seconds":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (E::NonFiniteState, "non_finite_state", json!({})),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(de_error_value(&error), json!({"kind":kind,"fields":fields}));
        }
    }
}
