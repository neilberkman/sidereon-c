use super::*;

// --- RTCM 3 decode/encode (sidereon_core::rtcm) ------------------------------

/// A decoded list of RTCM 3 messages. Opaque to C. Create with
/// sidereon_rtcm_decode_messages; release with sidereon_rtcm_messages_free.
pub struct SidereonRtcmMessages {
    pub(crate) messages: Vec<RtcmMessage>,
}

/// A set of scanned RTCM 3 transport frames. Opaque to C. Create with
/// sidereon_rtcm_scan_frames; release with sidereon_rtcm_frames_free.
pub struct SidereonRtcmFrames {
    pub(crate) frames: Vec<RtcmFrameRecord>,
}

/// Diagnostics from forgiving RTCM stream decoding. Opaque to C. Create with
/// sidereon_rtcm_decode_stream; release with
/// sidereon_rtcm_stream_diagnostics_free.
pub struct SidereonRtcmStreamDiagnostics {
    pub(crate) diagnostics: RtcmStreamDiagnostics,
}

/// Stateful MSM lock-time tracker for deriving RINEX LLI continuity bits.
/// Create with sidereon_rtcm_lock_time_tracker_new; release with
/// sidereon_rtcm_lock_time_tracker_free.
pub struct SidereonRtcmLockTimeTracker {
    pub(crate) tracker: RtcmLockTimeTracker,
}

/// Which RTCM message IR variant a decoded message is.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmMessageKind {
    /// An MSM4 / MSM7 multi-signal observation message.
    Msm = 0,
    /// A 1005 / 1006 station antenna reference point.
    StationCoordinates = 1,
    /// A 1007 / 1008 / 1033 antenna or receiver descriptor.
    AntennaDescriptor = 2,
    /// A 1019 GPS broadcast ephemeris.
    GpsEphemeris = 3,
    /// A 1020 GLONASS broadcast ephemeris.
    GlonassEphemeris = 4,
    /// An SSR correction message.
    Ssr = 5,
    /// A recognized-but-undecoded message, preserved verbatim.
    Unsupported = 6,
}

/// Which MSM variant an observation message is, mirroring
/// sidereon_core::rtcm::MsmKind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmMsmKind {
    /// MSM4: pseudorange + phase range, standard resolution.
    Msm4 = 0,
    /// MSM7: pseudorange + phase range + phase-range-rate, extended resolution.
    Msm7 = 1,
}

/// Why a CRC-valid RTCM frame could not be decoded into the message IR.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmFrameSkipReason {
    /// The body ended before all required fields of its recognized type.
    Truncated = 0,
    /// The body is internally inconsistent for its recognized type.
    Malformed = 1,
}

/// One CRC-valid frame skipped by sidereon_rtcm_decode_stream.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmFrameSkip {
    /// Byte offset of the frame preamble in the scanned input buffer.
    pub offset: usize,
    /// Whether message_number is present.
    pub has_message_number: bool,
    /// RTCM message number when present, otherwise 0.
    pub message_number: u16,
    /// Skip reason.
    pub reason: SidereonRtcmFrameSkipReason,
}

/// Previous MSM lock-state input for sidereon_rtcm_derive_lli.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmPreviousLock {
    /// Whether min_lock_time_ms is present.
    pub has_min_lock_time_ms: bool,
    /// Previous minimum continuous-lock time in milliseconds when present.
    pub min_lock_time_ms: u32,
    /// Elapsed milliseconds between previous and current observations.
    pub elapsed_ms: u64,
}

/// Derived RINEX LLI for one MSM signal cell.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmCellLli {
    /// Satellite id from the MSM signal cell.
    pub satellite_id: u8,
    /// Signal id from the MSM signal cell.
    pub signal_id: u8,
    /// Derived RINEX LLI value. Bits 0 and 1 are set by the RTCM MSM rules.
    pub lli: u8,
    /// Whether min_lock_time_ms is present.
    pub has_min_lock_time_ms: bool,
    /// Current normalized minimum lock time in milliseconds when present.
    pub min_lock_time_ms: u32,
}

/// A decoded 1005 / 1006 station antenna reference point, mirroring
/// sidereon_core::rtcm::StationCoordinates with derived metre values.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmStationCoordinates {
    /// 1005 or 1006.
    pub message_number: u16,
    /// Reference station identifier.
    pub reference_station_id: u16,
    /// ITRF realization year.
    pub itrf_realization_year: u8,
    /// GPS service supported.
    pub gps_indicator: bool,
    /// GLONASS service supported.
    pub glonass_indicator: bool,
    /// Galileo service supported.
    pub galileo_indicator: bool,
    /// Physical vs non-physical reference-station indicator.
    pub reference_station_indicator: bool,
    /// Single receiver oscillator indicator.
    pub single_receiver_oscillator: bool,
    /// Reserved bit, preserved for exact round-trip.
    pub reserved: bool,
    /// Quarter-cycle indicator.
    pub quarter_cycle_indicator: u8,
    /// Raw ECEF X integer (0.0001 m steps).
    pub ecef_x: i64,
    /// Raw ECEF Y integer (0.0001 m steps).
    pub ecef_y: i64,
    /// Raw ECEF Z integer (0.0001 m steps).
    pub ecef_z: i64,
    /// ECEF X in metres.
    pub x_m: f64,
    /// ECEF Y in metres.
    pub y_m: f64,
    /// ECEF Z in metres.
    pub z_m: f64,
    /// Whether an antenna height is present (true only for 1006).
    pub has_antenna_height: bool,
    /// Raw antenna-height integer (0.0001 m steps) when present.
    pub antenna_height: u16,
    /// Antenna height in metres when present, otherwise 0.
    pub antenna_height_m: f64,
}

/// A decoded 1007 / 1008 / 1033 antenna or receiver descriptor's scalar fields.
/// Read the variable-length string fields with sidereon_rtcm_message_antenna_string.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmAntennaDescriptor {
    /// 1007, 1008, or 1033.
    pub message_number: u16,
    /// Reference station identifier.
    pub reference_station_id: u16,
    /// Antenna setup id.
    pub antenna_setup_id: u8,
    /// Whether an antenna serial number is present.
    pub has_antenna_serial_number: bool,
    /// Whether a receiver type descriptor is present.
    pub has_receiver_type: bool,
    /// Whether a receiver firmware version is present.
    pub has_receiver_firmware_version: bool,
    /// Whether a receiver serial number is present.
    pub has_receiver_serial_number: bool,
}

/// Selects which antenna-descriptor string field a reader returns. Pass as a
/// uint32_t.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmAntennaStringField {
    /// Antenna descriptor.
    AntennaDescriptor = 0,
    /// Antenna serial number (optional).
    AntennaSerialNumber = 1,
    /// Receiver type descriptor (optional).
    ReceiverType = 2,
    /// Receiver firmware version (optional).
    ReceiverFirmwareVersion = 3,
    /// Receiver serial number (optional).
    ReceiverSerialNumber = 4,
}

/// MSM common header, mirroring sidereon_core::rtcm::MsmHeader.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMsmHeader {
    /// Reference station identifier.
    pub reference_station_id: u16,
    /// Raw 30-bit GNSS epoch time (constellation-specific meaning).
    pub epoch_time: u32,
    /// Multiple-message bit.
    pub multiple_message: bool,
    /// Issue of data station.
    pub iods: u8,
    /// Reserved field, preserved for round-trip.
    pub reserved: u8,
    /// Clock steering indicator.
    pub clock_steering: u8,
    /// External clock indicator.
    pub external_clock: u8,
    /// Divergence-free smoothing indicator.
    pub divergence_free_smoothing: bool,
    /// Smoothing interval.
    pub smoothing_interval: u8,
}

/// Summary of a decoded MSM observation message. Read the per-satellite and
/// per-signal cells with sidereon_rtcm_message_msm_satellites and
/// sidereon_rtcm_message_msm_signals.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMsmInfo {
    /// The message number (e.g. 1077).
    pub message_number: u16,
    /// The constellation, a SidereonGnssSystem value.
    pub system: SidereonGnssSystem,
    /// The MSM variant.
    pub kind: SidereonRtcmMsmKind,
    /// Common MSM header.
    pub header: SidereonRtcmMsmHeader,
    /// Number of active satellites.
    pub satellite_count: usize,
    /// Number of active signal cells.
    pub signal_count: usize,
}

