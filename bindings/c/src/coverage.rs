use super::*;

pub struct SidereonCoverageGrid {
    pub(crate) inner: LookAngleGrid,
    pub(crate) station_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCoverageLookAngle {
    pub ok: bool,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub range_km: f64,
}

/// Copy the complete typed error for one failed grid cell as JSON bytes.
/// Successful cells and out-of-range indices are rejected. Query the required
/// byte count with a NULL output and zero capacity, then retry with a caller
/// buffer; the grid remains owned by the caller throughout.
///
/// Safety: `grid` must be live; when `len` is nonzero, `out_payload` must point
/// to `len` writable bytes; both count outputs must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_look_angle_error_payload(
    grid: *const SidereonCoverageGrid,
    sat_index: usize,
    station_index: usize,
    out_payload: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_coverage_grid_look_angle_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let grid = c_try!(require_ref(grid, FN_NAME, "grid"));
        let Some(row) = grid.inner.get(sat_index) else {
            set_last_error(format!("{FN_NAME}: sat_index {sat_index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        let Some(cell) = row.get(station_index) else {
            set_last_error(format!(
                "{FN_NAME}: station_index {station_index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        let Err(error) = cell else {
            set_last_error(format!(
                "{FN_NAME}: cell succeeded and has no error payload"
            ));
            return SidereonStatus::InvalidArgument;
        };
        let payload = match serde_json::to_vec(&crate::tle::look_angle_error_value(error)) {
            Ok(payload) => payload,
            Err(error) => {
                set_last_error(format!("{FN_NAME}: could not serialize detail: {error}"));
                return SidereonStatus::Panic;
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out_payload",
            &payload,
            out_payload,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Build a one-epoch satellite/station look-angle grid. Delegates to
/// sidereon_core::astro::coverage::look_angles_batch.
///
/// Safety: tles points to tle_count live SidereonTle handles; stations points
/// to station_count SidereonGroundStation values; out_grid points to handle
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_look_angles(
    tles: *const *const SidereonTle,
    tle_count: usize,
    stations: *const SidereonGroundStation,
    station_count: usize,
    epoch_unix_us: i64,
    out_grid: *mut *mut SidereonCoverageGrid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_look_angles",
        SidereonStatus::Panic,
        || {
            let out_grid = c_try!(require_out(
                out_grid,
                "sidereon_coverage_look_angles",
                "out_grid"
            ));
            *out_grid = ptr::null_mut();
            let raw_tles = c_try!(require_slice(
                tles,
                tle_count,
                "sidereon_coverage_look_angles",
                "tles"
            ));
            let raw_stations = c_try!(require_slice(
                stations,
                station_count,
                "sidereon_coverage_look_angles",
                "stations"
            ));
            let mut satellites = Vec::with_capacity(raw_tles.len());
            for (idx, &tle_ptr) in raw_tles.iter().enumerate() {
                let Some(tle) = tle_ptr.as_ref() else {
                    set_last_error(format!("sidereon_coverage_look_angles: null tles[{idx}]"));
                    return SidereonStatus::NullPointer;
                };
                satellites.push(tle.satellite.clone());
            }
            let stations: Vec<GroundStation> =
                raw_stations.iter().map(ground_station_from_c).collect();
            let inner = coverage_look_angles_batch(
                &satellites,
                &stations,
                UtcInstant::from_unix_microseconds(epoch_unix_us),
            );
            write_boxed_handle(
                out_grid,
                SidereonCoverageGrid {
                    inner,
                    station_count,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Read the satellite and station counts for a coverage grid.
///
/// Safety: grid must be a live handle; out_sat_count and out_station_count must
/// point to disjoint size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_dimensions(
    grid: *const SidereonCoverageGrid,
    out_sat_count: *mut usize,
    out_station_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_grid_dimensions",
        SidereonStatus::Panic,
        || {
            if !out_sat_count.is_null() && !out_station_count.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_coverage_grid_dimensions",
                            out_sat_count,
                            1,
                            "out_sat_count"
                        )),
                        "out_sat_count",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_coverage_grid_dimensions",
                            out_station_count,
                            1,
                            "out_station_count"
                        )),
                        "out_station_count",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_coverage_grid_dimensions",
                    &outputs
                ));
            }
            let out_sat_count = c_try!(require_out(
                out_sat_count,
                "sidereon_coverage_grid_dimensions",
                "out_sat_count"
            ));
            let out_station_count = c_try!(require_out(
                out_station_count,
                "sidereon_coverage_grid_dimensions",
                "out_station_count"
            ));
            *out_sat_count = 0;
            *out_station_count = 0;
            let grid = c_try!(require_ref(
                grid,
                "sidereon_coverage_grid_dimensions",
                "grid"
            ));
            *out_sat_count = grid.inner.len();
            *out_station_count = grid.station_count;
            SidereonStatus::Ok
        },
    )
}

/// Read one coverage grid cell. A core look-angle error is returned as ok=false.
///
/// Safety: grid must be a live handle; out must point to
/// SidereonCoverageLookAngle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_look_angle(
    grid: *const SidereonCoverageGrid,
    sat_index: usize,
    station_index: usize,
    out: *mut SidereonCoverageLookAngle,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_grid_look_angle",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_coverage_grid_look_angle", "out"));
            *out = SidereonCoverageLookAngle {
                ok: false,
                azimuth_deg: 0.0,
                elevation_deg: 0.0,
                range_km: 0.0,
            };
            let grid = c_try!(require_ref(
                grid,
                "sidereon_coverage_grid_look_angle",
                "grid"
            ));
            let Some(row) = grid.inner.get(sat_index) else {
                set_last_error(format!(
                    "sidereon_coverage_grid_look_angle: sat_index {sat_index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(cell) = row.get(station_index) else {
                set_last_error(format!(
                    "sidereon_coverage_grid_look_angle: station_index {station_index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            if let Ok(look) = cell {
                *out = SidereonCoverageLookAngle {
                    ok: true,
                    azimuth_deg: look.azimuth_deg,
                    elevation_deg: look.elevation_deg,
                    range_km: look.range_km,
                };
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy the flattened visibility mask. Delegates to
/// sidereon_core::astro::coverage::visible_mask.
///
/// Safety: grid must be a live handle; out must point to len bool entries or be
/// NULL when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_visible_mask(
    grid: *const SidereonCoverageGrid,
    min_elevation_deg: f64,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_grid_visible_mask",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_coverage_grid_visible_mask",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_coverage_grid_visible_mask",
                "grid"
            ));
            let mask = coverage_visible_mask(&grid.inner, min_elevation_deg);
            let values: Vec<bool> = mask.into_iter().flatten().collect();
            c_try!(copy_prefix_to_c(
                "sidereon_coverage_grid_visible_mask",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-station access counts. Delegates to
/// sidereon_core::astro::coverage::access_counts.
///
/// Safety: grid must be a live handle; out must point to len size_t entries or
/// be NULL when len is 0; out_written and out_required must point to size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_access_counts(
    grid: *const SidereonCoverageGrid,
    min_elevation_deg: f64,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_grid_access_counts",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_coverage_grid_access_counts",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_coverage_grid_access_counts",
                "grid"
            ));
            let counts = coverage_access_counts(&grid.inner, min_elevation_deg);
            c_try!(copy_prefix_to_c(
                "sidereon_coverage_grid_access_counts",
                "out",
                &counts,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the per-station maximum successful elevation. Delegates to
/// sidereon_core::astro::coverage::max_elevation. Stations without a successful
/// cell are copied as NaN.
///
/// Safety: grid must be a live handle; out must point to len double entries or
/// be NULL when len is 0; out_written and out_required must point to size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_max_elevation_deg(
    grid: *const SidereonCoverageGrid,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_coverage_grid_max_elevation_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_coverage_grid_max_elevation_deg",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_coverage_grid_max_elevation_deg",
                "grid"
            ));
            let values: Vec<f64> = coverage_max_elevation(&grid.inner)
                .into_iter()
                .map(|value| value.unwrap_or(f64::NAN))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_coverage_grid_max_elevation_deg",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a coverage grid handle. Passing NULL is a no-op.
///
/// Safety: grid must be NULL or a live handle from
/// sidereon_coverage_look_angles that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_coverage_grid_free(grid: *mut SidereonCoverageGrid) {
    ffi_boundary("sidereon_coverage_grid_free", (), || {
        free_boxed(grid);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_cell_error_payload_is_complete_and_owned_by_grid() {
        let grid = SidereonCoverageGrid {
            inner: vec![vec![Err(LookAngleError::InvalidInput {
                field: "ground_station.latitude_deg",
                reason: "out of range",
            })]],
            station_count: 1,
        };
        let (mut written, mut required) = (0, 0);
        let status = unsafe {
            sidereon_coverage_grid_look_angle_error_payload(
                &grid,
                0,
                0,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, 0);
        assert!(required > 0);

        let mut bytes = vec![0; required];
        let status = unsafe {
            sidereon_coverage_grid_look_angle_error_payload(
                &grid,
                0,
                0,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, bytes.len());
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["kind"], "invalid_input");
        assert_eq!(payload["fields"]["field"], "ground_station.latitude_deg");
        assert_eq!(payload["fields"]["reason"], "out of range");
    }

    #[test]
    fn coverage_cell_error_mapper_retains_nested_ut1_refusal() {
        let error = LookAngleError::FrameTransform(
            sidereon_core::astro::frames::transforms::FrameTransformError::Ut1OutsideCoverage {
                reason: sidereon_core::astro::time::DegradeReason::BeforeCoverage,
            },
        );
        let payload = crate::tle::look_angle_error_value(&error);
        assert_eq!(payload["kind"], "frame_transform");
        assert_eq!(payload["fields"]["cause"]["kind"], "ut1_outside_coverage");
        assert_eq!(
            payload["fields"]["cause"]["fields"]["reason"],
            "before_coverage"
        );
    }

    #[test]
    fn public_coverage_api_preserves_real_station_and_ut1_errors_after_free() {
        let l1 = b"1 25544U 98067A   18184.80969102  .00001614  00000-0  31745-4 0  9993\0";
        let l2 = b"2 25544  51.6414 295.8524 0003435 262.6267 204.2868 15.54005638121106\0";
        let (mut tle, mut grid) = (ptr::null_mut(), ptr::null_mut());
        assert_eq!(
            unsafe { sidereon_tle_load(l1.as_ptr().cast(), l2.as_ptr().cast(), 0, &mut tle) },
            SidereonStatus::Ok
        );
        let tles = [tle as *const SidereonTle];
        let stations = [
            SidereonGroundStation {
                latitude_deg: 51.5074,
                longitude_deg: -0.1278,
                altitude_m: 80.0,
            },
            SidereonGroundStation {
                latitude_deg: 91.0,
                longitude_deg: 0.0,
                altitude_m: 0.0,
            },
        ];
        let epoch = 1_530_619_200_000_000i64;
        assert_eq!(
            unsafe {
                sidereon_coverage_look_angles(
                    tles.as_ptr(),
                    1,
                    stations.as_ptr(),
                    2,
                    epoch,
                    &mut grid,
                )
            },
            SidereonStatus::Ok
        );
        let (mut valid, mut invalid) = (
            std::mem::MaybeUninit::uninit(),
            std::mem::MaybeUninit::uninit(),
        );
        assert_eq!(
            unsafe { sidereon_coverage_grid_look_angle(grid, 0, 0, valid.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_coverage_grid_look_angle(grid, 0, 1, invalid.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert!(
            unsafe { valid.assume_init() }.ok,
            "valid station cell remains successful"
        );
        assert!(!unsafe { invalid.assume_init() }.ok);
        let mut required = 0;
        let mut written = 0;
        assert_eq!(
            unsafe {
                sidereon_coverage_grid_look_angle_error_payload(
                    grid,
                    0,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let mut bytes = vec![0; required];
        assert_eq!(
            unsafe {
                sidereon_coverage_grid_look_angle_error_payload(
                    grid,
                    0,
                    1,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, bytes.len());
        unsafe { sidereon_coverage_grid_free(grid) };
        let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload["kind"], "invalid_input");
        assert_eq!(payload["fields"]["field"], "ground_station.latitude_deg");

        let mut strict_grid = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_coverage_look_angles(
                    tles.as_ptr(),
                    1,
                    stations.as_ptr(),
                    1,
                    -2_208_988_800_000_000,
                    &mut strict_grid,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_coverage_grid_look_angle(strict_grid, 0, 0, valid.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        assert!(!unsafe { valid.assume_init() }.ok);
        let (mut required, mut written) = (0, 0);
        assert_eq!(
            unsafe {
                sidereon_coverage_grid_look_angle_error_payload(
                    strict_grid,
                    0,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let mut strict_bytes = vec![0; required];
        assert_eq!(
            unsafe {
                sidereon_coverage_grid_look_angle_error_payload(
                    strict_grid,
                    0,
                    0,
                    strict_bytes.as_mut_ptr(),
                    strict_bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        unsafe { sidereon_coverage_grid_free(strict_grid) };
        unsafe { sidereon_tle_free(tle) };
        let payload: serde_json::Value = serde_json::from_slice(&strict_bytes).unwrap();
        assert_eq!(payload["kind"], "frame_transform");
        assert_eq!(payload["fields"]["cause"]["kind"], "ut1_outside_coverage");
        assert_eq!(
            payload["fields"]["cause"]["fields"]["reason"],
            "before_coverage"
        );
    }
}
