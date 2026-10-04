use super::*;
use sidereon_core::rinex::observations::{
    CarrierPhaseRow, CorrectionUnavailable, ObsDowngradeChange, ObsHeader, RinexObsWriteError,
};

/// A parsed RINEX observation product. Create with sidereon_rinex_obs_parse and
/// release with sidereon_rinex_obs_free.
pub struct SidereonRinexObs {
    pub(crate) inner: RinexObs,
}

pub const RINEX_OBS_CODE_C_BYTES: usize = 9;

pub const RINEX_OBS_MARKER_C_BYTES: usize = 65;

/// RINEX observation kind inferred from the observation-code leading letter.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsKind {
    /// Code pseudorange.
    Pseudorange = 0,
    /// Carrier phase.
    CarrierPhase = 1,
    /// Doppler.
    Doppler = 2,
    /// Signal strength.
    SignalStrength = 3,
    /// Unknown or unsupported leading code letter.
    Unknown = 4,
}

/// Parsed RINEX observation header summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsHeader {
    /// Full RINEX version.
    pub version: f64,
    /// Whether approx_position_m is present.
    pub has_approx_position_m: bool,
    /// Surveyed a-priori receiver position, ECEF meters.
    pub approx_position_m: [f64; 3],
    /// Whether antenna_delta_hen_m is present.
    pub has_antenna_delta_hen_m: bool,
    /// Antenna offset in RINEX height/east/north convention, meters.
    pub antenna_delta_hen_m: [f64; 3],
    /// Whether interval_s is present.
    pub has_interval_s: bool,
    /// Nominal epoch spacing, seconds.
    pub interval_s: f64,
    /// Whether time_of_first_obs is present.
    pub has_time_of_first_obs: bool,
    /// First observation epoch.
    pub time_of_first_obs: SidereonCalendarEpoch,
    /// Time scale of time_of_first_obs as SidereonTimeScale.
    pub time_of_first_obs_scale: u32,
    /// Number of per-system observation-code rows.
    pub obs_code_count: usize,
    /// Number of phase-shift header rows.
    pub phase_shift_count: usize,
    /// Number of scale-factor header rows.
    pub scale_factor_count: usize,
    /// Number of GLONASS slot/channel rows.
    pub glonass_slot_count: usize,
    /// Whether marker_name is present.
    pub has_marker_name: bool,
    /// Marker name, null-terminated when present.
    pub marker_name: [c_char; RINEX_OBS_MARKER_C_BYTES],
}

/// One detached header snapshot in a RINEX observation header timeline.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsHeaderSegment {
    /// First epoch index for which this header is in effect.
    pub first_epoch_index: usize,
    /// A copied summary of the header in effect from first_epoch_index.
    pub header: SidereonRinexObsHeader,
}

/// Kind of one ordered change made by a RINEX 2 downgrade.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsDowngradeChangeKind {
    CodeRenamed = 0,
    CodeMoved = 1,
    CodeAdded = 2,
    CodeListRemoved = 3,
    ValueRounded = 4,
    CycleSlipRounded = 5,
    ScaleFactorsRemoved = 6,
    EpochPicosecondsRemoved = 7,
    ClockOffsetRounded = 8,
    InEventLists = 9,
    DeprecatedRecordsRemoved = 10,
    EventRecordsRewritten = 11,
}

/// Fixed-width metadata for one RINEX 2 downgrade change. Text and string-list
/// payloads are copied separately from the owning result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsDowngradeChange {
    pub kind: SidereonRinexObsDowngradeChangeKind,
    pub has_nested_change: bool,
    pub has_system: bool,
    pub system: u32,
    pub has_epoch_index: bool,
    pub epoch_index: usize,
    pub has_satellite: bool,
    pub satellite: SidereonSatelliteToken,
    pub has_from_index: bool,
    pub from_index: usize,
    pub has_to_index: bool,
    pub to_index: usize,
    pub has_from_value: bool,
    pub from_value: f64,
    pub has_to_value: bool,
    pub to_value: f64,
    pub has_picoseconds: bool,
    pub picoseconds: u32,
    pub has_count: bool,
    pub count: usize,
    pub has_code: bool,
    pub has_from_text: bool,
    pub has_to_text: bool,
    pub has_label: bool,
    pub codes_count: usize,
    pub records_count: usize,
    pub from_records_count: usize,
    pub to_records_count: usize,
}

/// Which scalar text payload to copy from a downgrade change.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsDowngradeTextField {
    Code = 0,
    From = 1,
    To = 2,
    Label = 3,
}

/// Which string-list payload to address on a downgrade change.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsDowngradeStringList {
    Codes = 0,
    Records = 1,
    FromRecords = 2,
    ToRecords = 3,
}

/// One per-system RINEX observation code from the header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsCode {
    /// GNSS system as SidereonGnssSystem.
    pub system: u32,
    /// Observation code, null-terminated.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
}

/// One parsed RINEX observation epoch summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsEpoch {
    /// Whether epoch carries a time. False for an event record whose epoch
    /// fields are blank, which RINEX 2.11 and 3.05 allow for an event without a
    /// significant epoch; an observation or cycle-slip epoch always has one.
    pub has_epoch: bool,
    /// Civil epoch in the file's time scale when has_epoch is true. When it is
    /// false the integer fields are 0, which names no calendar date, and second
    /// is NaN.
    pub epoch: SidereonCalendarEpoch,
    /// RINEX epoch flag.
    pub flag: u8,
    /// Number of satellites observed at this epoch.
    pub satellite_count: usize,
}

/// One labelled raw RINEX observation value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsValue {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// RINEX observation code.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Observation kind as SidereonRinexObsKind.
    pub kind: u32,
    /// Whether value is present. False means the field was blank.
    pub has_value: bool,
    /// Parsed value when present.
    pub value: f64,
    /// Loss-of-lock indicator, or -1 when absent.
    pub lli: i32,
    /// Signal-strength indicator, or -1 when absent.
    pub ssi: i32,
}

/// One selected single-frequency pseudorange row from a RINEX OBS epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsPseudorange {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Selected code pseudorange, meters.
    pub pseudorange_m: f64,
}

/// One carrier-phase row with carrier metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsCarrierPhase {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// RINEX carrier observation code.
    pub code: [c_char; RINEX_OBS_CODE_C_BYTES],
    /// Whether value_cycles is present.
    pub has_value_cycles: bool,
    /// Phase in cycles when present.
    pub value_cycles: f64,
    /// Loss-of-lock indicator, or -1 when absent.
    pub lli: i32,
    /// Signal-strength indicator, or -1 when absent.
    pub ssi: i32,
    /// Whether frequency_hz is present.
    pub has_frequency_hz: bool,
    /// Carrier frequency, hertz.
    pub frequency_hz: f64,
    /// Whether wavelength_m is present.
    pub has_wavelength_m: bool,
    /// Carrier wavelength, meters.
    pub wavelength_m: f64,
    /// Whether value_m is present.
    pub has_value_m: bool,
    /// Carrier phase converted to meters.
    pub value_m: f64,
    /// Whether the header in effect at the epoch states one `SYS / PHASE SHIFT`
    /// correction for this satellite's signal.
    pub phase_shift_status: SidereonRinexCorrectionStatus,
    /// The phase-shift correction in cycles when phase_shift_status is
    /// Available, and NaN otherwise. No record, or a blank correction, is an
    /// available 0, as is every signal of a RINEX 4 file, whose records the
    /// format says to ignore. RINEX 3 phases are already aligned, so this is
    /// metadata and is not applied to value_cycles or value_m.
    pub phase_shift_cycles: f64,
    /// How many corrections the header block gives this signal when
    /// phase_shift_status is Ambiguous, and 0 otherwise. Copy them with
    /// sidereon_rinex_obs_carrier_phase_conflicts.
    pub phase_shift_conflict_count: usize,
}

/// Whether a header states one correction for a satellite's signal. Mirrors the
/// engine's `Result<f64, CorrectionUnavailable>`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexCorrectionStatus {
    /// The header states one correction.
    Available = 0,
    /// The header declares the correction unknown: the only `SYS / PHASE SHIFT`
    /// record covering the signal names just its constellation, which RINEX
    /// 3.05 section 5.2.12 gives where the applied corrections are unknown.
    Unknown = 1,
    /// Records in one header block give the signal different corrections.
    /// RINEX gives a block's records no order to choose one by, so every
    /// correction is kept and none is picked.
    Ambiguous = 2,
}

/// One correction an ambiguous header block gives a signal, in the order the
/// records give them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexPhaseShiftCorrection {
    /// Whether the record gives a correction. False for a record whose
    /// correction field is blank.
    pub has_cycles: bool,
    /// The correction in cycles when has_cycles is true, and NaN otherwise.
    pub cycles: f64,
}

/// Parse RINEX 3 observation text into a typed product. On success writes a newly
/// owned handle to *out_obs. Release it with sidereon_rinex_obs_free.
///
/// Safety: data must point to len readable bytes; out_obs must point to storage
/// for a SidereonRinexObs*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_parse(
    data: *const u8,
    len: usize,
    out_obs: *mut *mut SidereonRinexObs,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_parse", SidereonStatus::Panic, || {
        let out_obs = c_try!(require_out(out_obs, "sidereon_rinex_obs_parse", "out_obs"));
        *out_obs = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_rinex_obs_parse", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_rinex_obs_parse: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let inner = match RinexObs::parse(text) {
            Ok(obs) => obs,
            Err(err) => {
                set_last_error(format!("sidereon_rinex_obs_parse: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_obs, SidereonRinexObs { inner });
        SidereonStatus::Ok
    })
}

/// Read and parse a RINEX observation file from a UTF-8 filesystem path. On
/// success writes a newly owned handle to *out_obs. Release it with
/// sidereon_rinex_obs_free. Delegates to sidereon::load_rinex_obs.
///
/// Safety: path must be a non-empty UTF-8 C string; out_obs must point to
/// storage for a SidereonRinexObs*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_load(
    path: *const c_char,
    out_obs: *mut *mut SidereonRinexObs,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_load", SidereonStatus::Panic, || {
        let out_obs = c_try!(require_out(out_obs, "sidereon_rinex_obs_load", "out_obs"));
        *out_obs = ptr::null_mut();
        let path = c_try!(parse_c_string("sidereon_rinex_obs_load", "path", path));
        let inner = match sidereon::load_rinex_obs(&path) {
            Ok(obs) => obs,
            Err(err) => {
                set_last_error(format!("sidereon_rinex_obs_load: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        write_boxed_handle(out_obs, SidereonRinexObs { inner });
        SidereonStatus::Ok
    })
}

/// Write the parsed RINEX version (e.g. 3.05) to *out_version.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_version
/// must point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_version(
    obs: *const SidereonRinexObs,
    out_version: *mut f64,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_version", SidereonStatus::Panic, || {
        let out_version = c_try!(require_out(
            out_version,
            "sidereon_rinex_obs_version",
            "out_version"
        ));
        *out_version = 0.0;
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_version", "obs"));
        *out_version = obs.inner.header().version;
        SidereonStatus::Ok
    })
}

/// Write the number of epoch records (file order, event records included) to
/// *out_count.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_count
/// must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_epoch_count(
    obs: *const SidereonRinexObs,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_epoch_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_rinex_obs_epoch_count",
                "out_count"
            ));
            *out_count = 0;
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_epoch_count", "obs"));
            *out_count = obs.inner.epochs().len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the parsed RINEX observation header summary.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out_header
/// must point to a SidereonRinexObsHeader.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_header(
    obs: *const SidereonRinexObs,
    out_header: *mut SidereonRinexObsHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_header", SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(
            out_header,
            "sidereon_rinex_obs_header",
            "out_header"
        ));
        *out_header = empty_rinex_obs_header();
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_header", "obs"));
        *out_header = rinex_obs_header_to_c(obs.inner.header());
        SidereonStatus::Ok
    })
}