/// Per-satellite MSM data, mirroring sidereon_core::rtcm::MsmSatellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMsmSatellite {
    /// Satellite id (1-based satellite-mask index).
    pub id: u8,
    /// Rough range, whole milliseconds (255 marks invalid).
    pub rough_range_ms: u8,
    /// Rough range remainder in 1/1024 ms.
    pub rough_range_mod1: u16,
    /// Whether extended info is present (MSM7).
    pub has_extended_info: bool,
    /// Extended satellite info when present.
    pub extended_info: u8,
    /// Whether a rough phase-range-rate is present (MSM7).
    pub has_rough_phase_range_rate: bool,
    /// Rough phase-range-rate in whole m/s when present.
    pub rough_phase_range_rate_m_s: i16,
}

/// Per-cell MSM signal data, mirroring sidereon_core::rtcm::MsmSignal.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMsmSignal {
    /// Owning satellite id (1-based satellite-mask index).
    pub satellite_id: u8,
    /// Signal id (1-based signal-mask index).
    pub signal_id: u8,
    /// Fine pseudorange (raw integer, scale per MSM variant).
    pub fine_pseudorange: i32,
    /// Fine phase range (raw integer, scale per MSM variant).
    pub fine_phase_range: i32,
    /// Phase-range lock-time indicator.
    pub lock_time_indicator: u16,
    /// Half-cycle ambiguity indicator.
    pub half_cycle_ambiguity: bool,
    /// Carrier-to-noise density ratio (raw integer, scale per MSM variant).
    pub cnr: u16,
    /// Whether a fine phase-range-rate is present (MSM7).
    pub has_fine_phase_range_rate: bool,
    /// Fine phase-range-rate when present.
    pub fine_phase_range_rate: i16,
}

/// A decoded 1019 GPS broadcast ephemeris, mirroring
/// sidereon_core::rtcm::GpsEphemeris. Every field is the raw transmitted integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmGpsEphemeris {
    /// GPS satellite PRN.
    pub satellite_id: u8,
    /// GPS week number.
    pub week_number: u16,
    /// SV accuracy / URA index.
    pub sv_accuracy: u8,
    /// Code on L2.
    pub code_on_l2: u8,
    /// Rate of inclination angle IDOT.
    pub idot: i32,
    /// Issue of data, ephemeris.
    pub iode: u8,
    /// Clock data reference time t_oc.
    pub t_oc: u16,
    /// Clock drift rate a_f2.
    pub a_f2: i16,
    /// Clock drift a_f1.
    pub a_f1: i32,
    /// Clock bias a_f0.
    pub a_f0: i32,
    /// Issue of data, clock.
    pub iodc: u16,
    /// Orbit-radius sine correction C_rs.
    pub c_rs: i32,
    /// Mean-motion difference dn.
    pub delta_n: i32,
    /// Mean anomaly at reference time M_0.
    pub m0: i64,
    /// Latitude-argument cosine correction C_uc.
    pub c_uc: i32,
    /// Eccentricity.
    pub eccentricity: u64,
    /// Latitude-argument sine correction C_us.
    pub c_us: i32,
    /// Square root of the semi-major axis.
    pub sqrt_a: u64,
    /// Ephemeris reference time t_oe.
    pub t_oe: u16,
    /// Inclination cosine correction C_ic.
    pub c_ic: i32,
    /// Longitude of ascending node Omega_0.
    pub omega0: i64,
    /// Inclination sine correction C_is.
    pub c_is: i32,
    /// Inclination at reference time i_0.
    pub i0: i64,
    /// Orbit-radius cosine correction C_rc.
    pub c_rc: i32,
    /// Argument of perigee omega.
    pub omega: i64,
    /// Rate of right ascension Omega-dot.
    pub omega_dot: i32,
    /// Group delay differential t_GD.
    pub t_gd: i16,
    /// SV health.
    pub sv_health: u8,
    /// L2 P-data flag.
    pub l2_p_data_flag: bool,
    /// Fit-interval flag.
    pub fit_interval: bool,
}

/// A decoded 1020 GLONASS broadcast ephemeris, mirroring
/// sidereon_core::rtcm::GlonassEphemeris. Every field is the raw transmitted
/// integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmGlonassEphemeris {
    /// GLONASS satellite slot number.
    pub satellite_id: u8,
    /// Frequency channel number (wire value is k + 7).
    pub frequency_channel: u8,
    /// Almanac health C_n.
    pub almanac_health: bool,
    /// Almanac health availability.
    pub almanac_health_availability: bool,
    /// P1 flag.
    pub p1: u8,
    /// Frame time t_k.
    pub t_k: u16,
    /// MSB of the B_n health word.
    pub b_n_msb: bool,
    /// P2 flag.
    pub p2: bool,
    /// Ephemeris reference time t_b.
    pub t_b: u8,
    /// X-velocity.
    pub xn_dot: i32,
    /// X-position.
    pub xn: i32,
    /// X-acceleration.
    pub xn_dot_dot: i8,
    /// Y-velocity.
    pub yn_dot: i32,
    /// Y-position.
    pub yn: i32,
    /// Y-acceleration.
    pub yn_dot_dot: i8,
    /// Z-velocity.
    pub zn_dot: i32,
    /// Z-position.
    pub zn: i32,
    /// Z-acceleration.
    pub zn_dot_dot: i8,
    /// P3 flag.
    pub p3: bool,
    /// Relative carrier-frequency offset gamma_n.
    pub gamma_n: i16,
    /// GLONASS-M P flag.
    pub m_p: u8,
    /// Third-string l_n health flag.
    pub m_l_n_third: bool,
    /// Clock bias tau_n.
    pub tau_n: i32,
    /// Inter-frequency bias delta_tau_n.
    pub delta_tau_n: i8,
    /// Age of operation E_n (days).
    pub e_n: u8,
    /// GLONASS-M P4 flag.
    pub m_p4: bool,
    /// GLONASS-M F_t accuracy index.
    pub m_f_t: u8,
    /// GLONASS-M N_t calendar day number.
    pub m_n_t: u16,
    /// GLONASS-M M satellite type.
    pub m_m: u8,
    /// Additional data availability.
    pub additional_data_available: bool,
    /// N_A almanac reference day.
    pub n_a: u16,
    /// System time scale offset tau_c.
    pub tau_c: i64,
    /// GLONASS-M N_4 four-year interval number.
    pub m_n4: u8,
    /// GLONASS-M tau_GPS offset to GPS time.
    pub m_tau_gps: i32,
    /// Fifth-string l_n health flag.
    pub m_l_n_fifth: bool,
    /// Reserved field, preserved for round-trip.
    pub reserved: u8,
}

/// Decode every CRC-valid RTCM 3 frame in a byte buffer into a message list.
/// Forgiving: bad frames are skipped. On success writes a newly owned handle to
/// *out_messages. Release it with sidereon_rtcm_messages_free. Delegates to
/// sidereon_core::rtcm::decode_messages.
///
/// Safety: bytes points to len readable bytes; out_messages points to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_decode_messages(
    bytes: *const u8,
    len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_decode_messages",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_decode_messages",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let data = c_try!(require_slice(
                bytes,
                len,
                "sidereon_rtcm_decode_messages",
                "bytes"
            ));
            let messages = core_rtcm::decode_messages(data);
            write_boxed_handle(out_messages, SidereonRtcmMessages { messages });
            SidereonStatus::Ok
        },
    )
}

/// Decode an RTCM 3 byte stream into messages plus forgiving stream diagnostics.
/// Bad CRC frames and incomplete trailing bytes count as resync bytes; CRC-valid
/// frames with undecodable bodies are reported in diagnostics. On success writes
/// newly owned handles to *out_messages and *out_diagnostics. Release them with
/// sidereon_rtcm_messages_free and sidereon_rtcm_stream_diagnostics_free.
///
/// Safety: bytes points to len readable bytes; out_messages points to a
/// SidereonRtcmMessages*; out_diagnostics points to a
/// SidereonRtcmStreamDiagnostics*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_decode_stream(
    bytes: *const u8,
    len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
    out_diagnostics: *mut *mut SidereonRtcmStreamDiagnostics,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_decode_stream", SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(
            out_messages,
            "sidereon_rtcm_decode_stream",
            "out_messages"
        ));
        *out_messages = ptr::null_mut();
        let out_diagnostics = c_try!(require_out(
            out_diagnostics,
            "sidereon_rtcm_decode_stream",
            "out_diagnostics"
        ));
        *out_diagnostics = ptr::null_mut();
        let data = c_try!(require_slice(
            bytes,
            len,
            "sidereon_rtcm_decode_stream",
            "bytes"
        ));
        let stream = core_rtcm::decode_stream(data);
        write_boxed_handle(
            out_messages,
            SidereonRtcmMessages {
                messages: stream.messages,
            },
        );
        write_boxed_handle(
            out_diagnostics,
            SidereonRtcmStreamDiagnostics {
                diagnostics: stream.diagnostics,
            },
        );
        SidereonStatus::Ok
    })
}

