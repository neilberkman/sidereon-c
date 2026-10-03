use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, nmea_error_value, nmea_skip_value, nmea_warning_value,
    record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};

// === Round-2 NMEA parse, accumulation, and GGA writing =======================

pub const NMEA_TALKER_C_BYTES: usize = 3;

pub struct SidereonNmeaLog {
    pub(crate) epochs: Vec<sidereon_core::nmea::EpochSnapshot>,
    pub(crate) sentences: Vec<sidereon_core::nmea::NmeaSentence>,
    pub(crate) diagnostics: sidereon_core::nmea::Diagnostics,
    pub(crate) sentence_count: usize,
    pub(crate) skip_count: usize,
    pub(crate) warning_count: usize,
}

/// Owned diagnostics copied from an NMEA log, accumulator, or epoch.
pub struct SidereonNmeaDiagnosticList {
    entries: Vec<OwnedNmeaDiagnostic>,
}

struct OwnedNmeaDiagnostic {
    info: SidereonNmeaDiagnosticInfo,
    payload: String,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNmeaDiagnosticSource {
    Parser = 0,
    EpochAssembly = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNmeaDiagnosticKind {
    Skip = 0,
    Warning = 1,
}

/// Metadata for one owned NMEA diagnostic JSON payload.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonNmeaDiagnosticInfo {
    pub source: SidereonNmeaDiagnosticSource,
    pub kind: SidereonNmeaDiagnosticKind,
    pub has_epoch_index: bool,
    pub epoch_index: usize,
    pub payload_len: usize,
}

pub struct SidereonNmeaAccumulator {
    pub(crate) inner: sidereon_core::nmea::NmeaAccumulator,
    pub(crate) epochs: Vec<sidereon_core::nmea::EpochSnapshot>,
    pub(crate) sentences: Vec<sidereon_core::nmea::NmeaSentence>,
    pub(crate) sentence_count: usize,
    pub(crate) skip_count: usize,
    pub(crate) warning_count: usize,
    pub(crate) diagnostics: sidereon_core::nmea::Diagnostics,
}

/// Independently owned, ordered NMEA sentence or epoch field records.
pub struct SidereonNmeaRecordList {
    records: Vec<String>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaSummary {
    pub sentence_count: usize,
    pub epoch_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaChunkSummary {
    pub sentence_count: usize,
    pub completed_epoch_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
    pub retained_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaEpochSummary {
    pub has_calendar_epoch: bool,
    pub calendar_epoch: SidereonCalendarEpoch,
    pub has_position: bool,
    pub position: SidereonGeodetic,
    pub has_instant_j2000_s: bool,
    pub instant_j2000_s: f64,
    pub has_pdop: bool,
    pub pdop: f64,
    pub has_hdop: bool,
    pub hdop: f64,
    pub has_vdop: bool,
    pub vdop: f64,
    pub used_satellite_count: usize,
    pub satellites_in_view: usize,
    pub sentence_count: usize,
    pub skip_count: usize,
    pub warning_count: usize,
    pub has_gga: bool,
    pub has_rmc: bool,
    pub has_gll: bool,
    pub gsa_count: usize,
    pub gsv_group_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNmeaGgaOptions {
    pub talker: [c_char; NMEA_TALKER_C_BYTES],
    pub utc_seconds_of_day: f64,
    pub position: SidereonGeodetic,
    pub quality: u32,
    pub satellites_used: u8,
    pub hdop: f64,
    pub coordinate_decimals: u8,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_parse(
    data: *const u8,
    len: usize,
    out_log: *mut *mut SidereonNmeaLog,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_parse", SidereonStatus::Panic, || {
        let out_log = c_try!(require_out(out_log, "sidereon_nmea_parse", "out_log"));
        *out_log = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_nmea_parse", "data"));
        let log = nmea_log_from_parsed(sidereon_core::nmea::parse_nmea(bytes));
        write_boxed_handle(out_log, log);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_summary(
    log: *const SidereonNmeaLog,
    out_summary: *mut SidereonNmeaSummary,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_log_summary", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_summary,
            "sidereon_nmea_log_summary",
            "out_summary"
        ));
        let log = c_try!(require_ref(log, "sidereon_nmea_log_summary", "log"));
        *out = nmea_summary(
            log.sentence_count,
            &log.epochs,
            log.skip_count,
            log.warning_count,
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_epochs(
    log: *const SidereonNmeaLog,
    out: *mut SidereonNmeaEpochSummary,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_nmea_log_epochs", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_nmea_log_epochs",
            out_written,
            out_required
        ));
        let log = c_try!(require_ref(log, "sidereon_nmea_log_epochs", "log"));
        let values: Vec<_> = log.epochs.iter().map(nmea_epoch_summary_to_c).collect();
        c_try!(copy_prefix_to_c(
            "sidereon_nmea_log_epochs",
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

/// Copy the log's accepted sentences into an owned, input-ordered field list.
/// Each payload is a JSON record with the talker and every decoded field from
/// the accepted typed sentence. Optional values are JSON `null`; enum values
/// include a stable `kind` and their raw or numeric value where applicable.
///
/// # Safety
/// `log` must be a live handle and `out` must point to one writable, aligned
/// result-list pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_sentences(
    log: *const SidereonNmeaLog,
    out: *mut *mut SidereonNmeaRecordList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_log_sentences";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let log = c_try!(require_ref(log, FN, "log"));
        write_boxed_handle(out, nmea_sentence_record_list(&log.sentences));
        SidereonStatus::Ok
    })
}

/// Copy the accumulator's accepted complete-line sentences into an owned,
/// input-ordered field list.
///
/// # Safety
/// `accumulator` must be live and `out` must point to one writable, aligned
/// result-list pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_sentences(
    accumulator: *const SidereonNmeaAccumulator,
    out: *mut *mut SidereonNmeaRecordList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_accumulator_sentences";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let accumulator = c_try!(require_ref(accumulator, FN, "accumulator"));
        write_boxed_handle(out, nmea_sentence_record_list(&accumulator.sentences));
        SidereonStatus::Ok
    })
}

/// Copy all complete singleton, GSA and GSV fields for each log epoch into an
/// owned, epoch-ordered field list. Optional singleton bodies are represented
/// as JSON null when absent; GSA entries and GSV groups retain core ordering.
///
/// # Safety
/// `log` must be live and `out` must point to one writable, aligned
/// result-list pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_epoch_records(
    log: *const SidereonNmeaLog,
    out: *mut *mut SidereonNmeaRecordList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_log_epoch_records";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let log = c_try!(require_ref(log, FN, "log"));
        write_boxed_handle(out, nmea_epoch_record_list(&log.epochs));
        SidereonStatus::Ok
    })
}

/// Copy all complete singleton, GSA and GSV fields for each accumulated epoch
/// into an owned, epoch-ordered field list.
///
/// # Safety
/// `accumulator` must be live and `out` must point to one writable, aligned
/// result-list pointer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_epoch_records(
    accumulator: *const SidereonNmeaAccumulator,
    out: *mut *mut SidereonNmeaRecordList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_accumulator_epoch_records";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let accumulator = c_try!(require_ref(accumulator, FN, "accumulator"));
        write_boxed_handle(out, nmea_epoch_record_list(&accumulator.epochs));
        SidereonStatus::Ok
    })
}

/// Free an owned NMEA sentence/epoch record list. NULL is accepted.
///
/// # Safety
/// A non-NULL pointer must be a live list returned by this binding and passed
/// here exactly once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_record_list_free(list: *mut SidereonNmeaRecordList) {
    free_boxed(list);
}

