use super::*;

/// Extended SSR metadata retaining the IGS version and the count of raw tail
/// bits omitted by the original summary type.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrInfoV2 {
    /// RTCM message number, including 4076 for IGS SSR.
    pub message_number: u16,
    /// Constellation carried by this SSR message.
    pub system: SidereonGnssSystem,
    /// SSR record group.
    pub kind: SidereonRtcmSsrKind,
    /// Common wire header.
    pub header: SidereonRtcmSsrHeader,
    /// Whether `igs_ssr_version` is present (only for message 4076).
    pub has_igs_ssr_version: bool,
    /// IGS SSR version when present; otherwise zero.
    pub igs_ssr_version: u8,
    /// Number of orbit records.
    pub orbit_count: usize,
    /// Number of clock records.
    pub clock_count: usize,
    /// Number of URA records.
    pub ura_count: usize,
    /// Number of code-bias satellite records.
    pub code_bias_count: usize,
    /// Number of phase-bias satellite records.
    pub phase_bias_count: usize,
    /// Number of raw trailing/padding bits after the final record.
    pub padding_bit_count: usize,
}

fn ssr_kind_from_c(kind: SidereonRtcmSsrKind) -> Option<RtcmSsrKind> {
    match kind {
        SidereonRtcmSsrKind::Orbit => Some(RtcmSsrKind::Orbit),
        SidereonRtcmSsrKind::Clock => Some(RtcmSsrKind::Clock),
        SidereonRtcmSsrKind::CombinedOrbitClock => Some(RtcmSsrKind::CombinedOrbitClock),
        SidereonRtcmSsrKind::CodeBias => Some(RtcmSsrKind::CodeBias),
        SidereonRtcmSsrKind::PhaseBias => Some(RtcmSsrKind::PhaseBias),
        SidereonRtcmSsrKind::Ura => Some(RtcmSsrKind::Ura),
        SidereonRtcmSsrKind::HighRateClock => Some(RtcmSsrKind::HighRateClock),
        SidereonRtcmSsrKind::Vtec => None,
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

fn ssr_info_v2_from_core(message: &RtcmSsrMessage) -> SidereonRtcmSsrInfoV2 {
    SidereonRtcmSsrInfoV2 {
        message_number: message.message_number,
        system: gnss_system_to_c(message.system),
        kind: ssr_kind_to_c(message.kind),
        header: SidereonRtcmSsrHeader {
            epoch_time_s: message.header.epoch_time_s,
            update_interval: message.header.update_interval,
            multiple_message: message.header.multiple_message,
            iod_ssr: message.header.iod_ssr,
            provider_id: message.header.provider_id,
            solution_id: message.header.solution_id,
            has_satellite_reference_datum: message.header.satellite_reference_datum.is_some(),
            satellite_reference_datum: message.header.satellite_reference_datum.unwrap_or(false),
            has_dispersive_bias_consistency: message.header.dispersive_bias_consistency.is_some(),
            dispersive_bias_consistency: message
                .header
                .dispersive_bias_consistency
                .unwrap_or(false),
            has_mw_consistency: message.header.mw_consistency.is_some(),
            mw_consistency: message.header.mw_consistency.unwrap_or(false),
            satellite_count: message.header.satellite_count,
        },
        has_igs_ssr_version: message.igs_ssr_version.is_some(),
        igs_ssr_version: message.igs_ssr_version.unwrap_or(0),
        orbit_count: message.orbit.len(),
        clock_count: message.clock.len(),
        ura_count: message.ura.len(),
        code_bias_count: message.code_bias.len(),
        phase_bias_count: message.phase_bias.len(),
        padding_bit_count: message.padding_bits.len(),
    }
}

fn ssr_header_from_c(header: &SidereonRtcmSsrHeader) -> RtcmSsrHeader {
    RtcmSsrHeader {
        epoch_time_s: header.epoch_time_s,
        update_interval: header.update_interval,
        multiple_message: header.multiple_message,
        iod_ssr: header.iod_ssr,
        provider_id: header.provider_id,
        solution_id: header.solution_id,
        satellite_reference_datum: header
            .has_satellite_reference_datum
            .then_some(header.satellite_reference_datum),
        dispersive_bias_consistency: header
            .has_dispersive_bias_consistency
            .then_some(header.dispersive_bias_consistency),
        mw_consistency: header.has_mw_consistency.then_some(header.mw_consistency),
        satellite_count: header.satellite_count,
    }
}

#[allow(clippy::too_many_arguments)]
fn ssr_message_from_c(
    info: &SidereonRtcmSsrInfoV2,
    orbit: &[SidereonRtcmSsrOrbitRecord],
    clock: &[SidereonRtcmSsrClockRecord],
    ura: &[SidereonRtcmSsrUraRecord],
    code_bias: &[SidereonRtcmSsrCodeBiasRecord],
    code_bias_signals: &[SidereonRtcmSsrCodeBiasSignal],
    phase_bias: &[SidereonRtcmSsrPhaseBiasRecord],
    phase_bias_signals: &[SidereonRtcmSsrPhaseBiasSignal],
    padding_bits: &[bool],
) -> std::result::Result<RtcmSsrMessage, &'static str> {
    let kind = ssr_kind_from_c(info.kind).ok_or("VTEC is not an SSR satellite-record kind")?;
    let code_bias = split_code_bias(code_bias, code_bias_signals)?;
    let phase_bias = split_phase_bias(phase_bias, phase_bias_signals)?;
    Ok(RtcmSsrMessage {
        message_number: info.message_number,
        igs_ssr_version: info.has_igs_ssr_version.then_some(info.igs_ssr_version),
        system: gnss_system_from_c_code(
            "sidereon_rtcm_build_ssr_v2",
            "info.system",
            info.system as u32,
        )
        .map_err(|_| "invalid SSR GNSS system")?,
        kind,
        header: ssr_header_from_c(&info.header),
        orbit: orbit
            .iter()
            .map(|record| RtcmSsrOrbitRecord {
                satellite_id: record.satellite_id,
                iode: record.iode,
                iod_crc: record.has_iod_crc.then_some(record.iod_crc),
                delta_radial: record.delta_radial,
                delta_along: record.delta_along,
                delta_cross: record.delta_cross,
                dot_delta_radial: record.dot_delta_radial,
                dot_delta_along: record.dot_delta_along,
                dot_delta_cross: record.dot_delta_cross,
            })
            .collect(),
        clock: clock
            .iter()
            .map(|record| RtcmSsrClockRecord {
                satellite_id: record.satellite_id,
                c0: record.c0,
                c1: record.c1,
                c2: record.c2,
            })
            .collect(),
        code_bias,
        phase_bias,
        ura: ura
            .iter()
            .map(|record| (record.satellite_id, record.ura_index))
            .collect(),
        padding_bits: padding_bits.to_vec(),
    })
}

fn split_code_bias(
    records: &[SidereonRtcmSsrCodeBiasRecord],
    signals: &[SidereonRtcmSsrCodeBiasSignal],
) -> std::result::Result<Vec<RtcmSsrCodeBiasRecord>, &'static str> {
    let mut offset = 0usize;
    let mut result = Vec::with_capacity(records.len());
    for record in records {
        let end = offset
            .checked_add(record.signal_count)
            .ok_or("code-bias signal count overflow")?;
        let rows = signals
            .get(offset..end)
            .ok_or("code-bias signal count exceeds supplied rows")?;
        result.push(RtcmSsrCodeBiasRecord {
            satellite_id: record.satellite_id,
            biases: rows
                .iter()
                .map(|signal| (signal.signal_id, signal.bias))
                .collect(),
        });
        offset = end;
    }
    if offset != signals.len() {
        return Err("unclaimed code-bias signal rows");
    }
    Ok(result)
}

