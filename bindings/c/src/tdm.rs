use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::tdm::{TdmError, TdmInputErrorKind};

const TDM_FIELD_C_BYTES: usize = 129;
const TDM_KEY_C_BYTES: usize = 49;
const TDM_EPOCH_C_BYTES: usize = 65;
const TDM_VALUE_TEXT_C_BYTES: usize = 65;
const TDM_PARTICIPANT_NAME_C_BYTES: usize = 65;
const TDM_PATH_PARTICIPANTS: usize = 8;

/// A parsed CCSDS Tracking Data Message. Opaque to C. Create with
/// sidereon_tdm_parse_kvn, serialize with sidereon_tdm_to_kvn, and release
/// with sidereon_tdm_free.
pub struct SidereonTdm {
    pub(crate) inner: sidereon_core::astro::tdm::Tdm,
}

/// Optional fixed-size null-terminated TDM string field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmStringField {
    /// Whether value is present.
    pub has_value: bool,
    /// Null-terminated string value when present.
    pub value: [c_char; 129],
}

/// TDM observable family.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTdmObservable {
    /// RANGE.
    Range = 0,
    /// DOPPLER_INSTANTANEOUS.
    DopplerInstantaneous = 1,
    /// DOPPLER_INTEGRATED.
    DopplerIntegrated = 2,
    /// RECEIVE_FREQ or RECEIVE_FREQ_n.
    ReceiveFreq = 3,
    /// TRANSMIT_FREQ or TRANSMIT_FREQ_n.
    TransmitFreq = 4,
    /// TRANSMIT_FREQ_RATE or TRANSMIT_FREQ_RATE_n.
    TransmitFreqRate = 5,
    /// ANGLE_1.
    Angle1 = 6,
    /// ANGLE_2.
    Angle2 = 7,
    /// A CCSDS table-defined observable without a dedicated enum variant.
    Other = 255,
}

/// TDM record unit.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTdmUnit {
    /// Kilometers.
    Kilometers = 0,
    /// Seconds.
    Seconds = 1,
    /// CCSDS range units.
    RangeUnits = 2,
    /// Kilometers per second.
    KilometersPerSecond = 3,
    /// Hertz.
    Hertz = 4,
    /// Hertz per second.
    HertzPerSecond = 5,
    /// Degrees.
    Degrees = 6,
    /// Decibel watts.
    DecibelWatts = 7,
    /// Decibel hertz.
    DecibelHertz = 8,
    /// Square meters.
    SquareMeters = 9,
    /// Meters.
    Meters = 10,
    /// Seconds per second.
    SecondsPerSecond = 11,
    /// Percent.
    Percent = 12,
    /// Kelvin.
    Kelvin = 13,
    /// Hectopascals.
    Hectopascals = 14,
    /// Total electron content units.
    TotalElectronContentUnits = 15,
    /// Dimensionless quantity.
    Dimensionless = 16,
    /// Unmodeled unit label.
    Unknown = 255,
}

/// Summary of one TDM metadata/data segment.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmSegmentSummary {
    /// Segment index in parse order.
    pub segment_index: usize,
    /// Optional MODE metadata value.
    pub mode: SidereonTdmStringField,
    /// Optional TIMETAG_REF metadata value.
    pub timetag_ref: SidereonTdmStringField,
    /// Optional TIME_SYSTEM metadata value.
    pub time_system: SidereonTdmStringField,
    /// Range unit as SidereonTdmUnit.
    pub range_unit: u32,
    /// Number of parsed participant entries.
    pub participant_count: usize,
    /// Number of parsed path entries.
    pub path_count: usize,
    /// Number of data records in the segment.
    pub record_count: usize,
}

/// One TDM participant metadata entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmParticipant {
    /// Segment index in parse order.
    pub segment_index: usize,
    /// PARTICIPANT_n suffix.
    pub index: u8,
    /// Null-terminated participant name.
    pub name: [c_char; 65],
}

/// One TDM PATH metadata entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmPath {
    /// Segment index in parse order.
    pub segment_index: usize,
    /// Original PATH keyword.
    pub key: [c_char; 49],
    /// Whether index carries the PATH_n suffix.
    pub has_index: bool,
    /// PATH_n suffix when present.
    pub index: u8,
    /// Number of participant indices copied into participants.
    pub participant_count: usize,
    /// Participant indices in path order.
    pub participants: [u8; 8],
}

