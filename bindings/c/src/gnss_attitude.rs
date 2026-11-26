use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, engine_f64, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::attitude::gnss::{
    self as gnss, AttitudeTrajectory, BeidouManufacturer, BeidouOrbitClass, GnssAttitudeError,
    GnssAttitudeLaw, GnssBlock, GnssBlockError,
};
use sidereon_core::astro::de::DeEphemeris;
use sidereon_core::astro::frames::{EarthOrientationProvider, Iers2010EarthOrientationProvider};
use sidereon_core::astro::propagator::{analytic as gnss_analytic, dense_output::DenseOutput};

/// Yaw law mode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidereonAttitudeMode {
    /// Nominal yaw steering.
    #[default]
    Nominal = 0,
    /// Noon turn.
    NoonTurn = 1,
    /// Midnight turn.
    MidnightTurn = 2,
    /// Shadow crossing or post-shadow recovery.
    Shadow = 3,
    /// Orbit-normal mode.
    OrbitNormal = 4,
    /// Yaw steering with modified Sun elevation.
    ModifiedYawSteering = 5,
}

/// Angle used to start a cosine yaw turn.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidereonCosineTurnStartAngle {
    /// Angle between the satellite and the Sun or anti-Sun direction.
    #[default]
    Colinearity = 0,
    /// In-plane angle from noon or midnight.
    OrbitAngle = 1,
}

/// BeiDou-3 spacecraft manufacturer.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonBeidouManufacturer {
    /// China Academy of Space Technology.
    Cast = 0,
    /// Shanghai Engineering Center for Microsatellites.
    Secm = 1,
}

/// BeiDou spacecraft orbit class.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonBeidouOrbitClass {
    /// Geostationary orbit.
    Geo = 0,
    /// Inclined geosynchronous orbit.
    Igso = 1,
    /// Medium Earth orbit.
    Meo = 2,
}

/// Spacecraft block with a published GNSS attitude law.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonGnssBlock {
    /// GPS IIA.
    GpsIIA = 0,
    /// GPS IIR.
    GpsIIR = 1,
    /// GPS IIR-M.
    GpsIIRM = 2,
    /// GPS IIF.
    GpsIIF = 3,
    /// GPS III.
    GpsIII = 4,
    /// GLONASS-M.
    GlonassM = 5,
    /// Galileo IOV.
    GalileoIov = 6,
    /// Galileo FOC.
    GalileoFoc = 7,
    /// BeiDou-2 GEO.
    Beidou2Geo = 8,
    /// BeiDou-2 IGSO.
    Beidou2Igso = 9,
    /// BeiDou-2 MEO.
    Beidou2Meo = 10,
    /// BeiDou-3 GEO.
    Beidou3Geo = 11,
    /// BeiDou-3 IGSO.
    Beidou3Igso = 12,
    /// BeiDou-3 MEO, unknown manufacturer.
    Beidou3MeoUnknown = 13,
    /// BeiDou-3 MEO, CAST.
    Beidou3MeoCast = 14,
    /// BeiDou-3 MEO, SECM.
    Beidou3MeoSecm = 15,
}

fn block_from_c(operation: &str, block: u32) -> Result<GnssBlock, SidereonStatus> {
    match block {
        0 => Ok(GnssBlock::GpsIIA),
        1 => Ok(GnssBlock::GpsIIR),
        2 => Ok(GnssBlock::GpsIIRM),
        3 => Ok(GnssBlock::GpsIIF),
        4 => Ok(GnssBlock::GpsIII),
        5 => Ok(GnssBlock::GlonassM),
        6 => Ok(GnssBlock::GalileoIov),
        7 => Ok(GnssBlock::GalileoFoc),
        8 => Ok(GnssBlock::Beidou2Geo),
        9 => Ok(GnssBlock::Beidou2Igso),
        10 => Ok(GnssBlock::Beidou2Meo),
        11 => Ok(GnssBlock::Beidou3Geo),
        12 => Ok(GnssBlock::Beidou3Igso),
        13 => Ok(GnssBlock::Beidou3Meo { manufacturer: None }),
        14 => Ok(GnssBlock::Beidou3Meo {
            manufacturer: Some(BeidouManufacturer::Cast),
        }),
        15 => Ok(GnssBlock::Beidou3Meo {
            manufacturer: Some(BeidouManufacturer::Secm),
        }),
        _ => Err(extra_invalid_arg(
            operation,
            format_args!("invalid GNSS block {block}"),
        )),
    }
}

fn block_to_c(block: GnssBlock) -> SidereonGnssBlock {
    match block {
        GnssBlock::GpsIIA => SidereonGnssBlock::GpsIIA,
        GnssBlock::GpsIIR => SidereonGnssBlock::GpsIIR,
        GnssBlock::GpsIIRM => SidereonGnssBlock::GpsIIRM,
        GnssBlock::GpsIIF => SidereonGnssBlock::GpsIIF,
        GnssBlock::GpsIII => SidereonGnssBlock::GpsIII,
        GnssBlock::GlonassM => SidereonGnssBlock::GlonassM,
        GnssBlock::GalileoIov => SidereonGnssBlock::GalileoIov,
        GnssBlock::GalileoFoc => SidereonGnssBlock::GalileoFoc,
        GnssBlock::Beidou2Geo => SidereonGnssBlock::Beidou2Geo,
        GnssBlock::Beidou2Igso => SidereonGnssBlock::Beidou2Igso,
        GnssBlock::Beidou2Meo => SidereonGnssBlock::Beidou2Meo,
        GnssBlock::Beidou3Geo => SidereonGnssBlock::Beidou3Geo,
        GnssBlock::Beidou3Igso => SidereonGnssBlock::Beidou3Igso,
        GnssBlock::Beidou3Meo { manufacturer: None } => SidereonGnssBlock::Beidou3MeoUnknown,
        GnssBlock::Beidou3Meo {
            manufacturer: Some(BeidouManufacturer::Cast),
        } => SidereonGnssBlock::Beidou3MeoCast,
        GnssBlock::Beidou3Meo {
            manufacturer: Some(BeidouManufacturer::Secm),
        } => SidereonGnssBlock::Beidou3MeoSecm,
    }
}

