use super::*;

/// Fixed buffer length for terrain store typed error text, including the NUL.
/// Which error the text of sidereon_last_terrain_error_text belongs to.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainErrorFamily {
    /// The last SidereonDtedTileError (sidereon_last_dted_tile_error).
    DtedTile = 0,
    /// The last SidereonTerrainStoreError.
    TerrainStore = 1,
    /// The last SidereonTerrainDatumError.
    TerrainDatum = 2,
    /// The last single-point SidereonTerrainLookupError
    /// (sidereon_last_terrain_lookup_error).
    TerrainLookup = 3,
    /// The last standalone or nested geoid parsing error.
    Geoid = 4,
}

/// Which text of the last terrain error sidereon_last_terrain_error_text
/// copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainErrorText {
    /// The path of the tile or file.
    Path = 0,
    /// The I/O, field-reading or engine message.
    Message = 1,
    /// The store parse reason.
    Reason = 2,
    /// The remediation for a missing EGM96 grid.
    Remediation = 3,
    /// The invalid geoid input field.
    Field = 4,
}

thread_local! {
    static LAST_TERRAIN_ERROR_TEXTS: RefCell<[Vec<(u32, String)>; 5]> =
        const { RefCell::new([Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()]) };
}

pub(crate) fn record_terrain_error_texts(
    family: SidereonTerrainErrorFamily,
    texts: Vec<(u32, String)>,
) {
    LAST_TERRAIN_ERROR_TEXTS.with(|slot| slot.borrow_mut()[family as usize] = texts);
}

/// Copy one text of the last terrain error of `family` (a
/// SidereonTerrainErrorFamily value) recorded on this thread: `part` is a
/// SidereonTerrainErrorText value. The bytes are copied whole, not
/// null-terminated, under the variable-length output contract: call once with
/// out=NULL and len 0 to learn *out_required, then again with a buffer of that
/// size. A text the error does not carry copies nothing.
///
/// Safety: out points to len writable bytes or is NULL when len is 0;
/// out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_terrain_error_text(
    family: u32,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_last_terrain_error_text";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        if family > SidereonTerrainErrorFamily::Geoid as u32 {
            set_last_error(format!("{fn_name}: unknown family {family}"));
            return SidereonStatus::InvalidArgument;
        }
        if part > SidereonTerrainErrorText::Field as u32 {
            set_last_error(format!("{fn_name}: unknown text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_TERRAIN_ERROR_TEXTS.with(|slot| {
            slot.borrow()[family as usize]
                .iter()
                .find(|(kind, _)| *kind == part)
                .map(|(_, text)| text.clone())
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Bytes of a DTED horizontal datum text and its terminator. The DSI datum
/// field is five bytes; decoded lossily it is at most fifteen bytes of UTF-8.
pub const SIDEREON_DTED_DATUM_TEXT_C_BYTES: usize = 65;

/// Bytes of a terrain field name or short field text and its terminator.
pub const SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES: usize = 32;

// --- DTED horizontal datum and typed terrain lookup errors -------------------

/// Horizontal datum a DTED tile's DSI record states (DSI character 145).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonDtedHorizontalDatum {
    /// `WGS84`, the datum MIL-PRF-89020B 3.2.1 requires.
    Wgs84 = 0,
    /// `WGS72`, stated by cells compiled on the earlier World Geodetic System.
    Wgs72 = 1,
    /// The field is blank or zero-filled; read as WGS84.
    Unstated = 2,
    /// Any other field content; the text carries it as read.
    Other = 3,
    /// A datum a later engine names that this binding has no code for yet;
    /// the text carries the engine's value (variant and content) whole.
    Unknown = 999,
}

/// A DTED horizontal datum with the field text of an Other datum.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonDtedHorizontalDatumValue {
    /// Which datum the field states.
    pub kind: SidereonDtedHorizontalDatum,
    /// For Other, the DSI field as read (lossily decoded if not UTF-8); for
    /// Unknown, the engine's value; null-terminated, empty for every other
    /// kind. The DSI field is 5 bytes; an engine value longer than 64 bytes
    /// is cut to 64.
    pub text: [c_char; SIDEREON_DTED_DATUM_TEXT_C_BYTES],
    /// Whether positions in a tile on this datum are WGS84 positions: true for
    /// Wgs84 and Unstated. Terrain lookups and the terrain-store converter
    /// refuse a tile for which this is false.
    pub wgs84_compatible: bool,
}

/// Which terrain lookup failure a lookup reported.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainLookupErrorKind {
    /// No lookup failure is recorded.
    None = 0,
    /// A coordinate is not finite or is outside the lookup domain.
    InvalidInput = 1,
    /// No tile of the terrain store covers the query. Carries the tile.
    MissingTile = 2,
    /// The lookup gives nonzero weight to a posting holding the DTED null
    /// value (all bits set, MIL-PRF-89020B 3.11.3.1), an unknown elevation, and
    /// no neighbouring tile knows the height at the same place. Carries the
    /// tile and the posting.
    UnknownElevation = 3,
    /// The tile states a horizontal datum other than WGS84, so it cannot
    /// answer a WGS84 query without a datum transformation, which the terrain
    /// readers do not perform. Carries the tile and the datum.
    NonWgs84Tile = 4,
    /// A tile could not be read. The engine's text is in the message and in
    /// the MESSAGE text.
    Parse = 5,
    /// A tile file could not be read as a DTED tile, or a lookup in it failed
    /// other than on a null posting. Carries the tile and, in tile_error, the
    /// typed tile failure; its path and message are the PATH and MESSAGE texts.
    Tile = 6,
    /// A tile file states an origin other than the one-degree cell its name
    /// gives. Carries the tile the name gives, the origin the file states
    /// (origin_latitude_deg, origin_longitude_deg) and the file as the PATH
    /// text.
    TileOrigin = 7,
    /// Another engine failure. Its text is in the message and in the MESSAGE
    /// text.
    Other = 999,
}

