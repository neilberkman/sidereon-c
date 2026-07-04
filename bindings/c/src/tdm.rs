use super::*;

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
    ffi_boundary("sidereon_tdm_parse_kvn", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_tdm, "sidereon_tdm_parse_kvn", "out_tdm"));
        *out = ptr::null_mut();
        let text = c_try!(ndm_text_from_utf8(data, len, "sidereon_tdm_parse_kvn"));
        match sidereon_core::astro::tdm::parse_kvn(text) {
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
    ffi_boundary("sidereon_tdm_to_kvn", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_tdm_to_kvn",
            out_written,
            out_required
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
        c_try!(init_copy_counts(
            "sidereon_tdm_segments",
            out_written,
            out_required
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
        c_try!(init_copy_counts(
            "sidereon_tdm_participants",
            out_written,
            out_required
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
        c_try!(init_copy_counts(
            "sidereon_tdm_paths",
            out_written,
            out_required
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
        c_try!(init_copy_counts(
            "sidereon_tdm_records",
            out_written,
            out_required
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

fn map_tdm_error(fn_name: &str, err: sidereon_core::astro::tdm::TdmError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}
