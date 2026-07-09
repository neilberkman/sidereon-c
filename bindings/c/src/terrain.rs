use super::*;

/// Fixed buffer length for terrain store typed error text, including the NUL.
pub const SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES: usize = 512;

// --- Memory-mappable terrain store (sidereon_core::terrain_store) -----------

/// Terrain store vertical datum. Terrain store postings are orthometric heights.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonVerticalDatum {
    /// Orthometric height in metres above the EGM96 mean sea level geoid.
    Egm96MslOrthometric = 1,
}

/// Geoid tier for converting terrain orthometric height to ellipsoidal height.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainGeoidModel {
    /// Embedded EGM96 1-degree grid, always available in process.
    Egm96OneDegree = 0,
    /// Caller-supplied EGM96 15-arcminute WW15MGH.DAC grid.
    Egm96FifteenMinute = 1,
}

/// Copy a terrain vertical-datum label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_vertical_datum_label(
    datum: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_vertical_datum_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_vertical_datum_label",
                out_written,
                out_required
            ));
            let label = c_try!(vertical_datum_label_from_c(
                "sidereon_vertical_datum_label",
                datum
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_vertical_datum_label",
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

/// Copy a terrain geoid-model label into out.
///
/// Safety: out points to len bytes or NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_terrain_geoid_model_label(
    model: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_terrain_geoid_model_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_terrain_geoid_model_label",
                out_written,
                out_required
            ));
            let label = c_try!(terrain_geoid_model_label_from_c(
                "sidereon_terrain_geoid_model_label",
                model
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_terrain_geoid_model_label",
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

/// Terrain store conversion or reader error kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainStoreErrorKind {
    /// No terrain store error is recorded for this thread.
    None = 0,
    /// File or directory I/O failed.
    Io = 1,
    /// Terrain store bytes or DTED input could not be parsed.
    Parse = 2,
    /// Terrain store version is not supported.
    UnsupportedVersion = 3,
    /// Terrain store vertical datum tag is not supported.
    UnsupportedDatum = 4,
    /// Two DTED inputs resolved to the same tile id.
    DuplicateTile = 5,
    /// A tile payload checksum did not match its index record.
    Checksum = 6,
    /// A DTED input's parsed origin did not match the supplied tile id.
    TileIdMismatch = 7,
}

/// Last typed terrain store error for this thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainStoreError {
    /// Error selector as SidereonTerrainStoreErrorKind.
    pub kind: u32,
    /// Path text for I/O errors, NUL-terminated when present.
    pub path: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    /// Message text for I/O errors, NUL-terminated when present.
    pub message: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    /// Parse reason text, NUL-terminated when present.
    pub reason: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    /// Unsupported version tag when kind is UnsupportedVersion.
    pub version: u16,
    /// Unsupported vertical datum tag when kind is UnsupportedDatum.
    pub tag: u8,
    /// Tile latitude id for duplicate-tile and checksum errors.
    pub lat_index: i32,
    /// Tile longitude id for duplicate-tile and checksum errors.
    pub lon_index: i32,
    /// Expected checksum for checksum errors.
    pub expected_checksum64: u64,
    /// Computed checksum for checksum errors.
    pub found_checksum64: u64,
}

/// Terrain datum conversion or geoid loading error kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainDatumErrorKind {
    /// No terrain datum error is recorded for this thread.
    None = 0,
    /// Terrain lookup failed before datum conversion.
    Terrain = 1,
    /// A geoid grid could not be parsed.
    Geoid = 2,
    /// A geoid grid could not be read for a reason other than absence.
    Io = 3,
    /// The EGM96 15-arcminute WW15MGH.DAC grid was requested but is absent.
    MissingEgm96Dac = 4,
}

/// Last typed terrain datum error for this thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainDatumError {
    /// Error selector as SidereonTerrainDatumErrorKind.
    pub kind: u32,
    /// Path text for I/O or MissingEgm96Dac errors, NUL-terminated when present.
    pub path: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    /// Error text for Terrain, Geoid, or I/O errors, NUL-terminated when present.
    pub message: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    /// Remediation text for MissingEgm96Dac, NUL-terminated when present.
    pub remediation: [c_char; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
}

/// Orthometric height H in metres above the EGM96 mean sea level geoid.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrthometricHeightM {
    /// Orthometric height H, metres.
    pub value_m: f64,
}