/// Copy the number of bytes skipped while resynchronizing during stream decode.
///
/// Safety: diagnostics is a live handle; out_resync_bytes points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_resync_bytes(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    out_resync_bytes: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_stream_diagnostics_resync_bytes",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_resync_bytes,
                "sidereon_rtcm_stream_diagnostics_resync_bytes",
                "out_resync_bytes"
            ));
            *out = 0;
            let diagnostics = c_try!(require_ref(
                diagnostics,
                "sidereon_rtcm_stream_diagnostics_resync_bytes",
                "diagnostics"
            ));
            *out = diagnostics.diagnostics.resync_bytes;
            SidereonStatus::Ok
        },
    )
}

/// Copy the number of CRC-valid frames skipped because their bodies could not
/// be decoded.
///
/// Safety: diagnostics is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_skipped_frames_count(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_stream_diagnostics_skipped_frames_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rtcm_stream_diagnostics_skipped_frames_count",
                "out_count"
            ));
            *out = 0;
            let diagnostics = c_try!(require_ref(
                diagnostics,
                "sidereon_rtcm_stream_diagnostics_skipped_frames_count",
                "diagnostics"
            ));
            *out = diagnostics.diagnostics.skipped_frames.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one skipped-frame diagnostic row into *out.
///
/// Safety: diagnostics is a live handle; out points to a
/// SidereonRtcmFrameSkip.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_skipped_frame(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    index: usize,
    out: *mut SidereonRtcmFrameSkip,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_stream_diagnostics_skipped_frame",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_stream_diagnostics_skipped_frame",
                "out"
            ));
            *out = SidereonRtcmFrameSkip {
                offset: 0,
                has_message_number: false,
                message_number: 0,
                reason: SidereonRtcmFrameSkipReason::Truncated,
            };
            let diagnostics = c_try!(require_ref(
                diagnostics,
                "sidereon_rtcm_stream_diagnostics_skipped_frame",
                "diagnostics"
            ));
            let skip = match diagnostics.diagnostics.skipped_frames.get(index) {
                Some(skip) => skip,
                None => {
                    set_last_error(format!(
                        "sidereon_rtcm_stream_diagnostics_skipped_frame: index {index} out of range ({} skipped frames)",
                        diagnostics.diagnostics.skipped_frames.len()
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = rtcm_frame_skip_to_c(skip);
            SidereonStatus::Ok
        },
    )
}

/// Copy the malformed-frame detail string for one skipped-frame row. Truncated
/// rows have an empty message. Variable-length output contract.
///
/// Safety: diagnostics is a live handle; out points to len writable bytes or
/// NULL when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_skipped_frame_message(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_stream_diagnostics_skipped_frame_message",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_stream_diagnostics_skipped_frame_message",
                out_written,
                out_required
            ));
            let diagnostics = c_try!(require_ref(
                diagnostics,
                "sidereon_rtcm_stream_diagnostics_skipped_frame_message",
                "diagnostics"
            ));
            let skip = match diagnostics.diagnostics.skipped_frames.get(index) {
                Some(skip) => skip,
                None => {
                    set_last_error(format!(
                        "sidereon_rtcm_stream_diagnostics_skipped_frame_message: index {index} out of range ({} skipped frames)",
                        diagnostics.diagnostics.skipped_frames.len()
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let message = match &skip.reason {
                core_rtcm::FrameSkipReason::Truncated => "",
                core_rtcm::FrameSkipReason::Malformed(message) => message.as_str(),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_stream_diagnostics_skipped_frame_message",
                "out",
                message.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a stream diagnostics handle from sidereon_rtcm_decode_stream. Passing
/// NULL is a no-op.
///
/// Safety: diagnostics must be NULL or a live diagnostics handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_free(
    diagnostics: *mut SidereonRtcmStreamDiagnostics,
) {
    ffi_boundary("sidereon_rtcm_stream_diagnostics_free", (), || {
        free_boxed(diagnostics);
    });
}

/// Number of messages in a decoded RTCM list.
///
/// Safety: messages is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_messages_count(
    messages: *const SidereonRtcmMessages,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_messages_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_rtcm_messages_count",
                "out_count"
            ));
            *out = 0;
            let handle = c_try!(require_ref(
                messages,
                "sidereon_rtcm_messages_count",
                "messages"
            ));
            *out = handle.messages.len();
            SidereonStatus::Ok
        },
    )
}

/// Report the IR variant and RTCM message number of one decoded message.
///
/// Safety: messages is a live handle; out_kind points to a
/// SidereonRtcmMessageKind; out_message_number points to a uint16_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_kind(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out_kind: *mut SidereonRtcmMessageKind,
    out_message_number: *mut u16,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_message_kind", SidereonStatus::Panic, || {
        let out_kind = c_try!(require_out(
            out_kind,
            "sidereon_rtcm_message_kind",
            "out_kind"
        ));
        let out_message_number = c_try!(require_out(
            out_message_number,
            "sidereon_rtcm_message_kind",
            "out_message_number"
        ));
        let message = c_try!(rtcm_message_at(
            "sidereon_rtcm_message_kind",
            messages,
            index
        ));
        *out_kind = rtcm_message_kind_of(message);
        *out_message_number = message.message_number();
        SidereonStatus::Ok
    })
}

/// Copy a decoded 1005 / 1006 station coordinates message into *out.
///
/// Safety: messages is a live handle; out points to a
/// SidereonRtcmStationCoordinates.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_station_coordinates(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmStationCoordinates,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_station_coordinates",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_station_coordinates",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_station_coordinates",
                messages,
                index
            ));
            match message {
                RtcmMessage::StationCoordinates(station) => {
                    *out = rtcm_station_to_c(station);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_station_coordinates",
                    "station coordinates",
                ),
            }
        },
    )
}

/// Copy a decoded 1007 / 1008 / 1033 antenna descriptor's scalar fields into
/// *out. Read the string fields with sidereon_rtcm_message_antenna_string.
///
/// Safety: messages is a live handle; out points to a
/// SidereonRtcmAntennaDescriptor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_antenna_descriptor(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmAntennaDescriptor,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_antenna_descriptor",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_antenna_descriptor",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_antenna_descriptor",
                messages,
                index
            ));
            match message {
                RtcmMessage::AntennaDescriptor(antenna) => {
                    *out = rtcm_antenna_to_c(antenna);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_antenna_descriptor",
                    "an antenna descriptor",
                ),
            }
        },
    )
}