/// Satellite and Sun geometry in a common geocentric inertial frame. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonAttitudeSample {
    /// Satellite position in metres.
    pub position_m: [f64; 3],
    /// Satellite velocity in metres per second.
    pub velocity_m_s: [f64; 3],
    /// Geocentric Sun position in metres.
    pub sun_position_m: [f64; 3],
}

/// Nominal yaw and body axes. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonNominalYaw {
    /// Nominal yaw angle, radians.
    pub yaw_rad: f64,
    /// Sun elevation above the orbit plane, radians.
    pub beta_rad: f64,
    /// Body X unit vector.
    pub body_x: [f64; 3],
    /// Body Y unit vector.
    pub body_y: [f64; 3],
    /// Body Z unit vector.
    pub body_z: [f64; 3],
}

/// Sun-fixed axes in the input frame. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonDybAxes {
    /// D unit vector.
    pub d: [f64; 3],
    /// Y unit vector.
    pub y: [f64; 3],
    /// B unit vector.
    pub b: [f64; 3],
}

/// Satellite body axes and yaw from a GNSS attitude law. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonGnssAttitude {
    /// Yaw law mode.
    pub mode: SidereonAttitudeMode,
    /// Yaw in radians.
    pub yaw_rad: f64,
    /// Nominal yaw in radians.
    pub nominal_yaw_rad: f64,
    /// Sun elevation above the orbit plane, radians.
    pub beta_rad: f64,
    /// In-plane orbit angle, radians.
    pub orbit_angle_rad: f64,
    /// Body X unit vector.
    pub body_x: [f64; 3],
    /// Body Y unit vector.
    pub body_y: [f64; 3],
    /// Body Z unit vector.
    pub body_z: [f64; 3],
    /// Whether the DYB frame is defined.
    pub has_dyb: bool,
    /// DYB unit vectors when defined.
    pub dyb: SidereonDybAxes,
}

/// Cosine yaw-turn configuration. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonCosineTurnParameters {
    /// Absolute Sun elevation limit, radians.
    pub beta_limit_rad: f64,
    /// Whether the elevation limit is inclusive.
    pub beta_limit_inclusive: bool,
    /// Start angle selector as SidereonCosineTurnStartAngle.
    pub start_angle: u32,
    /// Angle before noon or midnight at the start, radians.
    pub start_angle_rad: f64,
    /// Cosine period in seconds.
    pub period_s: f64,
}

impl From<gnss::CosineTurnParameters> for SidereonCosineTurnParameters {
    fn from(params: gnss::CosineTurnParameters) -> Self {
        Self {
            beta_limit_rad: params.beta_limit_rad,
            beta_limit_inclusive: params.beta_limit_inclusive,
            start_angle: match params.start_angle {
                gnss::CosineTurnStartAngle::Colinearity => {
                    SidereonCosineTurnStartAngle::Colinearity as u32
                }
                gnss::CosineTurnStartAngle::OrbitAngle => {
                    SidereonCosineTurnStartAngle::OrbitAngle as u32
                }
                _ => u32::MAX,
            },
            start_angle_rad: params.start_angle_rad,
            period_s: params.period_s,
        }
    }
}

fn cosine_turn_from_c(
    operation: &str,
    params: SidereonCosineTurnParameters,
) -> Result<gnss::CosineTurnParameters, SidereonStatus> {
    let start_angle = match params.start_angle {
        0 => gnss::CosineTurnStartAngle::Colinearity,
        1 => gnss::CosineTurnStartAngle::OrbitAngle,
        value => {
            return Err(extra_invalid_arg(
                operation,
                format_args!("invalid cosine turn start angle {value}"),
            ))
        }
    };
    Ok(gnss::CosineTurnParameters {
        beta_limit_rad: params.beta_limit_rad,
        beta_limit_inclusive: params.beta_limit_inclusive,
        start_angle,
        start_angle_rad: params.start_angle_rad,
        period_s: params.period_s,
    })
}

