use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};

fn error_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

fn sgp4_input_kind_name(kind: sidereon_core::astro::sgp4::Sgp4InputErrorKind) -> &'static str {
    use sidereon_core::astro::sgp4::Sgp4InputErrorKind as K;
    match kind {
        K::NonFinite => "non_finite",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::Missing => "missing",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

pub(crate) fn sgp4_error_value(error: &sidereon_core::astro::sgp4::Error) -> serde_json::Value {
    use sidereon_core::astro::sgp4::Error as E;
    match error {
        E::InvalidInput { field, kind } => error_node(
            "invalid_input",
            serde_json::json!({
                "field": field,
                "kind": sgp4_input_kind_name(*kind),
            }),
        ),
        E::NonFiniteOutput { field } => error_node(
            "non_finite_output",
            serde_json::json!({
                "field": field,
            }),
        ),
        E::InvalidTle(message) => error_node(
            "invalid_tle",
            serde_json::json!({
                "message": message,
            }),
        ),
        E::Sgp4 { code } => error_node(
            "sgp4",
            serde_json::json!({
                "code": code,
            }),
        ),
        E::ResonanceStepBudget { budget } => error_node(
            "resonance_step_budget",
            serde_json::json!({
                "budget": budget,
            }),
        ),
    }
}

pub(crate) fn look_angle_error_value(
    error: &sidereon_core::astro::passes::LookAngleError,
) -> serde_json::Value {
    use sidereon_core::astro::passes::LookAngleError as E;
    match error {
        E::InvalidInput { field, reason } => error_node(
            "invalid_input",
            serde_json::json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::Init(inner) => error_node(
            "init",
            serde_json::json!({
                "cause": sgp4_error_value(inner),
            }),
        ),
        E::Propagate(inner) => error_node(
            "propagate",
            serde_json::json!({
                "cause": sgp4_error_value(inner),
            }),
        ),
        E::FrameTransform(inner) => error_node(
            "frame_transform",
            serde_json::json!({
                "cause": crate::orbit_fit::frame_transform_error_value(inner),
            }),
        ),
    }
}

pub const TLE_FIELD_C_BYTES: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSgp4ErrorKind {
    None = 0,
    InvalidInput = 1,
    NonFiniteOutput = 2,
    InvalidTle = 3,
    Engine = 4,
    ResonanceStepBudget = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonSgp4ErrorInfo {
    pub kind: SidereonSgp4ErrorKind,
    pub has_code: bool,
    pub code: i32,
    pub has_budget: bool,
    pub budget: u64,
}

thread_local! {
    static LAST_SGP4_ERROR: std::cell::Cell<SidereonSgp4ErrorInfo> = const { std::cell::Cell::new(SidereonSgp4ErrorInfo { kind: SidereonSgp4ErrorKind::None, has_code: false, code: 0, has_budget: false, budget: 0 }) };
}

fn record_sgp4_error(error: &Sgp4Error) {
    let mut detail = SidereonSgp4ErrorInfo {
        kind: SidereonSgp4ErrorKind::None,
        has_code: false,
        code: 0,
        has_budget: false,
        budget: 0,
    };
    match error {
        Sgp4Error::InvalidInput { .. } => detail.kind = SidereonSgp4ErrorKind::InvalidInput,
        Sgp4Error::NonFiniteOutput { .. } => detail.kind = SidereonSgp4ErrorKind::NonFiniteOutput,
        Sgp4Error::InvalidTle(_) => detail.kind = SidereonSgp4ErrorKind::InvalidTle,
        Sgp4Error::Sgp4 { code } => {
            detail.kind = SidereonSgp4ErrorKind::Engine;
            detail.has_code = true;
            detail.code = *code;
        }
        Sgp4Error::ResonanceStepBudget { budget } => {
            detail.kind = SidereonSgp4ErrorKind::ResonanceStepBudget;
            detail.has_budget = true;
            detail.budget = *budget;
        }
    }
    LAST_SGP4_ERROR.with(|slot| slot.set(detail));
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_last_error_info(
    out_info: *mut SidereonSgp4ErrorInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_last_error_info",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_info,
                "sidereon_sgp4_last_error_info",
                "out_info"
            ));
            *out = LAST_SGP4_ERROR.with(std::cell::Cell::get);
            SidereonStatus::Ok
        },
    )
}

/// A parsed TLE and initialized SGP4 satellite. Opaque to C. Create with
/// sidereon_tle_load and release with sidereon_tle_free.
#[derive(Clone)]
pub struct SidereonTle {
    pub(crate) elements: TleElements,
    pub(crate) satellite: Satellite,
    pub(crate) checksum_warnings: Vec<ChecksumWarning>,
}

/// Stateful SGP4 decay latch. Opaque to C. Create with
/// sidereon_sgp4_decay_latch_new and release with
/// sidereon_sgp4_decay_latch_free.
pub struct SidereonSgp4DecayLatch {
    pub(crate) inner: DecayLatch,
}

/// A parsed multi-record CelesTrak/Space-Track TLE file. Opaque to C. Create
/// with sidereon_parse_tle_file and release with sidereon_tle_file_free.
pub struct SidereonTleFile {
    pub(crate) records: Vec<SidereonTleFileRecord>,
    pub(crate) rejected: Vec<sidereon::sgp4::RejectedTleRecord>,
}

/// A TEME state arc from TLE/SGP4 propagation. Opaque to C. Create with
/// sidereon_tle_propagate and release with sidereon_tle_propagation_free.
pub struct SidereonTlePropagation {
    pub(crate) inner: Vec<Prediction>,
}

/// Topocentric look-angle arc from a TLE. Opaque to C. Create with
/// sidereon_tle_look_angles and release with sidereon_look_angles_free.
pub struct SidereonLookAngles {
    pub(crate) inner: Vec<LookAngle>,
}

/// Constellation visibility snapshot at one instant. Opaque to C. Create with
/// sidereon_visible_from_satellites and release with sidereon_visible_list_free.
pub struct SidereonVisibleList {
    pub(crate) inner: Vec<VisibleSatellite>,
}

/// Batched TLE/SGP4 propagation result. Opaque to C. Create with
/// sidereon_propagate_tle_batch and release with
/// sidereon_tle_batch_propagation_free.
pub struct SidereonTleBatchPropagation {
    pub(crate) epoch_count: usize,
    pub(crate) inner: Vec<Vec<Prediction>>,
}

/// Batched topocentric look-angle result. Opaque to C. Create with
/// sidereon_tle_batch_look_angles and release with
/// sidereon_tle_batch_look_angles_free.
pub struct SidereonTleBatchLookAngles {
    pub(crate) epoch_count: usize,
    pub(crate) inner: Vec<Vec<LookAngle>>,
}

/// SGP4 operation mode selector. Pass these values as uint32_t opsmode
/// arguments.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTleOpsMode {
    /// AFSPC-compatible mode.
    Afspc = 0,
    /// Improved Vallado mode.
    Improved = 1,
}

/// Fixed-size null-terminated TLE line storage. Values returned by Sidereon are
/// always null-terminated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleLine {
    /// Null-terminated TLE line bytes.
    pub bytes: [c_char; 129],
}

/// Re-encoded TLE line pair.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleLines {
    /// TLE line 1.
    pub line1: SidereonTleLine,
    /// TLE line 2.
    pub line2: SidereonTleLine,
}

/// One TLE line pair for batch propagation inputs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTlePair {
    /// Null-terminated TLE line 1.
    pub line1: *const c_char,
    /// Null-terminated TLE line 2.
    pub line2: *const c_char,
}

/// TLE checksum-reading policy. Pass these values as uint32_t policy
/// arguments. Mirrors sidereon_core::astro::tle::TlePolicy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTlePolicy {
    /// Refuse a column-69 digit that disagrees with the checksum and a column
    /// 69 that is not a digit.
    Strict = 0,
    /// Read both and report each as a checksum warning, as Vallado's
    /// `twoline2rv` reads.
    Lenient = 1,
}

/// What column 69 of a line held when it did not confirm the checksum.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTleChecksumWarningKind {
    /// A digit that differs from the computed checksum; `found` is the digit.
    Mismatch = 0,
    /// A character other than a digit; `found` is its byte.
    NotDigit = 1,
    /// The line ends before column 69, so it carries no checksum; `found` is 0.
    Missing = 2,
}

/// A line whose column 69 did not confirm its checksum.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleChecksumWarning {
    /// TLE line number, 1 or 2.
    pub line_number: u8,
    /// What column 69 held.
    pub kind: SidereonTleChecksumWarningKind,
    /// The digit (MISMATCH) or byte (NOT_DIGIT) found in column 69; 0 for
    /// MISSING.
    pub found: u8,
    /// Checksum recomputed from columns 1 through 68 (or as many as the line
    /// has).
    pub computed: u8,
}

/// Why a stretch of a TLE file did not become a satellite. Mirrors
/// sidereon_core::astro::sgp4::TleRecordIssue.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTleRecordIssue {
    /// A line 1 and line 2 whose element set the TLE grammar, the checksum
    /// policy, or SGP4 initialization refused.
    Invalid = 0,
    /// A line 1 with no line 2 after it.
    MissingLine2 = 1,
    /// A line 2 with no line 1 before it.
    OrphanLine2 = 2,
    /// A name line not followed by an element set.
    OrphanName = 3,
}

/// One rejected stretch of a TLE file.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleRejectedRecord {
    /// One-based line number of the first rejected line: the name line when
    /// the record had one, otherwise its line 1 or line 2.
    pub line_number: usize,
    /// Why the lines were rejected.
    pub issue: SidereonTleRecordIssue,
}

