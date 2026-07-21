use super::*;

// --- DTED terrain lookup (sidereon_core::terrain) ---------------------------

/// DTED terrain cache rooted at a tile directory. Create with
/// sidereon_dted_terrain_new and release with sidereon_dted_terrain_free.
pub struct SidereonDtedTerrain {
    pub(crate) inner: DtedTerrain,
}

/// A loaded DTED tile. Create with sidereon_dted_tile_load and release with
/// sidereon_dted_tile_free.
pub struct SidereonDtedTile {
    pub(crate) inner: DtedTile,
}

/// DTED interpolation mode for orthometric terrain heights.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonDtedInterpolation {
    /// Nearest posting height.
    NearestPosting = 0,
    /// Bilinear interpolation across postings.
    Bilinear = 1,
}

/// Options for DTED lookup. Heights are orthometric meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDtedLookupOptions {
    /// Interpolation selector as SidereonDtedInterpolation.
    pub interpolation: u32,
}

/// One DTED terrain batch result. When has_height_m is true, height_m is an
/// orthometric height in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDtedHeightResult {
    /// Per-point status.
    pub status: SidereonStatus,
    /// Whether height_m carries a valid orthometric height.
    pub has_height_m: bool,
    /// Orthometric height, meters, when has_height_m is true.
    pub height_m: f64,
}

/// Copy a DTED interpolation label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_interpolation_label(
    interpolation: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_interpolation_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_dted_interpolation_label",
                out_written,
                out_required
            ));
            let label = c_try!(dted_interpolation_label_from_c(
                "sidereon_dted_interpolation_label",
                interpolation
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_dted_interpolation_label",
                "out",
                label.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Initialize DTED lookup options to bilinear interpolation. Heights returned by
/// DTED lookup functions are orthometric meters.
///
/// Safety: out_options must point to a SidereonDtedLookupOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_lookup_options_init(
    out_options: *mut SidereonDtedLookupOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_lookup_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_dted_lookup_options_init",
                "out_options"
            ));
            let options = DtedLookupOptions::default();
            *out = SidereonDtedLookupOptions {
                interpolation: match options.interpolation {
                    DtedInterpolation::NearestPosting => {
                        SidereonDtedInterpolation::NearestPosting as u32
                    }
                    DtedInterpolation::Bilinear => SidereonDtedInterpolation::Bilinear as u32,
                },
            };
            SidereonStatus::Ok
        },
    )
}

/// Create a DTED terrain cache rooted at `root`. Heights returned by this handle
/// are orthometric meters.
///
/// Safety: root must be a non-empty UTF-8 C string; out_terrain must point to a
/// SidereonDtedTerrain*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_terrain_new(
    root: *const c_char,
    out_terrain: *mut *mut SidereonDtedTerrain,
) -> SidereonStatus {
    ffi_boundary("sidereon_dted_terrain_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_terrain,
            "sidereon_dted_terrain_new",
            "out_terrain"
        ));
        *out = ptr::null_mut();
        let root = c_try!(parse_c_string("sidereon_dted_terrain_new", "root", root));
        write_boxed_handle(
            out,
            SidereonDtedTerrain {
                inner: DtedTerrain::new(root),
            },
        );
        SidereonStatus::Ok
    })
}

/// Query one terrain height. Inputs are longitude, latitude in degrees. The
/// returned height is orthometric meters.
///
/// Safety: terrain must be a live handle; out_height_m must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_terrain_height_m(
    terrain: *mut SidereonDtedTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_terrain_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_dted_terrain_height_m",
                "out_height_m"
            ));
            *out = 0.0;
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_dted_terrain_height_m",
                "terrain"
            ));
            match terrain.inner.height_m(longitude_deg, latitude_deg) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_dted_core_error("sidereon_dted_terrain_height_m", err),
            }
        },
    )
}

