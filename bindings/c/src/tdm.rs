use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::json;
use sidereon_core::astro::tdm::{
    TdmDeparture, TdmError, TdmInputErrorKind, TdmLeniency, TdmPolicy, TdmWarning, TdmWritePolicy,
};

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

/// Owned warnings returned by policy-aware parsing.
pub struct SidereonTdmWarnings {
    inner: Vec<TdmWarning>,
}

/// Owned departures returned by policy-aware writing or validation.
pub struct SidereonTdmDepartures {
    inner: Vec<TdmDeparture>,
}

/// Refuse a TDM departure.
pub const SIDEREON_TDM_LENIENCY_STRICT: u32 = 0;
/// Accept and report a TDM departure.
pub const SIDEREON_TDM_LENIENCY_FORGIVE: u32 = 1;

/// Policy for parsing TDM KVN.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonTdmPolicy {
    pub non_printable: u32,
    pub missing_keywords: u32,
    pub long_lines: u32,
    pub empty_data_sections: u32,
    pub record_order: u32,
    pub duplicate_records: u32,
    pub keyword_order: u32,
    pub final_terminator: u32,
}

/// Policy for encoding or validating a TDM.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonTdmWritePolicy {
    pub non_printable: u32,
    pub missing_keywords: u32,
    pub long_lines: u32,
    pub empty_data_sections: u32,
    pub record_order: u32,
    pub duplicate_records: u32,
    pub keyword_order: u32,
    pub final_terminator: u32,
    pub repeated_keywords: u32,
}

/// Positioned TDM comment summary. Read its lossless text by row index.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmComment {
    pub segment_index: usize,
    /// 0 header, 1 metadata, 2 data.
    pub section: u32,
    pub before_record: usize,
    pub text_len: usize,
}

/// Raw ordered TDM field summary. Read its lossless key/value by row index.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmField {
    pub segment_index: usize,
    /// 0 header, 1 metadata.
    pub section: u32,
    pub key_len: usize,
    pub value_len: usize,
}

/// Complete TDM header summary. Read scalar text with sidereon_tdm_header_text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmHeader {
    pub version_len: usize,
    pub has_creation_date: bool,
    pub creation_date_len: usize,
    pub has_originator: bool,
    pub originator_len: usize,
    pub has_message_id: bool,
    pub message_id_len: usize,
    pub comment_count: usize,
    pub field_count: usize,
    pub segment_count: usize,
}

/// Borrowed UTF-8 input text. A NULL pointer is valid only when len is zero.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmText {
    pub data: *const u8,
    pub len: usize,
}

/// Optional borrowed UTF-8 input text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmOptionalText {
    pub has_value: bool,
    pub value: SidereonTdmText,
}

/// Lossless scalar header input.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmHeaderInput {
    pub version: SidereonTdmText,
    pub creation_date: SidereonTdmOptionalText,
    pub originator: SidereonTdmOptionalText,
    pub message_id: SidereonTdmOptionalText,
}

/// Positioned comment input. Mutation functions read only these fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmCommentInput {
    pub text: SidereonTdmText,
    pub before_record: usize,
}

/// Ordered key/value field input. Mutation functions read only these fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmFieldInput {
    pub key: SidereonTdmText,
    pub value: SidereonTdmText,
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

/// Lossless input for constructing or inserting a TDM data record.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTdmDataRecordInput {
    pub observable: u32,
    pub has_observable_participant: bool,
    pub observable_participant: u8,
    pub observable_name: SidereonTdmText,
    pub unit: u32,
    pub unit_label: SidereonTdmText,
    pub keyword: SidereonTdmText,
    pub epoch: SidereonTdmText,
    pub value_text: SidereonTdmText,
    pub value: f64,
}

fn leniency_to_core(
    value: u32,
    fn_name: &str,
    operation_class: &str,
    field: &str,
) -> Result<TdmLeniency, SidereonStatus> {
    match value {
        SIDEREON_TDM_LENIENCY_STRICT => Ok(TdmLeniency::Strict),
        SIDEREON_TDM_LENIENCY_FORGIVE => Ok(TdmLeniency::Forgive),
        _ => Err(map_tdm_policy_error(fn_name, operation_class, field, value)),
    }
}

fn policy_to_core(value: SidereonTdmPolicy, fn_name: &str) -> Result<TdmPolicy, SidereonStatus> {
    Ok(TdmPolicy::strict()
        .with_non_printable(leniency_to_core(
            value.non_printable,
            fn_name,
            "parse",
            "non_printable",
        )?)
        .with_missing_keywords(leniency_to_core(
            value.missing_keywords,
            fn_name,
            "parse",
            "missing_keywords",
        )?)
        .with_long_lines(leniency_to_core(
            value.long_lines,
            fn_name,
            "parse",
            "long_lines",
        )?)
        .with_empty_data_sections(leniency_to_core(
            value.empty_data_sections,
            fn_name,
            "parse",
            "empty_data_sections",
        )?)
        .with_record_order(leniency_to_core(
            value.record_order,
            fn_name,
            "parse",
            "record_order",
        )?)
        .with_duplicate_records(leniency_to_core(
            value.duplicate_records,
            fn_name,
            "parse",
            "duplicate_records",
        )?)
        .with_keyword_order(leniency_to_core(
            value.keyword_order,
            fn_name,
            "parse",
            "keyword_order",
        )?)
        .with_final_terminator(leniency_to_core(
            value.final_terminator,
            fn_name,
            "parse",
            "final_terminator",
        )?))
}

fn write_policy_to_core(
    value: SidereonTdmWritePolicy,
    fn_name: &str,
    operation_class: &str,
) -> Result<TdmWritePolicy, SidereonStatus> {
    Ok(TdmWritePolicy::strict()
        .with_non_printable(leniency_to_core(
            value.non_printable,
            fn_name,
            operation_class,
            "non_printable",
        )?)
        .with_missing_keywords(leniency_to_core(
            value.missing_keywords,
            fn_name,
            operation_class,
            "missing_keywords",
        )?)
        .with_long_lines(leniency_to_core(
            value.long_lines,
            fn_name,
            operation_class,
            "long_lines",
        )?)
        .with_empty_data_sections(leniency_to_core(
            value.empty_data_sections,
            fn_name,
            operation_class,
            "empty_data_sections",
        )?)
        .with_record_order(leniency_to_core(
            value.record_order,
            fn_name,
            operation_class,
            "record_order",
        )?)
        .with_duplicate_records(leniency_to_core(
            value.duplicate_records,
            fn_name,
            operation_class,
            "duplicate_records",
        )?)
        .with_keyword_order(leniency_to_core(
            value.keyword_order,
            fn_name,
            operation_class,
            "keyword_order",
        )?)
        .with_final_terminator(leniency_to_core(
            value.final_terminator,
            fn_name,
            operation_class,
            "final_terminator",
        )?)
        .with_repeated_keywords(leniency_to_core(
            value.repeated_keywords,
            fn_name,
            operation_class,
            "repeated_keywords",
        )?))
}