/// Read an antenna-descriptor string field (selected by
/// SidereonRtcmAntennaStringField) into a caller buffer (not null-terminated).
/// An absent optional field reports *out_required 0 and writes nothing.
/// Variable-length output contract.
///
/// Safety: messages is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_antenna_string(
    messages: *const SidereonRtcmMessages,
    index: usize,
    field: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_antenna_string",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_antenna_string",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_antenna_string",
                messages,
                index
            ));
            let antenna = match message {
                RtcmMessage::AntennaDescriptor(antenna) => antenna,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_antenna_string",
                        "an antenna descriptor",
                    )
                }
            };
            let value: &str = match field {
                v if v == SidereonRtcmAntennaStringField::AntennaDescriptor as u32 => {
                    antenna.antenna_descriptor.as_str()
                }
                v if v == SidereonRtcmAntennaStringField::AntennaSerialNumber as u32 => {
                    antenna.antenna_serial_number.as_deref().unwrap_or("")
                }
                v if v == SidereonRtcmAntennaStringField::ReceiverType as u32 => {
                    antenna.receiver_type.as_deref().unwrap_or("")
                }
                v if v == SidereonRtcmAntennaStringField::ReceiverFirmwareVersion as u32 => {
                    antenna.receiver_firmware_version.as_deref().unwrap_or("")
                }
                v if v == SidereonRtcmAntennaStringField::ReceiverSerialNumber as u32 => {
                    antenna.receiver_serial_number.as_deref().unwrap_or("")
                }
                _ => {
                    set_last_error(
                        "sidereon_rtcm_message_antenna_string: invalid field code".to_string(),
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_antenna_string",
                "out",
                value.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy a decoded MSM observation message summary into *out. Read the cells with
/// sidereon_rtcm_message_msm_satellites and sidereon_rtcm_message_msm_signals.
///
/// Safety: messages is a live handle; out points to a SidereonRtcmMsmInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_msm_info(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmMsmInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_msm_info",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rtcm_message_msm_info", "out"));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_msm_info",
                messages,
                index
            ));
            match message {
                RtcmMessage::Msm(msm) => {
                    *out = SidereonRtcmMsmInfo {
                        message_number: msm.message_number,
                        system: gnss_system_to_c(msm.system),
                        kind: match msm.kind {
                            RtcmMsmKind::Msm4 => SidereonRtcmMsmKind::Msm4,
                            RtcmMsmKind::Msm7 => SidereonRtcmMsmKind::Msm7,
                        },
                        header: rtcm_msm_header_to_c(msm),
                        satellite_count: msm.satellites.len(),
                        signal_count: msm.signals.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_msm_info", "an MSM observation"),
            }
        },
    )
}

/// Copy an MSM message's per-satellite cells into a caller array.
/// Variable-length output contract.
///
/// Safety: messages is a live handle; out points to len writable
/// SidereonRtcmMsmSatellite or NULL when len is 0; out_written and out_required
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_msm_satellites(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmMsmSatellite,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_msm_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_msm_satellites",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_msm_satellites",
                messages,
                index
            ));
            let msm = match message {
                RtcmMessage::Msm(msm) => msm,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_msm_satellites",
                        "an MSM observation",
                    )
                }
            };
            let rows: Vec<SidereonRtcmMsmSatellite> =
                msm.satellites.iter().map(rtcm_msm_satellite_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_msm_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy an MSM message's per-cell signals into a caller array. Variable-length
/// output contract.
///
/// Safety: messages is a live handle; out points to len writable
/// SidereonRtcmMsmSignal or NULL when len is 0; out_written and out_required
/// point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_msm_signals(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmMsmSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_msm_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_msm_signals",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_msm_signals",
                messages,
                index
            ));
            let msm = match message {
                RtcmMessage::Msm(msm) => msm,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_msm_signals",
                        "an MSM observation",
                    )
                }
            };
            let rows: Vec<SidereonRtcmMsmSignal> =
                msm.signals.iter().map(rtcm_msm_signal_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_msm_signals",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the RINEX LLI bit constants derived from RTCM MSM fields: loss of lock
/// bit and half-cycle bit.
///
/// Safety: out_loss_of_lock and out_half_cycle point to uint8_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_lli_bits(
    out_loss_of_lock: *mut u8,
    out_half_cycle: *mut u8,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_lli_bits", SidereonStatus::Panic, || {
        let out_loss_of_lock = c_try!(require_out(
            out_loss_of_lock,
            "sidereon_rtcm_lli_bits",
            "out_loss_of_lock"
        ));
        let out_half_cycle = c_try!(require_out(
            out_half_cycle,
            "sidereon_rtcm_lli_bits",
            "out_half_cycle"
        ));
        *out_loss_of_lock = RTCM_LLI_LOSS_OF_LOCK;
        *out_half_cycle = RTCM_LLI_HALF_CYCLE;
        SidereonStatus::Ok
    })
}

/// Decode an MSM4/7 lock-time indicator to its minimum continuous-lock time.
/// kind is a SidereonRtcmMsmKind value encoded as uint32_t. Reserved or
/// out-of-range indicators return OK with *out_present false.
///
/// Safety: out_present points to bool storage; out_min_lock_time_ms points to
/// uint32_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_minimum_lock_time_ms(
    kind: u32,
    indicator: u16,
    out_present: *mut bool,
    out_min_lock_time_ms: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_minimum_lock_time_ms",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_rtcm_minimum_lock_time_ms",
                "out_present"
            ));
            *out_present = false;
            let out_min_lock_time_ms = c_try!(require_out(
                out_min_lock_time_ms,
                "sidereon_rtcm_minimum_lock_time_ms",
                "out_min_lock_time_ms"
            ));
            *out_min_lock_time_ms = 0;
            let kind = c_try!(rtcm_msm_kind_from_c_code(
                "sidereon_rtcm_minimum_lock_time_ms",
                "kind",
                kind
            ));
            if let Some(value) = core_rtcm::minimum_lock_time_ms(kind, indicator) {
                *out_present = true;
                *out_min_lock_time_ms = value;
            }
            SidereonStatus::Ok
        },
    )
}

/// Derive the RINEX LLI value for one MSM signal cell. Pass previous as NULL
/// when there is no previous observation for the cell. If
/// has_current_min_lock_time_ms is false, current_min_lock_time_ms is ignored.
///
/// Safety: previous is NULL or points to a SidereonRtcmPreviousLock; out_lli
/// points to uint8_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_derive_lli(
    previous: *const SidereonRtcmPreviousLock,
    has_current_min_lock_time_ms: bool,
    current_min_lock_time_ms: u32,
    half_cycle_ambiguity: bool,
    out_lli: *mut u8,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_derive_lli", SidereonStatus::Panic, || {
        let out_lli = c_try!(require_out(out_lli, "sidereon_rtcm_derive_lli", "out_lli"));
        *out_lli = 0;
        let previous = if previous.is_null() {
            None
        } else {
            let previous = c_try!(require_ref(
                previous,
                "sidereon_rtcm_derive_lli",
                "previous"
            ));
            Some(RtcmPreviousLock {
                min_lock_time_ms: previous
                    .has_min_lock_time_ms
                    .then_some(previous.min_lock_time_ms),
                elapsed_ms: previous.elapsed_ms,
            })
        };
        let current = has_current_min_lock_time_ms.then_some(current_min_lock_time_ms);
        *out_lli = core_rtcm::derive_lli(previous, current, half_cycle_ambiguity);
        SidereonStatus::Ok
    })
}

/// Compute elapsed milliseconds between two raw MSM epoch-time fields for one
/// constellation. system is a SidereonGnssSystem value encoded as uint32_t.
///
/// Safety: out_elapsed_ms points to uint64_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_msm_epoch_dt_ms(
    system: u32,
    previous_epoch_time: u32,
    current_epoch_time: u32,
    out_elapsed_ms: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_msm_epoch_dt_ms",
        SidereonStatus::Panic,
        || {
            let out_elapsed_ms = c_try!(require_out(
                out_elapsed_ms,
                "sidereon_rtcm_msm_epoch_dt_ms",
                "out_elapsed_ms"
            ));
            *out_elapsed_ms = 0;
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_rtcm_msm_epoch_dt_ms",
                "system",
                system
            ));
            *out_elapsed_ms =
                core_rtcm::msm_epoch_dt_ms(system, previous_epoch_time, current_epoch_time);
            SidereonStatus::Ok
        },
    )
}

/// Copy the RINEX 3 observation-code suffix for one MSM signal id. For example,
/// GPS signal id 2 returns "1C". Reserved signal ids report *out_required 0 and
/// write nothing. system is a SidereonGnssSystem value encoded as uint32_t.
/// Variable-length output contract.
///
/// Safety: out points to len writable bytes or NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_msm_signal_rinex_code(
    system: u32,
    signal_id: u8,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_msm_signal_rinex_code",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_msm_signal_rinex_code",
                out_written,
                out_required
            ));
            let system = c_try!(gnss_system_from_c_code(
                "sidereon_rtcm_msm_signal_rinex_code",
                "system",
                system
            ));
            let value = core_rtcm::msm_signal_rinex_code(system, signal_id).unwrap_or("");
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_msm_signal_rinex_code",
                "out",
                value.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Create a stateful RTCM MSM lock-time tracker for RINEX LLI derivation.
