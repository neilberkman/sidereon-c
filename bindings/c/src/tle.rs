use super::*;

pub const TLE_FIELD_C_BYTES: usize = 32;

/// A parsed TLE and initialized SGP4 satellite. Opaque to C. Create with
/// sidereon_tle_load and release with sidereon_tle_free.
#[derive(Clone)]
pub struct SidereonTle {
    pub(crate) elements: TleElements,
    pub(crate) satellite: Satellite,
    pub(crate) checksum_warnings: Vec<ChecksumWarning>,
}

/// A parsed multi-record CelesTrak/Space-Track TLE file. Opaque to C. Create
/// with sidereon_parse_tle_file and release with sidereon_tle_file_free.
pub struct SidereonTleFile {
    pub(crate) records: Vec<SidereonTleFileRecord>,
    pub(crate) skipped: usize,
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

/// Advisory checksum discrepancy from TLE parsing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTleChecksumWarning {
    /// TLE line number, 1 or 2.
    pub line_number: u8,
    /// Checksum digit found in column 69.
    pub expected: u8,
    /// Checksum recomputed from columns 1 through 68.
    pub computed: u8,
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
    /// Ephemeris type from line 1.
    pub ephemeris_type: i32,
    /// Element set number from line 1.
    pub elset_number: i32,
    /// Revolution number at epoch.
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
    /// Null-terminated caller-supplied satellite id (the matching ids[i] passed
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
        let tle = c_try!(parse_tle_handle("sidereon_tle_load", line1, line2, opsmode,));
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
            ephemeris_type: 0,
            elset_number: 0,
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
/// satellites. text must point to text_len readable UTF-8 bytes (the whole
/// file); opsmode is one of SidereonTleOpsMode_* encoded as uint32_t. Handles
/// bare 2-line sets, 3-line name+line1+line2 sets, and CelesTrak "0 NAME" name
/// lines; CRLF endings, blank lines, and surrounding whitespace are tolerated.
/// A record that fails SGP4 initialization is skipped and counted (see
/// sidereon_tle_file_skipped) rather than aborting the whole parse. On success
/// writes a newly owned handle to *out_file. Release it with
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
        let out_file = c_try!(require_out(out_file, "sidereon_parse_tle_file", "out_file"));
        *out_file = ptr::null_mut();
        let bytes = c_try!(require_slice(
            text,
            text_len,
            "sidereon_parse_tle_file",
            "text"
        ));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_parse_tle_file: text is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let mode = c_try!(tle_ops_mode_from_c("sidereon_parse_tle_file", opsmode));
        let parsed = parse_tle_file_with_opsmode(text, mode);
        let skipped = parsed.skipped;
        let mut records = Vec::with_capacity(parsed.satellites.len());
        for named in parsed.satellites {
            records.push(c_try!(named_satellite_to_record(
                "sidereon_parse_tle_file",
                named
            )));
        }
        write_boxed_handle(out_file, SidereonTleFile { records, skipped });
        SidereonStatus::Ok
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

/// Write the number of records that were found but skipped because their element
/// set failed SGP4 initialization to *out_skipped. An empty file
/// (count == 0, skipped == 0) is thus distinguishable from a fully corrupt one
/// (count == 0, skipped > 0).
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
        *out_skipped = file.skipped;
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
    ffi_boundary("sidereon_tle_look_angles", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_tle_find_passes", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_tle_ground_track", SidereonStatus::Panic, || {
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
/// is a parallel array of count null-terminated C strings, where ids[i] labels
/// tles[i] and becomes the result's catalog_number. Each id must be a non-empty
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
    ffi_boundary(
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
) -> Result<SidereonTle, SidereonStatus> {
    let line1 = tle_line_from_c(fn_name, "line1", line1)?;
    let line2 = tle_line_from_c(fn_name, "line2", line2)?;
    let mode = tle_ops_mode_from_c(fn_name, opsmode)?;
    let parsed = sidereon_tle::parse(&line1, &line2).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })?;
    let satellite = Satellite::from_tle_with_opsmode(&line1, &line2, mode)
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
    let NamedSatellite { name, satellite } = named;
    let parsed = sidereon_tle::parse(satellite.line1(), satellite.line2()).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })?;
    Ok(SidereonTleFileRecord {
        name,
        tle: SidereonTle {
            elements: parsed.elements,
            satellite,
            checksum_warnings: parsed.checksum_warnings,
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
        ephemeris_type: elements.ephemeris_type,
        elset_number: elements.elset_number,
        rev_number: elements.rev_number,
    }
}

fn checksum_warning_to_c(warning: &ChecksumWarning) -> SidereonTleChecksumWarning {
    let line_number = if warning.line_label == "line 1" { 1 } else { 2 };
    SidereonTleChecksumWarning {
        line_number,
        expected: warning.expected,
        computed: warning.computed,
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
            match err {
                Sgp4Error::InvalidInput { .. } => SidereonStatus::InvalidArgument,
                Sgp4Error::NonFiniteOutput { .. } => SidereonStatus::Solve,
                Sgp4Error::InvalidTle(_) => SidereonStatus::InvalidArgument,
                Sgp4Error::Sgp4 { .. } => SidereonStatus::Solve,
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
    match err {
        Sgp4Error::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        Sgp4Error::NonFiniteOutput { .. } => SidereonStatus::Solve,
        Sgp4Error::InvalidTle(_) => SidereonStatus::InvalidArgument,
        Sgp4Error::Sgp4 { .. } => SidereonStatus::Solve,
    }
}