fn policy_from_core(value: TdmPolicy) -> SidereonTdmPolicy {
    let cvt = |v| match v {
        TdmLeniency::Strict => SIDEREON_TDM_LENIENCY_STRICT,
        TdmLeniency::Forgive => SIDEREON_TDM_LENIENCY_FORGIVE,
    };
    SidereonTdmPolicy {
        non_printable: cvt(value.non_printable),
        missing_keywords: cvt(value.missing_keywords),
        long_lines: cvt(value.long_lines),
        empty_data_sections: cvt(value.empty_data_sections),
        record_order: cvt(value.record_order),
        duplicate_records: cvt(value.duplicate_records),
        keyword_order: cvt(value.keyword_order),
        final_terminator: cvt(value.final_terminator),
    }
}

fn write_policy_from_core(value: TdmWritePolicy) -> SidereonTdmWritePolicy {
    let cvt = |v| match v {
        TdmLeniency::Strict => SIDEREON_TDM_LENIENCY_STRICT,
        TdmLeniency::Forgive => SIDEREON_TDM_LENIENCY_FORGIVE,
    };
    SidereonTdmWritePolicy {
        non_printable: cvt(value.non_printable),
        missing_keywords: cvt(value.missing_keywords),
        long_lines: cvt(value.long_lines),
        empty_data_sections: cvt(value.empty_data_sections),
        record_order: cvt(value.record_order),
        duplicate_records: cvt(value.duplicate_records),
        keyword_order: cvt(value.keyword_order),
        final_terminator: cvt(value.final_terminator),
        repeated_keywords: cvt(value.repeated_keywords),
    }
}

/// Return the strict/default read policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_policy_strict() -> SidereonTdmPolicy {
    policy_from_core(TdmPolicy::strict())
}

/// Return the default read policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_policy_default() -> SidereonTdmPolicy {
    policy_from_core(TdmPolicy::default())
}

/// Return the fully lenient read policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_policy_lenient() -> SidereonTdmPolicy {
    policy_from_core(TdmPolicy::lenient())
}

/// Return the strict/default write policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_write_policy_strict() -> SidereonTdmWritePolicy {
    write_policy_from_core(TdmWritePolicy::strict())
}

/// Return the default write policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_write_policy_default() -> SidereonTdmWritePolicy {
    write_policy_from_core(TdmWritePolicy::default())
}

/// Return the fully lenient write policy.
#[no_mangle]
pub extern "C" fn sidereon_tdm_write_policy_lenient() -> SidereonTdmWritePolicy {
    write_policy_from_core(TdmWritePolicy::lenient())
}

/// Convert a write policy to its matching read policy.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_write_policy_as_read(
    policy: SidereonTdmWritePolicy,
    out_policy: *mut SidereonTdmPolicy,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tdm_write_policy_as_read",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_policy,
                "sidereon_tdm_write_policy_as_read",
                "out_policy"
            ));
            *out = sidereon_tdm_policy_default();
            let policy = c_try!(write_policy_to_core(
                policy,
                "sidereon_tdm_write_policy_as_read",
                "validation"
            ));
            *out = policy_from_core(policy.as_read());
            SidereonStatus::Ok
        },
    )
}

/// Parse KVN under an explicit policy and return an owned warning list.
/// Both output handles are initialized to NULL on failure.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_parse_kvn_with_policy(
    data: *const u8,
    len: usize,
    policy: SidereonTdmPolicy,
    out_tdm: *mut *mut SidereonTdm,
    out_warnings: *mut *mut SidereonTdmWarnings,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tdm_parse_kvn_with_policy",
        SidereonStatus::Panic,
        || {
            let out_tdm = c_try!(require_out(
                out_tdm,
                "sidereon_tdm_parse_kvn_with_policy",
                "out_tdm"
            ));
            let out_warnings = c_try!(require_out(
                out_warnings,
                "sidereon_tdm_parse_kvn_with_policy",
                "out_warnings"
            ));
            *out_tdm = ptr::null_mut();
            *out_warnings = ptr::null_mut();
            let text = c_try!(ndm_text_from_utf8(
                data,
                len,
                "sidereon_tdm_parse_kvn_with_policy"
            ));
            let policy = c_try!(policy_to_core(policy, "sidereon_tdm_parse_kvn_with_policy"));
            match sidereon_core::astro::tdm::parse_kvn_with_policy(text, policy) {
                Ok((inner, warnings)) => {
                    write_boxed_handle(out_tdm, SidereonTdm { inner });
                    write_boxed_handle(out_warnings, SidereonTdmWarnings { inner: warnings });
                    SidereonStatus::Ok
                }
                Err(err) => map_tdm_error_class("sidereon_tdm_parse_kvn_with_policy", "parse", err),
            }
        },
    )
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
            Err(err) => map_tdm_error_class("sidereon_tdm_parse_kvn", "parse", err),
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
            Err(err) => return map_tdm_error_class("sidereon_tdm_to_kvn", "write", err),
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

/// Encode KVN under an explicit write policy and return owned departures.
/// The output text is not null-terminated and follows the two-call buffer contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_to_kvn_with_policy(
    tdm: *const SidereonTdm,
    policy: SidereonTdmWritePolicy,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_departures: *mut *mut SidereonTdmDepartures,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tdm_to_kvn_with_policy",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_to_kvn_with_policy",
                out_written,
                "out_written",
                tdm,
                "tdm"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_to_kvn_with_policy",
                out_required,
                "out_required",
                tdm,
                "tdm"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_to_kvn_with_policy",
                out_departures,
                "out_departures",
                tdm,
                "tdm"
            ));
            c_try!(init_copy_counts(
                "sidereon_tdm_to_kvn_with_policy",
                out_written,
                out_required
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_tdm_to_kvn_with_policy",
                &[
                    (out.cast(), size_of::<u8>(), len, "out"),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required"),
                    (
                        out_departures.cast(),
                        size_of::<*mut SidereonTdmDepartures>(),
                        1,
                        "out_departures"
                    )
                ],
                tdm,
                "tdm",
            ));
            let departures_out = c_try!(require_out(
                out_departures,
                "sidereon_tdm_to_kvn_with_policy",
                "out_departures"
            ));
            *departures_out = ptr::null_mut();
            let tdm = c_try!(require_ref(tdm, "sidereon_tdm_to_kvn_with_policy", "tdm"));
            let policy = c_try!(write_policy_to_core(
                policy,
                "sidereon_tdm_to_kvn_with_policy",
                "write"
            ));
            let (text, departures) =
                match sidereon_core::astro::tdm::encode_kvn_with_policy(&tdm.inner, policy) {
                    Ok(result) => result,
                    Err(err) => {
                        return map_tdm_error_class("sidereon_tdm_to_kvn_with_policy", "write", err)
                    }
                };
            c_try!(copy_prefix_to_c(
                "sidereon_tdm_to_kvn_with_policy",
                "out",
                text.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            write_boxed_handle(departures_out, SidereonTdmDepartures { inner: departures });
            SidereonStatus::Ok
        },
    )
}