/// Parsed TLE element fields exposed as read-only metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleMetadata {
    /// Null-terminated NORAD catalog number.
    pub catalog_number: [c_char; 32],
    /// Null-terminated classification string.
    pub classification: [c_char; 32],
    /// Null-terminated international designator.
    pub international_designator: [c_char; 32],
    /// Four-digit epoch year.
    pub epoch_year: i32,
    /// Fractional day-of-year of the epoch.
    pub epoch_day_of_year: f64,
    /// Inclination in degrees.
    pub inclination_deg: f64,
    /// Right ascension of the ascending node in degrees.
    pub raan_deg: f64,
    /// Orbital eccentricity.
    pub eccentricity: f64,
    /// Argument of perigee in degrees.
    pub arg_perigee_deg: f64,
    /// Mean anomaly at epoch in degrees.
    pub mean_anomaly_deg: f64,
    /// Mean motion in revolutions per day.
    pub mean_motion_rev_per_day: f64,
    /// First derivative of mean motion in revolutions per day squared.
    pub mean_motion_dot: f64,
    /// Second derivative of mean motion in revolutions per day cubed.
    pub mean_motion_double_dot: f64,
    /// B* drag term in TLE convention.
    pub bstar: f64,
    /// Whether line 1 states an ephemeris type (the field may be blank).
    pub has_ephemeris_type: bool,
    /// Ephemeris type from line 1, zero when blank.
    pub ephemeris_type: i32,
    /// Whether line 1 states an element set number.
    pub has_elset_number: bool,
    /// Element set number from line 1, zero when blank.
    pub elset_number: i32,
    /// Whether line 2 states a revolution number.
    pub has_rev_number: bool,
    /// Revolution number at epoch, zero when blank.
    pub rev_number: i32,
}

/// WGS84 ground station with latitude/longitude in degrees and altitude in
/// meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGroundStation {
    /// Geodetic latitude in degrees.
    pub latitude_deg: f64,
    /// Geodetic longitude in degrees.
    pub longitude_deg: f64,
    /// Altitude above WGS84 in meters.
    pub altitude_m: f64,
}

/// One TEME Cartesian state from SGP4.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTemeState {
    /// TEME position in kilometers.
    pub position_km: [f64; 3],
    /// TEME velocity in kilometers per second.
    pub velocity_km_s: [f64; 3],
}

/// One topocentric look angle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonLookAngle {
    /// Azimuth in degrees clockwise from north.
    pub azimuth_deg: f64,
    /// Elevation in degrees above the horizon.
    pub elevation_deg: f64,
    /// Slant range in kilometers.
    pub range_km: f64,
}

/// Dense pass-finder options. Initialize with
/// sidereon_pass_finder_options_init before overriding fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPassFinderOptions {
    /// Elevation mask in degrees.
    pub elevation_mask_deg: f64,
    /// Dense sampling step in seconds.
    pub step_seconds: f64,
    /// Bisection time tolerance in seconds.
    pub time_tolerance_s: f64,
}

/// One pass over a ground station.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatellitePass {
    /// Acquisition of signal, UTC unix microseconds.
    pub aos_unix_us: i64,
    /// Loss of signal, UTC unix microseconds.
    pub los_unix_us: i64,
    /// Culmination time, UTC unix microseconds.
    pub culmination_unix_us: i64,
    /// Elevation at culmination in degrees.
    pub max_elevation_deg: f64,
    /// Pass duration in seconds.
    pub duration_s: f64,
}

/// One satellite visible above the elevation mask at a single instant.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVisibleSatellite {
    /// Null-terminated caller-supplied satellite id (the matching `ids[i]` passed
    /// to sidereon_visible_from_satellites). That input is bounded to at most
    /// MAX_VISIBLE_ID_BYTES (64) bytes, so it always fits this buffer without
    /// truncation. The buffer length is VISIBLE_ID_C_BYTES (MAX_VISIBLE_ID_BYTES
    /// + 1).
    pub catalog_number: [c_char; 65],
    /// Azimuth in degrees clockwise from north.
    pub azimuth_deg: f64,
    /// Elevation in degrees above the horizon.
    pub elevation_deg: f64,
    /// Slant range in kilometers.
    pub range_km: f64,
    /// TEME position in kilometers.
    pub position_km: [f64; 3],
}

/// Parse a TLE line pair and initialize an SGP4 satellite. opsmode is one of
/// SidereonTleOpsMode_* encoded as uint32_t. On success writes a newly owned
/// handle to *out_tle. Release it with sidereon_tle_free.
///
/// Safety: line1 and line2 must be null-terminated within 128 bytes; out_tle
/// must point to storage for a SidereonTle*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_load(
    line1: *const c_char,
    line2: *const c_char,
    opsmode: u32,
    out_tle: *mut *mut SidereonTle,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_load", SidereonStatus::Panic, || {
        let out_tle = c_try!(require_out(out_tle, "sidereon_tle_load", "out_tle"));
        *out_tle = ptr::null_mut();
        let tle = c_try!(parse_tle_handle(
            "sidereon_tle_load",
            line1,
            line2,
            opsmode,
            TlePolicy::Strict
        ));
        write_boxed_handle(out_tle, tle);
        SidereonStatus::Ok
    })
}

/// Parse a TLE line pair under `policy`, a SidereonTlePolicy value, and
/// initialize an SGP4 satellite. SIDEREON_TLE_POLICY_LENIENT reads a
/// column-69 checksum that disagrees or is not a digit and reports it through
/// sidereon_tle_checksum_warnings; sidereon_tle_load refuses both. On success
/// writes a newly owned handle to *out_tle. Release it with sidereon_tle_free.
///
/// Safety: line1 and line2 must be null-terminated within 128 bytes; out_tle
/// must point to storage for a SidereonTle*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_load_with_policy(
    line1: *const c_char,
    line2: *const c_char,
    opsmode: u32,
    policy: u32,
    out_tle: *mut *mut SidereonTle,
) -> SidereonStatus {
    let fn_name = "sidereon_tle_load_with_policy";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out_tle = c_try!(require_out(out_tle, fn_name, "out_tle"));
        *out_tle = ptr::null_mut();
        let policy = c_try!(tle_policy_from_c(fn_name, policy));
        let tle = c_try!(parse_tle_handle(fn_name, line1, line2, opsmode, policy));
        write_boxed_handle(out_tle, tle);
        SidereonStatus::Ok
    })
}

/// Re-encode the parsed TLE elements as two null-terminated TLE lines.
///
/// Safety: tle must be a live handle; out_lines must point to a
/// SidereonTleLines.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_to_lines(
    tle: *const SidereonTle,
    out_lines: *mut SidereonTleLines,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_to_lines", SidereonStatus::Panic, || {
        let out_lines = c_try!(require_out(out_lines, "sidereon_tle_to_lines", "out_lines"));
        *out_lines = SidereonTleLines {
            line1: SidereonTleLine {
                bytes: [0; TLE_LINE_C_BYTES],
            },
            line2: SidereonTleLine {
                bytes: [0; TLE_LINE_C_BYTES],
            },
        };
        let tle = c_try!(require_ref(tle, "sidereon_tle_to_lines", "tle"));
        let (line1, line2) = c_try!(sidereon_tle::encode(&tle.elements).map_err(|err| {
            set_last_error(format!("sidereon_tle_to_lines: {err}"));
            SidereonStatus::InvalidArgument
        }));
        *out_lines = SidereonTleLines {
            line1: SidereonTleLine {
                bytes: fixed_c_chars(&line1),
            },
            line2: SidereonTleLine {
                bytes: fixed_c_chars(&line2),
            },
        };
        SidereonStatus::Ok
    })
}

/// Copy parsed TLE metadata into *out_metadata.
///
/// Safety: tle must be a live handle; out_metadata must point to a
/// SidereonTleMetadata.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_metadata(
    tle: *const SidereonTle,
    out_metadata: *mut SidereonTleMetadata,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_metadata", SidereonStatus::Panic, || {
        let out_metadata = c_try!(require_out(
            out_metadata,
            "sidereon_tle_metadata",
            "out_metadata"
        ));
        *out_metadata = SidereonTleMetadata {
            catalog_number: [0; TLE_FIELD_C_BYTES],
            classification: [0; TLE_FIELD_C_BYTES],
            international_designator: [0; TLE_FIELD_C_BYTES],
            epoch_year: 0,
            epoch_day_of_year: 0.0,
            inclination_deg: 0.0,
            raan_deg: 0.0,
            eccentricity: 0.0,
            arg_perigee_deg: 0.0,
            mean_anomaly_deg: 0.0,
            mean_motion_rev_per_day: 0.0,
            mean_motion_dot: 0.0,
            mean_motion_double_dot: 0.0,
            bstar: 0.0,
            has_ephemeris_type: false,
            ephemeris_type: 0,
            has_elset_number: false,
            elset_number: 0,
            has_rev_number: false,
            rev_number: 0,
        };
        let tle = c_try!(require_ref(tle, "sidereon_tle_metadata", "tle"));
        *out_metadata = tle_metadata_to_c(&tle.elements);
        SidereonStatus::Ok
    })
}