/// Typed detail of a failed terrain lookup. Only the fields the kind names
/// carry meaning, and each group carries a present flag.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainLookupError {
    /// Which failure the lookup reported.
    pub kind: SidereonTerrainLookupErrorKind,
    /// Whether lat_index and lon_index name a tile.
    pub has_tile: bool,
    /// Integer latitude tile id.
    pub lat_index: i32,
    /// Integer longitude tile id.
    pub lon_index: i32,
    /// Whether latitude_posting and longitude_posting name a posting.
    pub has_posting: bool,
    /// Zero-based latitude posting index of the null posting in the tile.
    pub latitude_posting: usize,
    /// Zero-based longitude posting (profile) index of the null posting.
    pub longitude_posting: usize,
    /// Whether horizontal_datum carries the datum the tile states.
    pub has_horizontal_datum: bool,
    /// The datum a NonWgs84Tile tile states.
    pub horizontal_datum: SidereonDtedHorizontalDatumValue,
    /// Whether origin_latitude_deg and origin_longitude_deg carry the origin a
    /// TileOrigin tile file states.
    pub has_origin: bool,
    /// Origin latitude the tile file states, whole degrees.
    pub origin_latitude_deg: i32,
    /// Origin longitude the tile file states, whole degrees.
    pub origin_longitude_deg: i32,
    /// Whether tile_error carries the typed failure of a Tile lookup.
    pub has_tile_error: bool,
    /// The typed tile failure when kind is Tile; its kind is None otherwise.
    pub tile_error: SidereonDtedTileError,
}

thread_local! {
    static LAST_TERRAIN_LOOKUP_ERROR: RefCell<Option<SidereonTerrainLookupError>> =
        const { RefCell::new(None) };
}

pub(crate) fn dted_horizontal_datum_to_c(
    datum: &sidereon_core::terrain::DtedHorizontalDatum,
) -> SidereonDtedHorizontalDatumValue {
    use sidereon_core::terrain::DtedHorizontalDatum as Core;
    let (kind, text) = match datum {
        Core::Wgs84 => (SidereonDtedHorizontalDatum::Wgs84, String::new()),
        Core::Wgs72 => (SidereonDtedHorizontalDatum::Wgs72, String::new()),
        Core::Unstated => (SidereonDtedHorizontalDatum::Unstated, String::new()),
        Core::Other(text) => (SidereonDtedHorizontalDatum::Other, text.clone()),
        // `DtedHorizontalDatum` is non-exhaustive: a datum a later engine names
        // reads as Unknown, the text carrying its value as the engine holds it.
        other => (SidereonDtedHorizontalDatum::Unknown, format!("{other:?}")),
    };
    SidereonDtedHorizontalDatumValue {
        kind,
        text: fixed_c_chars(&text),
        wgs84_compatible: datum.is_wgs84_compatible(),
    }
}

