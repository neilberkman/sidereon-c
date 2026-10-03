use super::*;
use crate::terrain::{
    dted_horizontal_datum_to_c, no_terrain_lookup_error, record_terrain_lookup_error,
    terrain_lookup_error_to_c,
};
use sidereon_core::terrain_store::{
    dted_tile_list_to_mmap_store as core_dted_tile_list_to_mmap_store,
    write_dted_tile_list_to_mmap_store as core_write_dted_tile_list_to_mmap_store,
    DtedTileListEntry as CoreDtedTileListEntry, TerrainTileId as CoreTerrainTileId,
};

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

/// One explicit DTED source for list-based memory-mappable terrain-store
/// construction. The core builder parses the DTED header and validates that
/// the parsed origin matches tile_id.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDtedTileListEntry {
    /// Expected integer DTED tile id.
    pub tile_id: SidereonTerrainTileId,
    /// Non-empty UTF-8 path to the DTED `.dt2` tile.
    pub path: *const c_char,
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
    /// The typed failure when status is not OK; its kind is None for a height.
    /// A null posting the lookup weights is UnknownElevation, not a height.
    pub error: SidereonTerrainLookupError,
}

/// Which failure a DTED tile read or query reported. Every kind but None and
/// Unknown names a `DtedTileError` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonDtedTileErrorKind {
    /// No failure is recorded.
    None = 0,
    /// The tile could not be read from disk. Carries path and message.
    Io = 1,
    /// The tile does not contain the fixed DTED header area. Carries path.
    TooShort = 2,
    /// The tile does not start with the UHL1 marker. Carries path.
    MissingUhl1 = 3,
    /// A fixed-width field was not valid UTF-8. Carries message.
    InvalidEncoding = 4,
    /// A numeric field could not be parsed. Carries message.
    InvalidField = 5,
    /// The tile dimensions are too small to define a grid cell. Carries path,
    /// lon_count and lat_count.
    InvalidDimensions = 6,
    /// The tile ends before its declared data blocks end. Carries path,
    /// actual_bytes and expected_bytes.
    Truncated = 7,
    /// A query is outside the tile's one-degree extent. Carries longitude_deg,
    /// latitude_deg, origin_longitude_deg and origin_latitude_deg.
    Outside = 8,
    /// A rounded query did not map to a declared posting. Carries
    /// longitude_index and latitude_index.
    PostingIndexOutOfBounds = 9,
    /// A data block is missing its sentinel byte. Carries longitude_index.
    MissingDataSentinel = 10,
    /// A data block checksum does not match its contents. Carries
    /// longitude_index, checksum and sum.
    Checksum = 11,
    /// A coordinate field is empty.
    EmptyCoordinate = 12,
    /// A coordinate field has an unsupported hemisphere suffix. Carries
    /// hemisphere.
    InvalidHemisphere = 13,
    /// A rounded coordinate is negative. Carries negative_index.
    NegativePostingIndex = 14,
    /// A UHL origin field states degrees outside its axis, or minutes or
    /// seconds outside 0..60. Carries field and text.
    CoordinateOutOfRange = 15,
    /// A UHL origin field carries a hemisphere letter of the other axis.
    /// Carries field, hemisphere and expected_hemispheres.
    WrongHemisphere = 16,
    /// A UHL origin is not a whole degree. Carries field and text.
    OriginNotWholeDegree = 17,
    /// A UHL data interval and posting count do not span one degree. Carries
    /// field, interval_tenths_arcsec and count.
    IntervalCountMismatch = 18,
    /// A data record's longitude count does not match its position in the
    /// file. Carries longitude_index and declared.
    ProfileLongitudeCountMismatch = 19,
    /// A data record is a partial profile, which is refused rather than read at
    /// the wrong latitudes. Carries longitude_index and declared, the record's
    /// first latitude count.
    UnsupportedPartialProfile = 20,
    /// The posting holds the DTED null value (all bits set, MIL-PRF-89020B
    /// 3.11.3.1), an unknown elevation rather than a height. Carries
    /// longitude_index and latitude_index.
    NullPosting = 21,
    /// A failure this binding does not yet name. Its text is in the message.
    Unknown = 999,
}