///
/// Safety: out_tracker points to storage for a SidereonRtcmLockTimeTracker*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_lock_time_tracker_new(
    out_tracker: *mut *mut SidereonRtcmLockTimeTracker,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_lock_time_tracker_new",
        SidereonStatus::Panic,
        || {
            let out_tracker = c_try!(require_out(
                out_tracker,
                "sidereon_rtcm_lock_time_tracker_new",
                "out_tracker"
            ));
            *out_tracker = ptr::null_mut();
            write_boxed_handle(
                out_tracker,
                SidereonRtcmLockTimeTracker {
                    tracker: RtcmLockTimeTracker::new(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Reset all per-cell lock history in a tracker.
///
/// Safety: tracker is a live handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_lock_time_tracker_reset(
    tracker: *mut SidereonRtcmLockTimeTracker,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_lock_time_tracker_reset",
        SidereonStatus::Panic,
        || {
            let tracker = c_try!(require_out(
                tracker,
                "sidereon_rtcm_lock_time_tracker_reset",
                "tracker"
            ));
            tracker.tracker.reset();
            SidereonStatus::Ok
        },
    )
}

/// Derive LLI rows for one decoded MSM message and advance tracker state.
/// Variable-length output contract.
///
/// Safety: tracker is a live handle; messages is a live message-list handle; out
/// points to len writable SidereonRtcmCellLli entries or NULL when len is 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_lock_time_tracker_observe(
    tracker: *mut SidereonRtcmLockTimeTracker,
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmCellLli,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_lock_time_tracker_observe",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_lock_time_tracker_observe",
                out_written,
                out_required
            ));
            let tracker = c_try!(require_out(
                tracker,
                "sidereon_rtcm_lock_time_tracker_observe",
                "tracker"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_lock_time_tracker_observe",
                messages,
                index
            ));
            let msm = match message {
                RtcmMessage::Msm(msm) => msm,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_lock_time_tracker_observe",
                        "an MSM observation",
                    )
                }
            };
            let rows: Vec<SidereonRtcmCellLli> = tracker
                .tracker
                .observe(msm)
                .iter()
                .map(rtcm_cell_lli_to_c)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_lock_time_tracker_observe",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a lock-time tracker handle. Passing NULL is a no-op.
///
/// Safety: tracker must be NULL or a live tracker handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_lock_time_tracker_free(
    tracker: *mut SidereonRtcmLockTimeTracker,
) {
    ffi_boundary("sidereon_rtcm_lock_time_tracker_free", (), || {
        free_boxed(tracker);
    });
}

/// Copy a decoded 1019 GPS broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a SidereonRtcmGpsEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_gps_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmGpsEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_gps_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_gps_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_gps_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::GpsEphemeris(eph) => {
                    *out = rtcm_gps_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_gps_ephemeris", "a GPS ephemeris"),
            }
        },
    )
}

/// Copy a decoded 1020 GLONASS broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a
/// SidereonRtcmGlonassEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_glonass_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmGlonassEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_glonass_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_glonass_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_glonass_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::GlonassEphemeris(eph) => {
                    *out = rtcm_glonass_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_glonass_ephemeris",
                    "a GLONASS ephemeris",
                ),
            }
        },
    )
}

/// Encode one decoded message back into its RTCM body (without the transport
/// frame). Variable-length output contract. Delegates to
/// sidereon_core::rtcm::Message::encode.
///
/// Safety: messages is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_encode(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_encode",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_encode",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_encode",
                messages,
                index
            ));
            let body = message.encode();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_encode",
                "out",
                &body,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Encode one decoded message into a complete RTCM transport frame (with a fresh
/// CRC-24Q). Variable-length output contract. Delegates to
/// sidereon_core::rtcm::Message::to_frame.
///
/// Safety: messages is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_to_frame(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_to_frame",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_to_frame",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_to_frame",
                messages,
                index
            ));
            let frame = match message.to_frame() {
                Ok(frame) => frame,
                Err(err) => return map_rtcm_error("sidereon_rtcm_message_to_frame", err),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_to_frame",
                "out",
                &frame,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a decoded RTCM message list. Passing NULL is a no-op.
///
/// Safety: messages must be a handle from sidereon_rtcm_decode_messages or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_messages_free(messages: *mut SidereonRtcmMessages) {
    free_boxed(messages);
}

/// Wrap a message body in an RTCM 3 transport frame with a fresh CRC-24Q.
/// Variable-length output contract. Delegates to
/// sidereon_core::rtcm::encode_frame.
///
/// Safety: body points to body_len readable bytes or NULL when body_len is 0;
/// out points to len writable bytes or NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_encode_frame(
    body: *const u8,
    body_len: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_encode_frame", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rtcm_encode_frame",
            out_written,
            out_required
        ));
        let body = c_try!(require_slice(
            body,
            body_len,
            "sidereon_rtcm_encode_frame",
            "body"
        ));
        let frame = match core_rtcm::encode_frame(body) {
            Ok(frame) => frame,
            Err(err) => return map_rtcm_error("sidereon_rtcm_encode_frame", err),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_rtcm_encode_frame",
            "out",
            &frame,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Decode the single RTCM 3 frame at the start of a buffer, copying its message
/// body into a caller buffer (variable-length contract) and reporting the total
/// frame length. Delegates to sidereon_core::rtcm::decode_frame.
///
/// Safety: bytes points to len_bytes readable bytes; out_body points to body_len
/// writable bytes or NULL when body_len is 0; out_body_written, out_body_required,
/// and out_frame_len point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_decode_frame(
    bytes: *const u8,
    len_bytes: usize,
    out_body: *mut u8,
    body_len: usize,
    out_body_written: *mut usize,
    out_body_required: *mut usize,
    out_frame_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_decode_frame", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rtcm_decode_frame",
            out_body_written,
            out_body_required
        ));
        let out_frame_len = c_try!(require_out(
            out_frame_len,
            "sidereon_rtcm_decode_frame",
            "out_frame_len"
        ));
        *out_frame_len = 0;
        let data = c_try!(require_slice(
            bytes,
            len_bytes,
            "sidereon_rtcm_decode_frame",
            "bytes"
        ));
        let frame = match core_rtcm::decode_frame(data) {
            Ok(frame) => frame,
            Err(err) => return map_rtcm_error("sidereon_rtcm_decode_frame", err),
        };
        *out_frame_len = frame.frame_len;
        c_try!(copy_prefix_to_c(
            "sidereon_rtcm_decode_frame",
            "out_body",
            frame.body,
            out_body,
            body_len,
            out_body_written,
            out_body_required,
        ));
        SidereonStatus::Ok
    })
}

/// Scan every CRC-valid RTCM 3 transport frame in a byte buffer (the forgiving
/// FrameScanner). On success writes a newly owned handle to *out_frames. Release
/// it with sidereon_rtcm_frames_free. Delegates to
/// sidereon_core::rtcm::FrameScanner.
///
/// Safety: bytes points to len readable bytes; out_frames points to a
/// SidereonRtcmFrames*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_scan_frames(
    bytes: *const u8,
    len: usize,
    out_frames: *mut *mut SidereonRtcmFrames,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_scan_frames", SidereonStatus::Panic, || {
        let out_frames = c_try!(require_out(
            out_frames,
            "sidereon_rtcm_scan_frames",
            "out_frames"
        ));
        *out_frames = ptr::null_mut();
        let data = c_try!(require_slice(
            bytes,
            len,
            "sidereon_rtcm_scan_frames",
            "bytes"
        ));
        let frames = core_rtcm::FrameScanner::new(data)
            .map(|frame| RtcmFrameRecord {
                body: frame.body.to_vec(),
                frame_len: frame.frame_len,
            })
            .collect();
        write_boxed_handle(out_frames, SidereonRtcmFrames { frames });
        SidereonStatus::Ok
    })
}

/// Number of frames in a scanned RTCM frame set.
///
/// Safety: frames is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_frames_count(
    frames: *const SidereonRtcmFrames,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_frames_count", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_count,
            "sidereon_rtcm_frames_count",
            "out_count"
        ));
        *out = 0;
        let handle = c_try!(require_ref(frames, "sidereon_rtcm_frames_count", "frames"));
        *out = handle.frames.len();
        SidereonStatus::Ok
    })
}