fn block_error_value(error: &GnssBlockError) -> serde_json::Value {
    use GnssBlockError as E;
    let (kind, fields) = match error {
        E::Unknown { name } => ("unknown", json!({"name":name})),
        E::Unsupported { name } => ("unsupported", json!({"name":name})),
        E::BeidouOrbitClassRequired { block_type } => (
            "beidou_orbit_class_required",
            json!({"block_type":block_type}),
        ),
        _ => ("unknown", json!({})),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_block_error(operation: &str, error: GnssBlockError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::GnssBlock,
        operation,
        block_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

fn attitude_error_value(error: &GnssAttitudeError) -> serde_json::Value {
    use GnssAttitudeError as E;
    let (kind, fields) = match error {
        E::NonFinite { field } => ("non_finite", json!({"field":field})),
        E::ZeroLength { field } => ("zero_length", json!({"field":field})),
        E::TrajectoryUnavailable { t_s } => {
            ("trajectory_unavailable", json!({"t_s":engine_f64(*t_s)}))
        }
        E::InvalidParameter { field, value } => (
            "invalid_parameter",
            json!({"field":field, "value":engine_f64(*value)}),
        ),
        E::NoDefaultYawRate { prn } => ("no_default_yaw_rate", json!({"prn":prn})),
        E::DegenerateGeometry { t_s } => {
            ("degenerate_geometry", json!({"t_s":t_s.map(engine_f64)}))
        }
        E::BeidouManufacturerRequired { t_s } => (
            "beidou_manufacturer_required",
            json!({"t_s":engine_f64(*t_s)}),
        ),
        E::TurnEndNotFound { t_s } => ("turn_end_not_found", json!({"t_s":engine_f64(*t_s)})),
        E::OutsideValidatedDomain { t_s, reason } => (
            "outside_validated_domain",
            json!({"t_s":engine_f64(*t_s), "reason":reason}),
        ),
        _ => ("unknown", json!({})),
    };
    json!({"kind":kind, "fields":fields})
}

fn map_attitude_error(operation: &str, error: GnssAttitudeError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::GnssAttitude,
        operation,
        attitude_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

/// Owned GNSS satellite attitude law. Release with sidereon_gnss_attitude_law_free.
pub struct SidereonGnssAttitudeLaw {
    inner: GnssAttitudeLaw,
}

unsafe fn new_law(
    operation: &str,
    out: *mut *mut SidereonGnssAttitudeLaw,
    build: impl FnOnce() -> Result<GnssAttitudeLaw, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        out.write(ptr::null_mut());
        let inner = c_try!(build());
        out.write(Box::into_raw(Box::new(SidereonGnssAttitudeLaw { inner })));
        SidereonStatus::Ok
    })
}

/// Release a GNSS attitude law.
///
/// Safety: law is NULL or a live owned law handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_free(law: *mut SidereonGnssAttitudeLaw) {
    ffi_boundary("sidereon_gnss_attitude_law_free", (), || free_boxed(law));
}

/// Compare two attitude laws by value.
///
/// Safety: both handles are live and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_equal(
    left: *const SidereonGnssAttitudeLaw,
    right: *const SidereonGnssAttitudeLaw,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_equal";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", left, "left"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", right, "right"
        ));
        out.write(false);
        let left = c_try!(require_ref(left, OP, "left"));
        let right = c_try!(require_ref(right, OP, "right"));
        out.write(left.inner == right.inner);
        SidereonStatus::Ok
    })
}

/// Create the published attitude law for a satellite block.
/// Pass zero for an unspecified GPS IIA PRN.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_for_block(
    block: u32,
    prn: u8,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_for_block";
    new_law(OP, out, || {
        let block = block_from_c(OP, block)?;
        GnssAttitudeLaw::for_block(block, (prn != 0).then_some(prn))
            .map_err(|err| map_attitude_error(OP, err))
    })
}

/// Create the GPS IIA attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_gps_iia(
    yaw_rate_rad_s: f64,
    yaw_bias_rad: f64,
    shadow_limit_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_gps_iia";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::GpsIIA {
            yaw_rate_rad_s,
            yaw_bias_rad,
            shadow_limit_rad,
        })
    })
}

/// Create the GPS IIR attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_gps_iir(
    yaw_rate_rad_s: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_gps_iir";
    new_law(OP, out, || Ok(GnssAttitudeLaw::GpsIIR { yaw_rate_rad_s }))
}

/// Create the GPS IIF attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_gps_iif(
    yaw_rate_rad_s: f64,
    yaw_bias_rad: f64,
    shadow_limit_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_gps_iif";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::GpsIIF {
            yaw_rate_rad_s,
            yaw_bias_rad,
            shadow_limit_rad,
        })
    })
}

/// Create the GLONASS-M attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_glonass_m(
    yaw_rate_rad_s: f64,
    shadow_limit_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_glonass_m";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::GlonassM {
            yaw_rate_rad_s,
            shadow_limit_rad,
        })
    })
}

/// Create the Galileo IOV attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_galileo_iov(
    beta_x_rad: f64,
    beta_y_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_galileo_iov";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::GalileoIov {
            beta_x_rad,
            beta_y_rad,
        })
    })
}

/// Create the Cosine yaw turn attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_cosine_turn(
    parameters: SidereonCosineTurnParameters,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_cosine_turn";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::CosineTurn(cosine_turn_from_c(
            OP, parameters,
        )?))
    })
}

/// Create the BeiDou orbit-normal switch attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_beidou_orbit_normal_switch(
    ys_to_on_beta_rad: f64,
    on_to_ys_beta_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_beidou_orbit_normal_switch";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::BeidouOrbitNormalSwitch {
            ys_to_on_beta_rad,
            on_to_ys_beta_rad,
        })
    })
}

/// Create the Fixed-beta yaw steering attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_fixed_beta_yaw_steering(
    beta0_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_fixed_beta_yaw_steering";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::FixedBetaYawSteering { beta0_rad })
    })
}