fn split_phase_bias(
    records: &[SidereonRtcmSsrPhaseBiasRecord],
    signals: &[SidereonRtcmSsrPhaseBiasSignal],
) -> std::result::Result<Vec<RtcmSsrPhaseBiasRecord>, &'static str> {
    let mut offset = 0usize;
    let mut result = Vec::with_capacity(records.len());
    for record in records {
        let end = offset
            .checked_add(record.signal_count)
            .ok_or("phase-bias signal count overflow")?;
        let rows = signals
            .get(offset..end)
            .ok_or("phase-bias signal count exceeds supplied rows")?;
        result.push(RtcmSsrPhaseBiasRecord {
            satellite_id: record.satellite_id,
            yaw_angle: record.yaw_angle,
            yaw_rate: record.yaw_rate,
            biases: rows
                .iter()
                .map(|signal| RtcmSsrPhaseBiasSignal {
                    signal_id: signal.signal_id,
                    integer_indicator: signal.integer_indicator,
                    wide_lane_integer_indicator: signal.wide_lane_integer_indicator,
                    discontinuity_counter: signal.discontinuity_counter,
                    bias: signal.bias,
                })
                .collect(),
        });
        offset = end;
    }
    if offset != signals.len() {
        return Err("unclaimed phase-bias signal rows");
    }
    Ok(result)
}

