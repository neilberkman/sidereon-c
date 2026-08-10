use super::*;

/// Memory-mappable terrain reader backed by terrain store bytes. Create with
/// sidereon_mmap_terrain_from_bytes, sidereon_mmap_terrain_from_vec, or
/// sidereon_mmap_terrain_from_path. A trusted caller can defer payload hashing
/// with sidereon_mmap_terrain_from_path_attested. Release the handle with
/// sidereon_mmap_terrain_free. Terrain lookups use longitude, latitude degrees
/// and return orthometric height.
pub struct SidereonMmapTerrain {
    pub(crate) inner: CoreMmapTerrain<'static>,
}

/// Parse memory-mappable terrain store bytes into an owned reader handle. The C
/// binding copies the input byte span into handle-owned storage. Terrain lookup
/// APIs use longitude, latitude degrees and return orthometric height.
///
/// Safety: bytes must point to len readable bytes; out_terrain must point to a
/// SidereonMmapTerrain*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_from_bytes(
    bytes: *const u8,
    len: usize,
    out_terrain: *mut *mut SidereonMmapTerrain,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_from_bytes",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_terrain,
                "sidereon_mmap_terrain_from_bytes",
                "out_terrain"
            ));
            *out = ptr::null_mut();
            let bytes = c_try!(require_slice(
                bytes,
                len,
                "sidereon_mmap_terrain_from_bytes",
                "bytes"
            ));
            match CoreMmapTerrain::from_vec(bytes.to_vec()) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonMmapTerrain { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_store_error("sidereon_mmap_terrain_from_bytes", err),
            }
        },
    )
}

/// Parse memory-mappable terrain store bytes into an owned reader handle. This
/// is the same C ownership contract as sidereon_mmap_terrain_from_bytes.
///
/// Safety: bytes must point to len readable bytes; out_terrain must point to a
/// SidereonMmapTerrain*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_from_vec(
    bytes: *const u8,
    len: usize,
    out_terrain: *mut *mut SidereonMmapTerrain,
) -> SidereonStatus {
    sidereon_mmap_terrain_from_bytes(bytes, len, out_terrain)
}

/// Read and parse a memory-mappable terrain store file. Terrain lookup APIs use
/// longitude, latitude degrees and return orthometric height.
///
/// Safety: path must be a non-empty UTF-8 C string; out_terrain must point to a
/// SidereonMmapTerrain*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_from_path(
    path: *const c_char,
    out_terrain: *mut *mut SidereonMmapTerrain,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_from_path",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_terrain,
                "sidereon_mmap_terrain_from_path",
                "out_terrain"
            ));
            *out = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_mmap_terrain_from_path",
                "path",
                path
            ));
            match CoreMmapTerrain::from_path(std::path::Path::new(&path)) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonMmapTerrain { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_store_error("sidereon_mmap_terrain_from_path", err),
            }
        },
    )
}

/// Open a memory-mappable terrain store file using a caller-attested full-store
/// checksum. Header, index, dimension, length, and tile-bound validation still
/// run, but tile payload hashing is deferred until sidereon_mmap_terrain_verify.
/// The claim is returned by sidereon_mmap_terrain_checksum64 until verification.
///
/// Safety: path must be a non-empty UTF-8 C string; out_terrain must point to a
/// SidereonMmapTerrain*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_from_path_attested(
    path: *const c_char,
    claimed_checksum64: u64,
    out_terrain: *mut *mut SidereonMmapTerrain,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_from_path_attested",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_terrain,
                "sidereon_mmap_terrain_from_path_attested",
                "out_terrain"
            ));
            *out = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_mmap_terrain_from_path_attested",
                "path",
                path
            ));
            match CoreMmapTerrain::from_path_attested(
                std::path::Path::new(&path),
                claimed_checksum64,
            ) {
                Ok(inner) => {
                    write_boxed_handle(out, SidereonMmapTerrain { inner });
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_store_error("sidereon_mmap_terrain_from_path_attested", err)
                }
            }
        },
    )
}