/// Create the BeiDou-3 MEO with unknown manufacturer attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_beidou3_meo_unknown_manufacturer(
    cast: SidereonCosineTurnParameters,
    secm_beta0_rad: f64,
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_beidou3_meo_unknown_manufacturer";
    new_law(OP, out, || {
        Ok(GnssAttitudeLaw::Beidou3MeoUnknownManufacturer {
            cast: cosine_turn_from_c(OP, cast)?,
            secm_beta0_rad,
        })
    })
}

/// Create the Orbit-normal mode attitude law.
///
/// Safety: out points to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude_law_orbit_normal(
    out: *mut *mut SidereonGnssAttitudeLaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude_law_orbit_normal";
    new_law(OP, out, || Ok(GnssAttitudeLaw::OrbitNormal))
}

/// Parse an IGS/ANTEX satellite block name.
///
/// Safety: data is readable for len bytes. out points to disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_from_igs_name(
    data: *const u8,
    len: usize,
    out: *mut SidereonGnssBlock,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_from_igs_name";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let name = c_try!(text_bytes_from_c(OP, data, len));
        let block = c_try!(GnssBlock::from_igs_name(name).map_err(|err| map_block_error(OP, err)));
        out.write(block_to_c(block));
        SidereonStatus::Ok
    })
}

/// Parse a constellation block type. Pass NULL for an unspecified BeiDou orbit class.
///
/// Safety: data is readable for len bytes. beidou_orbit is NULL or points to
/// an orbit class code. out points to disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_from_constellation_block_type(
    system: u32,
    data: *const u8,
    len: usize,
    beidou_orbit: *const u32,
    out: *mut SidereonGnssBlock,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_from_constellation_block_type";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let system = c_try!(gnss_system_from_c_code(OP, "system", system));
        let name = c_try!(text_bytes_from_c(OP, data, len));
        let orbit = match beidou_orbit.as_ref().copied() {
            None => None,
            Some(0) => Some(BeidouOrbitClass::Geo),
            Some(1) => Some(BeidouOrbitClass::Igso),
            Some(2) => Some(BeidouOrbitClass::Meo),
            Some(value) => {
                return extra_invalid_arg(OP, format_args!("invalid BeiDou orbit class {value}"))
            }
        };
        let block = c_try!(
            GnssBlock::from_constellation_block_type(system, name, orbit)
                .map_err(|err| map_block_error(OP, err))
        );
        out.write(block_to_c(block));
        SidereonStatus::Ok
    })
}

/// Get the constellation for a spacecraft block.
///
/// Safety: out points to writable constellation storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_system(
    block: u32,
    out: *mut SidereonGnssSystem,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_system";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let block = c_try!(block_from_c(OP, block));
        out.write(gnss_system_to_c(block.system()));
        SidereonStatus::Ok
    })
}

/// Copy the canonical IGS name without a NUL terminator. NULL and zero
/// capacity query the full byte count.
///
/// Safety: all outputs are writable and disjoint. out_name has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_igs_name(
    block: u32,
    out_name: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_igs_name";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let block = c_try!(block_from_c(OP, block));
        c_try!(copy_prefix_to_c(
            OP,
            "out_name",
            block.igs_name().as_bytes(),
            out_name,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Make a BeiDou-3 MEO block. Pass NULL for an unspecified manufacturer.
///
/// Safety: manufacturer is NULL or points to a manufacturer code. out points
/// to disjoint writable block storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_beidou3_meo(
    manufacturer: *const u32,
    out: *mut SidereonGnssBlock,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_beidou3_meo";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let manufacturer = match manufacturer.as_ref().copied() {
            None => None,
            Some(0) => Some(BeidouManufacturer::Cast),
            Some(1) => Some(BeidouManufacturer::Secm),
            Some(value) => {
                return extra_invalid_arg(OP, format_args!("invalid BeiDou manufacturer {value}"))
            }
        };
        out.write(block_to_c(GnssBlock::Beidou3Meo { manufacturer }));
        SidereonStatus::Ok
    })
}

/// Compare two spacecraft blocks in satellite-block order. Writes -1, 0 or 1.
///
/// Safety: out points to writable comparison storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_block_compare(
    left: u32,
    right: u32,
    out: *mut i32,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_block_compare";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let left = c_try!(block_from_c(OP, left));
        let right = c_try!(block_from_c(OP, right));
        out.write(left.cmp(&right) as i32);
        SidereonStatus::Ok
    })
}

/// Look up the published GPS IIA maximum yaw rate by PRN.
///
/// Safety: out points to writable double storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gps_iia_default_yaw_rate_rad_s(
    prn: u8,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_gps_iia_default_yaw_rate_rad_s";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(0.0);
        let value =
            c_try!(
                gnss::gps_iia_default_yaw_rate_rad_s(prn).ok_or_else(|| map_attitude_error(
                    OP,
                    GnssAttitudeError::NoDefaultYawRate { prn: Some(prn) }
                ))
            );
        out.write(value);
        SidereonStatus::Ok
    })
}