/// Read the extended metadata of an SSR satellite message, including its IGS
/// version and raw post-record bit count.
///
/// # Safety
/// `messages` must be a live RTCM message handle; `out_info` must point to a
/// writable `SidereonRtcmSsrInfoV2`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_info_v2(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out_info: *mut SidereonRtcmSsrInfoV2,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_ssr_info_v2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        match message {
            RtcmMessage::Ssr(ssr) => {
                *out = ssr_info_v2_from_core(ssr);
                SidereonStatus::Ok
            }
            _ => rtcm_wrong_kind(FN_NAME, "an SSR satellite message"),
        }
    })
}

/// Construct a native RTCM SSR or IGS 4076 satellite message from raw record
/// arrays. Bias signal arrays are concatenated in satellite-record order.
///
/// # Safety
/// Every pointer must reference the stated number of readable elements, and
/// `out_messages` must point to a writable handle pointer. Null pointers are
/// accepted only for arrays with zero length. On success the returned handle
/// is owned by the caller and must be released with
/// `sidereon_rtcm_messages_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_ssr_v2(
    info: *const SidereonRtcmSsrInfoV2,
    orbit: *const SidereonRtcmSsrOrbitRecord,
    orbit_len: usize,
    clock: *const SidereonRtcmSsrClockRecord,
    clock_len: usize,
    ura: *const SidereonRtcmSsrUraRecord,
    ura_len: usize,
    code_bias: *const SidereonRtcmSsrCodeBiasRecord,
    code_bias_len: usize,
    code_bias_signals: *const SidereonRtcmSsrCodeBiasSignal,
    code_bias_signal_len: usize,
    phase_bias: *const SidereonRtcmSsrPhaseBiasRecord,
    phase_bias_len: usize,
    phase_bias_signals: *const SidereonRtcmSsrPhaseBiasSignal,
    phase_bias_signal_len: usize,
    padding_bits: *const bool,
    padding_bit_len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_ssr_v2";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out = ptr::null_mut();
        let info = c_try!(require_ref(info, FN_NAME, "info"));
        let orbit = c_try!(require_slice(orbit, orbit_len, FN_NAME, "orbit"));
        let clock = c_try!(require_slice(clock, clock_len, FN_NAME, "clock"));
        let ura = c_try!(require_slice(ura, ura_len, FN_NAME, "ura"));
        let code_bias = c_try!(require_slice(
            code_bias,
            code_bias_len,
            FN_NAME,
            "code_bias"
        ));
        let code_bias_signals = c_try!(require_slice(
            code_bias_signals,
            code_bias_signal_len,
            FN_NAME,
            "code_bias_signals"
        ));
        let phase_bias = c_try!(require_slice(
            phase_bias,
            phase_bias_len,
            FN_NAME,
            "phase_bias"
        ));
        let phase_bias_signals = c_try!(require_slice(
            phase_bias_signals,
            phase_bias_signal_len,
            FN_NAME,
            "phase_bias_signals"
        ));
        let padding_bits = c_try!(require_slice(
            padding_bits,
            padding_bit_len,
            FN_NAME,
            "padding_bits"
        ));
        if info.orbit_count != orbit_len
            || info.clock_count != clock_len
            || info.ura_count != ura_len
            || info.code_bias_count != code_bias_len
            || info.phase_bias_count != phase_bias_len
        {
            set_last_error(format!(
                "{FN_NAME}: SSR info record counts do not match supplied arrays"
            ));
            return SidereonStatus::InvalidArgument;
        }
        if padding_bit_len != info.padding_bit_count {
            set_last_error(format!(
                "{FN_NAME}: padding bit count {} does not match info count {}",
                padding_bit_len, info.padding_bit_count
            ));
            return SidereonStatus::InvalidArgument;
        }
        let message = match ssr_message_from_c(
            info,
            orbit,
            clock,
            ura,
            code_bias,
            code_bias_signals,
            phase_bias,
            phase_bias_signals,
            padding_bits,
        ) {
            Ok(message) => message,
            Err(problem) => {
                set_last_error(format!("{FN_NAME}: {problem}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        rtcm_build(out, RtcmMessage::Ssr(message));
        SidereonStatus::Ok
    })
}

/// Copy the raw trailing or SSR padding bits of a message into caller memory.
/// Unsupported messages have a raw body rather than a separate tail and are
/// not accepted by this accessor.
///
/// # Safety
/// `messages` must be a live handle; `out` must point to `len` writable bools
/// or be null when `len` is zero; both count pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_trailing_bits(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_trailing_bits";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let bits = match message {
            RtcmMessage::Msm(value) => &value.trailing_bits,
            RtcmMessage::LegacyObservations(value) => &value.trailing_bits,
            RtcmMessage::StationCoordinates(value) => &value.trailing_bits,
            RtcmMessage::AntennaDescriptor(value) => &value.trailing_bits,
            RtcmMessage::SystemParameters(value) => &value.trailing_bits,
            RtcmMessage::Text(value) => &value.trailing_bits,
            RtcmMessage::NetworkAuxiliaryStation(value) => &value.trailing_bits,
            RtcmMessage::NetworkCorrectionDifferences(value) => &value.trailing_bits,
            RtcmMessage::NetworkResiduals(value) => &value.trailing_bits,
            RtcmMessage::PhysicalReferenceStation(value) => &value.trailing_bits,
            RtcmMessage::FkpGradients(value) => &value.trailing_bits,
            RtcmMessage::HelmertTransformation(value) => &value.trailing_bits,
            RtcmMessage::ResidualGrid(value) => &value.trailing_bits,
            RtcmMessage::Projection(value) => &value.trailing_bits,
            RtcmMessage::GpsEphemeris(value) => &value.trailing_bits,
            RtcmMessage::GlonassEphemeris(value) => &value.trailing_bits,
            RtcmMessage::NavicEphemeris(value) => &value.trailing_bits,
            RtcmMessage::BeidouEphemeris(value) => &value.trailing_bits,
            RtcmMessage::QzssEphemeris(value) => &value.trailing_bits,
            RtcmMessage::GalileoFnavEphemeris(value) => &value.trailing_bits,
            RtcmMessage::GalileoInavEphemeris(value) => &value.trailing_bits,
            RtcmMessage::GlonassCodePhaseBiases(value) => &value.trailing_bits,
            RtcmMessage::Ssr(value) => &value.padding_bits,
            RtcmMessage::SsrVtec(value) => &value.trailing_bits,
            RtcmMessage::Unsupported(_) => {
                return rtcm_wrong_kind(FN_NAME, "a message with explicit trailing bits");
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            bits,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Clone one supported message, replacing its raw trailing/padding bits.
/// This is the additive tail-aware constructor path for message families
/// whose original field constructors predate tail inputs.
///
/// # Safety
/// `messages` must be a live handle; `trailing_bits` must point to
/// `trailing_bit_len` readable bools or be null when the length is zero;
/// `out_messages` must point to a writable handle pointer. On success the new
/// handle is owned by the caller and must be freed with
/// `sidereon_rtcm_messages_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_with_trailing_bits(
    messages: *const SidereonRtcmMessages,
    index: usize,
    trailing_bits: *const bool,
    trailing_bit_len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_with_trailing_bits";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out = ptr::null_mut();
        let trailing_bits = c_try!(require_slice(
            trailing_bits,
            trailing_bit_len,
            FN_NAME,
            "trailing_bits"
        ));
        let source = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let mut message = source.clone();
        if !replace_message_tail(&mut message, trailing_bits) {
            return rtcm_wrong_kind(FN_NAME, "a message with explicit trailing bits");
        }
        rtcm_build(out, message);
        SidereonStatus::Ok
    })
}