/// Number of retained parse warnings.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_warnings_count(
    warnings: *const SidereonTdmWarnings,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_warnings_count", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_warnings_count",
            out_count,
            "out_count",
            warnings,
            "warnings"
        ));
        let out = c_try!(require_out(
            out_count,
            "sidereon_tdm_warnings_count",
            "out_count"
        ));
        *out = 0;
        let warnings = c_try!(require_ref(
            warnings,
            "sidereon_tdm_warnings_count",
            "warnings"
        ));
        *out = warnings.inner.len();
        SidereonStatus::Ok
    })
}

/// Number of retained write/validation departures.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_departures_count(
    departures: *const SidereonTdmDepartures,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tdm_departures_count",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_departures_count",
                out_count,
                "out_count",
                departures,
                "departures"
            ));
            let out = c_try!(require_out(
                out_count,
                "sidereon_tdm_departures_count",
                "out_count"
            ));
            *out = 0;
            let departures = c_try!(require_ref(
                departures,
                "sidereon_tdm_departures_count",
                "departures"
            ));
            *out = departures.inner.len();
            SidereonStatus::Ok
        },
    )
}

unsafe fn copy_tdm_diagnostic(
    fn_name: &str,
    value: String,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        value.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

/// Copy one warning as UTF-8 JSON containing its kind, typed fields, and message.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_warning_detail(
    warnings: *const SidereonTdmWarnings,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_warning_detail", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_warning_detail",
            out_written,
            "out_written",
            warnings,
            "warnings"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_warning_detail",
            out_required,
            "out_required",
            warnings,
            "warnings"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_warning_detail",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_warning_detail",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            warnings,
            "warnings"
        ));
        let warnings = c_try!(require_ref(
            warnings,
            "sidereon_tdm_warning_detail",
            "warnings"
        ));
        let Some(warning) = warnings.inner.get(index) else {
            return invalid_argument("sidereon_tdm_warning_detail", "index is out of range");
        };
        copy_tdm_diagnostic(
            "sidereon_tdm_warning_detail",
            tdm_warning_value(warning).to_string(),
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy one departure as UTF-8 JSON containing its kind, typed fields, and message.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_departure_detail(
    departures: *const SidereonTdmDepartures,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tdm_departure_detail",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_departure_detail",
                out_written,
                "out_written",
                departures,
                "departures"
            ));
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_departure_detail",
                out_required,
                "out_required",
                departures,
                "departures"
            ));
            c_try!(init_copy_counts(
                "sidereon_tdm_departure_detail",
                out_written,
                out_required
            ));
            c_try!(reject_outputs_overlapping_handle(
                "sidereon_tdm_departure_detail",
                &[
                    (out.cast(), size_of::<u8>(), len, "out"),
                    (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                    (out_required.cast(), size_of::<usize>(), 1, "out_required")
                ],
                departures,
                "departures"
            ));
            let departures = c_try!(require_ref(
                departures,
                "sidereon_tdm_departure_detail",
                "departures"
            ));
            let Some(departure) = departures.inner.get(index) else {
                return invalid_argument("sidereon_tdm_departure_detail", "index is out of range");
            };
            copy_tdm_diagnostic(
                "sidereon_tdm_departure_detail",
                tdm_departure_value(departure).to_string(),
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Release a warning list. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_warnings_free(warnings: *mut SidereonTdmWarnings) {
    ffi_boundary("sidereon_tdm_warnings_free", (), || free_boxed(warnings));
}

/// Release a departure list. NULL is allowed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_departures_free(departures: *mut SidereonTdmDepartures) {
    ffi_boundary("sidereon_tdm_departures_free", (), || {
        free_boxed(departures)
    });
}

/// Construct an empty mutable TDM with the supplied version. Segments may be
/// appended before encoding; strict encoding refuses an incomplete message.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_create(
    version: *const u8,
    version_len: usize,
    out_tdm: *mut *mut SidereonTdm,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_create", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_tdm, "sidereon_tdm_create", "out_tdm"));
        *out = ptr::null_mut();
        let version = c_try!(ndm_text_from_utf8(
            version,
            version_len,
            "sidereon_tdm_create"
        ));
        write_boxed_handle(
            out,
            SidereonTdm {
                inner: sidereon_core::astro::tdm::Tdm {
                    version: version.to_owned(),
                    comments: Vec::new(),
                    creation_date: None,
                    originator: None,
                    message_id: None,
                    header_fields: Vec::new(),
                    segments: Vec::new(),
                },
            },
        );
        SidereonStatus::Ok
    })
}

/// Read the complete header summary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_header(
    tdm: *const SidereonTdm,
    out_header: *mut SidereonTdmHeader,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_header", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_header",
            out_header,
            "out_header",
            tdm,
            "tdm"
        ));
        let out = c_try!(require_out(out_header, "sidereon_tdm_header", "out_header"));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_header", "tdm"));
        *out = SidereonTdmHeader {
            version_len: tdm.inner.version.len(),
            has_creation_date: tdm.inner.creation_date.is_some(),
            creation_date_len: tdm.inner.creation_date.as_ref().map_or(0, String::len),
            has_originator: tdm.inner.originator.is_some(),
            originator_len: tdm.inner.originator.as_ref().map_or(0, String::len),
            has_message_id: tdm.inner.message_id.is_some(),
            message_id_len: tdm.inner.message_id.as_ref().map_or(0, String::len),
            comment_count: tdm.inner.comments.len(),
            field_count: tdm.inner.header_fields.len(),
            segment_count: tdm.inner.segments.len(),
        };
        SidereonStatus::Ok
    })
}

