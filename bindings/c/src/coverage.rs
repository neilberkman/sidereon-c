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
/// point to size_t storage.
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