/// Evaluate nominal yaw and body axes at a single epoch.
///
/// Safety: sample is readable and out points to disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nominal_yaw(
    sample: *const SidereonAttitudeSample,
    out: *mut SidereonNominalYaw,
) -> SidereonStatus {
    const OP: &str = "sidereon_nominal_yaw";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(SidereonNominalYaw::default());
        let sample = c_try!(require_ref(sample, OP, "sample"));
        let value = c_try!(gnss::nominal_yaw(
            sample.position_m,
            sample.velocity_m_s,
            sample.sun_position_m,
        )
        .map_err(|err| map_attitude_error(OP, err)));
        out.write(SidereonNominalYaw {
            yaw_rad: value.yaw_rad,
            beta_rad: value.beta_rad,
            body_x: value.body_x,
            body_y: value.body_y,
            body_z: value.body_z,
        });
        SidereonStatus::Ok
    })
}

/// Evaluate the Sun-fixed DYB frame from two geocentric inertial positions.
///
/// Safety: the inputs point to three doubles and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dyb_axes(
    position_m: *const f64,
    sun_position_m: *const f64,
    out: *mut SidereonDybAxes,
) -> SidereonStatus {
    const OP: &str = "sidereon_dyb_axes";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        out.write(SidereonDybAxes::default());
        let position = c_try!(read_vec3(OP, "position_m", position_m));
        let sun = c_try!(read_vec3(OP, "sun_position_m", sun_position_m));
        let value =
            c_try!(gnss::dyb_axes(position, sun).map_err(|err| map_attitude_error(OP, err)));
        out.write(SidereonDybAxes {
            d: value.d,
            y: value.y,
            b: value.b,
        });
        SidereonStatus::Ok
    })
}

/// Published gps iia default yaw bias rad.
#[no_mangle]
pub extern "C" fn sidereon_gps_iia_default_yaw_bias_rad() -> f64 {
    gnss::GPS_IIA_DEFAULT_YAW_BIAS_RAD
}

/// Published gps iir default yaw rate rad s.
#[no_mangle]
pub extern "C" fn sidereon_gps_iir_default_yaw_rate_rad_s() -> f64 {
    gnss::GPS_IIR_DEFAULT_YAW_RATE_RAD_S
}

/// Published gps iif default yaw rate rad s.
#[no_mangle]
pub extern "C" fn sidereon_gps_iif_default_yaw_rate_rad_s() -> f64 {
    gnss::GPS_IIF_DEFAULT_YAW_RATE_RAD_S
}

/// Published gps iif default yaw bias rad.
#[no_mangle]
pub extern "C" fn sidereon_gps_iif_default_yaw_bias_rad() -> f64 {
    gnss::GPS_IIF_DEFAULT_YAW_BIAS_RAD
}

/// Published gps shadow limit rad.
#[no_mangle]
pub extern "C" fn sidereon_gps_shadow_limit_rad() -> f64 {
    gnss::GPS_SHADOW_LIMIT_RAD
}

/// Published glonass m default yaw rate rad s.
#[no_mangle]
pub extern "C" fn sidereon_glonass_m_default_yaw_rate_rad_s() -> f64 {
    gnss::GLONASS_M_DEFAULT_YAW_RATE_RAD_S
}

/// Published glonass m shadow limit rad.
#[no_mangle]
pub extern "C" fn sidereon_glonass_m_shadow_limit_rad() -> f64 {
    gnss::GLONASS_M_SHADOW_LIMIT_RAD
}

/// Published galileo iov beta x rad.
#[no_mangle]
pub extern "C" fn sidereon_galileo_iov_beta_x_rad() -> f64 {
    gnss::GALILEO_IOV_BETA_X_RAD
}

/// Published galileo iov beta y rad.
#[no_mangle]
pub extern "C" fn sidereon_galileo_iov_beta_y_rad() -> f64 {
    gnss::GALILEO_IOV_BETA_Y_RAD
}

/// Published gps iii beta x rad.
#[no_mangle]
pub extern "C" fn sidereon_gps_iii_beta_x_rad() -> f64 {
    gnss::GPS_III_BETA_X_RAD
}

/// Published gps iii beta y rad.
#[no_mangle]
pub extern "C" fn sidereon_gps_iii_beta_y_rad() -> f64 {
    gnss::GPS_III_BETA_Y_RAD
}

/// Published beidou3 secm beta0 rad.
#[no_mangle]
pub extern "C" fn sidereon_beidou3_secm_beta0_rad() -> f64 {
    gnss::BEIDOU3_SECM_BETA0_RAD
}

/// Published beidou2 ys to on beta rad.
#[no_mangle]
pub extern "C" fn sidereon_beidou2_ys_to_on_beta_rad() -> f64 {
    gnss::BEIDOU2_YS_TO_ON_BETA_RAD
}

/// Published beidou2 on to ys beta rad.
#[no_mangle]
pub extern "C" fn sidereon_beidou2_on_to_ys_beta_rad() -> f64 {
    gnss::BEIDOU2_ON_TO_YS_BETA_RAD
}

/// Published galileo foc turn parameters.
#[no_mangle]
pub extern "C" fn sidereon_galileo_foc_turn() -> SidereonCosineTurnParameters {
    gnss::GALILEO_FOC_TURN.into()
}

/// Published beidou3 cast meo turn parameters.
#[no_mangle]
pub extern "C" fn sidereon_beidou3_cast_meo_turn() -> SidereonCosineTurnParameters {
    gnss::BEIDOU3_CAST_MEO_TURN.into()
}