/// Atomically replace scalar header values. Comments, fields, and segments are retained.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_set_header(
    tdm: *mut SidereonTdm,
    header: SidereonTdmHeaderInput,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_set_header", SidereonStatus::Panic, || {
        let version = c_try!(tdm_text_from_c(
            "sidereon_tdm_set_header",
            "header.version",
            header.version
        ));
        let creation_date = c_try!(tdm_optional_text_from_c(
            "sidereon_tdm_set_header",
            "header.creation_date",
            header.creation_date
        ));
        let originator = c_try!(tdm_optional_text_from_c(
            "sidereon_tdm_set_header",
            "header.originator",
            header.originator
        ));
        let message_id = c_try!(tdm_optional_text_from_c(
            "sidereon_tdm_set_header",
            "header.message_id",
            header.message_id
        ));
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_set_header", "tdm"));
        tdm.inner.version = version;
        tdm.inner.creation_date = creation_date;
        tdm.inner.originator = originator;
        tdm.inner.message_id = message_id;
        SidereonStatus::Ok
    })
}

/// Copy one scalar header text: 0 version, 1 creation date, 2 originator, 3 message ID.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_header_text(
    tdm: *const SidereonTdm,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_header_text", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_header_text",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_header_text",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_header_text",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_header_text",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_header_text", "tdm"));
        let value = match field {
            0 => Some(tdm.inner.version.as_str()),
            1 => tdm.inner.creation_date.as_deref(),
            2 => tdm.inner.originator.as_deref(),
            3 => tdm.inner.message_id.as_deref(),
            _ => return invalid_argument("sidereon_tdm_header_text", "field is invalid"),
        };
        let Some(value) = value else {
            return invalid_argument("sidereon_tdm_header_text", "field is absent");
        };
        copy_tdm_text(
            "sidereon_tdm_header_text",
            value,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy all positioned comments (header, metadata, then data).
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_comments(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmComment,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_comments", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_comments",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_comments",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_comments",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_comments",
            &[
                (out.cast(), size_of::<SidereonTdmComment>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_comments", "tdm"));
        let mut rows = Vec::new();
        rows.extend(tdm.inner.comments.iter().map(|c| tdm_comment_to_c(0, 0, c)));
        for (index, segment) in tdm.inner.segments.iter().enumerate() {
            rows.extend(
                segment
                    .metadata
                    .comments
                    .iter()
                    .map(|c| tdm_comment_to_c(index, 1, c)),
            );
            rows.extend(
                segment
                    .data
                    .comments
                    .iter()
                    .map(|c| tdm_comment_to_c(index, 2, c)),
            );
        }
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_comments",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the lossless text of a row returned by sidereon_tdm_comments.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_comment_text(
    tdm: *const SidereonTdm,
    row_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_comment_text", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_comment_text",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_comment_text",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_comment_text",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_comment_text",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_comment_text", "tdm"));
        let mut rows: Vec<&str> = tdm.inner.comments.iter().map(|c| c.text.as_str()).collect();
        for segment in &tdm.inner.segments {
            rows.extend(segment.metadata.comments.iter().map(|c| c.text.as_str()));
            rows.extend(segment.data.comments.iter().map(|c| c.text.as_str()));
        }
        let Some(value) = rows.get(row_index) else {
            return invalid_argument("sidereon_tdm_comment_text", "row_index is out of range");
        };
        copy_tdm_text(
            "sidereon_tdm_comment_text",
            value,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy all raw fields (header, then each metadata block).
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_fields(
    tdm: *const SidereonTdm,
    out: *mut SidereonTdmField,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_fields", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_fields",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_fields",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_fields",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_fields",
            &[
                (out.cast(), size_of::<SidereonTdmField>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_fields", "tdm"));
        let mut rows = Vec::new();
        rows.extend(
            tdm.inner
                .header_fields
                .iter()
                .map(|f| tdm_field_to_c(0, 0, f)),
        );
        for (index, segment) in tdm.inner.segments.iter().enumerate() {
            rows.extend(
                segment
                    .metadata
                    .fields
                    .iter()
                    .map(|f| tdm_field_to_c(index, 1, f)),
            );
        }
        c_try!(copy_prefix_to_c(
            "sidereon_tdm_fields",
            "out",
            &rows,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy a lossless key or value from a row returned by sidereon_tdm_fields.
/// part 0 selects the key and part 1 selects the value.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_field_text(
    tdm: *const SidereonTdm,
    row_index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_field_text", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_field_text",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_field_text",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_field_text",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_field_text",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_field_text", "tdm"));
        let mut rows: Vec<&sidereon_core::astro::tdm::TdmField> =
            tdm.inner.header_fields.iter().collect();
        for segment in &tdm.inner.segments {
            rows.extend(segment.metadata.fields.iter());
        }
        let Some(field) = rows.get(row_index) else {
            return invalid_argument("sidereon_tdm_field_text", "row_index is out of range");
        };
        let value = match part {
            0 => field.key.as_str(),
            1 => field.value.as_str(),
            _ => return invalid_argument("sidereon_tdm_field_text", "part is invalid"),
        };
        copy_tdm_text(
            "sidereon_tdm_field_text",
            value,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Replace header comments and raw header fields. Scalar header values and segments are retained.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_replace_header_raw(
    tdm: *mut SidereonTdm,
    comments: *const SidereonTdmCommentInput,
    comment_count: usize,
    fields: *const SidereonTdmFieldInput,
    field_count: usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_tdm_replace_header_raw",
        SidereonStatus::Panic,
        || {
            let comments = c_try!(require_slice(
                comments,
                comment_count,
                "sidereon_tdm_replace_header_raw",
                "comments"
            ));
            let fields = c_try!(require_slice(
                fields,
                field_count,
                "sidereon_tdm_replace_header_raw",
                "fields"
            ));
            let comments = c_try!(tdm_comments_from_c(
                "sidereon_tdm_replace_header_raw",
                comments
            ));
            let fields = c_try!(tdm_fields_from_c("sidereon_tdm_replace_header_raw", fields));
            let tdm = c_try!(require_mut(tdm, "sidereon_tdm_replace_header_raw", "tdm"));
            tdm.inner.comments = comments;
            tdm.inner.header_fields = fields;
            SidereonStatus::Ok
        },
    )
}

/// Append a segment from raw metadata fields, positioned metadata/data comments,
/// and data records. Core validation is atomic and uses the supplied policy.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_append_segment(
    tdm: *mut SidereonTdm,
    fields: *const SidereonTdmFieldInput,
    field_count: usize,
    metadata_comments: *const SidereonTdmCommentInput,
    metadata_comment_count: usize,
    data_comments: *const SidereonTdmCommentInput,
    data_comment_count: usize,
    records: *const SidereonTdmDataRecordInput,
    record_count: usize,
    policy: SidereonTdmWritePolicy,
    out_departures: *mut *mut SidereonTdmDepartures,
) -> SidereonStatus {
    engine_error_operation_boundary("sidereon_tdm_append_segment", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_append_segment",
            out_departures,
            "out_departures",
            tdm,
            "tdm"
        ));
        let departures_out = c_try!(require_out(
            out_departures,
            "sidereon_tdm_append_segment",
            "out_departures"
        ));
        *departures_out = ptr::null_mut();
        let raw_fields = c_try!(require_slice(
            fields,
            field_count,
            "sidereon_tdm_append_segment",
            "fields"
        ));
        let raw_meta_comments = c_try!(require_slice(
            metadata_comments,
            metadata_comment_count,
            "sidereon_tdm_append_segment",
            "metadata_comments"
        ));
        let raw_data_comments = c_try!(require_slice(
            data_comments,
            data_comment_count,
            "sidereon_tdm_append_segment",
            "data_comments"
        ));
        let raw_records = c_try!(require_slice(
            records,
            record_count,
            "sidereon_tdm_append_segment",
            "records"
        ));
        let fields = c_try!(tdm_fields_from_c("sidereon_tdm_append_segment", raw_fields));
        let metadata_comments = c_try!(tdm_comments_from_c(
            "sidereon_tdm_append_segment",
            raw_meta_comments
        ));
        let data_comments = c_try!(tdm_comments_from_c(
            "sidereon_tdm_append_segment",
            raw_data_comments
        ));
        let records = c_try!(tdm_records_from_c(
            "sidereon_tdm_append_segment",
            raw_records
        ));
        let (metadata, departures) =
            match sidereon_core::astro::tdm::TdmMetadata::from_raw_with_policy(
                fields,
                metadata_comments,
                c_try!(write_policy_to_core(
                    policy,
                    "sidereon_tdm_append_segment",
                    "validation"
                )),
            ) {
                Ok(value) => value,
                Err(err) => {
                    return map_tdm_error_class("sidereon_tdm_append_segment", "validation", err)
                }
            };
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_append_segment", "tdm"));
        tdm.inner
            .segments
            .push(sidereon_core::astro::tdm::TdmSegment {
                metadata,
                data: sidereon_core::astro::tdm::TdmDataSection {
                    comments: data_comments,
                    records,
                },
            });
        write_boxed_handle(departures_out, SidereonTdmDepartures { inner: departures });
        SidereonStatus::Ok
    })
}

/// Atomically replace one segment's raw metadata and return validation departures.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_replace_metadata(
    tdm: *mut SidereonTdm,
    segment_index: usize,
    fields: *const SidereonTdmFieldInput,
    field_count: usize,
    comments: *const SidereonTdmCommentInput,
    comment_count: usize,
    policy: SidereonTdmWritePolicy,
    out_departures: *mut *mut SidereonTdmDepartures,
) -> SidereonStatus {
    engine_error_operation_boundary(
        "sidereon_tdm_replace_metadata",
        SidereonStatus::Panic,
        || {
            c_try!(reject_output_overlaps_handle(
                "sidereon_tdm_replace_metadata",
                out_departures,
                "out_departures",
                tdm,
                "tdm"
            ));
            let departures_out = c_try!(require_out(
                out_departures,
                "sidereon_tdm_replace_metadata",
                "out_departures"
            ));
            *departures_out = ptr::null_mut();
            let raw_fields = c_try!(require_slice(
                fields,
                field_count,
                "sidereon_tdm_replace_metadata",
                "fields"
            ));
            let raw_comments = c_try!(require_slice(
                comments,
                comment_count,
                "sidereon_tdm_replace_metadata",
                "comments"
            ));
            let fields = c_try!(tdm_fields_from_c(
                "sidereon_tdm_replace_metadata",
                raw_fields
            ));
            let comments = c_try!(tdm_comments_from_c(
                "sidereon_tdm_replace_metadata",
                raw_comments
            ));
            let tdm = c_try!(require_mut(tdm, "sidereon_tdm_replace_metadata", "tdm"));
            let Some(segment) = tdm.inner.segments.get_mut(segment_index) else {
                return invalid_argument(
                    "sidereon_tdm_replace_metadata",
                    "segment_index is out of range",
                );
            };
            let departures = match segment.metadata.replace_raw_with_policy(
                fields,
                comments,
                c_try!(write_policy_to_core(
                    policy,
                    "sidereon_tdm_replace_metadata",
                    "validation"
                )),
            ) {
                Ok(value) => value,
                Err(err) => {
                    return map_tdm_error_class("sidereon_tdm_replace_metadata", "validation", err)
                }
            };
            write_boxed_handle(departures_out, SidereonTdmDepartures { inner: departures });
            SidereonStatus::Ok
        },
    )
}

/// Insert a data record in one segment.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_insert_record(
    tdm: *mut SidereonTdm,
    segment_index: usize,
    record_index: usize,
    record: SidereonTdmDataRecordInput,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_insert_record", SidereonStatus::Panic, || {
        let record = c_try!(tdm_record_from_c("sidereon_tdm_insert_record", &record));
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_insert_record", "tdm"));
        let Some(segment) = tdm.inner.segments.get_mut(segment_index) else {
            return invalid_argument(
                "sidereon_tdm_insert_record",
                "segment_index is out of range",
            );
        };
        if record_index > segment.data.records.len() {
            return invalid_argument("sidereon_tdm_insert_record", "record_index is out of range");
        }
        segment.data.records.insert(record_index, record);
        SidereonStatus::Ok
    })
}