/// Query one bilinear terrain height. Inputs are longitude, latitude degrees.
/// The returned value is orthometric height H in metres.
///
/// Safety: terrain must be a live handle; out_height_m must point to a
/// SidereonOrthometricHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_height_m(
    terrain: *mut SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    out_height_m: *mut SidereonOrthometricHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_height_m",
                "out_height_m"
            ));
            *out = SidereonOrthometricHeightM { value_m: 0.0 };
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_mmap_terrain_height_m",
                "terrain"
            ));
            match terrain.inner.height_m(longitude_deg, latitude_deg) {
                Ok(value_m) => {
                    *out = SidereonOrthometricHeightM { value_m };
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_core_error("sidereon_mmap_terrain_height_m", err),
            }
        },
    )
}

/// Query one terrain height with interpolation options. Inputs are longitude,
/// latitude degrees. The returned value is orthometric height H in metres.
///
/// Safety: terrain must be a live handle; options must point to
/// SidereonDtedLookupOptions; out_height_m must point to a
/// SidereonOrthometricHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_height_m_with_options(
    terrain: *mut SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    options: *const SidereonDtedLookupOptions,
    out_height_m: *mut SidereonOrthometricHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_height_m_with_options",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_height_m_with_options",
                "out_height_m"
            ));
            *out = SidereonOrthometricHeightM { value_m: 0.0 };
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_mmap_terrain_height_m_with_options",
                "terrain"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_height_m_with_options",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_height_m_with_options",
                options
            ));
            match terrain
                .inner
                .height_m_with_options(longitude_deg, latitude_deg, options)
            {
                Ok(value_m) => {
                    *out = SidereonOrthometricHeightM { value_m };
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_core_error("sidereon_mmap_terrain_height_m_with_options", err)
                }
            }
        },
    )
}

/// Query one typed orthometric terrain height. Inputs are longitude, latitude
/// degrees. The returned value is orthometric height H in metres.
///
/// Safety: terrain must be a live handle; out_height_m must point to a
/// SidereonOrthometricHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_orthometric_height_m(
    terrain: *const SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    out_height_m: *mut SidereonOrthometricHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_orthometric_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_orthometric_height_m",
                "out_height_m"
            ));
            *out = SidereonOrthometricHeightM { value_m: 0.0 };
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_orthometric_height_m",
                "terrain"
            ));
            match terrain
                .inner
                .orthometric_height_m(longitude_deg, latitude_deg)
            {
                Ok(value) => {
                    *out = orthometric_height_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_core_error("sidereon_mmap_terrain_orthometric_height_m", err)
                }
            }
        },
    )
}

/// Query one typed orthometric terrain height with interpolation options. Inputs
/// are longitude, latitude degrees. The returned value is orthometric height H
/// in metres.
///
/// Safety: terrain must be a live handle; options must point to
/// SidereonDtedLookupOptions; out_height_m must point to a
/// SidereonOrthometricHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_orthometric_height_m_with_options(
    terrain: *const SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    options: *const SidereonDtedLookupOptions,
    out_height_m: *mut SidereonOrthometricHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_orthometric_height_m_with_options",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_orthometric_height_m_with_options",
                "out_height_m"
            ));
            *out = SidereonOrthometricHeightM { value_m: 0.0 };
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_orthometric_height_m_with_options",
                "terrain"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_orthometric_height_m_with_options",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_orthometric_height_m_with_options",
                options
            ));
            match terrain.inner.orthometric_height_m_with_options(
                longitude_deg,
                latitude_deg,
                options,
            ) {
                Ok(value) => {
                    *out = orthometric_height_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_core_error(
                    "sidereon_mmap_terrain_orthometric_height_m_with_options",
                    err,
                ),
            }
        },
    )
}