/// Published beidou3 cast igso turn parameters.
#[no_mangle]
pub extern "C" fn sidereon_beidou3_cast_igso_turn() -> SidereonCosineTurnParameters {
    gnss::BEIDOU3_CAST_IGSO_TURN.into()
}

/// Compare two BeiDou manufacturers in enum order.
///
/// Safety: out points to writable comparison storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_beidou_manufacturer_compare(
    left: u32,
    right: u32,
    out: *mut i32,
) -> SidereonStatus {
    const OP: &str = "sidereon_beidou_manufacturer_compare";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let convert = |value| match value {
            0 => Ok(BeidouManufacturer::Cast),
            1 => Ok(BeidouManufacturer::Secm),
            _ => Err(extra_invalid_arg(
                OP,
                format_args!("invalid BeiDou manufacturer {value}"),
            )),
        };
        out.write(c_try!(convert(left)).cmp(&c_try!(convert(right))) as i32);
        SidereonStatus::Ok
    })
}

/// Compare two BeiDou orbit classes in enum order.
///
/// Safety: out points to writable comparison storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_beidou_orbit_class_compare(
    left: u32,
    right: u32,
    out: *mut i32,
) -> SidereonStatus {
    const OP: &str = "sidereon_beidou_orbit_class_compare";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        let convert = |value| match value {
            0 => Ok(BeidouOrbitClass::Geo),
            1 => Ok(BeidouOrbitClass::Igso),
            2 => Ok(BeidouOrbitClass::Meo),
            _ => Err(extra_invalid_arg(
                OP,
                format_args!("invalid BeiDou orbit class {value}"),
            )),
        };
        out.write(c_try!(convert(left)).cmp(&c_try!(convert(right))) as i32);
        SidereonStatus::Ok
    })
}

enum AttitudeOrbit {
    Sp3(Box<Sp3>, GnssSatelliteId),
    Broadcast(Arc<BroadcastEphemeris>, GnssSatelliteId),
    Scoring(Arc<crate::orbit_scoring::OwnedOrbitSource>, GnssSatelliteId),
    EcksteinHechler(gnss_analytic::EcksteinHechlerPropagator),
    BrouwerLyddane(gnss_analytic::BrouwerLyddanePropagator),
    DenseOutput(Arc<DenseOutput>),
}

/// GNSS attitude trajectory. Release with sidereon_orbit_attitude_trajectory_free.
/// The trajectory retains its inputs independently.
pub struct SidereonOrbitAttitudeTrajectory {
    orbit: AttitudeOrbit,
    orientation: Option<Arc<Iers2010EarthOrientationProvider>>,
    de_ephemeris: Option<Arc<DeEphemeris>>,
    velocity_half_step_s: f64,
}

impl SidereonOrbitAttitudeTrajectory {
    fn core<'a>(
        &'a self,
        default_orientation: &'a TdbEarthOrientationProvider,
    ) -> gnss::OrbitAttitudeTrajectory<'a> {
        let orientation: &dyn EarthOrientationProvider = match &self.orientation {
            Some(provider) => provider.as_ref(),
            None => default_orientation,
        };
        let mut trajectory = match &self.orbit {
            AttitudeOrbit::Sp3(product, sat) => {
                gnss::OrbitAttitudeTrajectory::from_sp3(product, *sat, orientation)
            }
            AttitudeOrbit::Broadcast(ephemeris, sat) => {
                gnss::OrbitAttitudeTrajectory::from_broadcast(ephemeris, *sat, orientation)
            }
            AttitudeOrbit::Scoring(source, sat) => {
                gnss::OrbitAttitudeTrajectory::from_orbit_source(source.source(), *sat, orientation)
            }
            AttitudeOrbit::EcksteinHechler(propagator) => {
                gnss::OrbitAttitudeTrajectory::from_eckstein_hechler(propagator)
            }
            AttitudeOrbit::BrouwerLyddane(propagator) => {
                gnss::OrbitAttitudeTrajectory::from_brouwer_lyddane(propagator)
            }
            AttitudeOrbit::DenseOutput(output) => {
                gnss::OrbitAttitudeTrajectory::from_dense_output(output)
            }
        };
        if let Some(ephemeris) = &self.de_ephemeris {
            trajectory = trajectory.with_de_ephemeris(ephemeris);
        }
        trajectory
            .with_velocity_half_step_s(self.velocity_half_step_s)
            .expect("validated velocity half step")
    }
}