/// Remove a data record from one segment.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_remove_record(
    tdm: *mut SidereonTdm,
    segment_index: usize,
    record_index: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_remove_record", SidereonStatus::Panic, || {
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_remove_record", "tdm"));
        let Some(segment) = tdm.inner.segments.get_mut(segment_index) else {
            return invalid_argument(
                "sidereon_tdm_remove_record",
                "segment_index is out of range",
            );
        };
        if record_index >= segment.data.records.len() {
            return invalid_argument("sidereon_tdm_remove_record", "record_index is out of range");
        }
        segment.data.records.remove(record_index);
        SidereonStatus::Ok
    })
}

/// Atomically replace a segment's data comments and records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_replace_data(
    tdm: *mut SidereonTdm,
    segment_index: usize,
    comments: *const SidereonTdmCommentInput,
    comment_count: usize,
    records: *const SidereonTdmDataRecordInput,
    record_count: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_replace_data", SidereonStatus::Panic, || {
        let comments = c_try!(require_slice(
            comments,
            comment_count,
            "sidereon_tdm_replace_data",
            "comments"
        ));
        let records = c_try!(require_slice(
            records,
            record_count,
            "sidereon_tdm_replace_data",
            "records"
        ));
        let comments = c_try!(tdm_comments_from_c("sidereon_tdm_replace_data", comments));
        let records = c_try!(tdm_records_from_c("sidereon_tdm_replace_data", records));
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_replace_data", "tdm"));
        let Some(segment) = tdm.inner.segments.get_mut(segment_index) else {
            return invalid_argument("sidereon_tdm_replace_data", "segment_index is out of range");
        };
        segment.data = sidereon_core::astro::tdm::TdmDataSection { comments, records };
        SidereonStatus::Ok
    })
}