/// Copy advisory TLE checksum warnings. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: tle must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_checksum_warnings(
    tle: *const SidereonTle,
    out: *mut SidereonTleChecksumWarning,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_checksum_warnings",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tle_checksum_warnings",
                out_written,
                out_required
            ));
            let tle = c_try!(require_ref(tle, "sidereon_tle_checksum_warnings", "tle"));
            let warnings: Vec<SidereonTleChecksumWarning> = tle
                .checksum_warnings
                .iter()
                .map(checksum_warning_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_tle_checksum_warnings",
                "out",
                &warnings,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Parse a multi-record CelesTrak/Space-Track TLE file into N initialized
/// satellites under the strict checksum policy. text must point to text_len
/// readable UTF-8 bytes (the whole file); opsmode is one of
/// SidereonTleOpsMode_* encoded as uint32_t. Handles bare 2-line sets, 3-line
/// name+line1+line2 sets, and CelesTrak "0 NAME" name lines; CRLF endings,
/// blank lines, and surrounding whitespace are tolerated. Every element set
/// that reads is kept, and every other non-blank line is reported as a
/// rejected record (sidereon_tle_file_rejected) with its line and reason: an
/// element set the grammar, checksum policy or SGP4 initialization refused, a
/// line 1 without a line 2, a stray line 2, or a name line with no element
/// set. On success writes a newly owned handle to *out_file. Release it with
/// sidereon_tle_file_free.
///
/// Safety: text must point to text_len readable bytes or be NULL when text_len
/// is 0; out_file must point to storage for a SidereonTleFile*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_tle_file(
    text: *const u8,
    text_len: usize,
    opsmode: u32,
    out_file: *mut *mut SidereonTleFile,
) -> SidereonStatus {
    ffi_boundary("sidereon_parse_tle_file", SidereonStatus::Panic, || {
        parse_tle_file_body(
            "sidereon_parse_tle_file",
            text,
            text_len,
            opsmode,
            TlePolicy::Strict,
            out_file,
        )
    })
}

/// Parse a multi-record TLE file as sidereon_parse_tle_file does, under
/// `policy`, a SidereonTlePolicy value. Under SIDEREON_TLE_POLICY_LENIENT a
/// column-69 checksum that disagrees or is not a digit is read and reported
/// in the record's checksum warnings instead of rejecting the record.
///
/// Safety: as for sidereon_parse_tle_file.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_tle_file_with_policy(
    text: *const u8,
    text_len: usize,
    opsmode: u32,
    policy: u32,
    out_file: *mut *mut SidereonTleFile,
) -> SidereonStatus {
    let fn_name = "sidereon_parse_tle_file_with_policy";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        if !out_file.is_null() {
            *out_file = ptr::null_mut();
        }
        let policy = c_try!(tle_policy_from_c(fn_name, policy));
        parse_tle_file_body(fn_name, text, text_len, opsmode, policy, out_file)
    })
}

unsafe fn parse_tle_file_body(
    fn_name: &str,
    text: *const u8,
    text_len: usize,
    opsmode: u32,
    policy: TlePolicy,
    out_file: *mut *mut SidereonTleFile,
) -> SidereonStatus {
    let out_file = c_try!(require_out(out_file, fn_name, "out_file"));
    *out_file = ptr::null_mut();
    let bytes = c_try!(require_slice(text, text_len, fn_name, "text"));
    let text = match str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            set_last_error(format!("{fn_name}: text is not valid UTF-8"));
            return SidereonStatus::InvalidToken;
        }
    };
    let mode = c_try!(tle_ops_mode_from_c(fn_name, opsmode));
    let parsed = sidereon::sgp4::parse_tle_file_with_policy(text, mode, policy);
    let mut records = Vec::with_capacity(parsed.satellites.len());
    for named in parsed.satellites {
        records.push(c_try!(named_satellite_to_record(fn_name, named)));
    }
    write_boxed_handle(
        out_file,
        SidereonTleFile {
            records,
            rejected: parsed.rejected,
        },
    );
    SidereonStatus::Ok
}

/// Copy rejected record `index` of a TLE file into *out_record. Its name line
/// is read with sidereon_tle_file_rejected_name and, for an INVALID record,
/// the refusal with sidereon_tle_file_rejected_error.
///
/// Safety: file must be a live handle; out_record must point to a
/// SidereonTleRejectedRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_rejected(
    file: *const SidereonTleFile,
    index: usize,
    out_record: *mut SidereonTleRejectedRecord,
) -> SidereonStatus {
    let fn_name = "sidereon_tle_file_rejected";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_record, fn_name, "out_record"));
        *out = SidereonTleRejectedRecord {
            line_number: 0,
            issue: SidereonTleRecordIssue::Invalid,
        };
        let file = c_try!(require_ref(file, fn_name, "file"));
        let record = c_try!(tle_rejected_at(fn_name, file, index));
        *out = SidereonTleRejectedRecord {
            line_number: record.line_number,
            issue: match record.issue {
                sidereon::sgp4::TleRecordIssue::Invalid(_) => SidereonTleRecordIssue::Invalid,
                sidereon::sgp4::TleRecordIssue::MissingLine2 => {
                    SidereonTleRecordIssue::MissingLine2
                }
                sidereon::sgp4::TleRecordIssue::OrphanLine2 => SidereonTleRecordIssue::OrphanLine2,
                sidereon::sgp4::TleRecordIssue::OrphanName => SidereonTleRecordIssue::OrphanName,
            },
        };
        SidereonStatus::Ok
    })
}

/// Copy the name line of rejected record `index` (not null-terminated, empty
/// when the record had none) under the variable-length output contract.
///
/// Safety: file must be a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_rejected_name(
    file: *const SidereonTleFile,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tle_file_rejected_name";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let file = c_try!(require_ref(file, fn_name, "file"));
        let record = c_try!(tle_rejected_at(fn_name, file, index));
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            record.name.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the refusal of rejected record `index` (not null-terminated) under
/// the variable-length output contract. Empty for a record whose issue is not
/// SIDEREON_TLE_RECORD_ISSUE_INVALID.
///
/// Safety: as for sidereon_tle_file_rejected_name.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_rejected_error(
    file: *const SidereonTleFile,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tle_file_rejected_error";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let file = c_try!(require_ref(file, fn_name, "file"));
        let record = c_try!(tle_rejected_at(fn_name, file, index));
        let text = match &record.issue {
            sidereon::sgp4::TleRecordIssue::Invalid(err) => err.to_string(),
            _ => String::new(),
        };
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

/// Write the one-based line number of record `index`'s line 1 in the file
/// text to *out_line_number.
///
/// Safety: file must be a live handle; out_line_number must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_line_number(
    file: *const SidereonTleFile,
    index: usize,
    out_line_number: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_tle_file_line_number";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_line_number, fn_name, "out_line_number"));
        *out = 0;
        let file = c_try!(require_ref(file, fn_name, "file"));
        let Some(record) = file.records.get(index) else {
            set_last_error(format!(
                "{fn_name}: index {index} out of range ({} records)",
                file.records.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = record.line_number;
        SidereonStatus::Ok
    })
}

fn tle_rejected_at<'a>(
    fn_name: &str,
    file: &'a SidereonTleFile,
    index: usize,
) -> Result<&'a sidereon::sgp4::RejectedTleRecord, SidereonStatus> {
    file.rejected.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: index {index} out of range ({} rejected records)",
            file.rejected.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

/// Write the number of successfully parsed satellites in a TLE file to
/// *out_count.
///
/// Safety: file must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_count(
    file: *const SidereonTleFile,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_file_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_tle_file_count",
            "out_count"
        ));
        *out_count = 0;
        let file = c_try!(require_ref(file, "sidereon_tle_file_count", "file"));
        *out_count = file.records.len();
        SidereonStatus::Ok
    })
}

/// Write the number of rejected records (sidereon_tle_file_rejected) to
/// *out_skipped. An empty file (count == 0, skipped == 0) is thus
/// distinguishable from a fully corrupt one (count == 0, skipped > 0).
///
/// Safety: file must be a live handle; out_skipped must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_skipped(
    file: *const SidereonTleFile,
    out_skipped: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_file_skipped", SidereonStatus::Panic, || {
        let out_skipped = c_try!(require_out(
            out_skipped,
            "sidereon_tle_file_skipped",
            "out_skipped"
        ));
        *out_skipped = 0;
        let file = c_try!(require_ref(file, "sidereon_tle_file_skipped", "file"));
        *out_skipped = file.rejected.len();
        SidereonStatus::Ok
    })
}

/// Copy the name line for the record at index into buf as a null-terminated C
/// string. Writes the total number of bytes required (including the
/// terminator) to *out_required. Pass buf NULL with len 0 to query the size;
/// the name is empty for a bare 2-line set, for which out_required is 1. If len
/// is nonzero but smaller than out_required, returns InvalidArgument and leaves
/// buf null-terminated (empty) when len is positive.
///
/// Safety: file must be a live handle; buf must point to at least len writable
/// bytes or be NULL when len is 0; out_required must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_name(
    file: *const SidereonTleFile,
    index: usize,
    buf: *mut c_char,
    len: usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_file_name", SidereonStatus::Panic, || {
        let out_required = c_try!(require_out(
            out_required,
            "sidereon_tle_file_name",
            "out_required"
        ));
        *out_required = 0;
        if !buf.is_null() && len > 0 {
            *buf = 0;
        }
        let file = c_try!(require_ref(file, "sidereon_tle_file_name", "file"));
        let record = match file.records.get(index) {
            Some(record) => record,
            None => {
                set_last_error(format!(
                    "sidereon_tle_file_name: index {index} out of range ({} records)",
                    file.records.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        let name = record.name.as_bytes();
        let required = name.len() + 1;
        *out_required = required;
        if buf.is_null() {
            if len == 0 {
                return SidereonStatus::Ok;
            }
            set_last_error("sidereon_tle_file_name: null buf".to_string());
            return SidereonStatus::NullPointer;
        }
        if len < required {
            set_last_error(format!(
                "sidereon_tle_file_name: buf needs room for {required} bytes"
            ));
            return SidereonStatus::InvalidArgument;
        }
        ptr::copy_nonoverlapping(name.as_ptr().cast::<c_char>(), buf, name.len());
        *buf.add(name.len()) = 0;
        SidereonStatus::Ok
    })
}

/// Write a newly owned, independent copy of the TLE handle for the record at
/// index to *out_tle. The returned handle can be used with any sidereon_tle_*
/// entry point (propagation, look-angles, metadata) and outlives the file; it
/// must be released with sidereon_tle_free.
///
/// Safety: file must be a live handle; out_tle must point to storage for a
/// SidereonTle*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_satellite(
    file: *const SidereonTleFile,
    index: usize,
    out_tle: *mut *mut SidereonTle,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_file_satellite", SidereonStatus::Panic, || {
        let out_tle = c_try!(require_out(
            out_tle,
            "sidereon_tle_file_satellite",
            "out_tle"
        ));
        *out_tle = ptr::null_mut();
        let file = c_try!(require_ref(file, "sidereon_tle_file_satellite", "file"));
        let record = match file.records.get(index) {
            Some(record) => record,
            None => {
                set_last_error(format!(
                    "sidereon_tle_file_satellite: index {index} out of range ({} records)",
                    file.records.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_tle, record.tle.clone());
        SidereonStatus::Ok
    })
}

/// Propagate a TLE over UTC unix-microsecond epochs. On success writes a newly
/// owned arc handle to *out_propagation. Release it with
/// sidereon_tle_propagation_free.
///
/// Safety: tle must be a live handle; epochs_unix_us must point to epoch_count
/// int64_t values; out_propagation must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_propagate(
    tle: *const SidereonTle,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    out_propagation: *mut *mut SidereonTlePropagation,
) -> SidereonStatus {
    ffi_boundary("sidereon_tle_propagate", SidereonStatus::Panic, || {
        let out_propagation = c_try!(require_out(
            out_propagation,
            "sidereon_tle_propagate",
            "out_propagation"
        ));
        *out_propagation = ptr::null_mut();
        let tle = c_try!(require_ref(tle, "sidereon_tle_propagate", "tle"));
        let instants = c_try!(unix_instants_from_c(
            "sidereon_tle_propagate",
            epochs_unix_us,
            epoch_count,
        ));
        let inner = c_try!(propagate_teme_arc(&tle.satellite, &instants)
            .map_err(|err| map_sgp4_error("sidereon_tle_propagate", err)));
        write_boxed_handle(out_propagation, SidereonTlePropagation { inner });
        SidereonStatus::Ok
    })
}