fn replace_message_tail(message: &mut RtcmMessage, bits: &[bool]) -> bool {
    let tail = match message {
        RtcmMessage::Msm(value) => &mut value.trailing_bits,
        RtcmMessage::LegacyObservations(value) => &mut value.trailing_bits,
        RtcmMessage::StationCoordinates(value) => &mut value.trailing_bits,
        RtcmMessage::AntennaDescriptor(value) => &mut value.trailing_bits,
        RtcmMessage::SystemParameters(value) => &mut value.trailing_bits,
        RtcmMessage::Text(value) => &mut value.trailing_bits,
        RtcmMessage::NetworkAuxiliaryStation(value) => &mut value.trailing_bits,
        RtcmMessage::NetworkCorrectionDifferences(value) => &mut value.trailing_bits,
        RtcmMessage::NetworkResiduals(value) => &mut value.trailing_bits,
        RtcmMessage::PhysicalReferenceStation(value) => &mut value.trailing_bits,
        RtcmMessage::FkpGradients(value) => &mut value.trailing_bits,
        RtcmMessage::HelmertTransformation(value) => &mut value.trailing_bits,
        RtcmMessage::ResidualGrid(value) => &mut value.trailing_bits,
        RtcmMessage::Projection(value) => &mut value.trailing_bits,
        RtcmMessage::GpsEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::GlonassEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::NavicEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::BeidouEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::QzssEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::GalileoFnavEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::GalileoInavEphemeris(value) => &mut value.trailing_bits,
        RtcmMessage::GlonassCodePhaseBiases(value) => &mut value.trailing_bits,
        RtcmMessage::Ssr(value) => &mut value.padding_bits,
        RtcmMessage::SsrVtec(value) => &mut value.trailing_bits,
        RtcmMessage::Unsupported(_) => return false,
    };
    tail.clear();
    tail.extend_from_slice(bits);
    true
}