/// Remove a complete segment.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_remove_segment(
    tdm: *mut SidereonTdm,
    segment_index: usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_remove_segment", SidereonStatus::Panic, || {
        let tdm = c_try!(require_mut(tdm, "sidereon_tdm_remove_segment", "tdm"));
        if segment_index >= tdm.inner.segments.len() {
            return invalid_argument(
                "sidereon_tdm_remove_segment",
                "segment_index is out of range",
            );
        }
        tdm.inner.segments.remove(segment_index);
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

/// Copy lossless text for a flattened record row. Parts: 0 keyword, 1 epoch,
/// 2 exact value token, 3 Other observable name, 4 canonical unit label.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_record_text(
    tdm: *const SidereonTdm,
    row_index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_record_text", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_record_text",
            out_written,
            "out_written",
            tdm,
            "tdm"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_record_text",
            out_required,
            "out_required",
            tdm,
            "tdm"
        ));
        c_try!(init_copy_counts(
            "sidereon_tdm_record_text",
            out_written,
            out_required
        ));
        c_try!(reject_outputs_overlapping_handle(
            "sidereon_tdm_record_text",
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            tdm,
            "tdm"
        ));
        let tdm = c_try!(require_ref(tdm, "sidereon_tdm_record_text", "tdm"));
        let record = tdm
            .inner
            .segments
            .iter()
            .flat_map(|s| &s.data.records)
            .nth(row_index);
        let Some(record) = record else {
            return invalid_argument("sidereon_tdm_record_text", "row_index is out of range");
        };
        let value = match part {
            0 => record.keyword.as_str(),
            1 => record.epoch.as_str(),
            2 => record.value.text.as_str(),
            3 => match &record.observable {
                sidereon_core::astro::tdm::TdmObservable::Other(name) => name.as_str(),
                _ => "",
            },
            4 => record.unit.as_str(),
            _ => return invalid_argument("sidereon_tdm_record_text", "part is invalid"),
        };
        copy_tdm_text(
            "sidereon_tdm_record_text",
            value,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare two complete TDM values using the engine's value equality.
///
/// Safety: a and b must be live handles and out_equal must point to writable
/// bool storage that does not overlap either handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_tdm_equal(
    a: *const SidereonTdm,
    b: *const SidereonTdm,
    out_equal: *mut bool,
) -> SidereonStatus {
    ffi_boundary("sidereon_tdm_equal", SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_equal",
            out_equal,
            "out_equal",
            a,
            "a"
        ));
        c_try!(reject_output_overlaps_handle(
            "sidereon_tdm_equal",
            out_equal,
            "out_equal",
            b,
            "b"
        ));
        let out = c_try!(require_out(out_equal, "sidereon_tdm_equal", "out_equal"));
        *out = false;
        let a = c_try!(require_ref(a, "sidereon_tdm_equal", "a"));
        let b = c_try!(require_ref(b, "sidereon_tdm_equal", "b"));
        *out = a.inner == b.inner;
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

fn tdm_comment_to_c(
    segment_index: usize,
    section: u32,
    comment: &sidereon_core::astro::tdm::TdmComment,
) -> SidereonTdmComment {
    SidereonTdmComment {
        segment_index,
        section,
        before_record: comment.before_record,
        text_len: comment.text.len(),
    }
}

fn tdm_field_to_c(
    segment_index: usize,
    section: u32,
    field: &sidereon_core::astro::tdm::TdmField,
) -> SidereonTdmField {
    SidereonTdmField {
        segment_index,
        section,
        key_len: field.key.len(),
        value_len: field.value.len(),
    }
}

unsafe fn tdm_fields_from_c(
    fn_name: &str,
    fields: &[SidereonTdmFieldInput],
) -> Result<Vec<sidereon_core::astro::tdm::TdmField>, SidereonStatus> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            Ok(sidereon_core::astro::tdm::TdmField {
                key: tdm_text_from_c(fn_name, &format!("fields[{index}].key"), field.key)?,
                value: tdm_text_from_c(fn_name, &format!("fields[{index}].value"), field.value)?,
            })
        })
        .collect()
}

unsafe fn tdm_comments_from_c(
    fn_name: &str,
    comments: &[SidereonTdmCommentInput],
) -> Result<Vec<sidereon_core::astro::tdm::TdmComment>, SidereonStatus> {
    comments
        .iter()
        .enumerate()
        .map(|(index, comment)| {
            Ok(sidereon_core::astro::tdm::TdmComment {
                text: tdm_text_from_c(fn_name, &format!("comments[{index}].text"), comment.text)?,
                before_record: comment.before_record,
            })
        })
        .collect()
}

unsafe fn tdm_records_from_c(
    fn_name: &str,
    records: &[SidereonTdmDataRecordInput],
) -> Result<Vec<sidereon_core::astro::tdm::TdmDataRecord>, SidereonStatus> {
    records
        .iter()
        .map(|record| tdm_record_from_c(fn_name, record))
        .collect()
}

unsafe fn tdm_record_from_c(
    fn_name: &str,
    record: &SidereonTdmDataRecordInput,
) -> Result<sidereon_core::astro::tdm::TdmDataRecord, SidereonStatus> {
    let participant = record
        .has_observable_participant
        .then_some(record.observable_participant);
    let observable = match record.observable {
        x if x == SidereonTdmObservable::Range as u32 => {
            sidereon_core::astro::tdm::TdmObservable::Range
        }
        x if x == SidereonTdmObservable::DopplerInstantaneous as u32 => {
            sidereon_core::astro::tdm::TdmObservable::DopplerInstantaneous
        }
        x if x == SidereonTdmObservable::DopplerIntegrated as u32 => {
            sidereon_core::astro::tdm::TdmObservable::DopplerIntegrated
        }
        x if x == SidereonTdmObservable::ReceiveFreq as u32 => {
            sidereon_core::astro::tdm::TdmObservable::ReceiveFreq { participant }
        }
        x if x == SidereonTdmObservable::TransmitFreq as u32 => {
            sidereon_core::astro::tdm::TdmObservable::TransmitFreq { participant }
        }
        x if x == SidereonTdmObservable::TransmitFreqRate as u32 => {
            sidereon_core::astro::tdm::TdmObservable::TransmitFreqRate { participant }
        }
        x if x == SidereonTdmObservable::Angle1 as u32 => {
            sidereon_core::astro::tdm::TdmObservable::Angle1
        }
        x if x == SidereonTdmObservable::Angle2 as u32 => {
            sidereon_core::astro::tdm::TdmObservable::Angle2
        }
        x if x == SidereonTdmObservable::Other as u32 => {
            sidereon_core::astro::tdm::TdmObservable::Other(tdm_text_from_c(
                fn_name,
                "record.observable_name",
                record.observable_name,
            )?)
        }
        _ => return Err(invalid_argument(fn_name, "record.observable is invalid")),
    };
    let unit_label = tdm_text_from_c(fn_name, "record.unit_label", record.unit_label)?;
    let unit = tdm_unit_from_c(record.unit, &unit_label)
        .ok_or_else(|| invalid_argument(fn_name, "record.unit is invalid"))?;
    Ok(sidereon_core::astro::tdm::TdmDataRecord {
        observable,
        keyword: tdm_text_from_c(fn_name, "record.keyword", record.keyword)?,
        epoch: tdm_text_from_c(fn_name, "record.epoch", record.epoch)?,
        value: sidereon_core::astro::tdm::TdmScalar {
            text: tdm_text_from_c(fn_name, "record.value_text", record.value_text)?,
            value: record.value,
        },
        unit,
    })
}