/// Propagate one TLE at minutes since element epoch using a decay latch.
///
/// The latch records the first decay-like SGP4 failure and rejects later
/// requests at the same or later epoch without returning a raw post-decay state.
/// Use sidereon_tle_propagate for the existing stateless UTC arc path.
///
/// Safety: tle and latch must be live handles; out_state must point to a
/// SidereonTemeState.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_propagate_with_decay_latch(
    tle: *const SidereonTle,
    minutes_since_epoch: f64,
    latch: *mut SidereonSgp4DecayLatch,
    out_state: *mut SidereonTemeState,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_propagate_with_decay_latch",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_state,
                "sidereon_tle_propagate_with_decay_latch",
                "out_state"
            ));
            *out = SidereonTemeState {
                position_km: [0.0; 3],
                velocity_km_s: [0.0; 3],
            };
            let tle = c_try!(require_ref(
                tle,
                "sidereon_tle_propagate_with_decay_latch",
                "tle"
            ));
            let latch = c_try!(require_mut(
                latch,
                "sidereon_tle_propagate_with_decay_latch",
                "latch"
            ));
            match tle.satellite.propagate_with_decay_latch(
                MinutesSinceEpoch(minutes_since_epoch),
                &mut latch.inner,
            ) {
                Ok(prediction) => {
                    *out = prediction_to_c(&prediction);
                    SidereonStatus::Ok
                }
                Err(err) => map_decay_latched_error("sidereon_tle_propagate_with_decay_latch", err),
            }
        },
    )
}

/// Write the number of epochs in a TLE propagation arc to *out_count.
///
/// Safety: propagation must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_propagation_epoch_count(
    propagation: *const SidereonTlePropagation,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_propagation_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_tle_propagation_epoch_count",
                "out_count"
            ));
            *out_count = 0;
            let propagation = c_try!(require_ref(
                propagation,
                "sidereon_tle_propagation_epoch_count",
                "propagation"
            ));
            *out_count = propagation.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy TEME states from a TLE propagation arc. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: propagation must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_propagation_states(
    propagation: *const SidereonTlePropagation,
    out: *mut SidereonTemeState,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_propagation_states",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tle_propagation_states",
                out_written,
                out_required
            ));
            let propagation = c_try!(require_ref(
                propagation,
                "sidereon_tle_propagation_states",
                "propagation"
            ));
            let states: Vec<SidereonTemeState> =
                propagation.inner.iter().map(prediction_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_tle_propagation_states",
                "out",
                &states,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compute topocentric look angles from a TLE over UTC unix-microsecond epochs.
/// On success writes a newly owned handle to *out_look_angles. Release it with
/// sidereon_look_angles_free.
///
/// Safety: tle must be a live handle; station must point to a
/// SidereonGroundStation; epochs_unix_us must point to epoch_count values;
/// out_look_angles must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_look_angles(
    tle: *const SidereonTle,
    station: *const SidereonGroundStation,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    out_look_angles: *mut *mut SidereonLookAngles,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tle_look_angles", SidereonStatus::Panic, || {
        let out_look_angles = c_try!(require_out(
            out_look_angles,
            "sidereon_tle_look_angles",
            "out_look_angles"
        ));
        *out_look_angles = ptr::null_mut();
        let tle = c_try!(require_ref(tle, "sidereon_tle_look_angles", "tle"));
        let station = c_try!(require_ref(station, "sidereon_tle_look_angles", "station"));
        let instants = c_try!(unix_instants_from_c(
            "sidereon_tle_look_angles",
            epochs_unix_us,
            epoch_count,
        ));
        let inner =
            c_try!(
                look_angle_arc(&tle.satellite, ground_station_from_c(station), &instants)
                    .map_err(|err| map_look_angle_error("sidereon_tle_look_angles", err))
            );
        write_boxed_handle(out_look_angles, SidereonLookAngles { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of epochs in a look-angle arc to *out_count.
///
/// Safety: look_angles must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_look_angles_epoch_count(
    look_angles: *const SidereonLookAngles,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_look_angles_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_look_angles_epoch_count",
                "out_count"
            ));
            *out_count = 0;
            let look_angles = c_try!(require_ref(
                look_angles,
                "sidereon_look_angles_epoch_count",
                "look_angles"
            ));
            *out_count = look_angles.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy look-angle rows. Uses the variable-length output contract documented
/// at the top of the header.
///
/// Safety: look_angles must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_look_angles_values(
    look_angles: *const SidereonLookAngles,
    out: *mut SidereonLookAngle,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_look_angles_values", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_look_angles_values",
            out_written,
            out_required
        ));
        let look_angles = c_try!(require_ref(
            look_angles,
            "sidereon_look_angles_values",
            "look_angles"
        ));
        let values: Vec<SidereonLookAngle> =
            look_angles.inner.iter().map(look_angle_to_c).collect();
        c_try!(copy_prefix_to_c(
            "sidereon_look_angles_values",
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

/// Find dense passes over a ground station within [start_unix_us, end_unix_us).
/// options may be NULL for defaults. On success writes a newly owned pass-list
/// handle to *out_passes. Release it with sidereon_pass_list_free.
///
/// Safety: tle must be a live handle; station must point to a
/// SidereonGroundStation; out_passes must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_find_passes(
    tle: *const SidereonTle,
    station: *const SidereonGroundStation,
    start_unix_us: i64,
    end_unix_us: i64,
    options: *const SidereonPassFinderOptions,
    out_passes: *mut *mut SidereonPassList,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tle_find_passes", SidereonStatus::Panic, || {
        let out_passes = c_try!(require_out(
            out_passes,
            "sidereon_tle_find_passes",
            "out_passes"
        ));
        *out_passes = ptr::null_mut();
        if end_unix_us <= start_unix_us {
            set_last_error("sidereon_tle_find_passes: end_unix_us must be after start_unix_us");
            return SidereonStatus::InvalidArgument;
        }
        let tle = c_try!(require_ref(tle, "sidereon_tle_find_passes", "tle"));
        let station = c_try!(require_ref(station, "sidereon_tle_find_passes", "station"));
        let options = c_try!(pass_finder_options_from_c(
            "sidereon_tle_find_passes",
            options
        ));
        let inner = c_try!(find_passes_for_satellite(
            &tle.satellite,
            ground_station_from_c(station),
            UtcInstant::from_unix_microseconds(start_unix_us),
            UtcInstant::from_unix_microseconds(end_unix_us),
            options,
        )
        .map_err(|err| map_pass_error("sidereon_tle_find_passes", err)));
        write_boxed_handle(out_passes, SidereonPassList { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of passes in a pass-list handle to *out_count.
///
/// Safety: passes must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pass_list_count(
    passes: *const SidereonPassList,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_pass_list_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_pass_list_count",
            "out_count"
        ));
        *out_count = 0;
        let passes = c_try!(require_ref(passes, "sidereon_pass_list_count", "passes"));
        *out_count = passes.inner.len();
        SidereonStatus::Ok
    })
}