/// Total length in bytes (preamble, length, body, CRC) of one scanned frame.
///
/// Safety: frames is a live handle; out_frame_len points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_frame_len(
    frames: *const SidereonRtcmFrames,
    index: usize,
    out_frame_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_frame_len", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_frame_len,
            "sidereon_rtcm_frame_len",
            "out_frame_len"
        ));
        *out = 0;
        let handle = c_try!(require_ref(frames, "sidereon_rtcm_frame_len", "frames"));
        let frame = match handle.frames.get(index) {
            Some(frame) => frame,
            None => {
                set_last_error(format!(
                    "sidereon_rtcm_frame_len: index {index} out of range ({} frames)",
                    handle.frames.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        *out = frame.frame_len;
        SidereonStatus::Ok
    })
}

/// Copy one scanned frame's message body into a caller buffer (not the transport
/// frame). Variable-length output contract.
///
/// Safety: frames is a live handle; out points to len writable bytes or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_frame_body(
    frames: *const SidereonRtcmFrames,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_frame_body", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rtcm_frame_body",
            out_written,
            out_required
        ));
        let handle = c_try!(require_ref(frames, "sidereon_rtcm_frame_body", "frames"));
        let frame = match handle.frames.get(index) {
            Some(frame) => frame,
            None => {
                set_last_error(format!(
                    "sidereon_rtcm_frame_body: index {index} out of range ({} frames)",
                    handle.frames.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_rtcm_frame_body",
            "out",
            &frame.body,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release a scanned RTCM frame set. Passing NULL is a no-op.
///
/// Safety: frames must be a handle from sidereon_rtcm_scan_frames or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_frames_free(frames: *mut SidereonRtcmFrames) {
    free_boxed(frames);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_info(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out_info: *mut SidereonRtcmSsrInfo,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_info",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_info,
                "sidereon_rtcm_message_ssr_info",
                "out_info"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_info",
                messages,
                index
            ));
            match message {
                RtcmMessage::Ssr(ssr) => {
                    *out = rtcm_ssr_info_to_c(ssr);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_ssr_info", "an SSR message"),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_orbits(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrOrbitRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_orbits",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_orbits",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_orbits",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => return rtcm_wrong_kind("sidereon_rtcm_message_ssr_orbits", "an SSR message"),
            };
            let rows: Vec<SidereonRtcmSsrOrbitRecord> =
                ssr.orbit.iter().map(rtcm_ssr_orbit_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_ssr_orbits",
                "out",
                &rows,
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
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_clocks(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrClockRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_clocks",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_clocks",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_clocks",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => return rtcm_wrong_kind("sidereon_rtcm_message_ssr_clocks", "an SSR message"),
            };
            let rows: Vec<SidereonRtcmSsrClockRecord> =
                ssr.clock.iter().map(rtcm_ssr_clock_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_ssr_clocks",
                "out",
                &rows,
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
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_ura(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrUraRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_ura",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_ura",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_ura",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => return rtcm_wrong_kind("sidereon_rtcm_message_ssr_ura", "an SSR message"),
            };
            let rows: Vec<SidereonRtcmSsrUraRecord> = ssr
                .ura
                .iter()
                .map(|(satellite_id, ura_index)| SidereonRtcmSsrUraRecord {
                    satellite_id: *satellite_id,
                    ura_index: *ura_index,
                })
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_ssr_ura",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Build a 1005 / 1006 station antenna reference point message from fields and
/// wrap it in a single-element SidereonRtcmMessages handle. Release with
/// sidereon_rtcm_messages_free; encode it with sidereon_rtcm_message_encode or
/// sidereon_rtcm_message_to_frame (index 0).
///
/// Safety: station points to a SidereonRtcmStationCoordinates; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_station_coordinates(
    station: *const SidereonRtcmStationCoordinates,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_build_station_coordinates",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_station_coordinates",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let station = c_try!(require_ref(
                station,
                "sidereon_rtcm_build_station_coordinates",
                "station"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::StationCoordinates(rtcm_station_from_c(station)),
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a 1007 / 1008 / 1033 antenna or receiver descriptor message from fields
/// and wrap it in a single-element SidereonRtcmMessages handle. The optional
/// string arguments may be NULL when absent; supply them consistently with
/// `message_number` (1008/1033 carry the serial, 1033 carries the receiver
/// strings). Release with sidereon_rtcm_messages_free.
///
/// Safety: antenna_descriptor must be a valid C string; the optional strings are
/// C strings or NULL; out_messages points to a SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_antenna_descriptor(
    message_number: u16,
    reference_station_id: u16,
    antenna_setup_id: u8,
    antenna_descriptor: *const c_char,
    antenna_serial_number: *const c_char,
    receiver_type: *const c_char,
    receiver_firmware_version: *const c_char,
    receiver_serial_number: *const c_char,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_build_antenna_descriptor",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_antenna_descriptor",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let fn_name = "sidereon_rtcm_build_antenna_descriptor";
            let antenna_descriptor = c_try!(parse_bounded_c_string(
                fn_name,
                "antenna_descriptor",
                antenna_descriptor,
                MAX_RTCM_STRING_BYTES,
            ));
            let antenna_serial_number = c_try!(optional_bounded_c_string(
                fn_name,
                "antenna_serial_number",
                antenna_serial_number,
                MAX_RTCM_STRING_BYTES,
            ));
            let receiver_type = c_try!(optional_bounded_c_string(
                fn_name,
                "receiver_type",
                receiver_type,
                MAX_RTCM_STRING_BYTES,
            ));
            let receiver_firmware_version = c_try!(optional_bounded_c_string(
                fn_name,
                "receiver_firmware_version",
                receiver_firmware_version,
                MAX_RTCM_STRING_BYTES,
            ));
            let receiver_serial_number = c_try!(optional_bounded_c_string(
                fn_name,
                "receiver_serial_number",
                receiver_serial_number,
                MAX_RTCM_STRING_BYTES,
            ));
            let descriptor = RtcmAntennaDescriptor {
                message_number,
                reference_station_id,
                antenna_descriptor,
                antenna_setup_id,
                antenna_serial_number,
                receiver_type,
                receiver_firmware_version,
                receiver_serial_number,
            };
            rtcm_build(out_messages, RtcmMessage::AntennaDescriptor(descriptor));
            SidereonStatus::Ok
        },
    )
}

/// Build an MSM4 / MSM7 observation message from a header summary plus the
/// per-satellite and per-cell arrays, and wrap it in a single-element
/// SidereonRtcmMessages handle. The satellite-mask and signal-mask are
/// reconstructed from the satellite ids and cell signal ids on encode. Release
/// with sidereon_rtcm_messages_free.
///
/// Safety: info points to a SidereonRtcmMsmInfo; satellites/signals point to
/// their counts of cells (or NULL when 0); out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_msm(
    info: *const SidereonRtcmMsmInfo,
    satellites: *const SidereonRtcmMsmSatellite,
    satellite_count: usize,
    signals: *const SidereonRtcmMsmSignal,
    signal_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_build_msm", SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(
            out_messages,
            "sidereon_rtcm_build_msm",
            "out_messages"
        ));
        *out_messages = ptr::null_mut();
        let info = c_try!(require_ref(info, "sidereon_rtcm_build_msm", "info"));
        let system = c_try!(gnss_system_from_c_code(
            "sidereon_rtcm_build_msm",
            "info.system",
            info.system as u32,
        ));
        let kind = match info.kind {
            SidereonRtcmMsmKind::Msm4 => RtcmMsmKind::Msm4,
            SidereonRtcmMsmKind::Msm7 => RtcmMsmKind::Msm7,
        };
        let raw_satellites = c_try!(require_slice(
            satellites,
            satellite_count,
            "sidereon_rtcm_build_msm",
            "satellites"
        ));
        let raw_signals = c_try!(require_slice(
            signals,
            signal_count,
            "sidereon_rtcm_build_msm",
            "signals"
        ));
        let message = RtcmMsmMessage {
            message_number: info.message_number,
            system,
            kind,
            header: rtcm_msm_header_from_c(&info.header),
            satellites: raw_satellites
                .iter()
                .map(rtcm_msm_satellite_from_c)
                .collect(),
            signals: raw_signals.iter().map(rtcm_msm_signal_from_c).collect(),
        };
        rtcm_build(out_messages, RtcmMessage::Msm(message));
        SidereonStatus::Ok
    })
}