/// One time-tagged TDM data record.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmDataRecord {
    /// Segment index in parse order.
    pub segment_index: usize,
    /// Observable family as SidereonTdmObservable.
    pub observable: u32,
    /// Whether observable_participant carries an indexed observable suffix.
    pub has_observable_participant: bool,
    /// Observable participant suffix when present.
    pub observable_participant: u8,
    /// Unit as SidereonTdmUnit.
    pub unit: u32,
    /// Original data keyword.
    pub keyword: [c_char; 49],
    /// Raw epoch string.
    pub epoch: [c_char; 65],
    /// Exact decimal token from the message.
    pub value_text: [c_char; 65],
    /// Parsed numeric value.
    pub value: f64,
}

/// Parse a CCSDS TDM in KVN form. On success writes a newly owned handle to
/// *out_tdm. Release it with sidereon_tdm_free.
///
/// Safety: data must point to len readable bytes; out_tdm must point to
/// storage for a SidereonTdm*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_parse_kvn(
    data: *const u8,
    len: usize,
    out_tdm: *mut *mut SidereonTdm,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tdm_parse_kvn", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_tdm, "sidereon_tdm_parse_kvn", "out_tdm"));
        let text = ndm_text_from_utf8(data, len, "sidereon_tdm_parse_kvn").map(str::to_owned);
        *out = ptr::null_mut();
        let text = c_try!(text);
        match sidereon_core::astro::tdm::parse_kvn(&text) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonTdm { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_tdm_error("sidereon_tdm_parse_kvn", err),
        }
    })
}

/// Serialize a TDM to KVN text. The output is not null-terminated.
///
/// Safety: tdm must be a live handle; out must point to len writable bytes or
/// be NULL when len is 0; out_written and out_required must point to size_t
/// storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_to_kvn(
    tdm: *const SidereonTdm,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tdm_to_kvn", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_to_kvn",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_to_kvn",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_to_kvn",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_to_kvn",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required"),
            ],
            tdm,
            "tdm",
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_to_kvn", "tdm"));
        let text = match sidereon_core::astro::tdm::encode_kvn(&tdm.inner) {
            Ok(text) => text,
            Err(err) => return map_tdm_error("sidereon_tdm_to_kvn", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_to_kvn",
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

/// Write the number of TDM segments to *out_count.
///
/// Safety: tdm must be live; out_count must point to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_segment_count(
    tdm: *const SidereonTdm,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_segment_count", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_count,
            "sidereon_tdm_segment_count",
            "out_count"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_segment_count",
            out,
            "out_count",
            tdm,
            "tdm"
        ));
        *out = 0;
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_segment_count", "tdm"));
        *out = tdm.inner.segments.len();
        SidereonStatus::Ok
    })
}

/// Write the total number of TDM data records to *out_count.
///
/// Safety: tdm must be live; out_count must point to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_record_count(
    tdm: *const SidereonTdm,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_record_count", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_count,
            "sidereon_tdm_record_count",
            "out_count"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_record_count",
            out,
            "out_count",
            tdm,
            "tdm"
        ));
        *out = 0;
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_record_count", "tdm"));
        *out = tdm
            .inner
            .segments
            .iter()
            .map(|segment| segment.data.records.len())
            .sum();
        SidereonStatus::Ok
    })
}