/// Query one terrain height with interpolation options. Inputs are longitude,
/// latitude in degrees. The returned height is orthometric meters.
///
/// Safety: terrain must be a live handle; options must point to a
/// SidereonDtedLookupOptions; out_height_m must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_terrain_height_m_with_options(
    terrain: *mut SidereonDtedTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    options: *const SidereonDtedLookupOptions,
    out_height_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_terrain_height_m_with_options",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_dted_terrain_height_m_with_options",
                "out_height_m"
            ));
            *out = 0.0;
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_dted_terrain_height_m_with_options",
                "terrain"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_dted_terrain_height_m_with_options",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_dted_terrain_height_m_with_options",
                options
            ));
            match terrain
                .inner
                .height_m_with_options(longitude_deg, latitude_deg, options)
            {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_dted_core_error("sidereon_dted_terrain_height_m_with_options", err),
            }
        },
    )
}

/// Query many terrain points using the same mutable DTED tile cache. Points are
/// longitude-first `(lon_deg, lat_deg)` pairs. Each successful result carries an
/// orthometric height in meters. Per-point lookup failures are written into
/// `out[i].status` and do not fail the whole call.
///
/// Safety: terrain must be a live handle; points points to count
/// SidereonLonLatDeg values; options must point to SidereonDtedLookupOptions;
/// out points to count writable SidereonDtedHeightResult entries, or NULL when
/// count is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_terrain_height_batch_m(
    terrain: *mut SidereonDtedTerrain,
    points: *const SidereonLonLatDeg,
    count: usize,
    options: *const SidereonDtedLookupOptions,
    out: *mut SidereonDtedHeightResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_terrain_height_batch_m",
        SidereonStatus::Panic,
        || {
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_dted_terrain_height_batch_m",
                "terrain"
            ));
            let raw_points = c_try!(require_slice(
                points,
                count,
                "sidereon_dted_terrain_height_batch_m",
                "points"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_dted_terrain_height_batch_m",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_dted_terrain_height_batch_m",
                options
            ));
            if count > 0 && out.is_null() {
                set_last_error("sidereon_dted_terrain_height_batch_m: null out");
                return SidereonStatus::NullPointer;
            }
            c_try!(validate_element_count::<SidereonDtedHeightResult>(
                "sidereon_dted_terrain_height_batch_m",
                "out",
                count
            ));
            for idx in 0..count {
                out.add(idx).write(SidereonDtedHeightResult {
                    status: SidereonStatus::InvalidArgument,
                    has_height_m: false,
                    height_m: 0.0,
                });
            }
            let points: Vec<(f64, f64)> = raw_points
                .iter()
                .map(|point| (point.lon_deg, point.lat_deg))
                .collect();
            let results = terrain.inner.height_batch(&points, options);
            for (idx, result) in results.into_iter().enumerate() {
                out.add(idx).write(dted_height_result_from_core(result));
            }
            SidereonStatus::Ok
        },
    )
}

/// Release a DTED terrain cache. Passing NULL is a no-op.
///
/// Safety: terrain must be NULL or a live handle from sidereon_dted_terrain_new.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_terrain_free(terrain: *mut SidereonDtedTerrain) {
    free_boxed(terrain);
}

/// Load one DTED tile. Tile heights are orthometric meters.
///
/// Safety: path must be a non-empty UTF-8 C string; out_tile must point to a
/// SidereonDtedTile*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tile_load(
    path: *const c_char,
    out_tile: *mut *mut SidereonDtedTile,
) -> SidereonStatus {
    ffi_boundary("sidereon_dted_tile_load", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_tile, "sidereon_dted_tile_load", "out_tile"));
        *out = ptr::null_mut();
        let path = c_try!(parse_c_string("sidereon_dted_tile_load", "path", path));
        match DtedTile::from_path(path) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonDtedTile { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_dted_string_error("sidereon_dted_tile_load", err),
        }
    })
}