/// Copy pass rows. Uses the variable-length output contract documented at the
/// top of the header.
///
/// Safety: passes must be a live handle; out must point to at least len
/// writable entries or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pass_list_values(
    passes: *const SidereonPassList,
    out: *mut SidereonSatellitePass,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_pass_list_values", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_pass_list_values",
            out_written,
            out_required
        ));
        let passes = c_try!(require_ref(passes, "sidereon_pass_list_values", "passes"));
        let values: Vec<SidereonSatellitePass> =
            passes.inner.iter().map(satellite_pass_to_c).collect();
        c_try!(copy_prefix_to_c(
            "sidereon_pass_list_values",
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

/// Compute the per-epoch sub-satellite (ground-track) geodetic points for a TLE
/// over UTC unix-microsecond epochs. On success writes a newly owned arc handle
/// to *out_track. Release it with sidereon_ground_track_free.
///
/// Safety: tle must be a live handle; epochs_unix_us must point to epoch_count
/// int64_t values; out_track must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_ground_track(
    tle: *const SidereonTle,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    out_track: *mut *mut SidereonGroundTrack,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tle_ground_track", SidereonStatus::Panic, || {
        let out_track = c_try!(require_out(
            out_track,
            "sidereon_tle_ground_track",
            "out_track"
        ));
        *out_track = ptr::null_mut();
        let tle = c_try!(require_ref(tle, "sidereon_tle_ground_track", "tle"));
        let instants = c_try!(unix_instants_from_c(
            "sidereon_tle_ground_track",
            epochs_unix_us,
            epoch_count,
        ));
        let inner = c_try!(ground_track(&tle.satellite, &instants)
            .map_err(|err| map_look_angle_error("sidereon_tle_ground_track", err)));
        write_boxed_handle(out_track, SidereonGroundTrack { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of points in a ground-track arc to *out_count.
///
/// Safety: track must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ground_track_count(
    track: *const SidereonGroundTrack,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ground_track_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_ground_track_count",
            "out_count"
        ));
        *out_count = 0;
        let track = c_try!(require_ref(track, "sidereon_ground_track_count", "track"));
        *out_count = track.inner.len();
        SidereonStatus::Ok
    })
}

/// Copy ground-track sub-satellite geodetic points. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: track must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ground_track_values(
    track: *const SidereonGroundTrack,
    out: *mut SidereonGeodetic,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ground_track_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ground_track_values",
                out_written,
                out_required
            ));
            let track = c_try!(require_ref(track, "sidereon_ground_track_values", "track"));
            let values: Vec<SidereonGeodetic> = track.inner.iter().map(geodetic_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_ground_track_values",
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

/// Write the number of visible satellites in a visibility snapshot to
/// *out_count.
///
/// Safety: visible must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_visible_list_count(
    visible: *const SidereonVisibleList,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_visible_list_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_visible_list_count",
            "out_count"
        ));
        *out_count = 0;
        let visible = c_try!(require_ref(
            visible,
            "sidereon_visible_list_count",
            "visible"
        ));
        *out_count = visible.inner.len();
        SidereonStatus::Ok
    })
}

/// Copy visible-satellite rows. Uses the variable-length output contract
/// documented at the top of the header.
///
/// Safety: visible must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_visible_list_values(
    visible: *const SidereonVisibleList,
    out: *mut SidereonVisibleSatellite,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_visible_list_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_visible_list_values",
                out_written,
                out_required
            ));
            let visible = c_try!(require_ref(
                visible,
                "sidereon_visible_list_values",
                "visible"
            ));
            let values: Vec<SidereonVisibleSatellite> =
                visible.inner.iter().map(visible_satellite_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_visible_list_values",
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

/// Propagate a fleet of TLEs over a shared UTC unix-microsecond epoch grid.
/// opsmode is one of SidereonTleOpsMode_* encoded as uint32_t. When parallel is
/// true the engine's rayon batch path is used. On success writes a newly owned
/// batch handle to *out_batch. Release it with
/// sidereon_tle_batch_propagation_free.
///
/// Safety: tles must point to tle_count line pairs; epochs_unix_us must point
/// to epoch_count int64_t values; out_batch must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_propagate_tle_batch(
    tles: *const SidereonTlePair,
    tle_count: usize,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    opsmode: u32,
    parallel: bool,
    out_batch: *mut *mut SidereonTleBatchPropagation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_propagate_tle_batch",
        SidereonStatus::Panic,
        || {
            let out_batch = c_try!(require_out(
                out_batch,
                "sidereon_propagate_tle_batch",
                "out_batch"
            ));
            *out_batch = ptr::null_mut();
            let satellites = c_try!(tle_pair_satellites_from_c(
                "sidereon_propagate_tle_batch",
                tles,
                tle_count,
                opsmode,
            ));
            let instants = c_try!(unix_instants_from_c(
                "sidereon_propagate_tle_batch",
                epochs_unix_us,
                epoch_count,
            ));
            let epoch_count = instants.len();
            let results = if parallel {
                propagate_teme_batch_parallel(&satellites, &instants)
            } else {
                propagate_teme_batch_serial(&satellites, &instants)
            };
            let inner = c_try!(unwrap_prediction_batch(
                "sidereon_propagate_tle_batch",
                results
            ));
            write_boxed_handle(
                out_batch,
                SidereonTleBatchPropagation { epoch_count, inner },
            );
            SidereonStatus::Ok
        },
    )
}

/// Copy the shape of a batched propagation as satellite_count and epoch_count.
///
/// Safety: batch must be a live handle; out_satellite_count and out_epoch_count
/// must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_propagation_shape(
    batch: *const SidereonTleBatchPropagation,
    out_satellite_count: *mut usize,
    out_epoch_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_batch_propagation_shape",
        SidereonStatus::Panic,
        || {
            let out_satellite_count = c_try!(require_out(
                out_satellite_count,
                "sidereon_tle_batch_propagation_shape",
                "out_satellite_count"
            ));
            *out_satellite_count = 0;
            let out_epoch_count = c_try!(require_out(
                out_epoch_count,
                "sidereon_tle_batch_propagation_shape",
                "out_epoch_count"
            ));
            *out_epoch_count = 0;
            let batch = c_try!(require_ref(
                batch,
                "sidereon_tle_batch_propagation_shape",
                "batch"
            ));
            *out_satellite_count = batch.inner.len();
            *out_epoch_count = batch.epoch_count;
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened satellite-major TEME states from a batched propagation. Uses
/// the variable-length output contract documented at the top of the header.
///
/// Safety: batch must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_propagation_states(
    batch: *const SidereonTleBatchPropagation,
    out: *mut SidereonTemeState,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_batch_propagation_states",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tle_batch_propagation_states",
                out_written,
                out_required
            ));
            let batch = c_try!(require_ref(
                batch,
                "sidereon_tle_batch_propagation_states",
                "batch"
            ));
            c_try!(copy_flattened_rows_to_c(
                "sidereon_tle_batch_propagation_states",
                &batch.inner,
                prediction_to_c,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compute topocentric look angles for a fleet of TLEs over a shared epoch
/// grid. When parallel is true the engine's rayon batch path is used. On
/// success writes a newly owned batch handle to *out_batch. Release it with
/// sidereon_tle_batch_look_angles_free.
///
/// Safety: tles must point to tle_count line pairs; station must point to a
/// SidereonGroundStation; epochs_unix_us must point to epoch_count int64_t
/// values; out_batch must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_look_angles(
    tles: *const SidereonTlePair,
    tle_count: usize,
    station: *const SidereonGroundStation,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    opsmode: u32,
    parallel: bool,
    out_batch: *mut *mut SidereonTleBatchLookAngles,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_batch_look_angles",
        SidereonStatus::Panic,
        || {
            let out_batch = c_try!(require_out(
                out_batch,
                "sidereon_tle_batch_look_angles",
                "out_batch"
            ));
            *out_batch = ptr::null_mut();
            let satellites = c_try!(tle_pair_satellites_from_c(
                "sidereon_tle_batch_look_angles",
                tles,
                tle_count,
                opsmode,
            ));
            let station = c_try!(require_ref(
                station,
                "sidereon_tle_batch_look_angles",
                "station"
            ));
            let instants = c_try!(unix_instants_from_c(
                "sidereon_tle_batch_look_angles",
                epochs_unix_us,
                epoch_count,
            ));
            let epoch_count = instants.len();
            let ground_station = ground_station_from_c(station);
            let results = if parallel {
                look_angle_batch_parallel(&satellites, ground_station, &instants)
            } else {
                look_angle_batch_serial(&satellites, ground_station, &instants)
            };
            let inner = c_try!(unwrap_look_batch("sidereon_tle_batch_look_angles", results));
            write_boxed_handle(out_batch, SidereonTleBatchLookAngles { epoch_count, inner });
            SidereonStatus::Ok
        },
    )
}

/// Copy the shape of a batched look-angle result as satellite_count and
/// epoch_count.
///
/// Safety: batch must be a live handle; out_satellite_count and out_epoch_count
/// must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_look_angles_shape(
    batch: *const SidereonTleBatchLookAngles,
    out_satellite_count: *mut usize,
    out_epoch_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_batch_look_angles_shape",
        SidereonStatus::Panic,
        || {
            let out_satellite_count = c_try!(require_out(
                out_satellite_count,
                "sidereon_tle_batch_look_angles_shape",
                "out_satellite_count"
            ));
            *out_satellite_count = 0;
            let out_epoch_count = c_try!(require_out(
                out_epoch_count,
                "sidereon_tle_batch_look_angles_shape",
                "out_epoch_count"
            ));
            *out_epoch_count = 0;
            let batch = c_try!(require_ref(
                batch,
                "sidereon_tle_batch_look_angles_shape",
                "batch"
            ));
            *out_satellite_count = batch.inner.len();
            *out_epoch_count = batch.epoch_count;
            SidereonStatus::Ok
        },
    )
}

