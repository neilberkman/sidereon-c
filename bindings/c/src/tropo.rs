use super::*;

// --- Troposphere (sidereon_core::atmosphere::troposphere) --------------------

/// Surface meteorology, mirroring sidereon_core::atmosphere::troposphere::Met.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMet {
    /// Pressure in hectopascals (millibars).
    pub pressure_hpa: f64,
    /// Temperature in kelvin.
    pub temperature_k: f64,
    /// Relative humidity as a unit fraction in [0, 1].
    pub relative_humidity: f64,
}

/// Initialize a SidereonMet with the engine's standard-atmosphere defaults,
/// sourced from sidereon_core::spp::SurfaceMet::default() (1013.25 hPa, 288.15 K,
/// 0.5 relative humidity) so C callers draw the standard atmosphere from the same
/// core source as the other bindings.
///
/// Safety: out_met must point to a SidereonMet.
#[no_mangle]
pub unsafe extern "C" fn sidereon_met_init(out_met: *mut SidereonMet) -> SidereonStatus {
    ffi_boundary("sidereon_met_init", SidereonStatus::Panic, || {
        let out_met = c_try!(require_out(out_met, "sidereon_met_init", "out_met"));
        let met = SurfaceMet::default();
        *out_met = SidereonMet {
            pressure_hpa: met.pressure_hpa,
            temperature_k: met.temperature_k,
            relative_humidity: met.relative_humidity,
        };
        SidereonStatus::Ok
    })
}

/// Zenith troposphere delay split, mirroring
/// sidereon_core::atmosphere::troposphere::ZenithDelay.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonZenithDelay {
    /// Hydrostatic (dry) zenith delay, meters.
    pub dry_m: f64,
    /// Wet zenith delay, meters.
    pub wet_m: f64,
}

/// Troposphere mapping factors, mirroring
/// sidereon_core::atmosphere::troposphere::MappingFactors.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonMappingFactors {
    /// Hydrostatic (dry) mapping factor.
    pub dry: f64,
    /// Wet mapping factor.
    pub wet: f64,
}

/// Troposphere mapping error category.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTropoMappingErrorKind {
    /// No mapping error.
    None = 0,
    /// Elevation is below the Niell mapping validity bound.
    LowElevation = 1,
    /// Another invalid input was rejected by core.
    InvalidInput = 2,
}

/// Typed troposphere mapping error detail.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTropoMappingError {
    /// Error kind as SidereonTropoMappingErrorKind.
    pub kind: u32,
    /// Input elevation, radians.
    pub elevation_rad: f64,
    /// Minimum supported mapping elevation, radians.
    pub min_elevation_rad: f64,
}

/// Zenith hydrostatic and wet troposphere delay for a receiver and surface met.
/// Delegates to sidereon_core::atmosphere::troposphere::tropo_zenith, selecting
/// the model (Saastamoinen) as a parameter to the core call rather than baking it
/// into this name. receiver latitude/longitude are radians, height meters.
///
/// Safety: receiver and met must point to their structs; out must point to a
/// SidereonZenithDelay.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tropo_zenith_delay(
    receiver: SidereonGeodetic,
    met: *const SidereonMet,
    out: *mut SidereonZenithDelay,
) -> SidereonStatus {
    ffi_boundary("sidereon_tropo_zenith_delay", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_tropo_zenith_delay", "out"));
        *out = SidereonZenithDelay {
            dry_m: 0.0,
            wet_m: 0.0,
        };
        let receiver = c_try!(geodetic_to_wgs84(
            "sidereon_tropo_zenith_delay",
            "receiver",
            receiver
        ));
        let met = c_try!(require_ref(met, "sidereon_tropo_zenith_delay", "met"));
        let met = c_try!(met_from_c("sidereon_tropo_zenith_delay", met));
        match sidereon_core::atmosphere::troposphere::tropo_zenith(
            sidereon_core::atmosphere::troposphere::TropoModel::Saastamoinen,
            receiver,
            met,
        ) {
            Ok(z) => {
                *out = SidereonZenithDelay {
                    dry_m: z.dry_m,
                    wet_m: z.wet_m,
                };
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_tropo_zenith_delay", err),
        }
    })
}

/// Hydrostatic and wet troposphere mapping factors at an elevation. Delegates to
/// sidereon_core::atmosphere::troposphere::tropo_mapping, selecting the model
/// (Niell) as a parameter to the core call rather than baking it into this name.
/// The epoch is a split Julian date in the given SidereonTimeScale.
///
/// Safety: out must point to a SidereonMappingFactors.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tropo_mapping_factors(
    elevation_rad: f64,
    receiver: SidereonGeodetic,
    scale: u32,
    jd_whole: f64,
    jd_fraction: f64,
    out: *mut SidereonMappingFactors,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tropo_mapping_factors",
        SidereonStatus::Panic,
        || {
            tropo_mapping_factors_common(
                "sidereon_tropo_mapping_factors",
                elevation_rad,
                receiver,
                TropoMappingEpoch {
                    scale,
                    jd_whole,
                    jd_fraction,
                },
                out,
                ptr::null_mut(),
            )
        },
    )
}