/// Write the number of owned NMEA records to `out_count`.
///
/// # Safety
/// `list` must be live and `out_count` must point to one writable, aligned
/// `usize` that does not alias the list.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_record_list_count(
    list: *const SidereonNmeaRecordList,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_record_list_count";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN, "out_count"));
        *out_count = 0;
        let list = c_try!(require_ref(list, FN, "list"));
        *out_count = list.records.len();
        SidereonStatus::Ok
    })
}

/// Copy the JSON field payload at `index` using the standard variable-length
/// byte output contract. A too-small buffer is left untouched and the required
/// size is returned.
///
/// # Safety
/// `list` must be live. Buffer and count pointers must meet the non-aliasing
/// writable-storage contract of `copy_prefix_to_c`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_record_list_get_payload(
    list: *const SidereonNmeaRecordList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_record_list_get_payload";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let list = c_try!(require_ref(list, FN, "list"));
        let Some(record) = list.records.get(index) else {
            set_last_error(format!(
                "{FN}: index {index} is out of range ({})",
                list.records.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            record.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy all parser and epoch-assembly diagnostics into an independently owned
/// list. Parser-scope entries come first; epochs follow in log order. Within
/// each scope, skips precede warnings and each core diagnostic vector keeps its
/// original order.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_diagnostics(
    log: *const SidereonNmeaLog,
    out: *mut *mut SidereonNmeaDiagnosticList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_log_diagnostics";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let log = c_try!(require_ref(log, FN, "log"));
        let mut entries = Vec::new();
        append_nmea_diagnostics(
            &mut entries,
            &log.diagnostics,
            SidereonNmeaDiagnosticSource::Parser,
            None,
        );
        for (index, epoch) in log.epochs.iter().enumerate() {
            append_nmea_diagnostics(
                &mut entries,
                &epoch.diagnostics,
                SidereonNmeaDiagnosticSource::EpochAssembly,
                Some(index),
            );
        }
        write_boxed_handle(out, SidereonNmeaDiagnosticList { entries });
        SidereonStatus::Ok
    })
}

/// Copy parser diagnostics and every retained epoch's assembly diagnostics into
/// an independently owned list.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_diagnostics(
    accumulator: *const SidereonNmeaAccumulator,
    out: *mut *mut SidereonNmeaDiagnosticList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_accumulator_diagnostics";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let accumulator = c_try!(require_ref(accumulator, FN, "accumulator"));
        let mut entries = Vec::new();
        append_nmea_diagnostics(
            &mut entries,
            &accumulator.diagnostics,
            SidereonNmeaDiagnosticSource::Parser,
            None,
        );
        for (index, epoch) in accumulator.epochs.iter().enumerate() {
            append_nmea_diagnostics(
                &mut entries,
                &epoch.diagnostics,
                SidereonNmeaDiagnosticSource::EpochAssembly,
                Some(index),
            );
        }
        write_boxed_handle(out, SidereonNmeaDiagnosticList { entries });
        SidereonStatus::Ok
    })
}