/// Copy flattened satellite-major look-angle rows from a batched result. Uses
/// the variable-length output contract documented at the top of the header.
///
/// Safety: batch must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_look_angles_values(
    batch: *const SidereonTleBatchLookAngles,
    out: *mut SidereonLookAngle,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tle_batch_look_angles_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_tle_batch_look_angles_values",
                out_written,
                out_required
            ));
            let batch = c_try!(require_ref(
                batch,
                "sidereon_tle_batch_look_angles_values",
                "batch"
            ));
            c_try!(copy_flattened_rows_to_c(
                "sidereon_tle_batch_look_angles_values",
                &batch.inner,
                look_angle_to_c,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a TLE handle. Null is a no-op. A non-null handle must come from
/// sidereon_tle_load and must be freed exactly once with this function.
///
/// Safety: tle must be NULL or a live handle from sidereon_tle_load. Passing a
/// handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_free(tle: *mut SidereonTle) {
    ffi_boundary("sidereon_tle_free", (), || {
        free_boxed(tle);
    });
}

/// Create an empty SGP4 decay latch.
///
/// Safety: out_latch must point to storage for a SidereonSgp4DecayLatch*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_decay_latch_new(
    out_latch: *mut *mut SidereonSgp4DecayLatch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_decay_latch_new",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_latch,
                "sidereon_sgp4_decay_latch_new",
                "out_latch"
            ));
            *out = ptr::null_mut();
            write_boxed_handle(
                out,
                SidereonSgp4DecayLatch {
                    inner: DecayLatch::new(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Clear the recorded decay state from a latch.
///
/// Safety: latch must be a live SidereonSgp4DecayLatch handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_decay_latch_clear(
    latch: *mut SidereonSgp4DecayLatch,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_decay_latch_clear",
        SidereonStatus::Panic,
        || {
            let latch = c_try!(require_mut(
                latch,
                "sidereon_sgp4_decay_latch_clear",
                "latch"
            ));
            latch.inner.clear();
            SidereonStatus::Ok
        },
    )
}

/// Copy the first failing epoch recorded by a decay latch.
///
/// Safety: latch must be live; out_has_epoch and out_minutes_since_epoch must
/// point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_decay_latch_first_failing_epoch(
    latch: *const SidereonSgp4DecayLatch,
    out_has_epoch: *mut bool,
    out_minutes_since_epoch: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sgp4_decay_latch_first_failing_epoch",
        SidereonStatus::Panic,
        || {
            let out_has = c_try!(require_out(
                out_has_epoch,
                "sidereon_sgp4_decay_latch_first_failing_epoch",
                "out_has_epoch"
            ));
            *out_has = false;
            let out_minutes = c_try!(require_out(
                out_minutes_since_epoch,
                "sidereon_sgp4_decay_latch_first_failing_epoch",
                "out_minutes_since_epoch"
            ));
            *out_minutes = 0.0;
            let latch = c_try!(require_ref(
                latch,
                "sidereon_sgp4_decay_latch_first_failing_epoch",
                "latch"
            ));
            if let Some(epoch) = latch.inner.first_failing_epoch() {
                *out_has = true;
                *out_minutes = epoch.0;
            }
            SidereonStatus::Ok
        },
    )
}

/// Release an SGP4 decay latch handle. Passing NULL is a no-op.
///
/// Safety: latch must be NULL or a live handle from
/// sidereon_sgp4_decay_latch_new.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sgp4_decay_latch_free(latch: *mut SidereonSgp4DecayLatch) {
    ffi_boundary("sidereon_sgp4_decay_latch_free", (), || {
        free_boxed(latch);
    });
}

/// Release a parsed TLE file handle. Null is a no-op. A non-null handle must
/// come from sidereon_parse_tle_file and must be freed exactly once with this
/// function. TLE handles previously obtained with sidereon_tle_file_satellite
/// are independent and are unaffected by freeing the file.
///
/// Safety: file must be NULL or a live handle from sidereon_parse_tle_file.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_file_free(file: *mut SidereonTleFile) {
    ffi_boundary("sidereon_tle_file_free", (), || {
        free_boxed(file);
    });
}

/// Release a TLE propagation handle. Null is a no-op. A non-null handle must
/// come from sidereon_tle_propagate and must be freed exactly once with this
/// function.
///
/// Safety: propagation must be NULL or a live handle from
/// sidereon_tle_propagate. Passing a handle after it has already been freed is
/// invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_propagation_free(propagation: *mut SidereonTlePropagation) {
    ffi_boundary("sidereon_tle_propagation_free", (), || {
        free_boxed(propagation);
    });
}

/// Release a look-angle handle. Null is a no-op. A non-null handle must come
/// from sidereon_tle_look_angles and must be freed exactly once with this
/// function.
///
/// Safety: look_angles must be NULL or a live handle from
/// sidereon_tle_look_angles. Passing a handle after it has already been freed
/// is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_look_angles_free(look_angles: *mut SidereonLookAngles) {
    ffi_boundary("sidereon_look_angles_free", (), || {
        free_boxed(look_angles);
    });
}

/// Release a pass-list handle. Null is a no-op. A non-null handle must come
/// from sidereon_tle_find_passes and must be freed exactly once with this
/// function.
///
/// Safety: passes must be NULL or a live handle from sidereon_tle_find_passes.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pass_list_free(passes: *mut SidereonPassList) {
    ffi_boundary("sidereon_pass_list_free", (), || {
        free_boxed(passes);
    });
}

/// Release a ground-track handle. Null is a no-op. A non-null handle must come
/// from sidereon_tle_ground_track and must be freed exactly once with this
/// function.
///
/// Safety: track must be NULL or a live handle from sidereon_tle_ground_track.
/// Passing a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ground_track_free(track: *mut SidereonGroundTrack) {
    ffi_boundary("sidereon_ground_track_free", (), || {
        free_boxed(track);
    });
}

/// Release a constellation-visibility handle. Null is a no-op. A non-null handle
/// must come from sidereon_visible_from_satellites and must be freed exactly once
/// with this function.
///
/// Safety: visible must be NULL or a live handle from
/// sidereon_visible_from_satellites. Passing a handle after it has already been
/// freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_visible_list_free(visible: *mut SidereonVisibleList) {
    ffi_boundary("sidereon_visible_list_free", (), || {
        free_boxed(visible);
    });
}

/// Release a batched propagation handle. Null is a no-op. A non-null handle
/// must come from sidereon_propagate_tle_batch and must be freed exactly once
/// with this function.
///
/// Safety: batch must be NULL or a live handle from
/// sidereon_propagate_tle_batch. Passing a handle after it has already been
/// freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_propagation_free(
    batch: *mut SidereonTleBatchPropagation,
) {
    ffi_boundary("sidereon_tle_batch_propagation_free", (), || {
        free_boxed(batch);
    });
}

/// Release a batched look-angle handle. Null is a no-op. A non-null handle must
/// come from sidereon_tle_batch_look_angles and must be freed exactly once with
/// this function.
///
/// Safety: batch must be NULL or a live handle from
/// sidereon_tle_batch_look_angles. Passing a handle after it has already been
/// freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tle_batch_look_angles_free(
    batch: *mut SidereonTleBatchLookAngles,
) {
    ffi_boundary("sidereon_tle_batch_look_angles_free", (), || {
        free_boxed(batch);
    });
}

/// Dense satellite pass list. Opaque to C. Create with
/// sidereon_tle_find_passes and release with sidereon_pass_list_free.
pub struct SidereonPassList {
    pub(crate) inner: Vec<sidereon::passes::SatellitePass>,
}

/// Initialize pass-finder options with engine defaults.
///
/// Safety: out_options must point to a SidereonPassFinderOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_pass_finder_options_init(
    out_options: *mut SidereonPassFinderOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_pass_finder_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_pass_finder_options_init",
                "out_options"
            ));
            *out_options = default_pass_finder_options();
            SidereonStatus::Ok
        },
    )
}

/// Per-epoch sub-satellite (ground-track) point arc. Opaque to C. Create with
/// sidereon_tle_ground_track and release with sidereon_ground_track_free.
pub struct SidereonGroundTrack {
    pub(crate) inner: Vec<Wgs84Geodetic>,
}

/// Find the satellites of a constellation visible above min_elevation_deg from a
/// ground station at one UTC unix-microsecond instant. tles is an array of count
/// live TLE handles (each carrying its own opsmode from sidereon_tle_load); ids
/// is a parallel array of count null-terminated C strings, where `ids[i]` labels
/// `tles[i]` and becomes the result's catalog_number. Each id must be a non-empty
/// string of 1..=64 bytes (MAX_VISIBLE_ID_BYTES, excluding the NUL terminator);
/// an empty id, or one not NUL-terminated within 64 bytes, yields
/// SIDEREON_STATUS_INVALID_ARGUMENT, matching the other id-accepting entry
/// points. Per-satellite propagation or frame failures are skipped; the result
/// is sorted by elevation descending. On success writes a newly owned handle to
/// *out_visible. Release it with sidereon_visible_list_free.
///
/// Safety: tles must point to count live TLE handle pointers; ids must point to
/// count null-terminated C-string pointers; station must point to a
/// SidereonGroundStation; out_visible must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_visible_from_satellites(
    tles: *const *const SidereonTle,
    ids: *const *const c_char,
    count: usize,
    station: *const SidereonGroundStation,
    epoch_unix_us: i64,
    min_elevation_deg: f64,
    out_visible: *mut *mut SidereonVisibleList,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_visible_from_satellites",
        SidereonStatus::Panic,
        || {
            let out_visible = c_try!(require_out(
                out_visible,
                "sidereon_visible_from_satellites",
                "out_visible"
            ));
            *out_visible = ptr::null_mut();
            let station = c_try!(require_ref(
                station,
                "sidereon_visible_from_satellites",
                "station"
            ));
            let tle_ptrs = c_try!(require_slice(
                tles,
                count,
                "sidereon_visible_from_satellites",
                "tles"
            ));
            let id_ptrs = c_try!(require_slice(
                ids,
                count,
                "sidereon_visible_from_satellites",
                "ids"
            ));
            let mut satellites = Vec::with_capacity(tle_ptrs.len());
            for (idx, tle_ptr) in tle_ptrs.iter().enumerate() {
                let tle = c_try!(require_ref(
                    *tle_ptr,
                    "sidereon_visible_from_satellites",
                    &format!("tles[{idx}]")
                ));
                satellites.push(tle.satellite.clone());
            }
            let mut id_strings = Vec::with_capacity(id_ptrs.len());
            for (idx, id_ptr) in id_ptrs.iter().enumerate() {
                id_strings.push(c_try!(parse_bounded_c_string(
                    "sidereon_visible_from_satellites",
                    &format!("ids[{idx}]"),
                    *id_ptr,
                    MAX_VISIBLE_ID_BYTES,
                )));
            }
            let inner = c_try!(visible_from_satellites(
                &satellites,
                &id_strings,
                ground_station_from_c(station),
                UtcInstant::from_unix_microseconds(epoch_unix_us),
                min_elevation_deg,
            )
            .map_err(|err| map_pass_error("sidereon_visible_from_satellites", err)));
            write_boxed_handle(out_visible, SidereonVisibleList { inner });
            SidereonStatus::Ok
        },
    )
}