/// Copy the header in effect at one epoch. The returned value is detached from
/// the observation handle. An index equal to the epoch count is rejected.
///
/// Safety: obs must be a live handle; out_header must point to one writable
/// SidereonRinexObsHeader.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_header_at(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out_header: *mut SidereonRinexObsHeader,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_header_at";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_header = c_try!(require_out(out_header, FN_NAME, "out_header"));
        *out_header = empty_rinex_obs_header();
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        let header = match obs.inner.header_at(epoch_index) {
            Ok(header) => header,
            Err(error) => {
                set_last_error(format!("{FN_NAME}: {error}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        *out_header = rinex_obs_header_to_c(&header);
        SidereonStatus::Ok
    })
}

/// Copy every detached header-timeline segment in file order. The first row is
/// always `(0, file_header)`; later rows are effective event headers only.
/// Uses the standard two-call variable-length output contract.
///
/// Safety: obs must be live; out points to len writable segments or is NULL
/// when len is zero; count outputs point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_header_timeline(
    obs: *const SidereonRinexObs,
    out: *mut SidereonRinexObsHeaderSegment,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_header_timeline";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        let timeline = match obs.inner.header_timeline() {
            Ok(timeline) => timeline,
            Err(error) => {
                set_last_error(format!("{FN_NAME}: {error}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let values: Vec<SidereonRinexObsHeaderSegment> = timeline
            .segments()
            .map(
                |(first_epoch_index, header)| SidereonRinexObsHeaderSegment {
                    first_epoch_index,
                    header: rinex_obs_header_to_c(header),
                },
            )
            .collect();
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Copy how many input records the reader deliberately skipped.
///
/// Safety: obs must be live; out_count points to writable size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_skipped_records(
    obs: *const SidereonRinexObs,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_skipped_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        *out_count = obs.inner.skipped_records;
        SidereonStatus::Ok
    })
}

/// Copy the per-system observation-code table from the header. Uses the
/// variable-length output contract documented at the top of the header.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsCode entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_codes(
    obs: *const SidereonRinexObs,
    out: *mut SidereonRinexObsCode,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_codes", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_codes",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_codes", "obs"));
        let values: Vec<SidereonRinexObsCode> = obs
            .inner
            .header()
            .obs_codes
            .iter()
            .flat_map(|(system, codes)| {
                codes.iter().map(move |code| SidereonRinexObsCode {
                    system: gnss_system_to_c(*system) as u32,
                    code: rinex_obs_code_to_c(code),
                })
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_codes",
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

/// Copy parsed epoch summaries in file order. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsEpoch entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_epochs(
    obs: *const SidereonRinexObs,
    out: *mut SidereonRinexObsEpoch,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_epochs", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_epochs",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_epochs", "obs"));
        let values: Vec<SidereonRinexObsEpoch> = obs
            .inner
            .epochs()
            .iter()
            .map(|epoch| SidereonRinexObsEpoch {
                has_epoch: epoch.epoch.is_some(),
                epoch: match epoch.epoch {
                    Some(time) => rinex_epoch_time_to_c(time),
                    None => SidereonCalendarEpoch {
                        year: 0,
                        month: 0,
                        day: 0,
                        hour: 0,
                        minute: 0,
                        second: f64::NAN,
                    },
                },
                flag: epoch.flag,
                satellite_count: epoch.sats.len(),
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_epochs",
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

/// Copy flattened raw observation values for one epoch. Uses every observation
/// code in the header, in satellite and header-code order.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsValue entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_values(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsValue,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_obs_values", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_obs_values",
            out_written,
            out_required
        ));
        let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_values", "obs"));
        let Some(epoch) = obs.inner.epochs().get(epoch_index) else {
            set_last_error(format!(
                "sidereon_rinex_obs_values: epoch_index {epoch_index} out of range ({})",
                obs.inner.epochs().len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        let rows =
            match rinex_obs_observation_values(&obs.inner, epoch, &RinexObservationFilter::all()) {
                Ok(rows) => rows,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_values", err),
            };
        let values: Vec<SidereonRinexObsValue> = rows
            .into_iter()
            .flat_map(|(sat, rows)| {
                rows.into_iter().map(move |row| SidereonRinexObsValue {
                    sat_id: satellite_token(sat),
                    code: rinex_obs_code_to_c(&row.code),
                    kind: rinex_obs_kind_to_c(row.kind),
                    has_value: row.value.is_some(),
                    value: row.value.unwrap_or(0.0),
                    lli: row.lli.map(i32::from).unwrap_or(-1),
                    ssi: row.ssi.map(i32::from).unwrap_or(-1),
                })
            })
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_obs_values",
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

/// Copy flattened default-policy single-frequency pseudoranges for one epoch.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsPseudorange entries or be NULL
/// when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_pseudoranges(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsPseudorange,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_pseudoranges",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_pseudoranges",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_pseudoranges", "obs"));
            let Some(epoch) = obs.inner.epochs().get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_pseudoranges: epoch_index {epoch_index} out of range ({})",
                    obs.inner.epochs().len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let policy = match RinexSignalPolicy::default_for(obs.inner.header().version) {
                Ok(policy) => policy,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_pseudoranges", err),
            };
            let rows = match rinex_obs_pseudoranges(&obs.inner, epoch, &policy) {
                Ok(rows) => rows,
                Err(err) => return rinex_obs_error("sidereon_rinex_obs_pseudoranges", err),
            };
            let values: Vec<SidereonRinexObsPseudorange> = rows
                .into_iter()
                .map(|(sat, pseudorange_m)| SidereonRinexObsPseudorange {
                    sat_id: satellite_token(sat),
                    pseudorange_m,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_pseudoranges",
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

/// Carrier-phase rows for one epoch, flattened in satellite then header-code
/// order, read under the header in effect at the epoch: the file header with
/// every event at or before it laid over it, so a phase shift or GLONASS
/// channel an event declares applies to the epochs after it.
fn rinex_obs_carrier_phase_rows_at(
    fn_name: &str,
    obs: &RinexObs,
    epoch_index: usize,
) -> Result<Vec<(GnssSatelliteId, CarrierPhaseRow)>, SidereonStatus> {
    let Some(epoch) = obs.epochs().get(epoch_index) else {
        set_last_error(format!(
            "{fn_name}: epoch_index {epoch_index} out of range ({})",
            obs.epochs().len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    };
    let header = obs
        .header_at(epoch_index)
        .map_err(|err| rinex_obs_error(fn_name, err))?;
    let rows = rinex_obs_carrier_phase_rows(&header, epoch, &RinexObservationFilter::all())
        .map_err(|err| rinex_obs_error(fn_name, err))?;
    Ok(rows
        .into_iter()
        .flat_map(|(sat, rows)| rows.into_iter().map(move |row| (sat, row)))
        .collect())
}

/// Copy flattened carrier-phase rows for one epoch, in satellite then
/// header-code order.
///
/// Every row is copied whether or not the header states its phase-shift
/// correction. A row whose correction is unknown or ambiguous keeps its phase,
/// frequency and wavelength, and reports the correction through
/// phase_shift_status; an ambiguous row's corrections are copied with
/// sidereon_rinex_obs_carrier_phase_conflicts by the row's index here.
///
/// The rows are read under the header in effect at the epoch, so a phase shift
/// or GLONASS channel an event declares applies to the epochs after it. An
/// event record that does not read fails the call with
/// SIDEREON_STATUS_INVALID_ARGUMENT; a product read from text never holds one.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexObsCarrierPhase entries or be NULL
/// when len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_carrier_phase(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    out: *mut SidereonRinexObsCarrierPhase,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_carrier_phase",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_carrier_phase",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_carrier_phase", "obs"));
            let rows = c_try!(rinex_obs_carrier_phase_rows_at(
                "sidereon_rinex_obs_carrier_phase",
                &obs.inner,
                epoch_index
            ));
            let values: Vec<SidereonRinexObsCarrierPhase> = rows
                .iter()
                .map(|(sat, row)| {
                    let (phase_shift_status, phase_shift_cycles, phase_shift_conflict_count) =
                        match &row.phase_shift_cycles {
                            Ok(cycles) => (SidereonRinexCorrectionStatus::Available, *cycles, 0),
                            Err(CorrectionUnavailable::Unknown) => {
                                (SidereonRinexCorrectionStatus::Unknown, f64::NAN, 0)
                            }
                            Err(CorrectionUnavailable::Ambiguous { corrections }) => (
                                SidereonRinexCorrectionStatus::Ambiguous,
                                f64::NAN,
                                corrections.len(),
                            ),
                        };
                    SidereonRinexObsCarrierPhase {
                        sat_id: satellite_token(*sat),
                        code: rinex_obs_code_to_c(&row.code),
                        has_value_cycles: row.value_cycles.is_some(),
                        value_cycles: row.value_cycles.unwrap_or(0.0),
                        lli: row.lli.map(i32::from).unwrap_or(-1),
                        ssi: row.ssi.map(i32::from).unwrap_or(-1),
                        has_frequency_hz: row.frequency_hz.is_some(),
                        frequency_hz: row.frequency_hz.unwrap_or(0.0),
                        has_wavelength_m: row.wavelength_m.is_some(),
                        wavelength_m: row.wavelength_m.unwrap_or(0.0),
                        has_value_m: row.value_m.is_some(),
                        value_m: row.value_m.unwrap_or(0.0),
                        phase_shift_status,
                        phase_shift_cycles,
                        phase_shift_conflict_count,
                    }
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_carrier_phase",
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

/// Copy the corrections an ambiguous header block gives one carrier-phase row,
/// in the order the records give them. `row_index` indexes the rows
/// sidereon_rinex_obs_carrier_phase copies for the same epoch. A row whose
/// phase_shift_status is not Ambiguous reports a required count of zero. Uses
/// the variable-length output contract.
///
/// Returns SIDEREON_STATUS_INVALID_ARGUMENT when epoch_index or row_index is
/// out of range.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must
/// point to at least len writable SidereonRinexPhaseShiftCorrection entries or
/// be NULL when len is 0; out_written and out_required must point to size_t
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_carrier_phase_conflicts(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    row_index: usize,
    out: *mut SidereonRinexPhaseShiftCorrection,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_carrier_phase_conflicts";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        let rows = c_try!(rinex_obs_carrier_phase_rows_at(
            FN_NAME,
            &obs.inner,
            epoch_index
        ));
        let Some((_, row)) = rows.get(row_index) else {
            set_last_error(format!(
                "{FN_NAME}: row_index {row_index} out of range ({} rows at epoch {epoch_index})",
                rows.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        let values: Vec<SidereonRinexPhaseShiftCorrection> = match &row.phase_shift_cycles {
            Err(CorrectionUnavailable::Ambiguous { corrections }) => corrections
                .iter()
                .map(|correction| SidereonRinexPhaseShiftCorrection {
                    has_cycles: correction.is_some(),
                    cycles: correction.unwrap_or(f64::NAN),
                })
                .collect(),
            _ => Vec::new(),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
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

/// Look up one observation value at `epoch_index` for satellite `sat_id` and
/// observation `code` (e.g. "C1C"). On success writes the value to *out_value and
/// whether the field was present to *out_present (a blank field is present=false
/// with out_value=0). The loss-of-lock and signal-strength indicators are written
/// to *out_lli and *out_ssi as -1 when absent. The numbers are exactly what the
/// engine parsed.
///
/// Returns SIDEREON_STATUS_INVALID_ARGUMENT if epoch_index is out of range, the
/// satellite is not observed at that epoch, or `code` is not a declared code for
/// that satellite's constellation.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; sat_id and
/// code must be null-terminated C strings; out_value, out_present, out_lli and
/// out_ssi must each point to writable storage of the documented type. These
/// output ranges must not overlap.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_observation(
    obs: *const SidereonRinexObs,
    epoch_index: usize,
    sat_id: *const c_char,
    code: *const c_char,
    out_value: *mut f64,
    out_present: *mut bool,
    out_lli: *mut i32,
    out_ssi: *mut i32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_observation",
        SidereonStatus::Panic,
        || {
            if !out_value.is_null()
                && !out_present.is_null()
                && !out_lli.is_null()
                && !out_ssi.is_null()
            {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_rinex_obs_observation",
                            out_value,
                            1,
                            "out_value"
                        )),
                        "out_value",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_rinex_obs_observation",
                            out_present,
                            1,
                            "out_present"
                        )),
                        "out_present",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_rinex_obs_observation",
                            out_lli,
                            1,
                            "out_lli"
                        )),
                        "out_lli",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_rinex_obs_observation",
                            out_ssi,
                            1,
                            "out_ssi"
                        )),
                        "out_ssi",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_rinex_obs_observation",
                    &outputs
                ));
            }
            let out_value = c_try!(require_out(
                out_value,
                "sidereon_rinex_obs_observation",
                "out_value"
            ));
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_rinex_obs_observation",
                "out_present"
            ));
            let out_lli = c_try!(require_out(
                out_lli,
                "sidereon_rinex_obs_observation",
                "out_lli"
            ));
            let out_ssi = c_try!(require_out(
                out_ssi,
                "sidereon_rinex_obs_observation",
                "out_ssi"
            ));
            *out_value = 0.0;
            *out_present = false;
            *out_lli = -1;
            *out_ssi = -1;
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_observation", "obs"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_rinex_obs_observation",
                sat_id
            ));
            let code = c_try!(parse_bounded_c_string(
                "sidereon_rinex_obs_observation",
                "code",
                code,
                MAX_ANTEX_FREQUENCY_BYTES
            ));

            let epochs = obs.inner.epochs();
            let Some(epoch) = epochs.get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: epoch_index {epoch_index} out of range ({})",
                    epochs.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(values) = epoch.sats.get(&sat) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: satellite {sat} not observed at epoch {epoch_index}"
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(codes) = obs.inner.obs_codes(sat.system) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: no observation codes for {}",
                    sat.system
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(code_index) = codes.iter().position(|c| c == &code) else {
                set_last_error(format!(
                    "sidereon_rinex_obs_observation: code {code} not declared for {}",
                    sat.system
                ));
                return SidereonStatus::InvalidArgument;
            };
            let Some(value) = values.get(code_index) else {
                // The satellite row is shorter than the declared code list (trailing
                // blanks), so this code has no field at this epoch.
                return SidereonStatus::Ok;
            };
            if let Some(v) = value.value {
                *out_value = v;
                *out_present = true;
            }
            if let Some(lli) = value.lli {
                *out_lli = i32::from(lli);
            }
            if let Some(ssi) = value.ssi {
                *out_ssi = i32::from(ssi);
            }
            SidereonStatus::Ok
        },
    )
}

/// Which refusal a RINEX observation write reported. Every kind but None names
/// a `RinexObsWriteError` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexObsWriteErrorKind {
    /// No refusal is recorded.
    None = 0,
    /// A version 2 product whose constellations' code lists are not what one
    /// list of version 2 names reads as. Carries system, position and, when
    /// the lists differ at a code rather than in length, the code.
    CodeListsNotVersionTwo = 1,
    /// A downgrade asked for a version that is not a version 2. Carries
    /// version.
    NotVersionTwo = 2,
    /// A version 2 product carrying `SYS / SCALE FACTOR` records. Carries count.
    ScaleFactorsInVersionTwo = 3,
    /// A satellite holds more observations or cycle slips than its
    /// constellation has codes. Carries epoch_index, satellite, codes and
    /// values.
    ValuesWithoutCodes = 4,
    /// A `PRN / # OF OBS` record holds more counts than its constellation has
    /// codes. Carries satellite, codes and, in values, the counts it holds.
    CountsWithoutCodes = 5,
    /// A version 2 product holding a code list the file would not state.
    /// Carries system.
    CodeListNotStated = 6,
    /// An epoch flag the one-digit flag field cannot hold. Carries epoch_index
    /// and flag.
    EpochFlagTooWide = 7,
    /// An observation or cycle-slip epoch with no epoch time. Carries
    /// epoch_index and flag.
    EpochTimeMissing = 8,
    /// Epoch picoseconds in a product below version 4.02. Carries epoch_index
    /// and version.
    EpochPicosecondsNotInVersion = 9,
    /// More observation types than the three-digit count declares. Carries
    /// count.
    TooManyObservationTypes = 10,
    /// A code list that is not the union of the lists the header and its
    /// events declare. Carries system.
    CodeListsNotUnion = 11,
    /// A value under a code the list in effect at its epoch does not declare.
    /// Carries epoch_index, satellite and, when the constellation has a list in
    /// effect, the code.
    ValueOutsideDeclaredList = 12,
    /// A version 2 declared list its type names do not state. Carries system.
    DeclaredListNotStated = 13,
    /// An event's header record does not read. Carries the reader's text as the
    /// detail.
    EventRecordsUnreadable = 14,
    /// A code on a carrier the target version cannot represent. Carries system,
    /// the code and version.
    ObservableNotRepresentable = 15,
    /// A `LEAP SECONDS` time system the target version does not support.
    /// Carries the identifier as the detail, and version.
    LeapSecondsTimeSystemNotInVersion = 16,
    /// An unknown or malformed `LEAP SECONDS` time system. Carries the
    /// identifier as the detail.
    InvalidLeapSecondsTimeSystem = 17,
    /// The written text would read back as a different product. Carries the
    /// first field that would change, with its values, as the detail.
    ReadBackMismatch = 18,
}

/// Typed detail of a refused RINEX observation write.
///
/// Only the fields the kind names carry meaning, and each carries a present
/// flag. An absent number is NaN and an absent count or index is 0. The two
/// text parts are read from the owned SidereonRinexObsWriteResult: the
/// observation code with sidereon_rinex_obs_write_result_get_code, and the
/// reader text, time-system identifier or changed field with
/// sidereon_rinex_obs_write_result_get_detail.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsWriteError {
    /// Which refusal the writer reported.
    pub kind: SidereonRinexObsWriteErrorKind,
    /// Whether system names a constellation.
    pub has_system: bool,
    /// The constellation the refusal concerns, as SidereonGnssSystem.
    pub system: u32,
    /// Whether satellite names a satellite.
    pub has_satellite: bool,
    /// The satellite the refusal concerns.
    pub satellite: SidereonSatelliteToken,
    /// Whether epoch_index names an epoch.
    pub has_epoch_index: bool,
    /// Zero-based index of the epoch the refusal concerns.
    pub epoch_index: usize,
    /// Whether position carries a code-list position.
    pub has_position: bool,
    /// Zero-based position in the constellation's code list, or its length
    /// when the lists differ in length.
    pub position: usize,
    /// Whether flag carries the epoch flag.
    pub has_flag: bool,
    /// The epoch flag.
    pub flag: u8,
    /// Whether version carries a RINEX version.
    pub has_version: bool,
    /// The product's version, or the version a downgrade or write targets.
    pub version: f64,
    /// Whether count carries a record or type count.
    pub has_count: bool,
    /// Scale-factor records held, or observation types a version 2 list needs.
    pub count: usize,
    /// Whether codes carries a code count.
    pub has_codes: bool,
    /// Codes the satellite's constellation has.
    pub codes: usize,
    /// Whether values carries a value count.
    pub has_values: bool,
    /// Values the satellite holds, or counts its `PRN / # OF OBS` record holds.
    pub values: usize,
    /// Whether the refusal names an observation code.
    pub has_code: bool,
    /// Whether the refusal carries a detail text.
    pub has_detail: bool,
}

/// The complete outcome of one RINEX observation write: the fixed-width part of
/// an owned SidereonRinexObsWriteResult.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsWriteOutcome {
    /// Whether the product was written.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK when written, otherwise
    /// SIDEREON_STATUS_INVALID_ARGUMENT, which every refusal maps to.
    pub status: SidereonStatus,
    /// The typed refusal; kind is None when is_ok is true.
    pub error: SidereonRinexObsWriteError,
}

/// An owned record of one RINEX observation write: the text on success, or the
/// typed refusal with its owned text parts. The result owns every string it
/// reports. Create with sidereon_rinex_obs_to_rinex_text_result or
/// sidereon_rinex_repair_text_result and release with
/// sidereon_rinex_obs_write_result_free.
pub struct SidereonRinexObsWriteResult {
    pub(crate) outcome: SidereonRinexObsWriteOutcome,
    /// The written text on success; empty on a refusal.
    pub(crate) text: Vec<u8>,
    /// The refusal text, prefixed with the route that produced it; empty on
    /// success.
    pub(crate) message: String,
    /// The observation code a refusal names; empty when it names none.
    pub(crate) code: String,
    /// The detail text a refusal carries; empty when it carries none.
    pub(crate) detail: String,
}

/// Fixed-width outcome of one RINEX 2 downgrade attempt.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexObsDowngradeOutcome {
    /// True when a fresh product is available to take exactly once.
    pub is_ok: bool,
    /// OK on success, INVALID_ARGUMENT on a typed semantic refusal.
    pub status: SidereonStatus,
    /// The complete typed refusal; kind is None on success.
    pub error: SidereonRinexObsWriteError,
    /// Number of ordered top-level changes on success.
    pub change_count: usize,
}

/// Owned RINEX 2 downgrade result. It owns the fresh product until taken, the
/// complete ordered change tree, and every typed refusal text payload.
pub struct SidereonRinexObsDowngradeResult {
    pub(crate) outcome: SidereonRinexObsDowngradeOutcome,
    pub(crate) product: Option<Box<SidereonRinexObs>>,
    pub(crate) changes: Vec<ObsDowngradeChange>,
    pub(crate) message: String,
    pub(crate) code: String,
    pub(crate) detail: String,
}

fn no_rinex_obs_write_error() -> SidereonRinexObsWriteError {
    SidereonRinexObsWriteError {
        kind: SidereonRinexObsWriteErrorKind::None,
        has_system: false,
        system: 0,
        has_satellite: false,
        satellite: SidereonSatelliteToken {
            bytes: [0; SATELLITE_TOKEN_C_BYTES],
        },
        has_epoch_index: false,
        epoch_index: 0,
        has_position: false,
        position: 0,
        has_flag: false,
        flag: 0,
        has_version: false,
        version: f64::NAN,
        has_count: false,
        count: 0,
        has_codes: false,
        codes: 0,
        has_values: false,
        values: 0,
        has_code: false,
        has_detail: false,
    }
}