unsafe fn new_attitude_trajectory<T>(
    operation: &str,
    source: *const T,
    orientation: *const SidereonIers2010EarthOrientationProvider,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
    orbit: impl FnOnce(&T) -> Result<AttitudeOrbit, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", source, "source"
        ));
        c_try!(reject_output_overlaps_handle(
            operation,
            out,
            "out",
            orientation,
            "orientation"
        ));
        out.write(ptr::null_mut());
        let source = c_try!(require_ref(source, operation, "source"));
        let orbit = c_try!(orbit(source));
        write_boxed_handle(
            out,
            SidereonOrbitAttitudeTrajectory {
                orbit,
                orientation: orientation
                    .as_ref()
                    .map(|provider| Arc::clone(&provider.inner)),
                de_ephemeris: None,
                velocity_half_step_s: sidereon_default_velocity_half_step_s(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Create an attitude trajectory from a parsed SP3 product.
/// Pass NULL for the default TDB Earth orientation provider.
///
/// Safety: product and any orientation provider are live during the call.
/// satellite is a NUL-terminated token and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_sp3(
    product: *const SidereonSp3,
    satellite: *const c_char,
    orientation: *const SidereonIers2010EarthOrientationProvider,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_sp3";
    new_attitude_trajectory(OP, product, orientation, out, |product| {
        Ok(AttitudeOrbit::Sp3(
            Box::new(product.inner.clone()),
            parse_satellite_token(OP, satellite)?,
        ))
    })
}

/// Create an attitude trajectory from a broadcast navigation set.
/// Pass NULL for the default TDB Earth orientation provider.
///
/// Safety: ephemeris and any orientation provider are live during the call.
/// satellite is a NUL-terminated token and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_broadcast(
    ephemeris: *const SidereonBroadcastEphemeris,
    satellite: *const c_char,
    orientation: *const SidereonIers2010EarthOrientationProvider,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_broadcast";
    new_attitude_trajectory(OP, ephemeris, orientation, out, |ephemeris| {
        Ok(AttitudeOrbit::Broadcast(
            Arc::clone(&ephemeris.inner),
            parse_satellite_token(OP, satellite)?,
        ))
    })
}

/// Create an attitude trajectory from a predicted SP3, broadcast orbit, or
/// sampled orbit scoring source. Pass NULL for the default TDB Earth
/// orientation provider.
///
/// Safety: source and any orientation provider are live during the call.
/// satellite is a NUL-terminated token and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_orbit_source(
    source: *const SidereonOrbitSource,
    satellite: *const c_char,
    orientation: *const SidereonIers2010EarthOrientationProvider,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_orbit_source";
    new_attitude_trajectory(OP, source, orientation, out, |source| {
        Ok(AttitudeOrbit::Scoring(
            Arc::clone(&source.inner),
            parse_satellite_token(OP, satellite)?,
        ))
    })
}

/// Create an attitude trajectory by copying a GCRF Eckstein–Hechler propagator.
///
/// Safety: propagator is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_eckstein_hechler(
    propagator: *const SidereonEcksteinHechlerPropagator,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_eckstein_hechler";
    new_attitude_trajectory(OP, propagator, ptr::null(), out, |source| {
        Ok(AttitudeOrbit::EcksteinHechler(source.inner.clone()))
    })
}

/// Create an attitude trajectory by copying a GCRF Brouwer–Lyddane propagator.
///
/// Safety: propagator is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_brouwer_lyddane(
    propagator: *const SidereonBrouwerLyddanePropagator,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_brouwer_lyddane";
    new_attitude_trajectory(OP, propagator, ptr::null(), out, |source| {
        Ok(AttitudeOrbit::BrouwerLyddane(source.inner.clone()))
    })
}

/// Create an attitude trajectory retaining the continuous output of a GCRF
/// numerical propagation.
///
/// Safety: output is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_from_dense_output(
    output: *const SidereonDenseOutput,
    out: *mut *mut SidereonOrbitAttitudeTrajectory,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_from_dense_output";
    new_attitude_trajectory(OP, output, ptr::null(), out, |source| {
        Ok(AttitudeOrbit::DenseOutput(Arc::clone(&source.inner)))
    })
}

/// Release an attitude trajectory.
///
/// Safety: trajectory is NULL or a live owned handle with no concurrent users.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_free(
    trajectory: *mut SidereonOrbitAttitudeTrajectory,
) {
    ffi_boundary("sidereon_orbit_attitude_trajectory_free", (), || {
        free_boxed(trajectory)
    });
}

/// Set the Sun ephemeris and retain it independently of its handle. Pass NULL
/// to use the default analytic Sun model.
///
/// Safety: trajectory is live and exclusively accessible. Ephemeris is NULL
/// or live.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_with_de_ephemeris(
    trajectory: *mut SidereonOrbitAttitudeTrajectory,
    ephemeris: *const SidereonDeEphemeris,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_with_de_ephemeris";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let trajectory = c_try!(require_mut(trajectory, OP, "trajectory"));
        trajectory.de_ephemeris = ephemeris.as_ref().map(|value| Arc::clone(&value.inner));
        SidereonStatus::Ok
    })
}

/// Set the Earth-fixed velocity difference half step, in seconds of the
/// source's time scale. The trajectory retains its previous step on refusal.
///
/// Safety: trajectory is live and exclusively accessible.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_with_velocity_half_step_s(
    trajectory: *mut SidereonOrbitAttitudeTrajectory,
    half_step_s: f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_with_velocity_half_step_s";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let trajectory = c_try!(require_mut(trajectory, OP, "trajectory"));
        let orientation = TdbEarthOrientationProvider::new();
        c_try!(trajectory
            .core(&orientation)
            .with_velocity_half_step_s(half_step_s)
            .map_err(|err| crate::orbit_scoring::map_score_error(OP, err)));
        trajectory.velocity_half_step_s = half_step_s;
        SidereonStatus::Ok
    })
}

