use super::*;

// --- SSR decode accessors, correction store, and corrected broadcast source --

pub struct SidereonSsrCorrectionStore {
    pub(crate) inner: SsrCorrectionStore,
}

/// An opaque decoded RTCM SSR message body. The handle owns one
/// `sidereon_core::rtcm::SsrMessage` and is released with
/// sidereon_ssr_message_free.
pub struct SidereonSsrMessage {
    pub(crate) inner: RtcmSsrMessage,
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
        RtcmSsrKind::Vtec => SidereonRtcmSsrKind::Vtec,
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
        _ => SidereonStatus::Solve,
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
    ffi_boundary("sidereon_ssr_message_decode", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_ssr_store_new", SidereonStatus::Panic, || {
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
pub unsafe extern "C" fn sidereon_ssr_store_from_rtcm(
    bytes: *const u8,
    len: usize,
    epoch: *const SidereonGnssWeekTow,
    out_store: *mut *mut SidereonSsrCorrectionStore,
) -> SidereonStatus {
    ffi_boundary(
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
            let inner = c_try!(guard(SidereonStatus::InvalidArgument, || {
                sidereon::ssr_store_from_rtcm(bytes, epoch)
            }));
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
    ffi_boundary(
        "sidereon_ssr_store_ingest_messages",
        SidereonStatus::Panic,
        || {
            let store = c_try!(require_out(
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
    ffi_boundary("sidereon_ssr_store_orbit", SidereonStatus::Panic, || {
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
    ffi_boundary("sidereon_ssr_store_clock", SidereonStatus::Panic, || {
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
    ffi_boundary(
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_code_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
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
            if let Some(value) = store.inner.code_bias(sat, signal) {
                *out_present = true;
                *out = value;
            }
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ssr_store_phase_bias_m(
    store: *const SidereonSsrCorrectionStore,
    sat_id: *const c_char,
    signal: u8,
    out_present: *mut bool,
    out_bias_m: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
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
    ffi_boundary(
        "sidereon_ssr_corrected_state",
        SidereonStatus::Panic,
        || {
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
    ffi_boundary(
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
            let inner = c_try!(guard(SidereonStatus::Solve, || {
                sidereon::solve_spp(
                    &corrected,
                    &solve_inputs,
                    inputs.with_geodetic,
                    SolvePolicy::default(),
                )
            }));
            write_boxed_handle(out_solution, SidereonSppSolution { inner });
            SidereonStatus::Ok
        },
    )
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
    ffi_boundary(
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

fn ssr_orbit_to_c(value: &SsrOrbitCorrection) -> SidereonSsrOrbitCorrection {
    SidereonSsrOrbitCorrection {
        source: match value.solution.source {
            sidereon_core::ssr::SsrSource::RtcmSsr => 0,
            sidereon_core::ssr::SsrSource::GalileoHas => 1,
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
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
        },
        provider_id: value.solution.provider_id,
        solution_id: value.solution.solution_id,
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
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) | CoreError::Parse(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_ssr_failure_outputs_and_nested_counts_are_deterministic() {
        let body = [
            0x42, 0x35, 0x46, 0x00, 0x2c, 0x80, 0x3d, 0xa0, 0x21, 0x88, 0x3d, 0x97, 0x24, 0x92,
            0x90,
        ];
        let mut handle = ptr::null_mut();
        let invalid = [0u8];
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
        assert_eq!((written, required), (0, 2));

        written = usize::MAX;
        required = usize::MAX;
        let mut rows: [SidereonRtcmSsrCodeBiasSignal; 2] = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe {
                sidereon_ssr_message_code_bias_signals(
                    handle,
                    1,
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
}