fn no_dted_horizontal_datum() -> SidereonDtedHorizontalDatumValue {
    SidereonDtedHorizontalDatumValue {
        kind: SidereonDtedHorizontalDatum::Unstated,
        text: [0; SIDEREON_DTED_DATUM_TEXT_C_BYTES],
        wgs84_compatible: false,
    }
}

pub(crate) fn no_terrain_lookup_error() -> SidereonTerrainLookupError {
    SidereonTerrainLookupError {
        kind: SidereonTerrainLookupErrorKind::None,
        has_tile: false,
        lat_index: 0,
        lon_index: 0,
        has_posting: false,
        latitude_posting: 0,
        longitude_posting: 0,
        has_horizontal_datum: false,
        horizontal_datum: no_dted_horizontal_datum(),
        has_origin: false,
        origin_latitude_deg: 0,
        origin_longitude_deg: 0,
        has_tile_error: false,
        tile_error: crate::dted::no_dted_tile_error(),
    }
}

/// Map every field of an engine terrain lookup failure into the C record.
pub(crate) fn terrain_lookup_error_to_c(err: &CoreError) -> SidereonTerrainLookupError {
    let mut out = no_terrain_lookup_error();
    match err {
        CoreError::InvalidInput(_) => out.kind = SidereonTerrainLookupErrorKind::InvalidInput,
        CoreError::Parse(_) => out.kind = SidereonTerrainLookupErrorKind::Parse,
        CoreError::TerrainTile {
            lat_index,
            lon_index,
            error,
        } => {
            out.kind = SidereonTerrainLookupErrorKind::Tile;
            out.has_tile = true;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
            out.has_tile_error = true;
            out.tile_error = crate::dted::dted_tile_error_to_c(error).0;
        }
        CoreError::TerrainTileOrigin {
            lat_index,
            lon_index,
            origin_latitude,
            origin_longitude,
            ..
        } => {
            out.kind = SidereonTerrainLookupErrorKind::TileOrigin;
            out.has_tile = true;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
            out.has_origin = true;
            out.origin_latitude_deg = *origin_latitude;
            out.origin_longitude_deg = *origin_longitude;
        }
        CoreError::MissingTerrainTile {
            lat_index,
            lon_index,
        } => {
            out.kind = SidereonTerrainLookupErrorKind::MissingTile;
            out.has_tile = true;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
        }
        CoreError::UnknownTerrainElevation {
            lat_index,
            lon_index,
            latitude_posting,
            longitude_posting,
        } => {
            out.kind = SidereonTerrainLookupErrorKind::UnknownElevation;
            out.has_tile = true;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
            out.has_posting = true;
            out.latitude_posting = *latitude_posting;
            out.longitude_posting = *longitude_posting;
        }
        CoreError::NonWgs84TerrainTile {
            lat_index,
            lon_index,
            datum,
        } => {
            out.kind = SidereonTerrainLookupErrorKind::NonWgs84Tile;
            out.has_tile = true;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
            out.has_horizontal_datum = true;
            out.horizontal_datum = dted_horizontal_datum_to_c(datum);
        }
        _ => out.kind = SidereonTerrainLookupErrorKind::Other,
    }
    out
}