/// Query many terrain points as orthometric heights H in metres. Points are
/// longitude, latitude degrees. Per-point failures are written into
/// `out[i].status`.
///
/// Safety: terrain must be a live handle; points must point to count
/// SidereonLonLatDeg values; options must point to SidereonDtedLookupOptions;
/// out must point to count SidereonTerrainHeightResult values or be NULL when
/// count is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_height_batch(
    terrain: *mut SidereonMmapTerrain,
    points: *const SidereonLonLatDeg,
    count: usize,
    options: *const SidereonDtedLookupOptions,
    out: *mut SidereonTerrainHeightResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_height_batch",
        SidereonStatus::Panic,
        || {
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_mmap_terrain_height_batch",
                "terrain"
            ));
            let raw_points = c_try!(require_slice(
                points,
                count,
                "sidereon_mmap_terrain_height_batch",
                "points"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_height_batch",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_height_batch",
                options
            ));
            if count > 0 && out.is_null() {
                set_last_error("sidereon_mmap_terrain_height_batch: null out");
                return SidereonStatus::NullPointer;
            }
            c_try!(validate_element_count::<SidereonTerrainHeightResult>(
                "sidereon_mmap_terrain_height_batch",
                "out",
                count
            ));
            for idx in 0..count {
                out.add(idx).write(SidereonTerrainHeightResult {
                    status: SidereonStatus::InvalidArgument,
                    has_orthometric_height_m: false,
                    orthometric_height_m: SidereonOrthometricHeightM { value_m: 0.0 },
                });
            }
            let points: Vec<(f64, f64)> = raw_points
                .iter()
                .map(|point| (point.lon_deg, point.lat_deg))
                .collect();
            let results = terrain.inner.height_batch(&points, options);
            for (idx, result) in results.into_iter().enumerate() {
                out.add(idx).write(terrain_height_result_from_f64(result));
            }
            SidereonStatus::Ok
        },
    )
}

/// Query many terrain points as typed orthometric heights H in metres. Points
/// are longitude, latitude degrees. Per-point failures are written into
/// `out[i].status`.
///
/// Safety: terrain must be a live handle; points must point to count
/// SidereonLonLatDeg values; options must point to SidereonDtedLookupOptions;
/// out must point to count SidereonTerrainHeightResult values or be NULL when
/// count is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_orthometric_height_batch(
    terrain: *const SidereonMmapTerrain,
    points: *const SidereonLonLatDeg,
    count: usize,
    options: *const SidereonDtedLookupOptions,
    out: *mut SidereonTerrainHeightResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_orthometric_height_batch",
        SidereonStatus::Panic,
        || {
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_orthometric_height_batch",
                "terrain"
            ));
            let raw_points = c_try!(require_slice(
                points,
                count,
                "sidereon_mmap_terrain_orthometric_height_batch",
                "points"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_orthometric_height_batch",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_orthometric_height_batch",
                options
            ));
            if count > 0 && out.is_null() {
                set_last_error("sidereon_mmap_terrain_orthometric_height_batch: null out");
                return SidereonStatus::NullPointer;
            }
            c_try!(validate_element_count::<SidereonTerrainHeightResult>(
                "sidereon_mmap_terrain_orthometric_height_batch",
                "out",
                count
            ));
            for idx in 0..count {
                out.add(idx).write(SidereonTerrainHeightResult {
                    status: SidereonStatus::InvalidArgument,
                    has_orthometric_height_m: false,
                    orthometric_height_m: SidereonOrthometricHeightM { value_m: 0.0 },
                });
            }
            let points: Vec<(f64, f64)> = raw_points
                .iter()
                .map(|point| (point.lon_deg, point.lat_deg))
                .collect();
            let results = terrain.inner.orthometric_height_batch(&points, options);
            for (idx, result) in results.into_iter().enumerate() {
                out.add(idx)
                    .write(terrain_height_result_from_orthometric(result));
            }
            SidereonStatus::Ok
        },
    )
}

/// Query one ellipsoidal terrain height h in metres using the embedded EGM96
/// 1-degree geoid grid for h = H + N. Inputs are longitude, latitude degrees.
///
/// Safety: terrain must be a live handle; out_height_m must point to a
/// SidereonEllipsoidalHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_ellipsoidal_height_m(
    terrain: *const SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    out_height_m: *mut SidereonEllipsoidalHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_ellipsoidal_height_m",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_ellipsoidal_height_m",
                "out_height_m"
            ));
            *out = SidereonEllipsoidalHeightM { value_m: 0.0 };
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_ellipsoidal_height_m",
                "terrain"
            ));
            match terrain
                .inner
                .ellipsoidal_height_m(longitude_deg, latitude_deg)
            {
                Ok(value) => {
                    *out = ellipsoidal_height_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => {
                    map_terrain_datum_error("sidereon_mmap_terrain_ellipsoidal_height_m", err)
                }
            }
        },
    )
}