/// Query the nearest stored posting in a loaded DTED tile. Inputs are longitude,
/// latitude in degrees. The returned integer elevation is an orthometric height
/// in meters.
///
/// Safety: tile must be a live handle; out_elevation_m must point to an int16_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tile_get_elevation(
    tile: *const SidereonDtedTile,
    longitude_deg: f64,
    latitude_deg: f64,
    out_elevation_m: *mut i16,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_tile_get_elevation",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_elevation_m,
                "sidereon_dted_tile_get_elevation",
                "out_elevation_m"
            ));
            *out = 0;
            let tile = c_try!(require_ref(
                tile,
                "sidereon_dted_tile_get_elevation",
                "tile"
            ));
            match tile.inner.get_elevation(longitude_deg, latitude_deg) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_dted_string_error("sidereon_dted_tile_get_elevation", err),
            }
        },
    )
}

/// Release a DTED tile. Passing NULL is a no-op.
///
/// Safety: tile must be NULL or a live handle from sidereon_dted_tile_load.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tile_free(tile: *mut SidereonDtedTile) {
    free_boxed(tile);
}

/// Convert a DTED tile tree rooted at root into memory-mappable terrain store
/// bytes. The output uses the variable-length output contract. Store postings
/// are orthometric heights in metres.
///
/// Safety: root must be a non-empty UTF-8 C string; out must point to len bytes
/// or be NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tree_to_mmap_store(
    root: *const c_char,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_tree_to_mmap_store",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_dted_tree_to_mmap_store",
                out_written,
                out_required
            ));
            let root = c_try!(parse_c_string(
                "sidereon_dted_tree_to_mmap_store",
                "root",
                root
            ));
            let bytes = match core_dted_tree_to_mmap_store(std::path::Path::new(&root)) {
                Ok(bytes) => bytes,
                Err(err) => {
                    return map_terrain_store_error("sidereon_dted_tree_to_mmap_store", err)
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_dted_tree_to_mmap_store",
                "out",
                &bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Convert a DTED tile tree and write memory-mappable terrain store bytes to
/// out_path. Store postings are orthometric heights in metres.
///
/// Safety: root and out_path must be non-empty UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn sidereon_write_dted_tree_to_mmap_store(
    root: *const c_char,
    out_path: *const c_char,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_write_dted_tree_to_mmap_store",
        SidereonStatus::Panic,
        || {
            let root = c_try!(parse_c_string(
                "sidereon_write_dted_tree_to_mmap_store",
                "root",
                root
            ));
            let out_path = c_try!(parse_c_string(
                "sidereon_write_dted_tree_to_mmap_store",
                "out_path",
                out_path
            ));
            match core_write_dted_tree_to_mmap_store(
                std::path::Path::new(&root),
                std::path::Path::new(&out_path),
            ) {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_terrain_store_error("sidereon_write_dted_tree_to_mmap_store", err),
            }
        },
    )
}

/// One longitude-first terrain lookup point, in degrees.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLonLatDeg {
    /// Longitude, degrees.
    pub lon_deg: f64,
    /// Latitude, degrees.
    pub lat_deg: f64,
}

fn dted_interpolation_label_from_c(
    fn_name: &str,
    interpolation: u32,
) -> Result<&'static str, SidereonStatus> {
    match interpolation {
        value if value == SidereonDtedInterpolation::NearestPosting as u32 => {
            Ok("DtedInterpolation.NEAREST_POSTING")
        }
        value if value == SidereonDtedInterpolation::Bilinear as u32 => {
            Ok("DtedInterpolation.BILINEAR")
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid DTED interpolation"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn map_dted_string_error(fn_name: &str, err: String) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_dted_core_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn dted_height_result_from_core(result: sidereon_core::Result<f64>) -> SidereonDtedHeightResult {
    match result {
        Ok(height_m) => SidereonDtedHeightResult {
            status: SidereonStatus::Ok,
            has_height_m: true,
            height_m,
        },
        Err(_) => SidereonDtedHeightResult {
            status: SidereonStatus::InvalidArgument,
            has_height_m: false,
            height_m: 0.0,
        },
    }
}