/// Copy TDM segment summaries.
///
/// Safety: tdm must be live; out must point to len writable entries or be NULL
/// when len is 0; out_written and out_required must point to size_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_segments(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmSegmentSummary,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_segments", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_segments",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_segments",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_segments",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_segments",
            &[
                (
                    out.cast(),
                    size_of::<SidereonTdmSegmentSummary>(),
                    len,
                    "out"
                ),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_segments", "tdm"));
        let rows: Vec<_> = tdm
            .inner
            .segments
            .iter()
            .enumerate()
            .map(tdm_segment_summary_to_c)
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_segments",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy parsed TDM participant entries across all segments.
///
/// Safety: tdm must be live; output pointers follow the variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_participants(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmParticipant,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_participants", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_participants",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_participants",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_participants",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_participants",
            &[
                (out.cast(), size_of::<SidereonTdmParticipant>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_participants", "tdm"));
        let mut rows = Vec::new();
        for (segment_index, segment) in tdm.inner.segments.iter().enumerate() {
            rows.extend(
                segment
                    .metadata
                    .participants
                    .iter()
                    .map(|participant| tdm_participant_to_c(segment_index, participant)),
            );
        }
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_participants",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy parsed TDM path entries across all segments.
///
/// Safety: tdm must be live; output pointers follow the variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_paths(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmPath,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_paths", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_paths",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_paths",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_paths",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_paths",
            &[
                (out.cast(), size_of::<SidereonTdmPath>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_paths", "tdm"));
        let mut rows = Vec::new();
        for (segment_index, segment) in tdm.inner.segments.iter().enumerate() {
            rows.extend(
                segment
                    .metadata
                    .paths
                    .iter()
                    .map(|path| tdm_path_to_c(segment_index, path)),
            );
        }
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_paths",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy flattened TDM data records across all segments.
///
/// Safety: tdm must be live; output pointers follow the variable-length output
/// contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_records(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmDataRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_records", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_records",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_records",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_records",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_records",
            &[
                (out.cast(), size_of::<SidereonTdmDataRecord>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_records", "tdm"));
        let mut rows = Vec::new();
        for (segment_index, segment) in tdm.inner.segments.iter().enumerate() {
            rows.extend(
                segment
                    .data
                    .records
                    .iter()
                    .map(|record| tdm_record_to_c(segment_index, record)),
            );
        }
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_records",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release a TDM handle. Passing NULL is a no-op.
///
/// Safety: tdm must be NULL or a live handle from sidereon_tdm_parse_kvn.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_free(tdm: *mut SidereonTdm) {
    ffi_boundary("sidereon_tdm_free", (), || {
        free_boxed(tdm);
    });
}

fn tdm_segment_summary_to_c(
    (segment_index, segment): (usize, &sidereon_core::astro::tdm::TdmSegment),
) -> SidereonTdmSegmentSummary {
    SidereonTdmSegmentSummary {
        segment_index,
        mode: optional_tdm_string(segment.metadata.mode.as_deref()),
        timetag_ref: optional_tdm_string(segment.metadata.timetag_ref.as_deref()),
        time_system: optional_tdm_string(segment.metadata.time_system.as_deref()),
        range_unit: tdm_unit_to_c(&segment.metadata.range_units),
        participant_count: segment.metadata.participants.len(),
        path_count: segment.metadata.paths.len(),
        record_count: segment.data.records.len(),
    }
}

fn tdm_participant_to_c(
    segment_index: usize,
    participant: &sidereon_core::astro::tdm::TdmParticipant,
) -> SidereonTdmParticipant {
    SidereonTdmParticipant {
        segment_index,
        index: participant.index,
        name: fixed_c_chars::<TDM_PARTICIPANT_NAME_C_BYTES>(&participant.name),
    }
}

fn tdm_path_to_c(
    segment_index: usize,
    path: &sidereon_core::astro::tdm::TdmPath,
) -> SidereonTdmPath {
    let mut participants = [0_u8; TDM_PATH_PARTICIPANTS];
    let count = path.participants.len().min(TDM_PATH_PARTICIPANTS);
    participants[..count].copy_from_slice(&path.participants[..count]);
    SidereonTdmPath {
        segment_index,
        key: fixed_c_chars::<TDM_KEY_C_BYTES>(&path.key),
        has_index: path.index.is_some(),
        index: path.index.unwrap_or(0),
        participant_count: count,
        participants,
    }
}

fn tdm_record_to_c(
    segment_index: usize,
    record: &sidereon_core::astro::tdm::TdmDataRecord,
) -> SidereonTdmDataRecord {
    let (observable, participant) = tdm_observable_to_c(&record.observable);
    SidereonTdmDataRecord {
        segment_index,
        observable,
        has_observable_participant: participant.is_some(),
        observable_participant: participant.unwrap_or(0),
        unit: tdm_unit_to_c(&record.unit),
        keyword: fixed_c_chars::<TDM_KEY_C_BYTES>(&record.keyword),
        epoch: fixed_c_chars::<TDM_EPOCH_C_BYTES>(&record.epoch),
        value_text: fixed_c_chars::<TDM_VALUE_TEXT_C_BYTES>(&record.value.text),
        value: record.value.value,
    }
}

fn optional_tdm_string(value: Option<&str>) -> SidereonTdmStringField {
    SidereonTdmStringField {
        has_value: value.is_some(),
        value: value
            .map(fixed_c_chars::<TDM_FIELD_C_BYTES>)
            .unwrap_or([0; TDM_FIELD_C_BYTES]),
    }
}

fn tdm_observable_to_c(observable: &sidereon_core::astro::tdm::TdmObservable) -> (u32, Option<u8>) {
    match observable {
        sidereon_core::astro::tdm::TdmObservable::Range => {
            (SidereonTdmObservable::Range as u32, None)
        }
        sidereon_core::astro::tdm::TdmObservable::DopplerInstantaneous => {
            (SidereonTdmObservable::DopplerInstantaneous as u32, None)
        }
        sidereon_core::astro::tdm::TdmObservable::DopplerIntegrated => {
            (SidereonTdmObservable::DopplerIntegrated as u32, None)
        }
        sidereon_core::astro::tdm::TdmObservable::ReceiveFreq { participant } => {
            (SidereonTdmObservable::ReceiveFreq as u32, *participant)
        }
        sidereon_core::astro::tdm::TdmObservable::TransmitFreq { participant } => {
            (SidereonTdmObservable::TransmitFreq as u32, *participant)
        }
        sidereon_core::astro::tdm::TdmObservable::TransmitFreqRate { participant } => {
            (SidereonTdmObservable::TransmitFreqRate as u32, *participant)
        }
        sidereon_core::astro::tdm::TdmObservable::Angle1 => {
            (SidereonTdmObservable::Angle1 as u32, None)
        }
        sidereon_core::astro::tdm::TdmObservable::Angle2 => {
            (SidereonTdmObservable::Angle2 as u32, None)
        }
        sidereon_core::astro::tdm::TdmObservable::Other(_) => {
            (SidereonTdmObservable::Other as u32, None)
        }
    }
}

fn tdm_unit_to_c(unit: &sidereon_core::astro::tdm::TdmUnit) -> u32 {
    match unit {
        sidereon_core::astro::tdm::TdmUnit::Kilometers => SidereonTdmUnit::Kilometers as u32,
        sidereon_core::astro::tdm::TdmUnit::Seconds => SidereonTdmUnit::Seconds as u32,
        sidereon_core::astro::tdm::TdmUnit::RangeUnits => SidereonTdmUnit::RangeUnits as u32,
        sidereon_core::astro::tdm::TdmUnit::KilometersPerSecond => {
            SidereonTdmUnit::KilometersPerSecond as u32
        }
        sidereon_core::astro::tdm::TdmUnit::Hertz => SidereonTdmUnit::Hertz as u32,
        sidereon_core::astro::tdm::TdmUnit::HertzPerSecond => {
            SidereonTdmUnit::HertzPerSecond as u32
        }
        sidereon_core::astro::tdm::TdmUnit::Degrees => SidereonTdmUnit::Degrees as u32,
        sidereon_core::astro::tdm::TdmUnit::DecibelWatts => SidereonTdmUnit::DecibelWatts as u32,
        sidereon_core::astro::tdm::TdmUnit::DecibelHertz => SidereonTdmUnit::DecibelHertz as u32,
        sidereon_core::astro::tdm::TdmUnit::SquareMeters => SidereonTdmUnit::SquareMeters as u32,
        sidereon_core::astro::tdm::TdmUnit::Meters => SidereonTdmUnit::Meters as u32,
        sidereon_core::astro::tdm::TdmUnit::SecondsPerSecond => {
            SidereonTdmUnit::SecondsPerSecond as u32
        }
        sidereon_core::astro::tdm::TdmUnit::Percent => SidereonTdmUnit::Percent as u32,
        sidereon_core::astro::tdm::TdmUnit::Kelvin => SidereonTdmUnit::Kelvin as u32,
        sidereon_core::astro::tdm::TdmUnit::Hectopascals => SidereonTdmUnit::Hectopascals as u32,
        sidereon_core::astro::tdm::TdmUnit::TotalElectronContentUnits => {
            SidereonTdmUnit::TotalElectronContentUnits as u32
        }
        sidereon_core::astro::tdm::TdmUnit::Dimensionless => SidereonTdmUnit::Dimensionless as u32,
        sidereon_core::astro::tdm::TdmUnit::Unknown(_) => SidereonTdmUnit::Unknown as u32,
    }
}

fn tdm_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    json!({
        "kind": kind,
        "fields": fields,
    })
}

fn tdm_input_error_kind_name(kind: TdmInputErrorKind) -> &'static str {
    match kind {
        TdmInputErrorKind::Missing => "missing",
        TdmInputErrorKind::FloatParse => "float_parse",
        TdmInputErrorKind::NonFinite => "non_finite",
        TdmInputErrorKind::NotPositive => "not_positive",
        TdmInputErrorKind::OutOfRange => "out_of_range",
        TdmInputErrorKind::InvalidIndex => "invalid_index",
        TdmInputErrorKind::UnknownKeyword => "unknown_keyword",
        TdmInputErrorKind::UnexpectedUnit => "unexpected_unit",
        TdmInputErrorKind::NonInteger => "non_integer",
        TdmInputErrorKind::Negative => "negative",
        TdmInputErrorKind::NegativeZero => "negative_zero",
        TdmInputErrorKind::UnitMismatch => "unit_mismatch",
        TdmInputErrorKind::DecimalMismatch => "decimal_mismatch",
        _ => "unknown",
    }
}

/// Convert a `TdmError` into a structured JSON error node.
pub(crate) fn tdm_error_value(error: &TdmError) -> serde_json::Value {
    match error {
        TdmError::NoSegments => tdm_node("no_segments", json!({})),
        TdmError::Section { line, detail } => tdm_node(
            "section",
            json!({
                "line": line,
                "detail": detail,
            }),
        ),
        TdmError::MalformedLine { line, text } => tdm_node(
            "malformed_line",
            json!({
                "line": line,
                "text": text,
            }),
        ),
        TdmError::NonPrintableCharacter {
            line,
            keyword,
            column,
            character,
        } => tdm_node(
            "non_printable_character",
            json!({
                "line": line,
                "keyword": keyword,
                "column": column,
                "character": character.to_string(),
            }),
        ),
        TdmError::LineTooLong {
            line,
            keyword,
            length,
        } => tdm_node(
            "line_too_long",
            json!({
                "line": line,
                "keyword": keyword,
                "length": length,
            }),
        ),
        TdmError::MalformedEpoch {
            line,
            keyword,
            text,
        } => tdm_node(
            "malformed_epoch",
            json!({
                "line": line,
                "keyword": keyword,
                "text": text,
            }),
        ),
        TdmError::RecordsOutOfOrder {
            segment,
            keyword,
            epoch,
        } => tdm_node(
            "records_out_of_order",
            json!({
                "segment": segment,
                "keyword": keyword,
                "epoch": epoch,
            }),
        ),
        TdmError::DuplicateRecord {
            segment,
            keyword,
            epoch,
        } => tdm_node(
            "duplicate_record",
            json!({
                "segment": segment,
                "keyword": keyword,
                "epoch": epoch,
            }),
        ),
        TdmError::UnterminatedFinalLine { line } => tdm_node(
            "unterminated_final_line",
            json!({
                "line": line,
            }),
        ),
        TdmError::Unwritable { keyword, reason } => tdm_node(
            "unwritable",
            json!({
                "keyword": keyword,
                "reason": reason,
            }),
        ),
        TdmError::KeywordOutOfOrder {
            line,
            keyword,
            section,
        } => tdm_node(
            "keyword_out_of_order",
            json!({
                "line": line,
                "keyword": keyword,
                "section": section,
            }),
        ),
        TdmError::UndefinedParticipant {
            segment,
            keyword,
            index,
        } => tdm_node(
            "undefined_participant",
            json!({
                "segment": segment,
                "keyword": keyword,
                "index": index,
            }),
        ),
        TdmError::ConflictingKeyword {
            line,
            keyword,
            section,
            first,
            second,
        } => tdm_node(
            "conflicting_keyword",
            json!({
                "line": line,
                "keyword": keyword,
                "section": section,
                "first": first,
                "second": second,
            }),
        ),
        TdmError::RepeatedKeyword {
            line,
            keyword,
            section,
        } => tdm_node(
            "repeated_keyword",
            json!({
                "line": line,
                "keyword": keyword,
                "section": section,
            }),
        ),
        TdmError::UndefinedKeyword {
            line,
            keyword,
            section,
        } => tdm_node(
            "undefined_keyword",
            json!({
                "line": line,
                "keyword": keyword,
                "section": section,
            }),
        ),
        TdmError::MissingKeyword { keyword, segment } => tdm_node(
            "missing_keyword",
            json!({
                "keyword": keyword,
                "segment": segment,
            }),
        ),
        TdmError::EmptyDataSection { segment } => tdm_node(
            "empty_data_section",
            json!({
                "segment": segment,
            }),
        ),
        TdmError::EmptyValue { line, keyword } => tdm_node(
            "empty_value",
            json!({
                "line": line,
                "keyword": keyword,
            }),
        ),
        TdmError::InvalidVersion { line, value } => tdm_node(
            "invalid_version",
            json!({
                "line": line,
                "value": value,
            }),
        ),
        TdmError::KeywordNotAssignable { keyword } => tdm_node(
            "keyword_not_assignable",
            json!({
                "keyword": keyword,
            }),
        ),
        TdmError::MalformedRecord { line, keyword } => tdm_node(
            "malformed_record",
            json!({
                "line": line,
                "keyword": keyword,
            }),
        ),
        TdmError::InvalidField { keyword, kind } => tdm_node(
            "invalid_field",
            json!({
                "keyword": keyword,
                "kind": tdm_input_error_kind_name(*kind),
            }),
        ),
        _ => tdm_node(
            "unknown",
            json!({
                "message": error.to_string(),
            }),
        ),
    }
}

fn map_tdm_error(fn_name: &str, err: TdmError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Tdm,
        fn_name,
        tdm_error_value(&err),
    );
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use crate::SidereonStatus;
    use serde_json::Value;
    use std::ptr;

    const TDM_KVN_FIXTURE: &str = include_str!("../tests/fixtures/tdm/annex_e_18.kvn");

    #[test]
    fn table_driven_tdm_error_mapping() {
        let cases: Vec<(TdmError, &'static str)> = vec![
            (TdmError::NoSegments, "no_segments"),
            (
                TdmError::Section {
                    line: 5,
                    detail: "unexpected section",
                },
                "section",
            ),
            (
                TdmError::MalformedLine {
                    line: 12,
                    text: "NOT A LINE".into(),
                },
                "malformed_line",
            ),
            (
                TdmError::NonPrintableCharacter {
                    line: Some(8),
                    keyword: "COMMENT".into(),
                    column: 4,
                    character: '\x07',
                },
                "non_printable_character",
            ),
            (
                TdmError::NonPrintableCharacter {
                    line: None,
                    keyword: "DATA".into(),
                    column: 1,
                    character: '\0',
                },
                "non_printable_character",
            ),
            (
                TdmError::LineTooLong {
                    line: Some(2),
                    keyword: "COMMENT".into(),
                    length: 260,
                },
                "line_too_long",
            ),
            (
                TdmError::LineTooLong {
                    line: None,
                    keyword: "DATA".into(),
                    length: 255,
                },
                "line_too_long",
            ),
            (
                TdmError::MalformedEpoch {
                    line: Some(10),
                    keyword: "RECEIVE_FREQ".into(),
                    text: "bad-epoch".into(),
                },
                "malformed_epoch",
            ),
            (
                TdmError::MalformedEpoch {
                    line: None,
                    keyword: "RANGE".into(),
                    text: "bad-epoch-2".into(),
                },
                "malformed_epoch",
            ),
            (
                TdmError::RecordsOutOfOrder {
                    segment: 1,
                    keyword: "RANGE".into(),
                    epoch: "2026-01-01T00:00:00".into(),
                },
                "records_out_of_order",
            ),
            (
                TdmError::DuplicateRecord {
                    segment: 2,
                    keyword: "DOPPLER".into(),
                    epoch: "2026-01-01T00:00:00".into(),
                },
                "duplicate_record",
            ),
            (
                TdmError::UnterminatedFinalLine { line: 99 },
                "unterminated_final_line",
            ),
            (
                TdmError::Unwritable {
                    keyword: "COMMENT".into(),
                    reason: "non-ascii",
                },
                "unwritable",
            ),
            (
                TdmError::KeywordOutOfOrder {
                    line: Some(14),
                    keyword: "MODE".into(),
                    section: "metadata",
                },
                "keyword_out_of_order",
            ),
            (
                TdmError::KeywordOutOfOrder {
                    line: None,
                    keyword: "MODE".into(),
                    section: "header",
                },
                "keyword_out_of_order",
            ),
            (
                TdmError::UndefinedParticipant {
                    segment: 1,
                    keyword: "PATH".into(),
                    index: 3,
                },
                "undefined_participant",
            ),
            (
                TdmError::ConflictingKeyword {
                    line: Some(6),
                    keyword: "TIME_SYSTEM".into(),
                    section: "metadata",
                    first: "UTC".into(),
                    second: "TAI".into(),
                },
                "conflicting_keyword",
            ),
            (
                TdmError::ConflictingKeyword {
                    line: None,
                    keyword: "TIME_SYSTEM".into(),
                    section: "header",
                    first: "UTC".into(),
                    second: "GPS".into(),
                },
                "conflicting_keyword",
            ),
            (
                TdmError::RepeatedKeyword {
                    line: Some(7),
                    keyword: "START_TIME".into(),
                    section: "metadata",
                },
                "repeated_keyword",
            ),
            (
                TdmError::RepeatedKeyword {
                    line: None,
                    keyword: "START_TIME".into(),
                    section: "metadata",
                },
                "repeated_keyword",
            ),
            (
                TdmError::UndefinedKeyword {
                    line: 22,
                    keyword: "UNKNOWN_KW".into(),
                    section: "header",
                },
                "undefined_keyword",
            ),
            (
                TdmError::MissingKeyword {
                    keyword: "TIME_SYSTEM".into(),
                    segment: Some(1),
                },
                "missing_keyword",
            ),
            (
                TdmError::MissingKeyword {
                    keyword: "CCSDS_TDM_VERS".into(),
                    segment: None,
                },
                "missing_keyword",
            ),
            (
                TdmError::EmptyDataSection { segment: 1 },
                "empty_data_section",
            ),
            (
                TdmError::EmptyValue {
                    line: Some(9),
                    keyword: "PARTICIPANT_1".into(),
                },
                "empty_value",
            ),
            (
                TdmError::EmptyValue {
                    line: None,
                    keyword: "PARTICIPANT_2".into(),
                },
                "empty_value",
            ),
            (
                TdmError::InvalidVersion {
                    line: Some(1),
                    value: "3.0".into(),
                },
                "invalid_version",
            ),
            (
                TdmError::InvalidVersion {
                    line: None,
                    value: "9.9".into(),
                },
                "invalid_version",
            ),
            (
                TdmError::KeywordNotAssignable {
                    keyword: "DATA_START".into(),
                },
                "keyword_not_assignable",
            ),
            (
                TdmError::MalformedRecord {
                    line: 33,
                    keyword: "RECEIVE_FREQ".into(),
                },
                "malformed_record",
            ),
            (
                TdmError::InvalidField {
                    keyword: "TRANSMIT_FREQ".into(),
                    kind: TdmInputErrorKind::NotPositive,
                },
                "invalid_field",
            ),
        ];

        for (err, expected_kind) in cases {
            let val = tdm_error_value(&err);
            assert_eq!(val["kind"], expected_kind);
            match &err {
                TdmError::NoSegments => {
                    assert_eq!(val["fields"], json!({}));
                }
                TdmError::Section { line, detail } => {
                    assert_eq!(val["fields"]["line"], *line);
                    assert_eq!(val["fields"]["detail"], *detail);
                }
                TdmError::MalformedLine { line, text } => {
                    assert_eq!(val["fields"]["line"], *line);
                    assert_eq!(val["fields"]["text"], text.as_str());
                }
                TdmError::NonPrintableCharacter {
                    line,
                    keyword,
                    column,
                    character,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["column"], *column);
                    assert_eq!(val["fields"]["character"], character.to_string());
                }
                TdmError::LineTooLong {
                    line,
                    keyword,
                    length,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["length"], *length);
                }
                TdmError::MalformedEpoch {
                    line,
                    keyword,
                    text,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["text"], text.as_str());
                }
                TdmError::RecordsOutOfOrder {
                    segment,
                    keyword,
                    epoch,
                } => {
                    assert_eq!(val["fields"]["segment"], *segment);
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["epoch"], epoch.as_str());
                }
                TdmError::DuplicateRecord {
                    segment,
                    keyword,
                    epoch,
                } => {
                    assert_eq!(val["fields"]["segment"], *segment);
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["epoch"], epoch.as_str());
                }
                TdmError::UnterminatedFinalLine { line } => {
                    assert_eq!(val["fields"]["line"], *line);
                }
                TdmError::Unwritable { keyword, reason } => {
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["reason"], *reason);
                }
                TdmError::KeywordOutOfOrder {
                    line,
                    keyword,
                    section,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["section"], *section);
                }
                TdmError::UndefinedParticipant {
                    segment,
                    keyword,
                    index,
                } => {
                    assert_eq!(val["fields"]["segment"], *segment);
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["index"], *index);
                }
                TdmError::ConflictingKeyword {
                    line,
                    keyword,
                    section,
                    first,
                    second,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["section"], *section);
                    assert_eq!(val["fields"]["first"], first.as_str());
                    assert_eq!(val["fields"]["second"], second.as_str());
                }
                TdmError::RepeatedKeyword {
                    line,
                    keyword,
                    section,
                } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["section"], *section);
                }
                TdmError::UndefinedKeyword {
                    line,
                    keyword,
                    section,
                } => {
                    assert_eq!(val["fields"]["line"], *line);
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["section"], *section);
                }
                TdmError::MissingKeyword { keyword, segment } => {
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["segment"], json!(segment));
                }
                TdmError::EmptyDataSection { segment } => {
                    assert_eq!(val["fields"]["segment"], *segment);
                }
                TdmError::EmptyValue { line, keyword } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                }
                TdmError::InvalidVersion { line, value } => {
                    assert_eq!(val["fields"]["line"], json!(line));
                    assert_eq!(val["fields"]["value"], value.as_str());
                }
                TdmError::KeywordNotAssignable { keyword } => {
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                }
                TdmError::MalformedRecord { line, keyword } => {
                    assert_eq!(val["fields"]["line"], *line);
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                }
                TdmError::InvalidField { keyword, kind } => {
                    assert_eq!(val["fields"]["keyword"], keyword.as_str());
                    assert_eq!(val["fields"]["kind"], tdm_input_error_kind_name(*kind));
                }
                _ => {}
            }
        }

        // Exhaustively verify all TdmInputErrorKind variants
        let input_kinds = [
            (TdmInputErrorKind::Missing, "missing"),
            (TdmInputErrorKind::FloatParse, "float_parse"),
            (TdmInputErrorKind::NonFinite, "non_finite"),
            (TdmInputErrorKind::NotPositive, "not_positive"),
            (TdmInputErrorKind::OutOfRange, "out_of_range"),
            (TdmInputErrorKind::InvalidIndex, "invalid_index"),
            (TdmInputErrorKind::UnknownKeyword, "unknown_keyword"),
            (TdmInputErrorKind::UnexpectedUnit, "unexpected_unit"),
            (TdmInputErrorKind::NonInteger, "non_integer"),
            (TdmInputErrorKind::Negative, "negative"),
            (TdmInputErrorKind::NegativeZero, "negative_zero"),
            (TdmInputErrorKind::UnitMismatch, "unit_mismatch"),
            (TdmInputErrorKind::DecimalMismatch, "decimal_mismatch"),
        ];
        for (kind, name) in input_kinds {
            assert_eq!(tdm_input_error_kind_name(kind), name);
            let val = tdm_error_value(&TdmError::InvalidField {
                keyword: "FREQ".into(),
                kind,
            });
            assert_eq!(val["fields"]["kind"], name);
        }

        assert_eq!(TdmError::NoSegments.to_string(), "missing TDM segment");
    }

    #[test]
    fn tdm_public_producer_control_and_refusals() {
        clear_engine_error();

        // Valid control: parse KVN fixture, count segments, serialize to KVN, free
        unsafe {
            let mut tdm: *mut SidereonTdm = ptr::null_mut();
            assert_eq!(
                sidereon_tdm_parse_kvn(TDM_KVN_FIXTURE.as_ptr(), TDM_KVN_FIXTURE.len(), &mut tdm,),
                SidereonStatus::Ok
            );
            assert!(!tdm.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Tdm,
                payload_len: 42,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Segment count
            let mut seg_count = 0;
            assert_eq!(
                sidereon_tdm_segment_count(tdm, &mut seg_count),
                SidereonStatus::Ok
            );
            assert_eq!(seg_count, 2);

            // Serialize to KVN
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_tdm_to_kvn(tdm, ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert!(required > 0);
            let mut buf = vec![0u8; required];
            assert_eq!(
                sidereon_tdm_to_kvn(
                    tdm,
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, required);

            sidereon_tdm_free(tdm);
        }

        // Real public refusal: malformed KVN in sidereon_tdm_parse_kvn
        unsafe {
            let mut tdm: *mut SidereonTdm = ptr::null_mut();
            let bad_kvn = b"CCSDS_TDM_VERS = 2.0\nNOT_A_VALID_KVN_LINE\n";
            assert_eq!(
                sidereon_tdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), &mut tdm),
                SidereonStatus::InvalidArgument
            );
            assert!(tdm.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Tdm);
            assert!(info.payload_len > 0);

            let mut written = 0;
            let mut required = 0;
            let mut buf = vec![0u8; info.payload_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "tdm");
            assert_eq!(payload["operation"], "sidereon_tdm_parse_kvn");
            assert_eq!(payload["error"]["kind"], "malformed_line");
            assert_eq!(payload["error"]["fields"]["line"], 2);
        }
    }

    #[test]
    fn tdm_producer_early_clearing_and_retention() {
        clear_engine_error();

        unsafe {
            // Seed engine error via a real refusal
            let mut tdm: *mut SidereonTdm = ptr::null_mut();
            let bad_kvn = b"CCSDS_TDM_VERS = 2.0\nNOT_A_VALID_LINE\n";
            assert_eq!(
                sidereon_tdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), &mut tdm),
                SidereonStatus::InvalidArgument
            );

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Tdm);
            let expected_len = info.payload_len;
            assert!(expected_len > 0);

            // Free retains the error
            sidereon_tdm_free(ptr::null_mut());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Tdm);
            assert_eq!(info.payload_len, expected_len);

            // Segment count retains the error
            let mut seg_count = 0;
            assert_eq!(
                sidereon_tdm_segment_count(ptr::null(), &mut seg_count),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Tdm);
            assert_eq!(info.payload_len, expected_len);

            // Two-pass payload retrieval: Pass 1 query length
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument and retains
            let mut short_buf = vec![0u8; expected_len - 1];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    short_buf.as_mut_ptr(),
                    short_buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Pass 2: Exact buffer query succeeds
            let mut buf = vec![0u8; expected_len];
            assert_eq!(
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert_eq!(written, expected_len);
            assert_eq!(required, expected_len);

            // Record is still retained after read
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Tdm);

            // Early-validation clearing: null out pointer on sidereon_tdm_parse_kvn
            assert_eq!(
                sidereon_tdm_parse_kvn(bad_kvn.as_ptr(), bad_kvn.len(), ptr::null_mut()),
                SidereonStatus::NullPointer
            );

            // The producer operation boundary cleared the slot before early checks!
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
