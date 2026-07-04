use super::*;

// === Standalone DOP =========================================================

/// An ECEF line-of-sight unit vector from the receiver toward a satellite.
///
/// The design-matrix row this contributes is `[-e_x, -e_y, -e_z, 1]`. The
/// vector must be unit length to the engine's tolerance.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLineOfSight {
    /// ECEF X component of the unit line-of-sight vector.
    pub e_x: f64,
    /// ECEF Y component of the unit line-of-sight vector.
    pub e_y: f64,
    /// ECEF Z component of the unit line-of-sight vector.
    pub e_z: f64,
}

/// Compute the dilution-of-precision scalars from line-of-sight unit vectors,
/// diagonal weights, and the receiver geodetic position. Writes the result to
/// *out_dop. This is the standalone DOP entry; the buried per-solution DOP is
/// also available via sidereon_spp_solution_dop. The numbers are exactly what
/// the engine's dop kernel produces.
///
/// `los` and `weights` must each point to `count` entries (`count` at least
/// four). A rank-deficient or singular geometry returns SIDEREON_STATUS_SOLVE.
///
/// Safety: los and weights must each point to count readable entries (or be
/// NULL when count is 0); out_dop must point to a SidereonDop.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dop(
    los: *const SidereonLineOfSight,
    weights: *const f64,
    count: usize,
    receiver: SidereonGeodetic,
    out_dop: *mut SidereonDop,
) -> SidereonStatus {
    ffi_boundary("sidereon_dop", SidereonStatus::Panic, || {
        let out_dop = c_try!(require_out(out_dop, "sidereon_dop", "out_dop"));
        *out_dop = empty_dop();
        let los = c_try!(require_slice(los, count, "sidereon_dop", "los"));
        let weights = c_try!(require_slice(weights, count, "sidereon_dop", "weights"));
        let receiver = c_try!(geodetic_to_wgs84("sidereon_dop", "receiver", receiver));
        let rows: Vec<LineOfSight> = los
            .iter()
            .map(|l| LineOfSight::new(l.e_x, l.e_y, l.e_z))
            .collect();
        let dop = match core_dop(&rows, weights, receiver) {
            Ok(dop) => dop,
            Err(err) => return map_dop_error("sidereon_dop", err),
        };
        *out_dop = dop_to_c(dop);
        SidereonStatus::Ok
    })
}

/// Construct an ECEF line-of-sight unit vector from topocentric azimuth and
/// elevation in degrees at the receiver, writing it to *out_los. Azimuth is
/// clockwise from geodetic north; elevation is positive above the horizon.
///
/// Safety: out_los must point to a SidereonLineOfSight.
#[no_mangle]
pub unsafe extern "C" fn sidereon_line_of_sight_from_az_el_deg(
    azimuth_deg: f64,
    elevation_deg: f64,
    receiver: SidereonGeodetic,
    out_los: *mut SidereonLineOfSight,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_line_of_sight_from_az_el_deg",
        SidereonStatus::Panic,
        || {
            let out_los = c_try!(require_out(
                out_los,
                "sidereon_line_of_sight_from_az_el_deg",
                "out_los"
            ));
            *out_los = SidereonLineOfSight {
                e_x: 0.0,
                e_y: 0.0,
                e_z: 0.0,
            };
            let receiver = c_try!(geodetic_to_wgs84(
                "sidereon_line_of_sight_from_az_el_deg",
                "receiver",
                receiver
            ));
            let los = match line_of_sight_from_az_el_deg(azimuth_deg, elevation_deg, receiver) {
                Ok(los) => los,
                Err(err) => return map_dop_error("sidereon_line_of_sight_from_az_el_deg", err),
            };
            *out_los = SidereonLineOfSight {
                e_x: los.e_x,
                e_y: los.e_y,
                e_z: los.e_z,
            };
            SidereonStatus::Ok
        },
    )
}

/// Dilution-of-precision scalars with an explicit ENU convention. Like
/// sidereon_dop but the horizontal/vertical split uses `convention`. Delegates
/// to the core `dop_with_convention`.
///
/// Safety: los and weights must each point to count readable entries (or be
/// NULL when count is 0); out_dop must point to a SidereonDop.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dop_with_convention(
    los: *const SidereonLineOfSight,
    weights: *const f64,
    count: usize,
    receiver: SidereonGeodetic,
    convention: u32,
    out_dop: *mut SidereonDop,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dop_with_convention",
        SidereonStatus::Panic,
        || {
            let out_dop = c_try!(require_out(
                out_dop,
                "sidereon_dop_with_convention",
                "out_dop"
            ));
            *out_dop = empty_dop();
            let convention = c_try!(enu_convention_from_c(
                "sidereon_dop_with_convention",
                "convention",
                convention
            ));
            let los = c_try!(require_slice(
                los,
                count,
                "sidereon_dop_with_convention",
                "los"
            ));
            let weights = c_try!(require_slice(
                weights,
                count,
                "sidereon_dop_with_convention",
                "weights"
            ));
            let receiver = c_try!(geodetic_to_wgs84(
                "sidereon_dop_with_convention",
                "receiver",
                receiver
            ));
            let rows: Vec<LineOfSight> = los
                .iter()
                .map(|l| LineOfSight::new(l.e_x, l.e_y, l.e_z))
                .collect();
            let dop = match core_dop_with_convention(&rows, weights, receiver, convention) {
                Ok(dop) => dop,
                Err(err) => return map_dop_error("sidereon_dop_with_convention", err),
            };
            *out_dop = dop_to_c(dop);
            SidereonStatus::Ok
        },
    )
}

fn enu_convention_from_c(
    fn_name: &str,
    arg_name: &str,
    convention: u32,
) -> Result<EnuConvention, SidereonStatus> {
    match convention {
        value if value == SidereonEnuConvention::GeodeticNormal as u32 => {
            Ok(EnuConvention::GeodeticNormal)
        }
        value if value == SidereonEnuConvention::GeocentricRadial as u32 => {
            Ok(EnuConvention::GeocentricRadial)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} ENU convention"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