/// Map every field of an engine observation write refusal into the C record
/// and its two text parts: the observation code and the detail text.
fn rinex_obs_write_refusal(
    error: &RinexObsWriteError,
) -> (SidereonRinexObsWriteError, Option<String>, Option<String>) {
    use SidereonRinexObsWriteErrorKind as Kind;
    let mut out = no_rinex_obs_write_error();
    let mut code = None;
    let mut detail = None;
    let system = |out: &mut SidereonRinexObsWriteError, system: GnssSystem| {
        out.has_system = true;
        out.system = gnss_system_to_c(system) as u32;
    };
    let satellite = |out: &mut SidereonRinexObsWriteError, sat: GnssSatelliteId| {
        out.has_satellite = true;
        out.satellite = satellite_token(sat);
    };
    let epoch_index = |out: &mut SidereonRinexObsWriteError, index: usize| {
        out.has_epoch_index = true;
        out.epoch_index = index;
    };
    let version = |out: &mut SidereonRinexObsWriteError, value: f64| {
        out.has_version = true;
        out.version = value;
    };
    let flag = |out: &mut SidereonRinexObsWriteError, value: u8| {
        out.has_flag = true;
        out.flag = value;
    };
    match error {
        RinexObsWriteError::CodeListsNotVersionTwo {
            system: sys,
            position,
            code: held,
        } => {
            out.kind = Kind::CodeListsNotVersionTwo;
            system(&mut out, *sys);
            out.has_position = true;
            out.position = *position;
            code = held.clone();
        }
        RinexObsWriteError::NotVersionTwo { version: target } => {
            out.kind = Kind::NotVersionTwo;
            version(&mut out, *target);
        }
        RinexObsWriteError::ScaleFactorsInVersionTwo { count } => {
            out.kind = Kind::ScaleFactorsInVersionTwo;
            out.has_count = true;
            out.count = *count;
        }
        RinexObsWriteError::ValuesWithoutCodes {
            epoch_index: index,
            satellite: sat,
            codes,
            values,
        } => {
            out.kind = Kind::ValuesWithoutCodes;
            epoch_index(&mut out, *index);
            satellite(&mut out, *sat);
            out.has_codes = true;
            out.codes = *codes;
            out.has_values = true;
            out.values = *values;
        }
        RinexObsWriteError::CountsWithoutCodes {
            satellite: sat,
            codes,
            counts,
        } => {
            out.kind = Kind::CountsWithoutCodes;
            satellite(&mut out, *sat);
            out.has_codes = true;
            out.codes = *codes;
            out.has_values = true;
            out.values = *counts;
        }
        RinexObsWriteError::CodeListNotStated { system: sys } => {
            out.kind = Kind::CodeListNotStated;
            system(&mut out, *sys);
        }
        RinexObsWriteError::EpochFlagTooWide {
            epoch_index: index,
            flag: value,
        } => {
            out.kind = Kind::EpochFlagTooWide;
            epoch_index(&mut out, *index);
            flag(&mut out, *value);
        }
        RinexObsWriteError::EpochTimeMissing {
            epoch_index: index,
            flag: value,
        } => {
            out.kind = Kind::EpochTimeMissing;
            epoch_index(&mut out, *index);
            flag(&mut out, *value);
        }
        RinexObsWriteError::EpochPicosecondsNotInVersion {
            epoch_index: index,
            version: held,
        } => {
            out.kind = Kind::EpochPicosecondsNotInVersion;
            epoch_index(&mut out, *index);
            version(&mut out, *held);
        }
        RinexObsWriteError::TooManyObservationTypes { count } => {
            out.kind = Kind::TooManyObservationTypes;
            out.has_count = true;
            out.count = *count;
        }
        RinexObsWriteError::CodeListsNotUnion { system: sys } => {
            out.kind = Kind::CodeListsNotUnion;
            system(&mut out, *sys);
        }
        RinexObsWriteError::ValueOutsideDeclaredList {
            epoch_index: index,
            satellite: sat,
            code: held,
        } => {
            out.kind = Kind::ValueOutsideDeclaredList;
            epoch_index(&mut out, *index);
            satellite(&mut out, *sat);
            code = held.clone();
        }
        RinexObsWriteError::DeclaredListNotStated { system: sys } => {
            out.kind = Kind::DeclaredListNotStated;
            system(&mut out, *sys);
        }
        RinexObsWriteError::EventRecordsUnreadable { message } => {
            out.kind = Kind::EventRecordsUnreadable;
            detail = Some(message.clone());
        }
        RinexObsWriteError::ObservableNotRepresentable {
            system: sys,
            code: held,
            version: target,
        } => {
            out.kind = Kind::ObservableNotRepresentable;
            system(&mut out, *sys);
            code = Some(held.clone());
            version(&mut out, *target);
        }
        RinexObsWriteError::LeapSecondsTimeSystemNotInVersion {
            time_system,
            version: target,
        } => {
            out.kind = Kind::LeapSecondsTimeSystemNotInVersion;
            detail = Some(time_system.clone());
            version(&mut out, *target);
        }
        RinexObsWriteError::InvalidLeapSecondsTimeSystem { time_system } => {
            out.kind = Kind::InvalidLeapSecondsTimeSystem;
            detail = Some(time_system.clone());
        }
        RinexObsWriteError::ReadBackMismatch { what } => {
            out.kind = Kind::ReadBackMismatch;
            detail = Some(what.clone());
        }
    }
    out.has_code = code.is_some();
    out.has_detail = detail.is_some();
    (out, code, detail)
}

/// Record the whole outcome of one observation write: the text, or the typed
/// refusal and its text parts. Nothing partial is kept from a refused write.
fn rinex_obs_write_result(
    fn_name: &str,
    written: Result<Vec<u8>, &RinexObsWriteError>,
) -> SidereonRinexObsWriteResult {
    match written {
        Ok(text) => SidereonRinexObsWriteResult {
            outcome: SidereonRinexObsWriteOutcome {
                is_ok: true,
                status: SidereonStatus::Ok,
                error: no_rinex_obs_write_error(),
            },
            text,
            message: String::new(),
            code: String::new(),
            detail: String::new(),
        },
        Err(err) => {
            let (error, code, detail) = rinex_obs_write_refusal(err);
            SidereonRinexObsWriteResult {
                outcome: SidereonRinexObsWriteOutcome {
                    is_ok: false,
                    status: SidereonStatus::InvalidArgument,
                    error,
                },
                text: Vec::new(),
                message: format!("{fn_name}: {err}"),
                code: code.unwrap_or_default(),
                detail: detail.unwrap_or_default(),
            }
        }
    }
}

fn empty_rinex_obs_downgrade_change(
    kind: SidereonRinexObsDowngradeChangeKind,
) -> SidereonRinexObsDowngradeChange {
    SidereonRinexObsDowngradeChange {
        kind,
        has_nested_change: false,
        has_system: false,
        system: 0,
        has_epoch_index: false,
        epoch_index: 0,
        has_satellite: false,
        satellite: SidereonSatelliteToken {
            bytes: [0; SATELLITE_TOKEN_C_BYTES],
        },
        has_from_index: false,
        from_index: 0,
        has_to_index: false,
        to_index: 0,
        has_from_value: false,
        from_value: f64::NAN,
        has_to_value: false,
        to_value: f64::NAN,
        has_picoseconds: false,
        picoseconds: 0,
        has_count: false,
        count: 0,
        has_code: false,
        has_from_text: false,
        has_to_text: false,
        has_label: false,
        codes_count: 0,
        records_count: 0,
        from_records_count: 0,
        to_records_count: 0,
    }
}

fn rinex_obs_downgrade_change_to_c(change: &ObsDowngradeChange) -> SidereonRinexObsDowngradeChange {
    use SidereonRinexObsDowngradeChangeKind as Kind;
    let mut out = empty_rinex_obs_downgrade_change(match change {
        ObsDowngradeChange::CodeRenamed { .. } => Kind::CodeRenamed,
        ObsDowngradeChange::CodeMoved { .. } => Kind::CodeMoved,
        ObsDowngradeChange::CodeAdded { .. } => Kind::CodeAdded,
        ObsDowngradeChange::CodeListRemoved { .. } => Kind::CodeListRemoved,
        ObsDowngradeChange::ValueRounded { .. } => Kind::ValueRounded,
        ObsDowngradeChange::CycleSlipRounded { .. } => Kind::CycleSlipRounded,
        ObsDowngradeChange::ScaleFactorsRemoved { .. } => Kind::ScaleFactorsRemoved,
        ObsDowngradeChange::EpochPicosecondsRemoved { .. } => Kind::EpochPicosecondsRemoved,
        ObsDowngradeChange::ClockOffsetRounded { .. } => Kind::ClockOffsetRounded,
        ObsDowngradeChange::InEventLists { .. } => Kind::InEventLists,
        ObsDowngradeChange::DeprecatedRecordsRemoved { .. } => Kind::DeprecatedRecordsRemoved,
        ObsDowngradeChange::EventRecordsRewritten { .. } => Kind::EventRecordsRewritten,
    });
    let system = |out: &mut SidereonRinexObsDowngradeChange, value: GnssSystem| {
        out.has_system = true;
        out.system = gnss_system_to_c(value) as u32;
    };
    let epoch = |out: &mut SidereonRinexObsDowngradeChange, value: usize| {
        out.has_epoch_index = true;
        out.epoch_index = value;
    };
    match change {
        ObsDowngradeChange::CodeRenamed { system: value, .. } => {
            system(&mut out, *value);
            out.has_from_text = true;
            out.has_to_text = true;
        }
        ObsDowngradeChange::CodeMoved {
            system: value,
            from,
            to,
            ..
        } => {
            system(&mut out, *value);
            out.has_code = true;
            out.has_from_index = true;
            out.from_index = *from;
            out.has_to_index = true;
            out.to_index = *to;
        }
        ObsDowngradeChange::CodeAdded { system: value, .. } => {
            system(&mut out, *value);
            out.has_code = true;
        }
        ObsDowngradeChange::CodeListRemoved {
            system: value,
            codes,
        } => {
            system(&mut out, *value);
            out.codes_count = codes.len();
        }
        ObsDowngradeChange::ValueRounded {
            epoch_index,
            satellite,
            from,
            to,
            ..
        }
        | ObsDowngradeChange::CycleSlipRounded {
            epoch_index,
            satellite,
            from,
            to,
            ..
        } => {
            epoch(&mut out, *epoch_index);
            out.has_satellite = true;
            out.satellite = satellite_token(*satellite);
            out.has_code = true;
            out.has_from_value = true;
            out.from_value = *from;
            out.has_to_value = true;
            out.to_value = *to;
        }
        ObsDowngradeChange::ScaleFactorsRemoved { count } => {
            out.has_count = true;
            out.count = *count;
        }
        ObsDowngradeChange::EpochPicosecondsRemoved {
            epoch_index,
            picoseconds,
        } => {
            epoch(&mut out, *epoch_index);
            out.has_picoseconds = true;
            out.picoseconds = *picoseconds;
        }
        ObsDowngradeChange::ClockOffsetRounded {
            epoch_index,
            from,
            to,
        } => {
            epoch(&mut out, *epoch_index);
            out.has_from_value = true;
            out.from_value = *from;
            out.has_to_value = true;
            out.to_value = *to;
        }
        ObsDowngradeChange::InEventLists { epoch_index, .. } => {
            epoch(&mut out, *epoch_index);
            out.has_nested_change = true;
        }
        ObsDowngradeChange::DeprecatedRecordsRemoved {
            epoch_index,
            records,
            ..
        } => {
            if let Some(value) = epoch_index {
                epoch(&mut out, *value);
            }
            out.has_label = true;
            out.records_count = records.len();
        }
        ObsDowngradeChange::EventRecordsRewritten {
            epoch_index,
            from,
            to,
        } => {
            epoch(&mut out, *epoch_index);
            out.from_records_count = from.len();
            out.to_records_count = to.len();
        }
    }
    out
}

fn rinex_obs_downgrade_change_at_depth(
    mut change: &ObsDowngradeChange,
    depth: usize,
) -> Option<&ObsDowngradeChange> {
    for _ in 0..depth {
        change = match change {
            ObsDowngradeChange::InEventLists { change, .. } => change.as_ref(),
            _ => return None,
        };
    }
    Some(change)
}

fn rinex_obs_downgrade_change_text(change: &ObsDowngradeChange, field: u32) -> Option<&str> {
    match (field, change) {
        (0, ObsDowngradeChange::CodeMoved { code, .. })
        | (0, ObsDowngradeChange::CodeAdded { code, .. })
        | (0, ObsDowngradeChange::ValueRounded { code, .. })
        | (0, ObsDowngradeChange::CycleSlipRounded { code, .. }) => Some(code),
        (1, ObsDowngradeChange::CodeRenamed { from, .. }) => Some(from),
        (2, ObsDowngradeChange::CodeRenamed { to, .. }) => Some(to),
        (3, ObsDowngradeChange::DeprecatedRecordsRemoved { label, .. }) => Some(label),
        _ => None,
    }
}

fn rinex_obs_downgrade_change_list(change: &ObsDowngradeChange, list: u32) -> Option<&[String]> {
    match (list, change) {
        (0, ObsDowngradeChange::CodeListRemoved { codes, .. }) => Some(codes),
        (1, ObsDowngradeChange::DeprecatedRecordsRemoved { records, .. }) => Some(records),
        (2, ObsDowngradeChange::EventRecordsRewritten { from, .. }) => Some(from),
        (3, ObsDowngradeChange::EventRecordsRewritten { to, .. }) => Some(to),
        _ => None,
    }
}

/// Serialize a RINEX observation product back to RINEX text. The output is not
/// null-terminated. Uses the variable-length output contract documented at the
/// top of the header: call once with out=NULL to learn *out_required, then again
/// with a buffer of that size. The version the header carries decides the
/// records written: below 3.0 the file is version 2 throughout, otherwise
/// version 3. Round-trips with sidereon_rinex_obs_parse.
///
/// The text is returned only when reading it back gives the product itself.
/// Anything the file cannot state exactly -- a value its column cannot hold, an
/// epoch flag wider than its field, an observation epoch with no time,
/// picoseconds below version 4.02, a version 2 product holding what version 2
/// cannot say -- is refused with SIDEREON_STATUS_INVALID_ARGUMENT and the
/// engine's text in the thread-local message. A refusal reports a required
/// length of zero and writes nothing. sidereon_rinex_obs_to_rinex_text_result
/// returns the same refusal typed, with owned text.
///
/// Safety: obs must be a live handle from sidereon_rinex_obs_parse; out must point
/// to at least len writable bytes or be NULL when len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_to_rinex_text(
    obs: *const SidereonRinexObs,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_to_rinex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_to_rinex_text",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(obs, "sidereon_rinex_obs_to_rinex_text", "obs"));
            let text = match obs.inner.to_rinex_string() {
                Ok(text) => text,
                Err(err) => {
                    set_last_error(format!("sidereon_rinex_obs_to_rinex_text: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_to_rinex_text",
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

/// Write a RINEX observation product and take an owned record of the attempt.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonRinexObsWriteResult: the text when the product
/// was written, or the typed refusal sidereon_rinex_obs_to_rinex_text reports
/// only as text. A null argument leaves `*out_result` NULL, returns
/// SIDEREON_STATUS_NULL_POINTER and allocates nothing.
///
/// Safety: `obs` must be a live SidereonRinexObs handle; `out_result` must
/// point to writable storage for one `SidereonRinexObsWriteResult *`, which is
/// set to NULL before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_rinex_obs_write_result_free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_to_rinex_text_result(
    obs: *const SidereonRinexObs,
    out_result: *mut *mut SidereonRinexObsWriteResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_to_rinex_text_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        let result = match obs.inner.to_rinex_string() {
            Ok(text) => rinex_obs_write_result(FN_NAME, Ok(text.into_bytes())),
            Err(err) => rinex_obs_write_result(FN_NAME, Err(&err)),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Release an owned RINEX observation write result. Passing NULL is a no-op.
///
/// Safety: `result` may be NULL; otherwise it must be a live
/// SidereonRinexObsWriteResult handle this binding produced, passed here exactly
/// once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_free(
    result: *mut SidereonRinexObsWriteResult,
) {
    ffi_boundary("sidereon_rinex_obs_write_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned RINEX observation write result.
/// `*out_outcome` is written before the result pointer is validated.
///
/// Safety: `result` must be a live SidereonRinexObsWriteResult handle;
/// `out_outcome` must point to one writable SidereonRinexObsWriteOutcome.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_get_outcome(
    result: *const SidereonRinexObsWriteResult,
    out_outcome: *mut SidereonRinexObsWriteOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_write_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_outcome = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        *out_outcome = SidereonRinexObsWriteOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_rinex_obs_write_error(),
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out_outcome = result.outcome;
        SidereonStatus::Ok
    })
}

/// Copy the text of an owned RINEX observation write result. Uses the
/// variable-length output contract; the bytes are not null-terminated.
///
/// A refused result has no text: the call returns
/// SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero, writes
/// nothing, and sets the thread-local message to the result's own refusal text.
///
/// Safety: `result` must be a live SidereonRinexObsWriteResult handle; `out`
/// may be NULL only when `len` is 0; `out_written` and `out_required` must each
/// point to a writable size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_get_text(
    result: *const SidereonRinexObsWriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_write_result_get_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if !result.outcome.is_ok {
            set_last_error(result.message.clone());
            return result.outcome.status;
        }
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &result.text,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the refusal text of an owned RINEX observation write result, prefixed
/// with the route that produced it. A written result reports a required length
/// of zero. Uses the variable-length output contract.
///
/// Safety: as sidereon_rinex_obs_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_get_message(
    result: *const SidereonRinexObsWriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_write_result_get_message";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.message.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the observation code a refused write names, as the product holds it.
/// The error's has_code says whether the refusal names one; when it does not,
/// the required length is zero. Uses the variable-length output contract.
///
/// Safety: as sidereon_rinex_obs_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_get_code(
    result: *const SidereonRinexObsWriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_write_result_get_code";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.code.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the detail text a refused write carries: the reader's text for an
/// unreadable event record, the `LEAP SECONDS` time-system identifier as the
/// product holds it, or the first field that would read back changed. The
/// error's has_detail says whether the refusal carries one. Uses the
/// variable-length output contract; the bytes are copied verbatim.
///
/// Safety: as sidereon_rinex_obs_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_write_result_get_detail(
    result: *const SidereonRinexObsWriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_write_result_get_detail";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.detail.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Downgrade an observation product to a RINEX 2 version without mutating the
/// source. A well-formed call always returns an owned result: either a fresh
/// product plus every ordered change, or the complete typed write refusal.
///
/// Safety: obs must be live; out_result points to one writable result pointer,
/// initialized to NULL by this function. Free it with
/// sidereon_rinex_obs_downgrade_result_free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_to_rinex2(
    obs: *const SidereonRinexObs,
    version: f64,
    out_result: *mut *mut SidereonRinexObsDowngradeResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_to_rinex2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let obs = c_try!(require_ref(obs, FN_NAME, "obs"));
        let result = match obs.inner.downgrade_to_rinex2(version) {
            Ok((product, changes)) => SidereonRinexObsDowngradeResult {
                outcome: SidereonRinexObsDowngradeOutcome {
                    is_ok: true,
                    status: SidereonStatus::Ok,
                    error: no_rinex_obs_write_error(),
                    change_count: changes.len(),
                },
                product: Some(Box::new(SidereonRinexObs { inner: product })),
                changes,
                message: String::new(),
                code: String::new(),
                detail: String::new(),
            },
            Err(error) => {
                let (typed, code, detail) = rinex_obs_write_refusal(&error);
                SidereonRinexObsDowngradeResult {
                    outcome: SidereonRinexObsDowngradeOutcome {
                        is_ok: false,
                        status: SidereonStatus::InvalidArgument,
                        error: typed,
                        change_count: 0,
                    },
                    product: None,
                    changes: Vec::new(),
                    message: format!("{FN_NAME}: {error}"),
                    code: code.unwrap_or_default(),
                    detail: detail.unwrap_or_default(),
                }
            }
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Release a downgrade result. Any fresh product not yet taken is released too.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_free(
    result: *mut SidereonRinexObsDowngradeResult,
) {
    ffi_boundary("sidereon_rinex_obs_downgrade_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of a downgrade attempt.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_outcome(
    result: *const SidereonRinexObsDowngradeResult,
    out_outcome: *mut SidereonRinexObsDowngradeOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_outcome = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        *out_outcome = SidereonRinexObsDowngradeOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_rinex_obs_write_error(),
            change_count: 0,
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out_outcome = result.outcome;
        SidereonStatus::Ok
    })
}

/// Transfer the fresh downgraded product out of a successful result exactly
/// once. The product remains valid after the result and source are freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_take_obs(
    result: *mut SidereonRinexObsDowngradeResult,
    out_obs: *mut *mut SidereonRinexObs,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_result_take_obs";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_obs = c_try!(require_out(out_obs, FN_NAME, "out_obs"));
        *out_obs = ptr::null_mut();
        let result = c_try!(require_mut(result, FN_NAME, "result"));
        if !result.outcome.is_ok {
            set_last_error(result.message.clone());
            return result.outcome.status;
        }
        let Some(product) = result.product.take() else {
            set_last_error(format!("{FN_NAME}: downgraded product was already taken"));
            return SidereonStatus::InvalidArgument;
        };
        *out_obs = Box::into_raw(product);
        SidereonStatus::Ok
    })
}