/// Sample the satellite and Sun in GCRF at TDB seconds past J2000. Satellite
/// position and Sun position are in metres and satellite velocity is in m/s.
///
/// Safety: trajectory is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_attitude_trajectory_sample(
    trajectory: *const SidereonOrbitAttitudeTrajectory,
    t_s: f64,
    out: *mut SidereonAttitudeSample,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_attitude_trajectory_sample";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            trajectory,
            "trajectory"
        ));
        out.write(SidereonAttitudeSample::default());
        let trajectory = c_try!(require_ref(trajectory, OP, "trajectory"));
        let orientation = TdbEarthOrientationProvider::new();
        let sample = c_try!(trajectory.core(&orientation).sample(t_s).ok_or_else(|| {
            map_attitude_error(OP, GnssAttitudeError::TrajectoryUnavailable { t_s })
        }));
        out.write(SidereonAttitudeSample {
            position_m: sample.position_m,
            velocity_m_s: sample.velocity_m_s,
            sun_position_m: sample.sun_position_m,
        });
        SidereonStatus::Ok
    })
}

/// Evaluate a GNSS yaw law along a trajectory at TDB seconds past J2000.
/// The result's body and DYB axes are expressed in GCRF.
///
/// Safety: law and trajectory are live and out is disjoint writable result
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_gnss_attitude(
    law: *const SidereonGnssAttitudeLaw,
    trajectory: *const SidereonOrbitAttitudeTrajectory,
    t_s: f64,
    out: *mut SidereonGnssAttitude,
) -> SidereonStatus {
    const OP: &str = "sidereon_gnss_attitude";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(OP, out, "out", law, "law"));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            trajectory,
            "trajectory"
        ));
        out.write(SidereonGnssAttitude::default());
        let law = c_try!(require_ref(law, OP, "law"));
        let trajectory = c_try!(require_ref(trajectory, OP, "trajectory"));
        let orientation = TdbEarthOrientationProvider::new();
        let value = c_try!(
            gnss::gnss_attitude(&law.inner, &trajectory.core(&orientation), t_s)
                .map_err(|err| map_attitude_error(OP, err))
        );
        let mode = match value.mode {
            gnss::AttitudeMode::Nominal => SidereonAttitudeMode::Nominal,
            gnss::AttitudeMode::NoonTurn => SidereonAttitudeMode::NoonTurn,
            gnss::AttitudeMode::MidnightTurn => SidereonAttitudeMode::MidnightTurn,
            gnss::AttitudeMode::Shadow => SidereonAttitudeMode::Shadow,
            gnss::AttitudeMode::OrbitNormal => SidereonAttitudeMode::OrbitNormal,
            gnss::AttitudeMode::ModifiedYawSteering => SidereonAttitudeMode::ModifiedYawSteering,
            _ => return extra_invalid_arg(OP, "unknown GNSS attitude mode"),
        };
        out.write(SidereonGnssAttitude {
            mode,
            yaw_rad: value.yaw_rad,
            nominal_yaw_rad: value.nominal_yaw_rad,
            beta_rad: value.beta_rad,
            orbit_angle_rad: value.orbit_angle_rad,
            body_x: value.body_x,
            body_y: value.body_y,
            body_z: value.body_z,
            has_dyb: value.dyb.is_some(),
            dyb: value
                .dyb
                .map_or_else(SidereonDybAxes::default, |axes| SidereonDybAxes {
                    d: axes.d,
                    y: axes.y,
                    b: axes.b,
                }),
        });
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn gnss_block_error_details() {
        let cases = [
            (
                GnssBlockError::Unknown {
                    name: "X".to_owned(),
                },
                json!({"kind":"unknown","fields":{"name":"X"}}),
            ),
            (
                GnssBlockError::Unsupported {
                    name: "BLOCK I".to_owned(),
                },
                json!({"kind":"unsupported","fields":{"name":"BLOCK I"}}),
            ),
            (
                GnssBlockError::BeidouOrbitClassRequired {
                    block_type: "BDS-3".to_owned(),
                },
                json!({"kind":"beidou_orbit_class_required","fields":{"block_type":"BDS-3"}}),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(block_error_value(&error), expected);
        }
    }

    #[test]
    fn gnss_attitude_error_details() {
        use GnssAttitudeError as E;
        let cases = [
            (
                E::NonFinite { field: "sample" },
                "non_finite",
                json!({"field":"sample"}),
            ),
            (
                E::ZeroLength { field: "sample" },
                "zero_length",
                json!({"field":"sample"}),
            ),
            (
                E::TrajectoryUnavailable { t_s: 0.5 },
                "trajectory_unavailable",
                json!({"t_s":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::InvalidParameter {
                    field: "sample",
                    value: 0.5,
                },
                "invalid_parameter",
                json!({"field":"sample", "value":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::NoDefaultYawRate { prn: Some(3) },
                "no_default_yaw_rate",
                json!({"prn":3}),
            ),
            (
                E::DegenerateGeometry { t_s: Some(0.5) },
                "degenerate_geometry",
                json!({"t_s":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::BeidouManufacturerRequired { t_s: 0.5 },
                "beidou_manufacturer_required",
                json!({"t_s":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::TurnEndNotFound { t_s: 0.5 },
                "turn_end_not_found",
                json!({"t_s":{"decimal":"0.5","bits_hex":"3fe0000000000000"}}),
            ),
            (
                E::OutsideValidatedDomain {
                    t_s: 0.5,
                    reason: "sample",
                },
                "outside_validated_domain",
                json!({"t_s":{"decimal":"0.5","bits_hex":"3fe0000000000000"}, "reason":"sample"}),
            ),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(
                attitude_error_value(&error),
                json!({"kind":kind,"fields":fields})
            );
        }
    }
}
