use super::*;

// --- Geoid undulation / orthometric height (sidereon_core::geoid) -------------

/// A loaded geoid undulation grid. Opaque to C. Create with
/// sidereon_geoid_grid_from_text or sidereon_geoid_grid_new; release with
/// sidereon_geoid_grid_free.
pub struct SidereonGeoidGrid {
    pub(crate) inner: GeoidGrid,
}

/// Geoid undulation N (metres above the WGS84 ellipsoid) at a geodetic position
/// in radians, from the coarse built-in global grid. Delegates to
/// sidereon_core::geoid::geoid_undulation.
///
/// Safety: out_undulation_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_undulation(
    lat_rad: f64,
    lon_rad: f64,
    out_undulation_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_geoid_undulation", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_undulation_m,
            "sidereon_geoid_undulation",
            "out_undulation_m"
        ));
        *out = geoid_undulation(lat_rad, lon_rad);
        SidereonStatus::Ok
    })
}

/// Orthometric height H = h - N (metres above mean sea level) from an
/// ellipsoidal height and a geodetic position in radians, using the built-in
/// grid. Delegates to sidereon_core::geoid::orthometric_height_m.
///
/// Safety: out_height_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orthometric_height_m(
    ellipsoidal_height_m_in: f64,
    lat_rad: f64,
    lon_rad: f64,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_orthometric_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_orthometric_height_m",
                "out_height_m"
            ));
            *out = orthometric_height_m(ellipsoidal_height_m_in, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

/// Ellipsoidal height h = H + N (metres above the WGS84 ellipsoid) from an
/// orthometric height and a geodetic position in radians, using the built-in
/// grid. Delegates to sidereon_core::geoid::ellipsoidal_height_m.
///
/// Safety: out_height_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ellipsoidal_height_m(
    orthometric_height_m_in: f64,
    lat_rad: f64,
    lon_rad: f64,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ellipsoidal_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_ellipsoidal_height_m",
                "out_height_m"
            ));
            *out = ellipsoidal_height_m(orthometric_height_m_in, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

/// Parse a geoid grid from the documented whitespace text format. On success
/// writes a newly owned handle to *out_grid. Delegates to
/// sidereon_core::geoid::GeoidGrid::from_text.
///
/// Safety: text points to len readable bytes; out_grid points to a
/// SidereonGeoidGrid*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_from_text(
    text: *const u8,
    len: usize,
    out_grid: *mut *mut SidereonGeoidGrid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_from_text",
        SidereonStatus::Panic,
        || {
            let out_grid = c_try!(require_out(
                out_grid,
                "sidereon_geoid_grid_from_text",
                "out_grid"
            ));
            *out_grid = ptr::null_mut();
            let bytes = c_try!(require_slice(
                text,
                len,
                "sidereon_geoid_grid_from_text",
                "text"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => {
                    set_last_error(
                        "sidereon_geoid_grid_from_text: text is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            match GeoidGrid::from_text(text) {
                Ok(inner) => {
                    write_boxed_handle(out_grid, SidereonGeoidGrid { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_geoid_error("sidereon_geoid_grid_from_text", err),
            }
        },
    )
}

/// Build a geoid grid from its origin, spacing, dimensions, and row-major
/// samples (metres, latitude ascending outer, longitude ascending inner). On
/// success writes a newly owned handle to *out_grid. Delegates to
/// sidereon_core::geoid::GeoidGrid::new.
///
/// Safety: values_m points to value_count readable doubles (must equal
/// n_lat * n_lon); out_grid points to a SidereonGeoidGrid*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_new(
    lat_min_deg: f64,
    lon_min_deg: f64,
    dlat_deg: f64,
    dlon_deg: f64,
    n_lat: usize,
    n_lon: usize,
    values_m: *const f64,
    value_count: usize,
    out_grid: *mut *mut SidereonGeoidGrid,
) -> SidereonStatus {
    ffi_boundary("sidereon_geoid_grid_new", SidereonStatus::Panic, || {
        let out_grid = c_try!(require_out(out_grid, "sidereon_geoid_grid_new", "out_grid"));
        *out_grid = ptr::null_mut();
        let values = c_try!(require_slice(
            values_m,
            value_count,
            "sidereon_geoid_grid_new",
            "values_m"
        ));
        match GeoidGrid::new(
            lat_min_deg,
            lon_min_deg,
            dlat_deg,
            dlon_deg,
            n_lat,
            n_lon,
            values.to_vec(),
        ) {
            Ok(inner) => {
                write_boxed_handle(out_grid, SidereonGeoidGrid { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_geoid_error("sidereon_geoid_grid_new", err),
        }
    })
}

/// Bilinearly interpolated undulation N (metres) at a geodetic position in
/// degrees. Delegates to sidereon_core::geoid::GeoidGrid::undulation_deg.
///
/// Safety: grid is a live handle; out_undulation_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_undulation_deg(
    grid: *const SidereonGeoidGrid,
    lat_deg: f64,
    lon_deg: f64,
    out_undulation_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_undulation_deg",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_undulation_m,
                "sidereon_geoid_grid_undulation_deg",
                "out_undulation_m"
            ));
            *out = 0.0;
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_undulation_deg",
                "grid"
            ));
            *out = grid.inner.undulation_deg(lat_deg, lon_deg);
            SidereonStatus::Ok
        },
    )
}