/// Copy one fixed-width change descriptor. `depth=0` selects an ordered
/// top-level change; each greater depth follows one InEventLists nested change.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_change(
    result: *const SidereonRinexObsDowngradeResult,
    change_index: usize,
    depth: usize,
    out_change: *mut SidereonRinexObsDowngradeChange,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_result_get_change";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_change = c_try!(require_out(out_change, FN_NAME, "out_change"));
        *out_change =
            empty_rinex_obs_downgrade_change(SidereonRinexObsDowngradeChangeKind::CodeRenamed);
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let Some(change) = result
            .changes
            .get(change_index)
            .and_then(|change| rinex_obs_downgrade_change_at_depth(change, depth))
        else {
            set_last_error(format!(
                "{FN_NAME}: no change at index {change_index}, depth {depth}"
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_change = rinex_obs_downgrade_change_to_c(change);
        SidereonStatus::Ok
    })
}

/// Copy one scalar text payload from a downgrade change. field is a
/// SidereonRinexObsDowngradeTextField value. An absent payload is an empty
/// successful output; invalid field values are rejected.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_change_text(
    result: *const SidereonRinexObsDowngradeResult,
    change_index: usize,
    depth: usize,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_result_get_change_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        if field > SidereonRinexObsDowngradeTextField::Label as u32 {
            set_last_error(format!("{FN_NAME}: invalid text field {field}"));
            return SidereonStatus::InvalidArgument;
        }
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let Some(change) = result
            .changes
            .get(change_index)
            .and_then(|change| rinex_obs_downgrade_change_at_depth(change, depth))
        else {
            set_last_error(format!(
                "{FN_NAME}: no change at index {change_index}, depth {depth}"
            ));
            return SidereonStatus::InvalidArgument;
        };
        let text = rinex_obs_downgrade_change_text(change, field).unwrap_or("");
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

/// Copy one string from a vector payload. list is a
/// SidereonRinexObsDowngradeStringList value; item_index must be in the count
/// reported by the fixed descriptor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_change_list_item(
    result: *const SidereonRinexObsDowngradeResult,
    change_index: usize,
    depth: usize,
    list: u32,
    item_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_obs_downgrade_result_get_change_list_item";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        if list > SidereonRinexObsDowngradeStringList::ToRecords as u32 {
            set_last_error(format!("{FN_NAME}: invalid string list {list}"));
            return SidereonStatus::InvalidArgument;
        }
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let Some(change) = result
            .changes
            .get(change_index)
            .and_then(|change| rinex_obs_downgrade_change_at_depth(change, depth))
        else {
            set_last_error(format!(
                "{FN_NAME}: no change at index {change_index}, depth {depth}"
            ));
            return SidereonStatus::InvalidArgument;
        };
        let Some(text) =
            rinex_obs_downgrade_change_list(change, list).and_then(|values| values.get(item_index))
        else {
            set_last_error(format!(
                "{FN_NAME}: no list item {item_index} for change {change_index}, depth {depth}, list {list}"
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

unsafe fn copy_rinex_obs_downgrade_result_text(
    fn_name: &str,
    result: *const SidereonRinexObsDowngradeResult,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let result = c_try!(require_ref(result, fn_name, "result"));
        let text = match field {
            0 => &result.message,
            1 => &result.code,
            2 => &result.detail,
            _ => unreachable!(),
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

/// Copy the route-prefixed typed refusal message, or empty text on success.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_message(
    result: *const SidereonRinexObsDowngradeResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    copy_rinex_obs_downgrade_result_text(
        "sidereon_rinex_obs_downgrade_result_get_message",
        result,
        0,
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy the observation-code payload of a typed refusal, or empty text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_code(
    result: *const SidereonRinexObsDowngradeResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    copy_rinex_obs_downgrade_result_text(
        "sidereon_rinex_obs_downgrade_result_get_code",
        result,
        1,
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy the detail payload of a typed refusal, or empty text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_downgrade_result_get_detail(
    result: *const SidereonRinexObsDowngradeResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    copy_rinex_obs_downgrade_result_text(
        "sidereon_rinex_obs_downgrade_result_get_detail",
        result,
        2,
        out,
        len,
        out_written,
        out_required,
    )
}

/// Release a RINEX observation handle from sidereon_rinex_obs_parse. Passing NULL
/// is a no-op.
///
/// Safety: obs must be NULL or a live handle from sidereon_rinex_obs_parse that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_free(obs: *mut SidereonRinexObs) {
    ffi_boundary("sidereon_rinex_obs_free", (), || {
        free_boxed(obs);
    });
}

/// Extract RINEX receiver-clock offsets as phase deviations in seconds. Event
/// epochs are returned with `has_phase_s == false`.
///
/// Safety: obs must be a live RINEX OBS handle; out points to len
/// SidereonClockPhaseSample entries or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_obs_receiver_clock_phase_deviations(
    obs: *const SidereonRinexObs,
    out: *mut SidereonClockPhaseSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_obs_receiver_clock_phase_deviations",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
                out_written,
                out_required
            ));
            let obs = c_try!(require_ref(
                obs,
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
                "obs"
            ));
            let values: Vec<SidereonClockPhaseSample> =
                core_receiver_clock_phase_deviations(&obs.inner)
                    .into_iter()
                    .map(|value| SidereonClockPhaseSample {
                        has_phase_s: value.is_some(),
                        phase_s: value.unwrap_or(0.0),
                    })
                    .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_obs_receiver_clock_phase_deviations",
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

// === GNSS constellation identity catalog (CelesTrak + NAVCEN) ==============
//
// Wraps sidereon_core::constellation: build a merged GPS identity catalog from
// CelesTrak gps-ops OMM/JSON and an optional NAVCEN status overlay, export the
// compact mapping CSV, and validate the catalog against a list of SP3/RINEX
// satellite ids. The catalog and the validation report are opaque handles whose
// fields are read back through accessor functions using the variable-length
// output contract documented at the top of the header.

// --- RINEX NAV auxiliary and raw-record routes -------------------------------

/// A lenient RINEX NAV parse result. The owned handle retains both successfully
/// parsed records and the core parser's skipped block diagnostics.
pub struct SidereonRinexNavParse {
    pub(crate) inner: sidereon_core::rinex::nav::NavParse,
}

/// One diagnostic for a NAV block skipped by the lenient parser.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSkippedNavBlock {
    /// Satellite token from the skipped block.
    pub satellite: SidereonSatelliteToken,
    /// Null-terminated core diagnostic text from the skipped block.
    pub message: [c_char; 256],
}

/// Owned list of full, pre-filter RINEX NAV broadcast records.
pub struct SidereonRinexNavRecords {
    pub(crate) records: Vec<sidereon_core::rinex::nav::BroadcastRecord>,
}

/// Owned list of parsed GLONASS RINEX state-vector records and separately
/// inspectable skipped-token diagnostics. Every slot token `R01`..`R99` is read,
/// the extended slots `R28` and up included.
pub struct SidereonRinexGlonassRecords {
    pub(crate) records: Vec<sidereon_core::rinex::nav::GlonassRecord>,
    pub(crate) skipped: Vec<sidereon_core::rinex::nav::SkippedGlonass>,
}

/// One GLONASS RINEX record skipped because its satellite token names no
/// satellite: a slot outside `01`..`99` such as `R00`, or a malformed slot
/// field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSkippedGlonassRecord {
    /// Raw satellite token from the skipped input record, such as `R00`.
    pub satellite: SidereonSatelliteToken,
}

/// Parse a RINEX NAV source while retaining core skipped-block diagnostics.
/// Header failures remain errors; malformed supported body blocks are retained
/// in the returned `SidereonRinexNavParse` diagnostics.
///
/// Safety: data points to len readable bytes; out_parse points to an owned
/// SidereonRinexNavParse handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_nav_lenient(
    data: *const u8,
    len: usize,
    out_parse: *mut *mut SidereonRinexNavParse,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_nav_lenient",
        SidereonStatus::Panic,
        || {
            let out_parse = c_try!(require_out(
                out_parse,
                "sidereon_parse_rinex_nav_lenient",
                "out_parse"
            ));
            *out_parse = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_nav_lenient",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_nav_lenient: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let inner = match sidereon_core::rinex::nav::parse_nav_lenient(text) {
                Ok(inner) => inner,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_nav_lenient: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_parse, SidereonRinexNavParse { inner });
            SidereonStatus::Ok
        },
    )
}

/// Release a lenient NAV parse handle. Passing NULL is a no-op.
///
/// Safety: parse is NULL or a live handle from
/// sidereon_parse_rinex_nav_lenient.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_free(parse: *mut SidereonRinexNavParse) {
    free_boxed(parse);
}

/// Write the number of successfully parsed records in a lenient NAV result.
///
/// Safety: parse is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_record_count(
    parse: *const SidereonRinexNavParse,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_record_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_nav_parse_record_count",
                "out_count"
            ));
            *out = 0;
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_record_count",
                "parse"
            ));
            *out = parse.inner.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one full successfully parsed NAV record by deterministic file-order
/// index.
///
/// Safety: parse is a live handle; out_record points to a writable full record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_record(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out_record: *mut SidereonBroadcastRecord,
) -> SidereonStatus {
    ffi_boundary("sidereon_nav_parse_record", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_record,
            "sidereon_nav_parse_record",
            "out_record"
        ));
        *out = empty_broadcast_record();
        let parse = c_try!(require_ref(parse, "sidereon_nav_parse_record", "parse"));
        let Some(record) = parse.inner.records.get(index) else {
            set_last_error(format!(
                "sidereon_nav_parse_record: index {index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = broadcast_record_to_c_full(record);
        SidereonStatus::Ok
    })
}

/// Write the number of skipped NAV block diagnostics in a lenient result.
///
/// Safety: parse is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped_count(
    parse: *const SidereonRinexNavParse,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_skipped_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_nav_parse_skipped_count",
                "out_count"
            ));
            *out = 0;
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_skipped_count",
                "parse"
            ));
            *out = parse.inner.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped NAV block diagnostic by deterministic file-order index.
///
/// Safety: parse is a live handle; out_skipped points to a writable diagnostic.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out_skipped: *mut SidereonSkippedNavBlock,
) -> SidereonStatus {
    ffi_boundary("sidereon_nav_parse_skipped", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_skipped,
            "sidereon_nav_parse_skipped",
            "out_skipped"
        ));
        *out = SidereonSkippedNavBlock {
            satellite: satellite_token_from_text(""),
            message: [0; 256],
        };
        let parse = c_try!(require_ref(parse, "sidereon_nav_parse_skipped", "parse"));
        let Some(skipped) = parse.inner.skipped.get(index) else {
            set_last_error(format!(
                "sidereon_nav_parse_skipped: index {index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = SidereonSkippedNavBlock {
            satellite: satellite_token_from_text(&skipped.satellite),
            message: fixed_c_chars::<256>(&skipped.message),
        };
        SidereonStatus::Ok
    })
}

/// Copy the complete core diagnostic text for one skipped NAV block. This
/// variable-length accessor complements the bounded C display field in
/// `SidereonSkippedNavBlock`.
///
/// Safety: parse is a live handle; out points to len writable bytes or is NULL
/// when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nav_parse_skipped_message(
    parse: *const SidereonRinexNavParse,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nav_parse_skipped_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_nav_parse_skipped_message",
                out_written,
                out_required
            ));
            let parse = c_try!(require_ref(
                parse,
                "sidereon_nav_parse_skipped_message",
                "parse"
            ));
            let Some(skipped) = parse.inner.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_nav_parse_skipped_message: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_nav_parse_skipped_message",
                "out",
                skipped.message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Parse all supported raw RINEX NAV records before the broadcast-store
/// health/message policy filter.
///
/// Safety: data points to len readable bytes; out_records points to an owned
/// list handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_nav_records(
    data: *const u8,
    len: usize,
    out_records: *mut *mut SidereonRinexNavRecords,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_nav_records",
        SidereonStatus::Panic,
        || {
            let out_records = c_try!(require_out(
                out_records,
                "sidereon_parse_rinex_nav_records",
                "out_records"
            ));
            *out_records = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_nav_records",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_nav_records: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let records = match sidereon_core::rinex::nav::parse_nav(text) {
                Ok(records) => records,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_nav_records: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(out_records, SidereonRinexNavRecords { records });
            SidereonStatus::Ok
        },
    )
}

/// Release a raw NAV record-list handle. Passing NULL is a no-op.
///
/// Safety: records is NULL or a live handle returned by
/// sidereon_parse_rinex_nav_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_free(records: *mut SidereonRinexNavRecords) {
    free_boxed(records);
}

/// Write the number of records in a raw NAV record list.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_count(
    records: *const SidereonRinexNavRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_nav_records_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_nav_records_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_nav_records_count",
                "records"
            ));
            *out = records.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one raw full NAV record by deterministic file-order index.
///
/// Safety: records is a live handle; out_record points to a writable full
/// record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_nav_records_item(
    records: *const SidereonRinexNavRecords,
    index: usize,
    out_record: *mut SidereonBroadcastRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_nav_records_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_record,
                "sidereon_rinex_nav_records_item",
                "out_record"
            ));
            *out = empty_broadcast_record();
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_nav_records_item",
                "records"
            ));
            let Some(record) = records.records.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_nav_records_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = broadcast_record_to_c_full(record);
            SidereonStatus::Ok
        },
    )
}