/// The texts of an engine terrain lookup failure that its typed record does
/// not hold: a Tile failure's path and message (as its SidereonDtedTileError
/// texts), a TileOrigin file's path, and the engine's text of every failure
/// with no typed text (InvalidInput, Parse, Other).
pub(crate) fn terrain_lookup_error_texts(err: &CoreError) -> Vec<(u32, String)> {
    match err {
        CoreError::TerrainTile { error, .. } => crate::dted::dted_tile_error_to_c(error).1,
        CoreError::TerrainTileOrigin { path, .. } => vec![(
            SidereonTerrainErrorText::Path as u32,
            path.display().to_string(),
        )],
        CoreError::MissingTerrainTile { .. }
        | CoreError::UnknownTerrainElevation { .. }
        | CoreError::NonWgs84TerrainTile { .. } => Vec::new(),
        other => vec![(SidereonTerrainErrorText::Message as u32, other.to_string())],
    }
}

/// Record a failed single-point terrain lookup for
/// sidereon_last_terrain_lookup_error, its texts in the TerrainLookup family,
/// a Tile failure's typed tile error for sidereon_last_dted_tile_error, and
/// the thread-local message, and return the status every lookup failure maps
/// to.
pub(crate) fn record_terrain_lookup_error(fn_name: &str, err: &CoreError) -> SidereonStatus {
    let typed = terrain_lookup_error_to_c(err);
    LAST_TERRAIN_LOOKUP_ERROR.with(|slot| *slot.borrow_mut() = Some(typed));
    record_terrain_error_texts(
        SidereonTerrainErrorFamily::TerrainLookup,
        terrain_lookup_error_texts(err),
    );
    if let CoreError::TerrainTile { error, .. } = err {
        crate::dted::record_dted_tile_error(error);
    }
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

thread_local! {
    static LAST_TERRAIN_BATCH_TEXTS: RefCell<Vec<Vec<(u32, String)>>> =
        const { RefCell::new(Vec::new()) };
}

/// Replace the recorded per-row texts of the last terrain batch lookup on this
/// thread with one empty row per point.
pub(crate) fn reset_terrain_batch_texts(count: usize) {
    LAST_TERRAIN_BATCH_TEXTS.with(|slot| *slot.borrow_mut() = vec![Vec::new(); count]);
}

/// Record the texts of one failed row of the current terrain batch lookup.
pub(crate) fn record_terrain_batch_row_texts(row: usize, err: &CoreError) {
    LAST_TERRAIN_BATCH_TEXTS.with(|slot| {
        if let Some(texts) = slot.borrow_mut().get_mut(row) {
            *texts = terrain_lookup_error_texts(err);
        }
    });
}

/// Copy one text of one row's failure in the last terrain batch lookup on this
/// thread that wrote its rows (sidereon_dted_terrain_height_batch_m,
/// sidereon_mmap_terrain_height_batch or
/// sidereon_mmap_terrain_orthometric_height_batch): `row` indexes that call's
/// points and `part` is a SidereonTerrainErrorText value. A row's texts are
/// those its error record's kind names: a Tile failure's path and message, a
/// TileOrigin file's path, and the engine's MESSAGE text of an InvalidInput,
/// Parse or Other failure. A row that succeeded, or a text the row does not
/// carry, copies nothing. A row past the last batch's point count is refused
/// with SIDEREON_STATUS_INVALID_ARGUMENT. The bytes are copied whole, not
/// null-terminated, under the variable-length output contract.
///
/// Safety: out points to len writable bytes or is NULL when len is 0;
/// out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_terrain_batch_error_text(
    row: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_terrain_batch_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        if part > SidereonTerrainErrorText::Remediation as u32 {
            set_last_error(format!("{FN_NAME}: unknown text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_TERRAIN_BATCH_TEXTS.with(|slot| {
            slot.borrow().get(row).map(|texts| {
                texts
                    .iter()
                    .find(|(kind, _)| *kind == part)
                    .map(|(_, text)| text.clone())
                    .unwrap_or_default()
            })
        });
        let Some(text) = text else {
            set_last_error(format!(
                "{FN_NAME}: row {row} is past the last terrain batch's points"
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the last typed terrain lookup failure for this thread: the most recent
/// single-point DTED or memory-mappable terrain height lookup that failed. If
/// none is recorded, kind is SidereonTerrainLookupErrorKind::None. Batch
/// lookups carry each point's failure in its own result instead.
///
/// Safety: out_error must point to a SidereonTerrainLookupError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_terrain_lookup_error(
    out_error: *mut SidereonTerrainLookupError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_terrain_lookup_error";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out = LAST_TERRAIN_LOOKUP_ERROR
            .with(|slot| *slot.borrow())
            .unwrap_or_else(no_terrain_lookup_error);
        SidereonStatus::Ok
    })
}

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
    /// A caller-attested full-store checksum did not match the opened bytes.
    AttestedChecksumMismatch = 8,
    /// A tile index record names a tile id outside the coordinate domain:
    /// latitude ids lie in -90..=89 and longitude ids in -180..=179. Carries
    /// lat_index and lon_index.
    TileIdOutOfRange = 9,
    /// A tile index bound is not the edge of the one-degree cell its tile id
    /// names. Carries lat_index, lon_index and the index field name in field.
    TileBoundsMismatch = 10,
    /// A DTED input states a horizontal datum other than WGS84. The store
    /// records no datum and answers WGS84 queries, so the tile is refused
    /// rather than stored as if it were WGS84. Carries path and
    /// horizontal_datum.
    NonWgs84Tile = 11,
    /// A DTED input could not be read as a tile. Carries path and, in
    /// tile_error, the typed tile failure; the same failure is also recorded
    /// for sidereon_last_dted_tile_error, whose texts
    /// (SIDEREON_TERRAIN_ERROR_FAMILY_DTED_TILE) carry its path and message.
    Tile = 12,
    /// A terrain-store error added by a later engine version.
    Unknown = 999,
}

/// Last typed terrain store error for this thread.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTerrainStoreError {
    /// Error selector as SidereonTerrainStoreErrorKind.
    pub kind: u32,
    /// Unsupported version tag when kind is UnsupportedVersion.
    pub version: u16,
    /// Unsupported vertical datum tag when kind is UnsupportedDatum.
    pub tag: u8,
    /// Tile latitude id for duplicate-tile and checksum errors.
    pub lat_index: i32,
    /// Tile longitude id for duplicate-tile and checksum errors.
    pub lon_index: i32,
    /// Supplied tile id for TileIdMismatch.
    pub expected_tile_id: SidereonTerrainTileId,
    /// Parsed tile id for TileIdMismatch.
    pub found_tile_id: SidereonTerrainTileId,
    /// Expected checksum for checksum and attested-checksum errors.
    pub expected_checksum64: u64,
    /// Computed checksum for checksum and attested-checksum errors.
    pub found_checksum64: u64,
    /// Index field name for TileBoundsMismatch, NUL-terminated when present.
    pub field: [c_char; SIDEREON_TERRAIN_ERROR_FIELD_C_BYTES],
    /// Whether horizontal_datum carries the datum a NonWgs84Tile input states.
    pub has_horizontal_datum: bool,
    /// The datum a NonWgs84Tile input states.
    pub horizontal_datum: SidereonDtedHorizontalDatumValue,
    /// Whether tile_error carries the failure a Tile input reported.
    pub has_tile_error: bool,
    /// The typed tile failure when kind is Tile; its kind is None otherwise.
    pub tile_error: SidereonDtedTileError,
}

/// Integer terrain tile id used by explicit DTED tile-list store builders.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonTerrainTileId {
    /// Integer latitude tile id, e.g. 36 for a tile covering 36..37 degrees.
    pub lat_index: i32,
    /// Integer longitude tile id, e.g. -107 for a tile covering -107..-106.
    pub lon_index: i32,
}

/// Terrain datum conversion or geoid loading error kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTerrainDatumErrorKind {
    /// No terrain datum error is recorded for this thread.
    None = 0,
    /// Terrain lookup failed before datum conversion. The engine's text is the
    /// MESSAGE text, and the file of a Tile or TileOrigin lookup failure the
    /// PATH text.
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
    /// The typed terrain lookup failure when kind is Terrain; its kind is None
    /// for every other kind.
    pub terrain: SidereonTerrainLookupError,
    /// The typed geoid failure when kind is Geoid; its kind is None otherwise.
    pub geoid: crate::geoid::SidereonGeoidError,
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
    /// The typed failure when status is not OK; its kind is None for a height.
    /// A null posting the lookup weights is UnknownElevation, not a height.
    pub error: SidereonTerrainLookupError,
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