/// Bilinearly interpolated undulation N (metres) at a geodetic position in
/// radians. Delegates to sidereon_core::geoid::GeoidGrid::undulation_rad.
///
/// Safety: grid is a live handle; out_undulation_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_undulation_rad(
    grid: *const SidereonGeoidGrid,
    lat_rad: f64,
    lon_rad: f64,
    out_undulation_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_undulation_rad",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_undulation_m,
                "sidereon_geoid_grid_undulation_rad",
                "out_undulation_m"
            ));
            *out = 0.0;
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_undulation_rad",
                "grid"
            ));
            *out = grid.inner.undulation_rad(lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

/// Release a geoid grid handle. Passing NULL is a no-op.
///
/// Safety: grid must be a handle from a sidereon_geoid_grid_* constructor or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_free(grid: *mut SidereonGeoidGrid) {
    free_boxed(grid);
}

/// Loaded EGM96 15-arcminute geoid grid for terrain datum conversion. Create
/// with sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes or
/// sidereon_egm96_15m_geoid_from_ww15mgh_dac_path, and release with
/// sidereon_egm96_15m_geoid_free.
pub struct SidereonEgm96FifteenMinuteGeoid {
    pub(crate) inner: CoreEgm96FifteenMinuteGeoid,
}

/// Load WW15MGH.DAC bytes as an EGM96 15-arcminute geoid grid. This function
/// does not fall back to the embedded 1-degree grid.
///
/// Safety: bytes must point to len readable bytes; out_geoid must point to a
/// SidereonEgm96FifteenMinuteGeoid*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes(
    bytes: *const u8,
    len: usize,
    out_geoid: *mut *mut SidereonEgm96FifteenMinuteGeoid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_geoid,
                "sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes",
                "out_geoid"
            ));
            *out = ptr::null_mut();
            let bytes = c_try!(require_slice(
                bytes,
                len,
                "sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes",
                "bytes"
            ));
            match CoreEgm96FifteenMinuteGeoid::from_ww15mgh_dac_bytes(bytes) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonEgm96FifteenMinuteGeoid { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_datum_error("sidereon_egm96_15m_geoid_from_ww15mgh_dac_bytes", err)
                }
            }
        },
    )
}

/// Read and load WW15MGH.DAC as an EGM96 15-arcminute geoid grid. A missing
/// file returns SidereonTerrainDatumErrorKind::MissingEgm96Dac through
/// sidereon_last_terrain_datum_error and does not fall back to the embedded
/// 1-degree grid.
///
/// Safety: path must be a non-empty UTF-8 C string; out_geoid must point to a
/// SidereonEgm96FifteenMinuteGeoid*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_15m_geoid_from_ww15mgh_dac_path(
    path: *const c_char,
    out_geoid: *mut *mut SidereonEgm96FifteenMinuteGeoid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_15m_geoid_from_ww15mgh_dac_path",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_geoid,
                "sidereon_egm96_15m_geoid_from_ww15mgh_dac_path",
                "out_geoid"
            ));
            *out = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_egm96_15m_geoid_from_ww15mgh_dac_path",
                "path",
                path
            ));
            match CoreEgm96FifteenMinuteGeoid::from_ww15mgh_dac_path(std::path::Path::new(&path)) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonEgm96FifteenMinuteGeoid { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_datum_error("sidereon_egm96_15m_geoid_from_ww15mgh_dac_path", err)
                }
            }
        },
    )
}

/// Release an EGM96 15-arcminute geoid grid handle. Passing NULL is a no-op.
///
/// Safety: geoid must be NULL or a live handle from sidereon_egm96_15m_geoid_*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_15m_geoid_free(
    geoid: *mut SidereonEgm96FifteenMinuteGeoid,
) {
    free_boxed(geoid);
}

// --- Embedded EGM96 geoid ---------------------------------------------------

/// Geoid undulation `N` (meters above the WGS84 ellipsoid) at a geodetic
/// position in radians, from the embedded genuine EGM96 1-degree grid, written
/// to *out. Latitude positive north, longitude positive east. Delegates to the
/// core `egm96_undulation`.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_undulation(
    lat_rad: f64,
    lon_rad: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_egm96_undulation", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_egm96_undulation", "out"));
        *out = core_egm96_undulation(lat_rad, lon_rad);
        SidereonStatus::Ok
    })
}