/// Encode an arbitrary caller-supplied full NAV record list. The list is
/// validated by the public BroadcastStore constructor before delegation to the
/// core `encode_nav` writer, preventing an invalid CNAV record from reaching a
/// panic-only encoder path.
///
/// Safety: records points to `record_count` readable full records (or is NULL
/// when the count is zero); out follows the standard variable-output contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_encode_rinex_nav(
    records: *const SidereonBroadcastRecord,
    record_count: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_encode_rinex_nav", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_encode_rinex_nav",
            out_written,
            out_required
        ));
        let records = c_try!(require_slice(
            records,
            record_count,
            "sidereon_encode_rinex_nav",
            "records"
        ));
        let mut core_records = Vec::with_capacity(record_count);
        for record in records {
            core_records.push(c_try!(broadcast_record_from_c(
                "sidereon_encode_rinex_nav",
                record
            )));
        }
        if let Err(err) = BroadcastEphemeris::new(core_records.clone()) {
            set_last_error(format!("sidereon_encode_rinex_nav: {err}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = match sidereon_core::rinex::nav::encode_nav(&core_records) {
            Ok(text) => text,
            Err(err) => {
                set_last_error(format!("sidereon_encode_rinex_nav: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_encode_rinex_nav",
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

/// Parse RINEX GLONASS state-vector records into an owned list. Representable
/// records are retained and malformed representable records fail the parse.
/// Extended slots that cannot be represented by the core satellite identifier
/// are skipped, with their raw satellite tokens retained for inspection by
/// `sidereon_rinex_glonass_records_skipped_count` and
/// `sidereon_rinex_glonass_records_skipped_item`.
///
/// Safety: data points to len readable UTF-8 bytes; out_records points to an
/// owned list handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_glonass_records(
    data: *const u8,
    len: usize,
    out_records: *mut *mut SidereonRinexGlonassRecords,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_glonass_records",
        SidereonStatus::Panic,
        || {
            let out_records = c_try!(require_out(
                out_records,
                "sidereon_parse_rinex_glonass_records",
                "out_records"
            ));
            *out_records = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_glonass_records",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_glonass_records: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let parsed = match sidereon_core::rinex::nav::parse_glonass_lenient(text) {
                Ok(parsed) => parsed,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_glonass_records: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            write_boxed_handle(
                out_records,
                SidereonRinexGlonassRecords {
                    records: parsed.records,
                    skipped: parsed.skipped,
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a GLONASS record-list handle. Passing NULL is a no-op.
///
/// Safety: records is NULL or a live handle returned by
/// sidereon_parse_rinex_glonass_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_free(
    records: *mut SidereonRinexGlonassRecords,
) {
    free_boxed(records);
}

/// Write the number of representable parsed GLONASS records.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_count(
    records: *const SidereonRinexGlonassRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_glonass_records_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_count",
                "records"
            ));
            *out = records.records.len();
            SidereonStatus::Ok
        },
    )
}

/// Write the number of GLONASS records skipped because their satellite token
/// names no satellite.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_skipped_count(
    records: *const SidereonRinexGlonassRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_skipped_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rinex_glonass_records_skipped_count",
                "out_count"
            ));
            *out = 0;
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_skipped_count",
                "records"
            ));
            *out = records.skipped.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one parsed GLONASS record by deterministic file-order index.
///
/// Safety: records is a live handle; out_record points to a writable record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_item(
    records: *const SidereonRinexGlonassRecords,
    index: usize,
    out_record: *mut SidereonGlonassRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_record,
                "sidereon_rinex_glonass_records_item",
                "out_record"
            ));
            *out = empty_glonass_record();
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_item",
                "records"
            ));
            let Some(record) = records.records.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_glonass_records_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = glonass_record_to_c(record);
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped GLONASS record by deterministic file-order index. The
/// returned satellite token is the raw token from the input record, such as
/// `R00`.
///
/// Safety: records is a live handle; out_skipped points to a writable value.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_glonass_records_skipped_item(
    records: *const SidereonRinexGlonassRecords,
    index: usize,
    out_skipped: *mut SidereonSkippedGlonassRecord,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_glonass_records_skipped_item",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_skipped,
                "sidereon_rinex_glonass_records_skipped_item",
                "out_skipped"
            ));
            *out = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(""),
            };
            let records = c_try!(require_ref(
                records,
                "sidereon_rinex_glonass_records_skipped_item",
                "records"
            ));
            let Some(skipped) = records.skipped.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_glonass_records_skipped_item: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(&skipped.token),
            };
            SidereonStatus::Ok
        },
    )
}

/// Parse RINEX NAV-header ionosphere corrections into a fixed, presence-tagged
/// value. Empty headers are valid and return all presence flags false.
///
/// Safety: data points to len readable UTF-8 bytes; out points to a writable
/// SidereonIonoCorrections.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_iono_corrections(
    data: *const u8,
    len: usize,
    out: *mut SidereonIonoCorrections,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_iono_corrections",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_parse_rinex_iono_corrections",
                "out"
            ));
            *out = empty_iono_corrections();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_iono_corrections",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_iono_corrections: data is not valid UTF-8"
                            .to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let iono = match sidereon_core::rinex::nav::parse_iono_corrections(text) {
                Ok(iono) => iono,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_iono_corrections: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = iono_corrections_to_c(&iono);
            SidereonStatus::Ok
        },
    )
}