/// Query one ellipsoidal terrain height h in metres using the embedded EGM96
/// 1-degree geoid grid and explicit terrain interpolation options. Inputs are
/// longitude, latitude degrees.
///
/// Safety: terrain must be a live handle; options must point to
/// SidereonDtedLookupOptions; out_height_m must point to a
/// SidereonEllipsoidalHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_ellipsoidal_height_m_with_options(
    terrain: *const SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    options: *const SidereonDtedLookupOptions,
    out_height_m: *mut SidereonEllipsoidalHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
                "out_height_m"
            ));
            *out = SidereonEllipsoidalHeightM { value_m: 0.0 };
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
                "terrain"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
                options
            ));
            match terrain.inner.ellipsoidal_height_m_with_options(
                longitude_deg,
                latitude_deg,
                options,
            ) {
                Ok(value) => {
                    *out = ellipsoidal_height_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_datum_error(
                    "sidereon_mmap_terrain_ellipsoidal_height_m_with_options",
                    err,
                ),
            }
        },
    )
}

/// Query one ellipsoidal terrain height h in metres using an explicit geoid
/// tier. The EGM96 15-arcminute tier requires a loaded WW15MGH.DAC handle and
/// never falls back to the embedded 1-degree grid. Inputs are longitude,
/// latitude degrees.
///
/// Safety: terrain must be a live handle; options must point to
/// SidereonDtedLookupOptions; geoid may be NULL only for Egm96OneDegree;
/// out_height_m must point to a SidereonEllipsoidalHeightM.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_ellipsoidal_height_m_with_model(
    terrain: *const SidereonMmapTerrain,
    longitude_deg: f64,
    latitude_deg: f64,
    options: *const SidereonDtedLookupOptions,
    geoid_model: u32,
    geoid: *const SidereonEgm96FifteenMinuteGeoid,
    out_height_m: *mut SidereonEllipsoidalHeightM,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_height_m,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                "out_height_m"
            ));
            *out = SidereonEllipsoidalHeightM { value_m: 0.0 };
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                "terrain"
            ));
            let options = c_try!(require_ref(
                options,
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                "options"
            ));
            let options = c_try!(dted_options_from_c(
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                options
            ));
            let model = c_try!(terrain_geoid_model_from_c(
                "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                geoid_model,
                geoid
            ));
            match terrain.inner.ellipsoidal_height_m_with_model(
                longitude_deg,
                latitude_deg,
                options,
                model,
            ) {
                Ok(value) => {
                    *out = ellipsoidal_height_to_c(value);
                    SidereonStatus::Ok
                }
                Err(err) => map_terrain_datum_error(
                    "sidereon_mmap_terrain_ellipsoidal_height_m_with_model",
                    err,
                ),
            }
        },
    )
}

/// Copy terrain store tile index rows. Uses the variable-length output contract.
/// Each row carries the tile bounds, payload location, checksum, and vertical datum.
///
/// Safety: terrain must be a live handle; out must point to len writable rows or
/// be NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_tile_index(
    terrain: *const SidereonMmapTerrain,
    out: *mut SidereonTerrainStoreTileIndex,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_tile_index",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_mmap_terrain_tile_index",
                out_written,
                out_required
            ));
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_tile_index",
                "terrain"
            ));
            let values: Vec<SidereonTerrainStoreTileIndex> = terrain
                .inner
                .tile_index()
                .iter()
                .map(terrain_store_tile_index_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_mmap_terrain_tile_index",
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

/// Return the store-level vertical datum. Terrain store postings are orthometric
/// heights in metres.
///
/// Safety: terrain must be a live handle; out_datum must point to a
/// SidereonVerticalDatum.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_vertical_datum(
    terrain: *const SidereonMmapTerrain,
    out_datum: *mut SidereonVerticalDatum,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_vertical_datum",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_datum,
                "sidereon_mmap_terrain_vertical_datum",
                "out_datum"
            ));
            *out = SidereonVerticalDatum::Egm96MslOrthometric;
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_vertical_datum",
                "terrain"
            ));
            *out = vertical_datum_to_c(terrain.inner.vertical_datum());
            SidereonStatus::Ok
        },
    )
}