/// Orthometric height `H = h - N` (meters above mean sea level) from an
/// ellipsoidal height and a geodetic position in radians, using the embedded
/// genuine EGM96 model, written to *out. Delegates to the core
/// `egm96_orthometric_height_m`.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_orthometric_height_m(
    ellipsoidal_height_m: f64,
    lat_rad: f64,
    lon_rad: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_orthometric_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_egm96_orthometric_height_m",
                "out"
            ));
            *out = core_egm96_orthometric_height_m(ellipsoidal_height_m, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

/// Ellipsoidal height `h = H + N` (meters above the WGS84 ellipsoid) from an
/// orthometric height and a geodetic position in radians, using the embedded
/// genuine EGM96 model, written to *out. Delegates to the core
/// `egm96_ellipsoidal_height_m`.
///
/// Safety: out must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_ellipsoidal_height_m(
    orthometric_height_m: f64,
    lat_rad: f64,
    lon_rad: f64,
    out: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_ellipsoidal_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_egm96_ellipsoidal_height_m",
                "out"
            ));
            *out = core_egm96_ellipsoidal_height_m(orthometric_height_m, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

// === Round-2 geoid batches and grid conversions =============================

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeoidPoint {
    pub latitude: f64,
    pub longitude: f64,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_undulations_rad(
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_undulations_rad",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geoid_undulations_rad",
                out_written,
                out_required
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_geoid_undulations_rad",
                points,
                point_count
            ));
            let values = sidereon_core::geoid::geoid_undulations_rad(&points);
            copy_geoid_values(
                "sidereon_geoid_undulations_rad",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_undulations_deg(
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_undulations_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geoid_undulations_deg",
                out_written,
                out_required
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_geoid_undulations_deg",
                points,
                point_count
            ));
            let values = sidereon_core::geoid::geoid_undulations_deg(&points);
            copy_geoid_values(
                "sidereon_geoid_undulations_deg",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_undulations_rad(
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_undulations_rad",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_egm96_undulations_rad",
                out_written,
                out_required
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_egm96_undulations_rad",
                points,
                point_count
            ));
            let values = sidereon_core::geoid::egm96_undulations_rad(&points);
            copy_geoid_values(
                "sidereon_egm96_undulations_rad",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_egm96_undulations_deg(
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_egm96_undulations_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_egm96_undulations_deg",
                out_written,
                out_required
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_egm96_undulations_deg",
                points,
                point_count
            ));
            let values = sidereon_core::geoid::egm96_undulations_deg(&points);
            copy_geoid_values(
                "sidereon_egm96_undulations_deg",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_from_egm96_dac(
    data: *const u8,
    len: usize,
    out_grid: *mut *mut SidereonGeoidGrid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_from_egm96_dac",
        SidereonStatus::Panic,
        || {
            let out_grid = c_try!(require_out(
                out_grid,
                "sidereon_geoid_grid_from_egm96_dac",
                "out_grid"
            ));
            *out_grid = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_geoid_grid_from_egm96_dac",
                "data"
            ));
            match GeoidGrid::from_egm96_dac(bytes) {
                Ok(inner) => {
                    write_boxed_handle(out_grid, SidereonGeoidGrid { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_geoid_error("sidereon_geoid_grid_from_egm96_dac", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_undulations_rad(
    grid: *const SidereonGeoidGrid,
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_undulations_rad",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geoid_grid_undulations_rad",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_undulations_rad",
                "grid"
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_geoid_grid_undulations_rad",
                points,
                point_count
            ));
            let values = grid.inner.undulations_rad(&points);
            copy_geoid_values(
                "sidereon_geoid_grid_undulations_rad",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_undulations_deg(
    grid: *const SidereonGeoidGrid,
    points: *const SidereonGeoidPoint,
    point_count: usize,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_undulations_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_geoid_grid_undulations_deg",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_undulations_deg",
                "grid"
            ));
            let points = c_try!(geoid_points_from_c(
                "sidereon_geoid_grid_undulations_deg",
                points,
                point_count
            ));
            let values = grid.inner.undulations_deg(&points);
            copy_geoid_values(
                "sidereon_geoid_grid_undulations_deg",
                &values,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_orthometric_height_rad(
    grid: *const SidereonGeoidGrid,
    ellipsoidal_height_m: f64,
    lat_rad: f64,
    lon_rad: f64,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_orthometric_height_rad",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_geoid_grid_orthometric_height_rad",
                "out_height_m"
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_orthometric_height_rad",
                "grid"
            ));
            *out = grid
                .inner
                .orthometric_height_rad(ellipsoidal_height_m, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_geoid_grid_ellipsoidal_height_rad(
    grid: *const SidereonGeoidGrid,
    orthometric_height_m: f64,
    lat_rad: f64,
    lon_rad: f64,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_geoid_grid_ellipsoidal_height_rad",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_geoid_grid_ellipsoidal_height_rad",
                "out_height_m"
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_geoid_grid_ellipsoidal_height_rad",
                "grid"
            ));
            *out = grid
                .inner
                .ellipsoidal_height_rad(orthometric_height_m, lat_rad, lon_rad);
            SidereonStatus::Ok
        },
    )
}

fn map_geoid_error(fn_name: &str, err: GeoidError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn geoid_points_from_c(
    fn_name: &str,
    points: *const SidereonGeoidPoint,
    point_count: usize,
) -> Result<Vec<(f64, f64)>, SidereonStatus> {
    let points = unsafe { require_slice(points, point_count, fn_name, "points") }?;
    Ok(points
        .iter()
        .map(|point| (point.latitude, point.longitude))
        .collect())
}

unsafe fn copy_geoid_values(
    fn_name: &str,
    values: &[f64],
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        values,
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}