fn tdm_unit_from_c(value: u32, label: &str) -> Option<sidereon_core::astro::tdm::TdmUnit> {
    use sidereon_core::astro::tdm::TdmUnit as U;
    Some(match value {
        x if x == SidereonTdmUnit::Kilometers as u32 => U::Kilometers,
        x if x == SidereonTdmUnit::Seconds as u32 => U::Seconds,
        x if x == SidereonTdmUnit::RangeUnits as u32 => U::RangeUnits,
        x if x == SidereonTdmUnit::KilometersPerSecond as u32 => U::KilometersPerSecond,
        x if x == SidereonTdmUnit::Hertz as u32 => U::Hertz,
        x if x == SidereonTdmUnit::HertzPerSecond as u32 => U::HertzPerSecond,
        x if x == SidereonTdmUnit::Degrees as u32 => U::Degrees,
        x if x == SidereonTdmUnit::DecibelWatts as u32 => U::DecibelWatts,
        x if x == SidereonTdmUnit::DecibelHertz as u32 => U::DecibelHertz,
        x if x == SidereonTdmUnit::SquareMeters as u32 => U::SquareMeters,
        x if x == SidereonTdmUnit::Meters as u32 => U::Meters,
        x if x == SidereonTdmUnit::SecondsPerSecond as u32 => U::SecondsPerSecond,
        x if x == SidereonTdmUnit::Percent as u32 => U::Percent,
        x if x == SidereonTdmUnit::Kelvin as u32 => U::Kelvin,
        x if x == SidereonTdmUnit::Hectopascals as u32 => U::Hectopascals,
        x if x == SidereonTdmUnit::TotalElectronContentUnits as u32 => U::TotalElectronContentUnits,
        x if x == SidereonTdmUnit::Dimensionless as u32 => U::Dimensionless,
        x if x == SidereonTdmUnit::Unknown as u32 => U::Unknown(label.to_owned()),
        _ => return None,
    })
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

fn tdm_diagnostic_node(
    kind: &str,
    fields: serde_json::Value,
    message: String,
) -> serde_json::Value {
    json!({ "kind": kind, "fields": fields, "message": message })
}

fn tdm_warning_value(warning: &TdmWarning) -> serde_json::Value {
    match warning {
        TdmWarning::NonPrintableCharacter {
            line,
            keyword,
            column,
            character,
        } => tdm_diagnostic_node(
            "non_printable_character",
            json!({"line": line, "keyword": keyword, "column": column, "character": character.to_string()}),
            warning.to_string(),
        ),
        TdmWarning::LineTooLong {
            line,
            keyword,
            length,
        } => tdm_diagnostic_node(
            "line_too_long",
            json!({"line": line, "keyword": keyword, "length": length}),
            warning.to_string(),
        ),
        TdmWarning::RepeatedKeyword {
            line,
            keyword,
            section,
        } => tdm_diagnostic_node(
            "repeated_keyword",
            json!({"line": line, "keyword": keyword, "section": section}),
            warning.to_string(),
        ),
        TdmWarning::MissingKeyword { keyword, segment } => tdm_diagnostic_node(
            "missing_keyword",
            json!({"keyword": keyword, "segment": segment}),
            warning.to_string(),
        ),
        TdmWarning::EmptyDataSection { segment } => tdm_diagnostic_node(
            "empty_data_section",
            json!({"segment": segment}),
            warning.to_string(),
        ),
        TdmWarning::RecordsOutOfOrder {
            segment,
            keyword,
            epoch,
        } => tdm_diagnostic_node(
            "records_out_of_order",
            json!({"segment": segment, "keyword": keyword, "epoch": epoch}),
            warning.to_string(),
        ),
        TdmWarning::UnterminatedFinalLine { line } => tdm_diagnostic_node(
            "unterminated_final_line",
            json!({"line": line}),
            warning.to_string(),
        ),
        TdmWarning::KeywordOutOfOrder {
            line,
            keyword,
            section,
        } => tdm_diagnostic_node(
            "keyword_out_of_order",
            json!({"line": line, "keyword": keyword, "section": section}),
            warning.to_string(),
        ),
        TdmWarning::DuplicateRecord {
            segment,
            keyword,
            epoch,
        } => tdm_diagnostic_node(
            "duplicate_record",
            json!({"segment": segment, "keyword": keyword, "epoch": epoch}),
            warning.to_string(),
        ),
        _ => tdm_diagnostic_node("unknown", json!({}), warning.to_string()),
    }
}

fn tdm_departure_value(departure: &TdmDeparture) -> serde_json::Value {
    match departure {
        TdmDeparture::NonPrintableCharacter { keyword, character } => tdm_diagnostic_node(
            "non_printable_character",
            json!({"keyword": keyword, "character": character.to_string()}),
            departure.to_string(),
        ),
        TdmDeparture::LineTooLong { keyword, length } => tdm_diagnostic_node(
            "line_too_long",
            json!({"keyword": keyword, "length": length}),
            departure.to_string(),
        ),
        TdmDeparture::MissingKeyword { keyword, segment } => tdm_diagnostic_node(
            "missing_keyword",
            json!({"keyword": keyword, "segment": segment}),
            departure.to_string(),
        ),
        TdmDeparture::EmptyDataSection { segment } => tdm_diagnostic_node(
            "empty_data_section",
            json!({"segment": segment}),
            departure.to_string(),
        ),
        TdmDeparture::RecordsOutOfOrder {
            segment,
            keyword,
            epoch,
        } => tdm_diagnostic_node(
            "records_out_of_order",
            json!({"segment": segment, "keyword": keyword, "epoch": epoch}),
            departure.to_string(),
        ),
        TdmDeparture::DuplicateRecord {
            segment,
            keyword,
            epoch,
        } => tdm_diagnostic_node(
            "duplicate_record",
            json!({"segment": segment, "keyword": keyword, "epoch": epoch}),
            departure.to_string(),
        ),
        TdmDeparture::RepeatedKeyword { keyword, section } => tdm_diagnostic_node(
            "repeated_keyword",
            json!({"keyword": keyword, "section": section}),
            departure.to_string(),
        ),
        TdmDeparture::KeywordOutOfOrder { keyword, section } => tdm_diagnostic_node(
            "keyword_out_of_order",
            json!({"keyword": keyword, "section": section}),
            departure.to_string(),
        ),
        TdmDeparture::UnterminatedFinalLine => {
            tdm_diagnostic_node("unterminated_final_line", json!({}), departure.to_string())
        }
        _ => tdm_diagnostic_node("unknown", json!({}), departure.to_string()),
    }
}

fn invalid_argument(fn_name: &str, detail: &str) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Tdm,
        fn_name,
        json!({
            "kind": "invalid_argument",
            "fields": {"detail": detail},
            "operation_class": "validation",
        }),
    );
    set_last_error(format!("{fn_name}: {detail}"));
    SidereonStatus::InvalidArgument
}