/// Typed detail of a failed DTED tile read or query. Only the fields the kind
/// names carry meaning; each number group carries a present flag and each text
/// is empty when the kind carries none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDtedTileError {
    /// Which failure was reported.
    pub kind: SidereonDtedTileErrorKind,
    /// UHL field name, null-terminated, when the kind carries one.
    pub field: [c_char; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
    /// UHL field text as read, null-terminated, when the kind carries one.
    pub text: [c_char; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
    /// Whether lon_count and lat_count carry the tile dimensions.
    pub has_counts: bool,
    /// Longitude count from the UHL header.
    pub lon_count: usize,
    /// Latitude count from the UHL header.
    pub lat_count: usize,
    /// Whether actual_bytes and expected_bytes carry the tile length.
    pub has_lengths: bool,
    /// Bytes the tile holds.
    pub actual_bytes: usize,
    /// Bytes the declared data blocks need.
    pub expected_bytes: usize,
    /// Whether the query and origin coordinates are carried.
    pub has_query: bool,
    /// Query longitude, degrees.
    pub longitude_deg: f64,
    /// Query latitude, degrees.
    pub latitude_deg: f64,
    /// Tile origin longitude, degrees.
    pub origin_longitude_deg: f64,
    /// Tile origin latitude, degrees.
    pub origin_latitude_deg: f64,
    /// Whether longitude_index carries a posting or data-block index.
    pub has_longitude_index: bool,
    /// Zero-based longitude posting (profile) or data-block index.
    pub longitude_index: usize,
    /// Whether latitude_index carries a posting index.
    pub has_latitude_index: bool,
    /// Zero-based latitude posting index.
    pub latitude_index: usize,
    /// Whether checksum and sum carry a data-block checksum comparison.
    pub has_checksum: bool,
    /// Checksum the block states.
    pub checksum: i32,
    /// Sum of the block's bytes.
    pub sum: i32,
    /// Whether hemisphere carries a hemisphere letter.
    pub has_hemisphere: bool,
    /// The hemisphere letter found.
    pub hemisphere: u32,
    /// Hemisphere letters the field allows, null-terminated, for
    /// WrongHemisphere.
    pub expected_hemispheres: [c_char; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
    /// Whether negative_index carries a rounded negative index.
    pub has_negative_index: bool,
    /// The rounded negative posting index.
    pub negative_index: i64,
    /// Whether interval_tenths_arcsec and count carry a UHL interval check.
    pub has_interval: bool,
    /// Interval in tenths of an arc second.
    pub interval_tenths_arcsec: u32,
    /// Posting count on the same axis.
    pub count: usize,
    /// Whether declared carries a data record's declared count.
    pub has_declared: bool,
    /// The longitude count a record declares (ProfileLongitudeCountMismatch)
    /// or its first latitude count (UnsupportedPartialProfile).
    pub declared: i32,
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
/// A missing tile reads as sea level. A lookup that gives nonzero weight to a
/// DTED null posting, an unknown elevation, is refused unless a neighbouring
/// tile knows the height at the same place, and a tile whose DSI names a
/// horizontal datum other than WGS84 is refused. A refusal returns
/// SIDEREON_STATUS_INVALID_ARGUMENT with `*out_height_m` 0;
/// sidereon_last_terrain_lookup_error reports it typed, with the tile and the
/// posting or datum.
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
            let terrain = c_try!(require_mut(
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
/// latitude in degrees. The returned height is orthometric meters. Refusals
/// follow sidereon_dted_terrain_height_m.
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
            let terrain = c_try!(require_mut(
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
/// `out[i].status` and `out[i].error` (their texts through
/// sidereon_last_terrain_batch_error_text) and do not fail the whole call.
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
            let terrain = c_try!(require_mut(
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
                    error: no_terrain_lookup_error(),
                });
            }
            let points: Vec<(f64, f64)> = raw_points
                .iter()
                .map(|point| (point.lon_deg, point.lat_deg))
                .collect();
            crate::terrain::reset_terrain_batch_texts(count);
            let results = terrain.inner.height_batch(&points, options);
            for (idx, result) in results.into_iter().enumerate() {
                if let Err(err) = &result {
                    crate::terrain::record_terrain_batch_row_texts(idx, err);
                }
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
/// The tile is checked against the metadata the reader places postings by: UHL
/// origins must be whole degrees inside their axis with that axis's hemisphere
/// letters, a stated UHL data interval must span one degree over the posting
/// count, and each data record must declare the longitude count of its
/// position and latitude count zero. A failure returns
/// SIDEREON_STATUS_INVALID_ARGUMENT; sidereon_last_dted_tile_error reports it
/// typed. A tile on any horizontal datum loads; read it with
/// sidereon_dted_tile_horizontal_datum.
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
            Err(err) => map_dted_tile_error("sidereon_dted_tile_load", &err),
        }
    })
}

/// Query the nearest stored posting in a loaded DTED tile. Inputs are longitude,
/// latitude in degrees. The returned integer elevation is an orthometric height
/// in meters.
///
/// A posting holding the DTED null value (all bits set) is an unknown
/// elevation, not a height: the call returns SIDEREON_STATUS_INVALID_ARGUMENT
/// with `*out_elevation_m` 0, and sidereon_last_dted_tile_error reports
/// NullPosting with the posting indices.
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
                Err(err) => map_dted_tile_error("sidereon_dted_tile_get_elevation", &err),
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

/// Convert an explicit DTED tile list into canonical memory-mappable terrain
/// store bytes. Header parsing, tile-id validation, sorting, deduplication,
/// alignment, and checksums are performed by the public core converter.
///
/// Safety: entries points to entry_count SidereonDtedTileListEntry values, or
/// is NULL when entry_count is zero; each path is a non-empty UTF-8 C string;
/// out points to len bytes or is NULL when len is zero; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tile_list_to_mmap_store(
    entries: *const SidereonDtedTileListEntry,
    entry_count: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_dted_tile_list_to_mmap_store",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_dted_tile_list_to_mmap_store",
                out_written,
                out_required
            ));
            let entries = c_try!(dted_tile_list_entries_from_c(
                "sidereon_dted_tile_list_to_mmap_store",
                entries,
                entry_count,
            ));
            let bytes = match core_dted_tile_list_to_mmap_store(&entries) {
                Ok(bytes) => bytes,
                Err(err) => {
                    return map_terrain_store_error("sidereon_dted_tile_list_to_mmap_store", err);
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_dted_tile_list_to_mmap_store",
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

/// Convert an explicit DTED tile list and write the canonical memory-mappable
/// terrain store through the public core writer.
///
/// Safety: entries points to entry_count SidereonDtedTileListEntry values, or
/// is NULL when entry_count is zero; each path and out_path is a non-empty
/// UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn sidereon_write_dted_tile_list_to_mmap_store(
    entries: *const SidereonDtedTileListEntry,
    entry_count: usize,
    out_path: *const c_char,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_write_dted_tile_list_to_mmap_store",
        SidereonStatus::Panic,
        || {
            let entries = c_try!(dted_tile_list_entries_from_c(
                "sidereon_write_dted_tile_list_to_mmap_store",
                entries,
                entry_count,
            ));
            let out_path = c_try!(parse_c_string(
                "sidereon_write_dted_tile_list_to_mmap_store",
                "out_path",
                out_path,
            ));
            match core_write_dted_tile_list_to_mmap_store(&entries, std::path::Path::new(&out_path))
            {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => {
                    map_terrain_store_error("sidereon_write_dted_tile_list_to_mmap_store", err)
                }
            }
        },
    )
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
                    return map_terrain_store_error("sidereon_dted_tree_to_mmap_store", err);
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

unsafe fn dted_tile_list_entries_from_c(
    fn_name: &str,
    entries: *const SidereonDtedTileListEntry,
    entry_count: usize,
) -> Result<Vec<CoreDtedTileListEntry>, SidereonStatus> {
    let raw = require_slice(entries, entry_count, fn_name, "entries")?;
    let mut converted = Vec::with_capacity(raw.len());
    for (idx, entry) in raw.iter().enumerate() {
        let path = parse_c_string(fn_name, &format!("entries[{idx}].path"), entry.path)?;
        converted.push(CoreDtedTileListEntry::new(
            CoreTerrainTileId::new(entry.tile_id.lat_index, entry.tile_id.lon_index),
            path,
        ));
    }
    Ok(converted)
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

thread_local! {
    static LAST_DTED_TILE_ERROR: RefCell<Option<SidereonDtedTileError>> =
        const { RefCell::new(None) };
}

pub(crate) fn no_dted_tile_error() -> SidereonDtedTileError {
    SidereonDtedTileError {
        kind: SidereonDtedTileErrorKind::None,
        field: [0; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
        text: [0; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
        has_counts: false,
        lon_count: 0,
        lat_count: 0,
        has_lengths: false,
        actual_bytes: 0,
        expected_bytes: 0,
        has_query: false,
        longitude_deg: f64::NAN,
        latitude_deg: f64::NAN,
        origin_longitude_deg: f64::NAN,
        origin_latitude_deg: f64::NAN,
        has_longitude_index: false,
        longitude_index: 0,
        has_latitude_index: false,
        latitude_index: 0,
        has_checksum: false,
        checksum: 0,
        sum: 0,
        has_hemisphere: false,
        hemisphere: 0,
        expected_hemispheres: [0; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
        has_negative_index: false,
        negative_index: 0,
        has_interval: false,
        interval_tenths_arcsec: 0,
        count: 0,
        has_declared: false,
        declared: 0,
    }
}

/// Map every field of an engine DTED tile failure into the C record.
pub(crate) fn dted_tile_error_to_c(
    err: &sidereon_core::terrain::DtedTileError,
) -> (SidereonDtedTileError, Vec<(u32, String)>) {
    use sidereon_core::terrain::DtedTileError as E;
    use SidereonDtedTileErrorKind as Kind;
    let mut texts = Vec::new();
    let mut out = no_dted_tile_error();
    match err {
        E::Io { path, message } => {
            out.kind = Kind::Io;
            texts.push((SidereonTerrainErrorText::Path as u32, String::from(path)));
            texts.push((
                SidereonTerrainErrorText::Message as u32,
                String::from(message),
            ));
        }
        E::TooShort { path } => {
            out.kind = Kind::TooShort;
            texts.push((SidereonTerrainErrorText::Path as u32, String::from(path)));
        }
        E::MissingUhl1 { path } => {
            out.kind = Kind::MissingUhl1;
            texts.push((SidereonTerrainErrorText::Path as u32, String::from(path)));
        }
        E::InvalidEncoding(message) => {
            out.kind = Kind::InvalidEncoding;
            texts.push((
                SidereonTerrainErrorText::Message as u32,
                String::from(message),
            ));
        }
        E::InvalidField(message) => {
            out.kind = Kind::InvalidField;
            texts.push((
                SidereonTerrainErrorText::Message as u32,
                String::from(message),
            ));
        }
        E::InvalidDimensions {
            path,
            lon_count,
            lat_count,
        } => {
            out.kind = Kind::InvalidDimensions;
            texts.push((SidereonTerrainErrorText::Path as u32, String::from(path)));
            out.has_counts = true;
            out.lon_count = *lon_count;
            out.lat_count = *lat_count;
        }
        E::Truncated {
            path,
            actual,
            expected,
        } => {
            out.kind = Kind::Truncated;
            texts.push((SidereonTerrainErrorText::Path as u32, String::from(path)));
            out.has_lengths = true;
            out.actual_bytes = *actual;
            out.expected_bytes = *expected;
        }
        E::Outside {
            longitude,
            latitude,
            origin_longitude,
            origin_latitude,
        } => {
            out.kind = Kind::Outside;
            out.has_query = true;
            out.longitude_deg = *longitude;
            out.latitude_deg = *latitude;
            out.origin_longitude_deg = *origin_longitude;
            out.origin_latitude_deg = *origin_latitude;
        }
        E::PostingIndexOutOfBounds {
            longitude_index,
            latitude_index,
        } => {
            out.kind = Kind::PostingIndexOutOfBounds;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
            out.has_latitude_index = true;
            out.latitude_index = *latitude_index;
        }
        E::MissingDataSentinel { longitude_index } => {
            out.kind = Kind::MissingDataSentinel;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
        }
        E::Checksum {
            longitude_index,
            checksum,
            sum,
        } => {
            out.kind = Kind::Checksum;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
            out.has_checksum = true;
            out.checksum = *checksum;
            out.sum = *sum;
        }
        E::EmptyCoordinate => out.kind = Kind::EmptyCoordinate,
        E::InvalidHemisphere { hemisphere } => {
            out.kind = Kind::InvalidHemisphere;
            out.has_hemisphere = true;
            out.hemisphere = u32::from(*hemisphere);
        }
        E::NegativePostingIndex { index } => {
            out.kind = Kind::NegativePostingIndex;
            out.has_negative_index = true;
            out.negative_index = *index;
        }
        E::CoordinateOutOfRange { field, text } => {
            out.kind = Kind::CoordinateOutOfRange;
            out.field = fixed_c_chars(field);
            out.text = fixed_c_chars(text);
        }
        E::WrongHemisphere {
            field,
            hemisphere,
            expected,
        } => {
            out.kind = Kind::WrongHemisphere;
            out.field = fixed_c_chars(field);
            out.has_hemisphere = true;
            out.hemisphere = u32::from(*hemisphere);
            out.expected_hemispheres = fixed_c_chars(expected);
        }
        E::OriginNotWholeDegree { field, text } => {
            out.kind = Kind::OriginNotWholeDegree;
            out.field = fixed_c_chars(field);
            out.text = fixed_c_chars(text);
        }
        E::IntervalCountMismatch {
            field,
            interval_tenths_arcsec,
            count,
        } => {
            out.kind = Kind::IntervalCountMismatch;
            out.field = fixed_c_chars(field);
            out.has_interval = true;
            out.interval_tenths_arcsec = *interval_tenths_arcsec;
            out.count = *count;
        }
        E::ProfileLongitudeCountMismatch {
            longitude_index,
            declared,
        } => {
            out.kind = Kind::ProfileLongitudeCountMismatch;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
            out.has_declared = true;
            out.declared = *declared;
        }
        E::UnsupportedPartialProfile {
            longitude_index,
            first_latitude_index,
        } => {
            out.kind = Kind::UnsupportedPartialProfile;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
            out.has_declared = true;
            out.declared = *first_latitude_index;
        }
        E::NullPosting {
            longitude_index,
            latitude_index,
        } => {
            out.kind = Kind::NullPosting;
            out.has_longitude_index = true;
            out.longitude_index = *longitude_index;
            out.has_latitude_index = true;
            out.latitude_index = *latitude_index;
        }
        // `DtedTileError` is non-exhaustive: a failure a later engine adds
        // reads as Unknown, with the engine's text and variant name in the
        // MESSAGE text.
        other => {
            out.kind = Kind::Unknown;
            texts.push((
                SidereonTerrainErrorText::Message as u32,
                format!("{other} ({other:?})"),
            ));
        }
    }
    (out, texts)
}

/// Record a DTED tile failure for sidereon_last_dted_tile_error and its texts
/// in the DTED-tile family.
pub(crate) fn record_dted_tile_error(err: &sidereon_core::terrain::DtedTileError) {
    let (typed, texts) = dted_tile_error_to_c(err);
    LAST_DTED_TILE_ERROR.with(|slot| *slot.borrow_mut() = Some(typed));
    record_terrain_error_texts(SidereonTerrainErrorFamily::DtedTile, texts);
}

fn map_dted_tile_error(
    fn_name: &str,
    err: &sidereon_core::terrain::DtedTileError,
) -> SidereonStatus {
    record_dted_tile_error(err);
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_dted_core_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    record_terrain_lookup_error(fn_name, &err)
}

fn dted_height_result_from_core(result: sidereon_core::Result<f64>) -> SidereonDtedHeightResult {
    match result {
        Ok(height_m) => SidereonDtedHeightResult {
            status: SidereonStatus::Ok,
            has_height_m: true,
            height_m,
            error: no_terrain_lookup_error(),
        },
        Err(err) => SidereonDtedHeightResult {
            status: SidereonStatus::InvalidArgument,
            has_height_m: false,
            height_m: 0.0,
            error: terrain_lookup_error_to_c(&err),
        },
    }
}

/// Copy the last typed DTED tile failure for this thread: the most recent
/// sidereon_dted_tile_load or sidereon_dted_tile_get_elevation call that
/// failed, or the most recent terrain store builder that could not read a
/// DTED input (SIDEREON_TERRAIN_STORE_ERROR_KIND_TILE). If none is recorded, kind is SidereonDtedTileErrorKind::None.
///
/// Safety: out_error must point to a SidereonDtedTileError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_dted_tile_error(
    out_error: *mut SidereonDtedTileError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_dted_tile_error";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out = LAST_DTED_TILE_ERROR
            .with(|slot| *slot.borrow())
            .unwrap_or_else(no_dted_tile_error);
        SidereonStatus::Ok
    })
}

/// Copy the horizontal datum a loaded DTED tile's DSI record states. A blank
/// field reads as Unstated, which counts as WGS84; a tile on any datum loads,
/// and the terrain lookups and store converter refuse one that is not WGS84
/// compatible.
///
/// Safety: tile must be a live handle; out_datum must point to a
/// SidereonDtedHorizontalDatumValue.
#[no_mangle]
pub unsafe extern "C" fn sidereon_dted_tile_horizontal_datum(
    tile: *const SidereonDtedTile,
    out_datum: *mut SidereonDtedHorizontalDatumValue,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_dted_tile_horizontal_datum";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_datum, FN_NAME, "out_datum"));
        *out = no_terrain_lookup_error().horizontal_datum;
        let tile = c_try!(require_ref(tile, FN_NAME, "tile"));
        *out = dted_horizontal_datum_to_c(tile.inner.horizontal_datum());
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    struct TempDirectoryGuard(std::path::PathBuf);

    impl Drop for TempDirectoryGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn dted_tile_list_marshalling_checks_paths_and_core_errors() {
        let temp_root = std::env::temp_dir().join(format!(
            "sidereon-c-dted-marshalling-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_root);
        let _ = std::fs::remove_file(&temp_root);
        std::fs::create_dir_all(&temp_root).unwrap();
        let _cleanup = TempDirectoryGuard(temp_root.clone());

        let entry = SidereonDtedTileListEntry {
            tile_id: SidereonTerrainTileId {
                lat_index: 36,
                lon_index: -107,
            },
            path: ptr::null(),
        };
        let error = unsafe { dted_tile_list_entries_from_c("test_dted_tile_list", &entry, 1) }
            .expect_err("a list entry must have a path");
        assert_eq!(error, SidereonStatus::NullPointer);

        let missing_path = temp_root.join("missing-input.dt2");
        let _ = std::fs::remove_file(&missing_path);
        let missing_path = CString::new(missing_path.to_str().unwrap()).unwrap();
        let entry = SidereonDtedTileListEntry {
            tile_id: SidereonTerrainTileId {
                lat_index: 36,
                lon_index: -107,
            },
            path: missing_path.as_ptr(),
        };
        // sidereon-core's own outcome for the same list: the unreadable input
        // is a typed tile failure naming the input path.
        let core_entries =
            unsafe { dted_tile_list_entries_from_c("test_dted_tile_list", &entry, 1) }
                .expect("a list entry with a path");
        let Err(TerrainStoreError::Tile {
            path: core_path,
            error: core_tile_error,
        }) = core_dted_tile_list_to_mmap_store(&core_entries)
        else {
            panic!("sidereon-core reports an unreadable DTED input as a tile failure");
        };
        let sidereon_core::terrain::DtedTileError::Io {
            path: core_io_path,
            message: core_io_message,
        } = core_tile_error.as_ref()
        else {
            panic!("sidereon-core reports a missing DTED input as an I/O tile failure");
        };
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        let status = unsafe {
            sidereon_dted_tile_list_to_mmap_store(
                &entry,
                1,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let mut typed = unsafe { std::mem::zeroed::<SidereonTerrainStoreError>() };
        assert_eq!(
            unsafe { sidereon_last_terrain_store_error(&mut typed) },
            SidereonStatus::Ok
        );
        assert_eq!(typed.kind, SidereonTerrainStoreErrorKind::Tile as u32);
        assert!(typed.has_tile_error);
        assert_eq!(typed.tile_error.kind, SidereonDtedTileErrorKind::Io);
        assert_eq!((written, required), (0, 0));
        let text = |family: SidereonTerrainErrorFamily, part: SidereonTerrainErrorText| {
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                unsafe {
                    sidereon_last_terrain_error_text(
                        family as u32,
                        part as u32,
                        ptr::null_mut(),
                        0,
                        &mut written,
                        &mut required,
                    )
                },
                SidereonStatus::Ok
            );
            let mut bytes = vec![0u8; required];
            assert_eq!(
                unsafe {
                    sidereon_last_terrain_error_text(
                        family as u32,
                        part as u32,
                        bytes.as_mut_ptr(),
                        bytes.len(),
                        &mut written,
                        &mut required,
                    )
                },
                SidereonStatus::Ok
            );
            assert_eq!(written, required);
            String::from_utf8(bytes).expect("terrain error text is UTF-8")
        };
        assert_eq!(
            text(
                SidereonTerrainErrorFamily::TerrainStore,
                SidereonTerrainErrorText::Path
            ),
            core_path.display().to_string()
        );
        let mut nested = no_dted_tile_error();
        assert_eq!(
            unsafe { sidereon_last_dted_tile_error(&mut nested) },
            SidereonStatus::Ok
        );
        assert_eq!(nested.kind, SidereonDtedTileErrorKind::Io);
        assert_eq!(
            text(
                SidereonTerrainErrorFamily::DtedTile,
                SidereonTerrainErrorText::Path
            ),
            *core_io_path
        );
        assert_eq!(
            text(
                SidereonTerrainErrorFamily::DtedTile,
                SidereonTerrainErrorText::Message
            ),
            *core_io_message
        );

        let missing_parent = temp_root.join("missing-parent");
        let _ = std::fs::remove_dir_all(&missing_parent);
        let output_path = missing_parent.join("sidereon.store");
        // sidereon-core's own outcome for writing an empty list there.
        assert!(matches!(
            core_write_dted_tile_list_to_mmap_store(&[], &output_path),
            Err(TerrainStoreError::Io { .. })
        ));
        assert!(!missing_parent.exists());
        let output_path = CString::new(output_path.to_str().unwrap()).unwrap();
        let status = unsafe {
            sidereon_write_dted_tile_list_to_mmap_store(ptr::null(), 0, output_path.as_ptr())
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        let mut typed = unsafe { std::mem::zeroed::<SidereonTerrainStoreError>() };
        unsafe { sidereon_last_terrain_store_error(&mut typed) };
        assert_eq!(typed.kind, SidereonTerrainStoreErrorKind::Io as u32);
        assert!(!missing_parent.exists());
    }

    fn terrain_text(
        read: impl Fn(*mut u8, usize, *mut usize, *mut usize) -> SidereonStatus,
    ) -> String {
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            read(ptr::null_mut(), 0, &mut written, &mut required),
            SidereonStatus::Ok
        );
        let mut bytes = vec![0u8; required];
        assert_eq!(
            read(bytes.as_mut_ptr(), bytes.len(), &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        String::from_utf8(bytes).expect("terrain error text is UTF-8")
    }

    /// Look up (-106.5, 36.5) in a terrain rooted at `root`, singly and as a
    /// one-point batch, and return the single-point record, the batch row's
    /// record and sidereon-core's own error for the same lookup.
    fn failed_lookup(
        root: &std::path::Path,
    ) -> (
        SidereonTerrainLookupError,
        SidereonTerrainLookupError,
        CoreError,
    ) {
        let core_error = sidereon_core::terrain::DtedTerrain::new(root)
            .height_m(-106.5, 36.5)
            .expect_err("sidereon-core refuses the tile");
        let root_c = CString::new(root.to_str().unwrap()).unwrap();
        let mut terrain: *mut SidereonDtedTerrain = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_dted_terrain_new(root_c.as_ptr(), &mut terrain) },
            SidereonStatus::Ok
        );
        let mut height = 0.0;
        assert_eq!(
            unsafe { sidereon_dted_terrain_height_m(terrain, -106.5, 36.5, &mut height) },
            SidereonStatus::InvalidArgument
        );
        let mut single = no_terrain_lookup_error();
        assert_eq!(
            unsafe { sidereon_last_terrain_lookup_error(&mut single) },
            SidereonStatus::Ok
        );
        unsafe { sidereon_dted_terrain_free(terrain) };

        let mut terrain: *mut SidereonDtedTerrain = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_dted_terrain_new(root_c.as_ptr(), &mut terrain) },
            SidereonStatus::Ok
        );
        let point = SidereonLonLatDeg {
            lon_deg: -106.5,
            lat_deg: 36.5,
        };
        let options = SidereonDtedLookupOptions {
            interpolation: SidereonDtedInterpolation::NearestPosting as u32,
        };
        let mut row = SidereonDtedHeightResult {
            status: SidereonStatus::Ok,
            has_height_m: true,
            height_m: 0.0,
            error: no_terrain_lookup_error(),
        };
        assert_eq!(
            unsafe { sidereon_dted_terrain_height_batch_m(terrain, &point, 1, &options, &mut row) },
            SidereonStatus::Ok
        );
        unsafe { sidereon_dted_terrain_free(terrain) };
        assert_eq!(row.status, SidereonStatus::InvalidArgument);
        assert!(!row.has_height_m);
        (single, row.error, core_error)
    }

    fn lookup_text(
        family_or_row: Result<SidereonTerrainErrorFamily, usize>,
        part: SidereonTerrainErrorText,
    ) -> String {
        terrain_text(|out, len, written, required| unsafe {
            match family_or_row {
                Ok(family) => sidereon_last_terrain_error_text(
                    family as u32,
                    part as u32,
                    out,
                    len,
                    written,
                    required,
                ),
                Err(row) => sidereon_last_terrain_batch_error_text(
                    row,
                    part as u32,
                    out,
                    len,
                    written,
                    required,
                ),
            }
        })
    }

    #[test]
    fn unreadable_and_misplaced_tiles_are_typed_lookup_failures() {
        let temp_root = std::env::temp_dir().join(format!(
            "sidereon-c-dted-lookup-errors-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_root);
        std::fs::create_dir_all(&temp_root).unwrap();
        let _cleanup = TempDirectoryGuard(temp_root.clone());
        let tile_path = temp_root.join("n36_w107_1arc_v3.dt2");

        // A file too short for the DTED headers, named for the queried cell.
        std::fs::write(&tile_path, b"not a tile").unwrap();
        let (single, row, core_error) = failed_lookup(&temp_root);
        let CoreError::TerrainTile {
            lat_index,
            lon_index,
            error: core_tile_error,
        } = &core_error
        else {
            panic!("sidereon-core reports an unreadable tile as TerrainTile: {core_error:?}");
        };
        let sidereon_core::terrain::DtedTileError::TooShort {
            path: core_tile_path,
        } = core_tile_error.as_ref()
        else {
            panic!("sidereon-core reports a short tile as TooShort: {core_tile_error:?}");
        };
        for record in [&single, &row] {
            assert_eq!(record.kind, SidereonTerrainLookupErrorKind::Tile);
            assert!(record.has_tile);
            assert_eq!(
                (record.lat_index, record.lon_index),
                (*lat_index, *lon_index)
            );
            assert!(record.has_tile_error);
            assert_eq!(record.tile_error.kind, SidereonDtedTileErrorKind::TooShort);
            assert!(!record.has_origin);
        }
        assert_eq!(
            lookup_text(
                Ok(SidereonTerrainErrorFamily::TerrainLookup),
                SidereonTerrainErrorText::Path
            ),
            *core_tile_path
        );
        assert_eq!(
            lookup_text(Err(0), SidereonTerrainErrorText::Path),
            *core_tile_path
        );
        let mut nested = no_dted_tile_error();
        assert_eq!(
            unsafe { sidereon_last_dted_tile_error(&mut nested) },
            SidereonStatus::Ok
        );
        assert_eq!(nested.kind, SidereonDtedTileErrorKind::TooShort);

        // A valid tile whose origin (-106) is not the cell its name gives (-107).
        std::fs::write(
            &tile_path,
            include_bytes!("../tests/fixtures/dted/tiles/n36_w106_1arc_v3.dt2"),
        )
        .unwrap();
        let (single, row, core_error) = failed_lookup(&temp_root);
        let CoreError::TerrainTileOrigin {
            path,
            lat_index,
            lon_index,
            origin_latitude,
            origin_longitude,
        } = &core_error
        else {
            panic!("sidereon-core reports a misplaced tile as TerrainTileOrigin: {core_error:?}");
        };
        for record in [&single, &row] {
            assert_eq!(record.kind, SidereonTerrainLookupErrorKind::TileOrigin);
            assert!(record.has_tile);
            assert_eq!(
                (record.lat_index, record.lon_index),
                (*lat_index, *lon_index)
            );
            assert!(record.has_origin);
            assert_eq!(
                (record.origin_latitude_deg, record.origin_longitude_deg),
                (*origin_latitude, *origin_longitude)
            );
            assert!(!record.has_tile_error);
        }
        let path = path.display().to_string();
        assert_eq!(
            lookup_text(
                Ok(SidereonTerrainErrorFamily::TerrainLookup),
                SidereonTerrainErrorText::Path
            ),
            path
        );
        assert_eq!(lookup_text(Err(0), SidereonTerrainErrorText::Path), path);

        // A row past the last batch's points is refused.
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_terrain_batch_error_text(
                    1,
                    SidereonTerrainErrorText::Path as u32,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
    }

    #[test]
    fn tile_list_store_error_keeps_expected_and_parsed_tile_ids() {
        let temp_root = std::env::temp_dir().join(format!(
            "sidereon-c-dted-store-id-mismatch-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_root);
        std::fs::create_dir_all(&temp_root).unwrap();
        let _cleanup = TempDirectoryGuard(temp_root.clone());
        let tile_path = temp_root.join("fixture.dt2");
        std::fs::write(
            &tile_path,
            include_bytes!("../tests/fixtures/dted/tiles/n36_w106_1arc_v3.dt2"),
        )
        .unwrap();
        let path = CString::new(tile_path.to_str().unwrap()).unwrap();
        let mut entry = SidereonDtedTileListEntry {
            tile_id: SidereonTerrainTileId {
                lat_index: 36,
                lon_index: -107,
            },
            path: path.as_ptr(),
        };
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_dted_tile_list_to_mmap_store(
                    &entry,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let mut error = empty_terrain_store_error();
        assert_eq!(
            unsafe { sidereon_last_terrain_store_error(&mut error) },
            SidereonStatus::Ok
        );
        assert_eq!(
            error.kind,
            SidereonTerrainStoreErrorKind::TileIdMismatch as u32
        );
        assert_eq!(
            (
                error.expected_tile_id.lat_index,
                error.expected_tile_id.lon_index
            ),
            (36, -107)
        );
        assert_eq!(
            (error.found_tile_id.lat_index, error.found_tile_id.lon_index),
            (36, -106)
        );

        entry.tile_id.lon_index = -106;
        assert_eq!(
            unsafe {
                sidereon_dted_tile_list_to_mmap_store(
                    &entry,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(required > 0);
        let mut bytes = vec![0u8; required];
        assert_eq!(
            unsafe {
                sidereon_dted_tile_list_to_mmap_store(
                    &entry,
                    1,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
    }
}