fn map_look_angle_error(fn_name: &str, err: LookAngleError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::LookAngle,
        fn_name,
        look_angle_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        LookAngleError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        LookAngleError::Init(_)
        | LookAngleError::Propagate(_)
        | LookAngleError::FrameTransform(_) => SidereonStatus::Solve,
    }
}

fn parse_tle_handle(
    fn_name: &str,
    line1: *const c_char,
    line2: *const c_char,
    opsmode: u32,
    policy: TlePolicy,
) -> Result<SidereonTle, SidereonStatus> {
    let line1 = tle_line_from_c(fn_name, "line1", line1)?;
    let line2 = tle_line_from_c(fn_name, "line2", line2)?;
    let mode = tle_ops_mode_from_c(fn_name, opsmode)?;
    let parsed = sidereon_tle::parse_with_policy(&line1, &line2, policy).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })?;
    let (satellite, _) = Satellite::from_tle_with_policy(&line1, &line2, mode, policy)
        .map_err(|err| map_sgp4_error(fn_name, err))?;
    Ok(SidereonTle {
        elements: parsed.elements,
        satellite,
        checksum_warnings: parsed.checksum_warnings,
    })
}

/// Wrap a core `NamedSatellite` (from `parse_tle_file`) into the binding's TLE
/// record. The core satellite was already SGP4-initialized from its source
/// lines, so re-parsing `line1`/`line2` here only recovers the element metadata
/// and any advisory checksum warnings; the cached satellite is reused as-is.
fn named_satellite_to_record(
    fn_name: &str,
    named: NamedSatellite,
) -> Result<SidereonTleFileRecord, SidereonStatus> {
    let NamedSatellite {
        name,
        satellite,
        line_number,
        checksum_warnings,
    } = named;
    // The file reader already applied its checksum policy and kept the
    // findings it accepted; the element fields read the same under both
    // policies, so the lenient re-read only recovers them.
    let parsed =
        sidereon_tle::parse_with_policy(satellite.line1(), satellite.line2(), TlePolicy::Lenient)
            .map_err(|err| {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        })?;
    Ok(SidereonTleFileRecord {
        name,
        line_number,
        tle: SidereonTle {
            elements: parsed.elements,
            satellite,
            checksum_warnings,
        },
    })
}

fn tle_metadata_to_c(elements: &TleElements) -> SidereonTleMetadata {
    SidereonTleMetadata {
        catalog_number: fixed_c_chars(&elements.catalog_number),
        classification: fixed_c_chars(&elements.classification),
        international_designator: fixed_c_chars(&elements.international_designator),
        epoch_year: elements.epoch_year,
        epoch_day_of_year: elements.epoch_day_of_year,
        inclination_deg: elements.inclination_deg,
        raan_deg: elements.raan_deg,
        eccentricity: elements.eccentricity,
        arg_perigee_deg: elements.arg_perigee_deg,
        mean_anomaly_deg: elements.mean_anomaly_deg,
        mean_motion_rev_per_day: elements.mean_motion,
        mean_motion_dot: elements.mean_motion_dot,
        mean_motion_double_dot: elements.mean_motion_double_dot,
        bstar: elements.bstar,
        has_ephemeris_type: elements.ephemeris_type.is_some(),
        ephemeris_type: elements.ephemeris_type.unwrap_or(0),
        has_elset_number: elements.elset_number.is_some(),
        elset_number: elements.elset_number.unwrap_or(0),
        has_rev_number: elements.rev_number.is_some(),
        rev_number: elements.rev_number.unwrap_or(0),
    }
}

fn checksum_warning_to_c(warning: &ChecksumWarning) -> SidereonTleChecksumWarning {
    let line_number = if warning.line_label == "line 1" { 1 } else { 2 };
    let (kind, found) = match warning.kind {
        ChecksumWarningKind::Mismatch { expected } => {
            (SidereonTleChecksumWarningKind::Mismatch, expected)
        }
        ChecksumWarningKind::NotDigit { found } => {
            (SidereonTleChecksumWarningKind::NotDigit, found as u8)
        }
        ChecksumWarningKind::Missing => (SidereonTleChecksumWarningKind::Missing, 0),
    };
    SidereonTleChecksumWarning {
        line_number,
        kind,
        found,
        computed: warning.computed,
    }
}