fn map_tdm_policy_error(
    fn_name: &str,
    operation_class: &str,
    field: &str,
    value: u32,
) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Tdm,
        fn_name,
        json!({
            "kind": "invalid_policy",
            "fields": {"field": field, "value": value},
            "operation_class": operation_class,
        }),
    );
    set_last_error(format!(
        "{fn_name}: policy.{field} has invalid value {value}"
    ));
    SidereonStatus::InvalidArgument
}

unsafe fn tdm_text_from_c(
    fn_name: &str,
    arg_name: &str,
    value: SidereonTdmText,
) -> Result<String, SidereonStatus> {
    let bytes = require_slice(value.data, value.len, fn_name, arg_name)?;
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_owned()),
        Err(_) => {
            record_engine_error(
                SidereonEngineErrorFamily::Tdm,
                fn_name,
                json!({
                    "kind": "invalid_utf8",
                    "fields": {"field": arg_name},
                    "operation_class": "validation",
                }),
            );
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn tdm_optional_text_from_c(
    fn_name: &str,
    arg_name: &str,
    value: SidereonTdmOptionalText,
) -> Result<Option<String>, SidereonStatus> {
    if value.has_value {
        tdm_text_from_c(fn_name, arg_name, value.value).map(Some)
    } else {
        Ok(None)
    }
}

unsafe fn copy_tdm_text(
    fn_name: &str,
    value: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        value.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
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

fn map_tdm_error_class(fn_name: &str, operation_class: &str, err: TdmError) -> SidereonStatus {
    let mut value = tdm_error_value(&err);
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "operation_class".to_owned(),
            serde_json::Value::String(operation_class.to_owned()),
        );
    }
    record_engine_error(SidereonEngineErrorFamily::Tdm, fn_name, value);
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

    #[test]
    fn policy_parse_reports_final_terminator_and_record_mutation_round_trips() {
        let text = TDM_KVN_FIXTURE.trim_end_matches(['\r', '\n']);
        unsafe {
            let mut tdm = ptr::null_mut();
            let mut warnings = ptr::null_mut();
            assert_eq!(
                sidereon_tdm_parse_kvn_with_policy(
                    text.as_ptr(),
                    text.len(),
                    sidereon_tdm_policy_lenient(),
                    &mut tdm,
                    &mut warnings,
                ),
                SidereonStatus::Ok
            );
            let mut count = 0;
            assert_eq!(
                sidereon_tdm_warnings_count(warnings, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            assert_eq!(
                tdm_warning_value(&(&(*warnings).inner)[0])["kind"],
                "unterminated_final_line"
            );
            let mut header: SidereonTdmHeader = std::mem::zeroed();
            assert_eq!(sidereon_tdm_header(tdm, &mut header), SidereonStatus::Ok);
            assert_eq!(header.segment_count, 2);
            assert_eq!(header.version_len, 3);
            assert_eq!(sidereon_tdm_remove_record(tdm, 0, 0), SidereonStatus::Ok);
            sidereon_tdm_warnings_free(warnings);
            sidereon_tdm_free(tdm);
        }
    }

    #[test]
    fn policy_presets_are_field_exact_core_conversions() {
        assert_eq!(
            policy_to_core(sidereon_tdm_policy_strict(), "test").unwrap(),
            TdmPolicy::strict()
        );
        assert_eq!(
            policy_to_core(sidereon_tdm_policy_default(), "test").unwrap(),
            TdmPolicy::default()
        );
        assert_eq!(
            policy_to_core(sidereon_tdm_policy_lenient(), "test").unwrap(),
            TdmPolicy::lenient()
        );
        assert_eq!(
            write_policy_to_core(sidereon_tdm_write_policy_strict(), "test", "validation").unwrap(),
            TdmWritePolicy::strict()
        );
        assert_eq!(
            write_policy_to_core(sidereon_tdm_write_policy_default(), "test", "validation")
                .unwrap(),
            TdmWritePolicy::default()
        );
        assert_eq!(
            write_policy_to_core(sidereon_tdm_write_policy_lenient(), "test", "validation")
                .unwrap(),
            TdmWritePolicy::lenient()
        );
    }

    #[test]
    fn complete_tdm_value_equality_handles_edits_and_nulls() {
        unsafe {
            let mut a = ptr::null_mut();
            let mut b = ptr::null_mut();
            assert_eq!(
                sidereon_tdm_parse_kvn(TDM_KVN_FIXTURE.as_ptr(), TDM_KVN_FIXTURE.len(), &mut a),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_tdm_parse_kvn(TDM_KVN_FIXTURE.as_ptr(), TDM_KVN_FIXTURE.len(), &mut b),
                SidereonStatus::Ok
            );

            let mut equal = false;
            assert_eq!(sidereon_tdm_equal(a, b, &mut equal), SidereonStatus::Ok);
            assert!(equal);

            assert_eq!(sidereon_tdm_remove_record(b, 0, 0), SidereonStatus::Ok);
            assert_eq!(sidereon_tdm_equal(a, b, &mut equal), SidereonStatus::Ok);
            assert!(!equal);

            equal = true;
            assert_eq!(
                sidereon_tdm_equal(ptr::null(), b, &mut equal),
                SidereonStatus::NullPointer
            );
            assert!(!equal);

            assert_eq!(
                sidereon_tdm_equal(a, b, a.cast()),
                SidereonStatus::InvalidArgument
            );

            sidereon_tdm_free(a);
            sidereon_tdm_free(b);
        }
    }
}