/// Parse the optional RINEX NAV-header GPS-minus-UTC leap-second value.
/// `out_present` distinguishes an absent header from a present zero value.
///
/// Safety: data points to len readable UTF-8 bytes; output pointers point to
/// writable values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_parse_rinex_leap_seconds(
    data: *const u8,
    len: usize,
    out_leap_seconds: *mut f64,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_parse_rinex_leap_seconds",
        SidereonStatus::Panic,
        || {
            let out_value = c_try!(require_out(
                out_leap_seconds,
                "sidereon_parse_rinex_leap_seconds",
                "out_leap_seconds"
            ));
            *out_value = 0.0;
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_parse_rinex_leap_seconds",
                "out_present"
            ));
            *out_present = false;
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_parse_rinex_leap_seconds",
                "data"
            ));
            let text = match str::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => {
                    set_last_error(
                        "sidereon_parse_rinex_leap_seconds: data is not valid UTF-8".to_string(),
                    );
                    return SidereonStatus::InvalidToken;
                }
            };
            let value = match sidereon_core::rinex::nav::parse_leap_seconds(text) {
                Ok(value) => value,
                Err(err) => {
                    set_last_error(format!("sidereon_parse_rinex_leap_seconds: {err}"));
                    return SidereonStatus::InvalidArgument;
                }
            };
            if let Some(value) = value {
                *out_value = value;
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

// --- RINEX navigation serialize (sidereon_core::rinex_nav) -------------------

/// Serialize a parsed broadcast-ephemeris store back to RINEX navigation text.
/// The store's records are written via
/// sidereon_core::rinex_nav::encode_nav. Uses the variable-length output
/// contract: pass out=NULL/len=0 to size the buffer (out_required), then call
/// again with a buffer of at least out_required bytes.
///
/// Safety: eph is a live broadcast handle; out points to len bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_encode_nav(
    eph: *const SidereonBroadcastEphemeris,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_encode_nav", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_encode_nav",
            out_written,
            out_required
        ));
        let eph = c_try!(require_ref(eph, "sidereon_rinex_encode_nav", "eph"));
        let text = match sidereon_core::rinex::nav::encode_nav(eph.inner.records()) {
            Ok(text) => text,
            Err(err) => {
                set_last_error(format!("sidereon_rinex_encode_nav: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_encode_nav",
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

// === Round-2 RINEX QC, lint, and repair =====================================

pub const RINEX_QC_CODE_C_BYTES: usize = 16;

pub const RINEX_QC_FIELD_C_BYTES: usize = 65;

pub struct SidereonRinexLintReport {
    pub(crate) inner: sidereon_core::rinex::qc::LintReport,
}

/// A RINEX lint repair. Opaque to C. Create with sidereon_rinex_repair_obs or
/// sidereon_rinex_repair_nav and release with sidereon_rinex_repair_free.
pub struct SidereonRinexRepair {
    /// The repaired text, or the refusal writing the repaired observation
    /// product gave. The actions and the remaining lint stay readable either
    /// way.
    pub(crate) text: Result<Vec<u8>, RinexObsWriteError>,
    /// The repaired observation product as CRINEX, or the text of the refusal
    /// to write it; `None` for a navigation repair, which has no CRINEX form.
    pub(crate) crinex_text: Option<Result<Vec<u8>, String>>,
    pub(crate) actions: Vec<sidereon_core::rinex::qc::RepairAction>,
    pub(crate) remaining: sidereon_core::rinex::qc::LintReport,
    pub(crate) decoded_from_crinex: bool,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexQcSeverity {
    Fatal = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexLintSummary {
    pub finding_count: usize,
    pub fatal_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub is_clean: bool,
    pub decoded_from_crinex: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexLintFinding {
    pub code: [c_char; RINEX_QC_CODE_C_BYTES],
    pub severity: u32,
    pub repairable: bool,
    pub has_epoch_index: bool,
    pub epoch_index: usize,
    pub has_satellite: bool,
    pub satellite: SidereonSatelliteToken,
    pub has_field: bool,
    pub field: [c_char; RINEX_QC_FIELD_C_BYTES],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexRepairOptions {
    pub has_file_stamp: bool,
    pub file_stamp_program: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub file_stamp_run_by: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub file_stamp_date: [c_char; RINEX_QC_FIELD_C_BYTES],
    pub set_interval: bool,
    pub set_time_of_last_obs: bool,
    pub set_obs_counts: bool,
    pub drop_empty_records: bool,
    pub sort_records: bool,
    pub drop_unsupported: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexRepairAction {
    pub id: [c_char; RINEX_QC_CODE_C_BYTES],
    pub message: [c_char; RINEX_QC_FIELD_C_BYTES],
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_obs(
    data: *const u8,
    len: usize,
    out_report: *mut *mut SidereonRinexLintReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_obs", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_rinex_lint_obs",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_lint_obs", data, len));
        let inner = sidereon_core::rinex::qc::lint_obs_text(text);
        write_boxed_handle(out_report, SidereonRinexLintReport { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_nav(
    data: *const u8,
    len: usize,
    out_report: *mut *mut SidereonRinexLintReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_nav", SidereonStatus::Panic, || {
        let out_report = c_try!(require_out(
            out_report,
            "sidereon_rinex_lint_nav",
            "out_report"
        ));
        *out_report = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_lint_nav", data, len));
        let inner = sidereon_core::rinex::qc::lint_nav_text(text);
        write_boxed_handle(out_report, SidereonRinexLintReport { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_summary(
    report: *const SidereonRinexLintReport,
    out_summary: *mut SidereonRinexLintSummary,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_lint_summary", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_summary,
            "sidereon_rinex_lint_summary",
            "out_summary"
        ));
        let report = c_try!(require_ref(report, "sidereon_rinex_lint_summary", "report"));
        *out = rinex_lint_summary_to_c(&report.inner);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_findings(
    report: *const SidereonRinexLintReport,
    out: *mut SidereonRinexLintFinding,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_lint_findings",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_lint_findings",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_rinex_lint_findings",
                "report"
            ));
            let values: Vec<_> = report
                .inner
                .findings
                .iter()
                .map(rinex_lint_finding_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_lint_findings",
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

/// Copy the stable JSON detail payload for one lint finding.
///
/// The report must be live. `out` may be null only when `len` is zero to query
/// the required byte count. The output is UTF-8 JSON without a trailing NUL and
/// has `kind`, `spec_ref`, and `details` fields. `kind` is the PascalCase core
/// variant name; `spec_ref` is its standards reference; `details` contains the
/// variant-specific payload. Optional values are JSON `null`, while counts are
/// exact JSON integers. Finite floating-point values are JSON numbers; NaN and
/// positive or negative infinity are the strings `NaN`, `Infinity`, or
/// `-Infinity` so they are not lost as JSON null.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_finding_details_json(
    report: *const SidereonRinexLintReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_lint_finding_details_json",
        SidereonStatus::Panic,
        || {
            let report = c_try!(require_ref(
                report,
                "sidereon_rinex_lint_finding_details_json",
                "report"
            ));
            let Some(finding) = report.inner.findings.get(index) else {
                set_last_error(format!(
                    "sidereon_rinex_lint_finding_details_json: finding index {index} is out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            let payload = match serde_json::to_vec(&rinex_lint_finding_detail_json(finding)) {
                Ok(payload) => payload,
                Err(error) => {
                    set_last_error(format!(
                        "sidereon_rinex_lint_finding_details_json: JSON serialization failed: {error}"
                    ));
                    return SidereonStatus::Panic;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_lint_finding_details_json",
                "out",
                &payload,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_lint_report_free(report: *mut SidereonRinexLintReport) {
    free_boxed(report);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_options_init(
    out_options: *mut SidereonRinexRepairOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_options_init",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_options,
                "sidereon_rinex_repair_options_init",
                "out_options"
            ));
            let defaults = sidereon_core::rinex::qc::RepairOptions::default();
            *out = SidereonRinexRepairOptions {
                has_file_stamp: false,
                file_stamp_program: [0; RINEX_QC_FIELD_C_BYTES],
                file_stamp_run_by: [0; RINEX_QC_FIELD_C_BYTES],
                file_stamp_date: [0; RINEX_QC_FIELD_C_BYTES],
                set_interval: defaults.set_interval,
                set_time_of_last_obs: defaults.set_time_of_last_obs,
                set_obs_counts: defaults.set_obs_counts,
                drop_empty_records: defaults.drop_empty_records,
                sort_records: defaults.sort_records,
                drop_unsupported: defaults.drop_unsupported,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_obs(
    data: *const u8,
    len: usize,
    options: *const SidereonRinexRepairOptions,
    out_repair: *mut *mut SidereonRinexRepair,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_obs", SidereonStatus::Panic, || {
        let out_repair = c_try!(require_out(
            out_repair,
            "sidereon_rinex_repair_obs",
            "out_repair"
        ));
        *out_repair = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_repair_obs", data, len));
        let options = c_try!(repair_options_from_c("sidereon_rinex_repair_obs", options));
        match sidereon_core::rinex::qc::repair_obs_text(text, &options) {
            Ok(repair) => {
                let crinex_text = Some(
                    sidereon_core::rinex::qc::repair_obs_to_crinex_string(&repair)
                        .map(String::into_bytes)
                        .map_err(|err| err.to_string()),
                );
                let text = repair.repaired.to_rinex_string().map(String::into_bytes);
                write_boxed_handle(
                    out_repair,
                    SidereonRinexRepair {
                        text,
                        crinex_text,
                        actions: repair.actions,
                        remaining: repair.remaining,
                        decoded_from_crinex: repair.decoded_from_crinex,
                    },
                );
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("sidereon_rinex_repair_obs: {err}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_nav(
    data: *const u8,
    len: usize,
    options: *const SidereonRinexRepairOptions,
    out_repair: *mut *mut SidereonRinexRepair,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_nav", SidereonStatus::Panic, || {
        let out_repair = c_try!(require_out(
            out_repair,
            "sidereon_rinex_repair_nav",
            "out_repair"
        ));
        *out_repair = ptr::null_mut();
        let text = c_try!(text_bytes_from_c("sidereon_rinex_repair_nav", data, len));
        let options = c_try!(repair_options_from_c("sidereon_rinex_repair_nav", options));
        match sidereon_core::rinex::qc::repair_nav_text(text, &options) {
            Ok(repair) => {
                let text = match sidereon_core::rinex::nav::encode_nav(&repair.records) {
                    Ok(text) => text.into_bytes(),
                    Err(err) => {
                        set_last_error(format!("sidereon_rinex_repair_nav: {err}"));
                        return SidereonStatus::InvalidArgument;
                    }
                };
                write_boxed_handle(
                    out_repair,
                    SidereonRinexRepair {
                        text: Ok(text),
                        crinex_text: None,
                        actions: repair.actions,
                        remaining: repair.remaining,
                        decoded_from_crinex: false,
                    },
                );
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("sidereon_rinex_repair_nav: {err}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Copy the repaired text. Uses the variable-length output contract; the bytes
/// are not null-terminated.
///
/// A repaired observation product is written through the same exact writer as
/// sidereon_rinex_obs_to_rinex_text. When that writer refuses it, the repair
/// handle still holds its actions and remaining lint, and this call returns
/// SIDEREON_STATUS_INVALID_ARGUMENT with the refusal in the thread-local
/// message, reports a required length of zero and writes nothing.
/// sidereon_rinex_repair_text_result returns the same refusal typed.
///
/// Safety: repair must be a live repair handle; out must point to at least len
/// writable bytes or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_text(
    repair: *const SidereonRinexRepair,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rinex_repair_text", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rinex_repair_text",
            out_written,
            out_required
        ));
        let repair = c_try!(require_ref(repair, "sidereon_rinex_repair_text", "repair"));
        let text = match &repair.text {
            Ok(text) => text,
            Err(err) => {
                set_last_error(format!(
                    "sidereon_rinex_repair_text: the repaired product cannot be written: {err}"
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_rinex_repair_text",
            "out",
            text,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Take an owned record of the repaired text: the text, or the typed refusal
/// writing the repaired observation product gave. A navigation repair's text
/// is always present. Read the result with the
/// sidereon_rinex_obs_write_result_* accessors.
///
/// Safety: `repair` must be a live repair handle; `out_result` must point to
/// writable storage for one `SidereonRinexObsWriteResult *`, which is set to
/// NULL before any work and receives a newly owned handle only on
/// SIDEREON_STATUS_OK; release it with sidereon_rinex_obs_write_result_free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_text_result(
    repair: *const SidereonRinexRepair,
    out_result: *mut *mut SidereonRinexObsWriteResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_repair_text_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let repair = c_try!(require_ref(repair, FN_NAME, "repair"));
        let result = match &repair.text {
            Ok(text) => rinex_obs_write_result(FN_NAME, Ok(text.clone())),
            Err(err) => rinex_obs_write_result(FN_NAME, Err(err)),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Copy the repaired observation product as CRINEX. Uses the variable-length
/// output contract; the bytes are not null-terminated.
///
/// A navigation repair has no CRINEX form, and a repaired observation product
/// that cannot be written as CRINEX is refused; both return
/// SIDEREON_STATUS_INVALID_ARGUMENT with the reason in the thread-local message
/// and write nothing.
///
/// Safety: repair must be a live repair handle; out must point to at least len
/// writable bytes or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_crinex_text(
    repair: *const SidereonRinexRepair,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_crinex_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_repair_crinex_text",
                out_written,
                out_required
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_crinex_text",
                "repair"
            ));
            let text = match repair.crinex_text.as_ref() {
                Some(Ok(text)) => text,
                Some(Err(message)) => {
                    set_last_error(format!(
                        "sidereon_rinex_repair_crinex_text: the repaired product cannot be \
                         written as CRINEX: {message}"
                    ));
                    return SidereonStatus::InvalidArgument;
                }
                None => {
                    set_last_error(
                        "sidereon_rinex_repair_crinex_text: a navigation repair has no CRINEX \
                         output",
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_repair_crinex_text",
                "out",
                text,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_summary(
    repair: *const SidereonRinexRepair,
    out_summary: *mut SidereonRinexLintSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_rinex_repair_summary",
                "out_summary"
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_summary",
                "repair"
            ));
            *out = rinex_lint_summary_to_c(&repair.remaining);
            (*out).decoded_from_crinex = repair.decoded_from_crinex;
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_actions(
    repair: *const SidereonRinexRepair,
    out: *mut SidereonRinexRepairAction,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rinex_repair_actions",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rinex_repair_actions",
                out_written,
                out_required
            ));
            let repair = c_try!(require_ref(
                repair,
                "sidereon_rinex_repair_actions",
                "repair"
            ));
            let values: Vec<_> = repair
                .actions
                .iter()
                .map(|action| SidereonRinexRepairAction {
                    id: fixed_c_chars::<RINEX_QC_CODE_C_BYTES>(action.id),
                    message: fixed_c_chars::<RINEX_QC_FIELD_C_BYTES>(&action.message),
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rinex_repair_actions",
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_repair_free(repair: *mut SidereonRinexRepair) {
    free_boxed(repair);
}

/// Decode a CRINEX (Hatanaka-compressed) observation byte buffer into RINEX
/// observation text. The output is not null-terminated. Uses the variable-length
/// output contract documented at the top of the header: call once with out=NULL
/// to learn *out_required, then again with a buffer of that size. The decoded
/// text is byte-for-byte what crx2rnx produces.
///
/// Safety: data must point to len readable bytes; out must point to at least
/// out_len writable bytes or be NULL when out_len is 0; out_written and
/// out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_crinex_decode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_crinex_decode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_crinex_decode",
            out_written,
            out_required
        ));
        let bytes = c_try!(require_slice(data, len, "sidereon_crinex_decode", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_crinex_decode: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let decoded = match crinex_decode(text) {
            Ok(decoded) => decoded,
            Err(err) => {
                set_last_error(format!("sidereon_crinex_decode: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_crinex_decode",
            "out",
            decoded.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

// --- CRINEX encode (sidereon_core::crinex::encode_crinex) --------------------

/// Encode RINEX observation text into CRINEX (Hatanaka-compressed) text. The
/// output is not null-terminated and is byte-for-byte what rnx2crx produces. Uses
/// the variable-length output contract. Delegates to
/// sidereon_core::crinex::encode_crinex.
///
/// Safety: data points to len readable bytes; out points to at least out_len
/// writable bytes or NULL when out_len is 0; out_written and out_required point to
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_crinex_encode(
    data: *const u8,
    len: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_crinex_encode", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_crinex_encode",
            out_written,
            out_required
        ));
        let bytes = c_try!(require_slice(data, len, "sidereon_crinex_encode", "data"));
        let text = match str::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                set_last_error("sidereon_crinex_encode: data is not valid UTF-8".to_string());
                return SidereonStatus::InvalidToken;
            }
        };
        let encoded = match sidereon_core::rinex::crinex::encode_crinex(text) {
            Ok(encoded) => encoded,
            Err(err) => {
                set_last_error(format!("sidereon_crinex_encode: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_crinex_encode",
            "out",
            encoded.as_bytes(),
            out,
            out_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn rinex_obs_kind_to_c(kind: RinexObservationKind) -> u32 {
    match kind {
        RinexObservationKind::Pseudorange => SidereonRinexObsKind::Pseudorange as u32,
        RinexObservationKind::CarrierPhase => SidereonRinexObsKind::CarrierPhase as u32,
        RinexObservationKind::Doppler => SidereonRinexObsKind::Doppler as u32,
        RinexObservationKind::SignalStrength => SidereonRinexObsKind::SignalStrength as u32,
        RinexObservationKind::Unknown => SidereonRinexObsKind::Unknown as u32,
    }
}

fn rinex_obs_code_to_c(code: &str) -> [c_char; RINEX_OBS_CODE_C_BYTES] {
    fixed_c_chars::<RINEX_OBS_CODE_C_BYTES>(code)
}

fn empty_rinex_obs_header() -> SidereonRinexObsHeader {
    SidereonRinexObsHeader {
        version: 0.0,
        has_approx_position_m: false,
        approx_position_m: [0.0; 3],
        has_antenna_delta_hen_m: false,
        antenna_delta_hen_m: [0.0; 3],
        has_interval_s: false,
        interval_s: 0.0,
        has_time_of_first_obs: false,
        time_of_first_obs: SidereonCalendarEpoch {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        },
        time_of_first_obs_scale: SidereonTimeScale::Utc as u32,
        obs_code_count: 0,
        phase_shift_count: 0,
        scale_factor_count: 0,
        glonass_slot_count: 0,
        has_marker_name: false,
        marker_name: [0; RINEX_OBS_MARKER_C_BYTES],
    }
}

fn rinex_obs_header_to_c(header: &ObsHeader) -> SidereonRinexObsHeader {
    let mut out = empty_rinex_obs_header();
    out.version = header.version;
    if let Some(position) = header.approx_position_m {
        out.has_approx_position_m = true;
        out.approx_position_m = position;
    }
    if let Some(delta) = header.antenna_delta_hen_m {
        out.has_antenna_delta_hen_m = true;
        out.antenna_delta_hen_m = delta;
    }
    if let Some(interval_s) = header.interval_s {
        out.has_interval_s = true;
        out.interval_s = interval_s;
    }
    if let Some((epoch, scale)) = header.time_of_first_obs {
        out.has_time_of_first_obs = true;
        out.time_of_first_obs = rinex_epoch_time_to_c(epoch);
        out.time_of_first_obs_scale = time_scale_to_c_code(scale);
    }
    out.obs_code_count = header.obs_codes.values().map(Vec::len).sum();
    out.phase_shift_count = header.phase_shifts.len();
    out.scale_factor_count = header.scale_factors.len();
    out.glonass_slot_count = header.glonass_slots.len();
    if let Some(marker_name) = &header.marker_name {
        out.has_marker_name = true;
        out.marker_name = fixed_c_chars::<RINEX_OBS_MARKER_C_BYTES>(marker_name);
    }
    out
}

fn rinex_obs_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn rinex_lint_summary_to_c(
    report: &sidereon_core::rinex::qc::LintReport,
) -> SidereonRinexLintSummary {
    use sidereon_core::rinex::qc::Severity;
    SidereonRinexLintSummary {
        finding_count: report.findings.len(),
        fatal_count: report.count(Severity::Fatal),
        error_count: report.count(Severity::Error),
        warning_count: report.count(Severity::Warning),
        info_count: report.count(Severity::Info),
        is_clean: report.is_clean(),
        decoded_from_crinex: report.decoded_from_crinex,
    }
}

fn rinex_lint_finding_to_c(
    finding: &sidereon_core::rinex::qc::Finding,
) -> SidereonRinexLintFinding {
    let at = finding.at();
    SidereonRinexLintFinding {
        code: fixed_c_chars::<RINEX_QC_CODE_C_BYTES>(finding.code()),
        severity: rinex_qc_severity_to_c(finding.severity()),
        repairable: finding.is_repairable(),
        has_epoch_index: at.epoch_index.is_some(),
        epoch_index: at.epoch_index.unwrap_or(0),
        has_satellite: at.satellite.is_some(),
        satellite: at
            .satellite
            .as_deref()
            .map(satellite_token_from_text)
            .unwrap_or_else(observation_qc_signal_empty_sat),
        has_field: at.field.is_some(),
        field: fixed_c_chars::<RINEX_QC_FIELD_C_BYTES>(at.field.unwrap_or("")),
    }
}

fn finding_json_float(value: f64) -> serde_json::Value {
    if let Some(number) = serde_json::Number::from_f64(value) {
        serde_json::Value::Number(number)
    } else if value.is_nan() {
        serde_json::Value::String("NaN".to_owned())
    } else if value.is_sign_positive() {
        serde_json::Value::String("Infinity".to_owned())
    } else {
        serde_json::Value::String("-Infinity".to_owned())
    }
}

fn finding_epoch_json(
    epoch: &sidereon_core::rinex::observations::ObsEpochTime,
) -> serde_json::Value {
    use serde_json::{json, Map, Value};
    let mut value = Map::new();
    value.insert("year".to_owned(), json!(epoch.year));
    value.insert("month".to_owned(), json!(epoch.month));
    value.insert("day".to_owned(), json!(epoch.day));
    value.insert("hour".to_owned(), json!(epoch.hour));
    value.insert("minute".to_owned(), json!(epoch.minute));
    value.insert("second".to_owned(), finding_json_float(epoch.second));
    Value::Object(value)
}

fn rinex_lint_finding_detail_json(
    finding: &sidereon_core::rinex::qc::Finding,
) -> serde_json::Value {
    use serde_json::json;
    use sidereon_core::rinex::qc::Finding as F;

    let (kind, details) = match finding {
        F::ObsFatalParse { message, .. } => ("ObsFatalParse", json!({"message": message})),
        F::ObsUnpublishedVersion { version, .. } => (
            "ObsUnpublishedVersion",
            json!({"version": finding_json_float(*version)}),
        ),
        F::ObsMissingHeader { label, .. } => ("ObsMissingHeader", json!({"label": label})),
        F::ObsMissingObsTypes { .. } => ("ObsMissingObsTypes", json!({})),
        F::ObsInvalidObsCode { system, code, .. } => (
            "ObsInvalidObsCode",
            json!({"system": system.as_str(), "code": code}),
        ),
        F::ObsDuplicateObsCode { system, code, .. } => (
            "ObsDuplicateObsCode",
            json!({"system": system.as_str(), "code": code}),
        ),
        F::ObsTimeOfFirstMismatch {
            declared,
            declared_scale,
            observed,
            observed_scale,
            ..
        } => (
            "ObsTimeOfFirstMismatch",
            json!({"declared": finding_epoch_json(declared), "declared_scale": declared_scale.abbrev(), "observed": finding_epoch_json(observed), "observed_scale": observed_scale.abbrev()}),
        ),
        F::ObsTimeOfLastMismatch {
            declared,
            declared_scale,
            observed,
            observed_scale,
            ..
        } => (
            "ObsTimeOfLastMismatch",
            json!({"declared": finding_epoch_json(declared), "declared_scale": declared_scale.abbrev(), "observed": finding_epoch_json(observed), "observed_scale": observed_scale.abbrev()}),
        ),
        F::ObsIntervalMismatch {
            declared_s,
            observed_s,
            ..
        } => (
            "ObsIntervalMismatch",
            json!({"declared_s": finding_json_float(*declared_s), "observed_s": finding_json_float(*observed_s)}),
        ),
        F::ObsIntervalUnavailable { .. } => ("ObsIntervalUnavailable", json!({})),
        F::ObsInvalidInterval { declared_s, .. } => (
            "ObsInvalidInterval",
            json!({"declared_s": finding_json_float(*declared_s)}),
        ),
        F::ObsSatelliteCountMismatch {
            declared, observed, ..
        } => (
            "ObsSatelliteCountMismatch",
            json!({"declared": declared, "observed": observed}),
        ),
        F::ObsPrnObsCountMismatch {
            satellite,
            code,
            declared,
            observed,
            ..
        } => (
            "ObsPrnObsCountMismatch",
            json!({"satellite": satellite.to_string(), "code": code, "declared": declared, "observed": observed}),
        ),
        F::ObsGlonassSlotIssue {
            satellite, issue, ..
        } => (
            "ObsGlonassSlotIssue",
            json!({"satellite": satellite.to_string(), "issue": issue}),
        ),
        F::ObsPhaseShiftUndeclaredCode { system, code, .. } => (
            "ObsPhaseShiftUndeclaredCode",
            json!({"system": system.as_str(), "code": code}),
        ),
        F::ObsScaleFactorIssue { system, code, .. } => (
            "ObsScaleFactorIssue",
            json!({"system": system.as_str(), "code": code}),
        ),
        F::ObsMarkerTypeIssue { marker_type, .. } => {
            ("ObsMarkerTypeIssue", json!({"marker_type": marker_type}))
        }
        F::ObsIdentityFieldIssue { label, value, .. } => (
            "ObsIdentityFieldIssue",
            json!({"label": label, "value": value}),
        ),
        F::ObsImplausibleApproxPosition { radius_m, .. } => (
            "ObsImplausibleApproxPosition",
            json!({"radius_m": finding_json_float(*radius_m)}),
        ),
        F::ObsImplausibleAntennaDelta {
            component, value_m, ..
        } => (
            "ObsImplausibleAntennaDelta",
            json!({"component": component, "value_m": finding_json_float(*value_m)}),
        ),
        F::ObsEpochOrder {
            previous, current, ..
        } => (
            "ObsEpochOrder",
            json!({"previous": finding_epoch_json(previous), "current": finding_epoch_json(current)}),
        ),
        F::ObsDuplicateEpoch { epoch, .. } => (
            "ObsDuplicateEpoch",
            json!({"epoch": finding_epoch_json(epoch)}),
        ),
        F::ObsSkippedRecords { count, .. } => ("ObsSkippedRecords", json!({"count": count})),
        F::ObsEpochSatCountMismatch {
            declared, retained, ..
        } => (
            "ObsEpochSatCountMismatch",
            json!({"declared": declared, "retained": retained}),
        ),
        F::ObsEventHeaderUnreadable { message, .. } => {
            ("ObsEventHeaderUnreadable", json!({"message": message}))
        }
        F::ObsUnretainedHeader { label, .. } => ("ObsUnretainedHeader", json!({"label": label})),
        F::ObsPseudorangeOutOfRange { code, value_m, .. } => (
            "ObsPseudorangeOutOfRange",
            json!({"code": code, "value_m": finding_json_float(*value_m)}),
        ),
        F::ObsLossOfLockOutOfRange { code, lli, .. } => {
            ("ObsLossOfLockOutOfRange", json!({"code": code, "lli": lli}))
        }
        F::ObsEventEpoch { flag, .. } => ("ObsEventEpoch", json!({"flag": flag})),
        F::ObsEmptySatelliteRecord { .. } => ("ObsEmptySatelliteRecord", json!({})),
        F::ObsEpochGap {
            gap_s, interval_s, ..
        } => (
            "ObsEpochGap",
            json!({"gap_s": finding_json_float(*gap_s), "interval_s": finding_json_float(*interval_s)}),
        ),
        F::NavFatalParse { message, .. } => ("NavFatalParse", json!({"message": message})),
        F::NavLeapSecondsAbsent { .. } => ("NavLeapSecondsAbsent", json!({})),
        F::NavIonoMalformed { message, .. } => ("NavIonoMalformed", json!({"message": message})),
        F::NavDroppedBlock {
            satellite, message, ..
        } => (
            "NavDroppedBlock",
            json!({"satellite": satellite, "message": message}),
        ),
        F::NavDuplicateRecord {
            satellite,
            same_payload,
            ..
        } => (
            "NavDuplicateRecord",
            json!({"satellite": satellite.to_string(), "same_payload": same_payload}),
        ),
        F::NavUnsortedRecords { .. } => ("NavUnsortedRecords", json!({})),
        F::NavImplausibleRecord {
            satellite,
            field,
            value,
            ..
        } => (
            "NavImplausibleRecord",
            json!({"satellite": satellite.to_string(), "field": field, "value": finding_json_float(*value)}),
        ),
        F::NavUnhealthyRecords { system, count, .. } => (
            "NavUnhealthyRecords",
            json!({"system": system.as_str(), "count": count}),
        ),
        F::NavOutOfScopeRecords { class, count, .. } => (
            "NavOutOfScopeRecords",
            json!({"class": class, "count": count}),
        ),
        _ => ("Unknown", json!({})),
    };
    json!({"kind": kind, "spec_ref": finding.spec_ref(), "details": details})
}

fn repair_options_from_c(
    fn_name: &str,
    options: *const SidereonRinexRepairOptions,
) -> Result<sidereon_core::rinex::qc::RepairOptions, SidereonStatus> {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(sidereon_core::rinex::qc::RepairOptions::default());
    };
    let file_stamp = if options.has_file_stamp {
        Some(sidereon_core::rinex::observations::PgmRunByDate {
            program: fixed_c_array_to_string(
                fn_name,
                "file_stamp_program",
                &options.file_stamp_program,
            )?,
            run_by: fixed_c_array_to_string(
                fn_name,
                "file_stamp_run_by",
                &options.file_stamp_run_by,
            )?,
            date: fixed_c_array_to_string(fn_name, "file_stamp_date", &options.file_stamp_date)?,
        })
    } else {
        None
    };
    let mut o = sidereon_core::rinex::qc::RepairOptions::default();
    o.file_stamp = file_stamp;
    o.set_interval = options.set_interval;
    o.set_time_of_last_obs = options.set_time_of_last_obs;
    o.set_obs_counts = options.set_obs_counts;
    o.drop_empty_records = options.drop_empty_records;
    o.sort_records = options.sort_records;
    o.drop_unsupported = options.drop_unsupported;
    Ok(o)
}

fn rinex_qc_severity_to_c(severity: sidereon_core::rinex::qc::Severity) -> u32 {
    match severity {
        sidereon_core::rinex::qc::Severity::Fatal => SidereonRinexQcSeverity::Fatal as u32,
        sidereon_core::rinex::qc::Severity::Error => SidereonRinexQcSeverity::Error as u32,
        sidereon_core::rinex::qc::Severity::Warning => SidereonRinexQcSeverity::Warning as u32,
        sidereon_core::rinex::qc::Severity::Info => SidereonRinexQcSeverity::Info as u32,
    }
}

#[cfg(test)]
mod rinex_parity_tests {
    use super::*;
    use std::ffi::CString;

    const NAV_FIXTURE: &str =
        include_str!("../tests/fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx");
    const CLOCK_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/clk/synthetic_rinex_clock.clk");
    // An SBAS MT2 capture: the 226-bit body and its framed form (CRC-24Q and six
    // zero pad bits), as sidereon-core's SBAS log tests carry them. The log
    // readers refuse a payload whose preamble is not an SBAS preamble.
    const SBAS_HEX: &str = "5308DFFC010005FFC00DFFC009FFDFFC001FFDFFDFFFBABBBBBB9BBB80";
    const SBAS_FRAME_HEX: &str = "5308DFFC010005FFC00DFFC009FFDFFC001FFDFFDFFFBABBBBBB9BBB83A9CE00";
    // A RINEX 3.04 GLONASS record has four lines (3.05 added a fifth), and
    // header labels sit in columns 61-80.
    const GLONASS_NAV: &str = "     3.04           NAVIGATION DATA     M                   RINEX VERSION / TYPE\n                                                            END OF HEADER\nR01 2020 06 24 23 15 00 6.355904042721e-05 0.000000000000e+00 3.420000000000e+05\n     1.090894238281e+04 1.407806396484e+00-1.862645149231e-09 0.000000000000e+00\n    -2.885726074219e+03 2.795855522156e+00-0.000000000000e+00 1.000000000000e+00\n     2.288353955078e+04-3.169984817505e-01-2.793967723846e-09 0.000000000000e+00\n";

    use sidereon_core::rinex::nav as core_nav;

    fn wire_form(form: sidereon_core::sbas::SbasWireForm) -> SidereonSbasWireForm {
        match form {
            sidereon_core::sbas::SbasWireForm::Framed250 => SidereonSbasWireForm::Framed250,
            sidereon_core::sbas::SbasWireForm::Body226 => SidereonSbasWireForm::Body226,
        }
    }

    fn token_text(token: SidereonSatelliteToken) -> String {
        let end = token
            .bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(token.bytes.len());
        String::from_utf8(token.bytes[..end].iter().map(|byte| *byte as u8).collect())
            .expect("C token is UTF-8")
    }

    #[test]
    fn auxiliary_parsers_keep_distinctions_and_map_errors() {
        // Every expected value is sidereon-core's own reading of the same
        // text.
        assert!(core_nav::parse_nav_lenient("not nav").is_err());
        let core_iono =
            iono_corrections_to_c(&core_nav::parse_iono_corrections(NAV_FIXTURE).expect("iono"));
        let core_leap = core_nav::parse_leap_seconds(NAV_FIXTURE).expect("leap seconds");
        unsafe {
            let mut nav = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_lenient(b"not nav".as_ptr(), 7, &mut nav),
                SidereonStatus::InvalidArgument
            );
            assert!(nav.is_null());

            let mut iono = empty_iono_corrections();
            assert_eq!(
                sidereon_parse_rinex_iono_corrections(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut iono
                ),
                SidereonStatus::Ok
            );
            assert_eq!(iono.gps.present, core_iono.gps.present);
            assert_eq!(iono.galileo.present, core_iono.galileo.present);

            let mut leap = 0.0;
            let mut present = false;
            assert_eq!(
                sidereon_parse_rinex_leap_seconds(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut leap,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            assert_eq!(present, core_leap.is_some());
            assert_eq!(leap.to_bits(), core_leap.unwrap_or(0.0).to_bits());

            let empty_header = b"     3.05           NAVIGATION DATA     MIXED               RINEX VERSION / TYPE\n                                                            END OF HEADER\n";
            assert_eq!(
                sidereon_parse_rinex_leap_seconds(
                    empty_header.as_ptr(),
                    empty_header.len(),
                    &mut leap,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                present,
                core_nav::parse_leap_seconds(
                    std::str::from_utf8(empty_header).expect("ASCII header")
                )
                .expect("leap seconds")
                .is_some()
            );

            let ems = format!("120,26,7,1,0,0,1,2,{SBAS_FRAME_HEX}\n");
            let rtklib = format!("2360 259200 120 2 : {SBAS_HEX}\n");
            let core_ems = sidereon_core::sbas::parse_ems_lines(&ems).expect("core EMS");
            let core_rtklib =
                sidereon_core::sbas::parse_rtklib_lines(&rtklib).expect("core RTKLIB");
            let mut ems_blocks = ptr::null_mut();
            let mut rtklib_blocks = ptr::null_mut();
            assert_eq!(
                sidereon_parse_sbas_ems_lines(ems.as_ptr(), ems.len(), &mut ems_blocks),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_parse_sbas_rtklib_lines(rtklib.as_ptr(), rtklib.len(), &mut rtklib_blocks),
                SidereonStatus::Ok
            );
            let mut count = 0;
            assert_eq!(
                sidereon_sbas_log_blocks_count(ems_blocks, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, core_ems.len());
            let mut block = empty_sbas_log_block();
            assert_eq!(
                sidereon_sbas_log_blocks_item(ems_blocks, 0, &mut block),
                SidereonStatus::Ok
            );
            let expected = &core_ems[0];
            assert_eq!(token_text(block.sat_id), expected.satellite_id.to_string());
            assert_eq!(block.form, wire_form(expected.form));
            assert_eq!(block.byte_count, expected.bytes.len());
            assert_eq!(
                (block.has_declared_message_type, block.declared_message_type),
                (
                    expected.declared_message_type.is_some(),
                    expected.declared_message_type.unwrap_or(0)
                )
            );
            assert_eq!(
                (block.has_message_type, block.message_type),
                (
                    expected.message_type().is_some(),
                    expected.message_type().unwrap_or(0)
                )
            );
            let mut rtklib_block = empty_sbas_log_block();
            assert_eq!(
                sidereon_sbas_log_blocks_item(rtklib_blocks, 0, &mut rtklib_block),
                SidereonStatus::Ok
            );
            assert_eq!(rtklib_block.form, wire_form(core_rtklib[0].form));
            sidereon_sbas_log_blocks_free(ems_blocks);
            sidereon_sbas_log_blocks_free(rtklib_blocks);

            let mut empty_blocks = ptr::null_mut();
            assert_eq!(
                sidereon_parse_sbas_ems_lines(b"not,enough".as_ptr(), 10, &mut empty_blocks),
                SidereonStatus::Ok
            );
            assert!(!empty_blocks.is_null());
            let mut empty_count = usize::MAX;
            sidereon_sbas_log_blocks_count(empty_blocks, &mut empty_count);
            assert_eq!(
                empty_count,
                sidereon_core::sbas::parse_ems_lines("not,enough")
                    .expect("core EMS")
                    .len()
            );
            sidereon_sbas_log_blocks_free(empty_blocks);
        }
    }

    #[test]
    fn path_nav_loader_keeps_store_and_leap_seconds_on_one_source() {
        struct RemoveFile(std::path::PathBuf);

        impl Drop for RemoveFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }

        let path = std::env::temp_dir().join(format!(
            "sidereon-c-rinex-nav-loader-{}.rnx",
            std::process::id()
        ));
        std::fs::write(&path, NAV_FIXTURE).expect("write NAV loader fixture");
        let _remove_file = RemoveFile(path.clone());
        let path = CString::new(path.to_str().expect("temporary path is UTF-8"))
            .expect("temporary path has no NUL");

        unsafe {
            let mut broadcast = ptr::null_mut();
            assert_eq!(
                sidereon_broadcast_ephemeris_load_nav(path.as_ptr(), &mut broadcast),
                SidereonStatus::Ok
            );
            assert!(!broadcast.is_null());
            let mut leap_seconds = 0.0;
            let mut present = false;
            assert_eq!(
                sidereon_broadcast_ephemeris_leap_seconds(
                    broadcast,
                    &mut leap_seconds,
                    &mut present
                ),
                SidereonStatus::Ok
            );
            let core_leap = core_nav::parse_leap_seconds(NAV_FIXTURE).expect("leap seconds");
            assert_eq!(present, core_leap.is_some());
            assert_eq!(leap_seconds.to_bits(), core_leap.unwrap_or(0.0).to_bits());
            sidereon_broadcast_ephemeris_free(broadcast);

            let invalid_path = std::env::temp_dir().join(format!(
                "sidereon-c-rinex-nav-loader-invalid-{}.rnx",
                std::process::id()
            ));
            std::fs::write(&invalid_path, [0xff, 0xfe]).expect("write invalid UTF-8 fixture");
            let _remove_invalid_file = RemoveFile(invalid_path.clone());
            let invalid_path =
                CString::new(invalid_path.to_str().expect("temporary path is UTF-8"))
                    .expect("temporary path has no NUL");
            let sentinel = std::ptr::NonNull::<SidereonBroadcastEphemeris>::dangling().as_ptr();
            let mut broadcast = sentinel;
            assert_eq!(
                sidereon_broadcast_ephemeris_load_nav(invalid_path.as_ptr(), &mut broadcast),
                SidereonStatus::InvalidToken
            );
            assert!(broadcast.is_null());

            let mut message = [0 as c_char; 128];
            sidereon_last_error_message(message.as_mut_ptr(), message.len());
            let message = CStr::from_ptr(message.as_ptr())
                .to_str()
                .expect("last error is UTF-8");
            assert_eq!(
                message,
                "sidereon_broadcast_ephemeris_load_nav: source is not valid UTF-8"
            );
        }
    }

    #[test]
    fn raw_records_round_trip_and_rich_store_preserve_fields() {
        // sidereon-core's own readings of the same texts.
        let nav_with_glonass = format!("{NAV_FIXTURE}{GLONASS_NAV}");
        let core_records = core_nav::parse_nav(NAV_FIXTURE).expect("core records");
        let core_glonass = core_nav::parse_glonass_lenient(GLONASS_NAV).expect("core GLONASS");
        let extended_glonass = GLONASS_NAV.replacen("R01 ", "R28 ", 1);
        let core_extended =
            core_nav::parse_glonass_lenient(&extended_glonass).expect("core GLONASS R28");
        let unnamed_glonass = GLONASS_NAV.replacen("R01 ", "R00 ", 1);
        let core_unnamed =
            core_nav::parse_glonass_lenient(&unnamed_glonass).expect("core GLONASS R00");
        let core_store = BroadcastEphemeris::from_nav(&nav_with_glonass).expect("core store");
        let core_store_leap =
            core_nav::parse_leap_seconds(&nav_with_glonass).expect("core leap seconds");
        unsafe {
            let mut records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_records(
                    NAV_FIXTURE.as_ptr(),
                    NAV_FIXTURE.len(),
                    &mut records
                ),
                SidereonStatus::Ok
            );
            let mut record_count = 0;
            assert_eq!(
                sidereon_rinex_nav_records_count(records, &mut record_count),
                SidereonStatus::Ok
            );
            assert_eq!(record_count, core_records.len());
            let mut record = empty_broadcast_record();
            assert_eq!(
                sidereon_rinex_nav_records_item(records, 0, &mut record),
                SidereonStatus::Ok
            );
            assert_eq!(
                token_text(record.sat_id),
                core_records[0].satellite_id.to_string()
            );
            assert_eq!(
                record.elements.sqrt_a.to_bits(),
                core_records[0].elements.sqrt_a.to_bits()
            );

            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_encode_rinex_nav(
                    &record,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert!(required > 0);
            let mut encoded = vec![0_u8; required];
            assert_eq!(
                sidereon_encode_rinex_nav(
                    &record,
                    1,
                    encoded.as_mut_ptr(),
                    encoded.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, encoded.len());
            let mut reparsed = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_records(encoded.as_ptr(), encoded.len(), &mut reparsed),
                SidereonStatus::Ok
            );
            let mut reparsed_count = 0;
            sidereon_rinex_nav_records_count(reparsed, &mut reparsed_count);
            // The one record written reads back as one record.
            assert_eq!(reparsed_count, 1);
            sidereon_rinex_nav_records_free(reparsed);
            sidereon_rinex_nav_records_free(records);

            let mut glonass_records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_glonass_records(
                    GLONASS_NAV.as_ptr(),
                    GLONASS_NAV.len(),
                    &mut glonass_records
                ),
                SidereonStatus::Ok
            );
            let mut standalone_glonass_count = 0;
            sidereon_rinex_glonass_records_count(glonass_records, &mut standalone_glonass_count);
            assert_eq!(standalone_glonass_count, core_glonass.records.len());
            let mut standalone_glonass = empty_glonass_record();
            sidereon_rinex_glonass_records_item(glonass_records, 0, &mut standalone_glonass);
            let expected = glonass_record_to_c(&core_glonass.records[0]);
            assert_eq!(standalone_glonass.sat_id.bytes, expected.sat_id.bytes);
            assert_eq!(standalone_glonass.freq_channel, expected.freq_channel);
            sidereon_rinex_glonass_records_free(glonass_records);

            // The extended slot R28 is inside the shared 01..99 token range and
            // is read with its own values.
            let mut extended_records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_glonass_records(
                    extended_glonass.as_ptr(),
                    extended_glonass.len(),
                    &mut extended_records
                ),
                SidereonStatus::Ok
            );
            let mut extended_record_count = usize::MAX;
            assert_eq!(
                sidereon_rinex_glonass_records_count(extended_records, &mut extended_record_count),
                SidereonStatus::Ok
            );
            assert_eq!(extended_record_count, core_extended.records.len());
            let mut extended_record = empty_glonass_record();
            sidereon_rinex_glonass_records_item(extended_records, 0, &mut extended_record);
            let expected = glonass_record_to_c(&core_extended.records[0]);
            assert_eq!(extended_record.sat_id.bytes, expected.sat_id.bytes);
            assert_eq!(extended_record.freq_channel, expected.freq_channel);
            let mut skipped_count = usize::MAX;
            assert_eq!(
                sidereon_rinex_glonass_records_skipped_count(extended_records, &mut skipped_count),
                SidereonStatus::Ok
            );
            assert_eq!(skipped_count, core_extended.skipped.len());
            sidereon_rinex_glonass_records_free(extended_records);

            // R00 names no satellite; it is skipped and reported by its token.
            let mut unnamed_records = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_glonass_records(
                    unnamed_glonass.as_ptr(),
                    unnamed_glonass.len(),
                    &mut unnamed_records
                ),
                SidereonStatus::Ok
            );
            let mut unnamed_record_count = usize::MAX;
            sidereon_rinex_glonass_records_count(unnamed_records, &mut unnamed_record_count);
            assert_eq!(unnamed_record_count, core_unnamed.records.len());
            assert_eq!(
                sidereon_rinex_glonass_records_skipped_count(unnamed_records, &mut skipped_count),
                SidereonStatus::Ok
            );
            assert_eq!(skipped_count, core_unnamed.skipped.len());
            let mut skipped = SidereonSkippedGlonassRecord {
                satellite: satellite_token_from_text(""),
            };
            assert_eq!(
                sidereon_rinex_glonass_records_skipped_item(unnamed_records, 0, &mut skipped),
                SidereonStatus::Ok
            );
            assert_eq!(token_text(skipped.satellite), core_unnamed.skipped[0].token);
            sidereon_rinex_glonass_records_free(unnamed_records);

            let mut broadcast = ptr::null_mut();
            assert_eq!(
                sidereon_broadcast_ephemeris_parse_nav(
                    nav_with_glonass.as_ptr(),
                    nav_with_glonass.len(),
                    &mut broadcast
                ),
                SidereonStatus::Ok
            );
            let mut full_count = 0;
            let mut glonass_count = 0;
            let mut channel_count = 0;
            sidereon_broadcast_ephemeris_record_count(broadcast, &mut full_count);
            sidereon_broadcast_ephemeris_glonass_record_count(broadcast, &mut glonass_count);
            sidereon_broadcast_ephemeris_glonass_frequency_channel_count(
                broadcast,
                &mut channel_count,
            );
            assert_eq!(full_count, core_store.records().len());
            assert_eq!(glonass_count, core_store.glonass_records().len());
            assert_eq!(channel_count, core_store.glonass_frequency_channels().len());
            let mut rich = empty_broadcast_record();
            let mut rich_written = 0;
            let mut rich_required = 0;
            assert_eq!(
                sidereon_broadcast_ephemeris_records_full(
                    broadcast,
                    &mut rich,
                    1,
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(rich_required, full_count);
            let mut rich_records = vec![empty_broadcast_record(); full_count];
            assert_eq!(
                sidereon_broadcast_ephemeris_records_full(
                    broadcast,
                    rich_records.as_mut_ptr(),
                    rich_records.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, full_count);
            assert_eq!(
                token_text(rich_records[0].sat_id),
                core_store.records()[0].satellite_id.to_string()
            );
            assert_eq!(
                rich_records[0].elements.sqrt_a.to_bits(),
                core_store.records()[0].elements.sqrt_a.to_bits()
            );

            let mut rich_glonass = vec![empty_glonass_record(); glonass_count];
            assert_eq!(
                sidereon_broadcast_ephemeris_glonass_records(
                    broadcast,
                    rich_glonass.as_mut_ptr(),
                    rich_glonass.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, glonass_count);
            let expected = glonass_record_to_c(&core_store.glonass_records()[0]);
            assert_eq!(rich_glonass[0].sat_id.bytes, expected.sat_id.bytes);
            assert_eq!(rich_glonass[0].freq_channel, expected.freq_channel);
            assert_eq!(
                rich_glonass[0].pos_m.map(f64::to_bits),
                expected.pos_m.map(f64::to_bits)
            );

            let mut channels = vec![
                SidereonFrequencyChannel {
                    slot: 0,
                    channel: 0,
                };
                channel_count
            ];
            assert_eq!(
                sidereon_broadcast_ephemeris_glonass_frequency_channels(
                    broadcast,
                    channels.as_mut_ptr(),
                    channels.len(),
                    &mut rich_written,
                    &mut rich_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(rich_written, channel_count);
            let core_channels: Vec<_> = core_store
                .glonass_frequency_channels()
                .into_iter()
                .collect();
            assert_eq!(
                (channels[0].slot, channels[0].channel),
                (core_channels[0].0, core_channels[0].1)
            );

            let mut iono = empty_iono_corrections();
            sidereon_broadcast_ephemeris_iono_corrections(broadcast, &mut iono);
            assert_eq!(
                iono.gps.present,
                iono_corrections_to_c(&core_store.iono_corrections())
                    .gps
                    .present
            );
            let mut rich_leap = 0.0;
            let mut rich_present = false;
            sidereon_broadcast_ephemeris_leap_seconds(broadcast, &mut rich_leap, &mut rich_present);
            assert_eq!(rich_present, core_store_leap.is_some());
            assert_eq!(
                rich_leap.to_bits(),
                core_store_leap.unwrap_or(0.0).to_bits()
            );
            sidereon_broadcast_ephemeris_free(broadcast);
        }
    }

    #[test]
    fn lenient_diagnostics_and_lossy_clock_samples_are_owned() {
        unsafe {
            let source = NAV_FIXTURE
                .lines()
                .find(|line| {
                    let bytes = line.as_bytes();
                    bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1].is_ascii_digit()
                })
                .expect("fixture has a NAV record");
            let bad_source = source.replacen("2020", "XXXX", 1);
            let bad_nav = NAV_FIXTURE.replacen(source, &bad_source, 1);
            // sidereon-core's own lenient reading of the damaged text.
            let core_parse = core_nav::parse_nav_lenient(&bad_nav).expect("core lenient parse");
            let mut parse = ptr::null_mut();
            assert_eq!(
                sidereon_parse_rinex_nav_lenient(bad_nav.as_ptr(), bad_nav.len(), &mut parse),
                SidereonStatus::Ok
            );
            let mut skipped_count = 0;
            sidereon_nav_parse_skipped_count(parse, &mut skipped_count);
            assert_eq!(skipped_count, core_parse.skipped.len());
            let mut skipped = SidereonSkippedNavBlock {
                satellite: satellite_token_from_text(""),
                message: [0; 256],
            };
            sidereon_nav_parse_skipped(parse, 0, &mut skipped);
            assert_eq!(
                token_text(skipped.satellite),
                core_parse.skipped[0].satellite
            );
            let mut message_written = 0;
            let mut message_required = 0;
            assert_eq!(
                sidereon_nav_parse_skipped_message(
                    parse,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut message_written,
                    &mut message_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(message_required, core_parse.skipped[0].message.len());
            sidereon_nav_parse_free(parse);

            // sidereon-core's own lossy reading of the clock fixture.
            let core_clock = sidereon_core::rinex::clock::RinexClock::parse_lossy(
                std::str::from_utf8(CLOCK_FIXTURE).expect("ASCII clock fixture"),
            );
            let core_series = core_clock.series();
            let mut clock = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_parse_lossy(
                    CLOCK_FIXTURE.as_ptr(),
                    CLOCK_FIXTURE.len(),
                    &mut clock
                ),
                SidereonStatus::Ok
            );
            let mut satellite_count = 0;
            let mut sample_count = 0;
            sidereon_rinex_clock_series_count(clock, &mut satellite_count);
            sidereon_rinex_clock_sample_count(clock, &mut sample_count);
            assert_eq!(satellite_count, core_series.len());
            assert_eq!(
                sample_count,
                core_series.values().map(Vec::len).sum::<usize>()
            );
            let mut satellites = [SidereonSatelliteToken { bytes: [0; 17] }; 2];
            let mut sat_written = 0;
            let mut sat_required = 0;
            assert_eq!(
                sidereon_rinex_clock_satellites(
                    clock,
                    satellites.as_mut_ptr(),
                    satellites.len(),
                    &mut sat_written,
                    &mut sat_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(sat_written, core_series.len());
            let first_satellite = core_series.keys().next().expect("a series");
            assert_eq!(token_text(satellites[0]), *first_satellite);
            let core_samples = &core_series[first_satellite];
            let satellite = CString::new(first_satellite.as_str()).expect("satellite");
            let mut series = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_series_for(clock, satellite.as_ptr(), &mut series),
                SidereonStatus::Ok
            );
            let mut series_count = 0;
            sidereon_rinex_clock_series_sample_count(series, &mut series_count);
            assert_eq!(series_count, core_samples.len());
            let mut samples = vec![
                SidereonClockPoint {
                    epoch: SidereonClockEpoch {
                        scale: 0,
                        representation: 0,
                        jd_whole: 0.0,
                        jd_fraction: 0.0,
                        nanos_high: 0,
                        nanos_low: 0,
                    },
                    bias_s: 0.0,
                    additional_value_count: 0,
                    additional_values: [0.0; SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES],
                };
                core_samples.len()
            ];
            let mut sample_written = 0;
            let mut sample_required = 0;
            assert_eq!(
                sidereon_rinex_clock_series_samples(
                    series,
                    samples.as_mut_ptr(),
                    samples.len(),
                    &mut sample_written,
                    &mut sample_required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(sample_written, core_samples.len());
            for (got, point) in samples.iter().zip(core_samples) {
                assert_eq!(got.epoch.scale, time_scale_to_c_code(point.epoch.scale));
                match point.epoch.repr {
                    InstantRepr::JulianDate(jd) => {
                        assert_eq!(
                            got.epoch.representation,
                            SidereonRinexClockInstantRepresentation::JulianDate as u32
                        );
                        assert_eq!(got.epoch.jd_whole.to_bits(), jd.jd_whole.to_bits());
                        assert_eq!(got.epoch.jd_fraction.to_bits(), jd.fraction.to_bits());
                    }
                    InstantRepr::Nanos(nanos) => {
                        assert_eq!(
                            got.epoch.representation,
                            SidereonRinexClockInstantRepresentation::Nanos as u32
                        );
                        let bits = nanos as u128;
                        assert_eq!(got.epoch.nanos_high, (bits >> 64) as u64 as i64);
                        assert_eq!(got.epoch.nanos_low, bits as u64);
                    }
                }
                assert_eq!(got.bias_s.to_bits(), point.bias_s.to_bits());
                assert_eq!(got.additional_value_count, point.additional_values.len());
                for (index, got_value) in got.additional_values.iter().enumerate() {
                    // Slots past the declared values read NaN.
                    let want = point
                        .additional_values
                        .get(index)
                        .copied()
                        .unwrap_or(f64::NAN);
                    assert_eq!(got_value.to_bits(), want.to_bits());
                }
            }
            sidereon_rinex_clock_series_free(series);
            sidereon_rinex_clock_free(clock);

            let malformed = b"AS G05  2026 05 13 00 00  bad-second  1   2.0e-04\n";
            let mut malformed_clock = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_parse_lossy(
                    malformed.as_ptr(),
                    malformed.len(),
                    &mut malformed_clock
                ),
                SidereonStatus::Ok
            );
            sidereon_rinex_clock_sample_count(malformed_clock, &mut sample_count);
            assert_eq!(
                sample_count,
                sidereon_core::rinex::clock::RinexClock::parse_lossy(
                    std::str::from_utf8(malformed).expect("ASCII line")
                )
                .series()
                .values()
                .map(Vec::len)
                .sum::<usize>()
            );
            sidereon_rinex_clock_free(malformed_clock);
        }
    }
}

#[cfg(test)]
mod rinex_obs_contract_c_tests {
    use super::*;

    const SCALE_FACTOR_V2_OBS: &str = concat!(
        "     2.11           OBSERVATION DATA    G (GPS)             RINEX VERSION / TYPE\n",
        "     1     Z                                                # / TYPES OF OBSERV\n",
        "G   10  0                                                   SYS / SCALE FACTOR\n",
        "                                                            END OF HEADER\n",
        " 15  1  1  0  0  0.0000000  0  1G 1\n",
        "      1234.567\n",
    );

    const UNTIMED_EVENT_OBS: &str = concat!(
        "     3.05           OBSERVATION DATA    M                   RINEX VERSION / TYPE\n",
        "G    1 C1C                                                  SYS / # / OBS TYPES\n",
        "                                                            END OF HEADER\n",
        "> 2020 01 01 00 00  0.0000000  0  1\n",
        "G01  20000000.000\n",
        ">                              4  1\n",
        "an event without a significant epoch                        COMMENT\n",
        "> 2020 01 01 00 01  0.0000000  0  1\n",
        "G01  20000100.000\n",
    );

    const AMBIGUOUS_PHASE_SHIFT_OBS: &str = concat!(
        "     3.05           OBSERVATION DATA    M (MIXED)           RINEX VERSION / TYPE\n",
        "G    2 C1C L1W                                              SYS / # / OBS TYPES\n",
        "G L1W  0.25000  01 G01                                      SYS / PHASE SHIFT\n",
        "G L1W  0.50000  02 G02 G01                                  SYS / PHASE SHIFT\n",
        "                                                            END OF HEADER\n",
        "> 2020 01 01 00 00  0.0000000  0  2\n",
        "G01  20000000.000   105000000.000\n",
        "G02  21000000.000   110000000.000\n",
    );

    fn parse(text: &str) -> SidereonRinexObs {
        SidereonRinexObs {
            inner: RinexObs::parse(text).expect("parse RINEX OBS test product"),
        }
    }

    fn token_text(token: SidereonSatelliteToken) -> String {
        let end = token
            .bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(token.bytes.len());
        token.bytes[..end]
            .iter()
            .map(|byte| *byte as u8 as char)
            .collect()
    }

    #[test]
    fn a_refused_write_is_typed_and_writes_nothing() {
        let obs = parse(SCALE_FACTOR_V2_OBS);
        // sidereon-core's own refusal to write the product.
        let Err(RinexObsWriteError::ScaleFactorsInVersionTwo { count: core_count }) =
            obs.inner.to_rinex_string()
        else {
            panic!("sidereon-core writes version 2 scale factors");
        };
        unsafe {
            let mut written = usize::MAX;
            let mut required = usize::MAX;
            assert_eq!(
                sidereon_rinex_obs_to_rinex_text(
                    &obs,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!((written, required), (0, 0));

            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_obs_to_rinex_text_result(&obs, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            let error = owned.outcome.error;
            assert!(!owned.outcome.is_ok);
            assert_eq!(
                error.kind,
                SidereonRinexObsWriteErrorKind::ScaleFactorsInVersionTwo
            );
            assert!(error.has_count && error.count == core_count);
            assert!(!error.has_code && !error.has_detail && !error.has_satellite);
            assert!(error.version.is_nan());
            assert!(owned.text.is_empty());
            assert!(owned
                .message
                .starts_with("sidereon_rinex_obs_to_rinex_text_result: "));
            sidereon_rinex_obs_write_result_free(result);
        }
    }

    #[test]
    fn an_event_with_blank_epoch_fields_reports_no_time() {
        let obs = parse(UNTIMED_EVENT_OBS);
        // sidereon-core's own epoch records.
        let core_epochs = obs.inner.epochs();
        unsafe {
            let mut epochs = [SidereonRinexObsEpoch {
                has_epoch: true,
                epoch: SidereonCalendarEpoch {
                    year: -1,
                    month: -1,
                    day: -1,
                    hour: -1,
                    minute: -1,
                    second: -1.0,
                },
                flag: 99,
                satellite_count: 99,
            }; 3];
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_rinex_obs_epochs(
                    &obs,
                    epochs.as_mut_ptr(),
                    epochs.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, core_epochs.len());
            for (got, want) in epochs.iter().zip(core_epochs) {
                assert_eq!(got.has_epoch, want.epoch.is_some());
                assert_eq!(got.flag, want.flag);
                assert_eq!(got.satellite_count, want.sats.len());
                match want.epoch {
                    Some(time) => {
                        let want = rinex_epoch_time_to_c(time);
                        assert_eq!(
                            (
                                got.epoch.year,
                                got.epoch.month,
                                got.epoch.day,
                                got.epoch.hour,
                                got.epoch.minute,
                                got.epoch.second.to_bits()
                            ),
                            (
                                want.year,
                                want.month,
                                want.day,
                                want.hour,
                                want.minute,
                                want.second.to_bits()
                            )
                        );
                    }
                    // An epoch with no time reads zero fields and a NaN second.
                    None => {
                        assert_eq!((got.epoch.year, got.epoch.month), (0, 0));
                        assert!(got.epoch.second.is_nan());
                    }
                }
            }
            // UNTIMED_EVENT_OBS holds the event with blank epoch fields.
            assert!(core_epochs.iter().any(|epoch| epoch.epoch.is_none()));
        }
    }

    #[test]
    fn an_ambiguous_phase_correction_keeps_the_row_and_every_correction() {
        let obs = parse(AMBIGUOUS_PHASE_SHIFT_OBS);
        // sidereon-core's own carrier-phase rows for the epoch.
        let rows = rinex_obs_carrier_phase_rows_at("test", &obs.inner, 0).expect("rows");
        unsafe {
            let mut values: Vec<SidereonRinexObsCarrierPhase> = Vec::with_capacity(rows.len());
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_rinex_obs_carrier_phase(
                    &obs,
                    0,
                    values.as_mut_ptr(),
                    rows.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            values.set_len(written);
            assert_eq!(written, rows.len());
            for (got, (sat, row)) in values.iter().zip(&rows) {
                assert_eq!(token_text(got.sat_id), sat.to_string());
                assert_eq!(
                    got.value_cycles.to_bits(),
                    row.value_cycles.unwrap_or(0.0).to_bits()
                );
                match &row.phase_shift_cycles {
                    Ok(cycles) => {
                        assert_eq!(
                            got.phase_shift_status,
                            SidereonRinexCorrectionStatus::Available
                        );
                        assert_eq!(got.phase_shift_cycles.to_bits(), cycles.to_bits());
                    }
                    Err(CorrectionUnavailable::Unknown) => {
                        assert_eq!(
                            got.phase_shift_status,
                            SidereonRinexCorrectionStatus::Unknown
                        );
                    }
                    Err(CorrectionUnavailable::Ambiguous { corrections }) => {
                        assert_eq!(
                            got.phase_shift_status,
                            SidereonRinexCorrectionStatus::Ambiguous
                        );
                        assert!(got.phase_shift_cycles.is_nan());
                        assert_eq!(got.phase_shift_conflict_count, corrections.len());
                    }
                }
            }
            // AMBIGUOUS_PHASE_SHIFT_OBS gives G01 L1W two corrections.
            let Err(CorrectionUnavailable::Ambiguous {
                corrections: core_corrections,
            }) = &rows[0].1.phase_shift_cycles
            else {
                panic!("sidereon-core resolves the G01 correction");
            };

            let mut conflicts = [SidereonRinexPhaseShiftCorrection {
                has_cycles: false,
                cycles: 0.0,
            }; 2];
            assert_eq!(
                sidereon_rinex_obs_carrier_phase_conflicts(
                    &obs,
                    0,
                    0,
                    conflicts.as_mut_ptr(),
                    conflicts.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                (written, required),
                (core_corrections.len(), core_corrections.len())
            );
            for (got, want) in conflicts.iter().zip(core_corrections) {
                assert_eq!(got.has_cycles, want.is_some());
                assert_eq!(got.cycles.to_bits(), want.unwrap_or(f64::NAN).to_bits());
            }
        }
    }
}

#[cfg(test)]
mod rinex_finding_detail_json_tests {
    use super::{finding_json_float, rinex_lint_finding_detail_json};
    use sidereon_core::rinex::qc::{Finding, FindingRef};
    use sidereon_core::{GnssSatelliteId, GnssSystem};

    #[test]
    fn nonfinite_and_signed_zero_floats_are_preserved_explicitly() {
        assert_eq!(finding_json_float(f64::NAN), serde_json::json!("NaN"));
        assert_eq!(
            finding_json_float(f64::INFINITY),
            serde_json::json!("Infinity")
        );
        assert_eq!(
            finding_json_float(f64::NEG_INFINITY),
            serde_json::json!("-Infinity")
        );
        assert!(finding_json_float(-0.0)
            .as_f64()
            .unwrap()
            .is_sign_negative());
    }

    #[test]
    fn large_counts_and_optional_zero_remain_exact() {
        let max_count = Finding::NavUnhealthyRecords {
            at: FindingRef::default(),
            system: GnssSystem::Gps,
            count: usize::MAX,
        };
        let value = rinex_lint_finding_detail_json(&max_count);
        assert_eq!(value["details"]["count"].as_u64(), Some(usize::MAX as u64));

        let satellite = GnssSatelliteId::new(GnssSystem::Gps, 1).unwrap();
        let absent = Finding::ObsPrnObsCountMismatch {
            at: FindingRef::default(),
            satellite,
            code: "C1C".to_owned(),
            declared: None,
            observed: 0,
        };
        let zero = Finding::ObsPrnObsCountMismatch {
            at: FindingRef::default(),
            satellite,
            code: "C1C".to_owned(),
            declared: Some(0),
            observed: 0,
        };
        assert!(rinex_lint_finding_detail_json(&absent)["details"]["declared"].is_null());
        assert_eq!(
            rinex_lint_finding_detail_json(&zero)["details"]["declared"].as_u64(),
            Some(0)
        );
    }
}