/// Return an FNV-1a checksum of the full terrain store byte span.
///
/// Safety: terrain must be a live handle; out_checksum64 must point to a
/// uint64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_checksum64(
    terrain: *const SidereonMmapTerrain,
    out_checksum64: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_checksum64",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_checksum64,
                "sidereon_mmap_terrain_checksum64",
                "out_checksum64"
            ));
            *out = 0;
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_checksum64",
                "terrain"
            ));
            *out = terrain.inner.checksum64();
            SidereonStatus::Ok
        },
    )
}

/// Return whether the checksum carried by this terrain handle is verified or
/// caller-attested.
///
/// Safety: terrain must be a live handle; out_provenance must point to a
/// SidereonDigestProvenance.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_digest_provenance(
    terrain: *const SidereonMmapTerrain,
    out_provenance: *mut SidereonDigestProvenance,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_digest_provenance",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_provenance,
                "sidereon_mmap_terrain_digest_provenance",
                "out_provenance"
            ));
            *out = SidereonDigestProvenance::Verified;
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_digest_provenance",
                "terrain"
            ));
            *out = digest_provenance_to_c(terrain.inner.digest_provenance());
            SidereonStatus::Ok
        },
    )
}

/// Hash and verify tile payloads and any caller-attested full-store checksum.
/// Success changes digest provenance to SIDEREON_DIGEST_PROVENANCE_VERIFIED.
///
/// Safety: terrain must be a live mutable handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_verify(
    terrain: *mut SidereonMmapTerrain,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_verify",
        SidereonStatus::Panic,
        || {
            let terrain = c_try!(require_out(
                terrain,
                "sidereon_mmap_terrain_verify",
                "terrain"
            ));
            match terrain.inner.verify() {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_terrain_store_error("sidereon_mmap_terrain_verify", err),
            }
        },
    )
}