/// Copy one epoch snapshot's full assembly diagnostics into an owned list.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_epoch_diagnostics(
    log: *const SidereonNmeaLog,
    epoch_index: usize,
    out: *mut *mut SidereonNmeaDiagnosticList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_log_epoch_diagnostics";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let log = c_try!(require_ref(log, FN, "log"));
        let epoch = match log.epochs.get(epoch_index) {
            Some(epoch) => epoch,
            None => {
                set_last_error(format!("{FN}: epoch index {epoch_index} is out of range"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let mut entries = Vec::new();
        append_nmea_diagnostics(
            &mut entries,
            &epoch.diagnostics,
            SidereonNmeaDiagnosticSource::EpochAssembly,
            Some(epoch_index),
        );
        write_boxed_handle(out, SidereonNmeaDiagnosticList { entries });
        SidereonStatus::Ok
    })
}

/// Copy one accumulated epoch's full assembly diagnostics into an owned list.
#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_epoch_diagnostics(
    accumulator: *const SidereonNmeaAccumulator,
    epoch_index: usize,
    out: *mut *mut SidereonNmeaDiagnosticList,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_accumulator_epoch_diagnostics";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN, "out"));
        *out = ptr::null_mut();
        let accumulator = c_try!(require_ref(accumulator, FN, "accumulator"));
        let epoch = match accumulator.epochs.get(epoch_index) {
            Some(epoch) => epoch,
            None => {
                set_last_error(format!("{FN}: epoch index {epoch_index} is out of range"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let mut entries = Vec::new();
        append_nmea_diagnostics(
            &mut entries,
            &epoch.diagnostics,
            SidereonNmeaDiagnosticSource::EpochAssembly,
            Some(epoch_index),
        );
        write_boxed_handle(out, SidereonNmeaDiagnosticList { entries });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_diagnostic_list_count(
    list: *const SidereonNmeaDiagnosticList,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_diagnostic_list_count";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN, "out_count"));
        let list = c_try!(require_ref(list, FN, "list"));
        *out = list.entries.len();
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_diagnostic_list_get_info(
    list: *const SidereonNmeaDiagnosticList,
    index: usize,
    out_info: *mut SidereonNmeaDiagnosticInfo,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_diagnostic_list_get_info";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        let list = c_try!(require_ref(list, FN, "list"));
        let out = c_try!(require_out(out_info, FN, "out_info"));
        let Some(entry) = list.entries.get(index) else {
            set_last_error(format!("{FN}: index {index} is out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *out = entry.info;
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_diagnostic_list_get_payload(
    list: *const SidereonNmeaDiagnosticList,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_diagnostic_list_get_payload";
    ffi_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let list = c_try!(require_ref(list, FN, "list"));
        let Some(entry) = list.entries.get(index) else {
            set_last_error(format!("{FN}: index {index} is out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            entry.payload.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_diagnostic_list_free(list: *mut SidereonNmeaDiagnosticList) {
    free_boxed(list);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_log_free(log: *mut SidereonNmeaLog) {
    free_boxed(log);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_new(
    out_accumulator: *mut *mut SidereonNmeaAccumulator,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_new",
        SidereonStatus::Panic,
        || {
            let out_accumulator = c_try!(require_out(
                out_accumulator,
                "sidereon_nmea_accumulator_new",
                "out_accumulator"
            ));
            *out_accumulator = ptr::null_mut();
            write_boxed_handle(
                out_accumulator,
                SidereonNmeaAccumulator {
                    inner: sidereon_core::nmea::NmeaAccumulator::new(),
                    epochs: Vec::new(),
                    sentences: Vec::new(),
                    sentence_count: 0,
                    skip_count: 0,
                    warning_count: 0,
                    diagnostics: sidereon_core::nmea::Diagnostics::new(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_push(
    accumulator: *mut SidereonNmeaAccumulator,
    data: *const u8,
    len: usize,
    out_summary: *mut SidereonNmeaChunkSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_push",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_push",
                "out_summary"
            ));
            *out = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            let accumulator = c_try!(require_mut(
                accumulator,
                "sidereon_nmea_accumulator_push",
                "accumulator"
            ));
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_nmea_accumulator_push",
                "data"
            ));
            let output = accumulator.inner.push_bytes(bytes);
            let sentence_count = output.sentences.len();
            accumulator
                .sentences
                .extend(output.sentences.iter().cloned());
            let mut skip_count = output.diagnostics.skips.len();
            let mut warning_count = output.diagnostics.warnings.len();
            accumulator
                .diagnostics
                .skips
                .extend(output.diagnostics.skips.iter().cloned());
            accumulator
                .diagnostics
                .warnings
                .extend(output.diagnostics.warnings.iter().cloned());
            let completed_epoch_count = output.snapshots.len();
            for epoch in &output.snapshots {
                let (epoch_skips, epoch_warnings) = nmea_epoch_diagnostic_counts(epoch);
                skip_count += epoch_skips;
                warning_count += epoch_warnings;
            }
            accumulator.sentence_count += sentence_count;
            accumulator.skip_count += skip_count;
            accumulator.warning_count += warning_count;
            accumulator.epochs.extend(output.snapshots);
            *out = SidereonNmeaChunkSummary {
                sentence_count,
                completed_epoch_count,
                skip_count,
                warning_count,
                retained_len: accumulator.inner.retained_len(),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_finish(
    accumulator: *mut SidereonNmeaAccumulator,
    out_summary: *mut SidereonNmeaChunkSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_finish",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_finish",
                "out_summary"
            ));
            *out = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            let accumulator = c_try!(require_mut(
                accumulator,
                "sidereon_nmea_accumulator_finish",
                "accumulator"
            ));
            let output = accumulator.inner.finish_with_output();
            let sentence_count = output.sentences.len();
            accumulator
                .sentences
                .extend(output.sentences.iter().cloned());
            let mut skip_count = output.diagnostics.skips.len();
            let mut warning_count = output.diagnostics.warnings.len();
            accumulator
                .diagnostics
                .skips
                .extend(output.diagnostics.skips.iter().cloned());
            accumulator
                .diagnostics
                .warnings
                .extend(output.diagnostics.warnings.iter().cloned());
            for epoch in &output.snapshots {
                let (epoch_skips, epoch_warnings) = nmea_epoch_diagnostic_counts(epoch);
                skip_count += epoch_skips;
                warning_count += epoch_warnings;
            }
            let completed_epoch_count = output.snapshots.len();
            accumulator.sentence_count += sentence_count;
            accumulator.skip_count += skip_count;
            accumulator.warning_count += warning_count;
            accumulator.epochs.extend(output.snapshots);
            *out = SidereonNmeaChunkSummary {
                sentence_count,
                completed_epoch_count,
                skip_count,
                warning_count,
                retained_len: accumulator.inner.retained_len(),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_summary(
    accumulator: *const SidereonNmeaAccumulator,
    out_summary: *mut SidereonNmeaSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_nmea_accumulator_summary",
                "out_summary"
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_summary",
                "accumulator"
            ));
            *out = nmea_summary(
                accumulator.sentence_count,
                &accumulator.epochs,
                accumulator.skip_count,
                accumulator.warning_count,
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_epochs(
    accumulator: *const SidereonNmeaAccumulator,
    out: *mut SidereonNmeaEpochSummary,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_epochs",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_nmea_accumulator_epochs",
                out_written,
                out_required
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_epochs",
                "accumulator"
            ));
            let values: Vec<_> = accumulator
                .epochs
                .iter()
                .map(nmea_epoch_summary_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_nmea_accumulator_epochs",
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
pub unsafe extern "C" fn sidereon_nmea_accumulator_retained_len(
    accumulator: *const SidereonNmeaAccumulator,
    out_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_nmea_accumulator_retained_len",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_len,
                "sidereon_nmea_accumulator_retained_len",
                "out_len"
            ));
            let accumulator = c_try!(require_ref(
                accumulator,
                "sidereon_nmea_accumulator_retained_len",
                "accumulator"
            ));
            *out = accumulator.inner.retained_len();
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_accumulator_free(accumulator: *mut SidereonNmeaAccumulator) {
    free_boxed(accumulator);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_nmea_write_gga(
    options: *const SidereonNmeaGgaOptions,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN: &str = "sidereon_nmea_write_gga";
    engine_error_operation_boundary(FN, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN, out_written, out_required));
        let options = c_try!(require_ref(options, FN, "options"));
        let talker = c_try!(nmea_talker_from_c(FN, &options.talker));
        let time = match sidereon_core::nmea::NmeaTime::from_seconds_of_day_floor_centis(
            options.utc_seconds_of_day,
        ) {
            Ok(time) => time,
            Err(err) => {
                return nmea_write_error(FN, &err);
            }
        };
        let position = c_try!(geodetic_to_wgs84(FN, "position", options.position));
        let quality = c_try!(nmea_gga_quality_from_c(FN, options.quality));
        let gga = match sidereon_core::nmea::Gga::vrs_position(
            position,
            time,
            quality,
            options.satellites_used,
            options.hdop,
            options.coordinate_decimals,
        ) {
            Ok(gga) => gga,
            Err(err) => {
                return nmea_write_error(FN, &err);
            }
        };
        let sentence = match sidereon_core::nmea::write_gga(talker, &gga) {
            Ok(sentence) => sentence,
            Err(err) => {
                return nmea_write_error(FN, &err);
            }
        };
        c_try!(copy_prefix_to_c(
            FN,
            "out",
            sentence.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn nmea_write_error(fn_name: &str, error: &sidereon_core::nmea::NmeaError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::Nmea,
        fn_name,
        nmea_error_value(error),
    );
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

fn nmea_summary(
    sentence_count: usize,
    epochs: &[sidereon_core::nmea::EpochSnapshot],
    skip_count: usize,
    warning_count: usize,
) -> SidereonNmeaSummary {
    SidereonNmeaSummary {
        sentence_count,
        epoch_count: epochs.len(),
        skip_count,
        warning_count,
    }
}

fn nmea_sentence_record_list(
    sentences: &[sidereon_core::nmea::NmeaSentence],
) -> SidereonNmeaRecordList {
    SidereonNmeaRecordList {
        records: sentences
            .iter()
            .enumerate()
            .map(|(index, sentence)| nmea_sentence_record(index, sentence).to_string())
            .collect(),
    }
}

fn nmea_epoch_record_list(epochs: &[sidereon_core::nmea::EpochSnapshot]) -> SidereonNmeaRecordList {
    SidereonNmeaRecordList {
        records: epochs
            .iter()
            .enumerate()
            .map(|(index, epoch)| nmea_epoch_record(index, epoch).to_string())
            .collect(),
    }
}

fn nmea_sentence_record(index: usize, sentence: &sidereon_core::nmea::NmeaSentence) -> Value {
    json!({
        "kind": "sentence",
        "index": index,
        "talker": nmea_talker_value(sentence.talker),
        "body": nmea_body_value(&sentence.body),
    })
}

fn nmea_epoch_record(index: usize, epoch: &sidereon_core::nmea::EpochSnapshot) -> Value {
    json!({
        "kind": "epoch",
        "index": index,
        "time": epoch.time_of_day.map(nmea_time_value),
        "date": epoch.date.map(nmea_date_value),
        "gga": epoch.gga.as_ref().map(nmea_gga_value),
        "rmc": epoch.rmc.as_ref().map(nmea_rmc_value),
        "gll": epoch.gll.as_ref().map(nmea_gll_value),
        "gst": epoch.gst.as_ref().map(nmea_gst_value),
        "vtg": epoch.vtg.as_ref().map(nmea_vtg_value),
        "zda": epoch.zda.as_ref().map(nmea_zda_value),
        "gsa": epoch.gsa.iter().map(|entry| json!({
            "system": entry.system.map(nmea_system_value),
            "body": nmea_gsa_value(&entry.gsa),
        })).collect::<Vec<_>>(),
        "gsv": epoch.gsv.iter().map(|group| json!({
            "talker": nmea_talker_value(group.talker),
            "signal": group.signal.map(nmea_signal_value),
            "claimed_in_view": group.claimed_in_view,
            "complete": group.complete,
            "satellites": group.satellites.iter().map(nmea_gsv_satellite_value).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "sentence_count": epoch.sentence_count,
        "diagnostic_counts": {
            "skips": epoch.diagnostics.skips.len(),
            "warnings": epoch.diagnostics.warnings.len(),
        },
    })
}

fn nmea_system_value(system: sidereon_core::GnssSystem) -> Value {
    json!(format!("{system:?}"))
}

fn nmea_talker_value(talker: sidereon_core::nmea::NmeaTalker) -> Value {
    use sidereon_core::nmea::NmeaTalker as T;
    match talker {
        T::System(system) => json!({
            "kind": "system",
            "system": nmea_system_value(system),
            "code": match system {
                sidereon_core::GnssSystem::Gps | sidereon_core::GnssSystem::Sbas => "GP",
                sidereon_core::GnssSystem::Glonass => "GL",
                sidereon_core::GnssSystem::Galileo => "GA",
                sidereon_core::GnssSystem::BeiDou => "GB",
                sidereon_core::GnssSystem::Qzss => "GQ",
                sidereon_core::GnssSystem::Navic => "GI",
            },
        }),
        T::Combined => json!({"kind": "combined", "code": "GN"}),
        T::Other(bytes) => json!({
            "kind": "other",
            "bytes": bytes,
            "code": String::from_utf8_lossy(&bytes),
        }),
    }
}

fn nmea_time_value(time: sidereon_core::nmea::NmeaTime) -> Value {
    json!({
        "hour": time.hour,
        "minute": time.minute,
        "second": time.second,
        "nanos": time.nanos,
        "decimals": time.decimals,
    })
}

fn nmea_date_value(date: sidereon_core::nmea::NmeaDate) -> Value {
    json!({"year": date.year, "month": date.month, "day": date.day})
}

fn nmea_coordinate_value(coordinate: sidereon_core::nmea::NmeaCoordinate) -> Value {
    json!({
        "degrees": coordinate.degrees,
        "minutes_scaled": coordinate.minutes_scaled,
        "decimals": coordinate.decimals,
        "negative": coordinate.negative,
        "degrees_f64": coordinate.degrees_f64(),
    })
}

fn nmea_quality_value(quality: sidereon_core::nmea::GgaQuality) -> Value {
    use sidereon_core::nmea::GgaQuality as Q;
    let kind = match quality {
        Q::Invalid => "invalid",
        Q::GpsSps => "gps_sps",
        Q::Differential => "differential",
        Q::Pps => "pps",
        Q::RtkFixed => "rtk_fixed",
        Q::RtkFloat => "rtk_float",
        Q::Estimated => "estimated",
        Q::Manual => "manual",
        Q::Simulator => "simulator",
        Q::Other(_) => "other",
    };
    json!({"kind": kind, "value": quality.value()})
}

fn nmea_satellite_value(number: sidereon_core::nmea::NmeaSatNumber) -> Value {
    json!({
        "raw": number.raw,
        "resolved": number.resolved.map(|satellite| satellite.to_string()),
    })
}

fn nmea_signal_value(signal: sidereon_core::nmea::NmeaSignalId) -> Value {
    json!({
        "system": signal.system.map(nmea_system_value),
        "id": signal.id,
        "carrier_band": signal.carrier_band().map(|band| format!("{band:?}")),
    })
}

fn nmea_status_value(status: sidereon_core::nmea::RmcStatus) -> Value {
    use sidereon_core::nmea::RmcStatus as S;
    match status {
        S::Valid => json!({"kind": "valid", "value": "A"}),
        S::Warning => json!({"kind": "warning", "value": "V"}),
        S::Other(value) => json!({"kind": "other", "value": value.to_string()}),
    }
}

fn nmea_gga_value(gga: &sidereon_core::nmea::Gga) -> Value {
    json!({
        "kind": "gga",
        "time": gga.time.map(nmea_time_value),
        "latitude": gga.latitude.map(nmea_coordinate_value),
        "longitude": gga.longitude.map(nmea_coordinate_value),
        "quality": gga.quality.map(nmea_quality_value),
        "satellites_used": gga.satellites_used,
        "hdop": gga.hdop,
        "altitude_msl_m": gga.altitude_msl_m,
        "geoid_separation_m": gga.geoid_separation_m,
        "differential_age_s": gga.differential_age_s,
        "differential_station_id": gga.differential_station_id,
    })
}

fn nmea_rmc_value(rmc: &sidereon_core::nmea::Rmc) -> Value {
    json!({
        "kind": "rmc",
        "time": rmc.time.map(nmea_time_value),
        "status": rmc.status.map(nmea_status_value),
        "latitude": rmc.latitude.map(nmea_coordinate_value),
        "longitude": rmc.longitude.map(nmea_coordinate_value),
        "speed_over_ground_kn": rmc.speed_over_ground_kn,
        "course_over_ground_deg": rmc.course_over_ground_deg,
        "date": rmc.date.map(nmea_date_value),
        "magnetic_variation_deg": rmc.magnetic_variation_deg,
        "faa_mode": rmc.faa_mode.map(|value| value.to_string()),
        "navigational_status": rmc.navigational_status.map(|value| value.to_string()),
    })
}

fn nmea_gsa_value(gsa: &sidereon_core::nmea::Gsa) -> Value {
    use sidereon_core::nmea::{GsaFixMode as F, GsaSelectionMode as M};
    let selection_mode = gsa.selection_mode.map(|mode| match mode {
        M::Manual => json!({"kind": "manual", "value": "M"}),
        M::Automatic => json!({"kind": "automatic", "value": "A"}),
        M::Other(value) => json!({"kind": "other", "value": value.to_string()}),
    });
    let fix_mode = gsa.fix_mode.map(|mode| match mode {
        F::None => json!({"kind": "none", "value": 1}),
        F::TwoD => json!({"kind": "two_d", "value": 2}),
        F::ThreeD => json!({"kind": "three_d", "value": 3}),
        F::Other(value) => json!({"kind": "other", "value" : value}),
    });
    json!({
        "kind": "gsa",
        "selection_mode": selection_mode,
        "fix_mode": fix_mode,
        "satellites": gsa.satellites.iter().copied().map(nmea_satellite_value).collect::<Vec<_>>(),
        "pdop": gsa.pdop,
        "hdop": gsa.hdop,
        "vdop": gsa.vdop,
        "system_id": gsa.system_id,
        "system": gsa.system.map(nmea_system_value),
    })
}

fn nmea_gsv_satellite_value(satellite: &sidereon_core::nmea::GsvSatellite) -> Value {
    json!({
        "sat_number": satellite.sat_number.map(nmea_satellite_value),
        "elevation_deg": satellite.elevation_deg,
        "azimuth_deg": satellite.azimuth_deg,
        "cn0_db_hz": satellite.cn0_db_hz,
    })
}

fn nmea_gsv_value(gsv: &sidereon_core::nmea::Gsv) -> Value {
    json!({
        "kind": "gsv",
        "total_messages": gsv.total_messages,
        "message_number": gsv.message_number,
        "satellites_in_view": gsv.satellites_in_view,
        "satellites": gsv.satellites.iter().map(nmea_gsv_satellite_value).collect::<Vec<_>>(),
        "signal": gsv.signal.map(nmea_signal_value),
    })
}

fn nmea_gst_value(gst: &sidereon_core::nmea::Gst) -> Value {
    json!({
        "kind": "gst",
        "time": gst.time.map(nmea_time_value),
        "rms_range_residual_m": gst.rms_range_residual_m,
        "semi_major_error_m": gst.semi_major_error_m,
        "semi_minor_error_m": gst.semi_minor_error_m,
        "orientation_deg": gst.orientation_deg,
        "latitude_sigma_m": gst.latitude_sigma_m,
        "longitude_sigma_m": gst.longitude_sigma_m,
        "altitude_sigma_m": gst.altitude_sigma_m,
    })
}

fn nmea_vtg_value(vtg: &sidereon_core::nmea::Vtg) -> Value {
    json!({
        "kind": "vtg",
        "course_true_deg": vtg.course_true_deg,
        "course_magnetic_deg": vtg.course_magnetic_deg,
        "speed_kn": vtg.speed_kn,
        "speed_kmh": vtg.speed_kmh,
        "faa_mode": vtg.faa_mode.map(|value| value.to_string()),
    })
}

fn nmea_gll_value(gll: &sidereon_core::nmea::Gll) -> Value {
    json!({
        "kind": "gll",
        "latitude": gll.latitude.map(nmea_coordinate_value),
        "longitude": gll.longitude.map(nmea_coordinate_value),
        "time": gll.time.map(nmea_time_value),
        "status": gll.status.map(nmea_status_value),
        "faa_mode": gll.faa_mode.map(|value| value.to_string()),
    })
}

fn nmea_zda_value(zda: &sidereon_core::nmea::Zda) -> Value {
    json!({
        "kind": "zda",
        "time": zda.time.map(nmea_time_value),
        "date": zda.date.map(nmea_date_value),
        "local_zone_hours": zda.local_zone_hours,
        "local_zone_minutes": zda.local_zone_minutes,
    })
}

fn nmea_body_value(body: &sidereon_core::nmea::NmeaBody) -> Value {
    use sidereon_core::nmea::NmeaBody as B;
    match body {
        B::Gga(value) => nmea_gga_value(value),
        B::Rmc(value) => nmea_rmc_value(value),
        B::Gsa(value) => nmea_gsa_value(value),
        B::Gsv(value) => nmea_gsv_value(value),
        B::Gst(value) => nmea_gst_value(value),
        B::Vtg(value) => nmea_vtg_value(value),
        B::Gll(value) => nmea_gll_value(value),
        B::Zda(value) => nmea_zda_value(value),
    }
}

fn append_nmea_diagnostics(
    entries: &mut Vec<OwnedNmeaDiagnostic>,
    diagnostics: &sidereon_core::nmea::Diagnostics,
    source: SidereonNmeaDiagnosticSource,
    epoch_index: Option<usize>,
) {
    for skip in &diagnostics.skips {
        let payload = nmea_skip_value(skip).to_string();
        entries.push(OwnedNmeaDiagnostic {
            info: SidereonNmeaDiagnosticInfo {
                source,
                kind: SidereonNmeaDiagnosticKind::Skip,
                has_epoch_index: epoch_index.is_some(),
                epoch_index: epoch_index.unwrap_or(0),
                payload_len: payload.len(),
            },
            payload,
        });
    }
    for warning in &diagnostics.warnings {
        let payload = nmea_warning_value(warning).to_string();
        entries.push(OwnedNmeaDiagnostic {
            info: SidereonNmeaDiagnosticInfo {
                source,
                kind: SidereonNmeaDiagnosticKind::Warning,
                has_epoch_index: epoch_index.is_some(),
                epoch_index: epoch_index.unwrap_or(0),
                payload_len: payload.len(),
            },
            payload,
        });
    }
}

fn nmea_epoch_summary_to_c(epoch: &sidereon_core::nmea::EpochSnapshot) -> SidereonNmeaEpochSummary {
    let calendar_epoch = nmea_epoch_calendar(epoch);
    let position = epoch.position();
    let pdop = epoch.pdop();
    let hdop = epoch.hdop();
    let vdop = epoch.vdop();
    let (skip_count, warning_count) = nmea_epoch_diagnostic_counts(epoch);
    let instant_j2000_s = calendar_epoch
        .map(|epoch| {
            sidereon_core::astro::time::civil::j2000_seconds(
                epoch.year,
                epoch.month,
                epoch.day,
                epoch.hour,
                epoch.minute,
                epoch.second,
            )
        })
        .unwrap_or(0.0);

    SidereonNmeaEpochSummary {
        has_calendar_epoch: calendar_epoch.is_some(),
        calendar_epoch: calendar_epoch.unwrap_or(SidereonCalendarEpoch {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0.0,
        }),
        has_position: position.is_some(),
        position: position
            .as_ref()
            .map(geodetic_to_c)
            .unwrap_or_else(empty_geodetic),
        has_instant_j2000_s: calendar_epoch.is_some(),
        instant_j2000_s,
        has_pdop: pdop.is_some(),
        pdop: pdop.unwrap_or(0.0),
        has_hdop: hdop.is_some(),
        hdop: hdop.unwrap_or(0.0),
        has_vdop: vdop.is_some(),
        vdop: vdop.unwrap_or(0.0),
        used_satellite_count: epoch.used_satellites().count(),
        satellites_in_view: epoch.satellites_in_view(),
        sentence_count: epoch.sentence_count,
        skip_count,
        warning_count,
        has_gga: epoch.gga.is_some(),
        has_rmc: epoch.rmc.is_some(),
        has_gll: epoch.gll.is_some(),
        gsa_count: epoch.gsa.len(),
        gsv_group_count: epoch.gsv.len(),
    }
}

fn nmea_gga_quality_from_c(
    fn_name: &str,
    quality: u32,
) -> Result<sidereon_core::nmea::GgaQuality, SidereonStatus> {
    if quality > u32::from(u8::MAX) {
        set_last_error(format!("{fn_name}: invalid GGA quality {quality}"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(match quality as u8 {
        0 => sidereon_core::nmea::GgaQuality::Invalid,
        1 => sidereon_core::nmea::GgaQuality::GpsSps,
        2 => sidereon_core::nmea::GgaQuality::Differential,
        3 => sidereon_core::nmea::GgaQuality::Pps,
        4 => sidereon_core::nmea::GgaQuality::RtkFixed,
        5 => sidereon_core::nmea::GgaQuality::RtkFloat,
        6 => sidereon_core::nmea::GgaQuality::Estimated,
        7 => sidereon_core::nmea::GgaQuality::Manual,
        8 => sidereon_core::nmea::GgaQuality::Simulator,
        other => sidereon_core::nmea::GgaQuality::Other(other),
    })
}

fn nmea_talker_from_c(
    fn_name: &str,
    talker: &[c_char; NMEA_TALKER_C_BYTES],
) -> Result<sidereon_core::nmea::NmeaTalker, SidereonStatus> {
    let talker = fixed_c_array_to_string(fn_name, "talker", talker)?;
    if talker.len() != 2 || !talker.bytes().all(|byte| byte.is_ascii()) {
        set_last_error(format!("{fn_name}: talker must be exactly two ASCII bytes"));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(sidereon_core::nmea::NmeaTalker::parse(&talker))
}

fn nmea_log_from_parsed(
    parsed: sidereon_core::nmea::Parsed<sidereon_core::nmea::NmeaLog>,
) -> SidereonNmeaLog {
    let sentence_count = parsed.value.sentences.len();
    let epochs = sidereon_core::nmea::group_epochs(&parsed.value);
    let sentences = parsed.value.sentences;
    let mut skip_count = parsed.diagnostics.skips.len();
    let mut warning_count = parsed.diagnostics.warnings.len();
    let diagnostics = parsed.diagnostics;
    for epoch in &epochs {
        let (epoch_skips, epoch_warnings) = nmea_epoch_diagnostic_counts(epoch);
        skip_count += epoch_skips;
        warning_count += epoch_warnings;
    }
    SidereonNmeaLog {
        epochs,
        sentences,
        diagnostics,
        sentence_count,
        skip_count,
        warning_count,
    }
}

fn nmea_epoch_diagnostic_counts(epoch: &sidereon_core::nmea::EpochSnapshot) -> (usize, usize) {
    (
        epoch.diagnostics.skips.len(),
        epoch.diagnostics.warnings.len(),
    )
}

fn nmea_epoch_calendar(
    epoch: &sidereon_core::nmea::EpochSnapshot,
) -> Option<SidereonCalendarEpoch> {
    let date = epoch.date?;
    let time = epoch.time_of_day?;
    Some(SidereonCalendarEpoch {
        year: i32::from(date.year),
        month: i32::from(date.month),
        day: i32::from(date.day),
        hour: i32::from(time.hour),
        minute: i32::from(time.minute),
        second: f64::from(time.second) + f64::from(time.nanos) * 1.0e-9,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::snapshot_engine_error_for_test;

    const GGA: &str = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";

    unsafe fn diagnostic_payload(list: *const SidereonNmeaDiagnosticList, index: usize) -> String {
        let mut info = std::mem::MaybeUninit::<SidereonNmeaDiagnosticInfo>::uninit();
        assert_eq!(
            sidereon_nmea_diagnostic_list_get_info(list, index, info.as_mut_ptr()),
            SidereonStatus::Ok
        );
        let info = info.assume_init();
        let mut bytes = vec![0u8; info.payload_len];
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            sidereon_nmea_diagnostic_list_get_payload(
                list,
                index,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
                &mut required,
            ),
            SidereonStatus::Ok
        );
        assert_eq!(written, info.payload_len);
        assert_eq!(required, info.payload_len);
        String::from_utf8(bytes).expect("diagnostic JSON is UTF-8")
    }

    unsafe fn record_payload(list: *const SidereonNmeaRecordList, index: usize) -> Value {
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            sidereon_nmea_record_list_get_payload(
                list,
                index,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            ),
            SidereonStatus::Ok
        );
        let mut bytes = vec![0u8; required];
        assert_eq!(
            sidereon_nmea_record_list_get_payload(
                list,
                index,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
                &mut required,
            ),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        serde_json::from_slice(&bytes).expect("NMEA record JSON")
    }

    #[test]
    fn nmea_record_lists_preserve_accepted_sentences_and_complete_epoch_fields() {
        unsafe {
            let core_sentences = concat!(
                "$GPRMC,123520,A,4807.038,N,01131.000,E,22.4,84.4,230394,3.1,W,A,S*72\n",
                "$GNGSA,A,3,01,02,03,04,,,,,,,,,1.5,0.9,1.2,1*3B\n",
                "$GPGSV,2,1,05,01,45,083,42,02,17,308,40,03,25,120,39,04,10,200,35,1*68\n",
                "$GPGSV,2,2,05,05,05,010,30,,,,,,,,,1*53\n",
                "$GPGST,123520,1.2,3.4,2.3,45.0,0.5,0.6,0.7*4E\n",
                "$GPVTG,84.4,T,83.1,M,22.4,N,41.5,K,A*25\n",
                "$GPGLL,4807.038,N,01131.000,E,123520,A,A*42\n",
                "$GPZDA,123520,23,03,1994,00,00*48\n",
            );
            let text = format!("{GGA}\n{core_sentences}");
            let mut log = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_parse(text.as_ptr(), text.len(), &mut log),
                SidereonStatus::Ok
            );

            let mut sentences = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_log_sentences(log, &mut sentences),
                SidereonStatus::Ok
            );
            let mut sentence_count = 0usize;
            assert_eq!(
                sidereon_nmea_record_list_count(sentences, &mut sentence_count),
                SidereonStatus::Ok
            );
            assert_eq!(sentence_count, 9);
            let gga = record_payload(sentences, 0);
            assert_eq!(gga["kind"], "sentence");
            assert_eq!(gga["index"], 0);
            assert_eq!(gga["body"]["kind"], "gga");
            assert_eq!(gga["body"]["quality"]["kind"], "gps_sps");
            assert_eq!(gga["body"]["latitude"]["degrees"], 48);

            let rmc = record_payload(sentences, 1);
            assert_eq!(rmc["body"]["kind"], "rmc");
            assert_eq!(rmc["body"]["date"]["year"], 1994);
            assert_eq!(rmc["body"]["status"]["kind"], "valid");

            let gsa = record_payload(sentences, 2);
            assert_eq!(gsa["talker"]["kind"], "combined");
            assert_eq!(gsa["body"]["kind"], "gsa");
            assert_eq!(gsa["body"]["selection_mode"]["kind"], "automatic");
            assert_eq!(gsa["body"]["fix_mode"]["kind"], "three_d");
            assert_eq!(gsa["body"]["satellites"][0]["raw"], 1);
            assert_eq!(gsa["body"]["satellites"][0]["resolved"], "G01");
            assert_eq!(gsa["body"]["system"], "Gps");

            let gsv = record_payload(sentences, 3);
            assert_eq!(gsv["body"]["kind"], "gsv");
            assert_eq!(gsv["body"]["total_messages"], 2);
            assert_eq!(gsv["body"]["message_number"], 1);
            assert_eq!(gsv["body"]["satellites"][0]["cn0_db_hz"], 42);
            assert_eq!(gsv["body"]["signal"]["id"], 1);

            let mut epochs = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_log_epoch_records(log, &mut epochs),
                SidereonStatus::Ok
            );
            let mut epoch_count = 0usize;
            assert_eq!(
                sidereon_nmea_record_list_count(epochs, &mut epoch_count),
                SidereonStatus::Ok
            );
            assert_eq!(epoch_count, 2);
            let first_epoch = record_payload(epochs, 0);
            assert_eq!(first_epoch["gga"]["quality"]["value"], 1);
            assert!(first_epoch["rmc"].is_null());
            let second_epoch = record_payload(epochs, 1);
            assert!(second_epoch["gga"].is_null());
            assert_eq!(second_epoch["rmc"]["kind"], "rmc");
            assert_eq!(second_epoch["gst"]["rms_range_residual_m"], 1.2);
            assert_eq!(second_epoch["vtg"]["speed_kmh"], 41.5);
            assert_eq!(second_epoch["gll"]["status"]["kind"], "valid");
            assert_eq!(second_epoch["zda"]["local_zone_minutes"], 0);
            assert_eq!(second_epoch["gsa"][0]["body"]["system_id"], 1);
            assert_eq!(second_epoch["gsa"][0]["body"]["satellites"][3]["raw"], 4);
            assert_eq!(
                second_epoch["gsv"][0]["satellites"]
                    .as_array()
                    .unwrap()
                    .len(),
                7
            );
            assert_eq!(second_epoch["gsv"][0]["claimed_in_view"], 5);
            assert_eq!(
                second_epoch["gsv"][0]["satellites"][4]["sat_number"]["raw"],
                5
            );
            for empty_slot in [5, 6] {
                assert!(second_epoch["gsv"][0]["satellites"][empty_slot]["sat_number"].is_null());
                assert!(
                    second_epoch["gsv"][0]["satellites"][empty_slot]["elevation_deg"].is_null()
                );
                assert!(second_epoch["gsv"][0]["satellites"][empty_slot]["azimuth_deg"].is_null());
                assert!(second_epoch["gsv"][0]["satellites"][empty_slot]["cn0_db_hz"].is_null());
            }
            assert_eq!(second_epoch["gsv"][0]["complete"], true);
            assert_eq!(second_epoch["sentence_count"], 8);

            sidereon_nmea_log_free(log);
            assert_eq!(
                record_payload(sentences, 2)["body"]["satellites"][0]["resolved"],
                "G01"
            );
            assert_eq!(record_payload(epochs, 1)["zda"]["date"]["day"], 23);
            sidereon_nmea_record_list_free(sentences);
            sidereon_nmea_record_list_free(epochs);

            let mut accumulator = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_new(&mut accumulator),
                SidereonStatus::Ok
            );
            let mut summary = std::mem::MaybeUninit::<SidereonNmeaChunkSummary>::uninit();
            assert_eq!(
                sidereon_nmea_accumulator_push(
                    accumulator,
                    text.as_ptr(),
                    text.len(),
                    summary.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            let mut finished = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            assert_eq!(
                sidereon_nmea_accumulator_finish(accumulator, &mut finished),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_accumulator_sentences(accumulator, &mut sentences),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_record_list_count(sentences, &mut sentence_count),
                SidereonStatus::Ok
            );
            assert_eq!(sentence_count, 9);
            assert_eq!(
                sidereon_nmea_accumulator_epoch_records(accumulator, &mut epochs),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_record_list_count(epochs, &mut epoch_count),
                SidereonStatus::Ok
            );
            assert_eq!(epoch_count, 2);
            assert_eq!(record_payload(sentences, 1)["body"]["kind"], "rmc");
            assert_eq!(record_payload(epochs, 1)["gsa"][0]["body"]["system_id"], 1);
            sidereon_nmea_record_list_free(sentences);
            sidereon_nmea_record_list_free(epochs);
            sidereon_nmea_accumulator_free(accumulator);
        }
    }

    #[test]
    fn accumulator_finish_retains_final_sentences_snapshots_and_diagnostics() {
        unsafe {
            const FIRST: &[u8] =
                b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n";
            const FINAL: &[u8] = b"$GPGGA,123520,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,";
            let mut accumulator = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_new(&mut accumulator),
                SidereonStatus::Ok
            );
            let mut summary = std::mem::MaybeUninit::<SidereonNmeaChunkSummary>::uninit();
            assert_eq!(
                sidereon_nmea_accumulator_push(
                    accumulator,
                    FIRST.as_ptr(),
                    FIRST.len(),
                    summary.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            assert_eq!(summary.assume_init().sentence_count, 1);
            assert_eq!(
                sidereon_nmea_accumulator_push(
                    accumulator,
                    FINAL.as_ptr(),
                    FINAL.len(),
                    summary.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            assert_eq!(summary.assume_init().sentence_count, 0);
            assert_eq!(
                sidereon_nmea_accumulator_finish(accumulator, summary.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let finished = summary.assume_init();
            assert_eq!(finished.sentence_count, 1);
            assert_eq!(finished.completed_epoch_count, 2);
            assert_eq!(finished.skip_count, 0);
            assert_eq!(finished.warning_count, 1);
            assert_eq!(finished.retained_len, 0);

            let mut sentence_records = ptr::null_mut();
            let mut epoch_records = ptr::null_mut();
            let mut diagnostics = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_sentences(accumulator, &mut sentence_records),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_accumulator_epoch_records(accumulator, &mut epoch_records),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_accumulator_diagnostics(accumulator, &mut diagnostics),
                SidereonStatus::Ok
            );
            let mut count = 0usize;
            assert_eq!(
                sidereon_nmea_record_list_count(sentence_records, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 2);
            assert_eq!(
                record_payload(sentence_records, 1)["body"]["time"]["second"],
                20
            );
            assert_eq!(
                sidereon_nmea_record_list_count(epoch_records, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 2);
            assert_eq!(record_payload(epoch_records, 0)["time"]["second"], 19);
            assert_eq!(record_payload(epoch_records, 1)["time"]["second"], 20);
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(diagnostics, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            let payload: serde_json::Value =
                serde_json::from_str(&diagnostic_payload(diagnostics, 0)).expect("warning JSON");
            assert_eq!(payload["fields"]["at"]["line"], 2);
            assert_eq!(payload["fields"]["warning_kind"], "missing_metadata");

            assert_eq!(
                sidereon_nmea_accumulator_finish(accumulator, summary.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let repeated = summary.assume_init();
            assert_eq!(repeated.sentence_count, 0);
            assert_eq!(repeated.completed_epoch_count, 0);
            assert_eq!(repeated.skip_count, 0);
            assert_eq!(repeated.warning_count, 0);
            sidereon_nmea_accumulator_free(accumulator);
            assert_eq!(record_payload(epoch_records, 1)["time"]["second"], 20);
            assert!(diagnostic_payload(diagnostics, 0).contains("missing_metadata"));
            sidereon_nmea_record_list_free(sentence_records);
            sidereon_nmea_record_list_free(epoch_records);
            sidereon_nmea_diagnostic_list_free(diagnostics);

            let mut malformed = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_new(&mut malformed),
                SidereonStatus::Ok
            );
            let bad_line = b"bad line";
            assert_eq!(
                sidereon_nmea_accumulator_push(
                    malformed,
                    bad_line.as_ptr(),
                    bad_line.len(),
                    summary.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_accumulator_finish(malformed, summary.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let malformed_summary = summary.assume_init();
            assert_eq!(malformed_summary.sentence_count, 0);
            assert_eq!(malformed_summary.completed_epoch_count, 0);
            assert_eq!(malformed_summary.skip_count, 1);
            let mut malformed_diagnostics = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_diagnostics(malformed, &mut malformed_diagnostics),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(malformed_diagnostics, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            let skip: serde_json::Value =
                serde_json::from_str(&diagnostic_payload(malformed_diagnostics, 0))
                    .expect("skip JSON");
            assert_eq!(skip["fields"]["at"]["line"], 1);
            sidereon_nmea_accumulator_free(malformed);
            assert_eq!(skip["fields"]["at"]["line"], 1);
            sidereon_nmea_diagnostic_list_free(malformed_diagnostics);
        }
    }

    unsafe fn last_error_text() -> String {
        let required = sidereon_last_error_message(ptr::null_mut(), 0);
        let mut bytes = vec![0 as c_char; required + 1];
        assert_eq!(
            sidereon_last_error_message(bytes.as_mut_ptr(), bytes.len()),
            required
        );
        std::ffi::CStr::from_ptr(bytes.as_ptr())
            .to_string_lossy()
            .into_owned()
    }

    fn valid_gga_options() -> SidereonNmeaGgaOptions {
        SidereonNmeaGgaOptions {
            talker: [b'G' as c_char, b'P' as c_char, 0],
            utc_seconds_of_day: 43_519.0,
            position: SidereonGeodetic {
                lat_rad: 48.1173_f64.to_radians(),
                lon_rad: 11.5166666667_f64.to_radians(),
                height_m: 592.3,
            },
            quality: 1,
            satellites_used: 8,
            hdop: 0.9,
            coordinate_decimals: 3,
        }
    }

    #[test]
    fn nmea_owned_diagnostics_retain_parser_and_epoch_causes_after_source_free() {
        unsafe {
            let text = format!("prefix{GGA}\nbad line\n{GGA}\n");
            let mut log = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_parse(text.as_ptr(), text.len(), &mut log),
                SidereonStatus::Ok
            );
            let mut list = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_log_diagnostics(log, &mut list),
                SidereonStatus::Ok
            );
            let mut count = 0usize;
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 3);
            let mut info_out = std::mem::MaybeUninit::<SidereonNmeaDiagnosticInfo>::uninit();
            assert_eq!(
                sidereon_nmea_diagnostic_list_get_info(list, 0, info_out.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let info = info_out.assume_init();
            assert_eq!(info.source, SidereonNmeaDiagnosticSource::Parser);
            assert_eq!(info.kind, SidereonNmeaDiagnosticKind::Skip);
            let skip: serde_json::Value =
                serde_json::from_str(&diagnostic_payload(list, 0)).expect("skip JSON");
            assert_eq!(skip["fields"]["at"]["line"], 2);
            assert_eq!(skip["fields"]["reason"]["kind"], "unknown_block");
            assert_eq!(
                skip["fields"]["reason"]["fields"]["block"],
                "no NMEA start delimiter"
            );

            let mut short = [0xa5u8; 2];
            let mut written = 7usize;
            let mut required = 7usize;
            assert_eq!(
                sidereon_nmea_diagnostic_list_get_payload(
                    list,
                    0,
                    short.as_mut_ptr(),
                    short.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(short, [0xa5; 2]);
            assert_eq!(written, 0);
            assert!(required > short.len());
            let mut untouched = SidereonNmeaDiagnosticInfo {
                source: SidereonNmeaDiagnosticSource::Parser,
                kind: SidereonNmeaDiagnosticKind::Warning,
                has_epoch_index: true,
                epoch_index: 999,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_nmea_diagnostic_list_get_info(list, usize::MAX, &mut untouched,),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(untouched.epoch_index, 999);
            assert!(diagnostic_payload(list, 0).contains("no NMEA start delimiter"));

            assert_eq!(
                sidereon_nmea_diagnostic_list_get_info(list, 1, info_out.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let parser_warning_info = info_out.assume_init();
            assert_eq!(
                parser_warning_info.source,
                SidereonNmeaDiagnosticSource::Parser
            );
            assert_eq!(
                parser_warning_info.kind,
                SidereonNmeaDiagnosticKind::Warning
            );
            let parser_warning: serde_json::Value =
                serde_json::from_str(&diagnostic_payload(list, 1)).expect("parser warning JSON");
            assert_eq!(parser_warning["fields"]["warning_kind"], "mismatch");
            assert_eq!(parser_warning["fields"]["at"]["line"], 1);

            assert_eq!(
                sidereon_nmea_diagnostic_list_get_info(list, 2, info_out.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let epoch_info = info_out.assume_init();
            assert_eq!(
                epoch_info.source,
                SidereonNmeaDiagnosticSource::EpochAssembly
            );
            assert!(epoch_info.has_epoch_index);
            assert_eq!(epoch_info.epoch_index, 0);
            let warning: serde_json::Value =
                serde_json::from_str(&diagnostic_payload(list, 2)).expect("warning JSON");
            assert_eq!(warning["kind"], "warning");
            assert_eq!(warning["fields"]["warning_kind"], "mismatch");

            let mut epoch_list = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_log_epoch_diagnostics(log, 0, &mut epoch_list),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(epoch_list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            sidereon_nmea_diagnostic_list_free(epoch_list);

            sidereon_nmea_log_free(log);
            assert!(diagnostic_payload(list, 0).contains("no NMEA start delimiter"));
            sidereon_nmea_diagnostic_list_free(list);
            sidereon_nmea_diagnostic_list_free(ptr::null_mut());

            let mut accumulator = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_new(&mut accumulator),
                SidereonStatus::Ok
            );
            let chunk = format!("{GGA}\nbad line\n{GGA}\n");
            let mut summary = std::mem::MaybeUninit::<SidereonNmeaChunkSummary>::uninit();
            assert_eq!(
                sidereon_nmea_accumulator_push(
                    accumulator,
                    chunk.as_ptr(),
                    chunk.len(),
                    summary.as_mut_ptr(),
                ),
                SidereonStatus::Ok
            );
            let mut finished = SidereonNmeaChunkSummary {
                sentence_count: 0,
                completed_epoch_count: 0,
                skip_count: 0,
                warning_count: 0,
                retained_len: 0,
            };
            assert_eq!(
                sidereon_nmea_accumulator_finish(accumulator, &mut finished),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_accumulator_diagnostics(accumulator, &mut list),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 2);
            sidereon_nmea_diagnostic_list_free(list);
            list = ptr::null_mut();
            assert_eq!(
                sidereon_nmea_accumulator_epoch_diagnostics(accumulator, 0, &mut list),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 1);
            sidereon_nmea_diagnostic_list_free(list);
            assert_eq!(
                sidereon_nmea_accumulator_diagnostics(accumulator, &mut list),
                SidereonStatus::Ok
            );
            assert_eq!(
                sidereon_nmea_diagnostic_list_count(list, &mut count),
                SidereonStatus::Ok
            );
            assert_eq!(count, 2);
            sidereon_nmea_accumulator_free(accumulator);
            assert!(diagnostic_payload(list, 0).contains("no NMEA start delimiter"));
            sidereon_nmea_diagnostic_list_free(list);
        }
    }

    #[test]
    fn nmea_writer_records_typed_error_and_clears_it_on_success_and_null() {
        unsafe {
            let mut options = valid_gga_options();
            options.utc_seconds_of_day = f64::NAN;
            let mut output = [0u8; 128];
            let mut written = 9usize;
            let mut required = 9usize;
            assert_eq!(
                sidereon_nmea_write_gga(
                    &options,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);
            assert_eq!(
                sidereon_last_error_message(std::ptr::null_mut(), 0),
                "sidereon_nmea_write_gga: invalid input time: must be finite and in [0, 86400)"
                    .len()
            );
            assert_eq!(
                last_error_text(),
                "sidereon_nmea_write_gga: invalid input time: must be finite and in [0, 86400)"
            );
            let error = snapshot_engine_error_for_test().expect("typed NMEA writer refusal");
            assert_eq!(error.0.family, SidereonEngineErrorFamily::Nmea);
            let payload: serde_json::Value = serde_json::from_str(&error.1).expect("error JSON");
            assert_eq!(payload["family"], "nmea");
            assert_eq!(payload["operation"], "sidereon_nmea_write_gga");
            assert_eq!(payload["error"]["kind"], "invalid_input");
            assert_eq!(payload["error"]["fields"]["field"], "time");

            options = valid_gga_options();
            assert_eq!(
                sidereon_nmea_write_gga(
                    &options,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::Ok
            );
            assert!(snapshot_engine_error_for_test().is_none());

            options.utc_seconds_of_day = f64::NAN;
            assert_eq!(
                sidereon_nmea_write_gga(
                    &options,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(snapshot_engine_error_for_test().is_some());
            assert_eq!(
                sidereon_nmea_write_gga(
                    ptr::null(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(written, 0);
            assert_eq!(required, 0);
            assert!(snapshot_engine_error_for_test().is_none());
        }
    }
}
