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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_dop",
        SidereonStatus::Panic,
        || {
            let out_dop = c_try!(require_uninit_out(out_dop, "sidereon_dop", "out_dop"));
            out_dop.write(empty_dop());
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
            out_dop.write(dop_to_c(dop));
            SidereonStatus::Ok
        },
    )
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
    crate::engine_error::engine_error_operation_boundary(
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
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_dop_with_convention",
        SidereonStatus::Panic,
        || {
            let out_dop = c_try!(require_uninit_out(
                out_dop,
                "sidereon_dop_with_convention",
                "out_dop"
            ));
            out_dop.write(empty_dop());
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
            out_dop.write(dop_to_c(dop));
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

#[cfg(test)]
mod dop_engine_error_tests {
    use super::*;
    use crate::engine_error::{clear_engine_error, snapshot_engine_error_for_test};

    fn receiver() -> SidereonGeodetic {
        SidereonGeodetic {
            lat_rad: 0.0,
            lon_rad: 0.0,
            height_m: 0.0,
        }
    }

    fn tetrahedron() -> [SidereonLineOfSight; 4] {
        let s = 1.0 / 3.0_f64.sqrt();
        [
            SidereonLineOfSight {
                e_x: s,
                e_y: s,
                e_z: s,
            },
            SidereonLineOfSight {
                e_x: s,
                e_y: -s,
                e_z: -s,
            },
            SidereonLineOfSight {
                e_x: -s,
                e_y: s,
                e_z: -s,
            },
            SidereonLineOfSight {
                e_x: -s,
                e_y: -s,
                e_z: s,
            },
        ]
    }

    fn assert_error(operation: &str, error: serde_json::Value, legacy: &str) {
        let (info, payload) = snapshot_engine_error_for_test().expect("typed DOP detail");
        assert_eq!(
            info.family,
            crate::engine_error::SidereonEngineErrorFamily::Dop
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("DOP JSON"),
            serde_json::json!({
                "schema_version": 1,
                "family": "dop",
                "operation": operation,
                "error": error,
            })
        );
        let required = unsafe { crate::sidereon_last_error_message(ptr::null_mut(), 0) };
        let mut message = vec![0 as std::ffi::c_char; required + 1];
        unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }
                .to_str()
                .expect("legacy utf-8"),
            legacy
        );
    }

    fn seed_too_few_dop(los: &[SidereonLineOfSight], weights: &[f64]) {
        let mut output = empty_dop();
        assert_eq!(
            unsafe {
                sidereon_dop(
                    los.as_ptr(),
                    weights.as_ptr(),
                    los.len(),
                    receiver(),
                    &mut output,
                )
            },
            SidereonStatus::Solve
        );
        assert!(snapshot_engine_error_for_test().is_some());
    }

    #[test]
    fn dop_producers_keep_full_errors_and_clear_on_success_or_early_null() {
        clear_engine_error();
        let valid = tetrahedron();
        let weights = [1.0; 4];
        let too_few = &valid[..3];
        let few_weights = &weights[..3];
        let mut output = empty_dop();
        assert_eq!(
            unsafe {
                sidereon_dop(
                    too_few.as_ptr(),
                    few_weights.as_ptr(),
                    too_few.len(),
                    receiver(),
                    &mut output,
                )
            },
            SidereonStatus::Solve
        );
        assert_error(
            "sidereon_dop",
            serde_json::json!({"kind":"too_few_satellites","fields":{}}),
            "sidereon_dop: fewer satellites than parameters: geometry is rank-deficient",
        );
        let singular = [SidereonLineOfSight {
            e_x: 1.0,
            e_y: 0.0,
            e_z: 0.0,
        }; 4];
        assert_eq!(
            unsafe {
                sidereon_dop(
                    singular.as_ptr(),
                    weights.as_ptr(),
                    singular.len(),
                    receiver(),
                    &mut output,
                )
            },
            SidereonStatus::Solve
        );
        assert_error(
            "sidereon_dop",
            serde_json::json!({"kind":"singular","fields":{}}),
            "sidereon_dop: singular or ill-conditioned geometry: no finite DOP",
        );
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe {
                sidereon_dop(
                    valid.as_ptr(),
                    weights.as_ptr(),
                    valid.len(),
                    receiver(),
                    &mut output,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe {
                sidereon_dop(
                    valid.as_ptr(),
                    weights.as_ptr(),
                    valid.len(),
                    receiver(),
                    ptr::null_mut(),
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let bad_los = [SidereonLineOfSight {
            e_x: 2.0,
            e_y: 0.0,
            e_z: 0.0,
        }; 4];
        assert_eq!(
            unsafe {
                sidereon_dop_with_convention(
                    bad_los.as_ptr(),
                    weights.as_ptr(),
                    bad_los.len(),
                    receiver(),
                    SidereonEnuConvention::GeodeticNormal as u32,
                    &mut output,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_error(
            "sidereon_dop_with_convention",
            serde_json::json!({"kind":"invalid_input","fields":{"field":"los","reason":"not unit length"}}),
            "sidereon_dop_with_convention: invalid DOP input los: not unit length",
        );
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe {
                sidereon_dop_with_convention(
                    valid.as_ptr(),
                    weights.as_ptr(),
                    valid.len(),
                    receiver(),
                    SidereonEnuConvention::GeodeticNormal as u32,
                    &mut output,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe {
                sidereon_dop_with_convention(
                    valid.as_ptr(),
                    weights.as_ptr(),
                    valid.len(),
                    receiver(),
                    SidereonEnuConvention::GeodeticNormal as u32,
                    ptr::null_mut(),
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let mut los_out = SidereonLineOfSight {
            e_x: 0.0,
            e_y: 0.0,
            e_z: 0.0,
        };
        assert_eq!(
            unsafe {
                sidereon_line_of_sight_from_az_el_deg(f64::NAN, 10.0, receiver(), &mut los_out)
            },
            SidereonStatus::InvalidArgument
        );
        assert_error(
            "sidereon_line_of_sight_from_az_el_deg",
            serde_json::json!({"kind":"invalid_input","fields":{"field":"azimuth_deg","reason":"not finite"}}),
            "sidereon_line_of_sight_from_az_el_deg: invalid DOP input azimuth_deg: not finite",
        );
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe { sidereon_line_of_sight_from_az_el_deg(0.0, 10.0, receiver(), &mut los_out) },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());
        seed_too_few_dop(too_few, few_weights);
        assert_eq!(
            unsafe {
                sidereon_line_of_sight_from_az_el_deg(0.0, 10.0, receiver(), ptr::null_mut())
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());
        clear_engine_error();
    }
}