pub(crate) fn tle_policy_from_c(fn_name: &str, policy: u32) -> Result<TlePolicy, SidereonStatus> {
    match policy {
        x if x == SidereonTlePolicy::Strict as u32 => Ok(TlePolicy::Strict),
        x if x == SidereonTlePolicy::Lenient as u32 => Ok(TlePolicy::Lenient),
        other => {
            set_last_error(format!("{fn_name}: unknown TLE policy {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn prediction_to_c(prediction: &Prediction) -> SidereonTemeState {
    SidereonTemeState {
        position_km: prediction.position,
        velocity_km_s: prediction.velocity,
    }
}

fn visible_satellite_to_c(sat: &VisibleSatellite) -> SidereonVisibleSatellite {
    SidereonVisibleSatellite {
        // The explicit turbofish ties the fixed buffer to VISIBLE_ID_C_BYTES, so
        // the struct's literal `[c_char; 65]` and the constant cannot drift apart
        // without a compile error.
        catalog_number: fixed_c_chars::<VISIBLE_ID_C_BYTES>(&sat.catalog_number),
        azimuth_deg: sat.azimuth_deg,
        elevation_deg: sat.elevation_deg,
        range_km: sat.range_km,
        position_km: sat.position_km,
    }
}

unsafe fn tle_pair_satellites_from_c(
    fn_name: &str,
    tles: *const SidereonTlePair,
    tle_count: usize,
    opsmode: u32,
) -> Result<Vec<Satellite>, SidereonStatus> {
    let raw_tles = require_slice(tles, tle_count, fn_name, "tles")?;
    validate_element_count::<Satellite>(fn_name, "tle_count", raw_tles.len())?;
    let mode = tle_ops_mode_from_c(fn_name, opsmode)?;
    let mut satellites = Vec::with_capacity(raw_tles.len());
    for (idx, row) in raw_tles.iter().enumerate() {
        let line1 = tle_line_from_c(fn_name, &format!("tles[{idx}].line1"), row.line1)?;
        let line2 = tle_line_from_c(fn_name, &format!("tles[{idx}].line2"), row.line2)?;
        let satellite = Satellite::from_tle_with_opsmode(&line1, &line2, mode).map_err(|err| {
            set_last_error(format!("{fn_name}: satellite {idx}: {err}"));
            record_sgp4_error(&err);
            match err {
                Sgp4Error::InvalidInput { .. } => SidereonStatus::InvalidArgument,
                Sgp4Error::NonFiniteOutput { .. } => SidereonStatus::Solve,
                Sgp4Error::InvalidTle(_) => SidereonStatus::InvalidArgument,
                Sgp4Error::Sgp4 { .. } => SidereonStatus::Solve,
                Sgp4Error::ResonanceStepBudget { .. } => SidereonStatus::Solve,
            }
        })?;
        satellites.push(satellite);
    }
    Ok(satellites)
}

fn unwrap_look_batch(
    fn_name: &str,
    results: Vec<Result<Vec<LookAngle>, LookAngleError>>,
) -> Result<Vec<Vec<LookAngle>>, SidereonStatus> {
    results
        .into_iter()
        .enumerate()
        .map(|(idx, arc)| {
            arc.map_err(|err| {
                set_last_error(format!("{fn_name}: satellite {idx}: {err}"));
                SidereonStatus::Solve
            })
        })
        .collect()
}

fn map_sgp4_error(fn_name: &str, err: Sgp4Error) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    record_sgp4_error(&err);
    match err {
        Sgp4Error::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        Sgp4Error::NonFiniteOutput { .. } => SidereonStatus::Solve,
        Sgp4Error::InvalidTle(_) => SidereonStatus::InvalidArgument,
        Sgp4Error::Sgp4 { .. } => SidereonStatus::Solve,
        Sgp4Error::ResonanceStepBudget { .. } => SidereonStatus::Solve,
    }
}

fn map_decay_latched_error(fn_name: &str, err: DecayLatchedError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    if let DecayLatchedError::Propagation(ref propagation_error) = err {
        record_sgp4_error(propagation_error);
    }
    match err {
        DecayLatchedError::Decayed { .. } => SidereonStatus::Solve,
        DecayLatchedError::Propagation(err) => match err {
            Sgp4Error::InvalidInput { .. } => SidereonStatus::InvalidArgument,
            Sgp4Error::NonFiniteOutput { .. }
            | Sgp4Error::Sgp4 { .. }
            | Sgp4Error::InvalidTle(_) => SidereonStatus::Solve,
            Sgp4Error::ResonanceStepBudget { .. } => SidereonStatus::Solve,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, snapshot_engine_error_for_test, SidereonEngineErrorFamily,
    };
    use serde_json::json;
    use sidereon_core::astro::frames::transforms::FrameTransformError;
    use sidereon_core::astro::passes::LookAngleError;
    use sidereon_core::astro::sgp4::{Error as Sgp4Error, Sgp4InputErrorKind};
    use sidereon_core::astro::time::DegradeReason;

    #[test]
    fn test_sgp4_error_value_variants() {
        let cases = [
            (
                Sgp4Error::InvalidInput {
                    field: "eccentricity",
                    kind: Sgp4InputErrorKind::OutOfRange,
                },
                json!({
                    "kind": "invalid_input",
                    "fields": { "field": "eccentricity", "kind": "out_of_range" }
                }),
            ),
            (
                Sgp4Error::NonFiniteOutput { field: "velocity" },
                json!({
                    "kind": "non_finite_output",
                    "fields": { "field": "velocity" }
                }),
            ),
            (
                Sgp4Error::InvalidTle("checksum mismatch".to_string()),
                json!({
                    "kind": "invalid_tle",
                    "fields": { "message": "checksum mismatch" }
                }),
            ),
            (
                Sgp4Error::Sgp4 { code: 6 },
                json!({
                    "kind": "sgp4",
                    "fields": { "code": 6 }
                }),
            ),
            (
                Sgp4Error::ResonanceStepBudget { budget: 1000 },
                json!({
                    "kind": "resonance_step_budget",
                    "fields": { "budget": 1000 }
                }),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(sgp4_error_value(&err), expected);
        }
    }

    #[test]
    fn test_look_angle_error_value_variants() {
        let cases = [
            (
                LookAngleError::InvalidInput {
                    field: "ground_station.latitude_deg",
                    reason: "out of range",
                },
                json!({
                    "kind": "invalid_input",
                    "fields": {
                        "field": "ground_station.latitude_deg",
                        "reason": "out of range"
                    }
                }),
            ),
            (
                LookAngleError::Init(Sgp4Error::InvalidTle("corrupt line 2".to_string())),
                json!({
                    "kind": "init",
                    "fields": {
                        "cause": {
                            "kind": "invalid_tle",
                            "fields": { "message": "corrupt line 2" }
                        }
                    }
                }),
            ),
            (
                LookAngleError::Propagate(Sgp4Error::NonFiniteOutput { field: "position" }),
                json!({
                    "kind": "propagate",
                    "fields": {
                        "cause": {
                            "kind": "non_finite_output",
                            "fields": { "field": "position" }
                        }
                    }
                }),
            ),
            (
                LookAngleError::FrameTransform(FrameTransformError::Ut1OutsideCoverage {
                    reason: DegradeReason::BeforeCoverage,
                }),
                json!({
                    "kind": "frame_transform",
                    "fields": {
                        "cause": {
                            "kind": "ut1_outside_coverage",
                            "fields": { "reason": "before_coverage" }
                        }
                    }
                }),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(look_angle_error_value(&err), expected);
        }
    }

    #[test]
    fn test_tle_find_passes_and_look_angles_lifecycle() {
        clear_engine_error();

        let l1 = b"1 25544U 98067A   18184.80969102  .00001614  00000-0  31745-4 0  9993\0";
        let l2 = b"2 25544  51.6414 295.8524 0003435 262.6267 204.2868 15.54005638121106\0";

        let mut tle: *mut SidereonTle = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_load(
                l1.as_ptr() as *const c_char,
                l2.as_ptr() as *const c_char,
                0,
                &mut tle,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!tle.is_null());

        let station = SidereonGroundStation {
            latitude_deg: 51.5074,
            longitude_deg: -0.1278,
            altitude_m: 80.0,
        };
        let start_unix_us = 1_530_619_200_000_000i64; // 2018-07-03 12:00:00 UTC
        let end_unix_us = 1_530_705_600_000_000i64; // 2018-07-04 12:00:00 UTC (+24h)
        let mut options = std::mem::MaybeUninit::<SidereonPassFinderOptions>::uninit();
        let status = unsafe { sidereon_pass_finder_options_init(options.as_mut_ptr()) };
        assert_eq!(status, SidereonStatus::Ok);
        let mut options = unsafe { options.assume_init() };
        options.elevation_mask_deg = 0.0;
        options.step_seconds = 10.0;
        options.time_tolerance_s = 1.0e-3;

        // 1. Valid control: create and hold live handle in SEPARATE pointer
        let mut live_passes: *mut SidereonPassList = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut live_passes,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!live_passes.is_null());
        assert!(snapshot_engine_error_for_test().is_none());

        // 2. Real producer refusal for Pass: station latitude out of range
        let invalid_station = SidereonGroundStation {
            latitude_deg: 150.0,
            longitude_deg: 0.0,
            altitude_m: 0.0,
        };
        let mut failure_out: *mut SidereonPassList = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &invalid_station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut failure_out,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(failure_out.is_null()); // Live pointer not clobbered

        let (info, payload) = snapshot_engine_error_for_test().expect("engine error recorded");
        assert_eq!(info.family, SidereonEngineErrorFamily::Pass);
        let v: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(v["family"], "pass");
        assert_eq!(v["operation"], "sidereon_tle_find_passes");
        assert_eq!(v["error"]["kind"], "invalid_input");
        assert_eq!(v["error"]["fields"]["field"], "ground_station.latitude_deg");
        assert_eq!(v["error"]["fields"]["reason"], "out of range");

        // 3. Real producer refusal for LookAngle: station latitude out of range
        let mut failure_looks: *mut SidereonLookAngles = ptr::null_mut();
        let epochs = [start_unix_us];
        let status = unsafe {
            sidereon_tle_look_angles(
                tle,
                &invalid_station,
                epochs.as_ptr(),
                1,
                &mut failure_looks,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(failure_looks.is_null());

        let (info, payload) = snapshot_engine_error_for_test().expect("engine error recorded");
        assert_eq!(info.family, SidereonEngineErrorFamily::LookAngle);
        let v: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(v["family"], "look_angle");
        assert_eq!(v["operation"], "sidereon_tle_look_angles");
        assert_eq!(v["error"]["kind"], "invalid_input");
        assert_eq!(v["error"]["fields"]["field"], "ground_station.latitude_deg");
        assert_eq!(v["error"]["fields"]["reason"], "out of range");

        // 4. Trigger failure again to test retention across ACTUALLY LIVE getters and free
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &invalid_station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut failure_out,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(failure_out.is_null());

        let (base_info, base_payload) =
            snapshot_engine_error_for_test().expect("engine error recorded before getters");
        assert_eq!(base_info.family, SidereonEngineErrorFamily::Pass);

        // Call count on the live handle
        let mut count = 0usize;
        let status = unsafe { sidereon_pass_list_count(live_passes, &mut count) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(count > 0);
        let (info_after_count, payload_after_count) =
            snapshot_engine_error_for_test().expect("retained across count");
        assert_eq!(info_after_count.family, base_info.family);
        assert_eq!(info_after_count.payload_len, base_info.payload_len);
        assert_eq!(payload_after_count.as_bytes(), base_payload.as_bytes());

        // Short buffer query on the live handle: out non-null with len 0 returns InvalidArgument
        let mut dummy = [SidereonSatellitePass {
            aos_unix_us: 0,
            los_unix_us: 0,
            culmination_unix_us: 0,
            max_elevation_deg: 0.0,
            duration_s: 0.0,
        }];
        let mut out_written = 999usize;
        let mut out_required = 0usize;
        let status = unsafe {
            sidereon_pass_list_values(
                live_passes,
                dummy.as_mut_ptr(),
                0,
                &mut out_written,
                &mut out_required,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(out_written, 0);
        assert_eq!(out_required, count);
        let (info_after_short, payload_after_short) =
            snapshot_engine_error_for_test().expect("retained across short copy");
        assert_eq!(info_after_short.family, base_info.family);
        assert_eq!(info_after_short.payload_len, base_info.payload_len);
        assert_eq!(payload_after_short.as_bytes(), base_payload.as_bytes());

        // Full buffer copy
        let mut passes_buf = vec![
            SidereonSatellitePass {
                aos_unix_us: 0,
                los_unix_us: 0,
                culmination_unix_us: 0,
                max_elevation_deg: 0.0,
                duration_s: 0.0,
            };
            count
        ];
        let status = unsafe {
            sidereon_pass_list_values(
                live_passes,
                passes_buf.as_mut_ptr(),
                count,
                &mut out_written,
                &mut out_required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(out_written, count);
        assert_eq!(out_required, count);
        let (info_after_full, payload_after_full) =
            snapshot_engine_error_for_test().expect("retained across full copy");
        assert_eq!(info_after_full.family, base_info.family);
        assert_eq!(info_after_full.payload_len, base_info.payload_len);
        assert_eq!(payload_after_full.as_bytes(), base_payload.as_bytes());

        // Free live handle
        unsafe { sidereon_pass_list_free(live_passes) };

        // FULL payload retained after ACTUALLY LIVE getters and free!
        let (info_after_free, payload_after_free) = snapshot_engine_error_for_test()
            .expect("engine error retained across live getters and free");
        assert_eq!(info_after_free.family, base_info.family);
        assert_eq!(info_after_free.payload_len, base_info.payload_len);
        assert_eq!(payload_after_free.as_bytes(), base_payload.as_bytes());
        let v: serde_json::Value = serde_json::from_str(&payload_after_free).expect("valid json");
        assert_eq!(v["family"], "pass");
        assert_eq!(v["operation"], "sidereon_tle_find_passes");
        assert_eq!(v["error"]["kind"], "invalid_input");

        // 5. Early NULL clear (seeded via real producer refusal)
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &invalid_station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut failure_out,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());

        let status = unsafe {
            sidereon_tle_find_passes(
                ptr::null(),
                &station,
                start_unix_us,
                end_unix_us,
                &options,
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        assert!(snapshot_engine_error_for_test().is_none());

        // 6. Success reset (seeded via real producer refusal)
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &invalid_station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut failure_out,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert!(snapshot_engine_error_for_test().is_some());

        let mut live_passes2: *mut SidereonPassList = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &station,
                start_unix_us,
                end_unix_us,
                &options,
                &mut live_passes2,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(snapshot_engine_error_for_test().is_none());

        unsafe { sidereon_pass_list_free(live_passes2) };
        unsafe { sidereon_tle_free(tle) };
    }
}