/// Copy the undecoded raw body of an unsupported message.
///
/// # Safety
/// `messages` must be a live handle; `out` must point to `len` writable bytes
/// or be null when `len` is zero; both count pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_unsupported_body(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_unsupported_body";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let unsupported = match message {
            RtcmMessage::Unsupported(value) => value,
            _ => return rtcm_wrong_kind(FN_NAME, "an unsupported message"),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &unsupported.body,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Construct an unsupported-message record while retaining its complete raw
/// body. The core encoder validates that the supplied body carries the stated
/// message number; this constructor does not rewrite wire bits.
///
/// # Safety
/// `body` must point to `body_len` readable bytes or be null when the length is
/// zero; `out_messages` must point to a writable handle pointer. The returned
/// handle belongs to the caller and must be freed with
/// `sidereon_rtcm_messages_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_unsupported(
    message_number: u16,
    body: *const u8,
    body_len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_unsupported";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out = ptr::null_mut();
        let body = c_try!(require_slice(body, body_len, FN_NAME, "body"));
        rtcm_build(
            out,
            RtcmMessage::Unsupported(core_rtcm::UnsupportedMessage {
                message_number,
                body: body.to_vec(),
            }),
        );
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sbas_orbit_constructor_preserves_crc_and_raw_tail() {
        let info = SidereonRtcmSsrInfoV2 {
            message_number: 1252,
            system: SidereonGnssSystem::Sbas,
            kind: SidereonRtcmSsrKind::Orbit,
            header: SidereonRtcmSsrHeader {
                epoch_time_s: 48,
                update_interval: 1,
                multiple_message: false,
                iod_ssr: 3,
                provider_id: 4,
                solution_id: 2,
                has_satellite_reference_datum: true,
                satellite_reference_datum: true,
                has_dispersive_bias_consistency: false,
                dispersive_bias_consistency: false,
                has_mw_consistency: false,
                mw_consistency: false,
                satellite_count: 1,
            },
            has_igs_ssr_version: false,
            igs_ssr_version: 0,
            orbit_count: 1,
            clock_count: 0,
            ura_count: 0,
            code_bias_count: 0,
            phase_bias_count: 0,
            padding_bit_count: 1,
        };
        let orbit = [SidereonRtcmSsrOrbitRecord {
            satellite_id: 1,
            iode: 17,
            has_iod_crc: true,
            iod_crc: 0x123456,
            delta_radial: -3,
            delta_along: 4,
            delta_cross: -5,
            dot_delta_radial: 6,
            dot_delta_along: -7,
            dot_delta_cross: 8,
        }];
        let padding = [true];
        let message = ssr_message_from_c(&info, &orbit, &[], &[], &[], &[], &[], &[], &padding)
            .expect("valid SBAS SSR record");
        assert_eq!(message.system, GnssSystem::Sbas);
        assert_eq!(message.orbit[0].iod_crc, Some(0x123456));
        assert_eq!(message.padding_bits, padding);
        let (encoded, _) = message
            .encode_with_policy(core_rtcm::RtcmPolicy::Lenient)
            .unwrap();
        let (decoded, _) =
            RtcmSsrMessage::decode_with_policy(&encoded, core_rtcm::RtcmPolicy::Lenient).unwrap();
        let mut expected = message;
        expected.padding_bits = [true, false, false, false].to_vec();
        assert_eq!(encoded.len(), 29);
        assert_eq!(decoded, expected);
    }

    #[test]
    fn igs_phase_bias_constructor_keeps_version_signal_flags_and_padding() {
        let info = SidereonRtcmSsrInfoV2 {
            message_number: 4076,
            system: SidereonGnssSystem::Sbas,
            kind: SidereonRtcmSsrKind::PhaseBias,
            header: SidereonRtcmSsrHeader {
                epoch_time_s: 4321,
                update_interval: 2,
                multiple_message: true,
                iod_ssr: 4,
                provider_id: 12,
                solution_id: 3,
                has_satellite_reference_datum: false,
                satellite_reference_datum: false,
                has_dispersive_bias_consistency: true,
                dispersive_bias_consistency: true,
                has_mw_consistency: true,
                mw_consistency: false,
                satellite_count: 1,
            },
            has_igs_ssr_version: true,
            igs_ssr_version: 1,
            orbit_count: 0,
            clock_count: 0,
            ura_count: 0,
            code_bias_count: 0,
            phase_bias_count: 1,
            padding_bit_count: 1,
        };
        let phase_bias = [SidereonRtcmSsrPhaseBiasRecord {
            satellite_id: 48,
            yaw_angle: 20,
            yaw_rate: -1,
            signal_count: 1,
        }];
        let signals = [SidereonRtcmSsrPhaseBiasSignal {
            signal_id: 7,
            integer_indicator: 1,
            wide_lane_integer_indicator: 0,
            discontinuity_counter: 9,
            bias: -12,
        }];
        let padding = [false];
        let message = ssr_message_from_c(
            &info,
            &[],
            &[],
            &[],
            &[],
            &[],
            &phase_bias,
            &signals,
            &padding,
        )
        .expect("valid IGS SBAS phase-bias record");
        assert_eq!(message.igs_ssr_version, Some(1));
        assert_eq!(message.phase_bias[0].biases[0].discontinuity_counter, 9);
        assert_eq!(message.padding_bits, padding);
        let info_v2 = ssr_info_v2_from_core(&message);
        assert!(info_v2.has_igs_ssr_version);
        assert_eq!(info_v2.igs_ssr_version, 1);
        assert_eq!(info_v2.padding_bit_count, 1);
        let (encoded, _) = message
            .encode_with_policy(core_rtcm::RtcmPolicy::Lenient)
            .unwrap();
        let (decoded, _) =
            RtcmSsrMessage::decode_with_policy(&encoded, core_rtcm::RtcmPolicy::Lenient).unwrap();
        let mut expected = message;
        expected.padding_bits = [false, false, false, false].to_vec();
        assert_eq!(encoded.len(), 18);
        assert_eq!(decoded, expected);
    }

    #[test]
    fn generic_tail_replacement_covers_ssr_and_rejects_unsupported() {
        let mut message = RtcmMessage::Ssr(RtcmSsrMessage {
            message_number: 1058,
            igs_ssr_version: None,
            system: GnssSystem::Gps,
            kind: RtcmSsrKind::Clock,
            header: RtcmSsrHeader {
                epoch_time_s: 1,
                update_interval: 0,
                multiple_message: false,
                iod_ssr: 0,
                provider_id: 0,
                solution_id: 0,
                satellite_reference_datum: None,
                dispersive_bias_consistency: None,
                mw_consistency: None,
                satellite_count: 0,
            },
            orbit: Vec::new(),
            clock: Vec::new(),
            code_bias: Vec::new(),
            phase_bias: Vec::new(),
            ura: Vec::new(),
            padding_bits: Vec::new(),
        });
        assert!(replace_message_tail(&mut message, &[true, false]));
        let RtcmMessage::Ssr(ssr) = message else {
            panic!("SSR message retained its variant");
        };
        assert_eq!(ssr.padding_bits, [true, false]);
        let mut unsupported = RtcmMessage::Unsupported(core_rtcm::UnsupportedMessage {
            message_number: 4090,
            body: vec![0xff, 0xa0],
        });
        assert!(!replace_message_tail(&mut unsupported, &[true]));
    }

    #[test]
    fn unsupported_body_constructor_and_accessor_preserve_raw_bytes() {
        let body = [0xff, 0xa0];
        let mut messages = ptr::null_mut();
        let build_status = unsafe {
            sidereon_rtcm_build_unsupported(4090, body.as_ptr(), body.len(), &mut messages)
        };
        assert_eq!(build_status, SidereonStatus::Ok);
        let mut returned_body = [0u8; 2];
        let mut written = 0;
        let mut required = 0;
        let read_status = unsafe {
            sidereon_rtcm_message_unsupported_body(
                messages,
                0,
                returned_body.as_mut_ptr(),
                returned_body.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(read_status, SidereonStatus::Ok);
        assert_eq!(returned_body, body);
        assert_eq!(written, body.len());
        assert_eq!(required, body.len());
        unsafe { sidereon_rtcm_messages_free(messages) };
    }
}