/// Serialize the parsed terrain store back to bytes. Uses the variable-length
/// output contract. The output bytes preserve orthometric terrain postings.
///
/// Safety: terrain must be a live handle; out must point to len writable bytes
/// or be NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_to_bytes(
    terrain: *const SidereonMmapTerrain,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_mmap_terrain_to_bytes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_mmap_terrain_to_bytes",
                out_written,
                out_required
            ));
            let terrain = c_try!(require_ref(
                terrain,
                "sidereon_mmap_terrain_to_bytes",
                "terrain"
            ));
            let bytes = terrain.inner.to_bytes();
            c_try!(copy_prefix_to_c(
                "sidereon_mmap_terrain_to_bytes",
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

/// Release a memory-mappable terrain reader handle. Passing NULL is a no-op.
///
/// Safety: terrain must be NULL or a live handle from sidereon_mmap_terrain_*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_mmap_terrain_free(terrain: *mut SidereonMmapTerrain) {
    free_boxed(terrain);
}

fn map_terrain_core_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn ellipsoidal_height_to_c(
    value: sidereon_core::terrain_store::EllipsoidalHeightM,
) -> SidereonEllipsoidalHeightM {
    SidereonEllipsoidalHeightM {
        value_m: value.metres(),
    }
}

fn terrain_height_result_from_f64(
    result: sidereon_core::Result<f64>,
) -> SidereonTerrainHeightResult {
    match result {
        Ok(value_m) => SidereonTerrainHeightResult {
            status: SidereonStatus::Ok,
            has_orthometric_height_m: true,
            orthometric_height_m: SidereonOrthometricHeightM { value_m },
        },
        Err(_) => SidereonTerrainHeightResult {
            status: SidereonStatus::InvalidArgument,
            has_orthometric_height_m: false,
            orthometric_height_m: SidereonOrthometricHeightM { value_m: 0.0 },
        },
    }
}

fn terrain_height_result_from_orthometric(
    result: sidereon_core::Result<CoreOrthometricHeightM>,
) -> SidereonTerrainHeightResult {
    match result {
        Ok(value) => SidereonTerrainHeightResult {
            status: SidereonStatus::Ok,
            has_orthometric_height_m: true,
            orthometric_height_m: orthometric_height_to_c(value),
        },
        Err(_) => SidereonTerrainHeightResult {
            status: SidereonStatus::InvalidArgument,
            has_orthometric_height_m: false,
            orthometric_height_m: SidereonOrthometricHeightM { value_m: 0.0 },
        },
    }
}

fn terrain_store_tile_index_to_c(
    value: &CoreTerrainStoreTileIndex,
) -> SidereonTerrainStoreTileIndex {
    SidereonTerrainStoreTileIndex {
        lat_index: value.lat_index,
        lon_index: value.lon_index,
        min_longitude_deg: value.min_longitude_deg,
        min_latitude_deg: value.min_latitude_deg,
        max_longitude_deg: value.max_longitude_deg,
        max_latitude_deg: value.max_latitude_deg,
        lon_count: value.lon_count,
        lat_count: value.lat_count,
        data_offset: value.data_offset,
        data_len: value.data_len,
        checksum64: value.checksum64,
        vertical_datum: vertical_datum_to_c(value.vertical_datum) as u32,
    }
}

fn vertical_datum_to_c(value: CoreVerticalDatum) -> SidereonVerticalDatum {
    match value {
        CoreVerticalDatum::Egm96MslOrthometric => SidereonVerticalDatum::Egm96MslOrthometric,
    }
}

fn orthometric_height_to_c(value: CoreOrthometricHeightM) -> SidereonOrthometricHeightM {
    SidereonOrthometricHeightM {
        value_m: value.metres(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    const DATA_OFFSET: usize = 4096;

    struct TempStore(PathBuf);

    impl TempStore {
        fn write(name: &str, bytes: &[u8]) -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sidereon-c-{name}-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&path, bytes).expect("write temporary terrain store");
            Self(path)
        }

        fn c_path(&self) -> CString {
            CString::new(self.0.to_string_lossy().as_bytes()).expect("temporary path has no NUL")
        }
    }

    impl Drop for TempStore {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn put_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn terrain_store() -> Vec<u8> {
        const HEADER_LEN: usize = 64;
        const PAYLOAD_LEN: usize = 8;
        let payload: Vec<u8> = [10_i16, 20, 30, 40]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect();
        let mut store = vec![0_u8; DATA_OFFSET + PAYLOAD_LEN];
        store[..8].copy_from_slice(b"TMMAP001");
        put_u16(&mut store, 8, 1);
        store[10] = 1;
        put_u32(&mut store, 12, 1);
        put_u64(&mut store, 16, HEADER_LEN as u64);
        put_u64(&mut store, 24, DATA_OFFSET as u64);
        let total_len = store.len() as u64;
        put_u64(&mut store, 32, total_len);

        let record = &mut store[HEADER_LEN..HEADER_LEN + 80];
        put_i32(record, 0, 36);
        put_i32(record, 4, -107);
        put_u32(record, 8, 2);
        put_u32(record, 12, 2);
        put_u64(record, 16, DATA_OFFSET as u64);
        put_u64(record, 24, PAYLOAD_LEN as u64);
        put_u64(record, 32, core_terrain_store_checksum64(&payload));
        put_f64(record, 40, 36.0);
        put_f64(record, 48, -107.0);
        put_f64(record, 56, 37.0);
        put_f64(record, 64, -106.0);
        record[72] = 1;
        store[DATA_OFFSET..].copy_from_slice(&payload);
        store
    }

    #[test]
    fn attested_open_defers_corrupt_payload_failure_until_verify() {
        let mut corrupt = terrain_store();
        corrupt[DATA_OFFSET + 1] ^= 1;
        let store = TempStore::write("corrupt-terrain.tmm", &corrupt);
        let path = store.c_path();

        let mut verified = ptr::null_mut();
        let status = unsafe { sidereon_mmap_terrain_from_path(path.as_ptr(), &mut verified) };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(verified.is_null());
        let mut typed = empty_terrain_store_error();
        assert_eq!(
            unsafe { sidereon_last_terrain_store_error(&mut typed) },
            SidereonStatus::Ok
        );
        assert_eq!(typed.kind, SidereonTerrainStoreErrorKind::Checksum as u32);

        let claim = 0x0123_4567_89ab_cdef;
        let mut attested = ptr::null_mut();
        let status = unsafe {
            sidereon_mmap_terrain_from_path_attested(path.as_ptr(), claim, &mut attested)
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!attested.is_null());

        let mut provenance = SidereonDigestProvenance::Verified;
        assert_eq!(
            unsafe { sidereon_mmap_terrain_digest_provenance(attested, &mut provenance) },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, SidereonDigestProvenance::Attested);
        let mut checksum = 0;
        assert_eq!(
            unsafe { sidereon_mmap_terrain_checksum64(attested, &mut checksum) },
            SidereonStatus::Ok
        );
        assert_eq!(checksum, claim);

        assert_eq!(
            unsafe { sidereon_mmap_terrain_verify(attested) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { sidereon_last_terrain_store_error(&mut typed) },
            SidereonStatus::Ok
        );
        assert_eq!(typed.kind, SidereonTerrainStoreErrorKind::Checksum as u32);
        assert_eq!(
            unsafe { sidereon_mmap_terrain_digest_provenance(attested, &mut provenance) },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, SidereonDigestProvenance::Attested);
        unsafe { sidereon_mmap_terrain_free(attested) };
    }

    #[test]
    fn pristine_attested_open_matches_verified_and_escalates() {
        let bytes = terrain_store();
        let claim = core_terrain_store_checksum64(&bytes);
        let store = TempStore::write("pristine-terrain.tmm", &bytes);
        let path = store.c_path();

        let mut verified = ptr::null_mut();
        let mut attested = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_mmap_terrain_from_path(path.as_ptr(), &mut verified) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_mmap_terrain_from_path_attested(path.as_ptr(), claim, &mut attested)
            },
            SidereonStatus::Ok
        );

        let mut verified_height = SidereonOrthometricHeightM { value_m: 0.0 };
        let mut attested_height = SidereonOrthometricHeightM { value_m: 0.0 };
        assert_eq!(
            unsafe { sidereon_mmap_terrain_height_m(verified, -106.5, 36.5, &mut verified_height) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_mmap_terrain_height_m(attested, -106.5, 36.5, &mut attested_height) },
            SidereonStatus::Ok
        );
        assert_eq!(
            verified_height.value_m.to_bits(),
            attested_height.value_m.to_bits()
        );

        let mut checksum = 0;
        assert_eq!(
            unsafe { sidereon_mmap_terrain_checksum64(attested, &mut checksum) },
            SidereonStatus::Ok
        );
        assert_eq!(checksum, claim);
        assert_eq!(
            unsafe { sidereon_mmap_terrain_verify(attested) },
            SidereonStatus::Ok
        );
        let mut provenance = SidereonDigestProvenance::Attested;
        assert_eq!(
            unsafe { sidereon_mmap_terrain_digest_provenance(attested, &mut provenance) },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, SidereonDigestProvenance::Verified);

        unsafe {
            sidereon_mmap_terrain_free(verified);
            sidereon_mmap_terrain_free(attested);
        }

        let wrong_claim = claim ^ 1;
        let mut wrong = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_mmap_terrain_from_path_attested(path.as_ptr(), wrong_claim, &mut wrong)
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_mmap_terrain_verify(wrong) },
            SidereonStatus::InvalidArgument
        );
        let mut typed = empty_terrain_store_error();
        assert_eq!(
            unsafe { sidereon_last_terrain_store_error(&mut typed) },
            SidereonStatus::Ok
        );
        assert_eq!(
            typed.kind,
            SidereonTerrainStoreErrorKind::AttestedChecksumMismatch as u32
        );
        assert_eq!(typed.expected_checksum64, wrong_claim);
        assert_eq!(typed.found_checksum64, claim);
        unsafe { sidereon_mmap_terrain_free(wrong) };
    }
}
