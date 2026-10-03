use super::*;

/// The instant a C IONEX query epoch names: whole UTC seconds since J2000.
/// IONEX labels its maps in UT, and a UTC query is built as a parsed product's
/// map epochs are, so a query on a map label meets that map exactly. The
/// engine carries the query and the map epochs onto UTC before they meet.
pub(crate) fn ionex_epoch_from_j2000_seconds(seconds: i64) -> Instant {
    ionex_epoch_from_whole_second(TimeScale::Utc, seconds)
}
use std::ptr;

/// A parsed IONEX vertical-TEC product. Create with sidereon_ionex_parse and
/// release with sidereon_ionex_free.
pub struct SidereonIonex {
    pub(crate) inner: Ionex,
}

/// Policy applied when an IONEX slant-delay query lands outside product coverage.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexCoveragePolicy {
    /// Return a typed error when the query is outside coverage.
    Strict = 0,
    /// Hold the nearest map or grid edge and return a held status.
    Hold = 1,
}

/// Policy applied when an IONEX interpolation weights grid nodes marked non-available.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexMissingNodePolicy {
    /// Return an error when an interpolation cell weights non-available nodes.
    Strict = 0,
    /// Interpolate from available weighted nodes and maps, marking the result degraded.
    Renormalize = 1,
}

/// The factor an IONEX slant delay maps vertical TEC to the line of sight with.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexMappingPolicy {
    /// The factor the product's MAPPING FUNCTION record defines (COSZ applies 1/cos(z')).
    Declared = 0,
    /// The single-layer 1/cos(z') at the shell height, reporting any non-COSZ declaration.
    SingleLayer = 1,
}

/// Composite policies applied to an IONEX slant-delay query.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexSlantPolicy {
    /// One of SidereonIonexCoveragePolicy.
    pub coverage: u32,
    /// One of SidereonIonexMissingNodePolicy.
    pub missing_nodes: u32,
    /// One of SidereonIonexMappingPolicy.
    pub mapping: u32,
}

/// Construct the default composite IONEX slant-delay policy: Strict coverage,
/// Strict missing nodes, and SingleLayer mapping.
#[no_mangle]
pub extern "C" fn sidereon_ionex_slant_policy_default() -> SidereonIonexSlantPolicy {
    SidereonIonexSlantPolicy {
        coverage: SidereonIonexCoveragePolicy::Strict as u32,
        missing_nodes: SidereonIonexMissingNodePolicy::Strict as u32,
        mapping: SidereonIonexMappingPolicy::SingleLayer as u32,
    }
}

/// Construct a composite IONEX slant-delay policy with explicit numeric tags.
#[no_mangle]
pub extern "C" fn sidereon_ionex_slant_policy_init(
    coverage: u32,
    missing_nodes: u32,
    mapping: u32,
) -> SidereonIonexSlantPolicy {
    SidereonIonexSlantPolicy {
        coverage,
        missing_nodes,
        mapping,
    }
}

/// Construct a composite IONEX slant-delay policy from an explicit coverage policy,
/// using default Strict missing nodes and SingleLayer mapping.
#[no_mangle]
pub extern "C" fn sidereon_ionex_slant_policy_from_coverage(
    coverage: u32,
) -> SidereonIonexSlantPolicy {
    SidereonIonexSlantPolicy {
        coverage,
        missing_nodes: SidereonIonexMissingNodePolicy::Strict as u32,
        mapping: SidereonIonexMappingPolicy::SingleLayer as u32,
    }
}

/// IONEX coverage miss associated with a held slant-delay value.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexCoverageErrorKind {
    /// No coverage error is associated with the value.
    None = 0,
    /// Query epoch precedes the first map epoch.
    EpochBeforeFirstMap = 1,
    /// Query epoch follows the last map epoch.
    EpochAfterLastMap = 2,
    /// Pierce-point latitude is outside the latitude nodes.
    LatitudeOutOfRange = 3,
    /// Pierce-point longitude is outside the longitude nodes.
    LongitudeOutOfRange = 4,
}

/// What mapping function declaration an IONEX product carries.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexMappingDeclarationKind {
    /// The product carries a declared MAPPING FUNCTION record.
    Declared = 0,
    /// The product carries no MAPPING FUNCTION record.
    Absent = 1,
}

/// Standard mapping function code variants.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexMappingFunctionKind {
    /// NONE: no mapping function was used.
    NoMapping = 0,
    /// COSZ: 1/cos(z).
    CosZ = 1,
    /// QFAC: Q-factor.
    QFactor = 2,
    /// Another code, preserved verbatim; read the text with
    /// sidereon_ionex_header_get_mapping_function_code.
    Other = 3,
}

/// Which mapping function a product declares where single-layer 1/cos(z') was assumed.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexAssumedMappingKind {
    /// Product declares COSZ, matching the applied factor (nominal).
    None = 0,
    /// Product declares NONE: no mapping function was used.
    NoMapping = 1,
    /// Product declares QFAC.
    QFactor = 2,
    /// Product declares another custom code. A row of an owned result list
    /// carries its own copy of the text, readable with
    /// sidereon_ionex_slant_result_get_mapping_code; a scalar route leaves it
    /// on the live product header, readable with
    /// sidereon_ionex_header_get_mapping_function_code.
    Other = 3,
    /// Product carries no MAPPING FUNCTION record.
    Absent = 4,
}

/// Nodes of one map's interpolation cell that carry weight in a query and are non-available.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexMissingNodes {
    /// Whether any node was missing.
    pub has_missing: bool,
    /// The map's number, counting from 1.
    pub map_number: usize,
    /// Index in lat_nodes_deg of the cell's first node row.
    pub lat_index: usize,
    /// Index in lon_nodes_deg of the cell's first node column.
    pub lon_index: usize,
    /// Index in lon_nodes_deg of the cell's second node column.
    pub lon_index_next: usize,
    /// Missing flags in order: [lat][lon], [lat][lon_next], [lat+1][lon], [lat+1][lon_next].
    pub missing: [bool; 4],
}

/// The non-available nodes a slant-delay query weights on each weighted map.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexNodeGap {
    /// Whether any gap condition was recorded.
    pub has_gap: bool,
    /// Missing nodes on the earlier map.
    pub earlier: SidereonIonexMissingNodes,
    /// Missing nodes on the later map.
    pub later: SidereonIonexMissingNodes,
}

/// Detailed independent status conditions for a slant-delay evaluation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexSlantDelayStatus {
    /// True when neither held nor degraded (assumed mapping does not invalidate).
    pub is_valid: bool,
    /// Whether the value was produced by the explicit hold policy.
    pub has_held: bool,
    /// The coverage miss the hold policy held the value through, when has_held
    /// is true.
    pub coverage_error: SidereonIonexCoverageErrorKind,
    /// Whether the value was degraded by missing nodes.
    pub has_degraded: bool,
    /// Detailed node gap information when has_degraded is true.
    pub gap: SidereonIonexNodeGap,
    /// Whether single-layer mapping was assumed for a product declaring something else.
    pub has_assumed_mapping: bool,
    /// What the product declares, when has_assumed_mapping is true.
    pub assumed_mapping: SidereonIonexAssumedMappingKind,
}

/// IONEX slant-delay value plus independent coverage, node-gap, and mapping status.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonIonexSlantDelayEvaluation {
    /// Slant ionospheric group delay in meters.
    pub delay_m: f64,
    /// Detailed independent status conditions.
    pub status: SidereonIonexSlantDelayStatus,
}

/// One slant-delay query for batch evaluation.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonIonexSlantRequest {
    /// Receiver geodetic latitude, degrees.
    pub lat_deg: f64,
    /// Receiver geodetic longitude, degrees.
    pub lon_deg: f64,
    /// Satellite azimuth, degrees.
    pub azimuth_deg: f64,
    /// Satellite elevation above the local horizon, degrees.
    pub elevation_deg: f64,
    /// Query epoch, integer UTC seconds since J2000.
    pub epoch_j2000_s: i64,
    /// Carrier frequency in hertz.
    pub frequency_hz: f64,
}

/// One IONEX slant query with an exact scale-tagged epoch. The epoch uses the
/// same lossless representation as RINEX clock queries; see
/// `SidereonClockEpoch` and `sidereon_rinex_clock_epoch_from_nanos`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonIonexInstantSlantRequest {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub epoch: SidereonClockEpoch,
    pub frequency_hz: f64,
}

/// Typed cause when an exact IONEX query epoch cannot be carried onto UTC.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexEpochErrorKind {
    None = 0,
    NotWholeSecond = 1,
    FractionalUtcSecond = 2,
    NoExactUtcOffset = 3,
    InsertedLeapSecond = 4,
    BeforeIntegerLeapSeconds = 5,
    OutOfRange = 6,
    YearOutOfField = 7,
    Unknown = 999,
}

/// Typed IONEX epoch conversion detail returned by exact-time query functions.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexEpochError {
    pub kind: SidereonIonexEpochErrorKind,
    pub scale: u32,
    pub has_utc_j2000_s: bool,
    pub utc_j2000_s: i64,
}

/// Caller-owned row output for `sidereon_ionex_slant_delay_results_at_instants`.
/// When `epoch_error` is nonempty, `error` remains the no-error value because
/// epoch conversion is reported separately.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonIonexInstantSlantRowResult {
    pub is_ok: bool,
    pub status: SidereonStatus,
    pub evaluation: SidereonIonexSlantDelayEvaluation,
    pub error: SidereonIonexSlantError,
    pub epoch_error: SidereonIonexEpochError,
}

/// Owned rows for exact-time IONEX batch results. Each row retains its typed
/// outcome, core error text, and any custom mapping code after source handles
/// are freed.
pub struct SidereonIonexInstantSlantResultList {
    rows: Vec<IonexInstantSlantResultRow>,
}

struct IonexInstantSlantResultRow {
    view: SidereonIonexInstantSlantRowResult,
    message: String,
    mapping_code: String,
}

/// Why an IONEX product gives no slant delay under the requested policy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexSlantRefusalKind {
    /// No refusal is recorded.
    None = 0,
    /// The product's height maps give nodes different single-layer heights.
    VaryingHeights = 1,
    /// A height map gives a node's height as non-available.
    HeightNotAvailable = 2,
    /// Under the Declared mapping policy, the product's MAPPING FUNCTION
    /// defines no factor the slant delay applies.
    MappingFunction = 3,
    /// A refusal this binding does not yet name. The engine's own text remains
    /// available through the message accessor for the route that produced it.
    Unknown = 999,
}

/// Which engine failure a slant-delay evaluation reported.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexSlantErrorKind {
    /// No failure is recorded; the evaluation holds a value.
    None = 0,
    /// The receiver position, geometry or carrier frequency was rejected. The
    /// full engine text is available through the message accessor.
    InvalidInput = 1,
    /// The query lies outside the product's coverage under a strict policy.
    OutOfCoverage = 2,
    /// The interpolation weights nodes the product gives as non-available.
    NodesNotAvailable = 3,
    /// The product gives no slant delay under the requested policy.
    SlantUnavailable = 4,
    /// A failure this binding does not yet name. The engine's own text remains
    /// available through the message accessor.
    Unknown = 999,
}

/// Typed detail of one failed IONEX slant-delay evaluation.
///
/// Only the fields the kind names carry meaning. The text of an `Other`
/// mapping-function code is not in this structure: an owned result list keeps
/// its own copy for a failed and a successful row alike, readable with
/// sidereon_ionex_slant_result_get_mapping_code, while a scalar route leaves it
/// on the live product header.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonIonexSlantError {
    /// Which engine failure the evaluation reported.
    pub kind: SidereonIonexSlantErrorKind,
    /// The coverage miss, when kind is OutOfCoverage.
    pub coverage_error: SidereonIonexCoverageErrorKind,
    /// Whether gap carries the non-available nodes the query weighted.
    pub has_gap: bool,
    /// Indexed corner masks, when kind is NodesNotAvailable.
    pub gap: SidereonIonexNodeGap,
    /// Why the product gives no slant delay, when kind is SlantUnavailable.
    pub refusal: SidereonIonexSlantRefusalKind,
    /// Height map number, counting from 1, for a height-map refusal.
    pub refusal_map_number: usize,
    /// Latitude index of the height node, for a height-map refusal.
    pub refusal_lat_index: usize,
    /// Longitude index of the height node, for a height-map refusal.
    pub refusal_lon_index: usize,
    /// Whether the refusal names a MAPPING FUNCTION declaration.
    pub has_mapping_declaration: bool,
    /// What the product declares, for a mapping refusal.
    pub mapping_declaration: SidereonIonexMappingDeclarationKind,
    /// Whether the declaration names a mapping function code.
    pub has_mapping_function: bool,
    /// The declared mapping code, for a mapping refusal that names one.
    pub mapping_function: SidereonIonexMappingFunctionKind,
}

/// One row of an owned IONEX slant-delay result list.
///
/// `is_ok` selects which of the two payloads carries meaning: a successful row
/// reads `evaluation`, a failed row reads `status` and `error`. A failed row's
/// `evaluation` holds a NaN delay with `status.is_valid` false, so it can never
/// be mistaken for a nominal zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonIonexSlantRowResult {
    /// Whether the row holds a delay.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, or the status the failure maps to.
    pub status: SidereonStatus,
    /// Delay evaluation; meaningful when is_ok is true.
    pub evaluation: SidereonIonexSlantDelayEvaluation,
    /// Typed failure detail; meaningful when is_ok is false.
    pub error: SidereonIonexSlantError,
}

/// Standalone GPS broadcast Klobuchar ionospheric group delay in native units (meters).
///
/// Safety: `alpha` must point to four readable doubles; `beta` must point to
/// four readable doubles; `out_delay_m` must point to one writable, aligned
/// double that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_klobuchar_native(
    alpha: *const f64,
    beta: *const f64,
    lat_deg: f64,
    lon_deg: f64,
    az_deg: f64,
    el_deg: f64,
    t_gps_s: f64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_klobuchar_native", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_klobuchar_native",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let alpha = c_try!(require_slice(
            alpha,
            4,
            "sidereon_klobuchar_native",
            "alpha"
        ));
        let beta = c_try!(require_slice(beta, 4, "sidereon_klobuchar_native", "beta"));
        let params = KlobucharParams {
            alpha: [alpha[0], alpha[1], alpha[2], alpha[3]],
            beta: [beta[0], beta[1], beta[2], beta[3]],
        };
        let delay = match klobuchar_native(
            &params,
            lat_deg,
            lon_deg,
            az_deg,
            el_deg,
            t_gps_s,
            frequency_hz,
        ) {
            Ok(delay) => delay,
            Err(err) => return map_iono_error("sidereon_klobuchar_native", err),
        };
        *out_delay_m = delay;
        SidereonStatus::Ok
    })
}

/// Parse an IONEX vertical-TEC product from a byte buffer using strict parsing.
/// On success writes a newly owned handle to *out_ionex. Release with sidereon_ionex_free.
///
/// Safety: `data` must point to `len` readable bytes, or may be NULL when `len`
/// is 0; the bytes are copied during the call, no NUL terminator is read or
/// required, and a NUL among them is copied as data; `out_ionex` must point to
/// writable, aligned storage for one `SidereonIonex *`, which is set to NULL
/// before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_ionex_free. The output slot
/// must be disjoint from the `len` bytes at `data`: it is written while a Rust
/// reference to that byte range is live. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_parse(
    data: *const u8,
    len: usize,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_parse", SidereonStatus::Panic, || {
        let out_ionex = c_try!(require_out(out_ionex, "sidereon_ionex_parse", "out_ionex"));
        *out_ionex = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_ionex_parse", "data"));
        let inner = match Ionex::parse(bytes) {
            Ok(ionex) => ionex,
            Err(err) => return map_iono_error("sidereon_ionex_parse", err),
        };
        write_boxed_handle(out_ionex, SidereonIonex { inner });
        SidereonStatus::Ok
    })
}

/// An owned list of findings reported while reading an IONEX product without refusing the file.
pub struct SidereonIonexWarningList {
    pub(crate) warnings: Vec<IonexWarning>,
}

/// Kind of IONEX warning reported by parse_with_warnings.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonIonexWarningKind {
    /// A mandatory record was omitted from the header.
    MissingRecord = 0,
    /// IONEX VERSION / TYPE record was not the first record.
    VersionRecordNotFirst = 1,
    /// EPOCH OF FIRST MAP or EPOCH OF LAST MAP disagrees with actual map epochs.
    EpochMismatch = 2,
    /// # OF MAPS IN FILE disagrees with map counts.
    MapCountMismatch = 3,
    /// A data field contained 'nan' instead of valid numbers or 9999.
    NotANumberValue = 4,
    /// Nonzero INTERVAL disagrees with consecutive map spacing.
    IntervalMismatch = 5,
    /// An exponent from an earlier map was carried forward across map boundaries.
    ExponentCarriedIntoMap = 6,
    /// Future unmapped warning kind.
    Unknown = 999,
}

/// Structured details of an IONEX warning record.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonIonexWarningInfo {
    /// Which finding the reader reported.
    pub kind: SidereonIonexWarningKind,
    /// One-based line of the record in the source text.
    pub line: usize,
    /// Associated map number (1-based), where applicable.
    pub map_number: usize,
    /// Line number of the map that originally set the carried exponent.
    pub set_by_line: usize,
    /// Declared count (e.g. # OF MAPS IN FILE).
    pub declared_count: u64,
    /// Actual TEC map count.
    pub tec_map_count: usize,
    /// Total maps present in file.
    pub all_map_count: usize,
    /// Declared interval in seconds.
    pub declared_interval_s: u32,
    /// Actual spacing in seconds.
    pub actual_spacing_s: i64,
    /// Exponent in effect.
    pub exponent: i32,
    /// Node latitude, degrees.
    pub lat_deg: f64,
    /// Node longitude, degrees.
    pub lon_deg: f64,
    /// Whether the four epoch fields below carry epochs.
    pub has_epochs: bool,
    /// Declared epoch as seconds since J2000, when has_epochs is true: the
    /// whole second declared_epoch_j2000_whole_s holds, converted to a double
    /// once, so it is exact up to 2^53 seconds and the nearest double past it.
    pub declared_epoch_j2000_s: f64,
    /// Actual map epoch as seconds since J2000, when has_epochs is true,
    /// converted from maps_epoch_j2000_whole_s the same way.
    pub maps_epoch_j2000_s: f64,
    /// Declared epoch as whole seconds since J2000, exact at every magnitude,
    /// when has_epochs is true.
    pub declared_epoch_j2000_whole_s: i64,
    /// Actual map epoch as whole seconds since J2000, exact at every
    /// magnitude, when has_epochs is true.
    pub maps_epoch_j2000_whole_s: i64,
}

/// Parse an IONEX product while retaining all non-fatal parser warnings.
///
/// On failure no owned handles are transferred and every output this call can
/// write is NULL. Each output is cleared before the other is validated, so a
/// refusal for a null partner still leaves the writable one NULL rather than
/// whatever the caller left in it. The reader's own working memory is not the
/// subject of that promise: it allocates internal temporaries while it reads
/// the bytes and releases them before returning.
///
/// Safety: `data` must point to `len` readable bytes, or may be NULL when `len`
/// is 0; the bytes are copied during the call, no NUL terminator is read or
/// required, and a NUL among them is copied as data; `out_ionex` must point to
/// writable, aligned storage for one `SidereonIonex *`, which is set to NULL
/// before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_ionex_free; `out_warnings` must
/// point to writable, aligned storage for one `SidereonIonexWarningList *`,
/// which is set to NULL before any work and receives a newly owned handle only
/// on SIDEREON_STATUS_OK; release it with sidereon_ionex_warning_list_free. The
/// two output slots must be disjoint from each other and from the `len` bytes
/// at `data`: both are written while a Rust reference to that byte range is
/// live. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_parse_with_warnings(
    data: *const u8,
    len: usize,
    out_ionex: *mut *mut SidereonIonex,
    out_warnings: *mut *mut SidereonIonexWarningList,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_parse_with_warnings",
        SidereonStatus::Panic,
        || {
            // Both outputs are cleared before either is validated, the same
            // order the owned-result constructors use. Validating one first
            // would let a null argument return while the other output kept
            // whatever the caller left in it, which a caller checking for NULL
            // reads as a transferred handle.
            if !out_ionex.is_null() {
                *out_ionex = ptr::null_mut();
            }
            if !out_warnings.is_null() {
                *out_warnings = ptr::null_mut();
            }
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_parse_with_warnings",
                "out_ionex"
            ));
            let out_warnings = c_try!(require_out(
                out_warnings,
                "sidereon_ionex_parse_with_warnings",
                "out_warnings"
            ));
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_ionex_parse_with_warnings",
                "data"
            ));
            let (inner, warnings) = match Ionex::parse_with_warnings(bytes) {
                Ok(res) => res,
                Err(err) => return map_iono_error("sidereon_ionex_parse_with_warnings", err),
            };
            write_boxed_handle(out_ionex, SidereonIonex { inner });
            write_boxed_handle(out_warnings, SidereonIonexWarningList { warnings });
            SidereonStatus::Ok
        },
    )
}

/// Release an IONEX warning list handle. Passing NULL is a no-op.
///
/// Safety: `list` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonIonexWarningList handle this binding produced, and it must be passed
/// here exactly once. The handle and every pointer read out of it are invalid
/// afterwards. This call returns nothing, so it reports no status: a pointer
/// that is neither NULL nor such a handle cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_warning_list_free(list: *mut SidereonIonexWarningList) {
    ffi_boundary("sidereon_ionex_warning_list_free", (), || {
        free_boxed(list);
    });
}

/// Write the number of warnings in an IONEX warning list to *out_count.
///
/// Safety: `list` must be a live SidereonIonexWarningList handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// size_t that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_warning_list_count(
    list: *const SidereonIonexWarningList,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_warning_list_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_warning_list_count",
                "out_count"
            ));
            *out_count = 0;
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_warning_list_count",
                "list"
            ));
            *out_count = list.warnings.len();
            SidereonStatus::Ok
        },
    )
}

/// Read typed numeric fields and coordinates of warning record at index.
///
/// Safety: `list` must be a live SidereonIonexWarningList handle, not freed for
/// the duration of the call; `out_info` must point to one writable, aligned
/// SidereonIonexWarningInfo that no other argument aliases. A NULL argument
/// this contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER
/// rather than dereferenced; a non-null pointer that is not valid for the whole
/// call cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_warning_get_info(
    list: *const SidereonIonexWarningList,
    index: usize,
    out_info: *mut SidereonIonexWarningInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_warning_get_info",
        SidereonStatus::Panic,
        || {
            let out_info = c_try!(require_out(
                out_info,
                "sidereon_ionex_warning_get_info",
                "out_info"
            ));
            *out_info = zero_warning_info();
            let list = c_try!(require_ref(list, "sidereon_ionex_warning_get_info", "list"));
            if index >= list.warnings.len() {
                set_last_error(format!(
                    "sidereon_ionex_warning_get_info: index {index} out of range ({})",
                    list.warnings.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
            *out_info = warning_info_to_c(&list.warnings[index]);
            SidereonStatus::Ok
        },
    )
}

/// Copy the label or record identifier associated with a warning. Uses variable-length output.
///
/// Safety: `list` must be a live SidereonIonexWarningList handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_warning_get_label(
    list: *const SidereonIonexWarningList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_warning_get_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_warning_get_label",
                out_written,
                out_required
            ));
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_warning_get_label",
                "list"
            ));
            if index >= list.warnings.len() {
                set_last_error(format!(
                    "sidereon_ionex_warning_get_label: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            }
            let label = warning_label(&list.warnings[index]);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_warning_get_label",
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

/// Copy the complete formatted warning message. Uses variable-length output.
///
/// Safety: `list` must be a live SidereonIonexWarningList handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_warning_get_message(
    list: *const SidereonIonexWarningList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_warning_get_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_warning_get_message",
                out_written,
                out_required
            ));
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_warning_get_message",
                "list"
            ));
            if index >= list.warnings.len() {
                set_last_error(format!(
                    "sidereon_ionex_warning_get_message: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            }
            let message = format!("{}", list.warnings[index]);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_warning_get_message",
                "out",
                message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// IONEX vertical-TEC-grid slant ionospheric group delay (meters) under the
/// engine default policy: a query outside the product's coverage, one whose
/// interpolation weights a non-available node, and a product whose height maps
/// do not give every node one height all refuse with an error status and a
/// thread-local message. Angles are degrees; the epoch is integer UTC seconds
/// since J2000; the delay is positive meters.
///
/// `*out_delay_m` is set to 0.0 before any other argument is read and keeps
/// 0.0 on every refusal, so a caller reading it must check the status first:
/// 0.0 is also what a zero-TEC product gives on success. Use
/// sidereon_ionex_slant_delay_with_policy for a refused value that is NaN and
/// carries a typed reason.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_delay_m` must point to one writable, aligned
/// double that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_slant_delay", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_ionex_slant_delay",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let request = IonexSlantDelayCRequest {
            lat_deg,
            lon_deg,
            azimuth_deg,
            elevation_deg,
            epoch_j2000_s,
            frequency_hz,
        };
        let evaluation = match ionex_slant_delay_eval_from_c(
            "sidereon_ionex_slant_delay",
            ionex,
            request,
            IonexSlantPolicy::default(),
        ) {
            Ok(evaluation) => evaluation,
            Err(failure) => return failure.status,
        };
        *out_delay_m = evaluation.delay_m;
        SidereonStatus::Ok
    })
}

/// IONEX slant delay under an explicit composite policy, reporting the held,
/// degraded and assumed-mapping conditions independently. The epoch is integer
/// UTC seconds since J2000, as for sidereon_ionex_slant_delay.
///
/// `out_error` may be NULL. When it is not, a refusal fills it with the typed
/// detail of the engine failure and `out` keeps the refused placeholder, whose
/// delay is NaN and whose status is not valid. The text of an `Other` mapping
/// code named by a mapping refusal stays on the product: read it with
/// sidereon_ionex_header_get_mapping_function_code while the handle lives.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` must point to one writable, aligned
/// SidereonIonexSlantDelayEvaluation that no other argument aliases;
/// `out_error` may be NULL, which discards the detail; otherwise it must point
/// to one writable, aligned SidereonIonexSlantError that no other argument
/// aliases. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_with_policy(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
    policy: SidereonIonexSlantPolicy,
    out: *mut SidereonIonexSlantDelayEvaluation,
    out_error: *mut SidereonIonexSlantError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_delay_with_policy",
        SidereonStatus::Panic,
        || {
            // Both outputs are cleared before either is validated, so a null
            // `out` still leaves a writable `out_error` holding no failure
            // rather than whatever the caller left in it.
            if !out.is_null() {
                *out = refused_ionex_slant_delay_evaluation();
            }
            if !out_error.is_null() {
                *out_error = no_ionex_slant_error();
            }
            let out = c_try!(require_out(
                out,
                "sidereon_ionex_slant_delay_with_policy",
                "out"
            ));
            let core_policy = c_try!(ionex_slant_policy_from_c(
                "sidereon_ionex_slant_delay_with_policy",
                policy
            ));
            let request = IonexSlantDelayCRequest {
                lat_deg,
                lon_deg,
                azimuth_deg,
                elevation_deg,
                epoch_j2000_s,
                frequency_hz,
            };
            match ionex_slant_delay_eval_from_c(
                "sidereon_ionex_slant_delay_with_policy",
                ionex,
                request,
                core_policy,
            ) {
                Ok(evaluation) => {
                    *out = ionex_slant_delay_evaluation_to_c(evaluation);
                    SidereonStatus::Ok
                }
                Err(failure) => {
                    if !out_error.is_null() {
                        *out_error = failure.detail;
                    }
                    failure.status
                }
            }
        },
    )
}

/// Evaluate a slant delay at a lossless scale-tagged instant. This additive
/// route preserves fractional seconds and reports the seven core IONEX epoch
/// conversion causes through `out_epoch_error`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_at_instant_with_policy(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch: *const SidereonClockEpoch,
    frequency_hz: f64,
    policy: SidereonIonexSlantPolicy,
    out: *mut SidereonIonexSlantDelayEvaluation,
    out_error: *mut SidereonIonexSlantError,
    out_epoch_error: *mut SidereonIonexEpochError,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_slant_delay_at_instant_with_policy";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        if !out.is_null() {
            *out = refused_ionex_slant_delay_evaluation();
        }
        if !out_error.is_null() {
            *out_error = no_ionex_slant_error();
        }
        if !out_epoch_error.is_null() {
            *out_epoch_error = no_ionex_epoch_error();
        }
        let out = c_try!(require_out(out, FN, "out"));
        let epoch = c_try!(require_ref(epoch, FN, "epoch"));
        let instant = c_try!(clock_epoch_from_c(FN, "epoch", epoch));
        let policy = c_try!(ionex_slant_policy_from_c(FN, policy));
        let ionex = c_try!(require_ref(ionex, FN, "ionex"));
        let receiver = c_try!(geodetic_to_wgs84(
            FN,
            "receiver",
            SidereonGeodetic {
                lat_rad: lat_deg * IONO_DEG_TO_RAD,
                lon_rad: lon_deg * IONO_DEG_TO_RAD,
                height_m: 0.0,
            },
        ));
        match ionex_slant_delay_with_policy(
            &ionex.inner,
            receiver,
            elevation_deg * IONO_DEG_TO_RAD,
            azimuth_deg * IONO_DEG_TO_RAD,
            instant,
            frequency_hz,
            policy,
        ) {
            Ok(evaluation) => {
                *out = ionex_slant_delay_evaluation_to_c(evaluation);
                SidereonStatus::Ok
            }
            Err(error) => {
                if let sidereon_core::Error::IonexEpoch(epoch_error) = &error {
                    if !out_epoch_error.is_null() {
                        *out_epoch_error = ionex_epoch_error_to_c(epoch_error);
                    }
                } else if !out_error.is_null() {
                    *out_error = ionex_slant_error_to_c(&error).0;
                }
                map_iono_error(FN, error)
            }
        }
    })
}

/// Evaluate a slant delay at an exact scale-tagged instant under the engine's
/// default policy. Epoch conversion causes are available through `out_epoch_error`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_at_instant(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch: *const SidereonClockEpoch,
    frequency_hz: f64,
    out_delay_m: *mut f64,
    out_epoch_error: *mut SidereonIonexEpochError,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_slant_delay_at_instant";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        if !out_epoch_error.is_null() {
            *out_epoch_error = no_ionex_epoch_error();
        }
        let out = c_try!(require_out(out_delay_m, FN, "out_delay_m"));
        *out = 0.0;
        let mut evaluation = refused_ionex_slant_delay_evaluation();
        let status = sidereon_ionex_slant_delay_at_instant_with_policy(
            ionex,
            lat_deg,
            lon_deg,
            azimuth_deg,
            elevation_deg,
            epoch,
            frequency_hz,
            sidereon_ionex_slant_policy_default(),
            &mut evaluation,
            ptr::null_mut(),
            out_epoch_error,
        );
        if status == SidereonStatus::Ok {
            *out = evaluation.delay_m;
        }
        status
    })
}

/// Evaluate exact-time IONEX requests into caller-owned, input-ordered rows.
/// Each row keeps its own typed epoch failure; no absolute epoch is reduced to
/// a floating-point seconds value before the core query.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_results_at_instants(
    ionex: *const SidereonIonex,
    requests: *const SidereonIonexInstantSlantRequest,
    count: usize,
    policy: SidereonIonexSlantPolicy,
    out_rows: *mut SidereonIonexInstantSlantRowResult,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_slant_delay_results_at_instants";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(require_out_slice(out_rows, count, FN, "out_rows"));
        if count != 0 && !ionex.is_null() {
            let output = c_try!(checked_output_range(FN, out_rows, count, "out_rows"));
            let product = c_try!(checked_output_range(
                FN,
                ionex as *mut SidereonIonex,
                1,
                "ionex"
            ));
            c_try!(reject_overlapping_outputs(
                FN, output, product, "out_rows", "ionex"
            ));
        }
        let requests_result =
            require_slice(requests, count, FN, "requests").map(|rows| rows.to_vec());
        for idx in 0..count {
            out_rows.add(idx).write(SidereonIonexInstantSlantRowResult {
                is_ok: false,
                status: SidereonStatus::InvalidArgument,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
                epoch_error: no_ionex_epoch_error(),
            });
        }
        let rows = if count == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(out_rows, count)
        };
        let ionex = c_try!(require_ref(ionex, FN, "ionex"));
        let requests = c_try!(requests_result);
        let policy = c_try!(ionex_slant_policy_from_c(FN, policy));
        for (request, row) in requests.iter().zip(rows.iter_mut()) {
            let instant = match clock_epoch_from_c(FN, "request.epoch", &request.epoch) {
                Ok(instant) => instant,
                Err(status) => {
                    row.status = status;
                    continue;
                }
            };
            let receiver = match Wgs84Geodetic::new(
                request.lat_deg * IONO_DEG_TO_RAD,
                request.lon_deg * IONO_DEG_TO_RAD,
                0.0,
            ) {
                Ok(receiver) => receiver,
                Err(error) => {
                    set_last_error(format!("{FN}: receiver: {error}"));
                    row.status = SidereonStatus::InvalidArgument;
                    row.error = invalid_input_error();
                    continue;
                }
            };
            match ionex_slant_delay_with_policy(
                &ionex.inner,
                receiver,
                request.elevation_deg * IONO_DEG_TO_RAD,
                request.azimuth_deg * IONO_DEG_TO_RAD,
                instant,
                request.frequency_hz,
                policy,
            ) {
                Ok(evaluation) => {
                    row.is_ok = true;
                    row.status = SidereonStatus::Ok;
                    row.evaluation = ionex_slant_delay_evaluation_to_c(evaluation);
                }
                Err(error) => {
                    if let sidereon_core::Error::IonexEpoch(epoch_error) = &error {
                        row.epoch_error = ionex_epoch_error_to_c(epoch_error);
                    } else {
                        row.error = ionex_slant_error_to_c(&error).0;
                    }
                    row.status = map_iono_error(FN, error);
                }
            }
        }
        SidereonStatus::Ok
    })
}

/// Evaluate exact-time IONEX requests into an independently owned result
/// list. Each row retains the complete error text and any custom mapping code;
/// an epoch-conversion failure appears in `epoch_error` and leaves `error` at
/// its no-error value. Release the returned list with
/// `sidereon_ionex_instant_slant_result_list_free`.
///
/// # Safety
/// `ionex` must be a live handle; `requests` must point to `count` readable,
/// aligned requests or be NULL when `count` is zero; `out_results` must point
/// to one writable, aligned handle slot disjoint from the input handles and
/// request slice.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_results_at_instants_owned(
    ionex: *const SidereonIonex,
    requests: *const SidereonIonexInstantSlantRequest,
    count: usize,
    policy: SidereonIonexSlantPolicy,
    out_results: *mut *mut SidereonIonexInstantSlantResultList,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_slant_delay_results_at_instants_owned";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_results = c_try!(require_out(out_results, FN, "out_results"));
        *out_results = ptr::null_mut();
        let ionex = c_try!(require_ref(ionex, FN, "ionex"));
        let requests = c_try!(require_slice(requests, count, FN, "requests"));
        let policy = c_try!(ionex_slant_policy_from_c(FN, policy));
        let declared_other_code = match &ionex.inner.header().mapping_function {
            Some(IonexMappingFunction::Other(code)) => code.clone(),
            _ => String::new(),
        };
        let mut rows = Vec::with_capacity(count);
        for request in requests {
            let instant = match clock_epoch_from_c(FN, "request.epoch", &request.epoch) {
                Ok(instant) => instant,
                Err(status) => {
                    rows.push(IonexInstantSlantResultRow {
                        view: SidereonIonexInstantSlantRowResult {
                            is_ok: false,
                            status,
                            evaluation: refused_ionex_slant_delay_evaluation(),
                            error: invalid_input_error(),
                            epoch_error: no_ionex_epoch_error(),
                        },
                        message: current_last_error_text(),
                        mapping_code: String::new(),
                    });
                    continue;
                }
            };
            let receiver = match Wgs84Geodetic::new(
                request.lat_deg * IONO_DEG_TO_RAD,
                request.lon_deg * IONO_DEG_TO_RAD,
                0.0,
            ) {
                Ok(receiver) => receiver,
                Err(error) => {
                    set_last_error(format!("{FN}: receiver: {error}"));
                    rows.push(IonexInstantSlantResultRow {
                        view: SidereonIonexInstantSlantRowResult {
                            is_ok: false,
                            status: SidereonStatus::InvalidArgument,
                            evaluation: refused_ionex_slant_delay_evaluation(),
                            error: invalid_input_error(),
                            epoch_error: no_ionex_epoch_error(),
                        },
                        message: format!("receiver: {error}"),
                        mapping_code: String::new(),
                    });
                    continue;
                }
            };
            match ionex_slant_delay_with_policy(
                &ionex.inner,
                receiver,
                request.elevation_deg * IONO_DEG_TO_RAD,
                request.azimuth_deg * IONO_DEG_TO_RAD,
                instant,
                request.frequency_hz,
                policy,
            ) {
                Ok(evaluation) => {
                    let mapping_code = if evaluation.status.assumed_mapping
                        == Some(sidereon_core::atmosphere::ionosphere::IonexAssumedMapping::Other)
                    {
                        declared_other_code.clone()
                    } else {
                        String::new()
                    };
                    rows.push(IonexInstantSlantResultRow {
                        view: SidereonIonexInstantSlantRowResult {
                            is_ok: true,
                            status: SidereonStatus::Ok,
                            evaluation: ionex_slant_delay_evaluation_to_c(evaluation),
                            error: no_ionex_slant_error(),
                            epoch_error: no_ionex_epoch_error(),
                        },
                        message: String::new(),
                        mapping_code,
                    });
                }
                Err(error) => {
                    let (detail, mapping_code) =
                        if matches!(&error, sidereon_core::Error::IonexEpoch(_)) {
                            (no_ionex_slant_error(), String::new())
                        } else {
                            ionex_slant_error_to_c(&error)
                        };
                    let epoch_error = match &error {
                        sidereon_core::Error::IonexEpoch(error) => ionex_epoch_error_to_c(error),
                        _ => no_ionex_epoch_error(),
                    };
                    let status = map_iono_error(FN, error.clone());
                    rows.push(IonexInstantSlantResultRow {
                        view: SidereonIonexInstantSlantRowResult {
                            is_ok: false,
                            status,
                            evaluation: refused_ionex_slant_delay_evaluation(),
                            error: detail,
                            epoch_error,
                        },
                        message: error.to_string(),
                        mapping_code,
                    });
                }
            }
        }
        write_boxed_handle(out_results, SidereonIonexInstantSlantResultList { rows });
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Release an exact-time result list. Passing NULL is a no-op.
///
/// # Safety
/// A non-NULL pointer must be a live handle returned by this binding and must
/// be passed exactly once.
pub unsafe extern "C" fn sidereon_ionex_instant_slant_result_list_free(
    list: *mut SidereonIonexInstantSlantResultList,
) {
    ffi_boundary("sidereon_ionex_instant_slant_result_list_free", (), || {
        free_boxed(list);
    });
}

#[no_mangle]
/// Write the row count of a live exact-time result list.
///
/// # Safety
/// `list` must be live and `out_count` must point to one writable, aligned
/// `usize` that does not alias the list.
pub unsafe extern "C" fn sidereon_ionex_instant_slant_result_list_count(
    list: *const SidereonIonexInstantSlantResultList,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_instant_slant_result_list_count";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN, "out_count"));
        *out_count = 0;
        let list = c_try!(require_ref(list, FN, "list"));
        *out_count = list.rows.len();
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Copy one typed row from an exact-time result list.
///
/// # Safety
/// `list` must be live and `out_row` must point to one writable, aligned row
/// that does not alias the list.
pub unsafe extern "C" fn sidereon_ionex_instant_slant_result_get_row(
    list: *const SidereonIonexInstantSlantResultList,
    index: usize,
    out_row: *mut SidereonIonexInstantSlantRowResult,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_instant_slant_result_get_row";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_row = c_try!(require_out(out_row, FN, "out_row"));
        *out_row = SidereonIonexInstantSlantRowResult {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            evaluation: refused_ionex_slant_delay_evaluation(),
            error: no_ionex_slant_error(),
            epoch_error: no_ionex_epoch_error(),
        };
        let list = c_try!(require_ref(list, FN, "list"));
        let Some(row) = list.rows.get(index) else {
            set_last_error(format!(
                "{FN}: index {index} is out of range ({})",
                list.rows.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_row = row.view;
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Copy one owned row message using the standard variable-length byte output
/// contract.
///
/// # Safety
/// `list` must be live. Buffer and count pointers must satisfy
/// `copy_prefix_to_c`'s non-aliasing writable-storage contract.
pub unsafe extern "C" fn sidereon_ionex_instant_slant_result_get_message(
    list: *const SidereonIonexInstantSlantResultList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_instant_slant_result_get_message";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let list = c_try!(require_ref(list, FN, "list"));
        let Some(row) = list.rows.get(index) else {
            set_last_error(format!(
                "{FN}: index {index} is out of range ({})",
                list.rows.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            row.message.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
/// Copy one owned custom mapping code using the standard variable-length byte
/// output contract.
///
/// # Safety
/// `list` must be live. Buffer and count pointers must satisfy
/// `copy_prefix_to_c`'s non-aliasing writable-storage contract.
pub unsafe extern "C" fn sidereon_ionex_instant_slant_result_get_mapping_code(
    list: *const SidereonIonexInstantSlantResultList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_ionex_instant_slant_result_get_mapping_code";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let list = c_try!(require_ref(list, FN, "list"));
        let Some(row) = list.rows.get(index) else {
            set_last_error(format!(
                "{FN}: index {index} is out of range ({})",
                list.rows.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            row.mapping_code.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn current_last_error_text() -> String {
    unsafe {
        let required = sidereon_last_error_message(ptr::null_mut(), 0);
        let mut bytes = vec![0 as c_char; required + 1];
        sidereon_last_error_message(bytes.as_mut_ptr(), bytes.len());
        std::ffi::CStr::from_ptr(bytes.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

pub(crate) fn no_ionex_epoch_error() -> SidereonIonexEpochError {
    SidereonIonexEpochError {
        kind: SidereonIonexEpochErrorKind::None,
        scale: SidereonTimeScale::Utc as u32,
        has_utc_j2000_s: false,
        utc_j2000_s: 0,
    }
}

pub(crate) fn ionex_epoch_error_to_c(
    error: &sidereon_core::atmosphere::ionosphere::IonexEpochError,
) -> SidereonIonexEpochError {
    use sidereon_core::atmosphere::ionosphere::IonexEpochError as E;
    let mut result = no_ionex_epoch_error();
    match error {
        E::NotWholeSecond { scale } => {
            result.kind = SidereonIonexEpochErrorKind::NotWholeSecond;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::FractionalUtcSecond { scale } => {
            result.kind = SidereonIonexEpochErrorKind::FractionalUtcSecond;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::NoExactUtcOffset { scale } => {
            result.kind = SidereonIonexEpochErrorKind::NoExactUtcOffset;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::InsertedLeapSecond { scale } => {
            result.kind = SidereonIonexEpochErrorKind::InsertedLeapSecond;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::BeforeIntegerLeapSeconds { scale } => {
            result.kind = SidereonIonexEpochErrorKind::BeforeIntegerLeapSeconds;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::OutOfRange { scale } => {
            result.kind = SidereonIonexEpochErrorKind::OutOfRange;
            result.scale = time_scale_to_c_code(*scale);
        }
        E::YearOutOfField { utc_j2000_s } => {
            result.kind = SidereonIonexEpochErrorKind::YearOutOfField;
            result.scale = SidereonTimeScale::Utc as u32;
            result.has_utc_j2000_s = true;
            result.utc_j2000_s = *utc_j2000_s;
        }
        _ => result.kind = SidereonIonexEpochErrorKind::Unknown,
    }
    result
}

/// IONEX slant delay under an explicit coverage policy tag, with the engine
/// default strict missing-node policy and single-layer mapping.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` must point to one writable, aligned
/// SidereonIonexSlantDelayEvaluation that no other argument aliases;
/// `out_error` may be NULL, which discards the detail; otherwise it must point
/// to one writable, aligned SidereonIonexSlantError that no other argument
/// aliases. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_with_coverage_policy(
    ionex: *const SidereonIonex,
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
    coverage_policy: u32,
    out: *mut SidereonIonexSlantDelayEvaluation,
    out_error: *mut SidereonIonexSlantError,
) -> SidereonStatus {
    let policy = sidereon_ionex_slant_policy_from_coverage(coverage_policy);
    sidereon_ionex_slant_delay_with_policy(
        ionex,
        lat_deg,
        lon_deg,
        azimuth_deg,
        elevation_deg,
        epoch_j2000_s,
        frequency_hz,
        policy,
        out,
        out_error,
    )
}

/// Batch IONEX slant delays into a caller-allocated array of `count` doubles
/// under the engine default policy.
///
/// This is the plain convenience form: it allocates nothing, and the first row
/// the engine refuses fails the whole call, leaving every output element zero.
/// Use sidereon_ionex_slant_delay_results when each row's own outcome matters.
///
/// The output array is validated and zeroed before any other argument is read,
/// so whenever `out_delays_m` and `count` together describe storage this call
/// can write, that storage holds `count` zeroes on every failure the call
/// reports: a null `ionex`, a null `requests` with a nonzero `count`, a row
/// the marshalling refuses, and a batch the engine refuses alike.
///
/// `out_delays_m` is the one argument that storage promise cannot cover. When
/// it is NULL with a nonzero `count` the call returns
/// SIDEREON_STATUS_NULL_POINTER, and when `count` exceeds the elements one
/// slice can span the call returns SIDEREON_STATUS_INVALID_ARGUMENT. Neither
/// names storage to initialize, so neither writes anything and the caller's
/// array keeps whatever was in it.
///
/// Safety: `out_delays_m` must point to `count` writable, aligned doubles that
/// overlap neither `requests` nor the product behind `ionex`, or may be NULL
/// when `count` is 0; `ionex` must be a live SidereonIonex handle, not freed
/// for the duration of the call; `requests` must point to `count` readable,
/// aligned SidereonIonexSlantRequest values, or may be NULL when `count` is 0.
/// A NULL argument this contract does not allow is refused
/// with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null
/// pointer that is not valid for the whole call cannot be checked and is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delays(
    ionex: *const SidereonIonex,
    requests: *const SidereonIonexSlantRequest,
    count: usize,
    out_delays_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_slant_delays", SidereonStatus::Panic, || {
        // The writable array is validated and cleared before the read-only
        // arguments are looked at. Checking `ionex` or `requests` first would
        // return on a null one while a perfectly good output buffer still held
        // whatever the caller left in it, which is not what this route's plain
        // convenience contract promises.
        c_try!(require_out_slice(
            out_delays_m,
            count,
            "sidereon_ionex_slant_delays",
            "out_delays_m"
        ));
        if count != 0 && !ionex.is_null() {
            let output = c_try!(checked_output_range(
                "sidereon_ionex_slant_delays",
                out_delays_m,
                count,
                "out_delays_m",
            ));
            let product = c_try!(checked_output_range(
                "sidereon_ionex_slant_delays",
                ionex as *mut SidereonIonex,
                1,
                "ionex",
            ));
            c_try!(reject_overlapping_outputs(
                "sidereon_ionex_slant_delays",
                output,
                product,
                "out_delays_m",
                "ionex",
            ));
        }
        let requests_result =
            require_slice(requests, count, "sidereon_ionex_slant_delays", "requests")
                .map(|rows| rows.to_vec());
        for idx in 0..count {
            out_delays_m.add(idx).write(0.0);
        }
        let out_slice = if count == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(out_delays_m, count)
        };
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_slant_delays", "ionex"));
        let req_slice = c_try!(requests_result);
        let core_requests = c_try!(convert_slant_requests(
            "sidereon_ionex_slant_delays",
            &req_slice
        ));
        match ionex_slant_delays(&ionex.inner, &core_requests, out_slice) {
            Ok(()) => SidereonStatus::Ok,
            Err(err) => {
                for out in out_slice.iter_mut() {
                    *out = 0.0;
                }
                map_iono_error("sidereon_ionex_slant_delays", err)
            }
        }
    })
}

/// An owned, ordered list of IONEX slant-delay results, one per input row.
///
/// The list owns every string it reports, so a row stays readable after the
/// product and header handles it came from are freed.
pub struct SidereonIonexSlantResultList {
    pub(crate) rows: Vec<IonexSlantResultRow>,
}

/// One stored row of an owned slant-delay result list.
pub(crate) struct IonexSlantResultRow {
    pub(crate) view: SidereonIonexSlantRowResult,
    /// The engine's own text for a failed row; empty for a successful one.
    pub(crate) message: String,
    /// The custom `Other` mapping code this row names: the one a mapping
    /// refusal reported, or the one a successful row assumed the single-layer
    /// factor over. Empty for any other row.
    pub(crate) mapping_code: String,
}

/// Evaluate a batch of IONEX slant delays into a newly owned result list.
///
/// Every input row appears in the list in its input order, whether the engine
/// gave it a value or refused it, including a row whose receiver geometry or
/// carrier frequency the engine rejects. The call itself fails only on a
/// malformed argument: a null product, a null request array with a nonzero
/// count, an unrecognized policy tag, or a null output pointer. Release the
/// list with sidereon_ionex_slant_result_list_free.
///
/// The list takes its own copy of every string a row reports, including the
/// custom mapping code a successful row assumed the single-layer factor over,
/// which the engine keeps only on the product header. A row is therefore
/// complete once the call returns and stays complete after the product and
/// header are freed.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `requests` must point to `count` readable, aligned
/// SidereonIonexSlantRequest values, or may be NULL when `count` is 0;
/// `out_results` must point to writable, aligned storage for one
/// `SidereonIonexSlantResultList *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_ionex_slant_result_list_free. The output slot must be disjoint
/// from the `count` requests and from the product behind `ionex`: it is
/// written while Rust references to both are live. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_delay_results(
    ionex: *const SidereonIonex,
    requests: *const SidereonIonexSlantRequest,
    count: usize,
    policy: SidereonIonexSlantPolicy,
    out_results: *mut *mut SidereonIonexSlantResultList,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_delay_results",
        SidereonStatus::Panic,
        || {
            let out_results = c_try!(require_out(
                out_results,
                "sidereon_ionex_slant_delay_results",
                "out_results"
            ));
            *out_results = ptr::null_mut();
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_slant_delay_results",
                "ionex"
            ));
            let req_slice = c_try!(require_slice(
                requests,
                count,
                "sidereon_ionex_slant_delay_results",
                "requests"
            ));
            let core_policy = c_try!(ionex_slant_policy_from_c(
                "sidereon_ionex_slant_delay_results",
                policy
            ));

            // IonexAssumedMapping::Other names the case without carrying the
            // code's text, which the engine keeps only in
            // IonexHeader::mapping_function. A successful row under the
            // single-layer factor would otherwise reach that text only through
            // the live product, so the list takes its own copy here and a row
            // keeps its mapping context after the product and header are freed.
            let declared_other_code = match &ionex.inner.header().mapping_function {
                Some(IonexMappingFunction::Other(code)) => code.clone(),
                _ => String::new(),
            };

            // A row whose receiver geometry the engine rejects has no core
            // request to evaluate, so the rows are marshalled first and the
            // engine batch runs over the ones that marshalled. Both kinds of
            // row keep their input position.
            let mut marshalled: Vec<bool> = Vec::with_capacity(count);
            let mut accepted: Vec<IonexSlantRequest> = Vec::new();
            let mut rejected: Vec<IonexSlantResultRow> = Vec::new();
            for request in req_slice {
                match slant_request_to_core("sidereon_ionex_slant_delay_results", request) {
                    Ok(core_request) => {
                        accepted.push(core_request);
                        marshalled.push(true);
                    }
                    Err(message) => {
                        rejected.push(refused_row(
                            SidereonStatus::InvalidArgument,
                            invalid_input_error(),
                            message,
                            String::new(),
                        ));
                        marshalled.push(false);
                    }
                }
            }

            let evaluated = ionex_slant_delay_results(&ionex.inner, &accepted, core_policy);
            let mut evaluated = evaluated.into_iter();
            let mut refused = rejected.into_iter();
            let mut ordered: Vec<IonexSlantResultRow> = Vec::with_capacity(count);
            for accepted_row in marshalled {
                let row = if accepted_row {
                    match evaluated.next() {
                        Some(Ok(evaluation)) => {
                            let view = SidereonIonexSlantRowResult {
                                is_ok: true,
                                status: SidereonStatus::Ok,
                                evaluation: ionex_slant_delay_evaluation_to_c(evaluation),
                                error: no_ionex_slant_error(),
                            };
                            let status = view.evaluation.status;
                            let mapping_code = if status.has_assumed_mapping
                                && status.assumed_mapping == SidereonIonexAssumedMappingKind::Other
                            {
                                declared_other_code.clone()
                            } else {
                                String::new()
                            };
                            IonexSlantResultRow {
                                view,
                                message: String::new(),
                                mapping_code,
                            }
                        }
                        Some(Err(err)) => {
                            let (detail, mapping_code) = ionex_slant_error_to_c(&err);
                            refused_row(
                                ionex_error_status(&err),
                                detail,
                                err.to_string(),
                                mapping_code,
                            )
                        }
                        // The engine gives one result per request it was
                        // handed, so this names a contract break rather than a
                        // reachable state. It still fills the row instead of
                        // dropping it, so the list stays one row per input.
                        None => refused_row(
                            SidereonStatus::Solve,
                            unknown_ionex_slant_error(),
                            "sidereon_ionex_slant_delay_results: the engine returned fewer \
                             results than requests"
                                .to_owned(),
                            String::new(),
                        ),
                    }
                } else {
                    match refused.next() {
                        Some(row) => row,
                        None => refused_row(
                            SidereonStatus::Solve,
                            unknown_ionex_slant_error(),
                            "sidereon_ionex_slant_delay_results: a rejected row lost its detail"
                                .to_owned(),
                            String::new(),
                        ),
                    }
                };
                ordered.push(row);
            }

            write_boxed_handle(out_results, SidereonIonexSlantResultList { rows: ordered });
            SidereonStatus::Ok
        },
    )
}

/// Release an IONEX slant-delay result list. Passing NULL is a no-op.
///
/// Safety: `list` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonIonexSlantResultList handle this binding produced, and it must be
/// passed here exactly once. The handle and every pointer read out of it are
/// invalid afterwards. This call returns nothing, so it reports no status: a
/// pointer that is neither NULL nor such a handle cannot be checked and is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_result_list_free(
    list: *mut SidereonIonexSlantResultList,
) {
    ffi_boundary("sidereon_ionex_slant_result_list_free", (), || {
        free_boxed(list);
    });
}

/// Write the number of rows in an IONEX slant-delay result list to *out_count.
///
/// Safety: `list` must be a live SidereonIonexSlantResultList handle, not freed
/// for the duration of the call; `out_count` must point to one writable,
/// aligned size_t that no other argument aliases. A NULL argument this contract
/// does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_result_list_count(
    list: *const SidereonIonexSlantResultList,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_result_list_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_slant_result_list_count",
                "out_count"
            ));
            *out_count = 0;
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_slant_result_list_count",
                "list"
            ));
            *out_count = list.rows.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the row at `index` of an IONEX slant-delay result list.
///
/// Safety: `list` must be a live SidereonIonexSlantResultList handle, not freed
/// for the duration of the call; `out_row` must point to one writable, aligned
/// SidereonIonexSlantRowResult that no other argument aliases. A NULL argument
/// this contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER
/// rather than dereferenced; a non-null pointer that is not valid for the whole
/// call cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_result_get_row(
    list: *const SidereonIonexSlantResultList,
    index: usize,
    out_row: *mut SidereonIonexSlantRowResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_result_get_row",
        SidereonStatus::Panic,
        || {
            let out_row = c_try!(require_out(
                out_row,
                "sidereon_ionex_slant_result_get_row",
                "out_row"
            ));
            *out_row = refused_row_view(SidereonStatus::InvalidArgument, no_ionex_slant_error());
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_slant_result_get_row",
                "list"
            ));
            let Some(row) = list.rows.get(index) else {
                set_last_error(format!(
                    "sidereon_ionex_slant_result_get_row: index {index} out of range ({})",
                    list.rows.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_row = row.view;
            SidereonStatus::Ok
        },
    )
}

/// Copy the engine's own text for the row at `index`. A successful row reports
/// a required length of zero. Uses the variable-length output contract.
///
/// Safety: `list` must be a live SidereonIonexSlantResultList handle, not freed
/// for the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_result_get_message(
    list: *const SidereonIonexSlantResultList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_result_get_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_slant_result_get_message",
                out_written,
                out_required
            ));
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_slant_result_get_message",
                "list"
            ));
            let Some(row) = list.rows.get(index) else {
                set_last_error(format!(
                    "sidereon_ionex_slant_result_get_message: index {index} out of range ({})",
                    list.rows.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_slant_result_get_message",
                "out",
                row.message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the MAPPING FUNCTION code the row at `index` names, as the product
/// wrote it. The list owns this text, so it survives the product and header
/// handles and any number of later failing calls. Uses the variable-length
/// output contract.
///
/// Two kinds of row name a code. A failed row whose refusal is
/// SIDEREON_IONEX_SLANT_REFUSAL_KIND_MAPPING_FUNCTION reports the code the
/// declaration carries. A successful row whose status has has_assumed_mapping
/// with assumed_mapping SIDEREON_IONEX_ASSUMED_MAPPING_KIND_OTHER reports the
/// custom code the product declared while the single-layer factor was applied:
/// that kind names the case without carrying its text, so this is the only
/// place the text outlives the product. Any other row reports a required length
/// of zero.
///
/// A zero required length is therefore not by itself an absent declaration, and
/// the row's typed fields keep the three cases apart. A failed row separates
/// them with mapping_declaration: Declared with has_mapping_function names a
/// code, Absent names no record at all. A successful row separates them with
/// assumed_mapping: Other names a custom code, Absent names no record, and
/// NoMapping or QFactor name a standard code this accessor does not repeat.
///
/// A blank custom code is not excluded. sidereon_ionex_header_set_mapping_function
/// accepts SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER with a zero-length code,
/// a product built from that header keeps it, and a single-layer evaluation
/// reports it, so a successful row can report Other with a required length of
/// zero exactly as an Absent row does. Only serialization refuses a blank code;
/// that refusal is sidereon_ionex_to_ionex_text's and does not reach
/// construction or evaluation. Read assumed_mapping, not the length, to tell
/// an empty custom code from no declaration at all.
///
/// Safety: `list` must be a live SidereonIonexSlantResultList handle, not freed
/// for the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_slant_result_get_mapping_code(
    list: *const SidereonIonexSlantResultList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_slant_result_get_mapping_code",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_slant_result_get_mapping_code",
                out_written,
                out_required
            ));
            let list = c_try!(require_ref(
                list,
                "sidereon_ionex_slant_result_get_mapping_code",
                "list"
            ));
            let Some(row) = list.rows.get(index) else {
                set_last_error(format!(
                    "sidereon_ionex_slant_result_get_mapping_code: index {index} out of range ({})",
                    list.rows.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_slant_result_get_mapping_code",
                "out",
                row.mapping_code.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the number of TEC map epochs in the product to *out_count.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_count` must point to one writable, aligned size_t
/// that no other argument aliases. A NULL argument this contract does not allow
/// is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_epoch_count(
    ionex: *const SidereonIonex,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_epoch_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_ionex_epoch_count",
            "out_count"
        ));
        *out_count = 0;
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_epoch_count", "ionex"));
        *out_count = ionex.inner.map_epochs_s().len();
        SidereonStatus::Ok
    })
}

/// Write the IONEX EXPONENT header field to *out_exponent.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_exponent` must point to one writable, aligned
/// int32_t that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_exponent(
    ionex: *const SidereonIonex,
    out_exponent: *mut i32,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_exponent", SidereonStatus::Panic, || {
        let out_exponent = c_try!(require_out(
            out_exponent,
            "sidereon_ionex_exponent",
            "out_exponent"
        ));
        *out_exponent = 0;
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_exponent", "ionex"));
        *out_exponent = ionex.inner.exponent();
        SidereonStatus::Ok
    })
}

/// Copy the latitude node axis in degrees, in the product's own order, which
/// may run north to south or south to north. Uses variable-length output contract.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_lat_nodes_deg(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_lat_nodes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_lat_nodes_deg",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_lat_nodes_deg", "ionex"));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_lat_nodes_deg",
                "out",
                ionex.inner.lat_nodes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the longitude node axis in degrees, in the product's own order, which
/// may run either way. Uses variable-length output contract.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_lon_nodes_deg(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_lon_nodes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_lon_nodes_deg",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_lon_nodes_deg", "ionex"));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_lon_nodes_deg",
                "out",
                ionex.inner.lon_nodes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the TEC map epoch axis as seconds since J2000. Uses variable-length output contract.
///
/// Each value is the exact whole second the engine reads the map epoch as, at
/// every magnitude an int64_t holds; this is the integer form of
/// sidereon_ionex_tec_grid_samples_epochs_j2000_s.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned int64_t values that do not overlap the handle being read.
/// `len` counts int64_t values, not bytes. `out_written` and `out_required`
/// must each point to one writable, aligned size_t, must alias neither each
/// other nor `out`, and are both set to 0 before anything else is read. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_map_epochs_j2000_s(
    ionex: *const SidereonIonex,
    out: *mut i64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_map_epochs_j2000_s",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_map_epochs_j2000_s",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_map_epochs_j2000_s",
                "ionex"
            ));
            let epochs = ionex.inner.map_epochs_s();
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_map_epochs_j2000_s",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Serialize an IONEX product back to IONEX text. Fallible: propagates errors if values
/// cannot be formatted into standard columns. Uses variable-length output contract.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned bytes that do not overlap the handle being read. `len` is
/// the buffer size in bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. The bytes are copied
/// verbatim: no NUL terminator is appended, so a caller wanting a C string must
/// allocate one more byte and write it. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_to_ionex_text(
    ionex: *const SidereonIonex,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_to_ionex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_to_ionex_text",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(ionex, "sidereon_ionex_to_ionex_text", "ionex"));
            let text = match ionex.inner.to_ionex_string() {
                Ok(text) => text,
                Err(err) => return map_iono_error("sidereon_ionex_to_ionex_text", err),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_to_ionex_text",
                "out",
                text.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release an IONEX product handle. Passing NULL is a no-op.
///
/// Safety: `ionex` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonIonex handle this binding produced, and it must be passed here
/// exactly once. The handle and every pointer read out of it are invalid
/// afterwards. This call returns nothing, so it reports no status: a pointer
/// that is neither NULL nor such a handle cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_free(ionex: *mut SidereonIonex) {
    ffi_boundary("sidereon_ionex_free", (), || {
        free_boxed(ionex);
    });
}

// --- IONEX header handle and typed accessors --------------------------------

/// An owned IONEX header handle holding descriptive records and metadata.
pub struct SidereonIonexHeader {
    pub(crate) inner: IonexHeader,
}

/// Extract a cloned copy of an IONEX product's header into a newly owned handle.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_header` must point to writable, aligned storage
/// for one `SidereonIonexHeader *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_ionex_header_free. The output slot must be disjoint from the
/// product's own storage: it is written while a Rust reference to the product
/// is live. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_get_header(
    ionex: *const SidereonIonex,
    out_header: *mut *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_get_header", SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(
            out_header,
            "sidereon_ionex_get_header",
            "out_header"
        ));
        *out_header = ptr::null_mut();
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_get_header", "ionex"));
        write_boxed_handle(
            out_header,
            SidereonIonexHeader {
                inner: ionex.inner.header().clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// The header a caller who supplied none gets: every record at the value the
/// spec gives for an unstated one, and no `MAPPING FUNCTION` record.
///
/// The core constructor `IonexHeader::new` takes the declared mapping function
/// and its unstated-record constructor is crate-private, so the declaration is
/// cleared through `IonexHeader::mapping_function`, the same field
/// `sidereon_ionex_header_set_mapping_function` and
/// `sidereon_ionex_header_clear_mapping_function` write. A product built on
/// this header declares nothing: it evaluates a slant query under
/// `IonexAssumedMapping::Absent`, is refused under
/// `SIDEREON_IONEX_MAPPING_POLICY_DECLARED`, and writes back no
/// `MAPPING FUNCTION` record.
fn undeclared_ionex_header() -> IonexHeader {
    let mut header = IonexHeader::new(IonexMappingFunction::CosZ);
    header.mapping_function = None;
    header
}

/// Create a new IONEX header handle declaring no mapping function.
///
/// Every record sits at the value the spec gives for an unstated one, and the
/// handle carries no `MAPPING FUNCTION` record. A product built on it reports
/// `has_assumed_mapping` true with kind
/// `SIDEREON_IONEX_ASSUMED_MAPPING_KIND_ABSENT` on a successful slant row, is
/// refused with `SIDEREON_IONEX_MAPPING_POLICY_DECLARED`, and writes back no
/// `MAPPING FUNCTION` record. Call
/// `sidereon_ionex_header_set_mapping_function` to declare one.
///
/// Safety: `out_header` must point to writable, aligned storage for one
/// `SidereonIonexHeader *`, which is set to NULL before any work and receives a
/// newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_ionex_header_free. A NULL argument this contract does not allow is
/// refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_new(
    out_header: *mut *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_header_new", SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(
            out_header,
            "sidereon_ionex_header_new",
            "out_header"
        ));
        *out_header = ptr::null_mut();
        write_boxed_handle(
            out_header,
            SidereonIonexHeader {
                inner: undeclared_ionex_header(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Clone an IONEX header into a new owned handle.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_header` must point to writable, aligned
/// storage for one `SidereonIonexHeader *`, which is set to NULL before any
/// work and receives a newly owned handle only on SIDEREON_STATUS_OK; release
/// it with sidereon_ionex_header_free. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_clone(
    header: *const SidereonIonexHeader,
    out_header: *mut *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_header_clone", SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(
            out_header,
            "sidereon_ionex_header_clone",
            "out_header"
        ));
        *out_header = ptr::null_mut();
        let header = c_try!(require_ref(header, "sidereon_ionex_header_clone", "header"));
        write_boxed_handle(
            out_header,
            SidereonIonexHeader {
                inner: header.inner.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Release an IONEX header handle. Passing NULL is a no-op.
///
/// Safety: `header` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonIonexHeader handle this binding produced, and it must be passed here
/// exactly once. The handle and every pointer read out of it are invalid
/// afterwards. This call returns nothing, so it reports no status: a pointer
/// that is neither NULL nor such a handle cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_free(header: *mut SidereonIonexHeader) {
    ffi_boundary("sidereon_ionex_header_free", (), || {
        free_boxed(header);
    });
}

/// Read the format version from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_version` must point to one writable, aligned
/// double that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_version(
    header: *const SidereonIonexHeader,
    out_version: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_version",
        SidereonStatus::Panic,
        || {
            let out_version = c_try!(require_out(
                out_version,
                "sidereon_ionex_header_get_version",
                "out_version"
            ));
            *out_version = 0.0;
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_version",
                "header"
            ));
            *out_version = header.inner.version;
            SidereonStatus::Ok
        },
    )
}

/// Assign the format version on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_version(
    header: *mut SidereonIonexHeader,
    version: f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_version",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_version",
                "header"
            ));
            header.inner.version = version;
            SidereonStatus::Ok
        },
    )
}

/// Copy the satellite system string from an IONEX header. Uses variable-length output.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_satellite_system(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_satellite_system",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_satellite_system",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_satellite_system",
                "header"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_satellite_system",
                "out",
                header.inner.satellite_system.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign the satellite system string on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_satellite_system(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_satellite_system",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_satellite_system",
                "header"
            ));
            header.inner.satellite_system = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_set_satellite_system",
                text,
                len
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the program name from an IONEX header. Uses variable-length output.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_program(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_program",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_program",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_program",
                "header"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_program",
                "out",
                header.inner.program.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign the program name on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_program(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_program",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_program",
                "header"
            ));
            header.inner.program = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_set_program",
                text,
                len
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the agency / run_by name from an IONEX header. Uses variable-length output.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_run_by(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_run_by",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_run_by",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_run_by",
                "header"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_run_by",
                "out",
                header.inner.run_by.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign the agency / run_by name on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_run_by(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_run_by",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_run_by",
                "header"
            ));
            header.inner.run_by = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_set_run_by",
                text,
                len
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the creation date string from an IONEX header. Uses variable-length output.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_date(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_date",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_date",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_date",
                "header"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_date",
                "out",
                header.inner.date.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign the creation date string on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_date(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_date",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_date",
                "header"
            ));
            header.inner.date = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_set_date",
                text,
                len
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy observables used description from an IONEX header. Uses variable-length output.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_observables_used(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_observables_used",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_observables_used",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_observables_used",
                "header"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_observables_used",
                "out",
                header.inner.observables_used.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign observables used description on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_observables_used(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_observables_used",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_observables_used",
                "header"
            ));
            header.inner.observables_used = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_set_observables_used",
                text,
                len
            ));
            SidereonStatus::Ok
        },
    )
}

/// Read interval between maps in seconds from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_interval_s` must point to one writable,
/// aligned uint32_t that no other argument aliases. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_interval_s(
    header: *const SidereonIonexHeader,
    out_interval_s: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_interval_s",
        SidereonStatus::Panic,
        || {
            let out_interval_s = c_try!(require_out(
                out_interval_s,
                "sidereon_ionex_header_get_interval_s",
                "out_interval_s"
            ));
            *out_interval_s = 0;
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_interval_s",
                "header"
            ));
            *out_interval_s = header.inner.interval_s;
            SidereonStatus::Ok
        },
    )
}

/// Assign interval between maps in seconds on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_interval_s(
    header: *mut SidereonIonexHeader,
    interval_s: u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_interval_s",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_interval_s",
                "header"
            ));
            header.inner.interval_s = interval_s;
            SidereonStatus::Ok
        },
    )
}

/// Read elevation cutoff in degrees from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_cutoff_deg` must point to one writable,
/// aligned double that no other argument aliases. A NULL argument this contract
/// does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_elevation_cutoff_deg(
    header: *const SidereonIonexHeader,
    out_cutoff_deg: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_elevation_cutoff_deg",
        SidereonStatus::Panic,
        || {
            let out_cutoff_deg = c_try!(require_out(
                out_cutoff_deg,
                "sidereon_ionex_header_get_elevation_cutoff_deg",
                "out_cutoff_deg"
            ));
            *out_cutoff_deg = 0.0;
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_elevation_cutoff_deg",
                "header"
            ));
            *out_cutoff_deg = header.inner.elevation_cutoff_deg;
            SidereonStatus::Ok
        },
    )
}

/// Assign elevation cutoff in degrees on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_elevation_cutoff_deg(
    header: *mut SidereonIonexHeader,
    cutoff_deg: f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_elevation_cutoff_deg",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_elevation_cutoff_deg",
                "header"
            ));
            header.inner.elevation_cutoff_deg = cutoff_deg;
            SidereonStatus::Ok
        },
    )
}

/// Read optional station count from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// uint32_t that no other argument aliases; `out_has_count` must point to one
/// writable, aligned bool that no other argument aliases. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_station_count(
    header: *const SidereonIonexHeader,
    out_count: *mut u32,
    out_has_count: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_station_count",
        SidereonStatus::Panic,
        || {
            if !out_count.is_null() && !out_has_count.is_null() {
                let outputs = [
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_station_count",
                            out_count,
                            1,
                            "out_count",
                        )),
                        "out_count",
                    )),
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_station_count",
                            out_has_count,
                            1,
                            "out_has_count",
                        )),
                        "out_has_count",
                    )),
                ];
                c_try!(reject_overlapping_optional_outputs(
                    "sidereon_ionex_header_get_station_count",
                    &outputs
                ));
            }
            // Both outputs are cleared before either is validated, so a null
            // partner never leaves the other holding the caller's stale value.
            if !out_count.is_null() {
                *out_count = 0;
            }
            if !out_has_count.is_null() {
                *out_has_count = false;
            }
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_header_get_station_count",
                "out_count"
            ));
            let out_has_count = c_try!(require_out(
                out_has_count,
                "sidereon_ionex_header_get_station_count",
                "out_has_count"
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_station_count",
                "header"
            ));
            if let Some(count) = header.inner.station_count {
                *out_count = count;
                *out_has_count = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Assign or clear optional station count on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_station_count(
    header: *mut SidereonIonexHeader,
    count: u32,
    has_count: bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_station_count",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_station_count",
                "header"
            ));
            header.inner.station_count = has_count.then_some(count);
            SidereonStatus::Ok
        },
    )
}

/// Read optional satellite count from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// uint32_t that no other argument aliases; `out_has_count` must point to one
/// writable, aligned bool that no other argument aliases. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_satellite_count(
    header: *const SidereonIonexHeader,
    out_count: *mut u32,
    out_has_count: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_satellite_count",
        SidereonStatus::Panic,
        || {
            if !out_count.is_null() && !out_has_count.is_null() {
                let outputs = [
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_satellite_count",
                            out_count,
                            1,
                            "out_count",
                        )),
                        "out_count",
                    )),
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_satellite_count",
                            out_has_count,
                            1,
                            "out_has_count",
                        )),
                        "out_has_count",
                    )),
                ];
                c_try!(reject_overlapping_optional_outputs(
                    "sidereon_ionex_header_get_satellite_count",
                    &outputs
                ));
            }
            // Both outputs are cleared before either is validated, so a null
            // partner never leaves the other holding the caller's stale value.
            if !out_count.is_null() {
                *out_count = 0;
            }
            if !out_has_count.is_null() {
                *out_has_count = false;
            }
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_header_get_satellite_count",
                "out_count"
            ));
            let out_has_count = c_try!(require_out(
                out_has_count,
                "sidereon_ionex_header_get_satellite_count",
                "out_has_count"
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_satellite_count",
                "header"
            ));
            if let Some(count) = header.inner.satellite_count {
                *out_count = count;
                *out_has_count = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Assign or clear optional satellite count on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_satellite_count(
    header: *mut SidereonIonexHeader,
    count: u32,
    has_count: bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_satellite_count",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_satellite_count",
                "header"
            ));
            header.inner.satellite_count = has_count.then_some(count);
            SidereonStatus::Ok
        },
    )
}

/// Read optional maps_in_file count from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// uint32_t that no other argument aliases; `out_has_count` must point to one
/// writable, aligned bool that no other argument aliases. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_maps_in_file(
    header: *const SidereonIonexHeader,
    out_count: *mut u32,
    out_has_count: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_maps_in_file",
        SidereonStatus::Panic,
        || {
            if !out_count.is_null() && !out_has_count.is_null() {
                let outputs = [
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_maps_in_file",
                            out_count,
                            1,
                            "out_count",
                        )),
                        "out_count",
                    )),
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_maps_in_file",
                            out_has_count,
                            1,
                            "out_has_count",
                        )),
                        "out_has_count",
                    )),
                ];
                c_try!(reject_overlapping_optional_outputs(
                    "sidereon_ionex_header_get_maps_in_file",
                    &outputs
                ));
            }
            // Both outputs are cleared before either is validated, so a null
            // partner never leaves the other holding the caller's stale value.
            if !out_count.is_null() {
                *out_count = 0;
            }
            if !out_has_count.is_null() {
                *out_has_count = false;
            }
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_header_get_maps_in_file",
                "out_count"
            ));
            let out_has_count = c_try!(require_out(
                out_has_count,
                "sidereon_ionex_header_get_maps_in_file",
                "out_has_count"
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_maps_in_file",
                "header"
            ));
            if let Some(count) = header.inner.maps_in_file {
                *out_count = count;
                *out_has_count = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Assign or clear optional maps_in_file count on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_maps_in_file(
    header: *mut SidereonIonexHeader,
    count: u32,
    has_count: bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_maps_in_file",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_maps_in_file",
                "header"
            ));
            header.inner.maps_in_file = has_count.then_some(count);
            SidereonStatus::Ok
        },
    )
}

/// Read mapping declaration status and function variant from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_decl_kind` must point to one writable,
/// aligned SidereonIonexMappingDeclarationKind that no other argument aliases;
/// `out_func_kind` must point to one writable, aligned
/// SidereonIonexMappingFunctionKind that no other argument aliases. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_mapping_declaration(
    header: *const SidereonIonexHeader,
    out_decl_kind: *mut SidereonIonexMappingDeclarationKind,
    out_func_kind: *mut SidereonIonexMappingFunctionKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_mapping_declaration",
        SidereonStatus::Panic,
        || {
            if !out_decl_kind.is_null() && !out_func_kind.is_null() {
                let outputs = [
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_mapping_declaration",
                            out_decl_kind,
                            1,
                            "out_decl_kind",
                        )),
                        "out_decl_kind",
                    )),
                    Some((
                        c_try!(checked_output_range(
                            "sidereon_ionex_header_get_mapping_declaration",
                            out_func_kind,
                            1,
                            "out_func_kind",
                        )),
                        "out_func_kind",
                    )),
                ];
                c_try!(reject_overlapping_optional_outputs(
                    "sidereon_ionex_header_get_mapping_declaration",
                    &outputs
                ));
            }
            // Both outputs are cleared before either is validated, so a null
            // partner never leaves the other holding the caller's stale value.
            if !out_decl_kind.is_null() {
                *out_decl_kind = SidereonIonexMappingDeclarationKind::Absent;
            }
            if !out_func_kind.is_null() {
                *out_func_kind = SidereonIonexMappingFunctionKind::NoMapping;
            }
            let out_decl_kind = c_try!(require_out(
                out_decl_kind,
                "sidereon_ionex_header_get_mapping_declaration",
                "out_decl_kind"
            ));
            let out_func_kind = c_try!(require_out(
                out_func_kind,
                "sidereon_ionex_header_get_mapping_declaration",
                "out_func_kind"
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_mapping_declaration",
                "header"
            ));
            match &header.inner.mapping_function {
                Some(function) => {
                    *out_decl_kind = SidereonIonexMappingDeclarationKind::Declared;
                    *out_func_kind = ionex_mapping_function_to_c(function);
                }
                None => {
                    *out_decl_kind = SidereonIonexMappingDeclarationKind::Absent;
                }
            }
            SidereonStatus::Ok
        },
    )
}

/// Copy the raw mapping function code text (e.g. "NONE", "COSZ", "QFAC", or custom text).
/// Uses variable-length output contract. If absent, required length is 0.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_mapping_function_code(
    header: *const SidereonIonexHeader,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_mapping_function_code",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_mapping_function_code",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_mapping_function_code",
                "header"
            ));
            let code = match &header.inner.mapping_function {
                Some(func) => func.code(),
                None => "",
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_mapping_function_code",
                "out",
                code.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Assign the mapping function on an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile;
/// `custom_code` must point to `custom_code_len` readable bytes, or may be NULL
/// when `custom_code_len` is 0; the bytes are copied during the call, no NUL
/// terminator is read or required, and a NUL among them is copied as data. A
/// NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_set_mapping_function(
    header: *mut SidereonIonexHeader,
    func_kind: u32,
    custom_code: *const u8,
    custom_code_len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_set_mapping_function",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_set_mapping_function",
                "header"
            ));
            let mf = match func_kind {
                v if v == SidereonIonexMappingFunctionKind::NoMapping as u32 => {
                    IonexMappingFunction::NoMapping
                }
                v if v == SidereonIonexMappingFunctionKind::CosZ as u32 => {
                    IonexMappingFunction::CosZ
                }
                v if v == SidereonIonexMappingFunctionKind::QFactor as u32 => {
                    IonexMappingFunction::QFactor
                }
                v if v == SidereonIonexMappingFunctionKind::Other as u32 => {
                    let code_str = c_try!(string_from_c_bytes(
                        "sidereon_ionex_header_set_mapping_function",
                        custom_code,
                        custom_code_len,
                    ));
                    IonexMappingFunction::Other(code_str)
                }
                _ => {
                    set_last_error(
                        "sidereon_ionex_header_set_mapping_function: invalid mapping function kind",
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            header.inner.mapping_function = Some(mf);
            SidereonStatus::Ok
        },
    )
}

/// Clear the mapping function on an IONEX header (marking it absent).
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_clear_mapping_function(
    header: *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_clear_mapping_function",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_clear_mapping_function",
                "header"
            ));
            header.inner.mapping_function = None;
            SidereonStatus::Ok
        },
    )
}

/// Write the count of description lines in an IONEX header to *out_count.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// size_t that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_description_count(
    header: *const SidereonIonexHeader,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_description_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_header_description_count",
                "out_count"
            ));
            *out_count = 0;
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_description_count",
                "header"
            ));
            *out_count = header.inner.descriptions.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy description line at index. Uses variable-length output contract.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_description(
    header: *const SidereonIonexHeader,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_description",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_description",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_description",
                "header"
            ));
            if index >= header.inner.descriptions.len() {
                set_last_error(format!(
                    "sidereon_ionex_header_get_description: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            }
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_description",
                "out",
                header.inner.descriptions[index].as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Append a description line to an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_add_description(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_add_description",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_add_description",
                "header"
            ));
            let desc = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_add_description",
                text,
                len
            ));
            header.inner.descriptions.push(desc);
            SidereonStatus::Ok
        },
    )
}

/// Clear all description lines from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_clear_descriptions(
    header: *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_clear_descriptions",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_clear_descriptions",
                "header"
            ));
            header.inner.descriptions.clear();
            SidereonStatus::Ok
        },
    )
}

/// Write the count of comment lines in an IONEX header to *out_count.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out_count` must point to one writable, aligned
/// size_t that no other argument aliases. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_comment_count(
    header: *const SidereonIonexHeader,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_comment_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_header_comment_count",
                "out_count"
            ));
            *out_count = 0;
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_comment_count",
                "header"
            ));
            *out_count = header.inner.comments.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy comment line at index. Uses variable-length output contract.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_get_comment(
    header: *const SidereonIonexHeader,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_get_comment",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_header_get_comment",
                out_written,
                out_required
            ));
            let header = c_try!(require_ref(
                header,
                "sidereon_ionex_header_get_comment",
                "header"
            ));
            if index >= header.inner.comments.len() {
                set_last_error(format!(
                    "sidereon_ionex_header_get_comment: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            }
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_header_get_comment",
                "out",
                header.inner.comments[index].as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Append a comment line to an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile; `text`
/// must point to `len` readable bytes, or may be NULL when `len` is 0; the
/// bytes are copied during the call, no NUL terminator is read or required, and
/// a NUL among them is copied as data. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced;
/// a non-null pointer that is not valid for the whole call cannot be checked
/// and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_add_comment(
    header: *mut SidereonIonexHeader,
    text: *const u8,
    len: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_add_comment",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_add_comment",
                "header"
            ));
            let comment = c_try!(string_from_c_bytes(
                "sidereon_ionex_header_add_comment",
                text,
                len
            ));
            header.inner.comments.push(comment);
            SidereonStatus::Ok
        },
    )
}

/// Clear all comment lines from an IONEX header.
///
/// Safety: `header` must be a live SidereonIonexHeader handle, exclusively
/// borrowed for the call: nothing else may read or write it meanwhile. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_header_clear_comments(
    header: *mut SidereonIonexHeader,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_header_clear_comments",
        SidereonStatus::Panic,
        || {
            let header = c_try!(require_mut(
                header,
                "sidereon_ionex_header_clear_comments",
                "header"
            ));
            header.inner.comments.clear();
            SidereonStatus::Ok
        },
    )
}

// --- IONEX sample IR --------------------------------------------------------

/// One IONEX vertical-TEC node sample. Angles are degrees, VTEC and RMS are
/// TECU, height offset is kilometers, and epoch is seconds since J2000 in time_scale.
/// Presence flags are authoritative: when false, the numeric value is ignored.
///
/// An IONEX map epoch is a whole second. On input the epoch is
/// `epoch_j2000_whole_s` when `has_epoch_j2000_whole_s` is true, and
/// `epoch_j2000_s` otherwise; that double must hold a whole second exactly or
/// the sample is refused as EpochNotRepresentable naming its index, never
/// rounded to a neighbouring second. A double states every whole second only
/// up to 2^53 seconds in magnitude, so a caller whose epoch lies past that
/// uses the integer field. On output both are filled from the product's
/// exact whole-second axis, `has_epoch_j2000_whole_s` is true, and the double
/// is that integer converted once.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonTecSample {
    /// Epoch time scale as SidereonTimeScale.
    pub time_scale: u32,
    /// Node epoch, seconds since J2000 in time_scale; read only when
    /// has_epoch_j2000_whole_s is false.
    pub epoch_j2000_s: f64,
    /// Whether epoch_j2000_whole_s carries the epoch.
    pub has_epoch_j2000_whole_s: bool,
    /// Node epoch, whole seconds since J2000 in time_scale, exact at every
    /// magnitude (valid when has_epoch_j2000_whole_s is true).
    pub epoch_j2000_whole_s: i64,
    /// Latitude node, degrees.
    pub lat_deg: f64,
    /// Longitude node, degrees.
    pub lon_deg: f64,
    /// Whether vtec_tecu carries an available value.
    pub has_vtec_tecu: bool,
    /// Vertical TEC, TECU (valid when has_vtec_tecu is true).
    pub vtec_tecu: f64,
    /// Whether rms_tecu carries an RMS value.
    pub has_rms_tecu: bool,
    /// RMS value, TECU (valid when has_rms_tecu is true).
    pub rms_tecu: f64,
    /// Whether height_offset_km carries a height offset value.
    pub has_height_offset_km: bool,
    /// Height offset above HGT1, kilometers (valid when has_height_offset_km is true).
    pub height_offset_km: f64,
}

/// Whole-grid IONEX vertical-TEC samples for sidereon_ionex_from_tec_grid_samples.
/// Arrays are caller-owned. Value buffers use [map][lat][lon] order.
/// Presence buffers are optional: if NULL, all corresponding values are treated as present.
/// An all-missing present map (e.g. has_rms_maps=true with all presence false) remains
/// distinct from no map (has_rms_maps=false).
///
/// The map epochs come from `map_epochs_j2000_whole_s` when it is non-NULL and
/// from `map_epochs_j2000_s` otherwise; the other buffer is not read. Each
/// double must hold a whole second exactly, or the grid is refused as
/// EpochNotRepresentable naming the epoch's index, never rounded to a
/// neighbouring second. A double states every whole second only up to 2^53
/// seconds in magnitude; the integer buffer states every one.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTecGridSamples {
    /// Epoch time scale as SidereonTimeScale.
    pub time_scale: u32,
    /// Map epochs, seconds since J2000 in time_scale; read only when
    /// map_epochs_j2000_whole_s is NULL.
    pub map_epochs_j2000_s: *const f64,
    /// Map epochs, whole seconds since J2000 in time_scale, or NULL to read
    /// map_epochs_j2000_s instead. Read over map_epoch_count entries.
    pub map_epochs_j2000_whole_s: *const i64,
    /// Number of map epochs.
    pub map_epoch_count: usize,
    /// Latitude nodes, degrees, in either regular order: the engine takes an
    /// axis running north to south or south to north, as long as dlat_deg
    /// carries the sign that takes the first node to the last.
    pub lat_nodes_deg: *const f64,
    /// Number of latitude nodes.
    pub lat_node_count: usize,
    /// Longitude nodes, degrees, in either regular order, with dlon_deg
    /// carrying the matching sign.
    pub lon_nodes_deg: *const f64,
    /// Number of longitude nodes.
    pub lon_node_count: usize,
    /// Signed latitude step, degrees.
    pub dlat_deg: f64,
    /// Signed longitude step, degrees.
    pub dlon_deg: f64,
    /// Single-layer shell height, kilometers.
    pub shell_height_km: f64,
    /// Mean earth radius used by the IONEX geometry, kilometers.
    pub base_radius_km: f64,
    /// IONEX EXPONENT header value.
    pub exponent: i32,
    /// Flattened VTEC maps, TECU.
    pub tec_maps_tecu: *const f64,
    /// Explicit per-cell presence buffer for VTEC maps (or NULL if all present).
    pub tec_maps_present: *const bool,
    /// Number of flattened VTEC values.
    pub tec_map_value_count: usize,
    /// Whether RMS maps are present.
    pub has_rms_maps: bool,
    /// Flattened RMS maps, TECU, when has_rms_maps is true.
    pub rms_maps_tecu: *const f64,
    /// Explicit per-cell presence buffer for RMS maps (or NULL if all present).
    pub rms_maps_present: *const bool,
    /// Number of flattened RMS values.
    pub rms_map_value_count: usize,
    /// Whether height maps are present.
    pub has_height_maps: bool,
    /// Flattened height maps, km, when has_height_maps is true.
    pub height_maps_km: *const f64,
    /// Explicit per-cell presence buffer for height maps (or NULL if all present).
    pub height_maps_present: *const bool,
    /// Number of flattened height values.
    pub height_map_value_count: usize,
    /// Optional header records handle (if NULL, defaults are used).
    pub header: *const SidereonIonexHeader,
}

/// Dimensions and metadata extracted from an IONEX vertical-TEC sample grid.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonTecGridSamplesInfo {
    /// Number of map epochs.
    pub map_epoch_count: usize,
    /// Number of latitude nodes.
    pub lat_node_count: usize,
    /// Number of longitude nodes.
    pub lon_node_count: usize,
    /// Signed latitude step, degrees.
    pub dlat_deg: f64,
    /// Signed longitude step, degrees.
    pub dlon_deg: f64,
    /// Single-layer shell height, kilometers.
    pub shell_height_km: f64,
    /// Mean earth radius used by the IONEX geometry, kilometers.
    pub base_radius_km: f64,
    /// IONEX EXPONENT header value.
    pub exponent: i32,
    /// Whether RMS maps are present.
    pub has_rms_maps: bool,
    /// Flattened VTEC value count.
    pub tec_map_value_count: usize,
    /// Flattened RMS value count.
    pub rms_map_value_count: usize,
    /// Whether height maps are present.
    pub has_height_maps: bool,
    /// Flattened height value count.
    pub height_map_value_count: usize,
}

/// Which failure building an IONEX product from samples reported.
///
/// Every kind but None and Unknown names a `TecSamplesError` variant of the
/// engine. EpochNotRepresentable, NonFiniteValue and the three count
/// mismatches can also come from this binding's own marshalling, which reads
/// the caller's buffers before the engine sees them; that detail then names
/// the input and index it refused.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTecSamplesErrorKind {
    /// No failure is recorded.
    None = 0,
    /// No TEC samples were supplied.
    Empty = 1,
    /// A latitude or longitude axis has fewer than two nodes; node_count
    /// names how many it has.
    TooFewNodes = 2,
    /// Latitude nodes are not strictly monotonic in the direction dlat_deg gives.
    NonMonotonicLat = 3,
    /// Longitude nodes are not strictly monotonic in the direction dlon_deg gives.
    NonMonotonicLon = 4,
    /// Map epochs are not strictly increasing.
    NonMonotonicEpochs = 5,
    /// A map epoch names no exact whole J2000 second: a fraction of a second,
    /// a non-finite value, or a value outside the int64_t second range. It is
    /// refused rather than rounded to a neighbouring second.
    EpochNotRepresentable = 6,
    /// Grid dimensions do not match the epoch or node axes, or the TEC value
    /// count does not match them.
    ShapeMismatch = 7,
    /// RMS map count or node coverage does not match the TEC maps.
    RmsCountMismatch = 8,
    /// Height map count or node coverage does not match the TEC maps.
    HeightCountMismatch = 9,
    /// A supplied value is NaN or infinite.
    NonFiniteValue = 10,
    /// A signed grid step is zero, so it names no direction for its axis.
    NonPositiveStep = 11,
    /// An axis coordinate or step falls outside [-360, 360] degrees;
    /// axis_value holds it.
    AxisOutOfRange = 12,
    /// A failure this binding does not yet name. The text stays with the
    /// owned result and in the thread-local message.
    Unknown = 999,
}

/// Which caller input a sample-construction failure names by index.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTecSamplesInput {
    /// No input is named.
    None = 0,
    /// An entry of map_epochs_j2000_s or map_epochs_j2000_whole_s.
    MapEpoch = 1,
    /// An entry of the flat tec_maps_tecu buffer.
    TecValue = 2,
    /// An entry of the flat rms_maps_tecu buffer.
    RmsValue = 3,
    /// An entry of the flat height_maps_km buffer.
    HeightValue = 4,
    /// The epoch of the sample at index.
    SampleEpoch = 5,
    /// The vtec_tecu of the sample at index.
    SampleVtec = 6,
    /// The rms_tecu of the sample at index.
    SampleRms = 7,
    /// The height_offset_km of the sample at index.
    SampleHeight = 8,
}

/// Typed detail of a failed IONEX sample construction.
///
/// Only the fields the kind names carry meaning.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonTecSamplesError {
    /// Which failure was reported.
    pub kind: SidereonTecSamplesErrorKind,
    /// The caller input index refers to, when has_index is true.
    pub input: SidereonTecSamplesInput,
    /// Whether index names an entry of a caller buffer.
    pub has_index: bool,
    /// Index into the buffer input names.
    pub index: usize,
    /// Nodes the short axis has, when kind is TooFewNodes.
    pub node_count: usize,
    /// Values supplied, for a count mismatch this binding found.
    pub value_count: usize,
    /// Values the axes require, for a count mismatch this binding found.
    pub expected_value_count: usize,
    /// Whether axis_value carries the refused coordinate.
    pub has_axis_value: bool,
    /// The refused axis coordinate or step in degrees, when kind is
    /// AxisOutOfRange.
    pub axis_value: f64,
}

/// The complete outcome of one IONEX sample construction: the fixed-width part
/// of an owned SidereonTecSamplesResult.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonTecSamplesOutcome {
    /// Whether a product was built.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, otherwise the status the failure maps
    /// to, which is SIDEREON_STATUS_INVALID_ARGUMENT for every kind.
    pub status: SidereonStatus,
    /// The typed failure detail; kind is None when is_ok is true.
    pub error: SidereonTecSamplesError,
}

/// An owned record of one IONEX sample construction.
///
/// It owns its text, so the message stays readable after any number of later
/// failing calls have overwritten the thread-local message. Release it with
/// sidereon_tec_samples_result_free.
pub struct SidereonTecSamplesResult {
    pub(crate) outcome: SidereonTecSamplesOutcome,
    /// The failure text, prefixed with the route that produced it; empty on
    /// success.
    pub(crate) message: String,
}

/// A sample construction that did not produce a product.
pub(crate) enum TecSamplesFailure {
    /// A malformed call: a null pointer, a buffer no slice can span, a time
    /// scale tag this binding does not name. The thread-local message is set.
    Structural(SidereonStatus),
    /// A typed refusal of the samples themselves, with its owned text.
    Refused {
        error: SidereonTecSamplesError,
        message: String,
    },
}

impl From<SidereonStatus> for TecSamplesFailure {
    fn from(status: SidereonStatus) -> Self {
        Self::Structural(status)
    }
}

/// Build an IONEX product from whole-grid TEC samples. On success writes a new
/// handle to *out_ionex; release it with sidereon_ionex_free.
///
/// The struct's pointer fields fall into three groups, and NULL means a
/// different thing in each.
///
/// The mandatory value buffers -- the map epochs, `lat_nodes_deg`,
/// `lon_nodes_deg` and `tec_maps_tecu` -- are read over the count beside each,
/// and each may be NULL only when that count is 0. The map epochs are read from
/// `map_epochs_j2000_whole_s` when it is non-NULL and from
/// `map_epochs_j2000_s` otherwise. `tec_map_value_count` must equal the product
/// of the three node counts, so `tec_maps_tecu` may be NULL only for a grid
/// with no cells at all.
///
/// The presence buffers -- `tec_maps_present`, `rms_maps_present` and
/// `height_maps_present` -- are optional at every count. A NULL one reads all
/// of its map's values as present, exactly as the field comment says; a
/// non-NULL one must be readable over the same count as the values it marks.
///
/// The RMS and height stacks are gated by `has_rms_maps` and
/// `has_height_maps`. When a flag is false its whole stack is ignored: neither
/// `rms_maps_tecu`/`rms_maps_present`/`rms_map_value_count` nor their height
/// counterparts are read at all, and they may hold anything. When a flag is
/// true that stack's value count must equal the same product, its value buffer
/// is mandatory on the rule above and its presence buffer stays optional.
///
/// A refusal of the samples themselves -- an epoch that is not a whole second,
/// a present value that is not finite, a count that disagrees with the axes,
/// or any engine `TecSamplesError` -- returns SIDEREON_STATUS_INVALID_ARGUMENT
/// with text naming the input and index in the thread-local message. Use
/// sidereon_ionex_from_tec_grid_samples_result for the same refusal as a
/// typed SidereonTecSamplesError with owned text.
///
/// Safety: `samples` must point to one readable, aligned SidereonTecGridSamples
/// whose pointer fields satisfy those three rules, with `header` either NULL,
/// which builds the header declaring no mapping function, or a live
/// SidereonIonexHeader handle, which is cloned rather than taken over;
/// `out_ionex` must point to writable, aligned storage for one `SidereonIonex
/// *`, which is set to NULL before any work and receives a newly owned handle
/// only on SIDEREON_STATUS_OK; release it with sidereon_ionex_free. The output
/// slot must be disjoint from the SidereonTecGridSamples struct, from every
/// buffer its pointer fields name, and from the header handle's own storage:
/// it is written while Rust references to all of them are live. Those input
/// buffers are read-only and may overlap one another. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER
/// rather than dereferenced; a non-null pointer that is not valid for the whole
/// call cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_grid_samples(
    samples: *const SidereonTecGridSamples,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_grid_samples",
        SidereonStatus::Panic,
        || {
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_grid_samples",
                "out_ionex"
            ));
            *out_ionex = ptr::null_mut();
            match build_ionex_from_grid_samples("sidereon_ionex_from_tec_grid_samples", samples) {
                Ok(inner) => {
                    write_boxed_handle(out_ionex, SidereonIonex { inner });
                    SidereonStatus::Ok
                }
                Err(failure) => tec_samples_failure_status(failure),
            }
        },
    )
}

/// Build an IONEX product from whole-grid TEC samples and take an owned record
/// of the attempt.
///
/// Same inputs as sidereon_ionex_from_tec_grid_samples. This route returns
/// SIDEREON_STATUS_OK whenever the call itself was well formed and hands back a
/// newly owned SidereonTecSamplesResult; a refusal of the samples is inside
/// it, with `*out_ionex` left NULL. Read it with
/// sidereon_tec_samples_result_get_outcome and
/// sidereon_tec_samples_result_get_message. A structural failure of the call
/// -- a null out-parameter, a null buffer with a nonzero count, a count no
/// slice can span, or a time scale tag this binding does not name -- leaves
/// both outputs NULL, returns a status that is not OK, allocates nothing, and
/// sets the thread-local message.
///
/// Safety: as sidereon_ionex_from_tec_grid_samples for `samples` and
/// `out_ionex`; `out_result` must point to writable, aligned storage for one
/// `SidereonTecSamplesResult *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_samples_result_free. Both output slots must be disjoint from
/// every input and from each other. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_grid_samples_result(
    samples: *const SidereonTecGridSamples,
    out_ionex: *mut *mut SidereonIonex,
    out_result: *mut *mut SidereonTecSamplesResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_grid_samples_result",
        SidereonStatus::Panic,
        || {
            if !out_ionex.is_null() {
                *out_ionex = ptr::null_mut();
            }
            if !out_result.is_null() {
                *out_result = ptr::null_mut();
            }
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_grid_samples_result",
                "out_ionex"
            ));
            let out_result = c_try!(require_out(
                out_result,
                "sidereon_ionex_from_tec_grid_samples_result",
                "out_result"
            ));
            let built = build_ionex_from_grid_samples(
                "sidereon_ionex_from_tec_grid_samples_result",
                samples,
            );
            c_try!(transfer_tec_samples_result(built, out_ionex, out_result));
            SidereonStatus::Ok
        },
    )
}

/// Build an IONEX product from one sample per grid node with default header.
///
/// The default header declares no mapping function: it is the header
/// `sidereon_ionex_header_new` builds. See
/// `sidereon_ionex_from_tec_samples_with_header` for what that means for a
/// slant query and for a product written back, and for the epoch contract.
///
/// Safety: `samples` must point to `count` readable, aligned SidereonTecSample
/// values, or may be NULL when `count` is 0; `out_ionex` must point to
/// writable, aligned storage for one `SidereonIonex *`, which is set to NULL
/// before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_ionex_free. The output slot
/// must be disjoint from the `count` samples: it is written while a Rust
/// reference to them is live. A NULL argument this contract does not allow is
/// refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_samples(
    samples: *const SidereonTecSample,
    count: usize,
    shell_height_km: f64,
    base_radius_km: f64,
    exponent: i32,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    sidereon_ionex_from_tec_samples_with_header(
        samples,
        count,
        shell_height_km,
        base_radius_km,
        exponent,
        ptr::null(),
        out_ionex,
    )
}

/// Build an IONEX product from one sample per grid node with an explicit header handle.
///
/// A NULL `header` builds a header with every record at the value the spec
/// gives for an unstated one and no `MAPPING FUNCTION` record. The product
/// then reports `SIDEREON_IONEX_ASSUMED_MAPPING_KIND_ABSENT` on a successful slant
/// row, is refused under `SIDEREON_IONEX_MAPPING_POLICY_DECLARED`, and writes
/// back no `MAPPING FUNCTION` record. Pass a header from
/// `sidereon_ionex_header_new` plus
/// `sidereon_ionex_header_set_mapping_function` to declare one.
///
/// Each sample's epoch follows the SidereonTecSample contract: the integer
/// field when its flag is set, otherwise a double that must hold a whole
/// second exactly. A refusal of the samples themselves returns
/// SIDEREON_STATUS_INVALID_ARGUMENT with text naming the sample index in the
/// thread-local message; sidereon_ionex_from_tec_samples_result returns the
/// same refusal typed, with owned text.
///
/// Safety: `samples` must point to `count` readable, aligned SidereonTecSample
/// values, or may be NULL when `count` is 0; `header` may be NULL, which builds
/// the header declaring no mapping function; otherwise it must be a live
/// SidereonIonexHeader handle, which is cloned rather than taken over, so the
/// caller still owns it and must still free it; `out_ionex` must point to
/// writable, aligned storage for one `SidereonIonex *`, which is set to NULL
/// before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_ionex_free. The output slot
/// must be disjoint from the `count` samples and from the header handle's own
/// storage: it is written while Rust references to both are live. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_from_tec_samples_with_header(
    samples: *const SidereonTecSample,
    count: usize,
    shell_height_km: f64,
    base_radius_km: f64,
    exponent: i32,
    header: *const SidereonIonexHeader,
    out_ionex: *mut *mut SidereonIonex,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_samples_with_header",
        SidereonStatus::Panic,
        || {
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_samples_with_header",
                "out_ionex"
            ));
            *out_ionex = ptr::null_mut();
            let built = build_ionex_from_node_samples(
                "sidereon_ionex_from_tec_samples_with_header",
                samples,
                count,
                shell_height_km,
                base_radius_km,
                exponent,
                header,
            );
            match built {
                Ok(inner) => {
                    write_boxed_handle(out_ionex, SidereonIonex { inner });
                    SidereonStatus::Ok
                }
                Err(failure) => tec_samples_failure_status(failure),
            }
        },
    )
}

/// Build an IONEX product from one sample per grid node and take an owned
/// record of the attempt.
///
/// Same inputs as sidereon_ionex_from_tec_samples_with_header, `header` NULL
/// included. The outcome contract is sidereon_ionex_from_tec_grid_samples_result's:
/// a refusal of the samples returns SIDEREON_STATUS_OK with `*out_ionex` NULL
/// and the typed refusal in the result, and a structural failure of the call
/// leaves both outputs NULL with a status that is not OK.
///
/// Safety: as sidereon_ionex_from_tec_samples_with_header for every argument
/// but `out_result`, which must point to writable, aligned storage for one
/// `SidereonTecSamplesResult *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_samples_result_free. Both output slots must be disjoint from
/// every input and from each other. A NULL argument this contract does not
/// allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_ionex_from_tec_samples_result(
    samples: *const SidereonTecSample,
    count: usize,
    shell_height_km: f64,
    base_radius_km: f64,
    exponent: i32,
    header: *const SidereonIonexHeader,
    out_ionex: *mut *mut SidereonIonex,
    out_result: *mut *mut SidereonTecSamplesResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_from_tec_samples_result",
        SidereonStatus::Panic,
        || {
            if !out_ionex.is_null() {
                *out_ionex = ptr::null_mut();
            }
            if !out_result.is_null() {
                *out_result = ptr::null_mut();
            }
            let out_ionex = c_try!(require_out(
                out_ionex,
                "sidereon_ionex_from_tec_samples_result",
                "out_ionex"
            ));
            let out_result = c_try!(require_out(
                out_result,
                "sidereon_ionex_from_tec_samples_result",
                "out_result"
            ));
            let built = build_ionex_from_node_samples(
                "sidereon_ionex_from_tec_samples_result",
                samples,
                count,
                shell_height_km,
                base_radius_km,
                exponent,
                header,
            );
            c_try!(transfer_tec_samples_result(built, out_ionex, out_result));
            SidereonStatus::Ok
        },
    )
}

/// Release an owned IONEX sample-construction result. Passing NULL is a no-op.
///
/// Safety: `result` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonTecSamplesResult handle this binding produced, and it must be
/// passed here exactly once. The handle and every pointer read out of it are
/// invalid afterwards. This call returns nothing, so it reports no status: a
/// pointer that is neither NULL nor such a handle cannot be checked and is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_samples_result_free(result: *mut SidereonTecSamplesResult) {
    ffi_boundary("sidereon_tec_samples_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned IONEX sample-construction result.
///
/// `out_outcome` is written before the result pointer is validated, so it never
/// keeps whatever the caller left in it.
///
/// Safety: `result` must be a live SidereonTecSamplesResult handle, not freed
/// for the duration of the call; `out_outcome` must point to one writable,
/// aligned SidereonTecSamplesOutcome that no other argument aliases. A NULL
/// argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_samples_result_get_outcome(
    result: *const SidereonTecSamplesResult,
    out_outcome: *mut SidereonTecSamplesOutcome,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_samples_result_get_outcome",
        SidereonStatus::Panic,
        || {
            let out_outcome = c_try!(require_out(
                out_outcome,
                "sidereon_tec_samples_result_get_outcome",
                "out_outcome"
            ));
            *out_outcome = SidereonTecSamplesOutcome {
                is_ok: false,
                status: SidereonStatus::InvalidArgument,
                error: no_tec_samples_error(),
            };
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_samples_result_get_outcome",
                "result"
            ));
            *out_outcome = result.outcome;
            SidereonStatus::Ok
        },
    )
}

/// Copy the failure text of an owned IONEX sample-construction result. A
/// successful result reports a required length of zero. The result owns this
/// text, so it is unaffected by later calls. Uses the variable-length output
/// contract.
///
/// Safety: `result` must be a live SidereonTecSamplesResult handle, not freed
/// for the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_samples_result_get_message(
    result: *const SidereonTecSamplesResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_samples_result_get_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_samples_result_get_message",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_samples_result_get_message",
                "result"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_samples_result_get_message",
                "out",
                result.message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the number of records a forgiving read of the product skipped: an
/// `AUX DATA` block counts as one, and so does each unrecognized header record
/// and each summary record that could not be read. A product built from
/// samples reports zero.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_count` must point to one writable, aligned size_t
/// that no other argument aliases. A NULL argument this contract does not allow
/// is refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_skipped_records(
    ionex: *const SidereonIonex,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_skipped_records",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_ionex_skipped_records",
                "out_count"
            ));
            *out_count = 0;
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_skipped_records",
                "ionex"
            ));
            *out_count = ionex.inner.skipped_records();
            SidereonStatus::Ok
        },
    )
}

/// Read IONEX TEC-grid sample dimensions and metadata.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out_info` must point to one writable, aligned
/// SidereonTecGridSamplesInfo that no other argument aliases. A NULL argument
/// this contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER
/// rather than dereferenced; a non-null pointer that is not valid for the whole
/// call cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_info(
    ionex: *const SidereonIonex,
    out_info: *mut SidereonTecGridSamplesInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_info",
        SidereonStatus::Panic,
        || {
            let out_info = c_try!(require_out(
                out_info,
                "sidereon_ionex_tec_grid_samples_info",
                "out_info"
            ));
            *out_info = SidereonTecGridSamplesInfo {
                map_epoch_count: 0,
                lat_node_count: 0,
                lon_node_count: 0,
                dlat_deg: 0.0,
                dlon_deg: 0.0,
                shell_height_km: 0.0,
                base_radius_km: 0.0,
                exponent: 0,
                has_rms_maps: false,
                tec_map_value_count: 0,
                rms_map_value_count: 0,
                has_height_maps: false,
                height_map_value_count: 0,
            };
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_info",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            *out_info = tec_grid_samples_info(&samples);
            SidereonStatus::Ok
        },
    )
}

/// Copy TEC-grid map epochs as seconds since J2000. Uses variable-length output.
///
/// Each value is the whole second `sidereon_ionex_map_epochs_j2000_s` copies,
/// converted to a double once, so it is exact wherever a double holds that
/// integer, which is every second up to 2^53 in magnitude, and the nearest
/// double past it. Read `sidereon_ionex_map_epochs_j2000_s` for the int64_t
/// axis that is exact at every magnitude. This call never fails on an epoch:
/// every map epoch a product holds is a whole second.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_epochs_j2000_s(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                "ionex"
            ));
            let epochs: Vec<f64> = ionex
                .inner
                .map_epochs_s()
                .into_iter()
                .map(|seconds| seconds as f64)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_epochs_j2000_s",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened IONEX VTEC maps, TECU. Missing nodes write NaN. Uses variable-length output.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_tec_maps_tecu(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_values(&samples.tec_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_tec_maps_tecu",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened VTEC per-cell presence flags (true if cell holds a value).
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned bools that do not overlap the handle being read. `len`
/// counts bools, not bytes. `out_written` and `out_required` must each point to
/// one writable, aligned size_t, must alias neither each other nor `out`, and
/// are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_tec_presence(
    ionex: *const SidereonIonex,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_tec_presence",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_tec_presence",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_tec_presence",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_presence(&samples.tec_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_tec_presence",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened IONEX RMS maps, TECU. If no RMS maps exist, required length is 0.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_rms_maps_tecu(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_values(&samples.rms_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_rms_maps_tecu",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened RMS per-cell presence flags. If no RMS maps exist, required length is 0.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned bools that do not overlap the handle being read. `len`
/// counts bools, not bytes. `out_written` and `out_required` must each point to
/// one writable, aligned size_t, must alias neither each other nor `out`, and
/// are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_rms_presence(
    ionex: *const SidereonIonex,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_rms_presence",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_rms_presence",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_rms_presence",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_presence(&samples.rms_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_rms_presence",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened IONEX height maps, km. If no height maps exist, required length is 0.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_height_maps_km(
    ionex: *const SidereonIonex,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_height_maps_km",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_height_maps_km",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_height_maps_km",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_values(&samples.height_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_height_maps_km",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened height per-cell presence flags. If no height maps exist, required length is 0.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned bools that do not overlap the handle being read. `len`
/// counts bools, not bytes. `out_written` and `out_required` must each point to
/// one writable, aligned size_t, must alias neither each other nor `out`, and
/// are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_grid_samples_height_presence(
    ionex: *const SidereonIonex,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ionex_tec_grid_samples_height_presence",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ionex_tec_grid_samples_height_presence",
                out_written,
                out_required
            ));
            let ionex = c_try!(require_ref(
                ionex,
                "sidereon_ionex_tec_grid_samples_height_presence",
                "ionex"
            ));
            let samples = ionex.inner.tec_grid_samples();
            let flat = flatten_grid_presence(&samples.height_maps);
            c_try!(copy_prefix_to_c(
                "sidereon_ionex_tec_grid_samples_height_presence",
                "out",
                &flat,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy one IONEX TEC sample per grid node. Uses variable-length output.
///
/// Every sample carries its map's exact whole second in
/// `epoch_j2000_whole_s`, with `has_epoch_j2000_whole_s` true, and the same
/// second converted to a double once in `epoch_j2000_s`, so the samples read
/// back through sidereon_ionex_from_tec_samples as the epochs they came from.
///
/// Safety: `ionex` must be a live SidereonIonex handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned SidereonTecSample values that do not overlap the handle
/// being read. `len` counts SidereonTecSample values, not bytes. `out_written`
/// and `out_required` must each point to one writable, aligned size_t, must
/// alias neither each other nor `out`, and are both set to 0 before anything
/// else is read. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ionex_tec_samples(
    ionex: *const SidereonIonex,
    out: *mut SidereonTecSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ionex_tec_samples", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ionex_tec_samples",
            out_written,
            out_required
        ));
        let ionex = c_try!(require_ref(ionex, "sidereon_ionex_tec_samples", "ionex"));
        let values = tec_samples_to_c(&ionex.inner);
        c_try!(copy_prefix_to_c(
            "sidereon_ionex_tec_samples",
            "out",
            &values,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// --- Standalone regular TecGrid coverage ------------------------------------

/// A standalone regular-grid vertical-TEC source.
///
/// The grid stores TECU values on strictly increasing epoch, latitude and
/// longitude axes. Epoch coordinates are `f64` Unix nanoseconds; latitude and
/// longitude are degrees; values are flat in epoch-latitude-longitude order
/// with longitude varying fastest.
pub struct SidereonTecGrid {
    pub(crate) inner: TecGrid,
}

/// Which failure a standalone TEC-grid construction or query reported.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTecGridErrorKind {
    /// No failure is recorded.
    None = 0,
    /// An axis has fewer than two nodes.
    AxesTooShort = 1,
    /// An axis is not strictly increasing.
    AxesNotIncreasing = 2,
    /// The product of the axis lengths overflowed.
    DimensionsOverflow = 3,
    /// The value count does not match the axis lengths.
    ValueCountMismatch = 4,
    /// A named input failed one of the engine's shared validation rules. The
    /// exact field label and reason are strings, so they are carried by an
    /// owned SidereonTecGridResult rather than by this fixed-width record:
    /// read them with sidereon_tec_grid_result_get_field and
    /// sidereon_tec_grid_result_get_reason.
    InvalidField = 5,
    /// The query weights grid nodes that hold no value.
    NodesNotAvailable = 6,
    /// The query lies outside an axis.
    OutOfBounds = 7,
    /// A value the caller marked present is not finite. This binding checks
    /// that before it builds a grid, so the engine never saw the value and
    /// named no field of its own; value_index names the rejected entry.
    ValueNotFinite = 8,
    /// A failure this binding does not yet name. The engine's own text stays in
    /// the thread-local message.
    Unknown = 999,
}

/// Which axis a standalone TEC-grid bound failure names.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTecGridAxis {
    /// No axis is named.
    None = 0,
    /// The epoch axis, in Unix nanoseconds.
    Epoch = 1,
    /// The latitude axis, in degrees.
    Latitude = 2,
    /// The longitude axis, in degrees.
    Longitude = 3,
    /// An axis this binding does not yet name.
    Unknown = 999,
}

/// Typed detail of a standalone TEC-grid failure.
///
/// Only the fields the kind names carry meaning. The engine's full text is
/// always in the thread-local message.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonTecGridError {
    /// Which failure the grid reported.
    pub kind: SidereonTecGridErrorKind,
    /// Whether gap carries the non-available nodes the query weighted.
    pub has_gap: bool,
    /// Indexed corner masks, when kind is NodesNotAvailable.
    pub gap: SidereonIonexNodeGap,
    /// The axis the query left, when kind is OutOfBounds.
    pub axis: SidereonTecGridAxis,
    /// Whether axis_value carries the query coordinate that left the axis.
    pub has_axis_value: bool,
    /// The query coordinate the axis refused, in that axis's own unit, after
    /// any clamp the engine applied. On the latitude axis this is the effective
    /// coordinate, clamped to `[-87.5, 87.5]` degrees, so a query at 89 degrees
    /// that a narrower axis still refuses names 87.5, not 89. An infinite
    /// latitude is clamped the same way and names the bound it was outside,
    /// never an infinity.
    pub axis_value: f64,
    /// Values supplied, when kind is ValueCountMismatch.
    pub value_count: usize,
    /// Values the axes require, when kind is ValueCountMismatch.
    pub expected_value_count: usize,
    /// Whether value_index names an entry of the caller's value buffer.
    pub has_value_index: bool,
    /// Index into the caller's flat value buffer, when kind is ValueNotFinite.
    pub value_index: usize,
}

/// The complete outcome of one standalone TEC-grid construction or evaluation.
///
/// This is the fixed-width part of an owned SidereonTecGridResult. The two
/// parts of a failure that are text -- the engine's full message and the field
/// label and reason of an InvalidField -- are read from the same result with
/// sidereon_tec_grid_result_get_message, _get_field and _get_reason.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidereonTecGridOutcome {
    /// Whether the route succeeded. A false value means error names the failure.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, otherwise the status this failure maps
    /// to: a malformed grid or a rejected input is an argument failure, a query
    /// the grid cannot answer is a solve failure.
    pub status: SidereonStatus,
    /// Whether vtec_tecu carries a value. False for a construction result,
    /// which builds a grid rather than evaluating one, and for any failure.
    pub has_vtec: bool,
    /// Vertical TEC in TECU when has_vtec is true, otherwise NaN.
    pub vtec_tecu: f64,
    /// The non-available nodes a returned value was interpolated around, under
    /// the renormalizing policy. Empty on success with no gap, and empty on a
    /// failure: a strict refusal names its corners in error.gap instead,
    /// because no value was produced to degrade.
    pub degraded: SidereonIonexNodeGap,
    /// The typed failure detail; kind is None when is_ok is true.
    pub error: SidereonTecGridError,
}

/// An owned record of one standalone TEC-grid construction or evaluation.
///
/// The result owns every string it reports, so the complete message, the exact
/// InvalidField label and reason, and the indexed details all stay readable
/// after the grid is freed and after any number of later failing calls have
/// overwritten the thread-local message. Release it with
/// sidereon_tec_grid_result_free.
pub struct SidereonTecGridResult {
    pub(crate) outcome: SidereonTecGridOutcome,
    /// The engine's own text for a failure, prefixed with the route that
    /// produced it; empty on success.
    pub(crate) message: String,
    /// The stable field label of an InvalidField failure; empty otherwise.
    pub(crate) field: String,
    /// The short reason of an InvalidField failure; empty otherwise.
    pub(crate) reason: String,
}

/// Construct a standalone TecGrid from epoch, latitude and longitude axes and
/// flat values in epoch-latitude-longitude order, longitude varying fastest.
///
/// `presence` may be NULL, which treats every value as present, or point at
/// `values_count` flags. A flag that is false marks a node without a value and
/// the matching entry in `values` is ignored, so a caller need not write NaN
/// there. A present value must be finite; an explicit zero is a value.
///
/// On success a newly owned handle is written to `*out_grid`; release it with
/// sidereon_tec_grid_free. On any failure `*out_grid` stays NULL, nothing is
/// allocated, and `out_error`, when it is not NULL, takes the typed detail.
///
/// This is the convenience route. Its failure text is the thread-local message,
/// which the next failing call in this thread overwrites, and an InvalidField's
/// field label and reason exist only inside that text. Use
/// sidereon_tec_grid_new_result for a failure the caller owns in full.
///
/// Safety: `epochs_ns` must point to `epochs_count` readable, aligned doubles,
/// or may be NULL when `epochs_count` is 0; `latitudes_deg` must point to
/// `latitudes_count` readable, aligned doubles, or may be NULL when
/// `latitudes_count` is 0; `longitudes_deg` must point to `longitudes_count`
/// readable, aligned doubles, or may be NULL when `longitudes_count` is 0;
/// `values` must point to `values_count` readable, aligned doubles, or may be
/// NULL when `values_count` is 0; `presence` may be NULL at any count, which
/// reads every value as present; otherwise it must point to `values_count`
/// readable, aligned bools; `out_grid` must point to writable, aligned storage
/// for one `SidereonTecGrid *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_grid_free; `out_error` may be NULL, which discards the detail;
/// otherwise it must point to one writable, aligned SidereonTecGridError that
/// no other argument aliases. Both outputs must be disjoint from every input
/// buffer above and from each other: they are written while Rust references to
/// those buffers are live. The input buffers themselves are read-only and may
/// overlap one another. A NULL argument this contract does not allow is
/// refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_new(
    epochs_ns: *const f64,
    epochs_count: usize,
    latitudes_deg: *const f64,
    latitudes_count: usize,
    longitudes_deg: *const f64,
    longitudes_count: usize,
    values: *const f64,
    presence: *const bool,
    values_count: usize,
    out_grid: *mut *mut SidereonTecGrid,
    out_error: *mut SidereonTecGridError,
) -> SidereonStatus {
    ffi_boundary("sidereon_tec_grid_new", SidereonStatus::Panic, || {
        // Both outputs are cleared before either is validated, so a null
        // `out_grid` still leaves a writable `out_error` holding no failure.
        if !out_grid.is_null() {
            *out_grid = ptr::null_mut();
        }
        if !out_error.is_null() {
            *out_error = no_tec_grid_error();
        }
        let out_grid = c_try!(require_out(out_grid, "sidereon_tec_grid_new", "out_grid"));
        let (grid, result) = c_try!(build_tec_grid(
            "sidereon_tec_grid_new",
            epochs_ns,
            epochs_count,
            latitudes_deg,
            latitudes_count,
            longitudes_deg,
            longitudes_count,
            values,
            presence,
            values_count,
        ));
        if !out_error.is_null() {
            *out_error = result.outcome.error;
        }
        match grid {
            Some(grid) => {
                write_boxed_handle(out_grid, SidereonTecGrid { inner: grid });
                SidereonStatus::Ok
            }
            None => {
                // This route reports the text through the thread-local
                // message, which the next failing call in this thread
                // overwrites. sidereon_tec_grid_new_result owns it instead.
                set_last_error(result.message.clone());
                result.outcome.status
            }
        }
    })
}

/// Construct a standalone TecGrid and take an owned record of the attempt.
///
/// Same inputs as sidereon_tec_grid_new. The difference is where the failure
/// detail lives: this route returns SIDEREON_STATUS_OK whenever the call itself
/// was well formed and hands back a newly owned SidereonTecGridResult that
/// keeps the complete engine message, the exact InvalidField label and reason,
/// and every indexed detail, for as long as the caller holds it. Neither a
/// later failing call nor releasing the grid disturbs it.
///
/// On a construction failure the outer status is still SIDEREON_STATUS_OK,
/// `*out_grid` stays NULL, and the result reports is_ok false with the failure.
/// Read it with sidereon_tec_grid_result_get_outcome. On a structural failure
/// of the call itself -- a null out-parameter, a null buffer with a nonzero
/// count, or a count no slice can span -- both outputs are set to NULL, the
/// returned status is not OK, nothing is allocated, and the text is in the
/// thread-local message.
///
/// Release the grid with sidereon_tec_grid_free and the result with
/// sidereon_tec_grid_result_free; they are independent allocations.
///
/// Safety: `epochs_ns` must point to `epochs_count` readable, aligned doubles,
/// or may be NULL when `epochs_count` is 0; `latitudes_deg` must point to
/// `latitudes_count` readable, aligned doubles, or may be NULL when
/// `latitudes_count` is 0; `longitudes_deg` must point to `longitudes_count`
/// readable, aligned doubles, or may be NULL when `longitudes_count` is 0;
/// `values` must point to `values_count` readable, aligned doubles, or may be
/// NULL when `values_count` is 0; `presence` may be NULL at any count, which
/// reads every value as present; otherwise it must point to `values_count`
/// readable, aligned bools; `out_grid` must point to writable, aligned storage
/// for one `SidereonTecGrid *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_grid_free; `out_result` must point to writable, aligned storage
/// for one `SidereonTecGridResult *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_grid_result_free. Both output slots must be disjoint from
/// every input buffer above and from each other: they are written while Rust
/// references to those buffers are live. The input buffers themselves are
/// read-only and may overlap one another. A NULL argument this contract does
/// not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_new_result(
    epochs_ns: *const f64,
    epochs_count: usize,
    latitudes_deg: *const f64,
    latitudes_count: usize,
    longitudes_deg: *const f64,
    longitudes_count: usize,
    values: *const f64,
    presence: *const bool,
    values_count: usize,
    out_grid: *mut *mut SidereonTecGrid,
    out_result: *mut *mut SidereonTecGridResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_new_result",
        SidereonStatus::Panic,
        || {
            // Both outputs are cleared before either is validated. Validating one
            // first would let a null argument return while the other output kept
            // whatever the caller left in it, which a caller checking for NULL
            // reads as a transferred handle.
            if !out_grid.is_null() {
                *out_grid = ptr::null_mut();
            }
            if !out_result.is_null() {
                *out_result = ptr::null_mut();
            }
            let out_grid = c_try!(require_out(
                out_grid,
                "sidereon_tec_grid_new_result",
                "out_grid"
            ));
            let out_result = c_try!(require_out(
                out_result,
                "sidereon_tec_grid_new_result",
                "out_result"
            ));
            let (grid, result) = c_try!(build_tec_grid(
                "sidereon_tec_grid_new_result",
                epochs_ns,
                epochs_count,
                latitudes_deg,
                latitudes_count,
                longitudes_deg,
                longitudes_count,
                values,
                presence,
                values_count,
            ));
            // Nothing below can fail, so the two handles are transferred together:
            // the caller never sees a grid without its result, and no allocated
            // result is orphaned behind a failing status.
            if let Some(grid) = grid {
                write_boxed_handle(out_grid, SidereonTecGrid { inner: grid });
            }
            write_boxed_handle(out_result, result);
            SidereonStatus::Ok
        },
    )
}

/// Release a standalone TecGrid handle. Passing NULL is a no-op.
///
/// Safety: `grid` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonTecGrid handle this binding produced, and it must be passed here
/// exactly once. The handle and every pointer read out of it are invalid
/// afterwards. This call returns nothing, so it reports no status: a pointer
/// that is neither NULL nor such a handle cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_free(grid: *mut SidereonTecGrid) {
    ffi_boundary("sidereon_tec_grid_free", (), || {
        free_boxed(grid);
    });
}

/// Write the axis lengths and the flat value count of a standalone TecGrid.
///
/// Every output pointer must be non-NULL. The value count is the product of
/// the three axis lengths and sizes both the value and the presence buffer.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out_epoch_count` must point to one writable, aligned
/// size_t that no other argument aliases; `out_latitude_count` must point to
/// one writable, aligned size_t that no other argument aliases;
/// `out_longitude_count` must point to one writable, aligned size_t that no
/// other argument aliases; `out_value_count` must point to one writable,
/// aligned size_t that no other argument aliases. A NULL argument this contract
/// does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather than
/// dereferenced; a non-null pointer that is not valid for the whole call cannot
/// be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_dimensions(
    grid: *const SidereonTecGrid,
    out_epoch_count: *mut usize,
    out_latitude_count: *mut usize,
    out_longitude_count: *mut usize,
    out_value_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_dimensions",
        SidereonStatus::Panic,
        || {
            // Every output is cleared before any is validated, so one null
            // output never leaves the others holding the caller's stale values.
            for out in [
                out_epoch_count,
                out_latitude_count,
                out_longitude_count,
                out_value_count,
            ] {
                if !out.is_null() {
                    *out = 0;
                }
            }
            let out_epoch_count = c_try!(require_out(
                out_epoch_count,
                "sidereon_tec_grid_dimensions",
                "out_epoch_count"
            ));
            let out_latitude_count = c_try!(require_out(
                out_latitude_count,
                "sidereon_tec_grid_dimensions",
                "out_latitude_count"
            ));
            let out_longitude_count = c_try!(require_out(
                out_longitude_count,
                "sidereon_tec_grid_dimensions",
                "out_longitude_count"
            ));
            let out_value_count = c_try!(require_out(
                out_value_count,
                "sidereon_tec_grid_dimensions",
                "out_value_count"
            ));
            let grid = c_try!(require_ref(grid, "sidereon_tec_grid_dimensions", "grid"));
            *out_epoch_count = grid.inner.epochs_ns().len();
            *out_latitude_count = grid.inner.latitudes_deg().len();
            *out_longitude_count = grid.inner.longitudes_deg().len();
            *out_value_count = grid.inner.values().len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the epoch axis of a standalone TecGrid, in Unix nanoseconds.
///
/// The axis is `f64`, so adjacent nanoseconds are not distinguishable at large
/// magnitudes; a query takes an exact `i64` and the engine converts it. Uses
/// the variable-length output contract.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_epochs_ns(
    grid: *const SidereonTecGrid,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tec_grid_epochs_ns", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_tec_grid_epochs_ns",
            out_written,
            out_required
        ));
        let grid = c_try!(require_ref(grid, "sidereon_tec_grid_epochs_ns", "grid"));
        c_try!(copy_prefix_to_c(
            "sidereon_tec_grid_epochs_ns",
            "out",
            grid.inner.epochs_ns(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the latitude axis of a standalone TecGrid, in degrees. Uses the
/// variable-length output contract.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_latitudes_deg(
    grid: *const SidereonTecGrid,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_latitudes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_latitudes_deg",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(grid, "sidereon_tec_grid_latitudes_deg", "grid"));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_latitudes_deg",
                "out",
                grid.inner.latitudes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the longitude axis of a standalone TecGrid, in degrees. Uses the
/// variable-length output contract.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_longitudes_deg(
    grid: *const SidereonTecGrid,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_longitudes_deg",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_longitudes_deg",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_tec_grid_longitudes_deg",
                "grid"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_longitudes_deg",
                "out",
                grid.inner.longitudes_deg(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the flat TEC values of a standalone TecGrid, in TECU, in
/// epoch-latitude-longitude order with longitude varying fastest.
///
/// A node without a value writes NaN; read
/// sidereon_tec_grid_value_presence for the authority on which nodes hold one.
/// Uses the variable-length output contract.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned doubles that do not overlap the handle being read. `len`
/// counts doubles, not bytes. `out_written` and `out_required` must each point
/// to one writable, aligned size_t, must alias neither each other nor `out`,
/// and are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_values_tecu(
    grid: *const SidereonTecGrid,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_values_tecu",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_values_tecu",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(grid, "sidereon_tec_grid_values_tecu", "grid"));
            let values: Vec<f64> = grid
                .inner
                .values()
                .iter()
                .map(|value| value.unwrap_or(f64::NAN))
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_values_tecu",
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

/// Copy the per-node presence flags of a standalone TecGrid, in the same flat
/// order as the values. A flag is true where the node holds a value, including
/// an explicit zero. Uses the variable-length output contract.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out` may be NULL only when `len` is 0, which queries
/// the required count through `out_required`; otherwise it must point to `len`
/// writable, aligned bools that do not overlap the handle being read. `len`
/// counts bools, not bytes. `out_written` and `out_required` must each point to
/// one writable, aligned size_t, must alias neither each other nor `out`, and
/// are both set to 0 before anything else is read. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_value_presence(
    grid: *const SidereonTecGrid,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_value_presence",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_value_presence",
                out_written,
                out_required
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_tec_grid_value_presence",
                "grid"
            ));
            let presence: Vec<bool> = grid.inner.values().iter().map(Option::is_some).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_value_presence",
                "out",
                &presence,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Evaluate vertical TEC at a pierce point on a standalone TecGrid.
///
/// The pierce-point latitude and longitude are degrees, the unit the engine
/// takes, and reach it unchanged, so a query on a grid node is evaluated at
/// that node exactly. The engine clamps the latitude to `[-87.5, 87.5]`
/// degrees, the band an IONEX grid covers, and evaluates at that effective
/// coordinate: a query at 89 degrees returns the value at 87.5 degrees with
/// SIDEREON_STATUS_OK and no degraded or gap marker, because nothing was
/// interpolated around.
///
/// The clamp compares, so it takes an infinite latitude too: a positive
/// infinity is above the upper bound and evaluates at 87.5, a negative
/// infinity is below the lower bound and evaluates at -87.5, each as ordinary
/// a success or an ordinary out-of-bounds failure as any other latitude
/// reaching that effective coordinate. A NaN latitude compares false against
/// both bounds, so it passes through unchanged and the engine's shared
/// validation refuses it as an InvalidField naming `latitude` and `not
/// finite`. The longitude and the epoch are not clamped. `unix_nanos` is an
/// exact integer the engine converts to its own `f64` epoch axis.
/// `missing_node_policy` is a SidereonIonexMissingNodePolicy tag: strict
/// refuses a query that weights a node without a value, renormalizing
/// interpolates from the weighted nodes that hold values and marks the result
/// degraded.
///
/// `out_gap` carries the non-available nodes in both directions: the ones a
/// renormalized value was interpolated around, and the ones a strict refusal
/// names. `out_error` may be NULL; when it is not, a failure fills it with the
/// typed detail. Every writable output is cleared before any argument is
/// validated: `*out_vtec` to NaN, `*out_gap` to no gap and `*out_error` to no
/// failure, so a null partner never leaves another output stale.
///
/// This is the convenience route. Its failure text is the thread-local message,
/// which the next failing call in this thread overwrites, and an InvalidField's
/// field label and reason exist only inside that text. Use
/// sidereon_tec_grid_vtec_at_pierce_point_result for a failure the caller owns
/// in full.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out_vtec` must point to one writable, aligned double
/// that no other argument aliases; `out_gap` must point to one writable,
/// aligned SidereonIonexNodeGap that no other argument aliases; `out_error` may
/// be NULL, which discards the detail; otherwise it must point to one writable,
/// aligned SidereonTecGridError that no other argument aliases. A NULL argument
/// this contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER
/// rather than dereferenced; a non-null pointer that is not valid for the whole
/// call cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_vtec_at_pierce_point(
    grid: *const SidereonTecGrid,
    unix_nanos: i64,
    lat_deg: f64,
    lon_deg: f64,
    missing_node_policy: u32,
    out_vtec: *mut f64,
    out_gap: *mut SidereonIonexNodeGap,
    out_error: *mut SidereonTecGridError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_vtec_at_pierce_point",
        SidereonStatus::Panic,
        || {
            if !out_vtec.is_null() {
                *out_vtec = f64::NAN;
            }
            if !out_gap.is_null() {
                *out_gap = empty_node_gap_c();
            }
            if !out_error.is_null() {
                *out_error = no_tec_grid_error();
            }
            let out_vtec = c_try!(require_out(
                out_vtec,
                "sidereon_tec_grid_vtec_at_pierce_point",
                "out_vtec"
            ));
            let out_gap = c_try!(require_out(
                out_gap,
                "sidereon_tec_grid_vtec_at_pierce_point",
                "out_gap"
            ));
            let grid = c_try!(require_ref(
                grid,
                "sidereon_tec_grid_vtec_at_pierce_point",
                "grid"
            ));
            let policy = c_try!(missing_node_policy_from_c(
                "sidereon_tec_grid_vtec_at_pierce_point",
                missing_node_policy
            ));
            let result = evaluate_tec_grid_pierce_point(
                "sidereon_tec_grid_vtec_at_pierce_point",
                grid,
                unix_nanos,
                lat_deg,
                lon_deg,
                policy,
            );
            let outcome = result.outcome;
            if !out_error.is_null() {
                *out_error = outcome.error;
            }
            // This route reports the gap in both directions: the nodes a
            // renormalized value was interpolated around, and the ones a strict
            // refusal names.
            if outcome.degraded.has_gap {
                *out_gap = outcome.degraded;
            } else if outcome.error.has_gap {
                *out_gap = outcome.error.gap;
            }
            if outcome.is_ok {
                *out_vtec = outcome.vtec_tecu;
                SidereonStatus::Ok
            } else {
                // The text is thread-local here, so the next failing call in
                // this thread overwrites it, and an InvalidField's field and
                // reason are only inside it.
                // sidereon_tec_grid_vtec_at_pierce_point_result owns both.
                set_last_error(result.message);
                outcome.status
            }
        },
    )
}

/// Evaluate vertical TEC at a pierce point and take an owned record of it.
///
/// Same inputs and same angular contract as
/// sidereon_tec_grid_vtec_at_pierce_point: the pierce-point latitude and
/// longitude are degrees and reach the engine unchanged, `unix_nanos` is an
/// exact integer, and `missing_node_policy` is a
/// SidereonIonexMissingNodePolicy tag. The engine clamps the latitude to
/// `[-87.5, 87.5]` degrees here too, and this route reports no more about the
/// clamp than the convenience one does: the effective coordinate is what the
/// grid was queried at, and an out-of-bounds `axis_value` on the latitude axis
/// is that effective coordinate, not the latitude the caller supplied. An
/// infinite latitude is clamped here
/// the same way, positive to 87.5 and negative to -87.5; a NaN latitude is
/// not clamped and is refused, and this route is where its InvalidField label
/// `latitude` and reason `not finite` survive as owned text.
///
/// The difference is ownership. This route returns SIDEREON_STATUS_OK whenever
/// the call itself was well formed and hands back a newly owned
/// SidereonTecGridResult carrying the value or the complete failure: the
/// status, the typed detail with its indexed corner masks, axis name and
/// counts, the exact InvalidField label and reason, and the engine's whole
/// message. All of it survives freeing the grid and any number of later
/// failing calls in this thread.
///
/// On a structural failure of the call itself -- a null grid, a null
/// out-parameter, or a policy tag this binding does not name -- `*out_result`
/// is set to NULL, the returned status is not OK, nothing is allocated, and
/// the text is in the thread-local message. Release the result with
/// sidereon_tec_grid_result_free.
///
/// Safety: `grid` must be a live SidereonTecGrid handle, not freed for the
/// duration of the call; `out_result` must point to writable, aligned storage
/// for one `SidereonTecGridResult *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_tec_grid_result_free. The output slot must be disjoint from the
/// grid's own storage: it is written while a Rust reference to the grid is
/// live. A NULL argument this contract does not allow is refused with
/// SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null pointer
/// that is not valid for the whole call cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_vtec_at_pierce_point_result(
    grid: *const SidereonTecGrid,
    unix_nanos: i64,
    lat_deg: f64,
    lon_deg: f64,
    missing_node_policy: u32,
    out_result: *mut *mut SidereonTecGridResult,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_vtec_at_pierce_point_result",
        SidereonStatus::Panic,
        || {
            let out_result = c_try!(require_out(
                out_result,
                "sidereon_tec_grid_vtec_at_pierce_point_result",
                "out_result"
            ));
            *out_result = ptr::null_mut();
            let grid = c_try!(require_ref(
                grid,
                "sidereon_tec_grid_vtec_at_pierce_point_result",
                "grid"
            ));
            let policy = c_try!(missing_node_policy_from_c(
                "sidereon_tec_grid_vtec_at_pierce_point_result",
                missing_node_policy
            ));
            let result = evaluate_tec_grid_pierce_point(
                "sidereon_tec_grid_vtec_at_pierce_point_result",
                grid,
                unix_nanos,
                lat_deg,
                lon_deg,
                policy,
            );
            write_boxed_handle(out_result, result);
            SidereonStatus::Ok
        },
    )
}

/// Release an owned standalone TEC-grid result. Passing NULL is a no-op.
///
/// Safety: `result` may be NULL, which is a no-op; otherwise it must be a live
/// SidereonTecGridResult handle this binding produced, and it must be passed
/// here exactly once. The handle and every pointer read out of it are invalid
/// afterwards. This call returns nothing, so it reports no status: a pointer
/// that is neither NULL nor such a handle cannot be checked and is undefined
/// behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_result_free(result: *mut SidereonTecGridResult) {
    ffi_boundary("sidereon_tec_grid_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned standalone TEC-grid result.
///
/// `out_outcome` is written before the result pointer is validated, so it never
/// keeps whatever the caller left in it.
///
/// Safety: `result` must be a live SidereonTecGridResult handle, not freed for
/// the duration of the call; `out_outcome` must point to one writable, aligned
/// SidereonTecGridOutcome that no other argument aliases. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_result_get_outcome(
    result: *const SidereonTecGridResult,
    out_outcome: *mut SidereonTecGridOutcome,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_result_get_outcome",
        SidereonStatus::Panic,
        || {
            let out_outcome = c_try!(require_out(
                out_outcome,
                "sidereon_tec_grid_result_get_outcome",
                "out_outcome"
            ));
            *out_outcome = unread_tec_grid_outcome();
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_grid_result_get_outcome",
                "result"
            ));
            *out_outcome = result.outcome;
            SidereonStatus::Ok
        },
    )
}

/// Copy the engine's own text from an owned standalone TEC-grid result. A
/// successful result reports a required length of zero. The result owns this
/// text, so it is unaffected by later calls. Uses the variable-length output
/// contract.
///
/// Safety: `result` must be a live SidereonTecGridResult handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_result_get_message(
    result: *const SidereonTecGridResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_result_get_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_result_get_message",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_grid_result_get_message",
                "result"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_result_get_message",
                "out",
                result.message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the stable field label of an InvalidField failure, exactly as the
/// engine's shared validation named it. Any other outcome reports a required
/// length of zero; this binding never invents a label. Uses the variable-length
/// output contract.
///
/// Safety: `result` must be a live SidereonTecGridResult handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_result_get_field(
    result: *const SidereonTecGridResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_result_get_field",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_result_get_field",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_grid_result_get_field",
                "result"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_result_get_field",
                "out",
                result.field.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the short reason of an InvalidField failure, exactly as the engine's
/// shared validation named it. Any other outcome reports a required length of
/// zero. Uses the variable-length output contract.
///
/// Safety: `result` must be a live SidereonTecGridResult handle, not freed for
/// the duration of the call; `out` may be NULL only when `len` is 0, which
/// queries the required count through `out_required`; otherwise it must point
/// to `len` writable, aligned bytes that do not overlap the handle being read.
/// `len` is the buffer size in bytes. `out_written` and `out_required` must
/// each point to one writable, aligned size_t, must alias neither each other
/// nor `out`, and are both set to 0 before anything else is read. The bytes are
/// copied verbatim: no NUL terminator is appended, so a caller wanting a C
/// string must allocate one more byte and write it. A NULL argument this
/// contract does not allow is refused with SIDEREON_STATUS_NULL_POINTER rather
/// than dereferenced; a non-null pointer that is not valid for the whole call
/// cannot be checked and is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tec_grid_result_get_reason(
    result: *const SidereonTecGridResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tec_grid_result_get_reason",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tec_grid_result_get_reason",
                out_written,
                out_required
            ));
            let result = c_try!(require_ref(
                result,
                "sidereon_tec_grid_result_get_reason",
                "result"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_tec_grid_result_get_reason",
                "out",
                result.reason.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

// --- Galileo NeQuick-G ionosphere (sidereon_core::atmosphere::ionosphere) ----

/// Galileo coefficient-driven single-frequency ionospheric group delay in native units (meters).
///
/// Safety: `out_delay_m` must point to one writable, aligned double that no
/// other argument aliases. A NULL argument this contract does not allow is
/// refused with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a
/// non-null pointer that is not valid for the whole call cannot be checked and
/// is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_galileo_nequick_g_native(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    lat_deg: f64,
    lon_deg: f64,
    el_deg: f64,
    t_gal_s: f64,
    day_of_year: f64,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_galileo_nequick_g_native",
        SidereonStatus::Panic,
        || {
            let out_delay_m = c_try!(require_out(
                out_delay_m,
                "sidereon_galileo_nequick_g_native",
                "out_delay_m"
            ));
            *out_delay_m = 0.0;
            let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
            let eval = GalileoNequickEval {
                lat_deg,
                lon_deg,
                el_deg,
                t_gal_s,
                day_of_year,
                frequency_hz,
            };
            match galileo_nequick_g_native(&coeffs, eval) {
                Ok(delay) => {
                    *out_delay_m = delay;
                    SidereonStatus::Ok
                }
                Err(err) => map_iono_error("sidereon_galileo_nequick_g_native", err),
            }
        },
    )
}

/// Receiver/satellite ray geometry and epoch for a full NeQuick-G evaluation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNequickGRay {
    /// Month of the year, 1..=12.
    pub month: u8,
    /// UTC time of day in hours, [0, 24].
    pub utc_hours: f64,
    /// Receiver geodetic longitude, degrees.
    pub station_lon_deg: f64,
    /// Receiver geodetic latitude, degrees.
    pub station_lat_deg: f64,
    /// Receiver height above the reference sphere, metres.
    pub station_height_m: f64,
    /// Satellite geodetic longitude, degrees.
    pub satellite_lon_deg: f64,
    /// Satellite geodetic latitude, degrees.
    pub satellite_lat_deg: f64,
    /// Satellite height above the reference sphere, metres.
    pub satellite_height_m: f64,
}

/// Full NeQuick-G slant total electron content along the ray, in TECU.
///
/// Safety: `ray` must point to one readable, aligned SidereonNequickGRay;
/// `out_stec_tecu` must point to one writable, aligned double that no other
/// argument aliases. A NULL argument this contract does not allow is refused
/// with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null
/// pointer that is not valid for the whole call cannot be checked and is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nequick_g_stec_tecu(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    ray: *const SidereonNequickGRay,
    out_stec_tecu: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nequick_g_stec_tecu",
        SidereonStatus::Panic,
        || {
            let out_stec_tecu = c_try!(require_out(
                out_stec_tecu,
                "sidereon_nequick_g_stec_tecu",
                "out_stec_tecu"
            ));
            *out_stec_tecu = 0.0;
            let ray = c_try!(require_ref(ray, "sidereon_nequick_g_stec_tecu", "ray"));
            let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
            match nequick_g_stec_tecu(&coeffs, &nequick_g_ray_from_c(ray)) {
                Ok(stec) => {
                    *out_stec_tecu = stec;
                    SidereonStatus::Ok
                }
                Err(err) => map_iono_error("sidereon_nequick_g_stec_tecu", err),
            }
        },
    )
}

/// Full NeQuick-G slant ionospheric group delay (meters) on frequency_hz.
///
/// Safety: `ray` must point to one readable, aligned SidereonNequickGRay;
/// `out_delay_m` must point to one writable, aligned double that no other
/// argument aliases. A NULL argument this contract does not allow is refused
/// with SIDEREON_STATUS_NULL_POINTER rather than dereferenced; a non-null
/// pointer that is not valid for the whole call cannot be checked and is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nequick_g_delay_m(
    ai0: f64,
    ai1: f64,
    ai2: f64,
    ray: *const SidereonNequickGRay,
    frequency_hz: f64,
    out_delay_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_nequick_g_delay_m", SidereonStatus::Panic, || {
        let out_delay_m = c_try!(require_out(
            out_delay_m,
            "sidereon_nequick_g_delay_m",
            "out_delay_m"
        ));
        *out_delay_m = 0.0;
        let ray = c_try!(require_ref(ray, "sidereon_nequick_g_delay_m", "ray"));
        let coeffs = GalileoNequickCoeffs { ai0, ai1, ai2 };
        match nequick_g_delay_m(&coeffs, &nequick_g_ray_from_c(ray), frequency_hz) {
            Ok(delay) => {
                *out_delay_m = delay;
                SidereonStatus::Ok
            }
            Err(err) => map_iono_error("sidereon_nequick_g_delay_m", err),
        }
    })
}

// --- Internal helpers -------------------------------------------------------

/// The evaluation a refused query leaves in a caller's output.
///
/// The delay is NaN and the status is not valid, so a refused row can never be
/// read as a nominal zero delay.
fn refused_ionex_slant_delay_evaluation() -> SidereonIonexSlantDelayEvaluation {
    SidereonIonexSlantDelayEvaluation {
        delay_m: f64::NAN,
        status: SidereonIonexSlantDelayStatus {
            is_valid: false,
            has_held: false,
            coverage_error: SidereonIonexCoverageErrorKind::None,
            has_degraded: false,
            gap: empty_node_gap_c(),
            has_assumed_mapping: false,
            assumed_mapping: SidereonIonexAssumedMappingKind::None,
        },
    }
}

/// The typed detail of a row that reports no failure.
fn no_ionex_slant_error() -> SidereonIonexSlantError {
    SidereonIonexSlantError {
        kind: SidereonIonexSlantErrorKind::None,
        coverage_error: SidereonIonexCoverageErrorKind::None,
        has_gap: false,
        gap: empty_node_gap_c(),
        refusal: SidereonIonexSlantRefusalKind::None,
        refusal_map_number: 0,
        refusal_lat_index: 0,
        refusal_lon_index: 0,
        has_mapping_declaration: false,
        mapping_declaration: SidereonIonexMappingDeclarationKind::Absent,
        has_mapping_function: false,
        mapping_function: SidereonIonexMappingFunctionKind::NoMapping,
    }
}

/// The typed detail of a failure this binding does not name. The engine's own
/// text stays with the row, so nothing about the failure is lost.
fn unknown_ionex_slant_error() -> SidereonIonexSlantError {
    SidereonIonexSlantError {
        kind: SidereonIonexSlantErrorKind::Unknown,
        ..no_ionex_slant_error()
    }
}

/// The typed detail of an input the engine rejected by name.
fn invalid_input_error() -> SidereonIonexSlantError {
    SidereonIonexSlantError {
        kind: SidereonIonexSlantErrorKind::InvalidInput,
        ..no_ionex_slant_error()
    }
}

fn refused_row_view(
    status: SidereonStatus,
    detail: SidereonIonexSlantError,
) -> SidereonIonexSlantRowResult {
    SidereonIonexSlantRowResult {
        is_ok: false,
        status,
        evaluation: refused_ionex_slant_delay_evaluation(),
        error: detail,
    }
}

fn refused_row(
    status: SidereonStatus,
    detail: SidereonIonexSlantError,
    message: String,
    mapping_code: String,
) -> IonexSlantResultRow {
    IonexSlantResultRow {
        view: refused_row_view(status, detail),
        message,
        mapping_code,
    }
}

/// The status code an engine IONEX failure maps to.
fn ionex_error_status(err: &CoreError) -> SidereonStatus {
    match err {
        CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // Every other failure, a later engine's included, is a solve failure;
        // the caller reads the engine's text, variant name included, from the
        // route's message.
        _ => SidereonStatus::Solve,
    }
}

/// The typed detail of an engine IONEX failure, with the MAPPING FUNCTION code
/// a mapping refusal named, which the caller's result list then owns.
fn ionex_slant_error_to_c(err: &CoreError) -> (SidereonIonexSlantError, String) {
    match err {
        CoreError::InvalidInput(_) => (invalid_input_error(), String::new()),
        CoreError::IonexOutOfCoverage(error) => (
            SidereonIonexSlantError {
                kind: SidereonIonexSlantErrorKind::OutOfCoverage,
                coverage_error: ionex_coverage_error_to_c(*error),
                ..no_ionex_slant_error()
            },
            String::new(),
        ),
        CoreError::IonexNodesNotAvailable(gap) => (
            SidereonIonexSlantError {
                kind: SidereonIonexSlantErrorKind::NodesNotAvailable,
                has_gap: true,
                gap: ionex_node_gap_to_c(**gap),
                ..no_ionex_slant_error()
            },
            String::new(),
        ),
        CoreError::IonexSlantUnavailable(refusal) => ionex_slant_refusal_to_c(refusal),
        // Another engine failure reaching an IONEX route. The row keeps the
        // engine's own text, so the caller reads what happened even where this
        // binding has no field for it yet.
        _ => (unknown_ionex_slant_error(), String::new()),
    }
}

/// The typed detail of an IONEX slant refusal, with its mapping code text.
fn ionex_slant_refusal_to_c(refusal: &IonexSlantRefusal) -> (SidereonIonexSlantError, String) {
    let base = SidereonIonexSlantError {
        kind: SidereonIonexSlantErrorKind::SlantUnavailable,
        ..no_ionex_slant_error()
    };
    match refusal {
        IonexSlantRefusal::VaryingHeights {
            map_number,
            lat_index,
            lon_index,
        } => (
            SidereonIonexSlantError {
                refusal: SidereonIonexSlantRefusalKind::VaryingHeights,
                refusal_map_number: *map_number,
                refusal_lat_index: *lat_index,
                refusal_lon_index: *lon_index,
                ..base
            },
            String::new(),
        ),
        IonexSlantRefusal::HeightNotAvailable {
            map_number,
            lat_index,
            lon_index,
        } => (
            SidereonIonexSlantError {
                refusal: SidereonIonexSlantRefusalKind::HeightNotAvailable,
                refusal_map_number: *map_number,
                refusal_lat_index: *lat_index,
                refusal_lon_index: *lon_index,
                ..base
            },
            String::new(),
        ),
        IonexSlantRefusal::MappingFunction(declaration) => {
            let (declaration_kind, function, code) = match declaration {
                IonexMappingDeclaration::Declared(function) => (
                    SidereonIonexMappingDeclarationKind::Declared,
                    Some(ionex_mapping_function_to_c(function)),
                    function.code().to_owned(),
                ),
                IonexMappingDeclaration::Absent => (
                    SidereonIonexMappingDeclarationKind::Absent,
                    None,
                    String::new(),
                ),
            };
            (
                SidereonIonexSlantError {
                    refusal: SidereonIonexSlantRefusalKind::MappingFunction,
                    has_mapping_declaration: true,
                    mapping_declaration: declaration_kind,
                    has_mapping_function: function.is_some(),
                    mapping_function: function
                        .unwrap_or(SidereonIonexMappingFunctionKind::NoMapping),
                    ..base
                },
                code,
            )
        }
        // A refusal the engine added after this binding was written. The row
        // keeps the engine's own text for it.
        _ => (
            SidereonIonexSlantError {
                refusal: SidereonIonexSlantRefusalKind::Unknown,
                ..base
            },
            String::new(),
        ),
    }
}

/// The typed detail of a standalone TEC-grid route that reports no failure.
fn no_tec_grid_error() -> SidereonTecGridError {
    SidereonTecGridError {
        kind: SidereonTecGridErrorKind::None,
        has_gap: false,
        gap: empty_node_gap_c(),
        axis: SidereonTecGridAxis::None,
        has_axis_value: false,
        axis_value: 0.0,
        value_count: 0,
        expected_value_count: 0,
        has_value_index: false,
        value_index: 0,
    }
}

/// The outcome an accessor writes before it has validated its result handle.
fn unread_tec_grid_outcome() -> SidereonTecGridOutcome {
    SidereonTecGridOutcome {
        is_ok: false,
        status: SidereonStatus::InvalidArgument,
        has_vtec: false,
        vtec_tecu: f64::NAN,
        degraded: empty_node_gap_c(),
        error: no_tec_grid_error(),
    }
}

/// An owned standalone TEC-grid result that reports a value.
fn tec_grid_value_result(vtec: f64, degraded: SidereonIonexNodeGap) -> SidereonTecGridResult {
    SidereonTecGridResult {
        outcome: SidereonTecGridOutcome {
            is_ok: true,
            status: SidereonStatus::Ok,
            has_vtec: true,
            vtec_tecu: vtec,
            degraded,
            error: no_tec_grid_error(),
        },
        message: String::new(),
        field: String::new(),
        reason: String::new(),
    }
}

/// An owned standalone TEC-grid result that reports a grid was built. A
/// construction evaluates nothing, so it carries no vertical TEC.
fn tec_grid_built_result() -> SidereonTecGridResult {
    SidereonTecGridResult {
        outcome: SidereonTecGridOutcome {
            is_ok: true,
            status: SidereonStatus::Ok,
            has_vtec: false,
            vtec_tecu: f64::NAN,
            degraded: empty_node_gap_c(),
            error: no_tec_grid_error(),
        },
        message: String::new(),
        field: String::new(),
        reason: String::new(),
    }
}

/// An owned standalone TEC-grid failure. The caller supplies every part that
/// is text, so the result never borrows from the thread-local message.
fn tec_grid_failure(
    status: SidereonStatus,
    error: SidereonTecGridError,
    message: String,
    field: String,
    reason: String,
) -> SidereonTecGridResult {
    SidereonTecGridResult {
        outcome: SidereonTecGridOutcome {
            is_ok: false,
            status,
            has_vtec: false,
            vtec_tecu: f64::NAN,
            degraded: empty_node_gap_c(),
            error,
        },
        message,
        field,
        reason,
    }
}

/// An owned standalone TEC-grid failure built from an engine error.
///
/// InvalidField's exact field label and reason are copied here, so they outlive
/// the borrow they arrive in. A variant this binding does not yet name keeps
/// the engine's own Display text as its message rather than claiming a detail
/// it cannot read.
fn tec_grid_core_failure(fn_name: &str, err: &TecGridError) -> SidereonTecGridResult {
    let (field, reason) = match err {
        TecGridError::InvalidField { field, reason } => ((*field).to_owned(), (*reason).to_owned()),
        _ => (String::new(), String::new()),
    };
    tec_grid_failure(
        tec_grid_error_status(err),
        tec_grid_error_to_c(err),
        format!("{fn_name}: {err}"),
        field,
        reason,
    )
}

/// Marshal the caller's axes and values and build a standalone TEC grid.
///
/// `Err` names a structural failure of the call itself: a null buffer with a
/// nonzero count, or a count no slice can span. `Ok` carries the grid when one
/// was built, and in either case the owned record of the attempt. Nothing is
/// allocated on the C side here, so an `Err` leaves no orphaned handle.
#[allow(clippy::too_many_arguments)]
unsafe fn build_tec_grid(
    fn_name: &str,
    epochs_ns: *const f64,
    epochs_count: usize,
    latitudes_deg: *const f64,
    latitudes_count: usize,
    longitudes_deg: *const f64,
    longitudes_count: usize,
    values: *const f64,
    presence: *const bool,
    values_count: usize,
) -> Result<(Option<TecGrid>, SidereonTecGridResult), SidereonStatus> {
    // The axis lengths are reconciled before any buffer is read, so a count
    // that overflows or disagrees with the value count is refused without ever
    // forming a slice over it.
    let expected = match epochs_count
        .checked_mul(latitudes_count)
        .and_then(|product| product.checked_mul(longitudes_count))
    {
        Some(expected) => expected,
        None => {
            return Ok((
                None,
                tec_grid_failure(
                    SidereonStatus::InvalidArgument,
                    SidereonTecGridError {
                        kind: SidereonTecGridErrorKind::DimensionsOverflow,
                        ..no_tec_grid_error()
                    },
                    format!("{fn_name}: the axis lengths overflow a value count"),
                    String::new(),
                    String::new(),
                ),
            ));
        }
    };
    if values_count != expected {
        return Ok((
            None,
            tec_grid_failure(
                SidereonStatus::InvalidArgument,
                SidereonTecGridError {
                    kind: SidereonTecGridErrorKind::ValueCountMismatch,
                    value_count: values_count,
                    expected_value_count: expected,
                    ..no_tec_grid_error()
                },
                format!("{fn_name}: expected {expected} values, got {values_count}"),
                String::new(),
                String::new(),
            ),
        ));
    }

    let epochs = require_slice(epochs_ns, epochs_count, fn_name, "epochs_ns")?;
    let lats = require_slice(latitudes_deg, latitudes_count, fn_name, "latitudes_deg")?;
    let lons = require_slice(longitudes_deg, longitudes_count, fn_name, "longitudes_deg")?;
    let vals = require_slice(values, values_count, fn_name, "values")?;
    let pres_slice = if presence.is_null() {
        None
    } else {
        Some(require_slice(presence, values_count, fn_name, "presence")?)
    };

    let mut core_values = Vec::with_capacity(values_count);
    for (idx, value) in vals.iter().enumerate() {
        let is_present = match pres_slice {
            Some(mask) => mask[idx],
            None => true,
        };
        if !is_present {
            core_values.push(None);
            continue;
        }
        if !value.is_finite() {
            // This binding rejects the entry by index before the engine sees
            // it, so the failure names no engine field and carries no engine
            // reason.
            return Ok((
                None,
                tec_grid_failure(
                    SidereonStatus::InvalidArgument,
                    SidereonTecGridError {
                        kind: SidereonTecGridErrorKind::ValueNotFinite,
                        has_value_index: true,
                        value_index: idx,
                        ..no_tec_grid_error()
                    },
                    format!(
                        "{fn_name}: the value at index {idx} is marked present and is not finite"
                    ),
                    String::new(),
                    String::new(),
                ),
            ));
        }
        core_values.push(Some(*value));
    }

    match TecGrid::new(epochs.to_vec(), lats.to_vec(), lons.to_vec(), core_values) {
        Ok(grid) => Ok((Some(grid), tec_grid_built_result())),
        Err(err) => Ok((None, tec_grid_core_failure(fn_name, &err))),
    }
}

/// Evaluate vertical TEC at a pierce point into an owned record.
///
/// The pierce-point angles arrive in degrees, the unit the engine takes, and
/// are handed over unchanged, longitude before latitude, so a query on a grid
/// node is the node itself rather than a unit round trip away from it. Both
/// exported pierce-point routes reach the grid through here, so the engine's
/// latitude clamp to `[-87.5, 87.5]` degrees, applied inside
/// `vtec_at_pierce_point_with_policy` before the axis lookup, governs both
/// alike. The binding neither reproduces the clamp nor reports it: an
/// evaluation at a clamped latitude is an ordinary success, and a latitude the
/// clamped value still cannot reach on a narrower axis is an ordinary
/// out-of-bounds failure whose axis value is the clamped coordinate.
///
/// The core clamp is a pair of comparisons, which decides the two non-finite
/// latitudes differently. An infinity compares above or below the bound and
/// leaves the clamp as +/-87.5, so it reaches the axis lookup as an ordinary
/// in-band coordinate. A NaN compares false either way, so the clamp returns
/// it unchanged and the engine's own validation refuses it as an InvalidField
/// on `latitude` with reason `not finite`. This binding adds no finite-angle
/// check of its own in front of either: the accepted inputs are the engine's.
fn evaluate_tec_grid_pierce_point(
    fn_name: &str,
    grid: &SidereonTecGrid,
    unix_nanos: i64,
    lat_deg: f64,
    lon_deg: f64,
    policy: IonexMissingNodePolicy,
) -> SidereonTecGridResult {
    let epoch = TecGridEpoch::new(unix_nanos, 0);
    match grid
        .inner
        .vtec_at_pierce_point_with_policy(epoch, lon_deg, lat_deg, policy)
    {
        Ok(evaluation) => {
            let degraded = match evaluation.degraded {
                Some(gap) => ionex_node_gap_to_c(gap),
                None => empty_node_gap_c(),
            };
            tec_grid_value_result(evaluation.value, degraded)
        }
        Err(err) => tec_grid_core_failure(fn_name, &err),
    }
}

/// The typed detail of a standalone TEC-grid failure.
fn tec_grid_error_to_c(err: &TecGridError) -> SidereonTecGridError {
    match err {
        TecGridError::AxesTooShort => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::AxesTooShort,
            ..no_tec_grid_error()
        },
        TecGridError::AxesNotIncreasing => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::AxesNotIncreasing,
            ..no_tec_grid_error()
        },
        TecGridError::DimensionsOverflow => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::DimensionsOverflow,
            ..no_tec_grid_error()
        },
        TecGridError::ValueCountMismatch { actual, expected } => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::ValueCountMismatch,
            value_count: *actual,
            expected_value_count: *expected,
            ..no_tec_grid_error()
        },
        TecGridError::InvalidField { .. } => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::InvalidField,
            ..no_tec_grid_error()
        },
        TecGridError::NodesNotAvailable(gap) => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::NodesNotAvailable,
            has_gap: true,
            gap: ionex_node_gap_to_c(*gap),
            ..no_tec_grid_error()
        },
        TecGridError::OutOfBounds { name, value } => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::OutOfBounds,
            axis: tec_grid_axis_to_c(name),
            has_axis_value: true,
            axis_value: *value,
            ..no_tec_grid_error()
        },
        // A failure the engine added after this binding was written. The
        // thread-local message still carries the engine's own text for it.
        _ => SidereonTecGridError {
            kind: SidereonTecGridErrorKind::Unknown,
            ..no_tec_grid_error()
        },
    }
}

/// The axis a standalone TEC-grid bound failure names, from the engine's label.
fn tec_grid_axis_to_c(name: &str) -> SidereonTecGridAxis {
    match name {
        "timestamp" => SidereonTecGridAxis::Epoch,
        "latitude" => SidereonTecGridAxis::Latitude,
        "longitude" => SidereonTecGridAxis::Longitude,
        _ => SidereonTecGridAxis::Unknown,
    }
}

/// The status code a standalone TEC-grid failure maps to. A malformed grid or a
/// rejected input is an argument failure; a query the grid cannot answer is a
/// solve failure.
fn tec_grid_error_status(err: &TecGridError) -> SidereonStatus {
    match err {
        TecGridError::AxesTooShort
        | TecGridError::AxesNotIncreasing
        | TecGridError::DimensionsOverflow
        | TecGridError::ValueCountMismatch { .. }
        | TecGridError::InvalidField { .. } => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

/// The engine missing-node policy a numeric tag names.
fn missing_node_policy_from_c(
    fn_name: &str,
    policy: u32,
) -> Result<IonexMissingNodePolicy, SidereonStatus> {
    match policy {
        value if value == SidereonIonexMissingNodePolicy::Strict as u32 => {
            Ok(IonexMissingNodePolicy::Strict)
        }
        value if value == SidereonIonexMissingNodePolicy::Renormalize as u32 => {
            Ok(IonexMissingNodePolicy::Renormalize)
        }
        _ => {
            set_last_error(format!(
                "{fn_name}: {policy} is not a missing-node policy tag"
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ionex_mapping_function_to_c(
    function: &IonexMappingFunction,
) -> SidereonIonexMappingFunctionKind {
    match function {
        IonexMappingFunction::NoMapping => SidereonIonexMappingFunctionKind::NoMapping,
        IonexMappingFunction::CosZ => SidereonIonexMappingFunctionKind::CosZ,
        IonexMappingFunction::QFactor => SidereonIonexMappingFunctionKind::QFactor,
        IonexMappingFunction::Other(_) => SidereonIonexMappingFunctionKind::Other,
    }
}

fn zero_warning_info() -> SidereonIonexWarningInfo {
    SidereonIonexWarningInfo {
        kind: SidereonIonexWarningKind::Unknown,
        line: 0,
        map_number: 0,
        set_by_line: 0,
        declared_count: 0,
        tec_map_count: 0,
        all_map_count: 0,
        declared_interval_s: 0,
        actual_spacing_s: 0,
        exponent: 0,
        lat_deg: 0.0,
        lon_deg: 0.0,
        has_epochs: false,
        declared_epoch_j2000_s: 0.0,
        maps_epoch_j2000_s: 0.0,
        declared_epoch_j2000_whole_s: 0,
        maps_epoch_j2000_whole_s: 0,
    }
}

fn empty_node_gap_c() -> SidereonIonexNodeGap {
    SidereonIonexNodeGap {
        has_gap: false,
        earlier: SidereonIonexMissingNodes {
            has_missing: false,
            map_number: 0,
            lat_index: 0,
            lon_index: 0,
            lon_index_next: 0,
            missing: [false; 4],
        },
        later: SidereonIonexMissingNodes {
            has_missing: false,
            map_number: 0,
            lat_index: 0,
            lon_index: 0,
            lon_index_next: 0,
            missing: [false; 4],
        },
    }
}

fn ionex_missing_nodes_to_c(nodes: Option<IonexMissingNodes>) -> SidereonIonexMissingNodes {
    match nodes {
        Some(n) => SidereonIonexMissingNodes {
            has_missing: true,
            map_number: n.map_number,
            lat_index: n.lat_index,
            lon_index: n.lon_index,
            lon_index_next: n.lon_index_next,
            missing: n.missing,
        },
        None => SidereonIonexMissingNodes {
            has_missing: false,
            map_number: 0,
            lat_index: 0,
            lon_index: 0,
            lon_index_next: 0,
            missing: [false; 4],
        },
    }
}

fn ionex_node_gap_to_c(gap: IonexNodeGap) -> SidereonIonexNodeGap {
    SidereonIonexNodeGap {
        has_gap: gap.earlier.is_some() || gap.later.is_some(),
        earlier: ionex_missing_nodes_to_c(gap.earlier),
        later: ionex_missing_nodes_to_c(gap.later),
    }
}

pub(crate) fn ionex_slant_policy_from_c(
    fn_name: &str,
    policy: SidereonIonexSlantPolicy,
) -> Result<IonexSlantPolicy, SidereonStatus> {
    let coverage = match policy.coverage {
        v if v == SidereonIonexCoveragePolicy::Strict as u32 => IonexCoveragePolicy::Strict,
        v if v == SidereonIonexCoveragePolicy::Hold as u32 => IonexCoveragePolicy::Hold,
        _ => {
            set_last_error(format!(
                "{fn_name}: {} is not a coverage policy tag",
                policy.coverage
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let missing_nodes = missing_node_policy_from_c(fn_name, policy.missing_nodes)?;
    let mapping = match policy.mapping {
        v if v == SidereonIonexMappingPolicy::Declared as u32 => IonexMappingPolicy::Declared,
        v if v == SidereonIonexMappingPolicy::SingleLayer as u32 => IonexMappingPolicy::SingleLayer,
        _ => {
            set_last_error(format!(
                "{fn_name}: {} is not a mapping policy tag",
                policy.mapping
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(IonexSlantPolicy::default()
        .with_coverage(coverage)
        .with_missing_nodes(missing_nodes)
        .with_mapping(mapping))
}

#[derive(Clone, Copy)]
struct IonexSlantDelayCRequest {
    lat_deg: f64,
    lon_deg: f64,
    azimuth_deg: f64,
    elevation_deg: f64,
    epoch_j2000_s: i64,
    frequency_hz: f64,
}

/// A failed scalar slant-delay evaluation: the status the caller receives and
/// the typed detail an optional output pointer takes. It is returned boxed,
/// since the typed detail makes it much wider than the evaluation it stands in
/// for.
struct IonexSlantFailure {
    status: SidereonStatus,
    detail: SidereonIonexSlantError,
}

unsafe fn ionex_slant_delay_eval_from_c(
    fn_name: &str,
    ionex: *const SidereonIonex,
    request: IonexSlantDelayCRequest,
    policy: IonexSlantPolicy,
) -> Result<IonexSlantDelayEvaluation, Box<IonexSlantFailure>> {
    let ionex = require_ref(ionex, fn_name, "ionex").map_err(|status| {
        Box::new(IonexSlantFailure {
            status,
            detail: no_ionex_slant_error(),
        })
    })?;
    let receiver = geodetic_to_wgs84(
        fn_name,
        "receiver",
        SidereonGeodetic {
            lat_rad: request.lat_deg * IONO_DEG_TO_RAD,
            lon_rad: request.lon_deg * IONO_DEG_TO_RAD,
            height_m: 0.0,
        },
    )
    .map_err(|status| {
        Box::new(IonexSlantFailure {
            status,
            detail: invalid_input_error(),
        })
    })?;
    match ionex_slant_delay_with_policy(
        &ionex.inner,
        receiver,
        request.elevation_deg * IONO_DEG_TO_RAD,
        request.azimuth_deg * IONO_DEG_TO_RAD,
        ionex_epoch_from_j2000_seconds(request.epoch_j2000_s),
        request.frequency_hz,
        policy,
    ) {
        Ok(evaluation) => Ok(evaluation),
        Err(err) => {
            let (detail, _) = ionex_slant_error_to_c(&err);
            Err(Box::new(IonexSlantFailure {
                status: map_iono_error(fn_name, err),
                detail,
            }))
        }
    }
}

fn ionex_slant_delay_evaluation_to_c(
    evaluation: IonexSlantDelayEvaluation,
) -> SidereonIonexSlantDelayEvaluation {
    let (has_held, coverage_error) = match evaluation.status.held {
        Some(error) => (true, ionex_coverage_error_to_c(error)),
        None => (false, SidereonIonexCoverageErrorKind::None),
    };
    let (has_degraded, gap) = match evaluation.status.degraded {
        Some(node_gap) => (true, ionex_node_gap_to_c(node_gap)),
        None => (false, empty_node_gap_c()),
    };
    let (has_assumed_mapping, assumed_mapping) = match evaluation.status.assumed_mapping {
        Some(assumed) => (true, ionex_assumed_mapping_to_c(assumed)),
        None => (false, SidereonIonexAssumedMappingKind::None),
    };
    SidereonIonexSlantDelayEvaluation {
        delay_m: evaluation.delay_m,
        status: SidereonIonexSlantDelayStatus {
            is_valid: evaluation.status.is_valid(),
            has_held,
            coverage_error,
            has_degraded,
            gap,
            has_assumed_mapping,
            assumed_mapping,
        },
    }
}

fn ionex_coverage_error_to_c(error: IonexCoverageError) -> SidereonIonexCoverageErrorKind {
    match error {
        IonexCoverageError::EpochBeforeFirstMap => {
            SidereonIonexCoverageErrorKind::EpochBeforeFirstMap
        }
        IonexCoverageError::EpochAfterLastMap => SidereonIonexCoverageErrorKind::EpochAfterLastMap,
        IonexCoverageError::LatitudeOutOfRange => {
            SidereonIonexCoverageErrorKind::LatitudeOutOfRange
        }
        IonexCoverageError::LongitudeOutOfRange => {
            SidereonIonexCoverageErrorKind::LongitudeOutOfRange
        }
    }
}

fn ionex_assumed_mapping_to_c(assumed: IonexAssumedMapping) -> SidereonIonexAssumedMappingKind {
    match assumed {
        IonexAssumedMapping::NoMapping => SidereonIonexAssumedMappingKind::NoMapping,
        IonexAssumedMapping::QFactor => SidereonIonexAssumedMappingKind::QFactor,
        IonexAssumedMapping::Other => SidereonIonexAssumedMappingKind::Other,
        IonexAssumedMapping::Absent => SidereonIonexAssumedMappingKind::Absent,
    }
}

fn warning_label(warning: &IonexWarning) -> &'static str {
    match warning {
        IonexWarning::MissingRecord(label) => label,
        IonexWarning::VersionRecordNotFirst { .. } => "IONEX VERSION / TYPE",
        IonexWarning::EpochMismatch { label, .. } => label,
        IonexWarning::MapCountMismatch { .. } => "# OF MAPS IN FILE",
        IonexWarning::NotANumberValue { kind, .. } => kind,
        IonexWarning::IntervalMismatch { .. } => "INTERVAL",
        IonexWarning::ExponentCarriedIntoMap { kind, .. } => kind,
        // A finding the engine added after this binding was written. It names
        // no record label here rather than borrowing one that does not apply;
        // the full text stays available through the message accessor.
        _ => "",
    }
}

fn warning_info_to_c(warning: &IonexWarning) -> SidereonIonexWarningInfo {
    let mut info = zero_warning_info();
    match warning {
        IonexWarning::MissingRecord(_) => {
            info.kind = SidereonIonexWarningKind::MissingRecord;
        }
        IonexWarning::VersionRecordNotFirst { line } => {
            info.kind = SidereonIonexWarningKind::VersionRecordNotFirst;
            info.line = *line;
        }
        IonexWarning::EpochMismatch {
            line,
            declared,
            maps,
            ..
        } => {
            info.kind = SidereonIonexWarningKind::EpochMismatch;
            info.line = *line;
            // The reader builds both epochs from whole seconds, so both read
            // back exactly; the flag stays false for an epoch in any other
            // form rather than writing a NaN a caller could read as a value.
            if let (Some(declared_s), Some(maps_s)) = (
                reader_epoch_whole_second(declared),
                reader_epoch_whole_second(maps),
            ) {
                info.has_epochs = true;
                info.declared_epoch_j2000_whole_s = declared_s;
                info.maps_epoch_j2000_whole_s = maps_s;
                info.declared_epoch_j2000_s = declared_s as f64;
                info.maps_epoch_j2000_s = maps_s as f64;
            }
        }
        IonexWarning::MapCountMismatch {
            line,
            declared,
            tec_maps,
            all_maps,
        } => {
            info.kind = SidereonIonexWarningKind::MapCountMismatch;
            info.line = *line;
            info.declared_count = *declared;
            info.tec_map_count = *tec_maps;
            info.all_map_count = *all_maps;
        }
        IonexWarning::NotANumberValue {
            map_number,
            line,
            lat_deg,
            lon_deg,
            ..
        } => {
            info.kind = SidereonIonexWarningKind::NotANumberValue;
            info.map_number = *map_number;
            info.line = *line;
            info.lat_deg = *lat_deg;
            info.lon_deg = *lon_deg;
        }
        IonexWarning::IntervalMismatch {
            line,
            declared_s,
            map_number,
            spacing_s,
        } => {
            info.kind = SidereonIonexWarningKind::IntervalMismatch;
            info.line = *line;
            info.declared_interval_s = *declared_s;
            info.map_number = *map_number;
            info.actual_spacing_s = *spacing_s;
        }
        IonexWarning::ExponentCarriedIntoMap {
            map_number,
            line,
            exponent,
            set_by_line,
            ..
        } => {
            info.kind = SidereonIonexWarningKind::ExponentCarriedIntoMap;
            info.map_number = *map_number;
            info.line = *line;
            info.exponent = *exponent;
            info.set_by_line = *set_by_line;
        }
        _ => {
            info.kind = SidereonIonexWarningKind::Unknown;
        }
    }
    info
}

fn map_iono_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
    }
}

fn flatten_grid_values(maps: &[Vec<Vec<Option<f64>>>]) -> Vec<f64> {
    maps.iter()
        .flat_map(|map| {
            map.iter()
                .flat_map(|row| row.iter().map(|v| v.unwrap_or(f64::NAN)))
        })
        .collect()
}

fn flatten_grid_presence(maps: &[Vec<Vec<Option<f64>>>]) -> Vec<bool> {
    maps.iter()
        .flat_map(|map| map.iter().flat_map(|row| row.iter().map(|v| v.is_some())))
        .collect()
}

unsafe fn string_from_c_bytes(
    fn_name: &str,
    data: *const u8,
    len: usize,
) -> Result<String, SidereonStatus> {
    if len == 0 {
        return Ok(String::new());
    }
    let slice = require_slice(data, len, fn_name, "text")?;
    std::str::from_utf8(slice)
        .map(|s| s.to_string())
        .map_err(|_| {
            set_last_error(format!("{fn_name}: invalid UTF-8 bytes"));
            SidereonStatus::InvalidArgument
        })
}

unsafe fn require_out_slice<T>(
    ptr: *mut T,
    len: usize,
    fn_name: &str,
    arg_name: &str,
) -> Result<(), SidereonStatus> {
    if len == 0 {
        return Ok(());
    }
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name} pointer"));
        return Err(SidereonStatus::NullPointer);
    }
    validate_element_count::<T>(fn_name, arg_name, len)?;
    Ok(())
}

fn convert_slant_requests(
    fn_name: &str,
    requests: &[SidereonIonexSlantRequest],
) -> Result<Vec<IonexSlantRequest>, SidereonStatus> {
    let mut out = Vec::with_capacity(requests.len());
    for request in requests {
        let core_request = slant_request_to_core(fn_name, request).map_err(|message| {
            set_last_error(message);
            SidereonStatus::InvalidArgument
        })?;
        out.push(core_request);
    }
    Ok(out)
}

/// Marshal one C slant-delay row into the engine request, converting the
/// degree-valued receiver position and line of sight to the radians the engine
/// takes. The error is the message the engine gave for the receiver position,
/// which a batch row keeps rather than only leaving in the thread-local slot.
fn slant_request_to_core(
    fn_name: &str,
    request: &SidereonIonexSlantRequest,
) -> Result<IonexSlantRequest, String> {
    let receiver = Wgs84Geodetic::new(
        request.lat_deg * IONO_DEG_TO_RAD,
        request.lon_deg * IONO_DEG_TO_RAD,
        0.0,
    )
    .map_err(|err| format!("{fn_name}: receiver: {err}"))?;
    Ok(IonexSlantRequest::new(
        receiver,
        request.elevation_deg * IONO_DEG_TO_RAD,
        request.azimuth_deg * IONO_DEG_TO_RAD,
        ionex_epoch_from_j2000_seconds(request.epoch_j2000_s),
        request.frequency_hz,
    ))
}

/// The typed detail of a sample construction that reports no failure.
fn no_tec_samples_error() -> SidereonTecSamplesError {
    SidereonTecSamplesError {
        kind: SidereonTecSamplesErrorKind::None,
        input: SidereonTecSamplesInput::None,
        has_index: false,
        index: 0,
        node_count: 0,
        value_count: 0,
        expected_value_count: 0,
        has_axis_value: false,
        axis_value: 0.0,
    }
}

/// A typed refusal this binding's marshalling found in one indexed input.
fn indexed_samples_refusal(
    kind: SidereonTecSamplesErrorKind,
    input: SidereonTecSamplesInput,
    index: usize,
    message: String,
) -> TecSamplesFailure {
    TecSamplesFailure::Refused {
        error: SidereonTecSamplesError {
            kind,
            input,
            has_index: true,
            index,
            ..no_tec_samples_error()
        },
        message,
    }
}

/// A typed refusal for a value count that disagrees with the axes.
fn samples_count_refusal(
    fn_name: &str,
    kind: SidereonTecSamplesErrorKind,
    field: &str,
    value_count: usize,
    expected_value_count: usize,
) -> TecSamplesFailure {
    TecSamplesFailure::Refused {
        error: SidereonTecSamplesError {
            kind,
            value_count,
            expected_value_count,
            ..no_tec_samples_error()
        },
        message: format!(
            "{fn_name}: {field} is {value_count}, but the axes give {expected_value_count} values"
        ),
    }
}

/// The typed detail of an engine sample-construction refusal. The engine names
/// no index, so none is claimed.
fn tec_samples_error_to_c(err: &TecSamplesError) -> SidereonTecSamplesError {
    let of = |kind| SidereonTecSamplesError {
        kind,
        ..no_tec_samples_error()
    };
    match err {
        TecSamplesError::Empty => of(SidereonTecSamplesErrorKind::Empty),
        TecSamplesError::TooFewNodes(count) => SidereonTecSamplesError {
            node_count: *count,
            ..of(SidereonTecSamplesErrorKind::TooFewNodes)
        },
        TecSamplesError::NonMonotonicLat => of(SidereonTecSamplesErrorKind::NonMonotonicLat),
        TecSamplesError::NonMonotonicLon => of(SidereonTecSamplesErrorKind::NonMonotonicLon),
        TecSamplesError::NonMonotonicEpochs => of(SidereonTecSamplesErrorKind::NonMonotonicEpochs),
        TecSamplesError::EpochNotRepresentable(_) => {
            of(SidereonTecSamplesErrorKind::EpochNotRepresentable)
        }
        TecSamplesError::ShapeMismatch => of(SidereonTecSamplesErrorKind::ShapeMismatch),
        TecSamplesError::RmsCountMismatch => of(SidereonTecSamplesErrorKind::RmsCountMismatch),
        TecSamplesError::HeightCountMismatch => {
            of(SidereonTecSamplesErrorKind::HeightCountMismatch)
        }
        TecSamplesError::NonFiniteValue => of(SidereonTecSamplesErrorKind::NonFiniteValue),
        TecSamplesError::NonPositiveStep => of(SidereonTecSamplesErrorKind::NonPositiveStep),
        TecSamplesError::AxisOutOfRange(value) => SidereonTecSamplesError {
            has_axis_value: true,
            axis_value: *value,
            ..of(SidereonTecSamplesErrorKind::AxisOutOfRange)
        },
    }
}

/// An engine sample-construction refusal as a typed failure with owned text.
fn tec_samples_core_failure(fn_name: &str, err: &TecSamplesError) -> TecSamplesFailure {
    TecSamplesFailure::Refused {
        error: tec_samples_error_to_c(err),
        message: format!("{fn_name}: {err}"),
    }
}

/// The status a convenience route returns for a failed sample construction.
/// A typed refusal leaves its text in the thread-local message.
fn tec_samples_failure_status(failure: TecSamplesFailure) -> SidereonStatus {
    match failure {
        TecSamplesFailure::Structural(status) => status,
        TecSamplesFailure::Refused { message, .. } => {
            set_last_error(message);
            SidereonStatus::InvalidArgument
        }
    }
}

/// Hand a sample construction's product and its owned record to the caller.
///
/// A structural failure transfers neither and is returned as its status. A
/// built product and a typed refusal both transfer a result, and only the
/// first transfers a product, so the caller never sees a product without its
/// record and no record is orphaned behind a failing status.
unsafe fn transfer_tec_samples_result(
    built: Result<Ionex, TecSamplesFailure>,
    out_ionex: *mut *mut SidereonIonex,
    out_result: *mut *mut SidereonTecSamplesResult,
) -> Result<(), SidereonStatus> {
    let result = match built {
        Ok(inner) => {
            write_boxed_handle(out_ionex, SidereonIonex { inner });
            SidereonTecSamplesResult {
                outcome: SidereonTecSamplesOutcome {
                    is_ok: true,
                    status: SidereonStatus::Ok,
                    error: no_tec_samples_error(),
                },
                message: String::new(),
            }
        }
        Err(TecSamplesFailure::Structural(status)) => return Err(status),
        Err(TecSamplesFailure::Refused { error, message }) => SidereonTecSamplesResult {
            outcome: SidereonTecSamplesOutcome {
                is_ok: false,
                status: SidereonStatus::InvalidArgument,
                error,
            },
            message,
        },
    };
    write_boxed_handle(out_result, result);
    Ok(())
}

/// Marshal a caller's whole-grid samples and build the product.
unsafe fn build_ionex_from_grid_samples(
    fn_name: &str,
    samples: *const SidereonTecGridSamples,
) -> Result<Ionex, TecSamplesFailure> {
    let samples = require_ref(samples, fn_name, "samples")?;
    let core_samples = tec_grid_samples_from_c(fn_name, samples)?;
    Ionex::from_samples(core_samples).map_err(|err| tec_samples_core_failure(fn_name, &err))
}

/// Marshal a caller's per-node samples and build the product.
unsafe fn build_ionex_from_node_samples(
    fn_name: &str,
    samples: *const SidereonTecSample,
    count: usize,
    shell_height_km: f64,
    base_radius_km: f64,
    exponent: i32,
    header: *const SidereonIonexHeader,
) -> Result<Ionex, TecSamplesFailure> {
    let raw = require_slice(samples, count, fn_name, "samples")?;
    let core_header = if header.is_null() {
        undeclared_ionex_header()
    } else {
        require_ref(header, fn_name, "header")?.inner.clone()
    };
    let mut parsed = Vec::with_capacity(raw.len());
    for (index, sample) in raw.iter().enumerate() {
        parsed.push(tec_sample_from_c(fn_name, index, sample)?);
    }
    Ionex::from_node_samples(
        parsed,
        shell_height_km,
        base_radius_km,
        exponent,
        core_header,
    )
    .map_err(|err| tec_samples_core_failure(fn_name, &err))
}

/// The IONEX map epoch a whole J2000 second names, in `scale`.
///
/// It is built as the engine's IONEX reader builds its own epochs, from the
/// split Julian date `split_julian_date_from_j2000_seconds` gives, so a
/// product built from samples holds the same epoch a product read from a file
/// holds for that second. That split always has a finite boundary and a
/// fraction in `[0, 1)`; the integer-nanosecond form, which the engine reads
/// exactly too, stands in only if a split were ever refused.
fn ionex_epoch_from_whole_second(scale: TimeScale, seconds: i64) -> Instant {
    let (jd_whole, fraction) = split_julian_date_from_j2000_seconds(seconds);
    match JulianDateSplit::new(jd_whole, fraction) {
        Ok(split) => Instant::from_julian_date(scale, split),
        Err(_) => Instant::from_nanos(scale, i128::from(seconds) * 1_000_000_000),
    }
}

/// The whole J2000 second a caller's double states, or why it states none.
///
/// Nothing is rounded: a double holding a fraction of a second names no IONEX
/// map epoch, and the engine would refuse the epoch the fraction gives as
/// EpochNotRepresentable. It is refused here, where its index is known, and
/// not converted into a split first, since adding a small fraction to a
/// residual day fraction can round back onto the whole second and so accept
/// an epoch the caller did not state.
fn whole_second_from_f64(value: f64) -> Result<i64, &'static str> {
    // -2^63 is exactly a double and an int64_t; 2^63 is the first double past
    // int64_t, so the range test is exact at both ends.
    const I64_LOWER: f64 = -9_223_372_036_854_775_808.0;
    const I64_UPPER: f64 = 9_223_372_036_854_775_808.0;
    if !value.is_finite() {
        return Err("is not finite");
    }
    if value.fract() != 0.0 {
        return Err("is not a whole second");
    }
    if !(I64_LOWER..I64_UPPER).contains(&value) {
        return Err("is outside the int64_t second range");
    }
    // Integral and inside the range just checked, so the cast is exact.
    Ok(value as i64)
}

/// The whole J2000 second an epoch the engine's IONEX reader built names, or
/// `None` for an epoch in another form.
///
/// The engine's own exact converter is not public, so this reads back only the
/// two forms an IONEX epoch takes: integer nanoseconds that are a whole number
/// of seconds, and the split Julian date `split_julian_date_from_j2000_seconds`
/// gives, which is how the reader builds every epoch it reads. The candidate
/// second is accepted only when encoding it again gives the same two doubles
/// bit for bit, so a value is never inferred from proximity.
fn reader_epoch_whole_second(epoch: &Instant) -> Option<i64> {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    match epoch.repr {
        InstantRepr::Nanos(nanos) => {
            if nanos.rem_euclid(NANOS_PER_SECOND) != 0 {
                return None;
            }
            i64::try_from(nanos.div_euclid(NANOS_PER_SECOND)).ok()
        }
        InstantRepr::JulianDate(split) => {
            let (origin, _) = split_julian_date_from_j2000_seconds(0);
            let days = split.jd_whole - origin;
            // The encoder's day count is an integer well inside 2^53, where the
            // subtraction is exact; anything else is not its form.
            if !days.is_finite() || days.fract() != 0.0 || days.abs() >= 9.007_199_254_740_992e15 {
                return None;
            }
            let residual = (split.fraction * 86_400.0).round();
            if !(0.0..86_400.0).contains(&residual) {
                return None;
            }
            let seconds = (days as i128)
                .checked_mul(86_400)?
                .checked_add(residual as i128)?;
            let seconds = i64::try_from(seconds).ok()?;
            let (jd_whole, fraction) = split_julian_date_from_j2000_seconds(seconds);
            (jd_whole.to_bits() == split.jd_whole.to_bits()
                && fraction.to_bits() == split.fraction.to_bits())
            .then_some(seconds)
        }
    }
}

unsafe fn tec_grid_samples_from_c(
    fn_name: &str,
    samples: &SidereonTecGridSamples,
) -> Result<CoreTecGridSamples, TecSamplesFailure> {
    let scale = time_scale_from_c_code(fn_name, "samples.time_scale", samples.time_scale)?;
    let map_epochs = if samples.map_epochs_j2000_whole_s.is_null() {
        let epochs_s = require_slice(
            samples.map_epochs_j2000_s,
            samples.map_epoch_count,
            fn_name,
            "samples.map_epochs_j2000_s",
        )?;
        let mut map_epochs = Vec::with_capacity(epochs_s.len());
        for (index, &epoch_s) in epochs_s.iter().enumerate() {
            let seconds = whole_second_from_f64(epoch_s).map_err(|reason| {
                indexed_samples_refusal(
                    SidereonTecSamplesErrorKind::EpochNotRepresentable,
                    SidereonTecSamplesInput::MapEpoch,
                    index,
                    format!(
                        "{fn_name}: samples.map_epochs_j2000_s[{index}] = {epoch_s:?} {reason}, \
                         so it names no IONEX map epoch"
                    ),
                )
            })?;
            map_epochs.push(ionex_epoch_from_whole_second(scale, seconds));
        }
        map_epochs
    } else {
        let epochs_s = require_slice(
            samples.map_epochs_j2000_whole_s,
            samples.map_epoch_count,
            fn_name,
            "samples.map_epochs_j2000_whole_s",
        )?;
        epochs_s
            .iter()
            .map(|&seconds| ionex_epoch_from_whole_second(scale, seconds))
            .collect()
    };
    let lat_nodes = require_slice(
        samples.lat_nodes_deg,
        samples.lat_node_count,
        fn_name,
        "samples.lat_nodes_deg",
    )?;
    let lon_nodes = require_slice(
        samples.lon_nodes_deg,
        samples.lon_node_count,
        fn_name,
        "samples.lon_nodes_deg",
    )?;
    let expected = checked_tec_grid_value_count(
        fn_name,
        samples.map_epoch_count,
        samples.lat_node_count,
        samples.lon_node_count,
    )?;
    if samples.tec_map_value_count != expected {
        return Err(samples_count_refusal(
            fn_name,
            SidereonTecSamplesErrorKind::ShapeMismatch,
            "samples.tec_map_value_count",
            samples.tec_map_value_count,
            expected,
        ));
    }
    let tec_flat = require_slice(
        samples.tec_maps_tecu,
        samples.tec_map_value_count,
        fn_name,
        "samples.tec_maps_tecu",
    )?;
    let tec_presence = if samples.tec_maps_present.is_null() {
        None
    } else {
        Some(require_slice(
            samples.tec_maps_present,
            samples.tec_map_value_count,
            fn_name,
            "samples.tec_maps_present",
        )?)
    };
    let dimensions = (
        samples.map_epoch_count,
        samples.lat_node_count,
        samples.lon_node_count,
    );
    let tec_maps = nested_tec_maps_with_presence(
        fn_name,
        "samples.tec_maps_tecu",
        SidereonTecSamplesInput::TecValue,
        tec_flat,
        tec_presence,
        dimensions,
    )?;

    let rms_maps = if samples.has_rms_maps {
        if samples.rms_map_value_count != expected {
            return Err(samples_count_refusal(
                fn_name,
                SidereonTecSamplesErrorKind::RmsCountMismatch,
                "samples.rms_map_value_count",
                samples.rms_map_value_count,
                expected,
            ));
        }
        let rms_flat = require_slice(
            samples.rms_maps_tecu,
            samples.rms_map_value_count,
            fn_name,
            "samples.rms_maps_tecu",
        )?;
        let rms_presence = if samples.rms_maps_present.is_null() {
            None
        } else {
            Some(require_slice(
                samples.rms_maps_present,
                samples.rms_map_value_count,
                fn_name,
                "samples.rms_maps_present",
            )?)
        };
        nested_tec_maps_with_presence(
            fn_name,
            "samples.rms_maps_tecu",
            SidereonTecSamplesInput::RmsValue,
            rms_flat,
            rms_presence,
            dimensions,
        )?
    } else {
        Vec::new()
    };

    let height_maps = if samples.has_height_maps {
        if samples.height_map_value_count != expected {
            return Err(samples_count_refusal(
                fn_name,
                SidereonTecSamplesErrorKind::HeightCountMismatch,
                "samples.height_map_value_count",
                samples.height_map_value_count,
                expected,
            ));
        }
        let height_flat = require_slice(
            samples.height_maps_km,
            samples.height_map_value_count,
            fn_name,
            "samples.height_maps_km",
        )?;
        let height_presence = if samples.height_maps_present.is_null() {
            None
        } else {
            Some(require_slice(
                samples.height_maps_present,
                samples.height_map_value_count,
                fn_name,
                "samples.height_maps_present",
            )?)
        };
        nested_tec_maps_with_presence(
            fn_name,
            "samples.height_maps_km",
            SidereonTecSamplesInput::HeightValue,
            height_flat,
            height_presence,
            dimensions,
        )?
    } else {
        Vec::new()
    };

    let header = if samples.header.is_null() {
        undeclared_ionex_header()
    } else {
        let h = require_ref(samples.header, fn_name, "samples.header")?;
        h.inner.clone()
    };

    Ok(CoreTecGridSamples {
        map_epochs,
        lat_nodes_deg: lat_nodes.to_vec(),
        lon_nodes_deg: lon_nodes.to_vec(),
        dlat_deg: samples.dlat_deg,
        dlon_deg: samples.dlon_deg,
        shell_height_km: samples.shell_height_km,
        base_radius_km: samples.base_radius_km,
        exponent: samples.exponent,
        tec_maps,
        rms_maps,
        height_maps,
        header,
    })
}

/// Nest one flat `[map][lat][lon]` value buffer, taking a node as missing
/// where its presence flag is false and refusing a present value that is not
/// finite by its flat index.
fn nested_tec_maps_with_presence(
    fn_name: &str,
    field: &str,
    input: SidereonTecSamplesInput,
    flat: &[f64],
    presence: Option<&[bool]>,
    (map_count, lat_count, lon_count): (usize, usize, usize),
) -> Result<Vec<Vec<Vec<Option<f64>>>>, TecSamplesFailure> {
    let mut maps = Vec::with_capacity(map_count);
    for map_idx in 0..map_count {
        let mut rows = Vec::with_capacity(lat_count);
        for lat_idx in 0..lat_count {
            let offset = (map_idx * lat_count + lat_idx) * lon_count;
            let mut row = Vec::with_capacity(lon_count);
            for lon_idx in 0..lon_count {
                let idx = offset + lon_idx;
                let is_present = match presence {
                    Some(mask) => mask[idx],
                    None => true,
                };
                if is_present {
                    let val = flat[idx];
                    if !val.is_finite() {
                        return Err(indexed_samples_refusal(
                            SidereonTecSamplesErrorKind::NonFiniteValue,
                            input,
                            idx,
                            format!(
                                "{fn_name}: {field}[{idx}] is marked present and is not finite"
                            ),
                        ));
                    }
                    row.push(Some(val));
                } else {
                    row.push(None);
                }
            }
            rows.push(row);
        }
        maps.push(rows);
    }
    Ok(maps)
}

/// One engine sample marshalled out to C with the whole second its map epoch
/// names. A node without a value writes NaN with its presence flag false; the
/// flag is the authority, and the NaN is a placeholder, never a measured value.
fn tec_sample_to_c(sample: &CoreTecSample, epoch_s: i64) -> SidereonTecSample {
    SidereonTecSample {
        time_scale: time_scale_to_c_code(sample.epoch.scale),
        epoch_j2000_s: epoch_s as f64,
        has_epoch_j2000_whole_s: true,
        epoch_j2000_whole_s: epoch_s,
        lat_deg: sample.lat_deg,
        lon_deg: sample.lon_deg,
        has_vtec_tecu: sample.vtec_tecu.is_some(),
        vtec_tecu: sample.vtec_tecu.unwrap_or(f64::NAN),
        has_rms_tecu: sample.rms_tecu.is_some(),
        rms_tecu: sample.rms_tecu.unwrap_or(f64::NAN),
        has_height_offset_km: sample.height_offset_km.is_some(),
        height_offset_km: sample.height_offset_km.unwrap_or(f64::NAN),
    }
}

/// Every per-node sample of a product, each carrying the exact whole second
/// of its map from `Ionex::map_epochs_s`.
///
/// `Ionex::tec_samples` emits the nodes map by map, one full latitude by
/// longitude grid per map epoch, so the samples fall into consecutive chunks
/// of that many nodes, one per entry of the epoch axis.
fn tec_samples_to_c(ionex: &Ionex) -> Vec<SidereonTecSample> {
    let epochs_s = ionex.map_epochs_s();
    let per_map = ionex.lat_nodes_deg().len() * ionex.lon_nodes_deg().len();
    let samples = ionex.tec_samples();
    if per_map == 0 {
        return Vec::new();
    }
    samples
        .chunks(per_map)
        .zip(epochs_s)
        .flat_map(|(chunk, epoch_s)| chunk.iter().map(move |s| tec_sample_to_c(s, epoch_s)))
        .collect()
}

unsafe fn tec_sample_from_c(
    fn_name: &str,
    index: usize,
    sample: &SidereonTecSample,
) -> Result<CoreTecSample, TecSamplesFailure> {
    let scale = time_scale_from_c_code(fn_name, "sample.time_scale", sample.time_scale)?;
    let seconds = if sample.has_epoch_j2000_whole_s {
        sample.epoch_j2000_whole_s
    } else {
        whole_second_from_f64(sample.epoch_j2000_s).map_err(|reason| {
            indexed_samples_refusal(
                SidereonTecSamplesErrorKind::EpochNotRepresentable,
                SidereonTecSamplesInput::SampleEpoch,
                index,
                format!(
                    "{fn_name}: samples[{index}].epoch_j2000_s = {:?} {reason}, so it names no \
                     IONEX map epoch",
                    sample.epoch_j2000_s
                ),
            )
        })?
    };
    let epoch = ionex_epoch_from_whole_second(scale, seconds);
    let present = |has: bool,
                   value: f64,
                   field: &str,
                   input: SidereonTecSamplesInput|
     -> Result<Option<f64>, TecSamplesFailure> {
        if !has {
            return Ok(None);
        }
        if !value.is_finite() {
            return Err(indexed_samples_refusal(
                SidereonTecSamplesErrorKind::NonFiniteValue,
                input,
                index,
                format!("{fn_name}: samples[{index}].{field} is marked present and is not finite"),
            ));
        }
        Ok(Some(value))
    };
    let vtec_tecu = present(
        sample.has_vtec_tecu,
        sample.vtec_tecu,
        "vtec_tecu",
        SidereonTecSamplesInput::SampleVtec,
    )?;
    let rms_tecu = present(
        sample.has_rms_tecu,
        sample.rms_tecu,
        "rms_tecu",
        SidereonTecSamplesInput::SampleRms,
    )?;
    let height_offset_km = present(
        sample.has_height_offset_km,
        sample.height_offset_km,
        "height_offset_km",
        SidereonTecSamplesInput::SampleHeight,
    )?;
    Ok(CoreTecSample {
        epoch,
        lat_deg: sample.lat_deg,
        lon_deg: sample.lon_deg,
        vtec_tecu,
        rms_tecu,
        height_offset_km,
    })
}

fn tec_grid_samples_info(samples: &CoreTecGridSamples) -> SidereonTecGridSamplesInfo {
    // Counted from the stored grids themselves, so the info a caller sizes its
    // buffers from is the same count the copy routes report.
    let flattened = |maps: &[Vec<Vec<Option<f64>>>]| -> usize {
        maps.iter()
            .map(|map| map.iter().map(Vec::len).sum::<usize>())
            .sum()
    };
    let tec_map_value_count = flattened(&samples.tec_maps);
    let has_rms_maps = !samples.rms_maps.is_empty();
    let rms_map_value_count = flattened(&samples.rms_maps);
    let has_height_maps = !samples.height_maps.is_empty();
    let height_map_value_count = flattened(&samples.height_maps);
    SidereonTecGridSamplesInfo {
        map_epoch_count: samples.map_epochs.len(),
        lat_node_count: samples.lat_nodes_deg.len(),
        lon_node_count: samples.lon_nodes_deg.len(),
        dlat_deg: samples.dlat_deg,
        dlon_deg: samples.dlon_deg,
        shell_height_km: samples.shell_height_km,
        base_radius_km: samples.base_radius_km,
        exponent: samples.exponent,
        has_rms_maps,
        tec_map_value_count,
        rms_map_value_count,
        has_height_maps,
        height_map_value_count,
    }
}

fn nequick_g_ray_from_c(ray: &SidereonNequickGRay) -> NequickGRayEval {
    NequickGRayEval {
        month: ray.month,
        utc_hours: ray.utc_hours,
        station_lon_deg: ray.station_lon_deg,
        station_lat_deg: ray.station_lat_deg,
        station_height_m: ray.station_height_m,
        satellite_lon_deg: ray.satellite_lon_deg,
        satellite_lat_deg: ray.satellite_lat_deg,
        satellite_height_m: ray.satellite_height_m,
    }
}

fn checked_tec_grid_value_count(
    fn_name: &str,
    map_count: usize,
    lat_count: usize,
    lon_count: usize,
) -> Result<usize, SidereonStatus> {
    let map_lat = map_count.checked_mul(lat_count).ok_or_else(|| {
        set_last_error(format!("{fn_name}: IONEX grid dimensions overflow"));
        SidereonStatus::InvalidArgument
    })?;
    let total = map_lat.checked_mul(lon_count).ok_or_else(|| {
        set_last_error(format!("{fn_name}: IONEX grid dimensions overflow"));
        SidereonStatus::InvalidArgument
    })?;
    validate_element_count::<f64>(fn_name, "IONEX grid values", total)?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-map product in the fixed IONEX 1 columns, with a declared `COSZ`
    /// mapping function, description and comment records, and both optional
    /// station and satellite counts.
    const VALID_IONEX_FIXTURE: &str = concat!(
        "     1.0            IONOSPHERE MAPS     GPS                 IONEX VERSION / TYPE\n",
        "SIDEREON            TEST                20200101 000000     PGM / RUN BY / DATE\n",
        "SEVEN BY SEVEN TEST GRID                                    DESCRIPTION\n",
        "SECOND DESCRIPTION LINE                                     DESCRIPTION\n",
        "A HEADER COMMENT                                            COMMENT\n",
        "  2020     1     1     0     0     0                        EPOCH OF FIRST MAP\n",
        "  2020     1     1     1     0     0                        EPOCH OF LAST MAP\n",
        "  3600                                                      INTERVAL\n",
        "     2                                                      # OF MAPS IN FILE\n",
        "  COSZ                                                      MAPPING FUNCTION\n",
        "    10.0                                                    ELEVATION CUTOFF\n",
        "  GPS L1 L2 PHASE                                           OBSERVABLES USED\n",
        "    12                                                      # OF STATIONS\n",
        "    31                                                      # OF SATELLITES\n",
        "  6371.0                                                    BASE RADIUS\n",
        "     2                                                      MAP DIMENSION\n",
        "   450.0 450.0   0.0                                        HGT1 / HGT2 / DHGT\n",
        "    60.0 -60.0 -20.0                                        LAT1 / LAT2 / DLAT\n",
        "  -180.0 180.0  60.0                                        LON1 / LON2 / DLON\n",
        "    -1                                                      EXPONENT\n",
        "                                                            END OF HEADER\n",
        "     1                                                      START OF TEC MAP\n",
        "  2020     1     1     0     0     0                        EPOCH OF CURRENT MAP\n",
        "    60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  100  101  102  103  104  105  106\n",
        "    40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  110  111  112  113  114  115  116\n",
        "    20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  120  121  122  123  124  125  126\n",
        "     0.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  130  131  132  133  134  135  136\n",
        "   -20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  140  141  142  143  144  145  146\n",
        "   -40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  150  151  152  153  154  155  156\n",
        "   -60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  160  161  162  163  164  165  166\n",
        "     1                                                      END OF TEC MAP\n",
        "     2                                                      START OF TEC MAP\n",
        "  2020     1     1     1     0     0                        EPOCH OF CURRENT MAP\n",
        "    60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  200  201  202  203  204  205  206\n",
        "    40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  210  211  212  213  214  215  216\n",
        "    20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  220  221  222  223  224  225  226\n",
        "     0.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  230  231  232  233  234  235  236\n",
        "   -20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  240  241  242  243  244  245  246\n",
        "   -40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  250  251  252  253  254  255  256\n",
        "   -60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
        "  260  261  262  263  264  265  266\n",
        "     2                                                      END OF TEC MAP\n",
        "                                                            END OF FILE\n",
    );

    /// The first epoch of [`VALID_IONEX_FIXTURE`], 2020-01-01 00:00:00, in
    /// seconds since J2000.
    const FIXTURE_FIRST_EPOCH_J2000_S: i64 = 631_108_800;

    /// A two-by-two-by-two sample grid the tests build products from.
    struct GridFixture {
        epochs: [f64; 2],
        lats: [f64; 2],
        lons: [f64; 2],
        tec_values: [f64; 8],
        tec_presence: [bool; 8],
        rms_values: [f64; 8],
        rms_presence: [bool; 8],
        height_values: [f64; 8],
        height_presence: [bool; 8],
    }

    impl GridFixture {
        fn new() -> Self {
            Self {
                epochs: [
                    FIXTURE_FIRST_EPOCH_J2000_S as f64,
                    FIXTURE_FIRST_EPOCH_J2000_S as f64 + 3600.0,
                ],
                lats: [40.0, -40.0],
                lons: [-20.0, 20.0],
                tec_values: [10.0; 8],
                tec_presence: [true; 8],
                rms_values: [1.0; 8],
                rms_presence: [true; 8],
                height_values: [0.0; 8],
                height_presence: [true; 8],
            }
        }

        /// The C sample grid, borrowing this fixture's buffers. `header` may be
        /// null, which builds the product on a header declaring no mapping
        /// function; see `an_unsupplied_header_declares_no_mapping_function`.
        fn samples(
            &self,
            has_rms: bool,
            has_height: bool,
            header: *const SidereonIonexHeader,
        ) -> SidereonTecGridSamples {
            SidereonTecGridSamples {
                time_scale: SidereonTimeScale::Utc as u32,
                map_epochs_j2000_s: self.epochs.as_ptr(),
                map_epochs_j2000_whole_s: ptr::null(),
                map_epoch_count: 2,
                lat_nodes_deg: self.lats.as_ptr(),
                lat_node_count: 2,
                lon_nodes_deg: self.lons.as_ptr(),
                lon_node_count: 2,
                dlat_deg: -80.0,
                dlon_deg: 40.0,
                shell_height_km: 450.0,
                base_radius_km: 6371.0,
                exponent: -1,
                tec_maps_tecu: self.tec_values.as_ptr(),
                tec_maps_present: self.tec_presence.as_ptr(),
                tec_map_value_count: 8,
                has_rms_maps: has_rms,
                rms_maps_tecu: self.rms_values.as_ptr(),
                rms_maps_present: self.rms_presence.as_ptr(),
                rms_map_value_count: if has_rms { 8 } else { 0 },
                has_height_maps: has_height,
                height_maps_km: self.height_values.as_ptr(),
                height_maps_present: self.height_presence.as_ptr(),
                height_map_value_count: if has_height { 8 } else { 0 },
                header,
            }
        }
    }

    unsafe fn build_product(samples: &SidereonTecGridSamples) -> *mut SidereonIonex {
        let mut ionex: *mut SidereonIonex = ptr::null_mut();
        assert_eq!(
            sidereon_ionex_from_tec_grid_samples(samples, &mut ionex),
            SidereonStatus::Ok
        );
        assert!(!ionex.is_null());
        ionex
    }

    unsafe fn copy_text(
        read: impl Fn(*mut u8, usize, *mut usize, *mut usize) -> SidereonStatus,
    ) -> String {
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            read(ptr::null_mut(), 0, &mut written, &mut required),
            SidereonStatus::Ok
        );
        let mut buffer = vec![0u8; required];
        assert_eq!(
            read(buffer.as_mut_ptr(), required, &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        String::from_utf8(buffer).expect("engine text is UTF-8")
    }

    fn policy(coverage: u32, missing_nodes: u32, mapping: u32) -> SidereonIonexSlantPolicy {
        sidereon_ionex_slant_policy_init(coverage, missing_nodes, mapping)
    }

    /// sidereon-core's own slant evaluation of the product the handle holds,
    /// for the inputs a C slant call takes (degrees, as the route converts
    /// them).
    #[allow(clippy::too_many_arguments)]
    unsafe fn core_slant(
        ionex: *const SidereonIonex,
        lat_deg: f64,
        lon_deg: f64,
        azimuth_deg: f64,
        elevation_deg: f64,
        epoch_j2000_s: i64,
        frequency_hz: f64,
        policy: SidereonIonexSlantPolicy,
    ) -> Result<IonexSlantDelayEvaluation, CoreError> {
        let receiver = geodetic_to_wgs84(
            "test",
            "receiver",
            SidereonGeodetic {
                lat_rad: lat_deg * IONO_DEG_TO_RAD,
                lon_rad: lon_deg * IONO_DEG_TO_RAD,
                height_m: 0.0,
            },
        )
        .expect("receiver");
        ionex_slant_delay_with_policy(
            &(*ionex).inner,
            receiver,
            elevation_deg * IONO_DEG_TO_RAD,
            azimuth_deg * IONO_DEG_TO_RAD,
            ionex_epoch_from_j2000_seconds(epoch_j2000_s),
            frequency_hz,
            ionex_slant_policy_from_c("test", policy).expect("policy"),
        )
    }

    /// The gap sidereon-core's strict standalone-grid evaluation names at the
    /// query the gap tests make (epoch 250 ns, latitude 0, longitude 30).
    unsafe fn core_strict_grid_gap(grid: *const SidereonTecGrid) -> IonexNodeGap {
        match (*grid).inner.vtec_at_pierce_point_with_policy(
            TecGridEpoch::new(250, 0),
            30.0,
            0.0,
            IonexMissingNodePolicy::Strict,
        ) {
            Err(TecGridError::NodesNotAvailable(gap)) => gap,
            Err(other) => panic!("sidereon-core refuses otherwise: {other:?}"),
            Ok(_) => panic!("sidereon-core evaluates across the missing node"),
        }
    }

    /// sidereon-core's own strict standalone-grid value at a query.
    unsafe fn core_grid_value(
        grid: *const SidereonTecGrid,
        unix_nanos: i64,
        lat_deg: f64,
        lon_deg: f64,
    ) -> Result<f64, TecGridError> {
        (*grid)
            .inner
            .vtec_at_pierce_point_with_policy(
                TecGridEpoch::new(unix_nanos, 0),
                lon_deg,
                lat_deg,
                IonexMissingNodePolicy::Strict,
            )
            .map(|evaluation| evaluation.value)
    }

    /// The coordinate sidereon-core's out-of-bounds refusal names.
    unsafe fn core_out_of_bounds_value(
        grid: *const SidereonTecGrid,
        unix_nanos: i64,
        lat_deg: f64,
        lon_deg: f64,
    ) -> f64 {
        match core_grid_value(grid, unix_nanos, lat_deg, lon_deg) {
            Err(TecGridError::OutOfBounds { value, .. }) => value,
            other => panic!("sidereon-core does not refuse out of bounds: {other:?}"),
        }
    }

    /// sidereon-core's own delay for one C slant request under a policy.
    unsafe fn core_request_delay(
        ionex: *const SidereonIonex,
        request: &SidereonIonexSlantRequest,
        policy: SidereonIonexSlantPolicy,
    ) -> f64 {
        core_slant(
            ionex,
            request.lat_deg,
            request.lon_deg,
            request.azimuth_deg,
            request.elevation_deg,
            request.epoch_j2000_s,
            request.frequency_hz,
            policy,
        )
        .expect("core evaluation")
        .delay_m
    }

    fn assert_missing_nodes(got: &SidereonIonexMissingNodes, want: Option<IonexMissingNodes>) {
        let want = want.expect("sidereon-core names the missing nodes");
        assert!(got.has_missing);
        assert_eq!(
            (
                got.map_number,
                got.lat_index,
                got.lon_index,
                got.lon_index_next,
                got.missing
            ),
            (
                want.map_number,
                want.lat_index,
                want.lon_index,
                want.lon_index_next,
                want.missing
            )
        );
    }

    #[test]
    fn parses_fixed_column_fixture_and_reads_its_whole_header() {
        // sidereon-core's own reading of the fixture; the texts compared as
        // literals are stated verbatim in VALID_IONEX_FIXTURE.
        let core = Ionex::parse(VALID_IONEX_FIXTURE.as_bytes()).expect("core parses the fixture");
        let core_header = core.header();
        unsafe {
            let mut ionex: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse(
                    VALID_IONEX_FIXTURE.as_ptr(),
                    VALID_IONEX_FIXTURE.len(),
                    &mut ionex,
                ),
                SidereonStatus::Ok
            );

            let mut epoch_count = 0usize;
            assert_eq!(
                sidereon_ionex_epoch_count(ionex, &mut epoch_count),
                SidereonStatus::Ok
            );
            assert_eq!(epoch_count, core.map_epochs_s().len());

            let mut exponent = 0i32;
            assert_eq!(
                sidereon_ionex_exponent(ionex, &mut exponent),
                SidereonStatus::Ok
            );
            assert_eq!(exponent, core.exponent());

            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(ionex, &mut header),
                SidereonStatus::Ok
            );

            let mut version = 0.0f64;
            assert_eq!(
                sidereon_ionex_header_get_version(header, &mut version),
                SidereonStatus::Ok
            );
            assert_eq!(version.to_bits(), core_header.version.to_bits());

            let mut interval_s = 0u32;
            assert_eq!(
                sidereon_ionex_header_get_interval_s(header, &mut interval_s),
                SidereonStatus::Ok
            );
            assert_eq!(interval_s, core_header.interval_s);

            // The optional counts the file gives are present, and a count it
            // leaves out reads as absent rather than as zero.
            let mut cutoff = 0.0f64;
            assert_eq!(
                sidereon_ionex_header_get_elevation_cutoff_deg(header, &mut cutoff),
                SidereonStatus::Ok
            );
            assert_eq!(cutoff.to_bits(), core_header.elevation_cutoff_deg.to_bits());

            let mut has_stations = false;
            let mut stations = 0u32;
            assert_eq!(
                sidereon_ionex_header_get_station_count(header, &mut stations, &mut has_stations),
                SidereonStatus::Ok
            );
            assert_eq!(has_stations, core_header.station_count.is_some());
            assert_eq!(stations, core_header.station_count.unwrap_or(0));

            let mut has_satellites = false;
            let mut satellites = 0u32;
            assert_eq!(
                sidereon_ionex_header_get_satellite_count(
                    header,
                    &mut satellites,
                    &mut has_satellites
                ),
                SidereonStatus::Ok
            );
            assert_eq!(has_satellites, core_header.satellite_count.is_some());
            assert_eq!(satellites, core_header.satellite_count.unwrap_or(0));

            let mut has_maps = false;
            let mut maps_in_file = 0u32;
            assert_eq!(
                sidereon_ionex_header_get_maps_in_file(header, &mut maps_in_file, &mut has_maps),
                SidereonStatus::Ok
            );
            assert_eq!(has_maps, core_header.maps_in_file.is_some());
            assert_eq!(maps_in_file, core_header.maps_in_file.unwrap_or(0));

            let mut declaration = SidereonIonexMappingDeclarationKind::Absent;
            let mut function = SidereonIonexMappingFunctionKind::Other;
            assert_eq!(
                sidereon_ionex_header_get_mapping_declaration(
                    header,
                    &mut declaration,
                    &mut function
                ),
                SidereonStatus::Ok
            );
            assert_eq!(declaration, SidereonIonexMappingDeclarationKind::Declared);
            assert_eq!(
                function,
                ionex_mapping_function_to_c(
                    core_header
                        .mapping_function
                        .as_ref()
                        .expect("MAPPING FUNCTION record")
                )
            );

            let code = copy_text(|out, len, written, required| {
                sidereon_ionex_header_get_mapping_function_code(header, out, len, written, required)
            });
            // VALID_IONEX_FIXTURE's MAPPING FUNCTION record states COSZ.
            assert_eq!(code, "COSZ");

            let mut description_count = 0usize;
            assert_eq!(
                sidereon_ionex_header_description_count(header, &mut description_count),
                SidereonStatus::Ok
            );
            assert_eq!(description_count, core_header.descriptions.len());
            let description = copy_text(|out, len, written, required| {
                sidereon_ionex_header_get_description(header, 0, out, len, written, required)
            });
            // VALID_IONEX_FIXTURE's two DESCRIPTION records and its COMMENT
            // record state these texts.
            assert_eq!(description, "SEVEN BY SEVEN TEST GRID");
            let second = copy_text(|out, len, written, required| {
                sidereon_ionex_header_get_description(header, 1, out, len, written, required)
            });
            assert_eq!(second, "SECOND DESCRIPTION LINE");

            let mut comment_count = 0usize;
            assert_eq!(
                sidereon_ionex_header_comment_count(header, &mut comment_count),
                SidereonStatus::Ok
            );
            assert_eq!(comment_count, core_header.comments.len());
            let comment = copy_text(|out, len, written, required| {
                sidereon_ionex_header_get_comment(header, 0, out, len, written, required)
            });
            assert_eq!(comment, "A HEADER COMMENT");

            // An index past the end is refused rather than read.
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_header_get_comment(
                    header,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::InvalidArgument
            );

            sidereon_ionex_header_free(header);
            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn presence_mask_separates_an_explicit_zero_from_a_missing_node() {
        unsafe {
            let mut fixture = GridFixture::new();
            // An explicit zero at the first node, a missing node at the third
            // whose numeric slot holds a value the mask says to ignore.
            fixture.tec_values = [0.0, 12.5, 999.0, 14.0, 20.0, 21.0, 22.0, 23.0];
            fixture.tec_presence = [true, true, false, true, true, true, true, true];
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);

            let mut values = [-1.0f64; 8];
            let mut presence = [true; 8];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_tec_grid_samples_tec_maps_tecu(
                    ionex,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 8);
            assert_eq!(
                sidereon_ionex_tec_grid_samples_tec_presence(
                    ionex,
                    presence.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );

            assert_eq!(values[0], 0.0);
            assert!(presence[0]);
            assert!(values[2].is_nan());
            assert!(!presence[2]);
            assert_eq!(values[3], 14.0);
            assert!(presence[3]);

            // The per-node sample view carries the same authority.
            let mut samples_out = vec![
                SidereonTecSample {
                    time_scale: 0,
                    epoch_j2000_s: 0.0,
                    has_epoch_j2000_whole_s: false,
                    epoch_j2000_whole_s: 0,
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    has_vtec_tecu: true,
                    vtec_tecu: 0.0,
                    has_rms_tecu: true,
                    rms_tecu: 0.0,
                    has_height_offset_km: true,
                    height_offset_km: 0.0,
                };
                8
            ];
            assert_eq!(
                sidereon_ionex_tec_samples(
                    ionex,
                    samples_out.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 8);
            assert!(samples_out[0].has_vtec_tecu);
            assert_eq!(samples_out[0].vtec_tecu, 0.0);
            assert!(!samples_out[2].has_vtec_tecu);
            // A product without RMS or height maps reports every node as
            // carrying neither, rather than carrying a zero.
            assert!(!samples_out[0].has_rms_tecu);
            assert!(!samples_out[0].has_height_offset_km);
            assert_eq!(
                samples_out[0].epoch_j2000_s,
                FIXTURE_FIRST_EPOCH_J2000_S as f64
            );
            assert!(samples_out[0].has_epoch_j2000_whole_s);
            assert_eq!(
                samples_out[0].epoch_j2000_whole_s,
                FIXTURE_FIRST_EPOCH_J2000_S
            );
            // The second map's nodes carry the second map's second.
            assert_eq!(
                samples_out[4].epoch_j2000_whole_s,
                FIXTURE_FIRST_EPOCH_J2000_S + 3600
            );
            assert_eq!(samples_out[0].time_scale, SidereonTimeScale::Utc as u32);

            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn an_absent_optional_map_is_not_an_all_missing_one() {
        unsafe {
            let mut fixture = GridFixture::new();
            let absent = fixture.samples(false, false, ptr::null());
            let without = build_product(&absent);
            let mut info = zeroed_samples_info();
            assert_eq!(
                sidereon_ionex_tec_grid_samples_info(without, &mut info),
                SidereonStatus::Ok
            );
            assert!(!info.has_rms_maps);
            assert_eq!(info.rms_map_value_count, 0);
            assert!(!info.has_height_maps);
            assert_eq!(info.height_map_value_count, 0);
            assert_eq!(info.tec_map_value_count, 8);
            sidereon_ionex_free(without);

            // A present height map whose every node is missing keeps its maps:
            // the count stays eight while no node holds a value, so the
            // distinction the samples surface promises survives a round trip.
            // The RMS side of the same promise is asserted separately by
            // an_all_missing_rms_map_is_not_an_absent_one.
            fixture.height_presence = [false; 8];
            let all_missing = fixture.samples(false, true, ptr::null());
            let with = build_product(&all_missing);
            let mut info = zeroed_samples_info();
            assert_eq!(
                sidereon_ionex_tec_grid_samples_info(with, &mut info),
                SidereonStatus::Ok
            );
            assert!(info.has_height_maps);
            assert_eq!(info.height_map_value_count, 8);

            let mut values = [0.0f64; 8];
            let mut presence = [true; 8];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_tec_grid_samples_height_maps_km(
                    with,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_ionex_tec_grid_samples_height_presence(
                    with,
                    presence.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 8);
            assert!(presence.iter().all(|present| !present));
            assert!(values.iter().all(|value| value.is_nan()));
            sidereon_ionex_free(with);
        }
    }

    /// The RMS half of the absent-versus-present-all-missing distinction, kept
    /// apart from the height half so a regression in one does not hide behind
    /// the other.
    ///
    /// An absent RMS map reports `has_rms_maps` false with a required length of
    /// zero. A map that is present with every node missing reports
    /// `has_rms_maps` true, the full flattened length, every presence flag
    /// false and every value NaN. The flag, not the length, is what separates
    /// them. The RMS authority stays in the engine; this binding reports what
    /// the engine holds and does not duplicate it.
    #[test]
    fn an_all_missing_rms_map_is_not_an_absent_one() {
        unsafe {
            let mut fixture = GridFixture::new();

            // No RMS map at all: no values, and the extraction routes report a
            // required length of zero rather than a buffer of zeros.
            let absent = fixture.samples(false, false, ptr::null());
            let without = build_product(&absent);
            let mut info = zeroed_samples_info();
            assert_eq!(
                sidereon_ionex_tec_grid_samples_info(without, &mut info),
                SidereonStatus::Ok
            );
            assert!(!info.has_rms_maps);
            assert_eq!(info.rms_map_value_count, 0);
            let mut written = 0usize;
            let mut required = 1usize;
            assert_eq!(
                sidereon_ionex_tec_grid_samples_rms_maps_tecu(
                    without,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 0);
            required = 1;
            assert_eq!(
                sidereon_ionex_tec_grid_samples_rms_presence(
                    without,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 0);
            sidereon_ionex_free(without);

            // A present RMS map whose every node is missing: the map is still
            // there, the count stays eight, every flag is false and every
            // value is NaN.
            fixture.rms_presence = [false; 8];
            let all_missing = fixture.samples(true, false, ptr::null());
            let with = build_product(&all_missing);
            let mut info = zeroed_samples_info();
            assert_eq!(
                sidereon_ionex_tec_grid_samples_info(with, &mut info),
                SidereonStatus::Ok
            );
            assert!(info.has_rms_maps);
            assert_eq!(info.rms_map_value_count, 8);

            let mut values = [0.0f64; 8];
            let mut presence = [true; 8];
            assert_eq!(
                sidereon_ionex_tec_grid_samples_rms_maps_tecu(
                    with,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 8);
            assert_eq!(
                sidereon_ionex_tec_grid_samples_rms_presence(
                    with,
                    presence.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 8);
            assert!(presence.iter().all(|present| !present));
            assert!(values.iter().all(|value| value.is_nan()));
            sidereon_ionex_free(with);
        }
    }

    fn zeroed_samples_info() -> SidereonTecGridSamplesInfo {
        SidereonTecGridSamplesInfo {
            map_epoch_count: 0,
            lat_node_count: 0,
            lon_node_count: 0,
            dlat_deg: 0.0,
            dlon_deg: 0.0,
            shell_height_km: 0.0,
            base_radius_km: 0.0,
            exponent: 0,
            has_rms_maps: false,
            tec_map_value_count: 0,
            rms_map_value_count: 0,
            has_height_maps: false,
            height_map_value_count: 0,
        }
    }

    #[test]
    fn rms_values_and_header_records_survive_a_sample_round_trip() {
        unsafe {
            let mut fixture = GridFixture::new();
            fixture.rms_values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            let agency = b"AGENCY";
            assert_eq!(
                sidereon_ionex_header_set_run_by(header, agency.as_ptr(), agency.len()),
                SidereonStatus::Ok
            );
            let note = "RMS ROUND TRIP".as_bytes();
            assert_eq!(
                sidereon_ionex_header_add_description(header, note.as_ptr(), note.len()),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_ionex_header_set_station_count(header, 7, true),
                SidereonStatus::Ok
            );

            let samples = fixture.samples(true, false, header);
            let ionex = build_product(&samples);

            let mut values = [0.0f64; 8];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_tec_grid_samples_rms_maps_tecu(
                    ionex,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(values, fixture.rms_values);

            let mut stored: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(ionex, &mut stored),
                SidereonStatus::Ok
            );
            let run_by = copy_text(|out, len, w, r| {
                sidereon_ionex_header_get_run_by(stored, out, len, w, r)
            });
            assert_eq!(run_by, "AGENCY");
            let description = copy_text(|out, len, w, r| {
                sidereon_ionex_header_get_description(stored, 0, out, len, w, r)
            });
            assert_eq!(description, "RMS ROUND TRIP");
            let mut has_stations = false;
            let mut stations = 0u32;
            assert_eq!(
                sidereon_ionex_header_get_station_count(stored, &mut stations, &mut has_stations),
                SidereonStatus::Ok
            );
            assert!(has_stations);
            assert_eq!(stations, 7);

            sidereon_ionex_header_free(stored);
            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);
        }
    }

    #[test]
    fn a_held_degraded_and_assumed_mapping_value_reports_all_three() {
        unsafe {
            let mut fixture = GridFixture::new();
            fixture.tec_values = [10.0, 12.0, 14.0, 16.0, 20.0, 22.0, 24.0, 26.0];
            // One weighted corner of the first map holds no value.
            fixture.tec_presence[0] = false;

            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::NoMapping as u32,
                    ptr::null(),
                    0
                ),
                SidereonStatus::Ok
            );

            let samples = fixture.samples(false, false, header);
            let ionex = build_product(&samples);

            let hold_and_renormalize = policy(
                SidereonIonexCoveragePolicy::Hold as u32,
                SidereonIonexMissingNodePolicy::Renormalize as u32,
                SidereonIonexMappingPolicy::SingleLayer as u32,
            );
            let mut evaluation = refused_ionex_slant_delay_evaluation();
            let mut error = no_ionex_slant_error();
            assert_eq!(
                sidereon_ionex_slant_delay_with_policy(
                    ionex,
                    30.0,
                    -10.0,
                    0.0,
                    85.0,
                    FIXTURE_FIRST_EPOCH_J2000_S - 1000,
                    1_575_420_000.0,
                    hold_and_renormalize,
                    &mut evaluation,
                    &mut error,
                ),
                SidereonStatus::Ok
            );

            let core_eval = core_slant(
                ionex,
                30.0,
                -10.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S - 1000,
                1_575_420_000.0,
                hold_and_renormalize,
            )
            .expect("core evaluation");
            let core_gap = core_eval.status.degraded.expect("core degraded gap");
            assert_eq!(evaluation.delay_m.to_bits(), core_eval.delay_m.to_bits());
            assert!(!evaluation.status.is_valid);
            assert!(evaluation.status.has_held);
            assert_eq!(
                evaluation.status.coverage_error,
                SidereonIonexCoverageErrorKind::EpochBeforeFirstMap
            );
            assert!(evaluation.status.has_degraded);
            assert!(evaluation.status.gap.has_gap);
            assert_missing_nodes(&evaluation.status.gap.earlier, core_gap.earlier);
            assert!(evaluation.status.has_assumed_mapping);
            assert_eq!(
                evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::NoMapping
            );
            // A successful evaluation records no failure.
            assert_eq!(error.kind, SidereonIonexSlantErrorKind::None);

            // The same query under the default policy is refused, and the
            // refusal carries the indexed corner masks rather than only a
            // status code.
            let mut refused = evaluation;
            let mut refusal = no_ionex_slant_error();
            let status = sidereon_ionex_slant_delay_with_policy(
                ionex,
                30.0,
                -10.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S - 1000,
                1_575_420_000.0,
                sidereon_ionex_slant_policy_default(),
                &mut refused,
                &mut refusal,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert!(refused.delay_m.is_nan());
            assert!(!refused.status.is_valid);
            assert_eq!(refusal.kind, SidereonIonexSlantErrorKind::OutOfCoverage);
            assert_eq!(
                refusal.coverage_error,
                SidereonIonexCoverageErrorKind::EpochBeforeFirstMap
            );

            // Inside coverage, the default policy names the weighted corner the
            // product gives as non-available.
            let mut node_refusal = no_ionex_slant_error();
            let status = sidereon_ionex_slant_delay_with_policy(
                ionex,
                30.0,
                -10.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S,
                1_575_420_000.0,
                sidereon_ionex_slant_policy_default(),
                &mut refused,
                &mut node_refusal,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert_eq!(
                node_refusal.kind,
                SidereonIonexSlantErrorKind::NodesNotAvailable
            );
            assert!(node_refusal.has_gap);
            assert!(node_refusal.gap.has_gap);
            let Err(CoreError::IonexNodesNotAvailable(core_gap)) = core_slant(
                ionex,
                30.0,
                -10.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S,
                1_575_420_000.0,
                sidereon_ionex_slant_policy_default(),
            ) else {
                panic!("sidereon-core evaluates across the missing corner");
            };
            assert_missing_nodes(&node_refusal.gap.earlier, core_gap.earlier);

            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);
        }
    }

    #[test]
    fn a_height_map_without_a_value_refuses_with_its_indices() {
        unsafe {
            let mut fixture = GridFixture::new();
            fixture.height_presence = [true; 8];
            // The second node of the first height map has no height, so the
            // product gives no single-layer height there.
            fixture.height_presence[1] = false;
            let samples = fixture.samples(false, true, ptr::null());
            let ionex = build_product(&samples);

            let mut evaluation = refused_ionex_slant_delay_evaluation();
            let mut error = no_ionex_slant_error();
            let status = sidereon_ionex_slant_delay_with_policy(
                ionex,
                0.0,
                0.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S,
                1_575_420_000.0,
                sidereon_ionex_slant_policy_default(),
                &mut evaluation,
                &mut error,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert!(evaluation.delay_m.is_nan());
            assert_eq!(error.kind, SidereonIonexSlantErrorKind::SlantUnavailable);
            assert_eq!(
                error.refusal,
                SidereonIonexSlantRefusalKind::HeightNotAvailable
            );
            let Err(CoreError::IonexSlantUnavailable(IonexSlantRefusal::HeightNotAvailable {
                map_number,
                lat_index,
                lon_index,
            })) = core_slant(
                ionex,
                0.0,
                0.0,
                0.0,
                85.0,
                FIXTURE_FIRST_EPOCH_J2000_S,
                1_575_420_000.0,
                sidereon_ionex_slant_policy_default(),
            )
            else {
                panic!("sidereon-core finds a height at the missing node");
            };
            assert_eq!(
                (
                    error.refusal_map_number,
                    error.refusal_lat_index,
                    error.refusal_lon_index
                ),
                (map_number, lat_index, lon_index)
            );

            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn an_owned_result_list_keeps_every_row_and_outlives_its_product() {
        unsafe {
            let fixture = GridFixture::new();
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            let custom = b"SLAB3D";
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::Other as u32,
                    custom.as_ptr(),
                    custom.len()
                ),
                SidereonStatus::Ok
            );
            let samples = fixture.samples(false, false, header);
            let ionex = build_product(&samples);

            let requests = [
                // Inside coverage.
                SidereonIonexSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 85.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                    frequency_hz: 1_575_420_000.0,
                },
                // A receiver latitude no geodetic position can hold.
                SidereonIonexSlantRequest {
                    lat_deg: 95.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 85.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                    frequency_hz: 1_575_420_000.0,
                },
                // A carrier frequency the engine rejects by name.
                SidereonIonexSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 85.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                    frequency_hz: 0.0,
                },
                // Before the first map, which the strict coverage policy
                // refuses.
                SidereonIonexSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 85.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S - 10_000,
                    frequency_hz: 1_575_420_000.0,
                },
                // Inside coverage again, so a failure cannot truncate the list.
                SidereonIonexSlantRequest {
                    lat_deg: -10.0,
                    lon_deg: 10.0,
                    azimuth_deg: 45.0,
                    elevation_deg: 60.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S + 1800,
                    frequency_hz: 1_575_420_000.0,
                },
            ];

            let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    requests.as_ptr(),
                    requests.len(),
                    // The Declared mapping policy reaches the product's own
                    // `MAPPING FUNCTION`, which defines no factor here.
                    policy(
                        SidereonIonexCoveragePolicy::Strict as u32,
                        SidereonIonexMissingNodePolicy::Strict as u32,
                        SidereonIonexMappingPolicy::Declared as u32,
                    ),
                    &mut list,
                ),
                SidereonStatus::Ok
            );
            assert!(!list.is_null());

            let mut count = 0usize;
            assert_eq!(
                sidereon_ionex_slant_result_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, requests.len());

            let row_at = |index: usize| {
                let mut row = SidereonIonexSlantRowResult {
                    is_ok: true,
                    status: SidereonStatus::Panic,
                    evaluation: refused_ionex_slant_delay_evaluation(),
                    error: no_ionex_slant_error(),
                };
                assert_eq!(
                    sidereon_ionex_slant_result_get_row(list, index, &mut row),
                    SidereonStatus::Ok
                );
                row
            };

            // Under the Declared mapping policy every geometrically valid row
            // is refused for the same reason, and each keeps its own detail.
            for index in [0usize, 4] {
                let row = row_at(index);
                assert!(!row.is_ok);
                assert_eq!(
                    row.error.kind,
                    SidereonIonexSlantErrorKind::SlantUnavailable
                );
                assert_eq!(
                    row.error.refusal,
                    SidereonIonexSlantRefusalKind::MappingFunction
                );
                assert!(row.error.has_mapping_declaration);
                assert_eq!(
                    row.error.mapping_declaration,
                    SidereonIonexMappingDeclarationKind::Declared
                );
                assert!(row.error.has_mapping_function);
                assert_eq!(
                    row.error.mapping_function,
                    SidereonIonexMappingFunctionKind::Other
                );
                assert!(row.evaluation.delay_m.is_nan());
                assert!(!row.evaluation.status.is_valid);
            }

            // The receiver latitude is refused before the engine batch, and the
            // row keeps its position and the engine's own text.
            let rejected = row_at(1);
            assert!(!rejected.is_ok);
            assert_eq!(rejected.status, SidereonStatus::InvalidArgument);
            assert_eq!(
                rejected.error.kind,
                SidereonIonexSlantErrorKind::InvalidInput
            );
            let message = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_message(list, 1, out, len, w, r)
            });
            assert!(message.contains("lat_rad"), "unexpected text {message:?}");

            // The carrier frequency is refused inside the engine batch.
            let bad_frequency = row_at(2);
            assert!(!bad_frequency.is_ok);
            assert_eq!(bad_frequency.status, SidereonStatus::InvalidArgument);
            assert_eq!(
                bad_frequency.error.kind,
                SidereonIonexSlantErrorKind::InvalidInput
            );
            let message = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_message(list, 2, out, len, w, r)
            });
            assert!(
                message.contains("frequency_hz"),
                "unexpected text {message:?}"
            );

            // The engine resolves the shell before it reads the query, so a
            // row outside coverage under this policy reports the same mapping
            // refusal. It still keeps its own row and its own text.
            let outside_coverage = row_at(3);
            assert!(!outside_coverage.is_ok);
            assert_eq!(
                outside_coverage.error.kind,
                SidereonIonexSlantErrorKind::SlantUnavailable
            );
            let message = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_message(list, 3, out, len, w, r)
            });
            assert!(
                message.contains("MAPPING FUNCTION"),
                "unexpected text {message:?}"
            );

            // The custom mapping code is owned by the list, so it survives the
            // product and header handles it was read from.
            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);
            let code = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_mapping_code(list, 0, out, len, w, r)
            });
            assert_eq!(code, "SLAB3D");

            // An index past the end is refused rather than read.
            let mut row = SidereonIonexSlantRowResult {
                is_ok: true,
                status: SidereonStatus::Ok,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
            };
            assert_eq!(
                sidereon_ionex_slant_result_get_row(list, count, &mut row),
                SidereonStatus::InvalidArgument
            );
            assert!(!row.is_ok);

            sidereon_ionex_slant_result_list_free(list);
            sidereon_ionex_slant_result_list_free(ptr::null_mut());
        }
    }

    /// A successful row under the single-layer factor keeps the custom code
    /// the product declared, in the list's own authority.
    ///
    /// `IonexAssumedMapping::Other` names the case without carrying the text,
    /// which the engine keeps only in `IonexHeader::mapping_function`, so the
    /// code is unreachable the moment the product and header are freed. The
    /// owned-list promise is that a row stays complete after that.
    #[test]
    fn a_successful_assumed_other_row_keeps_its_custom_mapping_code() {
        unsafe {
            let fixture = GridFixture::new();
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            let custom = b"SLAB_3D_CUSTOM";
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::Other as u32,
                    custom.as_ptr(),
                    custom.len()
                ),
                SidereonStatus::Ok
            );
            let samples = fixture.samples(false, false, header);
            let ionex = build_product(&samples);

            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };
            let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    &request,
                    1,
                    // SingleLayer applies 1/cos(z') and reports the declaration
                    // it did not use, so the row succeeds and still names a
                    // custom code.
                    policy(
                        SidereonIonexCoveragePolicy::Strict as u32,
                        SidereonIonexMissingNodePolicy::Strict as u32,
                        SidereonIonexMappingPolicy::SingleLayer as u32,
                    ),
                    &mut list,
                ),
                SidereonStatus::Ok
            );
            assert!(!list.is_null());

            let mut row = SidereonIonexSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
            };
            assert_eq!(
                sidereon_ionex_slant_result_get_row(list, 0, &mut row),
                SidereonStatus::Ok
            );
            assert!(row.is_ok);
            assert_eq!(row.status, SidereonStatus::Ok);
            assert_eq!(row.error.kind, SidereonIonexSlantErrorKind::None);
            assert_eq!(
                row.evaluation.delay_m.to_bits(),
                core_request_delay(
                    ionex,
                    &request,
                    policy(
                        SidereonIonexCoveragePolicy::Strict as u32,
                        SidereonIonexMissingNodePolicy::Strict as u32,
                        SidereonIonexMappingPolicy::SingleLayer as u32,
                    )
                )
                .to_bits()
            );
            assert!(row.evaluation.status.is_valid);
            assert!(row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Other
            );

            // Both handles the code could otherwise be read from are released.
            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);

            // A later failing call overwrites the thread-local message. The
            // list's copy is its own storage, not that message.
            let mut ignored = 0usize;
            assert_eq!(
                sidereon_ionex_slant_result_list_count(ptr::null(), &mut ignored),
                SidereonStatus::NullPointer
            );
            let mut last_error = [0u8; 128];
            let needed =
                sidereon_last_error_message(last_error.as_mut_ptr().cast(), last_error.len());
            assert!(needed > 0);
            assert!(last_error.starts_with(b"sidereon_ionex_slant_result_list_count: null list"));

            let code = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_mapping_code(list, 0, out, len, w, r)
            });
            assert_eq!(code.as_bytes(), custom);
            // A successful row still carries no engine text: the mapping code
            // is mapping context, not a failure message.
            let message = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_message(list, 0, out, len, w, r)
            });
            assert!(message.is_empty());

            sidereon_ionex_slant_result_list_free(list);
        }
    }

    /// A successful row whose product declares nothing reports no code, so a
    /// zero length under `Absent` is not confused with a custom code.
    #[test]
    fn a_successful_row_without_a_declaration_reports_no_mapping_code() {
        unsafe {
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);
            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };
            let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    &request,
                    1,
                    sidereon_ionex_slant_policy_default(),
                    &mut list,
                ),
                SidereonStatus::Ok
            );
            let mut row = SidereonIonexSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
            };
            assert_eq!(
                sidereon_ionex_slant_result_get_row(list, 0, &mut row),
                SidereonStatus::Ok
            );
            assert!(row.is_ok);
            assert!(row.evaluation.status.has_assumed_mapping);
            // The typed kind, not the length, is what separates this from a
            // declared custom code.
            assert_eq!(
                row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Absent
            );
            let code = copy_text(|out, len, w, r| {
                sidereon_ionex_slant_result_get_mapping_code(list, 0, out, len, w, r)
            });
            assert!(code.is_empty());

            sidereon_ionex_slant_result_list_free(list);
            sidereon_ionex_free(ionex);
        }
    }

    /// A caller who supplies no header gets one declaring no mapping function.
    ///
    /// `sidereon_ionex_header_new`, a NULL `header` on
    /// `sidereon_ionex_from_tec_samples_with_header`, and a NULL
    /// `SidereonTecGridSamples::header` all build the same undeclared header,
    /// so the declaration reads Absent rather than COSZ. That single fact has
    /// four consequences this asserts directly: the handle reports Absent, a
    /// successful row under the default single-layer policy reports assumed
    /// mapping Absent, the Declared policy refuses the same query naming an
    /// Absent declaration, and the IONEX text carries a blank
    /// `MAPPING FUNCTION` field that reads back Absent. Declaring COSZ on the
    /// same fixture reverses every one of them, which is what separates an
    /// undeclared header from one that declares the factor the engine applies.
    #[test]
    fn an_unsupplied_header_declares_no_mapping_function() {
        unsafe {
            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };
            let declared_policy = policy(
                SidereonIonexCoveragePolicy::Strict as u32,
                SidereonIonexMissingNodePolicy::Strict as u32,
                SidereonIonexMappingPolicy::Declared as u32,
            );
            let read_declaration = |header: *mut SidereonIonexHeader| {
                let mut declaration = SidereonIonexMappingDeclarationKind::Declared;
                let mut function = SidereonIonexMappingFunctionKind::CosZ;
                assert_eq!(
                    sidereon_ionex_header_get_mapping_declaration(
                        header,
                        &mut declaration,
                        &mut function
                    ),
                    SidereonStatus::Ok
                );
                (declaration, function)
            };
            let first_row = |ionex: *mut SidereonIonex, chosen: SidereonIonexSlantPolicy| {
                let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
                assert_eq!(
                    sidereon_ionex_slant_delay_results(ionex, &request, 1, chosen, &mut list),
                    SidereonStatus::Ok
                );
                assert!(!list.is_null());
                let mut row = SidereonIonexSlantRowResult {
                    is_ok: true,
                    status: SidereonStatus::Panic,
                    evaluation: refused_ionex_slant_delay_evaluation(),
                    error: no_ionex_slant_error(),
                };
                assert_eq!(
                    sidereon_ionex_slant_result_get_row(list, 0, &mut row),
                    SidereonStatus::Ok
                );
                sidereon_ionex_slant_result_list_free(list);
                row
            };
            // The data of the `MAPPING FUNCTION` record, which the writer emits
            // whether or not a code is declared: the label alone proves nothing.
            let mapping_field = |text: &str| {
                text.lines()
                    .find(|line| line.len() > 60 && line[60..].starts_with("MAPPING FUNCTION"))
                    .map(|line| line[..60].trim().to_string())
                    .expect("the writer emits a MAPPING FUNCTION record")
            };

            let fixture = GridFixture::new();

            // 1. The handle `sidereon_ionex_header_new` builds declares nothing.
            let mut fresh: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut fresh), SidereonStatus::Ok);
            assert_eq!(
                read_declaration(fresh).0,
                SidereonIonexMappingDeclarationKind::Absent
            );
            let fresh_code = copy_text(|out, len, w, r| {
                sidereon_ionex_header_get_mapping_function_code(fresh, out, len, w, r)
            });
            assert!(fresh_code.is_empty());
            sidereon_ionex_header_free(fresh);

            // The same holds for a product built with no header at all, read
            // back off the product rather than off the caller's handle.
            let bare_samples = fixture.samples(false, false, ptr::null());
            let bare = build_product(&bare_samples);
            let mut stored: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(bare, &mut stored),
                SidereonStatus::Ok
            );
            assert_eq!(
                read_declaration(stored).0,
                SidereonIonexMappingDeclarationKind::Absent
            );
            sidereon_ionex_header_free(stored);

            // The per-node convenience route, the third site that builds a
            // header for a caller who supplied none. Its nodes are the bare
            // product's own, so only the header route differs.
            let mut nodes = vec![
                SidereonTecSample {
                    time_scale: 0,
                    epoch_j2000_s: 0.0,
                    has_epoch_j2000_whole_s: false,
                    epoch_j2000_whole_s: 0,
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    has_vtec_tecu: false,
                    vtec_tecu: 0.0,
                    has_rms_tecu: false,
                    rms_tecu: 0.0,
                    has_height_offset_km: false,
                    height_offset_km: 0.0,
                };
                8
            ];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_tec_samples(
                    bare,
                    nodes.as_mut_ptr(),
                    nodes.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, nodes.len());
            let mut convenience: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_from_tec_samples(
                    nodes.as_ptr(),
                    nodes.len(),
                    450.0,
                    6371.0,
                    -1,
                    &mut convenience,
                ),
                SidereonStatus::Ok
            );
            let mut convenience_header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(convenience, &mut convenience_header),
                SidereonStatus::Ok
            );
            assert_eq!(
                read_declaration(convenience_header).0,
                SidereonIonexMappingDeclarationKind::Absent
            );
            sidereon_ionex_header_free(convenience_header);
            sidereon_ionex_free(convenience);

            // 2. A successful row under the default policy assumes Absent, not
            // the COSZ factor a declared header would report as no assumption.
            let bare_row = first_row(bare, sidereon_ionex_slant_policy_default());
            assert!(bare_row.is_ok);
            assert!(bare_row.evaluation.status.is_valid);
            assert_eq!(
                bare_row.evaluation.delay_m.to_bits(),
                core_request_delay(bare, &request, sidereon_ionex_slant_policy_default()).to_bits()
            );
            assert!(bare_row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                bare_row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Absent
            );

            // 3. The Declared policy refuses that same query: there is no
            // declaration to reach.
            let refused = first_row(bare, declared_policy);
            assert!(!refused.is_ok);
            assert_eq!(
                refused.error.kind,
                SidereonIonexSlantErrorKind::SlantUnavailable
            );
            assert_eq!(
                refused.error.refusal,
                SidereonIonexSlantRefusalKind::MappingFunction
            );
            assert!(refused.error.has_mapping_declaration);
            assert_eq!(
                refused.error.mapping_declaration,
                SidereonIonexMappingDeclarationKind::Absent
            );
            // Absent names no code, so the function field stays unset.
            assert!(!refused.error.has_mapping_function);
            assert!(refused.evaluation.delay_m.is_nan());
            assert!(!refused.evaluation.status.is_valid);

            // 4. The text keeps the absence: a blank field that reads back
            // Absent, not an invented COSZ record.
            let bare_text =
                copy_text(|out, len, w, r| sidereon_ionex_to_ionex_text(bare, out, len, w, r));
            assert_eq!(mapping_field(&bare_text), "");
            let mut reparsed: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse(bare_text.as_ptr(), bare_text.len(), &mut reparsed),
                SidereonStatus::Ok
            );
            let mut reparsed_header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(reparsed, &mut reparsed_header),
                SidereonStatus::Ok
            );
            assert_eq!(
                read_declaration(reparsed_header).0,
                SidereonIonexMappingDeclarationKind::Absent
            );
            sidereon_ionex_header_free(reparsed_header);
            sidereon_ionex_free(reparsed);
            sidereon_ionex_free(bare);

            // An explicitly declared COSZ reverses all four, so the Absent
            // assertions above are not simply what every header reports.
            let mut declared_header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_header_new(&mut declared_header),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    declared_header,
                    SidereonIonexMappingFunctionKind::CosZ as u32,
                    ptr::null(),
                    0
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                read_declaration(declared_header),
                (
                    SidereonIonexMappingDeclarationKind::Declared,
                    SidereonIonexMappingFunctionKind::CosZ
                )
            );
            let declared_samples = fixture.samples(false, false, declared_header);
            let declared = build_product(&declared_samples);

            // COSZ is the factor applied, so nothing is assumed.
            let declared_row = first_row(declared, sidereon_ionex_slant_policy_default());
            assert!(declared_row.is_ok);
            assert!(!declared_row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                declared_row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::None
            );
            // The same value either way: the declaration selects the policy
            // outcome, not the arithmetic.
            assert_eq!(declared_row.evaluation.delay_m, bare_row.evaluation.delay_m);

            // The Declared policy is satisfied where it refused the bare one.
            let declared_strict = first_row(declared, declared_policy);
            assert!(declared_strict.is_ok);
            assert_eq!(
                declared_strict.error.kind,
                SidereonIonexSlantErrorKind::None
            );

            let declared_text =
                copy_text(|out, len, w, r| sidereon_ionex_to_ionex_text(declared, out, len, w, r));
            assert_eq!(mapping_field(&declared_text), "COSZ");
            let mut declared_reparsed: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse(
                    declared_text.as_ptr(),
                    declared_text.len(),
                    &mut declared_reparsed
                ),
                SidereonStatus::Ok
            );
            let mut declared_reparsed_header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(declared_reparsed, &mut declared_reparsed_header),
                SidereonStatus::Ok
            );
            assert_eq!(
                read_declaration(declared_reparsed_header),
                (
                    SidereonIonexMappingDeclarationKind::Declared,
                    SidereonIonexMappingFunctionKind::CosZ
                )
            );
            let round_tripped = copy_text(|out, len, w, r| {
                sidereon_ionex_header_get_mapping_function_code(
                    declared_reparsed_header,
                    out,
                    len,
                    w,
                    r,
                )
            });
            assert_eq!(round_tripped, "COSZ");

            sidereon_ionex_header_free(declared_reparsed_header);
            sidereon_ionex_free(declared_reparsed);
            sidereon_ionex_free(declared);
            sidereon_ionex_header_free(declared_header);
        }
    }

    /// A custom code may be empty, and that is not the absent case.
    ///
    /// `sidereon_ionex_header_set_mapping_function` accepts Other with a
    /// zero-length code, the product keeps it, and `IonexAssumedMapping::of`
    /// reports Other without looking at the text, so the row succeeds with
    /// assumed_mapping Other and a required code length of zero. The absent
    /// declaration reports the same zero length under assumed_mapping Absent,
    /// so only the typed kind tells them apart. Nothing here refuses the blank
    /// code: the writer's refusal is the writer's alone.
    #[test]
    fn a_successful_row_may_assume_an_empty_custom_mapping_code() {
        unsafe {
            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };
            let single_layer = policy(
                SidereonIonexCoveragePolicy::Strict as u32,
                SidereonIonexMissingNodePolicy::Strict as u32,
                SidereonIonexMappingPolicy::SingleLayer as u32,
            );

            let fixture = GridFixture::new();
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            // A zero length reads no bytes, so the pointer is never touched.
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::Other as u32,
                    ptr::null(),
                    0
                ),
                SidereonStatus::Ok
            );
            let samples = fixture.samples(false, false, header);
            let ionex = build_product(&samples);

            let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(ionex, &request, 1, single_layer, &mut list),
                SidereonStatus::Ok
            );
            assert!(!list.is_null());

            let mut row = SidereonIonexSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
            };
            assert_eq!(
                sidereon_ionex_slant_result_get_row(list, 0, &mut row),
                SidereonStatus::Ok
            );
            // The evaluation succeeds: an empty code is a declaration the
            // single-layer factor reports, not an input the engine refuses.
            assert!(row.is_ok);
            assert_eq!(row.status, SidereonStatus::Ok);
            assert_eq!(row.error.kind, SidereonIonexSlantErrorKind::None);
            assert!(row.evaluation.status.is_valid);
            assert_eq!(
                row.evaluation.delay_m.to_bits(),
                core_request_delay(ionex, &request, single_layer).to_bits()
            );
            assert!(row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Other
            );

            // Both handles the code could otherwise be read from are released,
            // and the row still answers for itself.
            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);

            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_slant_result_get_mapping_code(
                    list,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(required, 0);
            assert_eq!(written, 0);
            sidereon_ionex_slant_result_list_free(list);

            // The absent declaration reaches the same zero length under a
            // different kind, which is the whole reason the kind is read.
            let bare = GridFixture::new();
            let bare_samples = bare.samples(false, false, ptr::null());
            let bare_ionex = build_product(&bare_samples);
            let mut bare_list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    bare_ionex,
                    &request,
                    1,
                    single_layer,
                    &mut bare_list
                ),
                SidereonStatus::Ok
            );
            let mut bare_row = SidereonIonexSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
            };
            assert_eq!(
                sidereon_ionex_slant_result_get_row(bare_list, 0, &mut bare_row),
                SidereonStatus::Ok
            );
            assert!(bare_row.is_ok);
            assert!(bare_row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                bare_row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Absent
            );
            let mut bare_required = 0usize;
            assert_eq!(
                sidereon_ionex_slant_result_get_mapping_code(
                    bare_list,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut bare_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(bare_required, required);
            assert_ne!(
                row.evaluation.status.assumed_mapping,
                bare_row.evaluation.status.assumed_mapping
            );

            sidereon_ionex_slant_result_list_free(bare_list);
            sidereon_ionex_free(bare_ionex);
        }
    }

    #[test]
    fn every_row_of_a_nominal_batch_holds_a_value() {
        unsafe {
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);

            let requests = [
                SidereonIonexSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 85.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                    frequency_hz: 1_575_420_000.0,
                },
                SidereonIonexSlantRequest {
                    lat_deg: 10.0,
                    lon_deg: -5.0,
                    azimuth_deg: 30.0,
                    elevation_deg: 70.0,
                    epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S + 1800,
                    frequency_hz: 1_575_420_000.0,
                },
            ];

            let mut list: *mut SidereonIonexSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    requests.as_ptr(),
                    requests.len(),
                    sidereon_ionex_slant_policy_default(),
                    &mut list,
                ),
                SidereonStatus::Ok
            );

            let mut delays = [0.0f64; 2];
            assert_eq!(
                sidereon_ionex_slant_delays(
                    ionex,
                    requests.as_ptr(),
                    requests.len(),
                    delays.as_mut_ptr()
                ),
                SidereonStatus::Ok
            );

            for (index, delay) in delays.iter().enumerate() {
                let mut row = SidereonIonexSlantRowResult {
                    is_ok: false,
                    status: SidereonStatus::Panic,
                    evaluation: refused_ionex_slant_delay_evaluation(),
                    error: no_ionex_slant_error(),
                };
                assert_eq!(
                    sidereon_ionex_slant_result_get_row(list, index, &mut row),
                    SidereonStatus::Ok
                );
                assert!(row.is_ok);
                assert_eq!(row.status, SidereonStatus::Ok);
                assert_eq!(row.error.kind, SidereonIonexSlantErrorKind::None);
                // The list and the plain array agree bit for bit: both reach
                // the same engine kernel in input order.
                assert_eq!(row.evaluation.delay_m.to_bits(), delay.to_bits());
                assert_eq!(
                    delay.to_bits(),
                    core_request_delay(
                        ionex,
                        &requests[index],
                        sidereon_ionex_slant_policy_default()
                    )
                    .to_bits()
                );
                // The product declares no mapping function, so the single-layer
                // factor is an assumption the status names without making the
                // value invalid.
                assert!(row.evaluation.status.is_valid);
                assert!(row.evaluation.status.has_assumed_mapping);
                assert_eq!(
                    row.evaluation.status.assumed_mapping,
                    SidereonIonexAssumedMappingKind::Absent
                );
            }

            sidereon_ionex_slant_result_list_free(list);
            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn malformed_batch_arguments_fail_the_whole_call_without_a_list() {
        unsafe {
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);
            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };

            let mut list: *mut SidereonIonexSlantResultList = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ptr::null(),
                    &request,
                    1,
                    sidereon_ionex_slant_policy_default(),
                    &mut list,
                ),
                SidereonStatus::NullPointer
            );
            assert!(list.is_null());

            let mut list: *mut SidereonIonexSlantResultList = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    ptr::null(),
                    1,
                    sidereon_ionex_slant_policy_default(),
                    &mut list,
                ),
                SidereonStatus::NullPointer
            );
            assert!(list.is_null());

            let mut list: *mut SidereonIonexSlantResultList = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results(ionex, &request, 1, policy(9, 0, 1), &mut list),
                SidereonStatus::InvalidArgument
            );
            assert!(list.is_null());

            assert_eq!(
                sidereon_ionex_slant_delay_results(
                    ionex,
                    &request,
                    1,
                    sidereon_ionex_slant_policy_default(),
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );

            sidereon_ionex_free(ionex);
        }
    }

    /// The plain batch route clears its output before it reads anything else.
    ///
    /// sidereon_ionex_slant_delays promises a caller who hands it storage it
    /// can write that the storage holds `count` zeroes on every failure the
    /// call reports. A null product and a null request array are the two
    /// failures that reach no engine at all, so each starts from its own
    /// sentinel that the zeroes cannot be mistaken for.
    ///
    /// The output pointer itself is outside that promise, and the two ways it
    /// can be unusable are checked in the opposite direction: they name no
    /// storage to initialize, so nothing is written and the caller's array
    /// keeps what it held.
    #[test]
    fn a_failed_batch_zeroes_its_output_before_checking_the_other_arguments() {
        unsafe {
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);
            let request = SidereonIonexSlantRequest {
                lat_deg: 0.0,
                lon_deg: 0.0,
                azimuth_deg: 0.0,
                elevation_deg: 85.0,
                epoch_j2000_s: FIXTURE_FIRST_EPOCH_J2000_S,
                frequency_hz: 1_575_420_000.0,
            };
            let requests = [request, request];
            let positive_zero = 0.0f64.to_bits();

            // A null product, from a sentinel that is neither zero nor NaN.
            let mut delays = [-12345.5f64, 6789.25];
            assert_eq!(
                sidereon_ionex_slant_delays(ptr::null(), requests.as_ptr(), 2, delays.as_mut_ptr()),
                SidereonStatus::NullPointer
            );
            assert!(
                delays.iter().all(|d| d.to_bits() == positive_zero),
                "a null product leaves the writable output zero, got {delays:?}"
            );

            // A null request array with a nonzero count, from a different one.
            let mut delays = [f64::MIN, f64::MAX];
            assert_eq!(
                sidereon_ionex_slant_delays(ionex, ptr::null(), 2, delays.as_mut_ptr()),
                SidereonStatus::NullPointer
            );
            assert!(
                delays.iter().all(|d| d.to_bits() == positive_zero),
                "a null request array leaves the writable output zero, got {delays:?}"
            );

            // A null output with a nonzero count names no storage, so the
            // refusal claims none was initialized.
            assert_eq!(
                sidereon_ionex_slant_delays(ionex, requests.as_ptr(), 2, ptr::null_mut()),
                SidereonStatus::NullPointer
            );

            // Neither does a count no slice can span: the array is untouched.
            let mut untouched = [-1.0f64, -2.0];
            assert_eq!(
                sidereon_ionex_slant_delays(
                    ionex,
                    requests.as_ptr(),
                    usize::MAX,
                    untouched.as_mut_ptr()
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(untouched, [-1.0, -2.0]);

            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn parse_with_warnings_reports_every_payload_it_reads() {
        const FIXTURE_WITH_WARNINGS: &str = concat!(
            "A COMMENT BEFORE THE VERSION RECORD                         COMMENT\n",
            "     1.0            IONOSPHERE MAPS     GPS                 IONEX VERSION / TYPE\n",
            "  2020     1     1     0     0     0                        EPOCH OF FIRST MAP\n",
            "  2020     1     1     2     0     0                        EPOCH OF LAST MAP\n",
            "  1800                                                      INTERVAL\n",
            "    99                                                      # OF MAPS IN FILE\n",
            "  COSZ                                                      MAPPING FUNCTION\n",
            "  6371.0                                                    BASE RADIUS\n",
            "     2                                                      MAP DIMENSION\n",
            "   450.0 450.0   0.0                                        HGT1 / HGT2 / DHGT\n",
            "    60.0 -60.0 -20.0                                        LAT1 / LAT2 / DLAT\n",
            "  -180.0 180.0  60.0                                        LON1 / LON2 / DLON\n",
            "                                                            END OF HEADER\n",
            "     1                                                      START OF TEC MAP\n",
            "  2020     1     1     0     0     0                        EPOCH OF CURRENT MAP\n",
            // Map 1 sets an exponent other than the default -1 the header
            // leaves in effect, so map 2, which states none, carries it. A map
            // restating the default carries nothing and is not reported.
            "    -2                                                      EXPONENT\n",
            "    60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  100  101  102  103  104  105  106\n",
            "    40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  110  111  112  113  114  115  116\n",
            "    20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  120  121  122  123  124  125  126\n",
            "     0.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  130  131  132  133  134  135  136\n",
            "   -20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  140  141  142  143  144  145  146\n",
            "   -40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  150  151  152  153  154  155  156\n",
            "   -60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  160  161  162  163  164  165  166\n",
            "     1                                                      END OF TEC MAP\n",
            "     2                                                      START OF TEC MAP\n",
            "  2020     1     1     1     0     0                        EPOCH OF CURRENT MAP\n",
            "    60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  200  201  202  203  204  205  206\n",
            "    40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  210  211  212  213  214  215  216\n",
            "    20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  220  221  222  223  224  225  226\n",
            "     0.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  230  231  232  233  234  235  236\n",
            "   -20.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  240  241  242  243  244  245  246\n",
            "   -40.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  250  251  252  253  254  255  256\n",
            "   -60.0-180.0 180.0  60.0 450.0                            LAT/LON1/LON2/DLON/H\n",
            "  nan  261  262  263  264  265  266\n",
            "     2                                                      END OF TEC MAP\n",
            "                                                            END OF FILE\n",
        );

        unsafe {
            let mut ionex: *mut SidereonIonex = ptr::null_mut();
            let mut warnings: *mut SidereonIonexWarningList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse_with_warnings(
                    FIXTURE_WITH_WARNINGS.as_ptr(),
                    FIXTURE_WITH_WARNINGS.len(),
                    &mut ionex,
                    &mut warnings,
                ),
                SidereonStatus::Ok
            );

            let mut count = 0usize;
            assert_eq!(
                sidereon_ionex_warning_list_count(warnings, &mut count),
                SidereonStatus::Ok
            );
            // sidereon-core's own warnings for the same text, in order.
            let (_, core_warnings) = Ionex::parse_with_warnings(FIXTURE_WITH_WARNINGS.as_bytes())
                .expect("core parses the fixture");
            assert_eq!(count, core_warnings.len());

            let mut seen_version = false;
            let mut seen_map_count = false;
            let mut seen_not_a_number = false;
            let mut seen_interval = false;
            let mut seen_exponent = false;
            let mut seen_epoch = false;
            for (index, core_warning) in core_warnings.iter().enumerate() {
                let mut info = zero_warning_info();
                assert_eq!(
                    sidereon_ionex_warning_get_info(warnings, index, &mut info),
                    SidereonStatus::Ok
                );
                let want = warning_info_to_c(core_warning);
                assert_eq!(info.kind, want.kind);
                assert_eq!(
                    (
                        info.line,
                        info.map_number,
                        info.set_by_line,
                        info.declared_count,
                        info.tec_map_count,
                        info.all_map_count
                    ),
                    (
                        want.line,
                        want.map_number,
                        want.set_by_line,
                        want.declared_count,
                        want.tec_map_count,
                        want.all_map_count
                    )
                );
                assert_eq!(
                    (
                        info.declared_interval_s,
                        info.actual_spacing_s,
                        info.exponent,
                        info.has_epochs,
                        info.declared_epoch_j2000_whole_s,
                        info.maps_epoch_j2000_whole_s
                    ),
                    (
                        want.declared_interval_s,
                        want.actual_spacing_s,
                        want.exponent,
                        want.has_epochs,
                        want.declared_epoch_j2000_whole_s,
                        want.maps_epoch_j2000_whole_s
                    )
                );
                for (got, want) in [
                    (info.lat_deg, want.lat_deg),
                    (info.lon_deg, want.lon_deg),
                    (info.declared_epoch_j2000_s, want.declared_epoch_j2000_s),
                    (info.maps_epoch_j2000_s, want.maps_epoch_j2000_s),
                ] {
                    assert_eq!(got.to_bits(), want.to_bits());
                }
                let message = copy_text(|out, len, w, r| {
                    sidereon_ionex_warning_get_message(warnings, index, out, len, w, r)
                });
                assert_eq!(message, core_warning.to_string());

                // FIXTURE_WITH_WARNINGS is written to raise each of these
                // kinds, so every arm of the kind mapping is read.
                match info.kind {
                    SidereonIonexWarningKind::VersionRecordNotFirst => seen_version = true,
                    SidereonIonexWarningKind::MapCountMismatch => seen_map_count = true,
                    SidereonIonexWarningKind::NotANumberValue => {
                        seen_not_a_number = true;
                        let label = copy_text(|out, len, w, r| {
                            sidereon_ionex_warning_get_label(warnings, index, out, len, w, r)
                        });
                        assert_eq!(label, warning_label(core_warning));
                    }
                    SidereonIonexWarningKind::IntervalMismatch => seen_interval = true,
                    SidereonIonexWarningKind::ExponentCarriedIntoMap => seen_exponent = true,
                    SidereonIonexWarningKind::EpochMismatch => seen_epoch = true,
                    _ => {}
                }
            }
            assert!(seen_version, "the version record follows a comment");
            assert!(seen_map_count, "the file declares 99 maps and holds 2");
            assert!(seen_not_a_number, "the last map gives a nan value");
            assert!(
                seen_exponent,
                "the second map inherits the EXPONENT the first one set"
            );
            assert!(seen_interval, "the file declares a 1800 s interval");
            assert!(
                seen_epoch,
                "EPOCH OF LAST MAP names an epoch the maps do not"
            );

            // An index past the end is refused rather than read.
            let mut info = zero_warning_info();
            assert_eq!(
                sidereon_ionex_warning_get_info(warnings, count, &mut info),
                SidereonStatus::InvalidArgument
            );

            sidereon_ionex_warning_list_free(warnings);
            sidereon_ionex_warning_list_free(ptr::null_mut());
            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn a_refused_parse_leaves_no_handle_behind() {
        unsafe {
            let mut ionex: *mut SidereonIonex = std::ptr::dangling_mut();
            let mut warnings: *mut SidereonIonexWarningList = std::ptr::dangling_mut();
            let garbage = b"not an IONEX file";
            assert_ne!(
                sidereon_ionex_parse_with_warnings(
                    garbage.as_ptr(),
                    garbage.len(),
                    &mut ionex,
                    &mut warnings,
                ),
                SidereonStatus::Ok
            );
            assert!(ionex.is_null());
            assert!(warnings.is_null());

            let mut ionex: *mut SidereonIonex = std::ptr::dangling_mut();
            assert_ne!(
                sidereon_ionex_parse(garbage.as_ptr(), garbage.len(), &mut ionex),
                SidereonStatus::Ok
            );
            assert!(ionex.is_null());
        }
    }

    /// A null output slot still clears the partner slot the call can write.
    ///
    /// Both outputs are cleared before either is validated, so whichever of
    /// the two the caller supplied comes back NULL instead of holding the
    /// sentinel it went in with. Both orders are exercised: only one of them
    /// is the slot the call happens to look at first, and the other is the one
    /// the earlier order left stale. The bytes are a product that parses, so
    /// the refusal is the null slot and nothing else, and no owned handle is
    /// transferred either way.
    #[test]
    fn a_null_parse_output_still_clears_the_partner_output() {
        unsafe {
            let data = VALID_IONEX_FIXTURE.as_bytes();

            // A null product slot: the warning slot is cleared all the same.
            let mut warnings: *mut SidereonIonexWarningList = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_ionex_parse_with_warnings(
                    data.as_ptr(),
                    data.len(),
                    ptr::null_mut(),
                    &mut warnings,
                ),
                SidereonStatus::NullPointer
            );
            assert!(warnings.is_null());

            // A null warning slot: the product slot is cleared all the same.
            let mut ionex: *mut SidereonIonex = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_ionex_parse_with_warnings(
                    data.as_ptr(),
                    data.len(),
                    &mut ionex,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert!(ionex.is_null());

            // Both null is the same refusal with nothing to write.
            assert_eq!(
                sidereon_ionex_parse_with_warnings(
                    data.as_ptr(),
                    data.len(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
        }
    }

    #[test]
    fn serializing_a_product_the_format_cannot_state_refuses_rather_than_writing_nothing() {
        unsafe {
            let fixture = GridFixture::new();

            // A product whose records the format holds writes text.
            let writable = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&writable);
            let text =
                copy_text(|out, len, w, r| sidereon_ionex_to_ionex_text(ionex, out, len, w, r));
            assert!(text.contains("END OF FILE"));
            sidereon_ionex_free(ionex);

            // A `MAPPING FUNCTION` code longer than the four bytes its field
            // holds cannot be written, and the route says so rather than
            // reporting an empty success.
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            let code = b"SLAB_3D_CUSTOM";
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::Other as u32,
                    code.as_ptr(),
                    code.len()
                ),
                SidereonStatus::Ok
            );
            let refusing = fixture.samples(false, false, header);
            let ionex = build_product(&refusing);

            let mut written = 1usize;
            let mut required = 1usize;
            assert_ne!(
                sidereon_ionex_to_ionex_text(
                    ionex,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);

            // The code itself round-trips through the header accessor even
            // though the IONEX text cannot state it.
            let mut stored: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(ionex, &mut stored),
                SidereonStatus::Ok
            );
            let read_back = copy_text(|out, len, w, r| {
                sidereon_ionex_header_get_mapping_function_code(stored, out, len, w, r)
            });
            assert_eq!(read_back, "SLAB_3D_CUSTOM");

            sidereon_ionex_header_free(stored);
            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);
        }
    }

    /// A standalone grid whose three axes differ, so an evaluation that swapped
    /// latitude for longitude, or left an angle in radians, cannot land on the
    /// same answer. The values double from the first epoch to the second.
    struct StandaloneGrid {
        epochs_ns: [f64; 2],
        latitudes_deg: [f64; 2],
        longitudes_deg: [f64; 2],
        values: [f64; 8],
        presence: [bool; 8],
    }

    impl StandaloneGrid {
        fn new() -> Self {
            Self {
                epochs_ns: [0.0, 1000.0],
                latitudes_deg: [-10.0, 10.0],
                longitudes_deg: [20.0, 60.0],
                // Flat epoch-latitude-longitude order, longitude fastest.
                values: [1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0],
                presence: [true; 8],
            }
        }

        unsafe fn build(&self) -> *mut SidereonTecGrid {
            let mut grid: *mut SidereonTecGrid = ptr::null_mut();
            let mut error = no_tec_grid_error();
            assert_eq!(
                sidereon_tec_grid_new(
                    self.epochs_ns.as_ptr(),
                    2,
                    self.latitudes_deg.as_ptr(),
                    2,
                    self.longitudes_deg.as_ptr(),
                    2,
                    self.values.as_ptr(),
                    self.presence.as_ptr(),
                    8,
                    &mut grid,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            assert!(!grid.is_null());
            assert_eq!(error.kind, SidereonTecGridErrorKind::None);
            grid
        }
    }

    #[test]
    fn a_standalone_grid_interpolates_to_an_independently_computed_value() {
        unsafe {
            let fixture = StandaloneGrid::new();
            let grid = fixture.build();

            // Query at a quarter of the epoch interval, the middle of the
            // latitude span, and a quarter of the longitude span.
            //
            // Within the first epoch the latitude rows interpolate along
            // longitude to 1.0*0.75 + 3.0*0.25 = 1.5 and
            // 7.0*0.75 + 15.0*0.25 = 9.0, and across latitude to
            // 1.5*0.5 + 9.0*0.5 = 5.25. The second epoch holds twice the first
            // everywhere, so it gives 10.5, and the time blend is
            // 5.25*0.75 + 10.5*0.25 = 6.5625 TECU.
            const EXPECTED_TECU: f64 = 6.5625;

            let mut vtec = f64::NAN;
            let mut gap = empty_node_gap_c();
            let mut error = no_tec_grid_error();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            // sidereon-core's own value for the same query.
            let core_value = (*grid)
                .inner
                .vtec_at_pierce_point_with_policy(
                    TecGridEpoch::new(250, 0),
                    30.0,
                    0.0,
                    IonexMissingNodePolicy::Strict,
                )
                .expect("core value")
                .value;
            assert_eq!(vtec.to_bits(), core_value.to_bits());
            // The degrees reach the engine unchanged and every weight here is
            // a dyadic fraction, so the value is also the hand-computed one.
            assert_eq!(vtec, EXPECTED_TECU);
            assert!(!gap.has_gap);
            assert_eq!(error.kind, SidereonTecGridErrorKind::None);

            // Passing the pair the other way round sends 30 degrees at the
            // latitude axis, which covers only 10 either side, so the argument
            // order is pinned by this refusal.
            let mut swapped = f64::NAN;
            let status = sidereon_tec_grid_vtec_at_pierce_point(
                grid,
                250,
                30.0,
                0.0,
                SidereonIonexMissingNodePolicy::Strict as u32,
                &mut swapped,
                &mut gap,
                &mut error,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert!(swapped.is_nan());
            assert_eq!(error.kind, SidereonTecGridErrorKind::OutOfBounds);
            assert_eq!(error.axis, SidereonTecGridAxis::Latitude);
            assert!(error.has_axis_value);
            assert_eq!(error.axis_value, 30.0);

            // Angles supplied in radians land inside the latitude axis but
            // short of the longitude one, which the axis names in turn, so the
            // unit is pinned too.
            let mut radians = f64::NAN;
            let status = sidereon_tec_grid_vtec_at_pierce_point(
                grid,
                250,
                0.0,
                30.0_f64.to_radians(),
                SidereonIonexMissingNodePolicy::Strict as u32,
                &mut radians,
                &mut gap,
                &mut error,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert_eq!(error.kind, SidereonTecGridErrorKind::OutOfBounds);
            assert_eq!(error.axis, SidereonTecGridAxis::Longitude);

            // A query on a node is that node's value exactly: at the first
            // epoch, the second latitude and the second longitude the grid
            // holds 15.0. A radians round trip on 60 degrees gives
            // 59.99999999999999, a query just off the node whose value is not
            // the node's.
            let mut on_node = f64::NAN;
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    0,
                    10.0,
                    60.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut on_node,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            // StandaloneGrid::new stores 15.0 at that node.
            assert_eq!(on_node, 15.0);

            sidereon_tec_grid_free(grid);
            sidereon_tec_grid_free(ptr::null_mut());
        }
    }

    #[test]
    fn a_standalone_grid_reports_its_stored_axes_and_flat_order() {
        unsafe {
            let fixture = StandaloneGrid::new();
            let grid = fixture.build();

            let mut epoch_count = 0usize;
            let mut latitude_count = 0usize;
            let mut longitude_count = 0usize;
            let mut value_count = 0usize;
            assert_eq!(
                sidereon_tec_grid_dimensions(
                    grid,
                    &mut epoch_count,
                    &mut latitude_count,
                    &mut longitude_count,
                    &mut value_count,
                ),
                SidereonStatus::Ok
            );
            assert_eq!((epoch_count, latitude_count, longitude_count), (2, 2, 2));
            assert_eq!(value_count, 8);

            let mut written = 0usize;
            let mut required = 0usize;
            let mut epochs = [0.0f64; 2];
            assert_eq!(
                sidereon_tec_grid_epochs_ns(
                    grid,
                    epochs.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(epochs, fixture.epochs_ns);

            let mut latitudes = [0.0f64; 2];
            assert_eq!(
                sidereon_tec_grid_latitudes_deg(
                    grid,
                    latitudes.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(latitudes, fixture.latitudes_deg);

            let mut longitudes = [0.0f64; 2];
            assert_eq!(
                sidereon_tec_grid_longitudes_deg(
                    grid,
                    longitudes.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(longitudes, fixture.longitudes_deg);

            let mut values = [0.0f64; 8];
            assert_eq!(
                sidereon_tec_grid_values_tecu(
                    grid,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 8);
            assert_eq!(values, fixture.values);
            // Longitude varies fastest: index 1 is the first epoch's first
            // latitude at the second longitude.
            assert_eq!(values[1], 3.0);
            // The next latitude of the same epoch follows the whole longitude
            // row, and the second epoch follows the whole first map.
            assert_eq!(values[2], 7.0);
            assert_eq!(values[4], 2.0);

            // A short buffer reports the required count and writes nothing.
            let mut small = [0.0f64; 2];
            assert_eq!(
                sidereon_tec_grid_values_tecu(
                    grid,
                    small.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, 8);

            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn a_standalone_grid_keeps_zero_apart_from_a_missing_node() {
        unsafe {
            let mut fixture = StandaloneGrid::new();
            // An explicit zero, and a missing node whose numeric slot holds a
            // value the mask says to ignore.
            fixture.values[0] = 0.0;
            fixture.values[3] = 4242.0;
            fixture.presence[3] = false;
            let grid = fixture.build();

            let mut written = 0usize;
            let mut required = 0usize;
            let mut values = [-1.0f64; 8];
            let mut presence = [true; 8];
            assert_eq!(
                sidereon_tec_grid_values_tecu(
                    grid,
                    values.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_tec_grid_value_presence(
                    grid,
                    presence.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(values[0], 0.0);
            assert!(presence[0]);
            assert!(values[3].is_nan());
            assert!(!presence[3]);

            // A strict query that weights the missing corner is refused and
            // names it; the renormalizing one interpolates around it and marks
            // the value degraded.
            let mut vtec = f64::NAN;
            let mut gap = empty_node_gap_c();
            let mut error = no_tec_grid_error();
            let status = sidereon_tec_grid_vtec_at_pierce_point(
                grid,
                250,
                0.0,
                30.0,
                SidereonIonexMissingNodePolicy::Strict as u32,
                &mut vtec,
                &mut gap,
                &mut error,
            );
            assert_ne!(status, SidereonStatus::Ok);
            assert!(vtec.is_nan());
            assert_eq!(error.kind, SidereonTecGridErrorKind::NodesNotAvailable);
            assert!(error.has_gap);
            assert!(gap.has_gap);
            assert_missing_nodes(&gap.earlier, core_strict_grid_gap(grid).earlier);

            let mut gap = empty_node_gap_c();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Renormalize as u32,
                    &mut vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            assert!(vtec.is_finite());
            assert!(gap.has_gap);
            assert!(gap.earlier.has_missing);

            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn a_standalone_grid_refuses_malformed_dimensions_and_tags() {
        unsafe {
            let fixture = StandaloneGrid::new();
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut error = no_tec_grid_error();

            // Axis lengths whose product overflows are refused before any
            // buffer is read, so the counts may exceed the arrays supplied.
            let huge = 1usize << 40;
            assert_eq!(
                sidereon_tec_grid_new(
                    fixture.epochs_ns.as_ptr(),
                    huge,
                    fixture.latitudes_deg.as_ptr(),
                    huge,
                    fixture.longitudes_deg.as_ptr(),
                    huge,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut error,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(grid.is_null());
            assert_eq!(error.kind, SidereonTecGridErrorKind::DimensionsOverflow);

            // A value count that disagrees with the axes is refused with both
            // counts.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_new(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    7,
                    &mut grid,
                    &mut error,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(grid.is_null());
            assert_eq!(error.kind, SidereonTecGridErrorKind::ValueCountMismatch);
            assert_eq!(error.value_count, 7);
            assert_eq!(error.expected_value_count, 8);

            // An axis that does not increase is the engine's own refusal.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let descending = [10.0f64, -10.0];
            assert_eq!(
                sidereon_tec_grid_new(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    descending.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut error,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(grid.is_null());
            assert_eq!(error.kind, SidereonTecGridErrorKind::AxesNotIncreasing);

            // A present value that is not finite is refused by index.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut values = fixture.values;
            values[5] = f64::NAN;
            assert_eq!(
                sidereon_tec_grid_new(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut error,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(grid.is_null());
            // This binding's own check, by index. The engine never saw the
            // value, so the failure claims no engine field or reason.
            assert_eq!(error.kind, SidereonTecGridErrorKind::ValueNotFinite);
            assert!(error.has_value_index);
            assert_eq!(error.value_index, 5);

            // A null output pointer is refused before anything is allocated.
            assert_eq!(
                sidereon_tec_grid_new(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    ptr::null_mut(),
                    &mut error,
                ),
                SidereonStatus::NullPointer
            );

            // An unrecognized missing-node tag is refused rather than read as a
            // policy the engine names.
            let grid = fixture.build();
            let mut vtec = 1.0f64;
            let mut gap = empty_node_gap_c();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    0.0,
                    30.0,
                    7,
                    &mut vtec,
                    &mut gap,
                    ptr::null_mut(),
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(vtec.is_nan());
            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn a_policy_tag_outside_the_named_values_is_refused() {
        unsafe {
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);

            let mut evaluation = refused_ionex_slant_delay_evaluation();
            let mut error = no_ionex_slant_error();
            for bad in [policy(9, 0, 1), policy(0, 9, 1), policy(0, 0, 9)] {
                assert_eq!(
                    sidereon_ionex_slant_delay_with_policy(
                        ionex,
                        0.0,
                        0.0,
                        0.0,
                        85.0,
                        FIXTURE_FIRST_EPOCH_J2000_S,
                        1_575_420_000.0,
                        bad,
                        &mut evaluation,
                        &mut error,
                    ),
                    SidereonStatus::InvalidArgument
                );
                assert!(evaluation.delay_m.is_nan());
                assert!(!evaluation.status.is_valid);
            }

            // The default and the coverage-only constructors agree on the two
            // fields the coverage form does not take.
            let default_policy = sidereon_ionex_slant_policy_default();
            let hold =
                sidereon_ionex_slant_policy_from_coverage(SidereonIonexCoveragePolicy::Hold as u32);
            assert_eq!(default_policy.missing_nodes, hold.missing_nodes);
            assert_eq!(default_policy.mapping, hold.mapping);
            assert_eq!(
                default_policy.coverage,
                SidereonIonexCoveragePolicy::Strict as u32
            );
            assert_eq!(hold.coverage, SidereonIonexCoveragePolicy::Hold as u32);

            sidereon_ionex_free(ionex);
        }
    }

    /// The shape of every owned standalone TEC-grid text accessor.
    type TecGridTextFn = unsafe extern "C" fn(
        *const SidereonTecGridResult,
        *mut u8,
        usize,
        *mut usize,
        *mut usize,
    ) -> SidereonStatus;

    /// Read one owned text through the variable-length output contract: first
    /// the required length, then the bytes.
    unsafe fn owned_tec_text(read: TecGridTextFn, result: *const SidereonTecGridResult) -> String {
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            read(result, ptr::null_mut(), 0, &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        let mut buf = vec![0u8; required];
        let len = buf.len();
        assert_eq!(
            read(result, buf.as_mut_ptr(), len, &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        assert_eq!(written, len);
        String::from_utf8(buf).expect("the engine's text is utf-8")
    }

    /// Compare two outcomes that report no value. A direct equality check
    /// would never hold: both carry NaN where a value would be.
    fn same_failed_outcome(left: SidereonTecGridOutcome, right: SidereonTecGridOutcome) {
        assert_eq!(left.is_ok, right.is_ok);
        assert_eq!(left.status, right.status);
        assert_eq!(left.has_vtec, right.has_vtec);
        assert!(left.vtec_tecu.is_nan() && right.vtec_tecu.is_nan());
        assert_eq!(left.degraded, right.degraded);
        assert_eq!(left.error, right.error);
    }

    unsafe fn owned_tec_outcome(result: *const SidereonTecGridResult) -> SidereonTecGridOutcome {
        let mut outcome = unread_tec_grid_outcome();
        assert_eq!(
            sidereon_tec_grid_result_get_outcome(result, &mut outcome),
            SidereonStatus::Ok
        );
        outcome
    }

    #[test]
    fn an_owned_grid_result_keeps_the_invalid_field_bytes_after_a_later_failure_and_release() {
        unsafe {
            let fixture = StandaloneGrid::new();
            let mut grid: *mut SidereonTecGrid = ptr::null_mut();
            let mut built: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    fixture.presence.as_ptr(),
                    8,
                    &mut grid,
                    &mut built,
                ),
                SidereonStatus::Ok
            );
            assert!(!grid.is_null());
            assert!(!built.is_null());
            let outcome = owned_tec_outcome(built);
            assert!(outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::Ok);
            // Building a grid evaluates nothing, so the result holds no value.
            assert!(!outcome.has_vtec);
            assert!(outcome.vtec_tecu.is_nan());
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::None);
            assert!(owned_tec_text(sidereon_tec_grid_result_get_message, built).is_empty());
            sidereon_tec_grid_result_free(built);

            // A latitude that is not finite reaches the engine's shared
            // validation, which names the field and the reason.
            let mut invalid: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    250,
                    f64::NAN,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut invalid,
                ),
                SidereonStatus::Ok
            );
            assert!(!invalid.is_null());
            let outcome = owned_tec_outcome(invalid);
            assert!(!outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::InvalidArgument);
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::InvalidField);
            assert!(!outcome.has_vtec);
            assert!(outcome.vtec_tecu.is_nan());

            // A second failing call on the convenience route overwrites the
            // thread-local message, and the grid is then released.
            let mut vtec = 0.0;
            let mut gap = empty_node_gap_c();
            let mut error = no_tec_grid_error();
            assert_ne!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    30.0,
                    0.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            let needed = sidereon_last_error_message(ptr::null_mut(), 0);
            let mut buf = vec![0 as c_char; needed + 1];
            let len = buf.len();
            sidereon_last_error_message(buf.as_mut_ptr(), len);
            let thread_local = CStr::from_ptr(buf.as_ptr())
                .to_str()
                .expect("the engine's text is utf-8")
                .to_owned();
            assert!(thread_local.contains("out of TEC grid bounds"));
            assert!(!thread_local.contains("not finite"));
            sidereon_tec_grid_free(grid);

            // The owned result still reports the exact field, reason and
            // message bytes the engine gave it.
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_field, invalid),
                "latitude"
            );
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_reason, invalid),
                "not finite"
            );
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_message, invalid),
                "sidereon_tec_grid_vtec_at_pierce_point_result: latitude not finite"
            );
            same_failed_outcome(owned_tec_outcome(invalid), outcome);

            sidereon_tec_grid_result_free(invalid);
            // Releasing NULL is a no-op.
            sidereon_tec_grid_result_free(ptr::null_mut());
        }
    }

    /// The engine clamps a pierce-point latitude to `[-87.5, 87.5]` degrees,
    /// and the value that comes back is the one at the effective coordinate.
    ///
    /// The grid's latitude axis is `[80.0, 89.0]`, so 89 degrees is a node of
    /// it: an unclamped evaluation would return that node's value outright.
    /// The clamp instead evaluates at 87.5, five sixths of the way up the span,
    /// and the two answers are far apart, so this cannot pass by coincidence.
    /// The clamp is silent: the row succeeds with no degraded or gap marker,
    /// because nothing was interpolated around and no node was missing. Both
    /// exported pierce-point routes reach the engine through one helper, so
    /// each is asserted, and the narrower axis shows the effective coordinate
    /// is also what an out-of-bounds failure names.
    #[test]
    fn a_pierce_point_latitude_is_clamped_to_the_grid_band() {
        unsafe {
            // Flat epoch-latitude-longitude order, longitude fastest, so at the
            // first epoch and the first longitude the latitude column is
            // 1.0 at 80 degrees and 7.0 at 89 degrees.
            let fixture = StandaloneGrid {
                latitudes_deg: [80.0, 89.0],
                ..StandaloneGrid::new()
            };
            let grid = fixture.build();

            // 87.5 is (87.5 - 80) / (89 - 80) of the way up that column.
            const CLAMPED_TECU: f64 = 1.0 + 6.0 * (7.5 / 9.0);
            // What the same query would give if the supplied 89 were used.
            const SUPPLIED_NODE_TECU: f64 = 7.0;
            assert!((CLAMPED_TECU - SUPPLIED_NODE_TECU).abs() > 0.9);

            let mut vtec = f64::NAN;
            let mut gap = empty_node_gap_c();
            let mut error = no_tec_grid_error();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    0,
                    89.0,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            // The clamp substitutes the literal 87.5, so the caller's 89 does
            // not reach the interpolation at all.
            let core_clamped = core_grid_value(grid, 0, 89.0, 20.0).expect("core value");
            assert_eq!(vtec.to_bits(), core_clamped.to_bits());
            // The value is the one at 87.5 degrees, not the supplied node's.
            assert!(
                (vtec - CLAMPED_TECU).abs() < 1.0e-12,
                "expected the value at 87.5 degrees, got {vtec}"
            );
            assert!((vtec - SUPPLIED_NODE_TECU).abs() > 0.9);
            // Nothing marks the substitution: no gap, no typed error.
            assert!(!gap.has_gap);
            assert_eq!(error.kind, SidereonTecGridErrorKind::None);

            // A supplied 87.5 lands on the same value, which is what makes the
            // clamped answer indistinguishable from an in-band query.
            let mut in_band = f64::NAN;
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    0,
                    87.5,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut in_band,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                in_band.to_bits(),
                core_grid_value(grid, 0, 87.5, 20.0)
                    .expect("core value")
                    .to_bits()
            );
            assert!((in_band - vtec).abs() < 1.0e-12);

            // The complete typed route clamps identically and reports no more.
            let mut result: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    0,
                    89.0,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            let outcome = owned_tec_outcome(result);
            assert!(outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::Ok);
            assert!(outcome.has_vtec);
            assert_eq!(outcome.vtec_tecu.to_bits(), core_clamped.to_bits());
            assert!(!outcome.degraded.has_gap);
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::None);
            sidereon_tec_grid_result_free(result);
            sidereon_tec_grid_free(grid);

            // On an axis narrower than the clamp band the query still fails,
            // and the coordinate the failure names is the effective 87.5, not
            // the 89 the caller supplied.
            let narrow = StandaloneGrid {
                latitudes_deg: [80.0, 85.0],
                ..StandaloneGrid::new()
            };
            let narrow_grid = narrow.build();
            let mut narrow_result: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    narrow_grid,
                    0,
                    89.0,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut narrow_result,
                ),
                SidereonStatus::Ok
            );
            let narrow_outcome = owned_tec_outcome(narrow_result);
            assert!(!narrow_outcome.is_ok);
            assert_eq!(narrow_outcome.status, SidereonStatus::Solve);
            assert_eq!(
                narrow_outcome.error.kind,
                SidereonTecGridErrorKind::OutOfBounds
            );
            assert_eq!(narrow_outcome.error.axis, SidereonTecGridAxis::Latitude);
            assert!(narrow_outcome.error.has_axis_value);
            let core_narrow = core_out_of_bounds_value(narrow_grid, 0, 89.0, 20.0);
            assert_eq!(
                narrow_outcome.error.axis_value.to_bits(),
                core_narrow.to_bits()
            );
            // The clamp bound, not the 89 supplied.
            assert_eq!(narrow_outcome.error.axis_value, 87.5);
            assert!(narrow_outcome.vtec_tecu.is_nan());
            sidereon_tec_grid_result_free(narrow_result);

            // The convenience route names the same effective coordinate.
            let mut narrow_vtec = 0.0;
            assert_ne!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    narrow_grid,
                    0,
                    89.0,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut narrow_vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(error.kind, SidereonTecGridErrorKind::OutOfBounds);
            assert_eq!(error.axis, SidereonTecGridAxis::Latitude);
            assert!(error.has_axis_value);
            assert_eq!(error.axis_value.to_bits(), core_narrow.to_bits());

            sidereon_tec_grid_free(narrow_grid);
        }
    }

    /// The clamp is a comparison, so it takes an infinite latitude and leaves
    /// a NaN one alone.
    ///
    /// On the same `[80.0, 89.0]` axis, `+INFINITY` is above the upper bound
    /// and evaluates at 87.5, giving bit for bit the value the 89-degree query
    /// gives; `-INFINITY` is below the lower bound and evaluates at -87.5,
    /// which that axis cannot reach, so it is an ordinary out-of-bounds
    /// failure naming -87.5 rather than an infinity. `NAN` compares false
    /// against both bounds, so no clamp applies, the engine's shared
    /// validation refuses it, and the owned route keeps the exact field and
    /// reason. The binding adds no finite-angle check of its own: these are
    /// the engine's own accepted inputs.
    #[test]
    fn an_infinite_pierce_point_latitude_is_clamped_and_a_nan_one_is_refused() {
        unsafe {
            let fixture = StandaloneGrid {
                latitudes_deg: [80.0, 89.0],
                ..StandaloneGrid::new()
            };
            let grid = fixture.build();

            // The same value the existing 89-degree query lands on.
            const CLAMPED_TECU: f64 = 1.0 + 6.0 * (7.5 / 9.0);

            let mut vtec = f64::NAN;
            let mut gap = empty_node_gap_c();
            let mut error = no_tec_grid_error();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    0,
                    f64::INFINITY,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut vtec,
                    &mut gap,
                    &mut error,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                vtec.to_bits(),
                core_grid_value(grid, 0, f64::INFINITY, 20.0)
                    .expect("core value")
                    .to_bits()
            );
            assert!(
                (vtec - CLAMPED_TECU).abs() < 1.0e-12,
                "an infinite latitude evaluates at 87.5, got {vtec}"
            );
            assert!(!gap.has_gap);
            assert_eq!(error.kind, SidereonTecGridErrorKind::None);

            // The typed route clamps an infinity the same silent way.
            let mut result: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    0,
                    f64::INFINITY,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            let outcome = owned_tec_outcome(result);
            assert!(outcome.is_ok);
            assert!(outcome.has_vtec);
            assert_eq!(outcome.vtec_tecu.to_bits(), vtec.to_bits());
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::None);
            sidereon_tec_grid_result_free(result);

            // A negative infinity clamps to the lower bound, which this axis
            // does not cover, so the failure names the effective -87.5.
            let mut below: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    0,
                    f64::NEG_INFINITY,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut below,
                ),
                SidereonStatus::Ok
            );
            let below_outcome = owned_tec_outcome(below);
            assert!(!below_outcome.is_ok);
            assert_eq!(below_outcome.status, SidereonStatus::Solve);
            assert_eq!(
                below_outcome.error.kind,
                SidereonTecGridErrorKind::OutOfBounds
            );
            assert_eq!(below_outcome.error.axis, SidereonTecGridAxis::Latitude);
            assert!(below_outcome.error.has_axis_value);
            // Exactly the bound: the clamp writes the literal, and an
            // infinity never reaches the axis lookup to be reported.
            assert_eq!(
                below_outcome.error.axis_value.to_bits(),
                core_out_of_bounds_value(grid, 0, f64::NEG_INFINITY, 20.0).to_bits()
            );
            assert_eq!(below_outcome.error.axis_value, -87.5);
            assert!(below_outcome.error.axis_value.is_finite());
            assert!(below_outcome.vtec_tecu.is_nan());
            sidereon_tec_grid_result_free(below);

            // NaN is not clamped: it reaches the shared validation, and the
            // owned result keeps the field and reason that names.
            let mut refused: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    0,
                    f64::NAN,
                    20.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut refused,
                ),
                SidereonStatus::Ok
            );
            let refused_outcome = owned_tec_outcome(refused);
            assert!(!refused_outcome.is_ok);
            assert_eq!(refused_outcome.status, SidereonStatus::InvalidArgument);
            assert_eq!(
                refused_outcome.error.kind,
                SidereonTecGridErrorKind::InvalidField
            );
            assert!(!refused_outcome.has_vtec);
            assert!(refused_outcome.vtec_tecu.is_nan());
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_field, refused),
                "latitude"
            );
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_reason, refused),
                "not finite"
            );
            sidereon_tec_grid_result_free(refused);

            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn an_owned_grid_result_names_the_out_of_bounds_axis_and_the_refused_coordinate() {
        unsafe {
            let fixture = StandaloneGrid::new();
            let grid = fixture.build();

            // Sending the pair the other way round puts 30 degrees on a
            // latitude axis that covers only 10 either side.
            let mut result: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    250,
                    30.0,
                    0.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            let outcome = owned_tec_outcome(result);
            assert!(!outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::Solve);
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::OutOfBounds);
            assert_eq!(outcome.error.axis, SidereonTecGridAxis::Latitude);
            assert!(outcome.error.has_axis_value);
            // The degrees reach the engine unchanged, so the coordinate the
            // axis refused is exactly the 30 degrees supplied.
            assert_eq!(outcome.error.axis_value, 30.0);
            // No value was produced, so nothing is marked degraded.
            assert!(!outcome.degraded.has_gap);
            assert!(!outcome.error.has_gap);
            let message = owned_tec_text(sidereon_tec_grid_result_get_message, result);
            assert!(
                message.starts_with("sidereon_tec_grid_vtec_at_pierce_point_result: latitude "),
                "{message}"
            );
            assert!(message.ends_with(" is out of TEC grid bounds"), "{message}");
            // The engine named no field, so this binding invents none.
            assert!(owned_tec_text(sidereon_tec_grid_result_get_field, result).is_empty());
            assert!(owned_tec_text(sidereon_tec_grid_result_get_reason, result).is_empty());

            sidereon_tec_grid_result_free(result);
            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn an_owned_grid_result_keeps_the_indexed_gap_and_the_degraded_value() {
        unsafe {
            let mut fixture = StandaloneGrid::new();
            fixture.values[3] = 4242.0;
            fixture.presence[3] = false;
            let grid = fixture.build();

            let mut strict: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut strict,
                ),
                SidereonStatus::Ok
            );
            let outcome = owned_tec_outcome(strict);
            assert!(!outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::Solve);
            assert_eq!(
                outcome.error.kind,
                SidereonTecGridErrorKind::NodesNotAvailable
            );
            assert!(outcome.error.has_gap);
            assert!(outcome.error.gap.has_gap);
            let core_gap = core_strict_grid_gap(grid);
            assert_missing_nodes(&outcome.error.gap.earlier, core_gap.earlier);
            assert_eq!(
                outcome.error.gap.later.has_missing,
                core_gap.later.is_some()
            );
            // A refusal produced no value, so the degraded field stays empty
            // and the corners are read from the failure.
            assert!(!outcome.degraded.has_gap);

            let mut renormalized: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Renormalize as u32,
                    &mut renormalized,
                ),
                SidereonStatus::Ok
            );
            let degraded_outcome = owned_tec_outcome(renormalized);
            assert!(degraded_outcome.is_ok);
            assert!(degraded_outcome.has_vtec);
            assert!(degraded_outcome.vtec_tecu.is_finite());
            // The value was interpolated around the gap, so it is reported as
            // degraded rather than as a failure.
            assert!(degraded_outcome.degraded.has_gap);
            assert!(degraded_outcome.degraded.earlier.has_missing);
            assert_eq!(
                degraded_outcome.degraded.earlier.missing,
                [false, false, false, true]
            );
            assert_eq!(degraded_outcome.error.kind, SidereonTecGridErrorKind::None);
            assert!(owned_tec_text(sidereon_tec_grid_result_get_message, renormalized).is_empty());

            sidereon_tec_grid_free(grid);
            // Both results outlive the grid and each other.
            same_failed_outcome(owned_tec_outcome(strict), outcome);
            assert_eq!(owned_tec_outcome(renormalized), degraded_outcome);
            sidereon_tec_grid_result_free(renormalized);
            same_failed_outcome(owned_tec_outcome(strict), outcome);
            sidereon_tec_grid_result_free(strict);
        }
    }

    #[test]
    fn an_owned_grid_construction_failure_transfers_a_result_and_no_grid() {
        unsafe {
            let fixture = StandaloneGrid::new();

            // A value count that disagrees with the axes.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    7,
                    &mut grid,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            assert!(grid.is_null());
            assert!(!result.is_null());
            let outcome = owned_tec_outcome(result);
            assert!(!outcome.is_ok);
            assert_eq!(outcome.status, SidereonStatus::InvalidArgument);
            assert_eq!(
                outcome.error.kind,
                SidereonTecGridErrorKind::ValueCountMismatch
            );
            assert_eq!(outcome.error.value_count, 7);
            assert_eq!(outcome.error.expected_value_count, 8);
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_message, result),
                "sidereon_tec_grid_new_result: expected 8 values, got 7"
            );
            sidereon_tec_grid_result_free(result);

            // A present value that is not finite is this binding's own check,
            // by index, and claims no engine field or reason.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            let mut values = fixture.values;
            values[5] = f64::INFINITY;
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            assert!(grid.is_null());
            let outcome = owned_tec_outcome(result);
            assert_eq!(outcome.error.kind, SidereonTecGridErrorKind::ValueNotFinite);
            assert!(outcome.error.has_value_index);
            assert_eq!(outcome.error.value_index, 5);
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_message, result),
                "sidereon_tec_grid_new_result: the value at index 5 is marked present and is not \
                 finite"
            );
            assert!(owned_tec_text(sidereon_tec_grid_result_get_field, result).is_empty());
            assert!(owned_tec_text(sidereon_tec_grid_result_get_reason, result).is_empty());
            sidereon_tec_grid_result_free(result);

            // An axis the engine refuses keeps the engine's own text.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            let descending = [10.0f64, -10.0];
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    descending.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            assert!(grid.is_null());
            let outcome = owned_tec_outcome(result);
            assert_eq!(
                outcome.error.kind,
                SidereonTecGridErrorKind::AxesNotIncreasing
            );
            assert_eq!(
                owned_tec_text(sidereon_tec_grid_result_get_message, result),
                "sidereon_tec_grid_new_result: TEC grid axes must be strictly increasing"
            );
            sidereon_tec_grid_result_free(result);
        }
    }

    #[test]
    fn an_owned_grid_route_initializes_its_outputs_and_allocates_nothing_on_a_structural_failure() {
        unsafe {
            let fixture = StandaloneGrid::new();

            // A null result out-parameter leaves the grid out-parameter
            // cleared and transfers nothing.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    &mut grid,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert!(grid.is_null());

            // The other ordering: a null grid out-parameter must still clear
            // the result out-parameter, or a caller checking it for NULL reads
            // whatever it left there as a transferred handle.
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    ptr::null_mut(),
                    &mut result,
                ),
                SidereonStatus::NullPointer
            );
            assert!(result.is_null());

            // Both out-parameters null is a no-op beyond the status: nothing
            // is written and nothing is allocated.
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );

            // A null value buffer with a matching nonzero count is structural,
            // so the call itself fails and no result is allocated.
            let mut grid: *mut SidereonTecGrid = std::ptr::dangling_mut();
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_new_result(
                    fixture.epochs_ns.as_ptr(),
                    2,
                    fixture.latitudes_deg.as_ptr(),
                    2,
                    fixture.longitudes_deg.as_ptr(),
                    2,
                    ptr::null(),
                    ptr::null(),
                    8,
                    &mut grid,
                    &mut result,
                ),
                SidereonStatus::NullPointer
            );
            assert!(grid.is_null());
            assert!(result.is_null());

            let grid_handle = fixture.build();

            // A policy tag this binding does not name is structural too.
            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid_handle,
                    250,
                    0.0,
                    30.0,
                    7,
                    &mut result,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(result.is_null());

            let mut result: *mut SidereonTecGridResult = std::ptr::dangling_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    ptr::null(),
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut result,
                ),
                SidereonStatus::NullPointer
            );
            assert!(result.is_null());

            // An accessor writes its outputs before it validates the handle.
            let mut outcome = SidereonTecGridOutcome {
                is_ok: true,
                status: SidereonStatus::Ok,
                has_vtec: true,
                vtec_tecu: 1.0,
                degraded: empty_node_gap_c(),
                error: no_tec_grid_error(),
            };
            assert_eq!(
                sidereon_tec_grid_result_get_outcome(ptr::null(), &mut outcome),
                SidereonStatus::NullPointer
            );
            assert!(!outcome.is_ok);
            assert!(!outcome.has_vtec);
            assert!(outcome.vtec_tecu.is_nan());
            assert_eq!(
                sidereon_tec_grid_result_get_outcome(ptr::null(), ptr::null_mut()),
                SidereonStatus::NullPointer
            );

            // A buffer shorter than the message writes nothing and still
            // reports what the message needs.
            let mut refused: *mut SidereonTecGridResult = ptr::null_mut();
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point_result(
                    grid_handle,
                    250,
                    f64::NAN,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut refused,
                ),
                SidereonStatus::Ok
            );
            let mut written = usize::MAX;
            let mut required = 0usize;
            assert_eq!(
                sidereon_tec_grid_result_get_message(
                    refused,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert!(required > 0);
            let mut short = vec![0u8; required - 1];
            let short_len = short.len();
            assert_eq!(
                sidereon_tec_grid_result_get_message(
                    refused,
                    short.as_mut_ptr(),
                    short_len,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, short_len + 1);
            assert!(short.iter().all(|byte| *byte == 0));

            // A non-NULL buffer with a nonzero length is required when one is
            // offered at all; NULL with length zero is the query form.
            assert_eq!(
                sidereon_tec_grid_result_get_message(
                    refused,
                    ptr::null_mut(),
                    4,
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_tec_grid_result_get_field(
                    refused,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_tec_grid_result_get_reason(
                    refused,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );

            sidereon_tec_grid_result_free(refused);
            sidereon_tec_grid_free(grid_handle);
        }
    }

    unsafe fn thread_local_message() -> String {
        let needed = sidereon_last_error_message(ptr::null_mut(), 0);
        let mut buf = vec![0 as c_char; needed + 1];
        let len = buf.len();
        sidereon_last_error_message(buf.as_mut_ptr(), len);
        CStr::from_ptr(buf.as_ptr())
            .to_str()
            .expect("the text is utf-8")
            .to_owned()
    }

    unsafe fn samples_outcome(
        result: *const SidereonTecSamplesResult,
    ) -> SidereonTecSamplesOutcome {
        let mut outcome = SidereonTecSamplesOutcome {
            is_ok: true,
            status: SidereonStatus::Panic,
            error: no_tec_samples_error(),
        };
        assert_eq!(
            sidereon_tec_samples_result_get_outcome(result, &mut outcome),
            SidereonStatus::Ok
        );
        outcome
    }

    #[test]
    fn a_fractional_map_epoch_is_refused_by_index_rather_than_rounded() {
        unsafe {
            // Half a second past the second map, and the double one ulp past
            // it: neither names an IONEX map epoch.
            let whole = (FIXTURE_FIRST_EPOCH_J2000_S + 3600) as f64;
            for fractional in [whole + 0.5, f64::from_bits(whole.to_bits() + 1)] {
                assert_ne!(fractional.fract(), 0.0);
                let mut fixture = GridFixture::new();
                fixture.epochs[1] = fractional;
                let samples = fixture.samples(false, false, ptr::null());

                let mut ionex: *mut SidereonIonex = ptr::null_mut();
                assert_eq!(
                    sidereon_ionex_from_tec_grid_samples(&samples, &mut ionex),
                    SidereonStatus::InvalidArgument
                );
                assert!(ionex.is_null());
                let message = thread_local_message();
                assert!(
                    message.contains("samples.map_epochs_j2000_s[1]")
                        && message.contains("is not a whole second"),
                    "{message}"
                );

                let mut result: *mut SidereonTecSamplesResult = ptr::null_mut();
                assert_eq!(
                    sidereon_ionex_from_tec_grid_samples_result(&samples, &mut ionex, &mut result),
                    SidereonStatus::Ok
                );
                assert!(ionex.is_null());
                assert!(!result.is_null());
                let outcome = samples_outcome(result);
                assert!(!outcome.is_ok);
                assert_eq!(outcome.status, SidereonStatus::InvalidArgument);
                assert_eq!(
                    outcome.error.kind,
                    SidereonTecSamplesErrorKind::EpochNotRepresentable
                );
                assert_eq!(outcome.error.input, SidereonTecSamplesInput::MapEpoch);
                assert!(outcome.error.has_index);
                assert_eq!(outcome.error.index, 1);
                let owned = copy_text(|out, len, w, r| {
                    sidereon_tec_samples_result_get_message(result, out, len, w, r)
                });
                assert!(
                    owned.starts_with(
                        "sidereon_ionex_from_tec_grid_samples_result: \
                         samples.map_epochs_j2000_s[1] = "
                    ),
                    "{owned}"
                );
                sidereon_tec_samples_result_free(result);
            }

            // A per-node sample names its own index.
            let fixture = GridFixture::new();
            let product = build_product(&fixture.samples(false, false, ptr::null()));
            let mut nodes = vec![
                SidereonTecSample {
                    time_scale: 0,
                    epoch_j2000_s: 0.0,
                    has_epoch_j2000_whole_s: false,
                    epoch_j2000_whole_s: 0,
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    has_vtec_tecu: false,
                    vtec_tecu: 0.0,
                    has_rms_tecu: false,
                    rms_tecu: 0.0,
                    has_height_offset_km: false,
                    height_offset_km: 0.0,
                };
                8
            ];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_tec_samples(
                    product,
                    nodes.as_mut_ptr(),
                    nodes.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            sidereon_ionex_free(product);
            nodes[5].has_epoch_j2000_whole_s = false;
            nodes[5].epoch_j2000_s += 0.25;
            let mut ionex: *mut SidereonIonex = ptr::null_mut();
            let mut result: *mut SidereonTecSamplesResult = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_from_tec_samples_result(
                    nodes.as_ptr(),
                    nodes.len(),
                    450.0,
                    6371.0,
                    -1,
                    ptr::null(),
                    &mut ionex,
                    &mut result,
                ),
                SidereonStatus::Ok
            );
            assert!(ionex.is_null());
            let outcome = samples_outcome(result);
            assert_eq!(
                outcome.error.kind,
                SidereonTecSamplesErrorKind::EpochNotRepresentable
            );
            assert_eq!(outcome.error.input, SidereonTecSamplesInput::SampleEpoch);
            assert!(outcome.error.has_index);
            assert_eq!(outcome.error.index, 5);
            sidereon_tec_samples_result_free(result);

            // A non-finite present value names its flat index too.
            let mut fixture = GridFixture::new();
            fixture.tec_values[6] = f64::INFINITY;
            let samples = fixture.samples(false, false, ptr::null());
            let mut result: *mut SidereonTecSamplesResult = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_from_tec_grid_samples_result(&samples, &mut ionex, &mut result),
                SidereonStatus::Ok
            );
            let outcome = samples_outcome(result);
            assert_eq!(
                outcome.error.kind,
                SidereonTecSamplesErrorKind::NonFiniteValue
            );
            assert_eq!(outcome.error.input, SidereonTecSamplesInput::TecValue);
            assert_eq!(outcome.error.index, 6);
            sidereon_tec_samples_result_free(result);
        }
    }

    #[test]
    fn an_engine_samples_refusal_is_typed_and_owned() {
        unsafe {
            // Two map epochs out of order is the engine's refusal; the binding
            // reads both as whole seconds and hands them over.
            let mut fixture = GridFixture::new();
            fixture.epochs.swap(0, 1);
            let samples = fixture.samples(false, false, ptr::null());
            let mut ionex: *mut SidereonIonex = ptr::null_mut();
            let mut result: *mut SidereonTecSamplesResult = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_from_tec_grid_samples_result(&samples, &mut ionex, &mut result),
                SidereonStatus::Ok
            );
            assert!(ionex.is_null());
            // A later failing call overwrites the thread-local message; the
            // owned text is unaffected.
            let mut ignored = 0usize;
            assert_eq!(
                sidereon_ionex_epoch_count(ptr::null(), &mut ignored),
                SidereonStatus::NullPointer
            );
            let outcome = samples_outcome(result);
            assert_eq!(
                outcome.error.kind,
                SidereonTecSamplesErrorKind::NonMonotonicEpochs
            );
            assert!(!outcome.error.has_index);
            let owned = copy_text(|out, len, w, r| {
                sidereon_tec_samples_result_get_message(result, out, len, w, r)
            });
            assert_eq!(
                owned,
                "sidereon_ionex_from_tec_grid_samples_result: IONEX map epochs must be strictly \
                 increasing"
            );
            sidereon_tec_samples_result_free(result);

            // A successful build transfers both, and the result holds no text.
            let fixture = GridFixture::new();
            let samples = fixture.samples(false, false, ptr::null());
            assert_eq!(
                sidereon_ionex_from_tec_grid_samples_result(&samples, &mut ionex, &mut result),
                SidereonStatus::Ok
            );
            assert!(!ionex.is_null());
            let outcome = samples_outcome(result);
            assert!(outcome.is_ok);
            assert_eq!(outcome.error.kind, SidereonTecSamplesErrorKind::None);
            assert!(copy_text(|out, len, w, r| {
                sidereon_tec_samples_result_get_message(result, out, len, w, r)
            })
            .is_empty());
            sidereon_tec_samples_result_free(result);
            sidereon_ionex_free(ionex);

            // A null partner output still clears the other one.
            let mut stale_result = ptr::NonNull::<SidereonTecSamplesResult>::dangling().as_ptr();
            assert_eq!(
                sidereon_ionex_from_tec_grid_samples_result(
                    &samples,
                    ptr::null_mut(),
                    &mut stale_result
                ),
                SidereonStatus::NullPointer
            );
            assert!(stale_result.is_null());
            sidereon_tec_samples_result_free(ptr::null_mut());
        }
    }

    #[test]
    fn a_whole_second_past_two_to_the_fifty_three_keeps_every_second() {
        unsafe {
            // 2^53 + 1 is the first whole second a double cannot hold; the
            // integer epoch field states it and every output keeps it.
            const FAR: i64 = 9_007_199_254_740_993;
            let epochs_whole = [FAR, FAR + 3600];
            let fixture = GridFixture::new();
            let mut samples = fixture.samples(false, false, ptr::null());
            samples.map_epochs_j2000_s = ptr::null();
            samples.map_epochs_j2000_whole_s = epochs_whole.as_ptr();
            let ionex = build_product(&samples);

            let mut axis = [0i64; 2];
            let mut written = 0usize;
            let mut required = 0usize;
            assert_eq!(
                sidereon_ionex_map_epochs_j2000_s(
                    ionex,
                    axis.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(axis, epochs_whole);

            // The double view is the integer converted once: 2^53 + 1 rounds
            // to the even neighbour 2^53, which is why the integer view exists.
            let mut doubles = [0.0f64; 2];
            assert_eq!(
                sidereon_ionex_tec_grid_samples_epochs_j2000_s(
                    ionex,
                    doubles.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(doubles, [FAR as f64, (FAR + 3600) as f64]);
            assert_eq!(doubles[0], 9_007_199_254_740_992.0);

            let mut nodes = vec![
                SidereonTecSample {
                    time_scale: 0,
                    epoch_j2000_s: 0.0,
                    has_epoch_j2000_whole_s: false,
                    epoch_j2000_whole_s: 0,
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    has_vtec_tecu: false,
                    vtec_tecu: 0.0,
                    has_rms_tecu: false,
                    rms_tecu: 0.0,
                    has_height_offset_km: false,
                    height_offset_km: 0.0,
                };
                8
            ];
            assert_eq!(
                sidereon_ionex_tec_samples(
                    ionex,
                    nodes.as_mut_ptr(),
                    8,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert!(nodes.iter().all(|node| node.has_epoch_j2000_whole_s));
            assert_eq!(nodes[0].epoch_j2000_whole_s, FAR);
            assert_eq!(nodes[7].epoch_j2000_whole_s, FAR + 3600);

            // The samples read back as the same product, second for second.
            let mut rebuilt: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_from_tec_samples(nodes.as_ptr(), 8, 450.0, 6371.0, -1, &mut rebuilt),
                SidereonStatus::Ok
            );
            let mut rebuilt_axis = [0i64; 2];
            assert_eq!(
                sidereon_ionex_map_epochs_j2000_s(
                    rebuilt,
                    rebuilt_axis.as_mut_ptr(),
                    2,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rebuilt_axis, epochs_whole);

            sidereon_ionex_free(rebuilt);
            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn every_writable_output_is_cleared_before_any_is_validated() {
        unsafe {
            let fixture = GridFixture::new();
            let ionex = build_product(&fixture.samples(false, false, ptr::null()));

            // A null evaluation output still clears the error detail.
            let mut error = SidereonIonexSlantError {
                kind: SidereonIonexSlantErrorKind::Unknown,
                ..no_ionex_slant_error()
            };
            assert_eq!(
                sidereon_ionex_slant_delay_with_policy(
                    ionex,
                    0.0,
                    0.0,
                    0.0,
                    85.0,
                    FIXTURE_FIRST_EPOCH_J2000_S,
                    1_575_420_000.0,
                    sidereon_ionex_slant_policy_default(),
                    ptr::null_mut(),
                    &mut error,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(error.kind, SidereonIonexSlantErrorKind::None);

            // Each has-count pair clears the writable half of the pair.
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_get_header(ionex, &mut header),
                SidereonStatus::Ok
            );
            type CountGetter = unsafe extern "C" fn(
                *const SidereonIonexHeader,
                *mut u32,
                *mut bool,
            ) -> SidereonStatus;
            let getters: [CountGetter; 3] = [
                sidereon_ionex_header_get_station_count,
                sidereon_ionex_header_get_satellite_count,
                sidereon_ionex_header_get_maps_in_file,
            ];
            for getter in getters {
                let mut has = true;
                assert_eq!(
                    getter(header, ptr::null_mut(), &mut has),
                    SidereonStatus::NullPointer
                );
                assert!(!has);
                let mut count = 77u32;
                assert_eq!(
                    getter(header, &mut count, ptr::null_mut()),
                    SidereonStatus::NullPointer
                );
                assert_eq!(count, 0);
            }
            let mut function = SidereonIonexMappingFunctionKind::Other;
            assert_eq!(
                sidereon_ionex_header_get_mapping_declaration(
                    header,
                    ptr::null_mut(),
                    &mut function
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(function, SidereonIonexMappingFunctionKind::NoMapping);
            let mut declaration = SidereonIonexMappingDeclarationKind::Declared;
            assert_eq!(
                sidereon_ionex_header_get_mapping_declaration(
                    header,
                    &mut declaration,
                    ptr::null_mut()
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(declaration, SidereonIonexMappingDeclarationKind::Absent);
            sidereon_ionex_header_free(header);

            // A null written count still clears the required count.
            let mut required = 99usize;
            assert_eq!(
                sidereon_ionex_lat_nodes_deg(
                    ionex,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut required
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(required, 0);
            sidereon_ionex_free(ionex);

            // The standalone grid routes.
            let grid_fixture = StandaloneGrid::new();
            let mut grid_error = SidereonTecGridError {
                kind: SidereonTecGridErrorKind::Unknown,
                ..no_tec_grid_error()
            };
            assert_eq!(
                sidereon_tec_grid_new(
                    grid_fixture.epochs_ns.as_ptr(),
                    2,
                    grid_fixture.latitudes_deg.as_ptr(),
                    2,
                    grid_fixture.longitudes_deg.as_ptr(),
                    2,
                    grid_fixture.values.as_ptr(),
                    ptr::null(),
                    8,
                    ptr::null_mut(),
                    &mut grid_error,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(grid_error.kind, SidereonTecGridErrorKind::None);

            let grid = grid_fixture.build();
            let mut gap = SidereonIonexNodeGap {
                has_gap: true,
                ..empty_node_gap_c()
            };
            grid_error.kind = SidereonTecGridErrorKind::Unknown;
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    ptr::null_mut(),
                    &mut gap,
                    &mut grid_error,
                ),
                SidereonStatus::NullPointer
            );
            assert!(!gap.has_gap);
            assert_eq!(grid_error.kind, SidereonTecGridErrorKind::None);
            let mut vtec = 1.0;
            assert_eq!(
                sidereon_tec_grid_vtec_at_pierce_point(
                    grid,
                    250,
                    0.0,
                    30.0,
                    SidereonIonexMissingNodePolicy::Strict as u32,
                    &mut vtec,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert!(vtec.is_nan());

            let mut counts = [5usize; 3];
            assert_eq!(
                sidereon_tec_grid_dimensions(
                    grid,
                    &mut counts[0],
                    &mut counts[1],
                    &mut counts[2],
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(counts, [0, 0, 0]);
            sidereon_tec_grid_free(grid);
        }
    }

    #[test]
    fn skipped_records_counts_an_aux_data_block() {
        unsafe {
            let mut clean: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse(
                    VALID_IONEX_FIXTURE.as_ptr(),
                    VALID_IONEX_FIXTURE.len(),
                    &mut clean
                ),
                SidereonStatus::Ok
            );
            let mut clean_count = 9usize;
            assert_eq!(
                sidereon_ionex_skipped_records(clean, &mut clean_count),
                SidereonStatus::Ok
            );
            sidereon_ionex_free(clean);
            let mut count = 0usize;

            let header_end = VALID_IONEX_FIXTURE
                .find("END OF HEADER\n")
                .expect("fixture has a header")
                + "END OF HEADER\n".len();
            let aux = format!(
                "{:<60}START OF AUX DATA\n{:<60}PRN / BIAS / RMS\n{:<60}END OF AUX DATA\n",
                "", "G01  -1.234  0.567", ""
            );
            let text = format!(
                "{}{}{}",
                &VALID_IONEX_FIXTURE[..header_end],
                aux,
                &VALID_IONEX_FIXTURE[header_end..]
            );
            let mut with_aux: *mut SidereonIonex = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_parse(text.as_ptr(), text.len(), &mut with_aux),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_ionex_skipped_records(with_aux, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, clean_count + 1);
            sidereon_ionex_free(with_aux);

            count = 9;
            assert_eq!(
                sidereon_ionex_skipped_records(ptr::null(), &mut count),
                SidereonStatus::NullPointer
            );
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn exact_time_slant_keeps_fraction_scale_and_epoch_error() {
        unsafe {
            let mut fixture = GridFixture::new();
            fixture.tec_values = [10.0, 10.0, 10.0, 10.0, 20.0, 20.0, 20.0, 20.0];
            let samples = fixture.samples(false, false, ptr::null());
            let ionex = build_product(&samples);
            let utc_query = Instant::from_nanos(
                TimeScale::Utc,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 1_800) * 1_000_000_000 + 500_000_000,
            );
            let gpst_query = Instant::from_nanos(
                TimeScale::Gpst,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 1_818) * 1_000_000_000 + 500_000_000,
            );
            let utc_epoch = instant_to_clock_epoch(&utc_query);
            let gpst_epoch = instant_to_clock_epoch(&gpst_query);
            let mut delay = f64::NAN;
            let mut epoch_error = no_ionex_epoch_error();
            let status = sidereon_ionex_slant_delay_at_instant(
                ionex,
                0.0,
                0.0,
                0.0,
                90.0,
                &utc_epoch,
                1_575_420_000.0,
                &mut delay,
                &mut epoch_error,
            );
            assert_eq!(status, SidereonStatus::Ok);
            let fraction = 1_800.5 / 3_600.0;
            let expected_vtec = (1.0 - fraction) * 10.0 + fraction * 20.0;
            let expected_delay =
                (40.3e16 / (1_575_420_000.0_f64 * 1_575_420_000.0)) * expected_vtec;
            assert_eq!(delay.to_bits(), expected_delay.to_bits());
            assert_eq!(epoch_error.kind, SidereonIonexEpochErrorKind::None);

            let mut gpst_delay = f64::NAN;
            assert_eq!(
                sidereon_ionex_slant_delay_at_instant(
                    ionex,
                    0.0,
                    0.0,
                    0.0,
                    90.0,
                    &gpst_epoch,
                    1_575_420_000.0,
                    &mut gpst_delay,
                    ptr::null_mut(),
                ),
                SidereonStatus::Ok
            );
            assert_eq!(delay.to_bits(), gpst_delay.to_bits());

            let requests = [
                SidereonIonexInstantSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 90.0,
                    epoch: utc_epoch,
                    frequency_hz: 1_575_420_000.0,
                },
                SidereonIonexInstantSlantRequest {
                    epoch: instant_to_clock_epoch(&Instant::from_nanos(
                        TimeScale::Tdb,
                        FIXTURE_FIRST_EPOCH_J2000_S as i128 * 1_000_000_000,
                    )),
                    ..SidereonIonexInstantSlantRequest {
                        lat_deg: 0.0,
                        lon_deg: 0.0,
                        azimuth_deg: 0.0,
                        elevation_deg: 90.0,
                        epoch: utc_epoch,
                        frequency_hz: 1_575_420_000.0,
                    }
                },
                SidereonIonexInstantSlantRequest {
                    epoch: instant_to_clock_epoch(&Instant::from_nanos(
                        TimeScale::Utc,
                        (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 3_600) * 1_000_000_000 + 500_000_000,
                    )),
                    ..SidereonIonexInstantSlantRequest {
                        lat_deg: 0.0,
                        lon_deg: 0.0,
                        azimuth_deg: 0.0,
                        elevation_deg: 90.0,
                        epoch: utc_epoch,
                        frequency_hz: 1_575_420_000.0,
                    }
                },
            ];
            let mut rows = [SidereonIonexInstantSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
                epoch_error: no_ionex_epoch_error(),
            }; 3];
            assert_eq!(
                sidereon_ionex_slant_delay_results_at_instants(
                    ionex,
                    requests.as_ptr(),
                    requests.len(),
                    sidereon_ionex_slant_policy_default(),
                    rows.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            assert!(rows[0].is_ok);
            assert_eq!(rows[0].status, SidereonStatus::Ok);
            assert!(!rows[1].is_ok);
            assert_eq!(rows[1].status, SidereonStatus::Solve);
            assert_eq!(
                rows[1].epoch_error.kind,
                SidereonIonexEpochErrorKind::NoExactUtcOffset
            );
            assert_eq!(rows[1].error.kind, SidereonIonexSlantErrorKind::None);
            assert!(!rows[2].is_ok);
            assert_eq!(
                rows[2].error.kind,
                SidereonIonexSlantErrorKind::OutOfCoverage
            );
            assert_eq!(
                rows[2].error.coverage_error,
                SidereonIonexCoverageErrorKind::EpochAfterLastMap
            );
            assert_eq!(rows[2].epoch_error.kind, SidereonIonexEpochErrorKind::None);

            let mut after_last_evaluation = refused_ionex_slant_delay_evaluation();
            let mut after_last_error = no_ionex_slant_error();
            assert_eq!(
                sidereon_ionex_slant_delay_at_instant_with_policy(
                    ionex,
                    0.0,
                    0.0,
                    0.0,
                    90.0,
                    &requests[2].epoch,
                    1_575_420_000.0,
                    sidereon_ionex_slant_policy_default(),
                    &mut after_last_evaluation,
                    &mut after_last_error,
                    ptr::null_mut(),
                ),
                SidereonStatus::Solve
            );
            assert_eq!(
                after_last_error.kind,
                SidereonIonexSlantErrorKind::OutOfCoverage
            );
            assert_eq!(
                after_last_error.coverage_error,
                SidereonIonexCoverageErrorKind::EpochAfterLastMap
            );

            let product_set = [ionex as *const SidereonIonex];
            let mut selected = ptr::null_mut();
            let mut metadata = empty_staleness_metadata();
            assert_eq!(
                sidereon_select_ionex_at_instant(
                    product_set.as_ptr(),
                    product_set.len(),
                    &gpst_epoch,
                    sidereon_staleness_policy_default(),
                    &mut selected,
                    &mut metadata,
                    &mut epoch_error,
                ),
                SidereonSelectionStatus::Ok
            );
            assert!(!selected.is_null());
            sidereon_ionex_free(selected);

            let tdb_selection = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Tdb,
                FIXTURE_FIRST_EPOCH_J2000_S as i128 * 1_000_000_000,
            ));
            assert_eq!(
                sidereon_select_ionex_at_instant(
                    product_set.as_ptr(),
                    product_set.len(),
                    &tdb_selection,
                    sidereon_staleness_policy_default(),
                    &mut selected,
                    &mut metadata,
                    &mut epoch_error,
                ),
                SidereonSelectionStatus::IonexEpoch
            );
            assert_eq!(
                epoch_error.kind,
                SidereonIonexEpochErrorKind::NoExactUtcOffset
            );
            assert!(selected.is_null());

            let start = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Utc,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 1_800) * 1_000_000_000 + 250_000_000,
            ));
            let end = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Gpst,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 1_818) * 1_000_000_000 + 750_000_000,
            ));
            assert_eq!(
                sidereon_select_ionex_over_instant_range(
                    product_set.as_ptr(),
                    product_set.len(),
                    &start,
                    &end,
                    sidereon_staleness_policy_default(),
                    &mut selected,
                    &mut metadata,
                    &mut epoch_error,
                ),
                SidereonSelectionStatus::Ok
            );
            assert_eq!(
                metadata.requested_epoch_j2000_s,
                FIXTURE_FIRST_EPOCH_J2000_S as f64 + 1_800.75
            );
            sidereon_ionex_free(selected);

            let after_last_map = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Utc,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 3_600) * 1_000_000_000 + 500_000_000,
            ));
            let strict_zero_staleness = SidereonStalenessPolicy {
                max_staleness_s: 0.0,
            };
            assert_eq!(
                sidereon_select_ionex_over_instant_range(
                    product_set.as_ptr(),
                    product_set.len(),
                    &after_last_map,
                    &after_last_map,
                    strict_zero_staleness,
                    &mut selected,
                    &mut metadata,
                    &mut epoch_error,
                ),
                SidereonSelectionStatus::BeyondStalenessCap
            );
            assert!(selected.is_null());
            assert_eq!(epoch_error.kind, SidereonIonexEpochErrorKind::None);

            epoch_error = SidereonIonexEpochError {
                kind: SidereonIonexEpochErrorKind::Unknown,
                ..no_ionex_epoch_error()
            };
            assert_eq!(
                sidereon_select_ionex_over_instant_range(
                    product_set.as_ptr(),
                    product_set.len(),
                    ptr::null(),
                    &utc_epoch,
                    sidereon_staleness_policy_default(),
                    &mut selected,
                    ptr::null_mut(),
                    &mut epoch_error,
                ),
                SidereonSelectionStatus::NullPointer
            );
            assert!(selected.is_null());
            assert_eq!(epoch_error.kind, SidereonIonexEpochErrorKind::None);

            let tdb = Instant::from_nanos(
                TimeScale::Tdb,
                FIXTURE_FIRST_EPOCH_J2000_S as i128 * 1_000_000_000,
            );
            let tdb_epoch = instant_to_clock_epoch(&tdb);
            epoch_error = SidereonIonexEpochError {
                kind: SidereonIonexEpochErrorKind::Unknown,
                ..no_ionex_epoch_error()
            };
            assert_eq!(
                sidereon_ionex_slant_delay_at_instant(
                    ionex,
                    0.0,
                    0.0,
                    0.0,
                    90.0,
                    &tdb_epoch,
                    1_575_420_000.0,
                    ptr::null_mut(),
                    &mut epoch_error,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(epoch_error.kind, SidereonIonexEpochErrorKind::None);

            assert_eq!(
                sidereon_ionex_slant_delay_at_instant(
                    ionex,
                    0.0,
                    0.0,
                    0.0,
                    90.0,
                    &tdb_epoch,
                    1_575_420_000.0,
                    &mut delay,
                    &mut epoch_error,
                ),
                SidereonStatus::Solve
            );
            assert_eq!(
                epoch_error.kind,
                SidereonIonexEpochErrorKind::NoExactUtcOffset
            );
            assert_eq!(epoch_error.scale, SidereonTimeScale::Tdb as u32);
            sidereon_ionex_free(ionex);
        }
    }

    #[test]
    fn exact_time_owned_rows_retain_epoch_message_and_mapping_code() {
        unsafe {
            let fixture = GridFixture::new();
            let mut header: *mut SidereonIonexHeader = ptr::null_mut();
            assert_eq!(sidereon_ionex_header_new(&mut header), SidereonStatus::Ok);
            let custom = b"SLAB3D_EXACT";
            assert_eq!(
                sidereon_ionex_header_set_mapping_function(
                    header,
                    SidereonIonexMappingFunctionKind::Other as u32,
                    custom.as_ptr(),
                    custom.len()
                ),
                SidereonStatus::Ok
            );
            let samples = fixture.samples(false, false, header);
            let ionex = build_product(&samples);
            let utc_fraction = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Utc,
                (FIXTURE_FIRST_EPOCH_J2000_S as i128 + 1_800) * 1_000_000_000 + 500_000_000,
            ));
            let tdb = instant_to_clock_epoch(&Instant::from_nanos(
                TimeScale::Tdb,
                FIXTURE_FIRST_EPOCH_J2000_S as i128 * 1_000_000_000,
            ));
            let requests = [
                SidereonIonexInstantSlantRequest {
                    lat_deg: 0.0,
                    lon_deg: 0.0,
                    azimuth_deg: 0.0,
                    elevation_deg: 90.0,
                    epoch: utc_fraction,
                    frequency_hz: 1_575_420_000.0,
                },
                SidereonIonexInstantSlantRequest {
                    epoch: tdb,
                    ..SidereonIonexInstantSlantRequest {
                        lat_deg: 0.0,
                        lon_deg: 0.0,
                        azimuth_deg: 0.0,
                        elevation_deg: 90.0,
                        epoch: utc_fraction,
                        frequency_hz: 1_575_420_000.0,
                    }
                },
                SidereonIonexInstantSlantRequest {
                    frequency_hz: 0.0,
                    ..SidereonIonexInstantSlantRequest {
                        lat_deg: 0.0,
                        lon_deg: 0.0,
                        azimuth_deg: 0.0,
                        elevation_deg: 90.0,
                        epoch: utc_fraction,
                        frequency_hz: 1_575_420_000.0,
                    }
                },
            ];
            let mut list: *mut SidereonIonexInstantSlantResultList = ptr::null_mut();
            assert_eq!(
                sidereon_ionex_slant_delay_results_at_instants_owned(
                    ionex,
                    requests.as_ptr(),
                    requests.len(),
                    sidereon_ionex_slant_policy_default(),
                    &mut list,
                ),
                SidereonStatus::Ok
            );
            let mut count = 0usize;
            assert_eq!(
                sidereon_ionex_instant_slant_result_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 3);
            let mut row = SidereonIonexInstantSlantRowResult {
                is_ok: false,
                status: SidereonStatus::Panic,
                evaluation: refused_ionex_slant_delay_evaluation(),
                error: no_ionex_slant_error(),
                epoch_error: no_ionex_epoch_error(),
            };
            assert_eq!(
                sidereon_ionex_instant_slant_result_get_row(list, 0, &mut row),
                SidereonStatus::Ok
            );
            assert!(row.is_ok);
            assert!(row.evaluation.status.has_assumed_mapping);
            assert_eq!(
                row.evaluation.status.assumed_mapping,
                SidereonIonexAssumedMappingKind::Other
            );
            assert_eq!(
                sidereon_ionex_instant_slant_result_get_row(list, 1, &mut row),
                SidereonStatus::Ok
            );
            assert!(!row.is_ok);
            assert_eq!(
                row.epoch_error.kind,
                SidereonIonexEpochErrorKind::NoExactUtcOffset
            );
            assert_eq!(row.error.kind, SidereonIonexSlantErrorKind::None);
            assert_eq!(
                sidereon_ionex_instant_slant_result_get_row(list, 2, &mut row),
                SidereonStatus::Ok
            );
            assert_eq!(row.status, SidereonStatus::InvalidArgument);
            assert_eq!(row.error.kind, SidereonIonexSlantErrorKind::InvalidInput);

            sidereon_ionex_free(ionex);
            sidereon_ionex_header_free(header);
            let epoch_message = copy_text(|out, len, written, required| {
                sidereon_ionex_instant_slant_result_get_message(
                    list, 1, out, len, written, required,
                )
            });
            assert!(
                epoch_message.contains("TDB"),
                "unexpected text {epoch_message:?}"
            );
            let input_message = copy_text(|out, len, written, required| {
                sidereon_ionex_instant_slant_result_get_message(
                    list, 2, out, len, written, required,
                )
            });
            assert!(
                input_message.contains("frequency_hz"),
                "unexpected text {input_message:?}"
            );
            let mapping_code = copy_text(|out, len, written, required| {
                sidereon_ionex_instant_slant_result_get_mapping_code(
                    list, 0, out, len, written, required,
                )
            });
            assert_eq!(mapping_code, "SLAB3D_EXACT");
            sidereon_ionex_instant_slant_result_list_free(list);
            sidereon_ionex_instant_slant_result_list_free(ptr::null_mut());
        }
    }

    #[test]
    fn ionex_epoch_error_c_mapping_covers_all_core_causes() {
        use sidereon_core::atmosphere::ionosphere::IonexEpochError as E;
        let cases = [
            (
                E::NotWholeSecond {
                    scale: TimeScale::Utc,
                },
                SidereonIonexEpochErrorKind::NotWholeSecond,
                SidereonTimeScale::Utc as u32,
                false,
                0,
            ),
            (
                E::FractionalUtcSecond {
                    scale: TimeScale::Tai,
                },
                SidereonIonexEpochErrorKind::FractionalUtcSecond,
                SidereonTimeScale::Tai as u32,
                false,
                0,
            ),
            (
                E::NoExactUtcOffset {
                    scale: TimeScale::Tdb,
                },
                SidereonIonexEpochErrorKind::NoExactUtcOffset,
                SidereonTimeScale::Tdb as u32,
                false,
                0,
            ),
            (
                E::InsertedLeapSecond {
                    scale: TimeScale::Utc,
                },
                SidereonIonexEpochErrorKind::InsertedLeapSecond,
                SidereonTimeScale::Utc as u32,
                false,
                0,
            ),
            (
                E::BeforeIntegerLeapSeconds {
                    scale: TimeScale::Gpst,
                },
                SidereonIonexEpochErrorKind::BeforeIntegerLeapSeconds,
                SidereonTimeScale::Gpst as u32,
                false,
                0,
            ),
            (
                E::OutOfRange {
                    scale: TimeScale::Utc,
                },
                SidereonIonexEpochErrorKind::OutOfRange,
                SidereonTimeScale::Utc as u32,
                false,
                0,
            ),
            (
                E::YearOutOfField { utc_j2000_s: 123 },
                SidereonIonexEpochErrorKind::YearOutOfField,
                SidereonTimeScale::Utc as u32,
                true,
                123,
            ),
        ];
        for (error, kind, scale, has_utc, utc_s) in cases {
            let actual = ionex_epoch_error_to_c(&error);
            assert_eq!(actual.kind, kind);
            assert_eq!(actual.scale, scale);
            assert_eq!(actual.has_utc_j2000_s, has_utc);
            assert_eq!(actual.utc_j2000_s, utc_s);
        }
    }
}
