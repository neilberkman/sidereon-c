use super::*;

// --- SSR decode accessors, correction store, and corrected broadcast source --

/// Start an SSR ingest or snapshot operation without a typed RTCM error from
/// an earlier operation on this thread.
fn ssr_operation_boundary<T>(fn_name: &str, panic_value: T, body: impl FnOnce() -> T) -> T {
    crate::rtcm::clear_rtcm_typed_error();
    clear_engine_error();
    ffi_boundary(fn_name, panic_value, body)
}

pub struct SidereonSsrCorrectionStore {
    pub(crate) inner: SsrCorrectionStore,
}

/// An opaque decoded RTCM SSR message body. The handle owns one
/// `sidereon_core::rtcm::SsrMessage` and is released with
/// sidereon_ssr_message_free.
pub struct SidereonSsrMessage {
    pub(crate) inner: RtcmSsrMessage,
}

/// What an SSR-corrected state does with a correction above the declared size
/// limit (`sidereon_core::ssr::SsrCorrectionSizePolicy`).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrCorrectionSizePolicy {
    /// Refuse the corrected state and report the measured size.
    Strict = 0,
    /// Apply the correction and report the measured size.
    Lenient = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrVtecQueryKind {
    NoModel = 0,
    BeforeModel = 1,
    Stale = 2,
    Evaluated = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrVtecLayerEvaluation {
    pub pierce_latitude_rad: f64,
    pub pierce_longitude_rad: f64,
    pub sun_fixed_longitude_rad: f64,
    pub vtec_tecu: f64,
    pub mapping_factor: f64,
    pub stec_tecu: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrVtecQueryResult {
    pub kind: SidereonSsrVtecQueryKind,
    pub age_s: f64,
    pub seconds_before_model: f64,
    pub max_age_s: f64,
    pub layer_count: usize,
    pub stec_tecu: f64,
    pub pseudorange_delay_m: f64,
    pub phase_range_advance_m: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrCorrectionSize {
    pub orbit_m: f64,
    pub clock_m: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrCorrectedStateResult {
    pub has_state: bool,
    pub position_ecef_m: [f64; 3],
    pub clock_s: f64,
    pub has_group_delay: bool,
    pub group_delay_s: f64,
    pub degraded: bool,
    pub has_size_event: bool,
    pub strict_refusal: bool,
    pub size: SidereonSsrCorrectionSize,
    pub has_oversized_report: bool,
    pub source: u32,
    pub provider_id: u16,
    pub solution_id: u8,
    pub orbit_ref_epoch_j2000_s: f64,
    pub clock_ref_epoch_j2000_s: f64,
    pub first_applied_epoch_j2000_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmSsrKind {
    Orbit = 0,
    Clock = 1,
    CombinedOrbitClock = 2,
    CodeBias = 3,
    PhaseBias = 4,
    Ura = 5,
    HighRateClock = 6,
    Vtec = 7,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrReferencePoint {
    AntennaPhaseCenter = 0,
    CenterOfMass = 1,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSsrMissingCorrectionAction {
    Decline = 0,
    FallBackToBroadcast = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrHeader {
    pub epoch_time_s: u32,
    pub update_interval: u8,
    pub multiple_message: bool,
    pub iod_ssr: u8,
    pub provider_id: u16,
    pub solution_id: u8,
    pub has_satellite_reference_datum: bool,
    pub satellite_reference_datum: bool,
    pub has_dispersive_bias_consistency: bool,
    pub dispersive_bias_consistency: bool,
    pub has_mw_consistency: bool,
    pub mw_consistency: bool,
    pub satellite_count: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrInfo {
    pub message_number: u16,
    pub system: SidereonGnssSystem,
    pub kind: SidereonRtcmSsrKind,
    pub header: SidereonRtcmSsrHeader,
    pub orbit_count: usize,
    pub clock_count: usize,
    pub ura_count: usize,
    pub code_bias_count: usize,
    pub phase_bias_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrOrbitRecord {
    pub satellite_id: u8,
    pub iode: u32,
    pub has_iod_crc: bool,
    pub iod_crc: u32,
    pub delta_radial: i32,
    pub delta_along: i32,
    pub delta_cross: i32,
    pub dot_delta_radial: i32,
    pub dot_delta_along: i32,
    pub dot_delta_cross: i32,
}

/// One satellite's raw RTCM SSR code-bias record. The nested signal rows are
/// copied with sidereon_rtcm_message_ssr_code_bias_signals or
/// sidereon_ssr_message_code_bias_signals.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrCodeBiasRecord {
    /// Constellation-native satellite id.
    pub satellite_id: u8,
    /// Number of signal rows belonging to this satellite record.
    pub signal_count: usize,
}

/// One raw signal and bias pair in an RTCM SSR code-bias record.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrCodeBiasSignal {
    /// Raw signal and tracking-mode id.
    pub signal_id: u8,
    /// Raw code bias integer.
    pub bias: i16,
}

/// One satellite's raw RTCM SSR phase-bias record. The nested signal rows are
/// copied with sidereon_rtcm_message_ssr_phase_bias_signals or
/// sidereon_ssr_message_phase_bias_signals.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrPhaseBiasRecord {
    /// Constellation-native satellite id.
    pub satellite_id: u8,
    /// Raw yaw angle.
    pub yaw_angle: u16,
    /// Raw yaw rate.
    pub yaw_rate: i8,
    /// Number of signal rows belonging to this satellite record.
    pub signal_count: usize,
}

/// One raw signal row in an RTCM SSR phase-bias record.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrPhaseBiasSignal {
    /// Raw signal and tracking-mode id.
    pub signal_id: u8,
    /// Signal integer indicator.
    pub integer_indicator: u8,
    /// Wide-lane integer indicator.
    pub wide_lane_integer_indicator: u8,
    /// Discontinuity counter.
    pub discontinuity_counter: u8,
    /// Raw phase bias integer.
    pub bias: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrClockRecord {
    pub satellite_id: u8,
    pub c0: i32,
    pub c1: i32,
    pub c2: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrUraRecord {
    pub satellite_id: u8,
    pub ura_index: u8,
}

pub(crate) fn ssr_info_to_c(message: &RtcmSsrMessage) -> SidereonRtcmSsrInfo {
    SidereonRtcmSsrInfo {
        message_number: message.message_number,
        system: gnss_system_to_c(message.system),
        kind: ssr_kind_to_c(message.kind),
        header: ssr_header_to_c(&message.header),
        orbit_count: message.orbit.len(),
        clock_count: message.clock.len(),
        ura_count: message.ura.len(),
        code_bias_count: message.code_bias.len(),
        phase_bias_count: message.phase_bias.len(),
    }
}

fn ssr_kind_to_c(kind: RtcmSsrKind) -> SidereonRtcmSsrKind {
    match kind {
        RtcmSsrKind::Orbit => SidereonRtcmSsrKind::Orbit,
        RtcmSsrKind::Clock => SidereonRtcmSsrKind::Clock,
        RtcmSsrKind::CombinedOrbitClock => SidereonRtcmSsrKind::CombinedOrbitClock,
        RtcmSsrKind::CodeBias => SidereonRtcmSsrKind::CodeBias,
        RtcmSsrKind::PhaseBias => SidereonRtcmSsrKind::PhaseBias,
        RtcmSsrKind::Ura => SidereonRtcmSsrKind::Ura,
        RtcmSsrKind::HighRateClock => SidereonRtcmSsrKind::HighRateClock,
    }
}

fn ssr_header_to_c(header: &RtcmSsrHeader) -> SidereonRtcmSsrHeader {
    SidereonRtcmSsrHeader {
        epoch_time_s: header.epoch_time_s,
        update_interval: header.update_interval,
        multiple_message: header.multiple_message,
        iod_ssr: header.iod_ssr,
        provider_id: header.provider_id,
        solution_id: header.solution_id,
        has_satellite_reference_datum: header.satellite_reference_datum.is_some(),
        satellite_reference_datum: header.satellite_reference_datum.unwrap_or(false),
        has_dispersive_bias_consistency: header.dispersive_bias_consistency.is_some(),
        dispersive_bias_consistency: header.dispersive_bias_consistency.unwrap_or(false),
        has_mw_consistency: header.mw_consistency.is_some(),
        mw_consistency: header.mw_consistency.unwrap_or(false),
        satellite_count: header.satellite_count,
    }
}

fn ssr_rtcm_orbit_to_c(record: &RtcmSsrOrbitRecord) -> SidereonRtcmSsrOrbitRecord {
    SidereonRtcmSsrOrbitRecord {
        satellite_id: record.satellite_id,
        iode: record.iode,
        has_iod_crc: record.iod_crc.is_some(),
        iod_crc: record.iod_crc.unwrap_or(0),
        delta_radial: record.delta_radial,
        delta_along: record.delta_along,
        delta_cross: record.delta_cross,
        dot_delta_radial: record.dot_delta_radial,
        dot_delta_along: record.dot_delta_along,
        dot_delta_cross: record.dot_delta_cross,
    }
}

fn ssr_rtcm_clock_to_c(record: &RtcmSsrClockRecord) -> SidereonRtcmSsrClockRecord {
    SidereonRtcmSsrClockRecord {
        satellite_id: record.satellite_id,
        c0: record.c0,
        c1: record.c1,
        c2: record.c2,
    }
}

fn ssr_code_bias_to_c(record: &RtcmSsrCodeBiasRecord) -> SidereonRtcmSsrCodeBiasRecord {
    SidereonRtcmSsrCodeBiasRecord {
        satellite_id: record.satellite_id,
        signal_count: record.biases.len(),
    }
}

fn ssr_code_bias_signal_to_c(signal_id: u8, bias: i16) -> SidereonRtcmSsrCodeBiasSignal {
    SidereonRtcmSsrCodeBiasSignal { signal_id, bias }
}

fn ssr_phase_bias_to_c(record: &RtcmSsrPhaseBiasRecord) -> SidereonRtcmSsrPhaseBiasRecord {
    SidereonRtcmSsrPhaseBiasRecord {
        satellite_id: record.satellite_id,
        yaw_angle: record.yaw_angle,
        yaw_rate: record.yaw_rate,
        signal_count: record.biases.len(),
    }
}

fn ssr_phase_bias_signal_to_c(signal: &RtcmSsrPhaseBiasSignal) -> SidereonRtcmSsrPhaseBiasSignal {
    SidereonRtcmSsrPhaseBiasSignal {
        signal_id: signal.signal_id,
        integer_indicator: signal.integer_indicator,
        wide_lane_integer_indicator: signal.wide_lane_integer_indicator,
        discontinuity_counter: signal.discontinuity_counter,
        bias: signal.bias,
    }
}

fn ssr_ura_to_c(satellite_id: u8, ura_index: u8) -> SidereonRtcmSsrUraRecord {
    SidereonRtcmSsrUraRecord {
        satellite_id,
        ura_index,
    }
}

pub(crate) unsafe fn ssr_copy_orbits(
    fn_name: &str,
    message: &RtcmSsrMessage,
    out: *mut SidereonRtcmSsrOrbitRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let rows: Vec<SidereonRtcmSsrOrbitRecord> =
        message.orbit.iter().map(ssr_rtcm_orbit_to_c).collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_clocks(
    fn_name: &str,
    message: &RtcmSsrMessage,
    out: *mut SidereonRtcmSsrClockRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let rows: Vec<SidereonRtcmSsrClockRecord> =
        message.clock.iter().map(ssr_rtcm_clock_to_c).collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_code_biases(
    fn_name: &str,
    message: &RtcmSsrMessage,
    out: *mut SidereonRtcmSsrCodeBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let rows: Vec<SidereonRtcmSsrCodeBiasRecord> =
        message.code_bias.iter().map(ssr_code_bias_to_c).collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_code_bias_signals(
    fn_name: &str,
    message: &RtcmSsrMessage,
    record_index: usize,
    out: *mut SidereonRtcmSsrCodeBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    init_copy_counts(fn_name, out_written, out_required)?;
    let record = match message.code_bias.get(record_index) {
        Some(record) => record,
        None => {
            set_last_error(format!(
                "{fn_name}: record index {record_index} out of range ({} code-bias records)",
                message.code_bias.len()
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let rows: Vec<SidereonRtcmSsrCodeBiasSignal> = record
        .biases
        .iter()
        .map(|&(signal_id, bias)| ssr_code_bias_signal_to_c(signal_id, bias))
        .collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_phase_biases(
    fn_name: &str,
    message: &RtcmSsrMessage,
    out: *mut SidereonRtcmSsrPhaseBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let rows: Vec<SidereonRtcmSsrPhaseBiasRecord> =
        message.phase_bias.iter().map(ssr_phase_bias_to_c).collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_phase_bias_signals(
    fn_name: &str,
    message: &RtcmSsrMessage,
    record_index: usize,
    out: *mut SidereonRtcmSsrPhaseBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    init_copy_counts(fn_name, out_written, out_required)?;
    let record = match message.phase_bias.get(record_index) {
        Some(record) => record,
        None => {
            set_last_error(format!(
                "{fn_name}: record index {record_index} out of range ({} phase-bias records)",
                message.phase_bias.len()
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let rows: Vec<SidereonRtcmSsrPhaseBiasSignal> = record
        .biases
        .iter()
        .map(ssr_phase_bias_signal_to_c)
        .collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

pub(crate) unsafe fn ssr_copy_ura(
    fn_name: &str,
    message: &RtcmSsrMessage,
    out: *mut SidereonRtcmSsrUraRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let rows: Vec<SidereonRtcmSsrUraRecord> = message
        .ura
        .iter()
        .map(|&(satellite_id, ura_index)| ssr_ura_to_c(satellite_id, ura_index))
        .collect();
    copy_prefix_to_c(fn_name, "out", &rows, out, len, out_written, out_required)
}

fn map_ssr_message_decode_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
    }
}

/// Decode one bare RTCM SSR message body. The body excludes the RTCM
/// transport preamble, length, and CRC. On success the returned handle owns
/// the decoded `sidereon_core::rtcm::SsrMessage` and is released with
/// sidereon_ssr_message_free.
///
/// Safety: body points to len readable bytes; out points to a
/// SidereonSsrMessage*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_decode(
    body: *const u8,
    len: usize,
    out: *mut *mut SidereonSsrMessage,
) -> SidereonStatus {
    ssr_operation_boundary("sidereon_ssr_message_decode", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_ssr_message_decode", "out"));
        *out = ptr::null_mut();
        let body = c_try!(require_slice(
            body,
            len,
            "sidereon_ssr_message_decode",
            "body"
        ));
        match RtcmSsrMessage::decode(body) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonSsrMessage { inner });
                SidereonStatus::Ok
            }
            Err(err) => map_ssr_message_decode_error("sidereon_ssr_message_decode", err),
        }
    })
}

/// Release a bare RTCM SSR message handle. Passing NULL is a no-op.
///
/// Safety: message must be NULL or a live handle returned by
/// sidereon_ssr_message_decode.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_free(message: *mut SidereonSsrMessage) {
    free_boxed(message);
}

/// Copy the bare RTCM SSR message summary into *out_info. The summary includes
/// all five record counts and the raw common SSR header.
///
/// Safety: message is a live handle; out_info points to a
/// SidereonRtcmSsrInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_info(
    message: *const SidereonSsrMessage,
    out_info: *mut SidereonRtcmSsrInfo,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_message_info", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_info,
            "sidereon_ssr_message_info",
            "out_info"
        ));
        let message = c_try!(require_ref(message, "sidereon_ssr_message_info", "message"));
        *out = ssr_info_to_c(&message.inner);
        SidereonStatus::Ok
    })
}

/// Copy the bare RTCM SSR message's orbit records. Values are raw wire
/// integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrOrbitRecord values or is NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_orbits(
    message: *const SidereonSsrMessage,
    out: *mut SidereonRtcmSsrOrbitRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_message_orbits", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ssr_message_orbits",
            out_written,
            out_required
        ));
        let message = c_try!(require_ref(
            message,
            "sidereon_ssr_message_orbits",
            "message"
        ));
        c_try!(ssr_copy_orbits(
            "sidereon_ssr_message_orbits",
            &message.inner,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the bare RTCM SSR message's clock records. Values are raw wire
/// integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrClockRecord values or is NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_clocks(
    message: *const SidereonSsrMessage,
    out: *mut SidereonRtcmSsrClockRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_message_clocks", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ssr_message_clocks",
            out_written,
            out_required
        ));
        let message = c_try!(require_ref(
            message,
            "sidereon_ssr_message_clocks",
            "message"
        ));
        c_try!(ssr_copy_clocks(
            "sidereon_ssr_message_clocks",
            &message.inner,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the bare RTCM SSR message's per-satellite code-bias records. Each row
/// exposes its nested signal count. Values are raw wire integers.
/// Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrCodeBiasRecord values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_code_biases(
    message: *const SidereonSsrMessage,
    out: *mut SidereonRtcmSsrCodeBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_message_code_biases",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ssr_message_code_biases",
                out_written,
                out_required
            ));
            let message = c_try!(require_ref(
                message,
                "sidereon_ssr_message_code_biases",
                "message"
            ));
            c_try!(ssr_copy_code_biases(
                "sidereon_ssr_message_code_biases",
                &message.inner,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the code-bias signal rows for one bare-message satellite record.
/// Values are raw wire integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrCodeBiasSignal values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_code_bias_signals(
    message: *const SidereonSsrMessage,
    record_index: usize,
    out: *mut SidereonRtcmSsrCodeBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_message_code_bias_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ssr_message_code_bias_signals",
                out_written,
                out_required
            ));
            let message = c_try!(require_ref(
                message,
                "sidereon_ssr_message_code_bias_signals",
                "message"
            ));
            c_try!(ssr_copy_code_bias_signals(
                "sidereon_ssr_message_code_bias_signals",
                &message.inner,
                record_index,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the bare RTCM SSR message's per-satellite phase-bias records. Each row
/// exposes its yaw fields and nested signal count. Values are raw wire
/// integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrPhaseBiasRecord values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_phase_biases(
    message: *const SidereonSsrMessage,
    out: *mut SidereonRtcmSsrPhaseBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_message_phase_biases",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ssr_message_phase_biases",
                out_written,
                out_required
            ));
            let message = c_try!(require_ref(
                message,
                "sidereon_ssr_message_phase_biases",
                "message"
            ));
            c_try!(ssr_copy_phase_biases(
                "sidereon_ssr_message_phase_biases",
                &message.inner,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the phase-bias signal rows for one bare-message satellite record.
/// Values are raw wire integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrPhaseBiasSignal values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_phase_bias_signals(
    message: *const SidereonSsrMessage,
    record_index: usize,
    out: *mut SidereonRtcmSsrPhaseBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ssr_message_phase_bias_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ssr_message_phase_bias_signals",
                out_written,
                out_required
            ));
            let message = c_try!(require_ref(
                message,
                "sidereon_ssr_message_phase_bias_signals",
                "message"
            ));
            c_try!(ssr_copy_phase_bias_signals(
                "sidereon_ssr_message_phase_bias_signals",
                &message.inner,
                record_index,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the bare RTCM SSR message's URA records. Values are raw wire
/// integers. Variable-length output contract.
///
/// Safety: message is a live handle; out points to len
/// SidereonRtcmSsrUraRecord values or is NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_message_ura(
    message: *const SidereonSsrMessage,
    out: *mut SidereonRtcmSsrUraRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ssr_message_ura", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ssr_message_ura",
            out_written,
            out_required
        ));
        let message = c_try!(require_ref(message, "sidereon_ssr_message_ura", "message"));
        c_try!(ssr_copy_ura(
            "sidereon_ssr_message_ura",
            &message.inner,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrOrbitCorrection {
    pub source: u32,
    pub provider_id: u16,
    pub solution_id: u8,
    /// Whether the correction came from Galileo HAS and so carries the
    /// navigation-message index its mask states. False for an RTCM SSR
    /// correction.
    pub has_nav_message: bool,
    /// The HAS navigation-message index NM as transmitted (0 is GPS LNAV or
    /// Galileo I/NAV; 1..=7 are reserved, and such a correction is stored but
    /// not applied). Zero when has_nav_message is false.
    pub has_nav_message_index: u8,
    pub iode: u32,
    pub iod_ssr: u8,
    pub crs_regional: bool,
    pub reference_point: SidereonSsrReferencePoint,
    pub radial_m: f64,
    pub along_m: f64,
    pub cross_m: f64,
    pub radial_rate_m_s: f64,
    pub along_rate_m_s: f64,
    pub cross_rate_m_s: f64,
    pub ref_epoch_j2000_s: f64,
    pub update_interval_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSsrClockCorrection {
    pub source: u32,
    pub provider_id: u16,
    pub solution_id: u8,
    /// Whether the correction came from Galileo HAS and so carries the
    /// navigation-message index its mask states. False for an RTCM SSR
    /// correction.
    pub has_nav_message: bool,
    /// The HAS navigation-message index NM as transmitted (0 is GPS LNAV or
    /// Galileo I/NAV; 1..=7 are reserved, and such a correction is stored but
    /// not applied). Zero when has_nav_message is false.
    pub has_nav_message_index: u8,
    pub iod_ssr: u8,
    pub c0_m: f64,
    pub c1_m_s: f64,
    pub c2_m_s2: f64,
    pub ref_epoch_j2000_s: f64,
    pub update_interval_s: f64,
    pub has_high_rate: bool,
    pub high_rate_c0_m: f64,
    pub high_rate_ref_epoch_j2000_s: f64,
    pub high_rate_update_interval_s: f64,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_new(
    reference_point: u32,
    out_store: *mut *mut SidereonSsrCorrectionStore,
) -> SidereonStatus {
    ssr_operation_boundary("sidereon_ssr_store_new", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_store,
            "sidereon_ssr_store_new",
            "out_store"
        ));
        *out = ptr::null_mut();
        let reference_point = c_try!(ssr_reference_point_from_c(
            "sidereon_ssr_store_new",
            reference_point
        ));
        write_boxed_handle(
            out,
            SidereonSsrCorrectionStore {
                inner: SsrCorrectionStore::new().with_reference_point(reference_point),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_set_vtec_max_age(
    store: *mut SidereonSsrCorrectionStore,
    max_age_s: f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_store_set_vtec_max_age";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let store = c_try!(require_mut(store, FN_NAME, "store"));
        if !max_age_s.is_finite() || max_age_s < 0.0 {
            set_last_error(format!(
                "{FN_NAME}: max_age_s must be finite and nonnegative"
            ));
            return SidereonStatus::InvalidArgument;
        }
        store.inner = store
            .inner
            .clone()
            .with_vtec_staleness(StalenessPolicy::seconds(max_age_s));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_evaluate_vtec(
    store: *const SidereonSsrCorrectionStore,
    receiver_ecef_m: *const f64,
    satellite_transmit_ecef_m: *const f64,
    query_time: *const SidereonGnssWeekTow,
    frequency_hz: f64,
    out_result: *mut SidereonSsrVtecQueryResult,
    out_layers: *mut SidereonSsrVtecLayerEvaluation,
    layer_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_store_evaluate_vtec";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = SidereonSsrVtecQueryResult {
            kind: SidereonSsrVtecQueryKind::NoModel,
            age_s: 0.0,
            seconds_before_model: 0.0,
            max_age_s: 0.0,
            layer_count: 0,
            stec_tecu: 0.0,
            pseudorange_delay_m: 0.0,
            phase_range_advance_m: 0.0,
        };
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let receiver = c_try!(require_slice(
            receiver_ecef_m,
            3,
            FN_NAME,
            "receiver_ecef_m"
        ));
        let satellite = c_try!(require_slice(
            satellite_transmit_ecef_m,
            3,
            FN_NAME,
            "satellite_transmit_ecef_m"
        ));
        let query_time = c_try!(require_ref(query_time, FN_NAME, "query_time"));
        let query_time = c_try!(gnss_week_tow_from_c(FN_NAME, query_time));
        let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
        let satellite_transmit_ecef_m = [satellite[0], satellite[1], satellite[2]];
        let query = match store.inner.evaluate_vtec(
            receiver_ecef_m,
            satellite_transmit_ecef_m,
            query_time,
            frequency_hz,
        ) {
            Ok(query) => query,
            Err(error) => return map_ssr_error(FN_NAME, error),
        };
        let mut layers = Vec::new();
        match query {
            CoreSsrVtecQuery::NoModel => {}
            CoreSsrVtecQuery::BeforeModel {
                seconds_before_model,
            } => {
                (*out_result).kind = SidereonSsrVtecQueryKind::BeforeModel;
                (*out_result).seconds_before_model = seconds_before_model;
            }
            CoreSsrVtecQuery::Stale { age_s, max_age_s } => {
                (*out_result).kind = SidereonSsrVtecQueryKind::Stale;
                (*out_result).age_s = age_s;
                (*out_result).max_age_s = max_age_s;
            }
            CoreSsrVtecQuery::Evaluated { age_s, evaluation } => {
                (*out_result).kind = SidereonSsrVtecQueryKind::Evaluated;
                (*out_result).age_s = age_s;
                (*out_result).layer_count = evaluation.layers.len();
                (*out_result).stec_tecu = evaluation.stec_tecu;
                (*out_result).pseudorange_delay_m = evaluation.pseudorange_delay_m;
                (*out_result).phase_range_advance_m = evaluation.phase_range_advance_m;
                layers = evaluation
                    .layers
                    .iter()
                    .map(|layer| SidereonSsrVtecLayerEvaluation {
                        pierce_latitude_rad: layer.pierce_latitude_rad,
                        pierce_longitude_rad: layer.pierce_longitude_rad,
                        sun_fixed_longitude_rad: layer.sun_fixed_longitude_rad,
                        vtec_tecu: layer.vtec_tecu,
                        mapping_factor: layer.mapping_factor,
                        stec_tecu: layer.stec_tecu,
                    })
                    .collect();
            }
        }
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out_layers",
            &layers,
            out_layers,
            layer_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Build an SSR correction store from framed RTCM bytes, refusing anything it
/// cannot read and apply in full: every byte must belong to a CRC-valid frame
/// whose body decodes under the strict RTCM policy, and the store must ingest
/// every message. sidereon_ssr_store_from_rtcm_reading reads what it can and
/// reports the rest.
///
/// Safety: bytes points to len readable bytes; epoch points to a
/// SidereonGnssWeekTow; out_store points to a SidereonSsrCorrectionStore*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_from_rtcm(
    bytes: *const u8,
    len: usize,
    epoch: *const SidereonGnssWeekTow,
    out_store: *mut *mut SidereonSsrCorrectionStore,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_store_from_rtcm",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_store,
                "sidereon_ssr_store_from_rtcm",
                "out_store"
            ));
            *out = ptr::null_mut();
            let bytes = c_try!(require_slice(
                bytes,
                len,
                "sidereon_ssr_store_from_rtcm",
                "bytes"
            ));
            let epoch = c_try!(require_ref(epoch, "sidereon_ssr_store_from_rtcm", "epoch"));
            let epoch = c_try!(gnss_week_tow_from_c("sidereon_ssr_store_from_rtcm", epoch));
            let inner = c_try!(guard(
                "sidereon_ssr_store_from_rtcm",
                SidereonStatus::InvalidArgument,
                || { sidereon::ssr_store_from_rtcm_strict(bytes, epoch) }
            ));
            write_boxed_handle(out, SidereonSsrCorrectionStore { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_ingest_messages(
    store: *mut SidereonSsrCorrectionStore,
    messages: *const SidereonRtcmMessages,
    epoch: *const SidereonGnssWeekTow,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_store_ingest_messages",
        SidereonStatus::Panic,
        || {
            let store = c_try!(require_mut(
                store,
                "sidereon_ssr_store_ingest_messages",
                "store"
            ));
            let messages = c_try!(require_ref(
                messages,
                "sidereon_ssr_store_ingest_messages",
                "messages"
            ));
            let epoch = c_try!(require_ref(
                epoch,
                "sidereon_ssr_store_ingest_messages",
                "epoch"
            ));
            let epoch = c_try!(gnss_week_tow_from_c(
                "sidereon_ssr_store_ingest_messages",
                epoch
            ));
            for message in &messages.messages {
                if let Err(err) = store.inner.ingest(message, epoch) {
                    return map_ssr_error("sidereon_ssr_store_ingest_messages", err);
                }
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_orbit(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_orbit: *mut SidereonSsrOrbitCorrection,
) -> SidereonStatus {
    ssr_operation_boundary("sidereon_ssr_store_orbit", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_ssr_store_orbit",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_orbit,
            "sidereon_ssr_store_orbit",
            "out_orbit"
        ));
        *out = SidereonSsrOrbitCorrection {
            source: 0,
            provider_id: 0,
            solution_id: 0,
            has_nav_message: false,
            has_nav_message_index: 0,
            iode: 0,
            iod_ssr: 0,
            crs_regional: false,
            reference_point: SidereonSsrReferencePoint::CenterOfMass,
            radial_m: 0.0,
            along_m: 0.0,
            cross_m: 0.0,
            radial_rate_m_s: 0.0,
            along_rate_m_s: 0.0,
            cross_rate_m_s: 0.0,
            ref_epoch_j2000_s: 0.0,
            update_interval_s: 0.0,
        };
        let store = c_try!(require_ref(store, "sidereon_ssr_store_orbit", "store"));
        let sat = c_try!(parse_satellite_token("sidereon_ssr_store_orbit", sat_id));
        if let Some(value) = store.inner.orbit(sat) {
            *out_present = true;
            *out = ssr_orbit_to_c(value);
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_clock(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_clock: *mut SidereonSsrClockCorrection,
) -> SidereonStatus {
    ssr_operation_boundary("sidereon_ssr_store_clock", SidereonStatus::Panic, || {
        let out_present = c_try!(require_out(
            out_present,
            "sidereon_ssr_store_clock",
            "out_present"
        ));
        *out_present = false;
        let out = c_try!(require_out(
            out_clock,
            "sidereon_ssr_store_clock",
            "out_clock"
        ));
        *out = SidereonSsrClockCorrection {
            source: 0,
            provider_id: 0,
            solution_id: 0,
            has_nav_message: false,
            has_nav_message_index: 0,
            iod_ssr: 0,
            c0_m: 0.0,
            c1_m_s: 0.0,
            c2_m_s2: 0.0,
            ref_epoch_j2000_s: 0.0,
            update_interval_s: 0.0,
            has_high_rate: false,
            high_rate_c0_m: 0.0,
            high_rate_ref_epoch_j2000_s: 0.0,
            high_rate_update_interval_s: 0.0,
        };
        let store = c_try!(require_ref(store, "sidereon_ssr_store_clock", "store"));
        let sat = c_try!(parse_satellite_token("sidereon_ssr_store_clock", sat_id));
        if let Some(value) = store.inner.clock(sat) {
            *out_present = true;
            *out = ssr_clock_to_c(value);
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_ura_index(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    out_present: *mut bool,
    out_ura_index: *mut u8,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_store_ura_index",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_ura_index",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_ura_index,
                "sidereon_ssr_store_ura_index",
                "out_ura_index"
            ));
            *out = 0;
            let store = c_try!(require_ref(store, "sidereon_ssr_store_ura_index", "store"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_ura_index",
                sat_id
            ));
            if let Some(value) = store.inner.ura_index(sat) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

/// The latest code bias, metres, stored for satellite `sat_id` and the raw
/// signal index `signal` its source transmitted. `source` is 0 for RTCM SSR
/// (a signal and tracking mode identifier), 1 for Galileo HAS (HAS SIS ICD
/// Table 20), or 2 for IGS SSR (IGS SSR signal identifiers); an index the
/// source's table assigns to a physical signal is looked up as that signal, so
/// biases from different source tables for one physical signal share an entry.
/// This inspector ignores lifetime, staleness, do-not-use exclusion and phase
/// continuity.
///
/// Safety: store is a live handle; sat_id is a null-terminated token;
/// out_present points to a bool; out_bias_m points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_code_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    source: u32,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_store_code_bias_m",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_code_bias_m",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_bias_m,
                "sidereon_ssr_store_code_bias_m",
                "out_bias_m"
            ));
            *out = 0.0;
            let store = c_try!(require_ref(
                store,
                "sidereon_ssr_store_code_bias_m",
                "store"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_code_bias_m",
                sat_id
            ));
            let source = c_try!(ssr_source_from_c("sidereon_ssr_store_code_bias_m", source));
            let signal = sidereon_core::ssr::SsrRawSignal::new(source, sat.system, signal);
            if let Some(value) = store.inner.code_bias(sat, signal) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

/// The latest phase bias, metres, stored for satellite `sat_id` and the raw
/// signal index `signal` of `source`, as for sidereon_ssr_store_code_bias_m.
///
/// Safety: as for sidereon_ssr_store_code_bias_m.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_phase_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    source: u32,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_store_phase_bias_m",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_store_phase_bias_m",
                "out_present"
            ));
            *out_present = false;
            let out = c_try!(require_out(
                out_bias_m,
                "sidereon_ssr_store_phase_bias_m",
                "out_bias_m"
            ));
            *out = 0.0;
            let store = c_try!(require_ref(
                store,
                "sidereon_ssr_store_phase_bias_m",
                "store"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_store_phase_bias_m",
                sat_id
            ));
            let source = c_try!(ssr_source_from_c("sidereon_ssr_store_phase_bias_m", source));
            let signal = sidereon_core::ssr::SsrRawSignal::new(source, sat.system, signal);
            if let Some(value) = store.inner.phase_bias(sat, signal) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_corrected_state(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    t_j2000_s: f64,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    out_present: *mut bool,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_corrected_state",
        SidereonStatus::Panic,
        || {
            if !out_present.is_null() && !out_position_ecef_m.is_null() && !out_clock_s.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_ssr_corrected_state",
                            out_present,
                            1,
                            "out_present"
                        )),
                        "out_present",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_ssr_corrected_state",
                            out_position_ecef_m,
                            3,
                            "out_position_ecef_m"
                        )),
                        "out_position_ecef_m",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_ssr_corrected_state",
                            out_clock_s,
                            1,
                            "out_clock_s"
                        )),
                        "out_clock_s",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_ssr_corrected_state",
                    &outputs
                ));
            }
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ssr_corrected_state",
                "out_present"
            ));
            *out_present = false;
            c_try!(require_out(
                out_position_ecef_m,
                "sidereon_ssr_corrected_state",
                "out_position_ecef_m"
            ));
            zero_f64_prefix(out_position_ecef_m, 3, 3);
            let out_clock = c_try!(require_out(
                out_clock_s,
                "sidereon_ssr_corrected_state",
                "out_clock_s"
            ));
            *out_clock = 0.0;
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_corrected_state",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_corrected_state", "store"));
            let sat = c_try!(parse_satellite_token(
                "sidereon_ssr_corrected_state",
                sat_id
            ));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_corrected_state",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            if let Some((position, clock)) = corrected.corrected_state(sat, t_j2000_s) {
                c_try!(copy_exact_f64s(
                    "sidereon_ssr_corrected_state",
                    "out_position_ecef_m",
                    out_position_ecef_m,
                    3,
                    &position
                ));
                *out_present = true;
                *out_clock = clock;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_corrected_state_at_epoch_queries(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    out_result: *mut SidereonSsrCorrectedStateResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_corrected_state_at_epoch_queries";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        record_degrade_reason(None);
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = SidereonSsrCorrectedStateResult {
            has_state: false,
            position_ecef_m: [0.0; 3],
            clock_s: 0.0,
            has_group_delay: false,
            group_delay_s: 0.0,
            degraded: false,
            has_size_event: false,
            strict_refusal: false,
            size: SidereonSsrCorrectionSize {
                orbit_m: 0.0,
                clock_m: 0.0,
            },
            has_oversized_report: false,
            source: 0,
            provider_id: 0,
            solution_id: 0,
            orbit_ref_epoch_j2000_s: 0.0,
            clock_ref_epoch_j2000_s: 0.0,
            first_applied_epoch_j2000_s: 0.0,
        };
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let size_policy = match size_policy {
            value if value == SidereonSsrCorrectionSizePolicy::Strict as u32 => {
                sidereon_core::ssr::SsrCorrectionSizePolicy::Strict
            }
            value if value == SidereonSsrCorrectionSizePolicy::Lenient as u32 => {
                sidereon_core::ssr::SsrCorrectionSizePolicy::Lenient
            }
            other => {
                set_last_error(format!("{FN_NAME}: unknown size_policy {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        let refusal = corrected.correction_size_refusal_at_epoch_query(
            sat,
            &state_epoch.inner,
            &selection_epoch.inner,
        );
        let state = c_try!(guard_core(
            || corrected.corrected_state_with_group_delay_checked_selected_query(
                sat,
                &state_epoch.inner,
                &selection_epoch.inner,
            ),
            |error| map_ssr_error(FN_NAME, error),
        ));
        record_degrade_reason(state.degraded);
        (*out_result).degraded = state.degraded.is_some();
        if let Some((position, clock, group_delay)) = state.value {
            (*out_result).has_state = true;
            (*out_result).position_ecef_m = position;
            (*out_result).clock_s = clock;
            (*out_result).has_group_delay = group_delay.is_some();
            (*out_result).group_delay_s = group_delay.unwrap_or_default();
        }
        let applied_report = corrected
            .oversized_corrections()
            .into_iter()
            .find(|report| report.sat == sat);
        if let Some(report) = applied_report {
            (*out_result).has_oversized_report = true;
            (*out_result).source = match report.solution.source {
                sidereon_core::ssr::SsrSource::RtcmSsr => 0,
                sidereon_core::ssr::SsrSource::GalileoHas => 1,
                sidereon_core::ssr::SsrSource::IgsSsr => 2,
            };
            (*out_result).provider_id = report.solution.provider_id;
            (*out_result).solution_id = report.solution.solution_id;
            (*out_result).orbit_ref_epoch_j2000_s = report.orbit_ref_epoch_j2000_s;
            (*out_result).clock_ref_epoch_j2000_s = report.clock_ref_epoch_j2000_s;
            (*out_result).first_applied_epoch_j2000_s = report.t_j2000_s;
        }
        let had_refusal = refusal.is_some();
        let size = refusal.or_else(|| applied_report.map(|report| report.size));
        if let Some(size) = size {
            (*out_result).has_size_event = true;
            (*out_result).strict_refusal =
                had_refusal && size_policy == sidereon_core::ssr::SsrCorrectionSizePolicy::Strict;
            (*out_result).size = SidereonSsrCorrectionSize {
                orbit_m: size.orbit_m,
                clock_m: size.clock_m,
            };
        }
        SidereonStatus::Ok
    })
}

/// Query clock at exact transmit and selection epochs while returning typed
/// policy diagnostics.
///
/// Safety: non-null output pointers must each point to writable storage of the
/// documented type, and all output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_transmit_epoch_clock_at_epoch_queries(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    transmit_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    out_has_clock: *mut bool,
    out_clock_s: *mut f64,
    out_degraded: *mut bool,
    out_policy_result: *mut SidereonSsrCorrectedStateResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_transmit_epoch_clock_at_epoch_queries";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_has_clock.is_null()
            && !out_clock_s.is_null()
            && !out_degraded.is_null()
            && !out_policy_result.is_null()
        {
            let outputs = [
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_has_clock,
                        1,
                        "out_has_clock"
                    )),
                    "out_has_clock",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_clock_s,
                        1,
                        "out_clock_s"
                    )),
                    "out_clock_s",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_degraded,
                        1,
                        "out_degraded"
                    )),
                    "out_degraded",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_policy_result,
                        1,
                        "out_policy_result"
                    )),
                    "out_policy_result",
                )),
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                FN_NAME, &outputs
            ));
        }
        let out_has_clock = c_try!(require_out(out_has_clock, FN_NAME, "out_has_clock"));
        let out_clock_s = c_try!(require_out(out_clock_s, FN_NAME, "out_clock_s"));
        let out_degraded = c_try!(require_out(out_degraded, FN_NAME, "out_degraded"));
        let out_policy_result =
            c_try!(require_out(out_policy_result, FN_NAME, "out_policy_result"));
        *out_has_clock = false;
        *out_clock_s = 0.0;
        *out_degraded = false;
        record_degrade_reason(None);
        *out_policy_result = empty_ssr_corrected_state_result();
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let transmit_epoch = c_try!(require_ref(transmit_epoch, FN_NAME, "transmit_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let size_policy = c_try!(ssr_size_policy_from_c(FN_NAME, size_policy));
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        let clock = c_try!(guard_core(
            || sidereon_core::positioning::EphemerisSource::try_transmit_epoch_clock_at_epoch_query(
                &corrected,
                satellite,
                &transmit_epoch.inner,
                &selection_epoch.inner,
            ),
            |error| map_ssr_error(FN_NAME, error),
        ));
        *out_policy_result = ssr_policy_report(
            &corrected,
            satellite,
            &transmit_epoch.inner,
            &selection_epoch.inner,
            size_policy,
        );
        if let Some(clock) = clock {
            *out_has_clock = true;
            *out_clock_s = clock.value;
            record_degrade_reason(clock.degraded);
            *out_degraded = clock.degraded.is_some();
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_clock_relativity_at_epoch_query(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    epoch: *const SidereonExactEpochQuery,
    position_ecef_m: *const f64,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    out_kind: *mut SidereonClockRelativityKind,
    out_term_s: *mut f64,
    out_policy_result: *mut SidereonSsrCorrectedStateResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_clock_relativity_at_epoch_query";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_kind.is_null() && !out_term_s.is_null() && !out_policy_result.is_null() {
            let outputs = [
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME, out_kind, 1, "out_kind"
                    )),
                    "out_kind",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_term_s,
                        1,
                        "out_term_s"
                    )),
                    "out_term_s",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_policy_result,
                        1,
                        "out_policy_result"
                    )),
                    "out_policy_result",
                )),
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                FN_NAME, &outputs
            ));
        }
        let out_kind = c_try!(require_out(out_kind, FN_NAME, "out_kind"));
        let out_term_s = c_try!(require_out(out_term_s, FN_NAME, "out_term_s"));
        let out_policy_result =
            c_try!(require_out(out_policy_result, FN_NAME, "out_policy_result"));
        *out_kind = SidereonClockRelativityKind::NotApplicable;
        *out_term_s = 0.0;
        *out_policy_result = empty_ssr_corrected_state_result();
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let epoch = c_try!(require_ref(epoch, FN_NAME, "epoch"));
        let position = c_try!(require_slice(
            position_ecef_m,
            3,
            FN_NAME,
            "position_ecef_m"
        ));
        let position = [position[0], position[1], position[2]];
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let size_policy = c_try!(ssr_size_policy_from_c(FN_NAME, size_policy));
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        *out_policy_result = ssr_policy_report(
            &corrected,
            satellite,
            &epoch.inner,
            &epoch.inner,
            size_policy,
        );
        match sidereon_core::positioning::EphemerisSource::clock_relativity_for_state_at_epoch_query(
            &corrected,
            satellite,
            &epoch.inner,
            position,
        ) {
            sidereon_core::positioning::ClockRelativity::NotApplicable => {}
            sidereon_core::positioning::ClockRelativity::Term(term) => {
                *out_kind = SidereonClockRelativityKind::Term;
                *out_term_s = term;
            }
            sidereon_core::positioning::ClockRelativity::Unavailable => {
                *out_kind = SidereonClockRelativityKind::Unavailable;
            }
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ephemeris_variance_at_epoch_queries(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    out_variance_m2: *mut f64,
    out_policy_result: *mut SidereonSsrCorrectedStateResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_ephemeris_variance_at_epoch_queries";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_variance_m2 = c_try!(require_out(out_variance_m2, FN_NAME, "out_variance_m2"));
        let out_policy_result =
            c_try!(require_out(out_policy_result, FN_NAME, "out_policy_result"));
        *out_variance_m2 = 0.0;
        *out_policy_result = empty_ssr_corrected_state_result();
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let size_policy = c_try!(ssr_size_policy_from_c(FN_NAME, size_policy));
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        *out_policy_result = ssr_policy_report(
            &corrected,
            satellite,
            &state_epoch.inner,
            &selection_epoch.inner,
            size_policy,
        );
        *out_variance_m2 =
            sidereon_core::positioning::EphemerisSource::ephemeris_variance_at_epoch_query(
                &corrected,
                satellite,
                &state_epoch.inner,
                &selection_epoch.inner,
            );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_solve_broadcast(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    inputs: *const SidereonSppInputs,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_solve_broadcast",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_ssr_solve_broadcast",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_solve_broadcast",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_solve_broadcast", "store"));
            let inputs = c_try!(require_ref(
                inputs,
                "sidereon_ssr_solve_broadcast",
                "inputs"
            ));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_solve_broadcast",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            let solve_inputs = c_try!(build_spp_solve_inputs(
                "sidereon_ssr_solve_broadcast",
                inputs,
                None,
                None,
                BTreeMap::new(),
            ));
            let inner = c_try!(guard(
                "sidereon_ssr_solve_broadcast",
                SidereonStatus::Solve,
                || {
                    sidereon::solve_spp(
                        &corrected,
                        &solve_inputs,
                        inputs.with_geodetic,
                        SolvePolicy::default(),
                    )
                }
            ));
            write_boxed_handle(out_solution, SidereonSppSolution { inner });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_solve_broadcast_at_exact_epoch(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    inputs: *const SidereonSppInputs,
    receive_epoch: *const SidereonExactEpoch,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_solve_broadcast_at_exact_epoch";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(out_solution, FN_NAME, "out_solution"));
        *out_solution = ptr::null_mut();
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let inputs = c_try!(require_ref(inputs, FN_NAME, "inputs"));
        let receive_epoch = c_try!(require_ref(receive_epoch, FN_NAME, "receive_epoch"));
        let size_policy = match size_policy {
            value if value == SidereonSsrCorrectionSizePolicy::Strict as u32 => {
                sidereon_core::ssr::SsrCorrectionSizePolicy::Strict
            }
            value if value == SidereonSsrCorrectionSizePolicy::Lenient as u32 => {
                sidereon_core::ssr::SsrCorrectionSizePolicy::Lenient
            }
            other => {
                set_last_error(format!("{FN_NAME}: unknown size_policy {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        let solve_inputs = c_try!(build_spp_solve_inputs(
            FN_NAME,
            inputs,
            None,
            None,
            BTreeMap::new(),
        ));
        let exact_inputs = sidereon_core::positioning::ExactSolveInputs {
            inputs: solve_inputs,
            receive_epoch: receive_epoch.inner,
        };
        let inner = c_try!(guard_result(FN_NAME, SidereonStatus::Solve, || {
            sidereon_core::positioning::solve_with_exact_epoch(
                &corrected,
                &exact_inputs,
                inputs.with_geodetic,
            )
        }));
        write_boxed_handle(out_solution, SidereonSppSolution { inner });
        SidereonStatus::Ok
    })
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_ssr_solve_broadcast_v2_with_models_at_exact_epoch(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    size_policy: u32,
    inputs: *const SidereonSppInputsV2,
    models: *const SidereonSppModelOptions,
    receive_epoch: *const SidereonExactEpoch,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_ssr_solve_broadcast_v2_with_models_at_exact_epoch";
    ssr_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_solution = c_try!(require_out(out_solution, FN_NAME, "out_solution"));
        *out_solution = ptr::null_mut();
        let broadcast = c_try!(require_ref(broadcast, FN_NAME, "broadcast"));
        let store = c_try!(require_ref(store, FN_NAME, "store"));
        let inputs = c_try!(require_ref(inputs, FN_NAME, "inputs"));
        let receive_epoch = c_try!(require_ref(receive_epoch, FN_NAME, "receive_epoch"));
        let size_policy = c_try!(ssr_size_policy_from_c(FN_NAME, size_policy));
        let fallback = c_try!(ssr_fallback_from_c(
            FN_NAME,
            missing_action,
            allow_regional_provider,
            regional_provider_id,
        ));
        let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
            .with_staleness(StalenessPolicy::seconds(staleness_s))
            .with_fallback(fallback)
            .with_correction_size_policy(size_policy);
        let mut solve_inputs = c_try!(crate::spp::build_spp_solve_inputs_with_models(
            FN_NAME, inputs, models
        ));
        solve_inputs.t_rx_j2000_s = receive_epoch.inner.j2000_seconds();
        let exact_inputs = sidereon_core::positioning::ExactSolveInputs {
            inputs: solve_inputs,
            receive_epoch: receive_epoch.inner,
        };
        let policy = c_try!(solve_policy_from_c(FN_NAME, &inputs.policy));
        let solution = c_try!(guard_result(FN_NAME, SidereonStatus::Solve, || {
            sidereon_core::positioning::solve_with_exact_epoch_and_policy(
                &corrected,
                &exact_inputs,
                inputs.base.with_geodetic,
                policy,
            )
        }));
        write_boxed_handle(out_solution, SidereonSppSolution { inner: solution });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ephemeris_sample(
    broadcast: *const SidereonBroadcastEphemeris,
    store: *const SidereonSsrCorrectionStore,
    staleness_s: f64,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
    satellites: *const *const c_char,
    satellite_count: usize,
    start_j2000_s: f64,
    stop_j2000_s: f64,
    step_s: f64,
    out: *mut SidereonEphemerisSampleRow,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ssr_operation_boundary(
        "sidereon_ssr_ephemeris_sample",
        SidereonStatus::Panic,
        || {
            let broadcast = c_try!(require_ref(
                broadcast,
                "sidereon_ssr_ephemeris_sample",
                "broadcast"
            ));
            let store = c_try!(require_ref(store, "sidereon_ssr_ephemeris_sample", "store"));
            let fallback = c_try!(ssr_fallback_from_c(
                "sidereon_ssr_ephemeris_sample",
                missing_action,
                allow_regional_provider,
                regional_provider_id,
            ));
            let corrected = SsrCorrectedEphemeris::new(&broadcast.inner, &store.inner)
                .with_staleness(StalenessPolicy::seconds(staleness_s))
                .with_fallback(fallback);
            ephemeris_sample_common(
                "sidereon_ssr_ephemeris_sample",
                &corrected,
                satellites,
                satellite_count,
                start_j2000_s,
                stop_j2000_s,
                step_s,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_free(store: *mut SidereonSsrCorrectionStore) {
    free_boxed(store);
}

// ============================================================================
// Newly merged core features: full NeQuick-G slant integration, the standalone
// range RAIM/FDE design, the RTK and PPP arc drivers, and RTCM 3 from-scratch
// message construction. Every function below marshals C input into the cited
// sidereon-core type, calls the engine entry point, and copies the result back.
// No modeling lives here.

fn ssr_reference_point_from_c(
    fn_name: &str,
    value: u32,
) -> Result<OrbitReferencePoint, SidereonStatus> {
    match value {
        v if v == SidereonSsrReferencePoint::AntennaPhaseCenter as u32 => {
            Ok(OrbitReferencePoint::AntennaPhaseCenter)
        }
        v if v == SidereonSsrReferencePoint::CenterOfMass as u32 => {
            Ok(OrbitReferencePoint::CenterOfMass)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid SSR reference point"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ssr_fallback_from_c(
    fn_name: &str,
    missing_action: u32,
    allow_regional_provider: bool,
    regional_provider_id: u16,
) -> Result<SsrFallbackPolicy, SidereonStatus> {
    let regional = if allow_regional_provider {
        let mut providers = BTreeSet::new();
        providers.insert(regional_provider_id);
        RegionalPolicy::AllowProviders(providers)
    } else {
        RegionalPolicy::DeclineRegional
    };
    Ok(SsrFallbackPolicy {
        on_missing_correction: ssr_missing_action_from_c(fn_name, missing_action)?,
        regional,
    })
}

fn empty_ssr_corrected_state_result() -> SidereonSsrCorrectedStateResult {
    SidereonSsrCorrectedStateResult {
        has_state: false,
        position_ecef_m: [0.0; 3],
        clock_s: 0.0,
        has_group_delay: false,
        group_delay_s: 0.0,
        degraded: false,
        has_size_event: false,
        strict_refusal: false,
        size: SidereonSsrCorrectionSize {
            orbit_m: 0.0,
            clock_m: 0.0,
        },
        has_oversized_report: false,
        source: 0,
        provider_id: 0,
        solution_id: 0,
        orbit_ref_epoch_j2000_s: 0.0,
        clock_ref_epoch_j2000_s: 0.0,
        first_applied_epoch_j2000_s: 0.0,
    }
}

fn ssr_policy_report(
    corrected: &SsrCorrectedEphemeris<'_>,
    satellite: GnssSatelliteId,
    state_epoch: &sidereon_core::astro::time::ExactEpochQuery,
    selection_epoch: &sidereon_core::astro::time::ExactEpochQuery,
    size_policy: sidereon_core::ssr::SsrCorrectionSizePolicy,
) -> SidereonSsrCorrectedStateResult {
    let mut result = empty_ssr_corrected_state_result();
    let refusal =
        corrected.correction_size_refusal_at_epoch_query(satellite, state_epoch, selection_epoch);
    let applied_report = corrected
        .oversized_corrections()
        .into_iter()
        .find(|report| report.sat == satellite);
    if let Some(report) = applied_report {
        result.has_oversized_report = true;
        result.source = match report.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
            sidereon_core::ssr::SsrSource::IgsSsr => 2,
        };
        result.provider_id = report.solution.provider_id;
        result.solution_id = report.solution.solution_id;
        result.orbit_ref_epoch_j2000_s = report.orbit_ref_epoch_j2000_s;
        result.clock_ref_epoch_j2000_s = report.clock_ref_epoch_j2000_s;
        result.first_applied_epoch_j2000_s = report.t_j2000_s;
    }
    if let Some(size) = refusal.or_else(|| applied_report.map(|report| report.size)) {
        result.has_size_event = true;
        result.strict_refusal =
            refusal.is_some() && size_policy == sidereon_core::ssr::SsrCorrectionSizePolicy::Strict;
        result.size = SidereonSsrCorrectionSize {
            orbit_m: size.orbit_m,
            clock_m: size.clock_m,
        };
    }
    result
}

fn ssr_size_policy_from_c(
    fn_name: &str,
    size_policy: u32,
) -> Result<sidereon_core::ssr::SsrCorrectionSizePolicy, SidereonStatus> {
    match size_policy {
        value if value == SidereonSsrCorrectionSizePolicy::Strict as u32 => {
            Ok(sidereon_core::ssr::SsrCorrectionSizePolicy::Strict)
        }
        value if value == SidereonSsrCorrectionSizePolicy::Lenient as u32 => {
            Ok(sidereon_core::ssr::SsrCorrectionSizePolicy::Lenient)
        }
        other => {
            set_last_error(format!("{fn_name}: unknown size_policy {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ssr_orbit_to_c(value: &SsrOrbitCorrection) -> SidereonSsrOrbitCorrection {
    SidereonSsrOrbitCorrection {
        source: match value.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
            sidereon_core::ssr::SsrSource::IgsSsr => 2,
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
        has_nav_message: !matches!(
            value.nav_message,
            sidereon_core::ssr::SsrNavigationMessage::Rtcm
        ),
        has_nav_message_index: match value.nav_message {
            sidereon_core::ssr::SsrNavigationMessage::Has(index) => index,
            sidereon_core::ssr::SsrNavigationMessage::Rtcm
            | sidereon_core::ssr::SsrNavigationMessage::IgsSsr => 0,
        },
        iode: value.iode,
        iod_ssr: value.iod_ssr,
        crs_regional: value.crs_regional,
        reference_point: ssr_reference_point_to_c(value.reference_point),
        radial_m: value.radial_m,
        along_m: value.along_m,
        cross_m: value.cross_m,
        radial_rate_m_s: value.radial_rate_m_s,
        along_rate_m_s: value.along_rate_m_s,
        cross_rate_m_s: value.cross_rate_m_s,
        ref_epoch_j2000_s: value.ref_epoch_j2000_s,
        update_interval_s: value.update_interval_s,
    }
}

fn ssr_clock_to_c(value: &SsrClockCorrection) -> SidereonSsrClockCorrection {
    SidereonSsrClockCorrection {
        source: match value.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
            sidereon_core::ssr::SsrSource::IgsSsr => 2,
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
        has_nav_message: !matches!(
            value.nav_message,
            sidereon_core::ssr::SsrNavigationMessage::Rtcm
        ),
        has_nav_message_index: match value.nav_message {
            sidereon_core::ssr::SsrNavigationMessage::Has(index) => index,
            sidereon_core::ssr::SsrNavigationMessage::Rtcm
            | sidereon_core::ssr::SsrNavigationMessage::IgsSsr => 0,
        },
        iod_ssr: value.iod_ssr,
        c0_m: value.c0_m,
        c1_m_s: value.c1_m_s,
        c2_m_s2: value.c2_m_s2,
        ref_epoch_j2000_s: value.ref_epoch_j2000_s,
        update_interval_s: value.update_interval_s,
        has_high_rate: value.high_rate.is_some(),
        high_rate_c0_m: value.high_rate.map(|h| h.c0_m).unwrap_or(0.0),
        high_rate_ref_epoch_j2000_s: value.high_rate.map(|h| h.ref_epoch_j2000_s).unwrap_or(0.0),
        high_rate_update_interval_s: value.high_rate.map(|h| h.update_interval_s).unwrap_or(0.0),
    }
}

fn map_ssr_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    crate::rtcm::record_rtcm_typed_error(&err);
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        // The typed encoder, conversion and SBAS encoder refusals refuse the
        // caller's input, as the InvalidInput text they replace did; their
        // detail is in sidereon_rtcm_last_error_info and its payload.
        CoreError::InvalidInput(_)
        | CoreError::Parse(_)
        | CoreError::RtcmEncode(_)
        | CoreError::RtcmConversion(_)
        | CoreError::SbasEncode(_) => SidereonStatus::InvalidArgument,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
    }
}

fn ssr_reference_point_to_c(value: OrbitReferencePoint) -> SidereonSsrReferencePoint {
    match value {
        OrbitReferencePoint::AntennaPhaseCenter => SidereonSsrReferencePoint::AntennaPhaseCenter,
        OrbitReferencePoint::CenterOfMass => SidereonSsrReferencePoint::CenterOfMass,
    }
}

fn ssr_missing_action_from_c(
    fn_name: &str,
    value: u32,
) -> Result<MissingCorrectionAction, SidereonStatus> {
    match value {
        v if v == SidereonSsrMissingCorrectionAction::Decline as u32 => {
            Ok(MissingCorrectionAction::Decline)
        }
        v if v == SidereonSsrMissingCorrectionAction::FallBackToBroadcast as u32 => {
            Ok(MissingCorrectionAction::FallBackToBroadcast)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid SSR missing-correction action"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ssr_source_from_c(
    fn_name: &str,
    source: u32,
) -> Result<sidereon_core::ssr::SsrSource, SidereonStatus> {
    match source {
        0 => Ok(sidereon_core::ssr::SsrSource::RtcmSsr),
        1 => Ok(sidereon_core::ssr::SsrSource::GalileoHas),
        2 => Ok(sidereon_core::ssr::SsrSource::IgsSsr),
        other => {
            set_last_error(format!("{fn_name}: unknown SSR source {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// The decoded RTCM messages an SSR store refused to ingest, each with its
/// message number and the refusal. Release with
/// sidereon_ssr_ingest_refusals_free.
pub struct SidereonSsrIngestRefusals {
    pub(crate) refusals: Vec<sidereon::SsrIngestRefusal>,
}

/// Build an SSR correction store from every readable frame of framed RTCM
/// bytes under the lenient RTCM policy, reporting what was not read or not
/// applied instead of failing. Writes newly owned handles to *out_store,
/// *out_diagnostics (bytes passed over while resynchronizing, CRC-24Q
/// failures, frames that did not decode, departures read) and *out_refusals
/// (messages that decoded but that the store refused), and the length of a
/// trailing partial frame to *out_trailing_partial_frame_len.
/// sidereon_ssr_store_from_rtcm refuses all of these instead.
///
/// Safety: bytes points to len readable bytes; epoch points to a
/// SidereonGnssWeekTow; each out pointer points to storage of its type.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_from_rtcm_reading(
    bytes: *const u8,
    len: usize,
    epoch: *const SidereonGnssWeekTow,
    out_store: *mut *mut SidereonSsrCorrectionStore,
    out_diagnostics: *mut *mut SidereonRtcmStreamDiagnostics,
    out_trailing_partial_frame_len: *mut usize,
    out_refusals: *mut *mut SidereonSsrIngestRefusals,
) -> SidereonStatus {
    let fn_name = "sidereon_ssr_store_from_rtcm_reading";
    ssr_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out_store = c_try!(require_out(out_store, fn_name, "out_store"));
        *out_store = ptr::null_mut();
        let out_diagnostics = c_try!(require_out(out_diagnostics, fn_name, "out_diagnostics"));
        *out_diagnostics = ptr::null_mut();
        let out_trailing = c_try!(require_out(
            out_trailing_partial_frame_len,
            fn_name,
            "out_trailing_partial_frame_len"
        ));
        *out_trailing = 0;
        let out_refusals = c_try!(require_out(out_refusals, fn_name, "out_refusals"));
        *out_refusals = ptr::null_mut();
        let bytes = c_try!(require_slice(bytes, len, fn_name, "bytes"));
        let epoch = c_try!(require_ref(epoch, fn_name, "epoch"));
        let epoch = c_try!(gnss_week_tow_from_c(fn_name, epoch));
        let ingest = sidereon::ssr_store_from_rtcm(bytes, epoch);
        *out_trailing = ingest.trailing_partial_frame_len;
        write_boxed_handle(
            out_store,
            SidereonSsrCorrectionStore {
                inner: ingest.store,
            },
        );
        write_boxed_handle(
            out_diagnostics,
            SidereonRtcmStreamDiagnostics {
                diagnostics: ingest.diagnostics,
            },
        );
        write_boxed_handle(
            out_refusals,
            SidereonSsrIngestRefusals {
                refusals: ingest.ingest_refusals,
            },
        );
        SidereonStatus::Ok
    })
}

/// Write the number of refused messages.
///
/// Safety: refusals is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ingest_refusals_count(
    refusals: *const SidereonSsrIngestRefusals,
    out_count: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ssr_ingest_refusals_count";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, fn_name, "out_count"));
        *out = 0;
        let refusals = c_try!(require_ref(refusals, fn_name, "refusals"));
        *out = refusals.refusals.len();
        SidereonStatus::Ok
    })
}

/// Copy refusal `index`: its RTCM message number to *out_message_number and
/// the refusal text (not null-terminated) into out under the variable-length
/// output contract.
///
/// Safety: refusals is a live handle; out_message_number points to a
/// uint16_t; out points to len writable bytes or is NULL when len is 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ingest_refusal(
    refusals: *const SidereonSsrIngestRefusals,
    index: usize,
    out_message_number: *mut u16,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ssr_ingest_refusal";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let out_message_number = c_try!(require_out(
            out_message_number,
            fn_name,
            "out_message_number"
        ));
        *out_message_number = 0;
        let refusals = c_try!(require_ref(refusals, fn_name, "refusals"));
        let Some(refusal) = refusals.refusals.get(index) else {
            set_last_error(format!(
                "{fn_name}: index {index} out of range ({} refusals)",
                refusals.refusals.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_message_number = refusal.message_number;
        let text = refusal.error.to_string();
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

/// Release a refusal list. Passing NULL is a no-op.
///
/// Safety: refusals is NULL or a live handle not yet freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_ingest_refusals_free(
    refusals: *mut SidereonSsrIngestRefusals,
) {
    free_boxed(refusals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_ssr_failure_outputs_and_nested_counts_are_deterministic() {
        let body = [
            0x42, 0x35, 0x46, 0x00, 0x2c, 0x80, 0x3d, 0xa0, 0x21, 0x88, 0x3d, 0x97, 0x24, 0x92,
            0x90,
        ];
        let core = RtcmSsrMessage::decode(&body).expect("sidereon-core decodes the body");
        let mut handle = ptr::null_mut();
        let invalid = [0u8];
        // sidereon-core refuses the one-byte body; map_ssr_message_decode_error
        // reports that refusal as SP3_PARSE.
        assert!(RtcmSsrMessage::decode(&invalid).is_err());
        assert_eq!(
            unsafe { sidereon_ssr_message_decode(invalid.as_ptr(), invalid.len(), &mut handle) },
            SidereonStatus::Sp3Parse
        );
        assert!(handle.is_null());
        assert_eq!(
            unsafe { sidereon_ssr_message_decode(ptr::null(), 1, &mut handle) },
            SidereonStatus::NullPointer
        );
        assert!(handle.is_null());
        assert_eq!(
            unsafe { sidereon_ssr_message_decode(body.as_ptr(), body.len(), &mut handle) },
            SidereonStatus::Ok
        );
        let handle = handle;

        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_ssr_message_code_bias_signals(
                    handle,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!((written, required), (0, core.code_bias[0].biases.len()));

        written = usize::MAX;
        required = usize::MAX;
        let mut rows: [SidereonRtcmSsrCodeBiasSignal; 2] = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe {
                sidereon_ssr_message_code_bias_signals(
                    handle,
                    core.code_bias.len(),
                    rows.as_mut_ptr(),
                    rows.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));
        assert_eq!(
            unsafe { sidereon_ssr_message_info(handle, ptr::null_mut()) },
            SidereonStatus::NullPointer
        );
        unsafe { sidereon_ssr_message_free(handle) };
    }

    #[test]
    fn test_ssr_public_control_and_early_null_checks() {
        use crate::engine_error::{
            sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
            SidereonEngineErrorFamily, SidereonEngineErrorInfo,
        };

        unsafe fn trigger_refusal() {
            let malformed = b"INVALID SP3";
            let mut refused = ptr::null_mut();
            let status =
                crate::sp3::sidereon_sp3_load(malformed.as_ptr(), malformed.len(), &mut refused);
            assert_eq!(status, SidereonStatus::Sp3Parse);
            assert!(refused.is_null());
        }

        // 1. Seed through a real public refusal
        unsafe { trigger_refusal() };
        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Facade);
        assert!(info.payload_len > 0);

        // 2. Early null out_solution clears TLS (8 arguments)
        let status = unsafe {
            sidereon_ssr_solve_broadcast(
                ptr::null(),
                ptr::null(),
                0.0,
                0,
                false,
                0,
                ptr::null(),
                ptr::null_mut(),
            )
        };
        assert_eq!(status, SidereonStatus::NullPointer);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 3. Re-seed refusal before testing successful producer reset
        unsafe { trigger_refusal() };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Facade);
        assert!(info.payload_len > 0);

        // 4. Successful producer resets TLS
        let body = [
            0x42, 0x35, 0x46, 0x00, 0x2c, 0x80, 0x3d, 0xa0, 0x21, 0x88, 0x3d, 0x97, 0x24, 0x92,
            0x90,
        ];
        let mut live_handle = ptr::null_mut();
        let status =
            unsafe { sidereon_ssr_message_decode(body.as_ptr(), body.len(), &mut live_handle) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!live_handle.is_null());

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        // 5. While holding live_handle, pre-seed refusal and take full snapshot
        unsafe { trigger_refusal() };
        let mut seed_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut seed_info) },
            SidereonStatus::Ok
        );
        assert_eq!(seed_info.family, SidereonEngineErrorFamily::Facade);
        assert!(seed_info.payload_len > 0);
        let mut seed_payload = vec![0u8; seed_info.payload_len];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    seed_payload.as_mut_ptr(),
                    seed_payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, seed_info.payload_len);

        let verify_seed_tls = || {
            let mut cur_info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                unsafe { sidereon_last_engine_error_info(&mut cur_info) },
                SidereonStatus::Ok
            );
            assert_eq!(cur_info.family, SidereonEngineErrorFamily::Facade);
            assert_eq!(cur_info.payload_len, seed_info.payload_len);
            let mut cur_payload = vec![0u8; cur_info.payload_len];
            let mut w = 0;
            let mut r = 0;
            assert_eq!(
                unsafe {
                    sidereon_last_engine_error_payload(
                        cur_payload.as_mut_ptr(),
                        cur_payload.len(),
                        &mut w,
                        &mut r,
                    )
                },
                SidereonStatus::Ok
            );
            assert_eq!(cur_payload, seed_payload);
        };

        // 6. Live reader on live_handle retains complete generic TLS
        let mut msg_info: SidereonRtcmSsrInfo = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { sidereon_ssr_message_info(live_handle, &mut msg_info) },
            SidereonStatus::Ok
        );
        verify_seed_tls();

        // 7. Free retains complete generic TLS
        unsafe { sidereon_ssr_message_free(live_handle) };
        verify_seed_tls();

        // 8. Actual unrelated successful producer resets TLS
        let mut handle2 = ptr::null_mut();
        let status =
            unsafe { sidereon_ssr_message_decode(body.as_ptr(), body.len(), &mut handle2) };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!handle2.is_null());
        unsafe { sidereon_ssr_message_free(handle2) };

        let mut reset_info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::Facade,
            payload_len: 999,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut reset_info) },
            SidereonStatus::Ok
        );
        assert_eq!(reset_info.family, SidereonEngineErrorFamily::None);
        assert_eq!(reset_info.payload_len, 0);
    }
}