/// Ellipsoidal height h in metres above the WGS84 reference ellipsoid.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEllipsoidalHeightM {
    /// Ellipsoidal height h, metres.
    pub value_m: f64,
}

/// One memory-mappable terrain store batch result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainHeightResult {
    /// Per-point status.
    pub status: SidereonStatus,
    /// Whether orthometric_height_m carries a valid terrain height.
    pub has_orthometric_height_m: bool,
    /// Orthometric height H, metres, when has_orthometric_height_m is true.
    pub orthometric_height_m: SidereonOrthometricHeightM,
}

/// One tile index record from a memory-mappable terrain store.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainStoreTileIndex {
    /// Integer latitude tile id.
    pub lat_index: i32,
    /// Integer longitude tile id.
    pub lon_index: i32,
    /// Western edge longitude, degrees.
    pub min_longitude_deg: f64,
    /// Southern edge latitude, degrees.
    pub min_latitude_deg: f64,
    /// Eastern edge longitude, degrees.
    pub max_longitude_deg: f64,
    /// Northern edge latitude, degrees.
    pub max_latitude_deg: f64,
    /// Number of longitude postings.
    pub lon_count: u32,
    /// Number of latitude postings.
    pub lat_count: u32,
    /// Byte offset of this tile's posting payload in the store.
    pub data_offset: u64,
    /// Byte length of this tile's posting payload in the store.
    pub data_len: u64,
    /// FNV-1a checksum of this tile's posting payload bytes.
    pub checksum64: u64,
    /// Vertical datum selector as SidereonVerticalDatum.
    pub vertical_datum: u32,
}

/// Copy the last typed terrain store error for this thread. If no terrain store
/// error is recorded, kind is SidereonTerrainStoreErrorKind::None.
///
/// Safety: out_error must point to a SidereonTerrainStoreError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_terrain_store_error(
    out_error: *mut SidereonTerrainStoreError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_last_terrain_store_error",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_error,
                "sidereon_last_terrain_store_error",
                "out_error"
            ));
            *out = LAST_TERRAIN_STORE_ERROR
                .with(|slot| *slot.borrow())
                .unwrap_or_else(empty_terrain_store_error);
            SidereonStatus::Ok
        },
    )
}

/// Copy the last typed terrain datum error for this thread. If no terrain datum
/// error is recorded, kind is SidereonTerrainDatumErrorKind::None.
///
/// Safety: out_error must point to a SidereonTerrainDatumError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_terrain_datum_error(
    out_error: *mut SidereonTerrainDatumError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_last_terrain_datum_error",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_error,
                "sidereon_last_terrain_datum_error",
                "out_error"
            ));
            *out = LAST_TERRAIN_DATUM_ERROR
                .with(|slot| *slot.borrow())
                .unwrap_or_else(empty_terrain_datum_error);
            SidereonStatus::Ok
        },
    )
}

/// Return an FNV-1a checksum for terrain store bytes.
///
/// Safety: bytes must point to len readable bytes; out_checksum64 must point to
/// a uint64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_terrain_store_checksum64(
    bytes: *const u8,
    len: usize,
    out_checksum64: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_terrain_store_checksum64",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_checksum64,
                "sidereon_terrain_store_checksum64",
                "out_checksum64"
            ));
            *out = 0;
            let bytes = c_try!(require_slice(
                bytes,
                len,
                "sidereon_terrain_store_checksum64",
                "bytes"
            ));
            *out = core_terrain_store_checksum64(bytes);
            SidereonStatus::Ok
        },
    )
}

fn vertical_datum_label_from_c(fn_name: &str, datum: u32) -> Result<&'static str, SidereonStatus> {
    match datum {
        value if value == SidereonVerticalDatum::Egm96MslOrthometric as u32 => {
            Ok("VerticalDatum.EGM96_MSL_ORTHOMETRIC")
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid vertical datum"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn terrain_geoid_model_label_from_c(
    fn_name: &str,
    model: u32,
) -> Result<&'static str, SidereonStatus> {
    match model {
        value if value == SidereonTerrainGeoidModel::Egm96OneDegree as u32 => {
            Ok("egm96_one_degree")
        }
        value if value == SidereonTerrainGeoidModel::Egm96FifteenMinute as u32 => {
            Ok("egm96_fifteen_minute")
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid terrain geoid model"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}