/// Hydrostatic and wet mapping factors with typed low-elevation error detail.
///
/// Safety: out must point to a SidereonMappingFactors; out_error must point to
/// a SidereonTropoMappingError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tropo_mapping_factors_checked(
    elevation_rad: f64,
    receiver: SidereonGeodetic,
    scale: u32,
    jd_whole: f64,
    jd_fraction: f64,
    out: *mut SidereonMappingFactors,
    out_error: *mut SidereonTropoMappingError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tropo_mapping_factors_checked",
        SidereonStatus::Panic,
        || {
            let _ = c_try!(require_out(
                out_error,
                "sidereon_tropo_mapping_factors_checked",
                "out_error"
            ));
            tropo_mapping_factors_common(
                "sidereon_tropo_mapping_factors_checked",
                elevation_rad,
                receiver,
                TropoMappingEpoch {
                    scale,
                    jd_whole,
                    jd_fraction,
                },
                out,
                out_error,
            )
        },
    )
}

/// Total slant troposphere delay in meters (Saastamoinen zenith, Niell mapping).
/// Delegates to sidereon_core::atmosphere::troposphere::tropo_slant, which
/// composes the zenith and mapping models internally.
///
/// Safety: met must point to a SidereonMet; out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tropo_slant_delay(
    elevation_rad: f64,
    receiver: SidereonGeodetic,
    met: *const SidereonMet,
    scale: u32,
    jd_whole: f64,
    jd_fraction: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_tropo_slant_delay", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_tropo_slant_delay", "out"));
        *out = 0.0;
        let receiver = c_try!(geodetic_to_wgs84(
            "sidereon_tropo_slant_delay",
            "receiver",
            receiver
        ));
        let met = c_try!(require_ref(met, "sidereon_tropo_slant_delay", "met"));
        let met = c_try!(met_from_c("sidereon_tropo_slant_delay", met));
        let epoch = c_try!(instant_from_jd_c(
            "sidereon_tropo_slant_delay",
            scale,
            jd_whole,
            jd_fraction
        ));
        match sidereon_core::atmosphere::troposphere::tropo_slant(
            elevation_rad,
            receiver,
            met,
            epoch,
        ) {
            Ok(v) => {
                *out = v;
                SidereonStatus::Ok
            }
            Err(err) => extra_invalid_arg("sidereon_tropo_slant_delay", err),
        }
    })
}

fn met_from_c(
    fn_name: &str,
    met: &SidereonMet,
) -> Result<sidereon_core::atmosphere::troposphere::Met, SidereonStatus> {
    sidereon_core::atmosphere::troposphere::Met::new(
        met.pressure_hpa,
        met.temperature_k,
        met.relative_humidity,
    )
    .map_err(|err| extra_invalid_arg(fn_name, err))
}

fn instant_from_jd_c(
    fn_name: &str,
    scale: u32,
    jd_whole: f64,
    jd_fraction: f64,
) -> Result<Instant, SidereonStatus> {
    let scale = time_scale_from_c_code(fn_name, "scale", scale)?;
    let jd = sidereon_core::astro::time::JulianDateSplit::new(jd_whole, jd_fraction)
        .map_err(|err| extra_invalid_arg(fn_name, err))?;
    Ok(Instant::from_julian_date(scale, jd))
}

fn zero_tropo_mapping_error(elevation_rad: f64) -> SidereonTropoMappingError {
    SidereonTropoMappingError {
        kind: SidereonTropoMappingErrorKind::None as u32,
        elevation_rad,
        min_elevation_rad: sidereon_core::atmosphere::troposphere::NIELL_MIN_MAPPING_ELEVATION_RAD,
    }
}

fn classify_tropo_mapping_error(elevation_rad: f64) -> SidereonTropoMappingError {
    let min_elevation_rad = sidereon_core::atmosphere::troposphere::NIELL_MIN_MAPPING_ELEVATION_RAD;
    let kind = if elevation_rad.is_finite() && elevation_rad < min_elevation_rad {
        SidereonTropoMappingErrorKind::LowElevation
    } else {
        SidereonTropoMappingErrorKind::InvalidInput
    };
    SidereonTropoMappingError {
        kind: kind as u32,
        elevation_rad,
        min_elevation_rad,
    }
}

struct TropoMappingEpoch {
    scale: u32,
    jd_whole: f64,
    jd_fraction: f64,
}

unsafe fn tropo_mapping_factors_common(
    fn_name: &str,
    elevation_rad: f64,
    receiver: SidereonGeodetic,
    epoch: TropoMappingEpoch,
    out: *mut SidereonMappingFactors,
    out_error: *mut SidereonTropoMappingError,
) -> SidereonStatus {
    let out = c_try!(require_out(out, fn_name, "out"));
    *out = SidereonMappingFactors { dry: 0.0, wet: 0.0 };
    if !out_error.is_null() {
        *out_error = zero_tropo_mapping_error(elevation_rad);
    }
    let receiver = c_try!(geodetic_to_wgs84(fn_name, "receiver", receiver));
    let epoch = c_try!(instant_from_jd_c(
        fn_name,
        epoch.scale,
        epoch.jd_whole,
        epoch.jd_fraction
    ));
    match sidereon_core::atmosphere::troposphere::tropo_mapping(
        sidereon_core::atmosphere::troposphere::MappingModel::Niell,
        elevation_rad,
        receiver,
        epoch,
    ) {
        Ok(m) => {
            *out = SidereonMappingFactors {
                dry: m.dry,
                wet: m.wet,
            };
            SidereonStatus::Ok
        }
        Err(err) => {
            if !out_error.is_null() {
                *out_error = classify_tropo_mapping_error(elevation_rad);
            }
            extra_invalid_arg(fn_name, err)
        }
    }
}