/// Build a 1019 GPS broadcast ephemeris message from raw transmitted-integer
/// fields and wrap it in a single-element SidereonRtcmMessages handle. Release
/// with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmGpsEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_gps_ephemeris(
    eph: *const SidereonRtcmGpsEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_build_gps_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_gps_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(eph, "sidereon_rtcm_build_gps_ephemeris", "eph"));
            rtcm_build(
                out_messages,
                RtcmMessage::GpsEphemeris(rtcm_gps_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a 1020 GLONASS broadcast ephemeris message from raw transmitted-integer
/// fields and wrap it in a single-element SidereonRtcmMessages handle. Release
/// with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmGlonassEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_glonass_ephemeris(
    eph: *const SidereonRtcmGlonassEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_build_glonass_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_glonass_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(
                eph,
                "sidereon_rtcm_build_glonass_ephemeris",
                "eph"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::GlonassEphemeris(rtcm_glonass_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

// ============================================================================
// Universal-parity additions: capabilities the core exposes that this binding
// had not yet surfaced. Every function below is a thin extern-C wrapper that
// marshals C input into the cited sidereon-core / sidereon type, calls the
// reference entry point, and copies the result back. No modeling logic lives
// here.

fn map_rtcm_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        _ => SidereonStatus::Solve,
    }
}

fn rtcm_wrong_kind(fn_name: &str, expected: &str) -> SidereonStatus {
    set_last_error(format!("{fn_name}: message is not {expected}"));
    SidereonStatus::InvalidArgument
}

fn rtcm_message_kind_of(message: &RtcmMessage) -> SidereonRtcmMessageKind {
    match message {
        RtcmMessage::Msm(_) => SidereonRtcmMessageKind::Msm,
        RtcmMessage::StationCoordinates(_) => SidereonRtcmMessageKind::StationCoordinates,
        RtcmMessage::AntennaDescriptor(_) => SidereonRtcmMessageKind::AntennaDescriptor,
        RtcmMessage::GpsEphemeris(_) => SidereonRtcmMessageKind::GpsEphemeris,
        RtcmMessage::GlonassEphemeris(_) => SidereonRtcmMessageKind::GlonassEphemeris,
        RtcmMessage::Ssr(_) => SidereonRtcmMessageKind::Ssr,
        RtcmMessage::Unsupported(_) => SidereonRtcmMessageKind::Unsupported,
    }
}

fn rtcm_station_to_c(station: &RtcmStationCoordinates) -> SidereonRtcmStationCoordinates {
    SidereonRtcmStationCoordinates {
        message_number: station.message_number,
        reference_station_id: station.reference_station_id,
        itrf_realization_year: station.itrf_realization_year,
        gps_indicator: station.gps_indicator,
        glonass_indicator: station.glonass_indicator,
        galileo_indicator: station.galileo_indicator,
        reference_station_indicator: station.reference_station_indicator,
        single_receiver_oscillator: station.single_receiver_oscillator,
        reserved: station.reserved,
        quarter_cycle_indicator: station.quarter_cycle_indicator,
        ecef_x: station.ecef_x,
        ecef_y: station.ecef_y,
        ecef_z: station.ecef_z,
        x_m: station.x_m(),
        y_m: station.y_m(),
        z_m: station.z_m(),
        has_antenna_height: station.antenna_height.is_some(),
        antenna_height: station.antenna_height.unwrap_or(0),
        antenna_height_m: station.antenna_height_m().unwrap_or(0.0),
    }
}

fn rtcm_antenna_to_c(antenna: &RtcmAntennaDescriptor) -> SidereonRtcmAntennaDescriptor {
    SidereonRtcmAntennaDescriptor {
        message_number: antenna.message_number,
        reference_station_id: antenna.reference_station_id,
        antenna_setup_id: antenna.antenna_setup_id,
        has_antenna_serial_number: antenna.antenna_serial_number.is_some(),
        has_receiver_type: antenna.receiver_type.is_some(),
        has_receiver_firmware_version: antenna.receiver_firmware_version.is_some(),
        has_receiver_serial_number: antenna.receiver_serial_number.is_some(),
    }
}

fn rtcm_msm_header_to_c(message: &RtcmMsmMessage) -> SidereonRtcmMsmHeader {
    let h = &message.header;
    SidereonRtcmMsmHeader {
        reference_station_id: h.reference_station_id,
        epoch_time: h.epoch_time,
        multiple_message: h.multiple_message,
        iods: h.iods,
        reserved: h.reserved,
        clock_steering: h.clock_steering,
        external_clock: h.external_clock,
        divergence_free_smoothing: h.divergence_free_smoothing,
        smoothing_interval: h.smoothing_interval,
    }
}

fn rtcm_msm_satellite_to_c(satellite: &RtcmMsmSatellite) -> SidereonRtcmMsmSatellite {
    SidereonRtcmMsmSatellite {
        id: satellite.id,
        rough_range_ms: satellite.rough_range_ms,
        rough_range_mod1: satellite.rough_range_mod1,
        has_extended_info: satellite.extended_info.is_some(),
        extended_info: satellite.extended_info.unwrap_or(0),
        has_rough_phase_range_rate: satellite.rough_phase_range_rate_m_s.is_some(),
        rough_phase_range_rate_m_s: satellite.rough_phase_range_rate_m_s.unwrap_or(0),
    }
}

fn rtcm_msm_signal_to_c(signal: &RtcmMsmSignal) -> SidereonRtcmMsmSignal {
    SidereonRtcmMsmSignal {
        satellite_id: signal.satellite_id,
        signal_id: signal.signal_id,
        fine_pseudorange: signal.fine_pseudorange,
        fine_phase_range: signal.fine_phase_range,
        lock_time_indicator: signal.lock_time_indicator,
        half_cycle_ambiguity: signal.half_cycle_ambiguity,
        cnr: signal.cnr,
        has_fine_phase_range_rate: signal.fine_phase_range_rate.is_some(),
        fine_phase_range_rate: signal.fine_phase_range_rate.unwrap_or(0),
    }
}

fn rtcm_msm_kind_from_c_code(
    fn_name: &str,
    arg_name: &str,
    kind: u32,
) -> Result<RtcmMsmKind, SidereonStatus> {
    match kind {
        value if value == SidereonRtcmMsmKind::Msm4 as u32 => Ok(RtcmMsmKind::Msm4),
        value if value == SidereonRtcmMsmKind::Msm7 as u32 => Ok(RtcmMsmKind::Msm7),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} RTCM MSM kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn rtcm_frame_skip_to_c(skip: &core_rtcm::FrameSkip) -> SidereonRtcmFrameSkip {
    SidereonRtcmFrameSkip {
        offset: skip.offset,
        has_message_number: skip.message_number.is_some(),
        message_number: skip.message_number.unwrap_or(0),
        reason: match skip.reason {
            core_rtcm::FrameSkipReason::Truncated => SidereonRtcmFrameSkipReason::Truncated,
            core_rtcm::FrameSkipReason::Malformed(_) => SidereonRtcmFrameSkipReason::Malformed,
        },
    }
}

fn rtcm_cell_lli_to_c(cell: &core_rtcm::CellLli) -> SidereonRtcmCellLli {
    SidereonRtcmCellLli {
        satellite_id: cell.satellite_id,
        signal_id: cell.signal_id,
        lli: cell.lli,
        has_min_lock_time_ms: cell.min_lock_time_ms.is_some(),
        min_lock_time_ms: cell.min_lock_time_ms.unwrap_or(0),
    }
}

fn rtcm_gps_ephemeris_to_c(eph: &RtcmGpsEphemeris) -> SidereonRtcmGpsEphemeris {
    SidereonRtcmGpsEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        sv_accuracy: eph.sv_accuracy,
        code_on_l2: eph.code_on_l2,
        idot: eph.idot,
        iode: eph.iode,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        iodc: eph.iodc,
        c_rs: eph.c_rs,
        delta_n: eph.delta_n,
        m0: eph.m0,
        c_uc: eph.c_uc,
        eccentricity: eph.eccentricity,
        c_us: eph.c_us,
        sqrt_a: eph.sqrt_a,
        t_oe: eph.t_oe,
        c_ic: eph.c_ic,
        omega0: eph.omega0,
        c_is: eph.c_is,
        i0: eph.i0,
        c_rc: eph.c_rc,
        omega: eph.omega,
        omega_dot: eph.omega_dot,
        t_gd: eph.t_gd,
        sv_health: eph.sv_health,
        l2_p_data_flag: eph.l2_p_data_flag,
        fit_interval: eph.fit_interval,
    }
}

fn rtcm_glonass_ephemeris_to_c(eph: &RtcmGlonassEphemeris) -> SidereonRtcmGlonassEphemeris {
    SidereonRtcmGlonassEphemeris {
        satellite_id: eph.satellite_id,
        frequency_channel: eph.frequency_channel,
        almanac_health: eph.almanac_health,
        almanac_health_availability: eph.almanac_health_availability,
        p1: eph.p1,
        t_k: eph.t_k,
        b_n_msb: eph.b_n_msb,
        p2: eph.p2,
        t_b: eph.t_b,
        xn_dot: eph.xn_dot,
        xn: eph.xn,
        xn_dot_dot: eph.xn_dot_dot,
        yn_dot: eph.yn_dot,
        yn: eph.yn,
        yn_dot_dot: eph.yn_dot_dot,
        zn_dot: eph.zn_dot,
        zn: eph.zn,
        zn_dot_dot: eph.zn_dot_dot,
        p3: eph.p3,
        gamma_n: eph.gamma_n,
        m_p: eph.m_p,
        m_l_n_third: eph.m_l_n_third,
        tau_n: eph.tau_n,
        delta_tau_n: eph.delta_tau_n,
        e_n: eph.e_n,
        m_p4: eph.m_p4,
        m_f_t: eph.m_f_t,
        m_n_t: eph.m_n_t,
        m_m: eph.m_m,
        additional_data_available: eph.additional_data_available,
        n_a: eph.n_a,
        tau_c: eph.tau_c,
        m_n4: eph.m_n4,
        m_tau_gps: eph.m_tau_gps,
        m_l_n_fifth: eph.m_l_n_fifth,
        reserved: eph.reserved,
    }
}

fn rtcm_ssr_info_to_c(message: &RtcmSsrMessage) -> SidereonRtcmSsrInfo {
    SidereonRtcmSsrInfo {
        message_number: message.message_number,
        system: gnss_system_to_c(message.system),
        kind: rtcm_ssr_kind_to_c(message.kind),
        header: rtcm_ssr_header_to_c(&message.header),
        orbit_count: message.orbit.len(),
        clock_count: message.clock.len(),
        ura_count: message.ura.len(),
        code_bias_count: message.code_bias.len(),
        phase_bias_count: message.phase_bias.len(),
    }
}

fn rtcm_ssr_orbit_to_c(record: &RtcmSsrOrbitRecord) -> SidereonRtcmSsrOrbitRecord {
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

fn rtcm_ssr_clock_to_c(record: &RtcmSsrClockRecord) -> SidereonRtcmSsrClockRecord {
    SidereonRtcmSsrClockRecord {
        satellite_id: record.satellite_id,
        c0: record.c0,
        c1: record.c1,
        c2: record.c2,
    }
}

fn rtcm_station_from_c(station: &SidereonRtcmStationCoordinates) -> RtcmStationCoordinates {
    RtcmStationCoordinates {
        message_number: station.message_number,
        reference_station_id: station.reference_station_id,
        itrf_realization_year: station.itrf_realization_year,
        gps_indicator: station.gps_indicator,
        glonass_indicator: station.glonass_indicator,
        galileo_indicator: station.galileo_indicator,
        reference_station_indicator: station.reference_station_indicator,
        ecef_x: station.ecef_x,
        single_receiver_oscillator: station.single_receiver_oscillator,
        reserved: station.reserved,
        ecef_y: station.ecef_y,
        quarter_cycle_indicator: station.quarter_cycle_indicator,
        ecef_z: station.ecef_z,
        antenna_height: station.has_antenna_height.then_some(station.antenna_height),
    }
}

fn rtcm_msm_header_from_c(header: &SidereonRtcmMsmHeader) -> core_rtcm::MsmHeader {
    core_rtcm::MsmHeader {
        reference_station_id: header.reference_station_id,
        epoch_time: header.epoch_time,
        multiple_message: header.multiple_message,
        iods: header.iods,
        reserved: header.reserved,
        clock_steering: header.clock_steering,
        external_clock: header.external_clock,
        divergence_free_smoothing: header.divergence_free_smoothing,
        smoothing_interval: header.smoothing_interval,
    }
}

fn rtcm_msm_satellite_from_c(satellite: &SidereonRtcmMsmSatellite) -> RtcmMsmSatellite {
    RtcmMsmSatellite {
        id: satellite.id,
        rough_range_ms: satellite.rough_range_ms,
        rough_range_mod1: satellite.rough_range_mod1,
        extended_info: satellite
            .has_extended_info
            .then_some(satellite.extended_info),
        rough_phase_range_rate_m_s: satellite
            .has_rough_phase_range_rate
            .then_some(satellite.rough_phase_range_rate_m_s),
    }
}

fn rtcm_msm_signal_from_c(signal: &SidereonRtcmMsmSignal) -> RtcmMsmSignal {
    RtcmMsmSignal {
        satellite_id: signal.satellite_id,
        signal_id: signal.signal_id,
        fine_pseudorange: signal.fine_pseudorange,
        fine_phase_range: signal.fine_phase_range,
        lock_time_indicator: signal.lock_time_indicator,
        half_cycle_ambiguity: signal.half_cycle_ambiguity,
        cnr: signal.cnr,
        fine_phase_range_rate: signal
            .has_fine_phase_range_rate
            .then_some(signal.fine_phase_range_rate),
    }
}

fn rtcm_gps_ephemeris_from_c(eph: &SidereonRtcmGpsEphemeris) -> RtcmGpsEphemeris {
    RtcmGpsEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        sv_accuracy: eph.sv_accuracy,
        code_on_l2: eph.code_on_l2,
        idot: eph.idot,
        iode: eph.iode,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        iodc: eph.iodc,
        c_rs: eph.c_rs,
        delta_n: eph.delta_n,
        m0: eph.m0,
        c_uc: eph.c_uc,
        eccentricity: eph.eccentricity,
        c_us: eph.c_us,
        sqrt_a: eph.sqrt_a,
        t_oe: eph.t_oe,
        c_ic: eph.c_ic,
        omega0: eph.omega0,
        c_is: eph.c_is,
        i0: eph.i0,
        c_rc: eph.c_rc,
        omega: eph.omega,
        omega_dot: eph.omega_dot,
        t_gd: eph.t_gd,
        sv_health: eph.sv_health,
        l2_p_data_flag: eph.l2_p_data_flag,
        fit_interval: eph.fit_interval,
    }
}

fn rtcm_glonass_ephemeris_from_c(eph: &SidereonRtcmGlonassEphemeris) -> RtcmGlonassEphemeris {
    RtcmGlonassEphemeris {
        satellite_id: eph.satellite_id,
        frequency_channel: eph.frequency_channel,
        almanac_health: eph.almanac_health,
        almanac_health_availability: eph.almanac_health_availability,
        p1: eph.p1,
        t_k: eph.t_k,
        b_n_msb: eph.b_n_msb,
        p2: eph.p2,
        t_b: eph.t_b,
        xn_dot: eph.xn_dot,
        xn: eph.xn,
        xn_dot_dot: eph.xn_dot_dot,
        yn_dot: eph.yn_dot,
        yn: eph.yn,
        yn_dot_dot: eph.yn_dot_dot,
        zn_dot: eph.zn_dot,
        zn: eph.zn,
        zn_dot_dot: eph.zn_dot_dot,
        p3: eph.p3,
        gamma_n: eph.gamma_n,
        m_p: eph.m_p,
        m_l_n_third: eph.m_l_n_third,
        tau_n: eph.tau_n,
        delta_tau_n: eph.delta_tau_n,
        e_n: eph.e_n,
        m_p4: eph.m_p4,
        m_f_t: eph.m_f_t,
        m_n_t: eph.m_n_t,
        m_m: eph.m_m,
        additional_data_available: eph.additional_data_available,
        n_a: eph.n_a,
        tau_c: eph.tau_c,
        m_n4: eph.m_n4,
        m_tau_gps: eph.m_tau_gps,
        m_l_n_fifth: eph.m_l_n_fifth,
        reserved: eph.reserved,
    }
}

unsafe fn rtcm_build(out_messages: &mut *mut SidereonRtcmMessages, message: RtcmMessage) {
    *out_messages = ptr::null_mut();
    write_boxed_handle(
        out_messages,
        SidereonRtcmMessages {
            messages: vec![message],
        },
    );
}

fn rtcm_ssr_kind_to_c(kind: RtcmSsrKind) -> SidereonRtcmSsrKind {
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

fn rtcm_ssr_header_to_c(header: &RtcmSsrHeader) -> SidereonRtcmSsrHeader {
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
