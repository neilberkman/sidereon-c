use super::*;

mod retention;
pub use retention::*;

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
    /// A 1042 BeiDou broadcast ephemeris.
    BeidouEphemeris = 7,
    /// A 1044 QZSS broadcast ephemeris.
    QzssEphemeris = 8,
    /// A 1045 Galileo F/NAV broadcast ephemeris.
    GalileoFnavEphemeris = 9,
    /// A 1046 Galileo I/NAV broadcast ephemeris.
    GalileoInavEphemeris = 10,
    LegacyObservations = 11,
    SystemParameters = 12,
    Text = 13,
    Network = 14,
    Transformation = 15,
    GlonassCodePhaseBiases = 16,
    NavicEphemeris = 17,
    SsrVtec = 18,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Category reported by the thread-local structured RTCM/SBAS error accessors.
pub enum SidereonRtcmErrorClass {
    /// No typed error is recorded for this thread.
    None = 0,
    /// The last typed failure was RTCM encoding.
    Encode = 1,
    /// The last typed failure was RTCM conversion.
    Conversion = 2,
    /// The last operation recorded an unclassified error.
    Other = 3,
    /// The last typed failure was SBAS encoding.
    SbasEncode = 4,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Stable SBAS encode error discriminants reported in `SidereonRtcmErrorInfo.kind`.
pub enum SidereonSbasEncodeErrorKind {
    Unknown = 0,
    /// `SbasEncodeError::FieldOutOfRange`.
    FieldOutOfRange = 1,
    /// `SbasEncodeError::UnrecognizedPreamble`.
    UnrecognizedPreamble = 2,
    /// `SbasEncodeError::MessageType`.
    MessageType = 3,
    /// `SbasEncodeError::RawPayload`.
    RawPayload = 4,
    /// `SbasEncodeError::ReservedLayout`.
    ReservedLayout = 5,
    /// `SbasEncodeError::LongTermRecordCount`.
    LongTermRecordCount = 6,
    /// `SbasEncodeError::LongTermFieldNotCarried`.
    LongTermFieldNotCarried = 7,
    /// `SbasEncodeError::LongTermMissingTimeOfDay`.
    LongTermMissingTimeOfDay = 8,
    /// `SbasEncodeError::PadBits`.
    PadBits = 9,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmEncodeErrorKind {
    FieldOutOfRange = 1,
    NegativeZeroWithValue = 2,
    NegativeZeroMask = 3,
    MessageNumber = 4,
    FieldPresence = 5,
    SatelliteFieldPresence = 6,
    CountMismatch = 7,
    ValueOutOfRange = 8,
    NonLatin1Character = 9,
    SatelliteIdOutOfRange = 10,
    SsrSatelliteIdOutOfRange = 11,
    SsrRecordsNotCarried = 12,
    SsrCombinedRecordCounts = 13,
    SsrCombinedSatelliteMismatch = 14,
    SsrHighRateClockTerms = 15,
    SsrSatelliteCount = 16,
    MsmMask = 17,
    MsmOptional = 18,
    TrailingZeroBits = 19,
    StrictDeparture = 20,
    UnsupportedBodyTooShort = 21,
    UnsupportedBodyNumber = 22,
    UnsupportedDecodedNumber = 23,
    FrameBodyTooLong = 24,
    FrameReservedOutOfRange = 25,
    Other = 255,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmConversionErrorKind {
    SatelliteIdOutOfRange = 1,
    InvalidSatellite = 2,
    SbasPrnOutsideWindow = 3,
    NoLnavRecord = 4,
    WeekMismatch = 5,
    NavicWeekMismatch = 6,
    TimeNotRepresentable = 7,
    GalileoWeekOverflow = 8,
    SisaSpare = 9,
    SisaNoPrediction = 10,
    UraOutOfRange = 11,
    UraNoPrediction = 12,
    FitInterval = 13,
    VtecEvaluation = 14,
    Other = 255,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Class and stable variant discriminant for a thread-local typed error.
/// `kind` uses [`SidereonSbasEncodeErrorKind`] when `class` is `SbasEncode`.
pub struct SidereonRtcmErrorInfo {
    pub class: SidereonRtcmErrorClass,
    pub kind: u32,
    pub payload_len: usize,
}

thread_local! {
    static LAST_RTCM_TYPED_ERROR: std::cell::RefCell<Option<(SidereonRtcmErrorClass, u32, String)>> = const { std::cell::RefCell::new(None) };
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
    Msm1 = 2,
    Msm2 = 3,
    Msm3 = 4,
    Msm5 = 5,
    Msm6 = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrVtecInfo {
    pub message_number: u16,
    pub has_igs_ssr_version: bool,
    pub igs_ssr_version: u8,
    pub epoch_time_s: u32,
    pub update_interval: u8,
    pub multiple_message: bool,
    pub iod_ssr: u8,
    pub provider_id: u16,
    pub solution_id: u8,
    pub quality_indicator: u16,
    pub layer_count: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSsrVtecLayer {
    pub height: u8,
    pub degree: u8,
    pub order: u8,
    pub cosine_count: usize,
    pub sine_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmLegacyHeader {
    pub message_number: u16,
    pub reference_station_id: u16,
    pub epoch_time: u32,
    pub synchronous_gnss: bool,
    pub satellite_count: u8,
    pub divergence_free_smoothing: bool,
    pub smoothing_interval: u8,
    pub satellite_records: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmLegacyL1 {
    pub code_indicator: bool,
    pub pseudorange: u32,
    pub phase_range_minus_pseudorange: i32,
    pub lock_time_indicator: u8,
    pub has_pseudorange_modulus_ambiguity: bool,
    pub pseudorange_modulus_ambiguity: u8,
    pub has_cnr: bool,
    pub cnr: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmLegacyL2 {
    pub code_indicator: u8,
    pub pseudorange_difference: i16,
    pub phase_range_minus_l1_pseudorange: i32,
    pub lock_time_indicator: u8,
    pub has_cnr: bool,
    pub cnr: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmLegacySatellite {
    pub satellite_id: u8,
    pub has_frequency_channel: bool,
    pub frequency_channel: u8,
    pub l1: SidereonRtcmLegacyL1,
    pub has_l2: bool,
    pub l2: SidereonRtcmLegacyL2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmSystemParameters {
    pub reference_station_id: u16,
    pub mjd: u16,
    pub seconds_of_day: u32,
    pub announcement_count: u8,
    pub leap_seconds: u8,
    pub announcements: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMessageAnnouncement {
    pub message_number: u16,
    pub synchronous: bool,
    pub interval: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmTextMessage {
    pub reference_station_id: u16,
    pub mjd: u16,
    pub seconds_of_day: u32,
    pub character_count: u8,
    pub code_unit_count: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SidereonRtcmDeparture {
    pub kind: u32,
    pub message_number: u16,
    pub reserved: u8,
    pub layer_index: usize,
    pub degree: u8,
    pub order: u8,
    pub declared: usize,
    pub read: usize,
    pub cells: usize,
    pub bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNetworkAuxiliaryStation {
    pub network_id: u8,
    pub subnetwork_id: u8,
    pub auxiliary_station_count: u8,
    pub master_station_id: u16,
    pub auxiliary_station_id: u16,
    pub delta_latitude: i32,
    pub delta_longitude: i32,
    pub delta_height: i32,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmPhysicalReferenceStation {
    pub non_physical_station_id: u16,
    pub physical_station_id: u16,
    pub itrf_realization_year: u8,
    pub ecef_x: i64,
    pub ecef_y: i64,
    pub ecef_z: i64,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmGlonassCodePhaseBiases {
    pub reference_station_id: u16,
    pub aligned: bool,
    pub reserved: u8,
    pub has_l1_ca: bool,
    pub l1_ca: i16,
    pub has_l1_p: bool,
    pub l1_p: i16,
    pub has_l2_ca: bool,
    pub l2_ca: i16,
    pub has_l2_p: bool,
    pub l2_p: i16,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNetworkDifference {
    pub satellite_id: u8,
    pub ambiguity_status: u8,
    pub non_sync_count: u8,
    pub has_geometric: bool,
    pub geometric: i32,
    pub has_iod: bool,
    pub iod: u8,
    pub has_ionospheric: bool,
    pub ionospheric: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNetworkDifferences {
    pub message_number: u16,
    pub network_id: u8,
    pub subnetwork_id: u8,
    pub epoch_time: u32,
    pub multiple_message: bool,
    pub master_station_id: u16,
    pub auxiliary_station_id: u16,
    pub satellite_count: u8,
    pub satellite_records: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNetworkResidual {
    pub satellite_id: u8,
    pub s_oc: u8,
    pub s_od: u16,
    pub s_oh: u8,
    pub s_lc: u16,
    pub s_ld: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNetworkResiduals {
    pub message_number: u16,
    pub epoch_time: u32,
    pub reference_station_id: u16,
    pub reference_station_count: u8,
    pub satellite_count: u8,
    pub satellite_records: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmFkpGradient {
    pub satellite_id: u8,
    pub iod: u8,
    pub geometric_north: i16,
    pub geometric_east: i16,
    pub ionospheric_north: i16,
    pub ionospheric_east: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmFkpGradients {
    pub message_number: u16,
    pub reference_station_id: u16,
    pub epoch_time: u32,
    pub satellite_count: u8,
    pub satellite_records: usize,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SidereonRtcmGridResidual {
    pub horizontal_1: i16,
    pub horizontal_2: i16,
    pub height: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmResidualGrid {
    pub message_number: u16,
    pub system_id: u8,
    pub horizontal_shift: bool,
    pub vertical_shift: bool,
    pub origin_1: i32,
    pub origin_2: i32,
    pub extension_1: u16,
    pub extension_2: u16,
    pub mean_offset_1: i16,
    pub mean_offset_2: i16,
    pub mean_height_offset: i16,
    pub residuals: [SidereonRtcmGridResidual; 16],
    pub horizontal_interpolation: u8,
    pub vertical_interpolation: u8,
    pub horizontal_quality: u8,
    pub vertical_quality: u8,
    pub mjd: u16,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmProjection {
    pub message_number: u16,
    pub system_id: u8,
    pub projection_type: u8,
    pub rectification: bool,
    pub latitude: i64,
    pub longitude: i64,
    pub standard_parallel_1: i64,
    pub standard_parallel_2: i64,
    pub azimuth: u64,
    pub rectified_to_skew: i32,
    pub add_scale: u32,
    pub easting: u64,
    pub northing: i64,
    pub trailing_bit_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmHelmertTransformation {
    pub message_number: u16,
    pub source_name: [u8; 31],
    pub source_name_len: u8,
    pub target_name: [u8; 31],
    pub target_name_len: u8,
    pub system_id: u8,
    pub utilized_messages: u16,
    pub plate_number: u8,
    pub computation_indicator: u8,
    pub height_indicator: u8,
    pub validity_latitude: i32,
    pub validity_longitude: i32,
    pub validity_extension_latitude: u16,
    pub validity_extension_longitude: u16,
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
    pub r1: i32,
    pub r2: i32,
    pub r3: i32,
    pub ds: i32,
    pub has_rotation_point: bool,
    pub rotation_point_x: i64,
    pub rotation_point_y: i64,
    pub rotation_point_z: i64,
    pub add_as: u32,
    pub add_bs: u32,
    pub add_at: u32,
    pub add_bt: u32,
    pub horizontal_quality: u8,
    pub vertical_quality: u8,
    pub trailing_bit_count: usize,
}

/// Why a CRC-valid RTCM frame could not be decoded into the message IR.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmFrameSkipReason {
    /// The body ended before all required fields of its recognized type.
    Truncated = 0,
    /// The body is internally inconsistent for its recognized type.
    Malformed = 1,
    /// The frame or body departs from the RTCM 3 format and was refused under
    /// the strict policy. sidereon_rtcm_stream_diagnostics_skipped_frame_message
    /// names the departure.
    Departure = 2,
}

/// How the RTCM reader treats a frame that departs from the format. Mirrors
/// sidereon_core::rtcm::RtcmPolicy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRtcmPolicy {
    /// Refuse the first departure.
    Strict = 0,
    /// Read the frame and report each departure.
    Lenient = 1,
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
    /// The constellation, a SidereonGnssSystem value; an unknown value is
    /// refused on input.
    pub system: u32,
    /// The MSM variant, a SidereonRtcmMsmKind value; an unknown value is
    /// refused on input.
    pub kind: u32,
    /// Common MSM header.
    pub header: SidereonRtcmMsmHeader,
    /// Number of active satellites.
    pub satellite_count: usize,
    /// Number of active signal cells.
    pub signal_count: usize,
    /// The signal mask (DF395) as transmitted: bit `32 - id` is set for each
    /// signal id the message lists. A listed signal may have no cell. When
    /// building a message, zero builds the mask from the cells' signal ids.
    pub signal_mask: u32,
}

/// Per-satellite MSM data, mirroring sidereon_core::rtcm::MsmSatellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmMsmSatellite {
    /// Satellite id (1-based satellite-mask index).
    pub id: u8,
    pub has_rough_range_ms: bool,
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
    pub has_fine_pseudorange: bool,
    /// Fine pseudorange (raw integer, scale per MSM variant).
    pub fine_pseudorange: i32,
    pub has_fine_phase_range: bool,
    /// Fine phase range (raw integer, scale per MSM variant).
    pub fine_phase_range: i32,
    pub has_lock_time_indicator: bool,
    /// Phase-range lock-time indicator.
    pub lock_time_indicator: u16,
    pub has_half_cycle_ambiguity: bool,
    /// Half-cycle ambiguity indicator.
    pub half_cycle_ambiguity: bool,
    pub has_cnr: bool,
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

/// A decoded 1045 Galileo F/NAV broadcast ephemeris, mirroring
/// sidereon_core::rtcm::GalileoFnavEphemeris. Every field is the raw
/// transmitted integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmGalileoFnavEphemeris {
    pub satellite_id: u8,
    pub week_number: u16,
    pub iod_nav: u16,
    pub sisa: u8,
    pub idot: i32,
    pub t_oc: u16,
    pub a_f2: i16,
    pub a_f1: i32,
    pub a_f0: i64,
    pub c_rs: i32,
    pub delta_n: i32,
    pub m0: i64,
    pub c_uc: i32,
    pub eccentricity: u64,
    pub c_us: i32,
    pub sqrt_a: u64,
    pub t_oe: u16,
    pub c_ic: i32,
    pub omega0: i64,
    pub c_is: i32,
    pub i0: i64,
    pub c_rc: i32,
    pub omega: i64,
    pub omega_dot: i32,
    pub bgd_e5a_e1: i16,
    pub e5a_signal_health: u8,
    pub e5a_data_validity: bool,
    pub reserved: u8,
}

/// A decoded 1046 Galileo I/NAV broadcast ephemeris, mirroring
/// sidereon_core::rtcm::GalileoInavEphemeris. Every field is the raw
/// transmitted integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmGalileoInavEphemeris {
    pub satellite_id: u8,
    pub week_number: u16,
    pub iod_nav: u16,
    pub sisa_index: u8,
    pub idot: i32,
    pub t_oc: u16,
    pub a_f2: i16,
    pub a_f1: i32,
    pub a_f0: i64,
    pub c_rs: i32,
    pub delta_n: i32,
    pub m0: i64,
    pub c_uc: i32,
    pub eccentricity: u64,
    pub c_us: i32,
    pub sqrt_a: u64,
    pub t_oe: u16,
    pub c_ic: i32,
    pub omega0: i64,
    pub c_is: i32,
    pub i0: i64,
    pub c_rc: i32,
    pub omega: i64,
    pub omega_dot: i32,
    pub bgd_e5a_e1: i16,
    pub bgd_e5b_e1: i16,
    pub e5b_signal_health: u8,
    pub e5b_data_validity: bool,
    pub e1b_signal_health: u8,
    pub e1b_data_validity: bool,
    pub reserved: u8,
}

/// A decoded 1042 BeiDou broadcast ephemeris, mirroring
/// sidereon_core::rtcm::BeidouEphemeris. Every field is the raw transmitted
/// integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmBeidouEphemeris {
    pub satellite_id: u8,
    pub week_number: u16,
    pub sv_urai: u8,
    pub idot: i32,
    pub aode: u8,
    pub t_oc: u32,
    pub a_f2: i16,
    pub a_f1: i32,
    pub a_f0: i32,
    pub aodc: u8,
    pub c_rs: i32,
    pub delta_n: i32,
    pub m0: i64,
    pub c_uc: i32,
    pub eccentricity: u64,
    pub c_us: i32,
    pub sqrt_a: u64,
    pub t_oe: u32,
    pub c_ic: i32,
    pub omega0: i64,
    pub c_is: i32,
    pub i0: i64,
    pub c_rc: i32,
    pub omega: i64,
    pub omega_dot: i32,
    pub t_gd1: i16,
    pub t_gd2: i16,
    pub sv_health: bool,
}

/// A decoded 1044 QZSS broadcast ephemeris, mirroring
/// sidereon_core::rtcm::QzssEphemeris. Every field is the raw transmitted
/// integer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmQzssEphemeris {
    pub satellite_id: u8,
    pub t_oc: u16,
    pub a_f2: i16,
    pub a_f1: i32,
    pub a_f0: i32,
    pub iode: u8,
    pub c_rs: i32,
    pub delta_n: i32,
    pub m0: i64,
    pub c_uc: i32,
    pub eccentricity: u64,
    pub c_us: i32,
    pub sqrt_a: u64,
    pub t_oe: u16,
    pub c_ic: i32,
    pub omega0: i64,
    pub c_is: i32,
    pub i0: i64,
    pub c_rc: i32,
    pub omega: i64,
    pub omega_dot: i32,
    pub idot: i32,
    pub codes_on_l2: u8,
    pub week_number: u16,
    pub ura: u8,
    pub sv_health: u8,
    pub t_gd: i16,
    pub iodc: u16,
    pub fit_interval: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRtcmNavicEphemeris {
    pub satellite_id: u8,
    pub week_number: u16,
    pub a_f0: i32,
    pub a_f1: i32,
    pub a_f2: i16,
    pub ura: u8,
    pub t_oc: u16,
    pub t_gd: i16,
    pub delta_n: i32,
    pub iodec: u8,
    pub reserved: u16,
    pub l5_flag: bool,
    pub s_flag: bool,
    pub c_uc: i32,
    pub c_us: i32,
    pub c_ic: i32,
    pub c_is: i32,
    pub c_rc: i32,
    pub c_rs: i32,
    pub idot: i32,
    pub m0: i64,
    pub t_oe: u16,
    pub eccentricity: u64,
    pub sqrt_a: u64,
    pub omega0: i64,
    pub omega: i64,
    pub omega_dot: i32,
    pub i0: i64,
    pub spare_df544: u8,
    pub spare_df545: u8,
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
    /// The sign-magnitude fields transmitted as negative zero, one bit per
    /// field as sidereon_core::rtcm::GlonassEphemeris::negative_zero defines
    /// them. Each such field reads as 0; the bit keeps the sign so the body
    /// re-encodes as transmitted. Zero for a message built by hand.
    pub negative_zero: u16,
}

/// Decode a complete RTCM 3 byte stream into a message list under the strict
/// policy. The stream is refused unless every byte belongs to a CRC-valid
/// frame whose body decodes: a stray byte, a CRC-24Q failure, a trailing
/// partial frame or a skipped frame fails the call with
/// SIDEREON_STATUS_SP3_PARSE, naming what was not read.
/// sidereon_rtcm_decode_stream reads a noisy stream frame by frame instead. On
/// success writes a newly owned handle to *out_messages. Release it with
/// sidereon_rtcm_messages_free. Delegates to
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
    rtcm_operation_boundary(
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
            let messages = match core_rtcm::decode_messages(data) {
                Ok(messages) => messages,
                Err(err) => return map_rtcm_error("sidereon_rtcm_decode_messages", err),
            };
            write_boxed_handle(out_messages, SidereonRtcmMessages { messages });
            SidereonStatus::Ok
        },
    )
}

/// Decode an RTCM 3 byte stream into messages plus stream diagnostics under the
/// strict policy. Bad CRC frames and incomplete trailing bytes count as resync
/// bytes; CRC-valid frames with undecodable bodies, and frames that depart
/// from the format, are reported in diagnostics. On success writes newly owned
/// handles to *out_messages and *out_diagnostics. Release them with
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
    rtcm_operation_boundary("sidereon_rtcm_decode_stream", SidereonStatus::Panic, || {
        rtcm_decode_stream_body(
            "sidereon_rtcm_decode_stream",
            bytes,
            len,
            core_rtcm::RtcmPolicy::Strict,
            out_messages,
            out_diagnostics,
        )
    })
}

/// Decode an RTCM 3 byte stream as sidereon_rtcm_decode_stream does, under
/// `policy`, a SidereonRtcmPolicy value. Under SIDEREON_RTCM_POLICY_LENIENT a
/// frame that departs from the format is read, and each departure is recorded
/// in the diagnostics (sidereon_rtcm_stream_diagnostics_departure).
///
/// Safety: as for sidereon_rtcm_decode_stream.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_decode_stream_with_policy(
    bytes: *const u8,
    len: usize,
    policy: u32,
    out_messages: *mut *mut SidereonRtcmMessages,
    out_diagnostics: *mut *mut SidereonRtcmStreamDiagnostics,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_decode_stream_with_policy";
    rtcm_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let policy = match policy {
            x if x == SidereonRtcmPolicy::Strict as u32 => core_rtcm::RtcmPolicy::Strict,
            x if x == SidereonRtcmPolicy::Lenient as u32 => core_rtcm::RtcmPolicy::Lenient,
            other => {
                if !out_messages.is_null() {
                    *out_messages = ptr::null_mut();
                }
                if !out_diagnostics.is_null() {
                    *out_diagnostics = ptr::null_mut();
                }
                set_last_error(format!("{fn_name}: unknown RTCM policy {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        rtcm_decode_stream_body(fn_name, bytes, len, policy, out_messages, out_diagnostics)
    })
}

unsafe fn rtcm_decode_stream_body(
    fn_name: &str,
    bytes: *const u8,
    len: usize,
    policy: core_rtcm::RtcmPolicy,
    out_messages: *mut *mut SidereonRtcmMessages,
    out_diagnostics: *mut *mut SidereonRtcmStreamDiagnostics,
) -> SidereonStatus {
    let out_messages = c_try!(require_out(out_messages, fn_name, "out_messages"));
    *out_messages = ptr::null_mut();
    let out_diagnostics = c_try!(require_out(out_diagnostics, fn_name, "out_diagnostics"));
    *out_diagnostics = ptr::null_mut();
    let data = c_try!(require_slice(bytes, len, fn_name, "bytes"));
    let stream = core_rtcm::decode_stream_with_policy(data, policy);
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
}

/// Copy the number of preambles whose declared frame lay within the buffer
/// but failed its CRC-24Q. Each also counts one resync byte.
///
/// Safety: diagnostics is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_crc_failures(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    out_count: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_stream_diagnostics_crc_failures";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, fn_name, "out_count"));
        *out = 0;
        let diagnostics = c_try!(require_ref(diagnostics, fn_name, "diagnostics"));
        *out = diagnostics.diagnostics.crc_failures;
        SidereonStatus::Ok
    })
}

/// Copy the number of departures read under the lenient policy.
///
/// Safety: diagnostics is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_departure_count(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    out_count: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_stream_diagnostics_departure_count";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, fn_name, "out_count"));
        *out = 0;
        let diagnostics = c_try!(require_ref(diagnostics, fn_name, "diagnostics"));
        *out = diagnostics.diagnostics.departures.len();
        SidereonStatus::Ok
    })
}

/// Copy one departure read under the lenient policy: the byte offset of its
/// frame preamble to *out_offset, and its description (not null-terminated)
/// into out under the variable-length output contract.
///
/// Safety: diagnostics is a live handle; out_offset points to a size_t; out
/// points to len writable bytes or is NULL when len is 0; out_written and
/// out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_stream_diagnostics_departure(
    diagnostics: *const SidereonRtcmStreamDiagnostics,
    index: usize,
    out_offset: *mut usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_stream_diagnostics_departure";
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let out_offset = c_try!(require_out(out_offset, fn_name, "out_offset"));
        *out_offset = 0;
        let diagnostics = c_try!(require_ref(diagnostics, fn_name, "diagnostics"));
        let Some(departure) = diagnostics.diagnostics.departures.get(index) else {
            set_last_error(format!(
                "{fn_name}: index {index} out of range ({} departures)",
                diagnostics.diagnostics.departures.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out_offset = departure.offset;
        let text = departure.departure.to_string();
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
                core_rtcm::FrameSkipReason::Truncated => String::new(),
                core_rtcm::FrameSkipReason::Malformed(message) => message.clone(),
                core_rtcm::FrameSkipReason::Departure(departure) => departure.to_string(),
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_legacy_header(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmLegacyHeader,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_legacy_header",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_legacy_header",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_legacy_header",
                messages,
                index
            ));
            match message {
                RtcmMessage::LegacyObservations(value) => {
                    *out = SidereonRtcmLegacyHeader {
                        message_number: value.message_number,
                        reference_station_id: value.reference_station_id,
                        epoch_time: value.epoch_time,
                        synchronous_gnss: value.synchronous_gnss,
                        satellite_count: value.satellite_count,
                        divergence_free_smoothing: value.divergence_free_smoothing,
                        smoothing_interval: value.smoothing_interval,
                        satellite_records: value.satellites.len(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_legacy_header", "legacy observations"),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_legacy_satellite(
    messages: *const SidereonRtcmMessages,
    index: usize,
    satellite_index: usize,
    out: *mut SidereonRtcmLegacySatellite,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_legacy_satellite",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_legacy_satellite",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_legacy_satellite",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::LegacyObservations(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_legacy_satellite",
                        "legacy observations",
                    );
                }
            };
            let satellite = match value.satellites.get(satellite_index) {
                Some(satellite) => satellite,
                None => {
                    set_last_error(
                        "sidereon_rtcm_message_legacy_satellite: satellite_index is out of range",
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = SidereonRtcmLegacySatellite {
                satellite_id: satellite.satellite_id,
                has_frequency_channel: satellite.frequency_channel.is_some(),
                frequency_channel: satellite.frequency_channel.unwrap_or_default(),
                l1: SidereonRtcmLegacyL1 {
                    code_indicator: satellite.l1.code_indicator,
                    pseudorange: satellite.l1.pseudorange,
                    phase_range_minus_pseudorange: satellite.l1.phase_range_minus_pseudorange,
                    lock_time_indicator: satellite.l1.lock_time_indicator,
                    has_pseudorange_modulus_ambiguity: satellite
                        .l1
                        .pseudorange_modulus_ambiguity
                        .is_some(),
                    pseudorange_modulus_ambiguity: satellite
                        .l1
                        .pseudorange_modulus_ambiguity
                        .unwrap_or_default(),
                    has_cnr: satellite.l1.cnr.is_some(),
                    cnr: satellite.l1.cnr.unwrap_or_default(),
                },
                has_l2: satellite.l2.is_some(),
                l2: satellite
                    .l2
                    .map(|l2| SidereonRtcmLegacyL2 {
                        code_indicator: l2.code_indicator,
                        pseudorange_difference: l2.pseudorange_difference,
                        phase_range_minus_l1_pseudorange: l2.phase_range_minus_l1_pseudorange,
                        lock_time_indicator: l2.lock_time_indicator,
                        has_cnr: l2.cnr.is_some(),
                        cnr: l2.cnr.unwrap_or_default(),
                    })
                    .unwrap_or(SidereonRtcmLegacyL2 {
                        code_indicator: 0,
                        pseudorange_difference: 0,
                        phase_range_minus_l1_pseudorange: 0,
                        lock_time_indicator: 0,
                        has_cnr: false,
                        cnr: 0,
                    }),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_legacy(
    message_number: u16,
    reference_station_id: u16,
    epoch_time: u32,
    synchronous_gnss: bool,
    satellite_count: u8,
    divergence_free_smoothing: bool,
    smoothing_interval: u8,
    satellites: *const SidereonRtcmLegacySatellite,
    satellite_len: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary("sidereon_rtcm_build_legacy", SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(
            out_messages,
            "sidereon_rtcm_build_legacy",
            "out_messages"
        ));
        let satellites = c_try!(require_slice(
            satellites,
            satellite_len,
            "sidereon_rtcm_build_legacy",
            "satellites"
        ));
        let trailing_bits = c_try!(require_slice(
            trailing_bits,
            trailing_bit_count,
            "sidereon_rtcm_build_legacy",
            "trailing_bits"
        ));
        let value = RtcmLegacyObservations {
            message_number,
            reference_station_id,
            epoch_time,
            synchronous_gnss,
            satellite_count,
            divergence_free_smoothing,
            smoothing_interval,
            satellites: satellites
                .iter()
                .map(|satellite| RtcmLegacySatellite {
                    satellite_id: satellite.satellite_id,
                    frequency_channel: satellite
                        .has_frequency_channel
                        .then_some(satellite.frequency_channel),
                    l1: RtcmLegacyL1 {
                        code_indicator: satellite.l1.code_indicator,
                        pseudorange: satellite.l1.pseudorange,
                        phase_range_minus_pseudorange: satellite.l1.phase_range_minus_pseudorange,
                        lock_time_indicator: satellite.l1.lock_time_indicator,
                        pseudorange_modulus_ambiguity: satellite
                            .l1
                            .has_pseudorange_modulus_ambiguity
                            .then_some(satellite.l1.pseudorange_modulus_ambiguity),
                        cnr: satellite.l1.has_cnr.then_some(satellite.l1.cnr),
                    },
                    l2: satellite.has_l2.then_some(RtcmLegacyL2 {
                        code_indicator: satellite.l2.code_indicator,
                        pseudorange_difference: satellite.l2.pseudorange_difference,
                        phase_range_minus_l1_pseudorange: satellite
                            .l2
                            .phase_range_minus_l1_pseudorange,
                        lock_time_indicator: satellite.l2.lock_time_indicator,
                        cnr: satellite.l2.has_cnr.then_some(satellite.l2.cnr),
                    }),
                })
                .collect(),
            trailing_bits: trailing_bits.to_vec(),
        };
        rtcm_build(out_messages, RtcmMessage::LegacyObservations(value));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_network_auxiliary_station(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNetworkAuxiliaryStation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_network_auxiliary_station",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_network_auxiliary_station",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_network_auxiliary_station",
                messages,
                index
            ));
            match message {
                RtcmMessage::NetworkAuxiliaryStation(value) => {
                    *out = SidereonRtcmNetworkAuxiliaryStation {
                        network_id: value.network_id,
                        subnetwork_id: value.subnetwork_id,
                        auxiliary_station_count: value.auxiliary_station_count,
                        master_station_id: value.master_station_id,
                        auxiliary_station_id: value.auxiliary_station_id,
                        delta_latitude: value.delta_latitude,
                        delta_longitude: value.delta_longitude,
                        delta_height: value.delta_height,
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_network_auxiliary_station",
                    "network auxiliary station message",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_network_auxiliary_station(
    fields: *const SidereonRtcmNetworkAuxiliaryStation,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_network_auxiliary_station",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_network_auxiliary_station",
                "fields"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_network_auxiliary_station",
                "out_messages"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_network_auxiliary_station",
                "trailing_bits"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::NetworkAuxiliaryStation(RtcmNetworkAuxiliaryStation {
                    network_id: fields.network_id,
                    subnetwork_id: fields.subnetwork_id,
                    auxiliary_station_count: fields.auxiliary_station_count,
                    master_station_id: fields.master_station_id,
                    auxiliary_station_id: fields.auxiliary_station_id,
                    delta_latitude: fields.delta_latitude,
                    delta_longitude: fields.delta_longitude,
                    delta_height: fields.delta_height,
                    trailing_bits: trailing_bits.to_vec(),
                }),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_network_differences(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNetworkDifferences,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_network_differences",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_network_differences",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_network_differences",
                messages,
                index
            ));
            match message {
                RtcmMessage::NetworkCorrectionDifferences(value) => {
                    *out = SidereonRtcmNetworkDifferences {
                        message_number: value.message_number,
                        network_id: value.network_id,
                        subnetwork_id: value.subnetwork_id,
                        epoch_time: value.epoch_time,
                        multiple_message: value.multiple_message,
                        master_station_id: value.master_station_id,
                        auxiliary_station_id: value.auxiliary_station_id,
                        satellite_count: value.satellite_count,
                        satellite_records: value.satellites.len(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_network_differences",
                    "network correction-difference message",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_network_difference_satellites(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNetworkDifference,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_network_difference_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_network_difference_satellites",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_network_difference_satellites",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::NetworkCorrectionDifferences(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_network_difference_satellites",
                        "network correction-difference message",
                    );
                }
            };
            let rows = value
                .satellites
                .iter()
                .map(|satellite| SidereonRtcmNetworkDifference {
                    satellite_id: satellite.satellite_id,
                    ambiguity_status: satellite.ambiguity_status,
                    non_sync_count: satellite.non_sync_count,
                    has_geometric: satellite.geometric.is_some(),
                    geometric: satellite.geometric.unwrap_or_default(),
                    has_iod: satellite.iod.is_some(),
                    iod: satellite.iod.unwrap_or_default(),
                    has_ionospheric: satellite.ionospheric.is_some(),
                    ionospheric: satellite.ionospheric.unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_network_difference_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_network_differences(
    fields: *const SidereonRtcmNetworkDifferences,
    satellites: *const SidereonRtcmNetworkDifference,
    satellite_len: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_network_differences",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_network_differences",
                "fields"
            ));
            let satellites = c_try!(require_slice(
                satellites,
                satellite_len,
                "sidereon_rtcm_build_network_differences",
                "satellites"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_network_differences",
                "trailing_bits"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_network_differences",
                "out_messages"
            ));
            let value = RtcmNetworkCorrectionDifferences {
                message_number: fields.message_number,
                network_id: fields.network_id,
                subnetwork_id: fields.subnetwork_id,
                epoch_time: fields.epoch_time,
                multiple_message: fields.multiple_message,
                master_station_id: fields.master_station_id,
                auxiliary_station_id: fields.auxiliary_station_id,
                satellite_count: fields.satellite_count,
                satellites: satellites
                    .iter()
                    .map(|satellite| RtcmNetworkCorrectionDifference {
                        satellite_id: satellite.satellite_id,
                        ambiguity_status: satellite.ambiguity_status,
                        non_sync_count: satellite.non_sync_count,
                        geometric: satellite.has_geometric.then_some(satellite.geometric),
                        iod: satellite.has_iod.then_some(satellite.iod),
                        ionospheric: satellite.has_ionospheric.then_some(satellite.ionospheric),
                    })
                    .collect(),
                trailing_bits: trailing_bits.to_vec(),
            };
            rtcm_build(
                out_messages,
                RtcmMessage::NetworkCorrectionDifferences(value),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_network_residuals(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNetworkResiduals,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_network_residuals",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_network_residuals",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_network_residuals",
                messages,
                index
            ));
            match message {
                RtcmMessage::NetworkResiduals(value) => {
                    *out = SidereonRtcmNetworkResiduals {
                        message_number: value.message_number,
                        epoch_time: value.epoch_time,
                        reference_station_id: value.reference_station_id,
                        reference_station_count: value.reference_station_count,
                        satellite_count: value.satellite_count,
                        satellite_records: value.satellites.len(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_network_residuals",
                    "network residual message",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_network_residual_satellites(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNetworkResidual,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_network_residual_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_network_residual_satellites",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_network_residual_satellites",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::NetworkResiduals(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_network_residual_satellites",
                        "network residual message",
                    );
                }
            };
            let rows = value
                .satellites
                .iter()
                .map(|item| SidereonRtcmNetworkResidual {
                    satellite_id: item.satellite_id,
                    s_oc: item.s_oc,
                    s_od: item.s_od,
                    s_oh: item.s_oh,
                    s_lc: item.s_lc,
                    s_ld: item.s_ld,
                })
                .collect::<Vec<_>>();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_network_residual_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_network_residuals(
    fields: *const SidereonRtcmNetworkResiduals,
    satellites: *const SidereonRtcmNetworkResidual,
    satellite_len: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_network_residuals",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_network_residuals",
                "fields"
            ));
            let satellites = c_try!(require_slice(
                satellites,
                satellite_len,
                "sidereon_rtcm_build_network_residuals",
                "satellites"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_network_residuals",
                "trailing_bits"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_network_residuals",
                "out_messages"
            ));
            let value = RtcmNetworkResiduals {
                message_number: fields.message_number,
                epoch_time: fields.epoch_time,
                reference_station_id: fields.reference_station_id,
                reference_station_count: fields.reference_station_count,
                satellite_count: fields.satellite_count,
                satellites: satellites
                    .iter()
                    .map(|item| RtcmNetworkResidual {
                        satellite_id: item.satellite_id,
                        s_oc: item.s_oc,
                        s_od: item.s_od,
                        s_oh: item.s_oh,
                        s_lc: item.s_lc,
                        s_ld: item.s_ld,
                    })
                    .collect(),
                trailing_bits: trailing_bits.to_vec(),
            };
            rtcm_build(out_messages, RtcmMessage::NetworkResiduals(value));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_fkp_gradients(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmFkpGradients,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_fkp_gradients",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_fkp_gradients",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_fkp_gradients",
                messages,
                index
            ));
            match message {
                RtcmMessage::FkpGradients(value) => {
                    *out = SidereonRtcmFkpGradients {
                        message_number: value.message_number,
                        reference_station_id: value.reference_station_id,
                        epoch_time: value.epoch_time,
                        satellite_count: value.satellite_count,
                        satellite_records: value.satellites.len(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_fkp_gradients",
                    "FKP gradient message",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_fkp_gradient_satellites(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmFkpGradient,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_fkp_gradient_satellites",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_fkp_gradient_satellites",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_fkp_gradient_satellites",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::FkpGradients(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_fkp_gradient_satellites",
                        "FKP gradient message",
                    );
                }
            };
            let rows = value
                .satellites
                .iter()
                .map(|item| SidereonRtcmFkpGradient {
                    satellite_id: item.satellite_id,
                    iod: item.iod,
                    geometric_north: item.geometric_north,
                    geometric_east: item.geometric_east,
                    ionospheric_north: item.ionospheric_north,
                    ionospheric_east: item.ionospheric_east,
                })
                .collect::<Vec<_>>();
            c_try!(copy_prefix_to_c(
                "sidereon_rtcm_message_fkp_gradient_satellites",
                "out",
                &rows,
                out,
                len,
                out_written,
                out_required
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_fkp_gradients(
    fields: *const SidereonRtcmFkpGradients,
    satellites: *const SidereonRtcmFkpGradient,
    satellite_len: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_fkp_gradients",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_fkp_gradients",
                "fields"
            ));
            let satellites = c_try!(require_slice(
                satellites,
                satellite_len,
                "sidereon_rtcm_build_fkp_gradients",
                "satellites"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_fkp_gradients",
                "trailing_bits"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_fkp_gradients",
                "out_messages"
            ));
            let value = RtcmFkpGradients {
                message_number: fields.message_number,
                reference_station_id: fields.reference_station_id,
                epoch_time: fields.epoch_time,
                satellite_count: fields.satellite_count,
                satellites: satellites
                    .iter()
                    .map(|item| RtcmFkpGradient {
                        satellite_id: item.satellite_id,
                        iod: item.iod,
                        geometric_north: item.geometric_north,
                        geometric_east: item.geometric_east,
                        ionospheric_north: item.ionospheric_north,
                        ionospheric_east: item.ionospheric_east,
                    })
                    .collect(),
                trailing_bits: trailing_bits.to_vec(),
            };
            rtcm_build(out_messages, RtcmMessage::FkpGradients(value));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_residual_grid(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmResidualGrid,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_residual_grid",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_residual_grid",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_residual_grid",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::ResidualGrid(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_residual_grid",
                        "coordinate transformation residual grid",
                    );
                }
            };
            *out = SidereonRtcmResidualGrid {
                message_number: value.message_number,
                system_id: value.system_id,
                horizontal_shift: value.horizontal_shift,
                vertical_shift: value.vertical_shift,
                origin_1: value.origin_1,
                origin_2: value.origin_2,
                extension_1: value.extension_1,
                extension_2: value.extension_2,
                mean_offset_1: value.mean_offset_1,
                mean_offset_2: value.mean_offset_2,
                mean_height_offset: value.mean_height_offset,
                residuals: value.residuals.map(|item| SidereonRtcmGridResidual {
                    horizontal_1: item.horizontal_1,
                    horizontal_2: item.horizontal_2,
                    height: item.height,
                }),
                horizontal_interpolation: value.horizontal_interpolation,
                vertical_interpolation: value.vertical_interpolation,
                horizontal_quality: value.horizontal_quality,
                vertical_quality: value.vertical_quality,
                mjd: value.mjd,
                trailing_bit_count: value.trailing_bits.len(),
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_residual_grid(
    fields: *const SidereonRtcmResidualGrid,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_residual_grid",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_residual_grid",
                "fields"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_residual_grid",
                "trailing_bits"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_residual_grid",
                "out_messages"
            ));
            let value = RtcmResidualGrid {
                message_number: fields.message_number,
                system_id: fields.system_id,
                horizontal_shift: fields.horizontal_shift,
                vertical_shift: fields.vertical_shift,
                origin_1: fields.origin_1,
                origin_2: fields.origin_2,
                extension_1: fields.extension_1,
                extension_2: fields.extension_2,
                mean_offset_1: fields.mean_offset_1,
                mean_offset_2: fields.mean_offset_2,
                mean_height_offset: fields.mean_height_offset,
                residuals: fields.residuals.map(|item| RtcmGridResidual {
                    horizontal_1: item.horizontal_1,
                    horizontal_2: item.horizontal_2,
                    height: item.height,
                }),
                horizontal_interpolation: fields.horizontal_interpolation,
                vertical_interpolation: fields.vertical_interpolation,
                horizontal_quality: fields.horizontal_quality,
                vertical_quality: fields.vertical_quality,
                mjd: fields.mjd,
                trailing_bits: trailing_bits.to_vec(),
            };
            rtcm_build(out_messages, RtcmMessage::ResidualGrid(value));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_projection(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmProjection,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_projection",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_rtcm_message_projection", "out"));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_projection",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::Projection(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_projection",
                        "projection parameter message",
                    );
                }
            };
            let mut fields = SidereonRtcmProjection {
                message_number: value.message_number(),
                system_id: value.system_id,
                projection_type: value.projection_type,
                rectification: false,
                latitude: 0,
                longitude: 0,
                standard_parallel_1: 0,
                standard_parallel_2: 0,
                azimuth: 0,
                rectified_to_skew: 0,
                add_scale: 0,
                easting: 0,
                northing: 0,
                trailing_bit_count: value.trailing_bits.len(),
            };
            match value.parameters {
                RtcmProjectionParameters::NaturalOrigin {
                    latitude,
                    longitude,
                    add_scale,
                    false_easting,
                    false_northing,
                } => {
                    fields.latitude = latitude;
                    fields.longitude = longitude;
                    fields.add_scale = add_scale;
                    fields.easting = false_easting;
                    fields.northing = false_northing;
                }
                RtcmProjectionParameters::LambertConicConformal {
                    latitude,
                    longitude,
                    standard_parallel_1,
                    standard_parallel_2,
                    false_easting,
                    false_northing,
                } => {
                    fields.latitude = latitude;
                    fields.longitude = longitude;
                    fields.standard_parallel_1 = standard_parallel_1;
                    fields.standard_parallel_2 = standard_parallel_2;
                    fields.easting = false_easting;
                    fields.northing = false_northing;
                }
                RtcmProjectionParameters::ObliqueMercator {
                    rectification,
                    latitude,
                    longitude,
                    azimuth,
                    rectified_to_skew,
                    add_scale,
                    easting,
                    northing,
                } => {
                    fields.rectification = rectification;
                    fields.latitude = latitude;
                    fields.longitude = longitude;
                    fields.azimuth = azimuth;
                    fields.rectified_to_skew = rectified_to_skew;
                    fields.add_scale = add_scale;
                    fields.easting = easting;
                    fields.northing = northing;
                }
            }
            *out = fields;
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_projection(
    fields: *const SidereonRtcmProjection,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_projection",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_projection",
                "fields"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_projection",
                "trailing_bits"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_projection",
                "out_messages"
            ));
            let parameters = match fields.message_number {
                1025 => RtcmProjectionParameters::NaturalOrigin {
                    latitude: fields.latitude,
                    longitude: fields.longitude,
                    add_scale: fields.add_scale,
                    false_easting: fields.easting,
                    false_northing: fields.northing,
                },
                1026 => RtcmProjectionParameters::LambertConicConformal {
                    latitude: fields.latitude,
                    longitude: fields.longitude,
                    standard_parallel_1: fields.standard_parallel_1,
                    standard_parallel_2: fields.standard_parallel_2,
                    false_easting: fields.easting,
                    false_northing: fields.northing,
                },
                1027 => RtcmProjectionParameters::ObliqueMercator {
                    rectification: fields.rectification,
                    latitude: fields.latitude,
                    longitude: fields.longitude,
                    azimuth: fields.azimuth,
                    rectified_to_skew: fields.rectified_to_skew,
                    add_scale: fields.add_scale,
                    easting: fields.easting,
                    northing: fields.northing,
                },
                other => {
                    set_last_error(format!(
                        "sidereon_rtcm_build_projection: unsupported message number {other}"
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            rtcm_build(
                out_messages,
                RtcmMessage::Projection(RtcmProjection {
                    system_id: fields.system_id,
                    projection_type: fields.projection_type,
                    parameters,
                    trailing_bits: trailing_bits.to_vec(),
                }),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_helmert_transformation(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmHelmertTransformation,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_helmert_transformation";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let value = match message {
            RtcmMessage::HelmertTransformation(value) => value,
            _ => return rtcm_wrong_kind(FN_NAME, "Helmert transformation message"),
        };
        let source_name = match transformation_name_to_c(FN_NAME, &value.source_name) {
            Ok(name) => name,
            Err(status) => return status,
        };
        let target_name = match transformation_name_to_c(FN_NAME, &value.target_name) {
            Ok(name) => name,
            Err(status) => return status,
        };
        let rotation_point = value
            .rotation_point
            .unwrap_or(RtcmRotationPoint { x: 0, y: 0, z: 0 });
        *out = SidereonRtcmHelmertTransformation {
            message_number: value.message_number,
            source_name: source_name.0,
            source_name_len: source_name.1,
            target_name: target_name.0,
            target_name_len: target_name.1,
            system_id: value.system_id,
            utilized_messages: value.utilized_messages,
            plate_number: value.plate_number,
            computation_indicator: value.computation_indicator,
            height_indicator: value.height_indicator,
            validity_latitude: value.validity_latitude,
            validity_longitude: value.validity_longitude,
            validity_extension_latitude: value.validity_extension_latitude,
            validity_extension_longitude: value.validity_extension_longitude,
            dx: value.dx,
            dy: value.dy,
            dz: value.dz,
            r1: value.r1,
            r2: value.r2,
            r3: value.r3,
            ds: value.ds,
            has_rotation_point: value.rotation_point.is_some(),
            rotation_point_x: rotation_point.x,
            rotation_point_y: rotation_point.y,
            rotation_point_z: rotation_point.z,
            add_as: value.add_as,
            add_bs: value.add_bs,
            add_at: value.add_at,
            add_bt: value.add_bt,
            horizontal_quality: value.horizontal_quality,
            vertical_quality: value.vertical_quality,
            trailing_bit_count: value.trailing_bits.len(),
        };
        SidereonStatus::Ok
    })
}

fn transformation_name_to_c(fn_name: &str, value: &str) -> Result<([u8; 31], u8), SidereonStatus> {
    let mut result = [0; 31];
    let mut len = 0usize;
    for character in value.chars() {
        let Ok(byte) = u8::try_from(u32::from(character)) else {
            set_last_error(format!(
                "{fn_name}: transformation name contains a non-Latin-1 character"
            ));
            return Err(SidereonStatus::InvalidArgument);
        };
        if len == result.len() {
            set_last_error(format!(
                "{fn_name}: transformation name exceeds 31 characters"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        result[len] = byte;
        len += 1;
    }
    Ok((result, len as u8))
}

fn transformation_name_from_c(bytes: &[u8; 31], len: u8) -> Result<String, SidereonStatus> {
    let count = usize::from(len);
    if count > bytes.len() {
        set_last_error("sidereon_rtcm_build_helmert_transformation: name length exceeds storage");
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(bytes[..count]
        .iter()
        .map(|byte| char::from(*byte))
        .collect())
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_helmert_transformation(
    fields: *const SidereonRtcmHelmertTransformation,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_helmert_transformation";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let fields = c_try!(require_ref(fields, FN_NAME, "fields"));
        let trailing_bits = c_try!(require_slice(
            trailing_bits,
            trailing_bit_count,
            FN_NAME,
            "trailing_bits"
        ));
        let out_messages = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        let source_name =
            match transformation_name_from_c(&fields.source_name, fields.source_name_len) {
                Ok(name) => name,
                Err(status) => return status,
            };
        let target_name =
            match transformation_name_from_c(&fields.target_name, fields.target_name_len) {
                Ok(name) => name,
                Err(status) => return status,
            };
        let rotation_point = fields.has_rotation_point.then_some(RtcmRotationPoint {
            x: fields.rotation_point_x,
            y: fields.rotation_point_y,
            z: fields.rotation_point_z,
        });
        rtcm_build(
            out_messages,
            RtcmMessage::HelmertTransformation(RtcmHelmertTransformation {
                message_number: fields.message_number,
                source_name,
                target_name,
                system_id: fields.system_id,
                utilized_messages: fields.utilized_messages,
                plate_number: fields.plate_number,
                computation_indicator: fields.computation_indicator,
                height_indicator: fields.height_indicator,
                validity_latitude: fields.validity_latitude,
                validity_longitude: fields.validity_longitude,
                validity_extension_latitude: fields.validity_extension_latitude,
                validity_extension_longitude: fields.validity_extension_longitude,
                dx: fields.dx,
                dy: fields.dy,
                dz: fields.dz,
                r1: fields.r1,
                r2: fields.r2,
                r3: fields.r3,
                ds: fields.ds,
                rotation_point,
                add_as: fields.add_as,
                add_bs: fields.add_bs,
                add_at: fields.add_at,
                add_bt: fields.add_bt,
                horizontal_quality: fields.horizontal_quality,
                vertical_quality: fields.vertical_quality,
                trailing_bits: trailing_bits.to_vec(),
            }),
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_physical_reference_station(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmPhysicalReferenceStation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_physical_reference_station",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_physical_reference_station",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_physical_reference_station",
                messages,
                index
            ));
            match message {
                RtcmMessage::PhysicalReferenceStation(value) => {
                    *out = SidereonRtcmPhysicalReferenceStation {
                        non_physical_station_id: value.non_physical_station_id,
                        physical_station_id: value.physical_station_id,
                        itrf_realization_year: value.itrf_realization_year,
                        ecef_x: value.ecef_x,
                        ecef_y: value.ecef_y,
                        ecef_z: value.ecef_z,
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_physical_reference_station",
                    "physical reference station message",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_physical_reference_station(
    fields: *const SidereonRtcmPhysicalReferenceStation,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_physical_reference_station",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_physical_reference_station",
                "fields"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_physical_reference_station",
                "out_messages"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_physical_reference_station",
                "trailing_bits"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::PhysicalReferenceStation(RtcmPhysicalReferenceStation {
                    non_physical_station_id: fields.non_physical_station_id,
                    physical_station_id: fields.physical_station_id,
                    itrf_realization_year: fields.itrf_realization_year,
                    ecef_x: fields.ecef_x,
                    ecef_y: fields.ecef_y,
                    ecef_z: fields.ecef_z,
                    trailing_bits: trailing_bits.to_vec(),
                }),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_glonass_code_phase_biases(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmGlonassCodePhaseBiases,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_glonass_code_phase_biases",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_glonass_code_phase_biases",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_glonass_code_phase_biases",
                messages,
                index
            ));
            match message {
                RtcmMessage::GlonassCodePhaseBiases(value) => {
                    *out = SidereonRtcmGlonassCodePhaseBiases {
                        reference_station_id: value.reference_station_id,
                        aligned: value.aligned,
                        reserved: value.reserved,
                        has_l1_ca: value.l1_ca.is_some(),
                        l1_ca: value.l1_ca.unwrap_or_default(),
                        has_l1_p: value.l1_p.is_some(),
                        l1_p: value.l1_p.unwrap_or_default(),
                        has_l2_ca: value.l2_ca.is_some(),
                        l2_ca: value.l2_ca.unwrap_or_default(),
                        has_l2_p: value.l2_p.is_some(),
                        l2_p: value.l2_p.unwrap_or_default(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_glonass_code_phase_biases",
                    "GLONASS code-phase biases",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_glonass_code_phase_biases(
    fields: *const SidereonRtcmGlonassCodePhaseBiases,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_glonass_code_phase_biases",
        SidereonStatus::Panic,
        || {
            let fields = c_try!(require_ref(
                fields,
                "sidereon_rtcm_build_glonass_code_phase_biases",
                "fields"
            ));
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_glonass_code_phase_biases",
                "out_messages"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_glonass_code_phase_biases",
                "trailing_bits"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::GlonassCodePhaseBiases(RtcmGlonassCodePhaseBiases {
                    reference_station_id: fields.reference_station_id,
                    aligned: fields.aligned,
                    reserved: fields.reserved,
                    l1_ca: fields.has_l1_ca.then_some(fields.l1_ca),
                    l1_p: fields.has_l1_p.then_some(fields.l1_p),
                    l2_ca: fields.has_l2_ca.then_some(fields.l2_ca),
                    l2_p: fields.has_l2_p.then_some(fields.l2_p),
                    trailing_bits: trailing_bits.to_vec(),
                }),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_system_parameters(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSystemParameters,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_system_parameters",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_system_parameters",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_system_parameters",
                messages,
                index
            ));
            match message {
                RtcmMessage::SystemParameters(value) => {
                    *out = SidereonRtcmSystemParameters {
                        reference_station_id: value.reference_station_id,
                        mjd: value.mjd,
                        seconds_of_day: value.seconds_of_day,
                        announcement_count: value.announcement_count,
                        leap_seconds: value.leap_seconds,
                        announcements: value.announcements.len(),
                        trailing_bit_count: value.trailing_bits.len(),
                    };
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_system_parameters",
                    "system parameters",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_announcement(
    messages: *const SidereonRtcmMessages,
    index: usize,
    announcement_index: usize,
    out: *mut SidereonRtcmMessageAnnouncement,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_announcement",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_announcement",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_announcement",
                messages,
                index
            ));
            let value = match message {
                RtcmMessage::SystemParameters(value) => value,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_announcement",
                        "system parameters",
                    );
                }
            };
            let item = match value.announcements.get(announcement_index) {
                Some(item) => item,
                None => {
                    set_last_error(
                        "sidereon_rtcm_message_announcement: announcement_index is out of range",
                    );
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = SidereonRtcmMessageAnnouncement {
                message_number: item.message_number,
                synchronous: item.synchronous,
                interval: item.interval,
            };
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_text(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmTextMessage,
    code_units: *mut u8,
    code_unit_capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_rtcm_message_text", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_rtcm_message_text",
            out_written,
            out_required
        ));
        let out = c_try!(require_out(out, "sidereon_rtcm_message_text", "out"));
        let message = c_try!(rtcm_message_at(
            "sidereon_rtcm_message_text",
            messages,
            index
        ));
        let value = match message {
            RtcmMessage::Text(value) => value,
            _ => return rtcm_wrong_kind("sidereon_rtcm_message_text", "text message"),
        };
        *out = SidereonRtcmTextMessage {
            reference_station_id: value.reference_station_id,
            mjd: value.mjd,
            seconds_of_day: value.seconds_of_day,
            character_count: value.character_count,
            code_unit_count: value.code_units.len(),
            trailing_bit_count: value.trailing_bits.len(),
        };
        c_try!(copy_prefix_to_c(
            "sidereon_rtcm_message_text",
            "code_units",
            &value.code_units,
            code_units,
            code_unit_capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_system_parameters(
    reference_station_id: u16,
    mjd: u16,
    seconds_of_day: u32,
    announcement_count: u8,
    leap_seconds: u8,
    announcements: *const SidereonRtcmMessageAnnouncement,
    announcement_len: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_system_parameters",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_system_parameters",
                "out_messages"
            ));
            let announcements = c_try!(require_slice(
                announcements,
                announcement_len,
                "sidereon_rtcm_build_system_parameters",
                "announcements"
            ));
            let trailing_bits = c_try!(require_slice(
                trailing_bits,
                trailing_bit_count,
                "sidereon_rtcm_build_system_parameters",
                "trailing_bits"
            ));
            let value = RtcmSystemParameters {
                reference_station_id,
                mjd,
                seconds_of_day,
                announcement_count,
                leap_seconds,
                announcements: announcements
                    .iter()
                    .map(|item| RtcmMessageAnnouncement {
                        message_number: item.message_number,
                        synchronous: item.synchronous,
                        interval: item.interval,
                    })
                    .collect(),
                trailing_bits: trailing_bits.to_vec(),
            };
            rtcm_build(out_messages, RtcmMessage::SystemParameters(value));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_text(
    reference_station_id: u16,
    mjd: u16,
    seconds_of_day: u32,
    character_count: u8,
    code_units: *const u8,
    code_unit_count: usize,
    trailing_bits: *const bool,
    trailing_bit_count: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary("sidereon_rtcm_build_text", SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(
            out_messages,
            "sidereon_rtcm_build_text",
            "out_messages"
        ));
        let code_units = c_try!(require_slice(
            code_units,
            code_unit_count,
            "sidereon_rtcm_build_text",
            "code_units"
        ));
        let trailing_bits = c_try!(require_slice(
            trailing_bits,
            trailing_bit_count,
            "sidereon_rtcm_build_text",
            "trailing_bits"
        ));
        rtcm_build(
            out_messages,
            RtcmMessage::Text(RtcmTextMessage {
                reference_station_id,
                mjd,
                seconds_of_day,
                character_count,
                code_units: code_units.to_vec(),
                trailing_bits: trailing_bits.to_vec(),
            }),
        );
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
                    );
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
                        system: gnss_system_to_c(msm.system) as u32,
                        kind: match msm.kind {
                            RtcmMsmKind::Msm1 => SidereonRtcmMsmKind::Msm1 as u32,
                            RtcmMsmKind::Msm2 => SidereonRtcmMsmKind::Msm2 as u32,
                            RtcmMsmKind::Msm3 => SidereonRtcmMsmKind::Msm3 as u32,
                            RtcmMsmKind::Msm4 => SidereonRtcmMsmKind::Msm4 as u32,
                            RtcmMsmKind::Msm5 => SidereonRtcmMsmKind::Msm5 as u32,
                            RtcmMsmKind::Msm6 => SidereonRtcmMsmKind::Msm6 as u32,
                            RtcmMsmKind::Msm7 => SidereonRtcmMsmKind::Msm7 as u32,
                        },
                        header: rtcm_msm_header_to_c(msm),
                        satellite_count: msm.satellites.len(),
                        signal_count: msm.signals.len(),
                        signal_mask: msm.signal_mask,
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
                    );
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
                    );
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
    rtcm_operation_boundary(
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
    rtcm_operation_boundary(
        "sidereon_rtcm_lock_time_tracker_reset",
        SidereonStatus::Panic,
        || {
            let tracker = c_try!(require_mut(
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
    rtcm_operation_boundary(
        "sidereon_rtcm_lock_time_tracker_observe",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_lock_time_tracker_observe",
                out_written,
                out_required
            ));
            let tracker = c_try!(require_mut(
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
                    );
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

/// Copy a decoded 1042 BeiDou broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a SidereonRtcmBeidouEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_beidou_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmBeidouEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_beidou_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_beidou_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_beidou_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::BeidouEphemeris(eph) => {
                    *out = rtcm_beidou_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_beidou_ephemeris",
                    "a BeiDou ephemeris",
                ),
            }
        },
    )
}

/// Copy a decoded 1044 QZSS broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a SidereonRtcmQzssEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_qzss_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmQzssEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_qzss_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_qzss_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_qzss_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::QzssEphemeris(eph) => {
                    *out = rtcm_qzss_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_qzss_ephemeris", "a QZSS ephemeris"),
            }
        },
    )
}

/// Copy a decoded 1045 Galileo F/NAV broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a
/// SidereonRtcmGalileoFnavEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_galileo_fnav_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmGalileoFnavEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_galileo_fnav_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_galileo_fnav_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_galileo_fnav_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::GalileoFnavEphemeris(eph) => {
                    *out = rtcm_galileo_fnav_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_galileo_fnav_ephemeris",
                    "a Galileo F/NAV ephemeris",
                ),
            }
        },
    )
}

/// Copy a decoded 1046 Galileo I/NAV broadcast ephemeris into *out.
///
/// Safety: messages is a live handle; out points to a
/// SidereonRtcmGalileoInavEphemeris.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_galileo_inav_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmGalileoInavEphemeris,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_galileo_inav_ephemeris",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_rtcm_message_galileo_inav_ephemeris",
                "out"
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_galileo_inav_ephemeris",
                messages,
                index
            ));
            match message {
                RtcmMessage::GalileoInavEphemeris(eph) => {
                    *out = rtcm_galileo_inav_ephemeris_to_c(eph);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind(
                    "sidereon_rtcm_message_galileo_inav_ephemeris",
                    "a Galileo I/NAV ephemeris",
                ),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_navic_ephemeris(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmNavicEphemeris,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_navic_ephemeris";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        match message {
            RtcmMessage::NavicEphemeris(eph) => {
                *out = rtcm_navic_ephemeris_to_c(eph);
                SidereonStatus::Ok
            }
            _ => rtcm_wrong_kind(FN_NAME, "a NavIC ephemeris"),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_navic_trailing_bits(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_navic_trailing_bits";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let eph = match message {
            RtcmMessage::NavicEphemeris(eph) => eph,
            _ => return rtcm_wrong_kind(FN_NAME, "a NavIC ephemeris"),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &eph.trailing_bits,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Encode one decoded message back into its RTCM body (without the transport
/// frame). Variable-length output contract. Delegates to
/// sidereon_core::rtcm::Message::encode.
///
/// A message whose fields the body cannot state is refused with
/// SIDEREON_STATUS_INVALID_ARGUMENT and the engine's reason in the thread-local
/// message, reporting a required length of zero and writing nothing: an MSM
/// with a satellite id outside 1..=64, a signal id outside 1..=32, a satellite
/// or satellite/signal cell listed twice, or a signal whose satellite is not
/// listed; an ephemeris whose satellite id is wider than the message's field
/// (six bits, four for QZSS 1044); an SSR message whose satellite field is
/// wider than the message's (five bits for GLONASS, four for the native QZSS
/// messages 1246..1251 and 1268, six otherwise). Each of those would otherwise
/// be written as a different mask or satellite.
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
    rtcm_operation_boundary(
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
            let body = match message.encode() {
                Ok(body) => body,
                Err(err) => return map_rtcm_error("sidereon_rtcm_message_encode", err),
            };
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_encode_with_policy(
    messages: *const SidereonRtcmMessages,
    index: usize,
    policy: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_departures: *mut SidereonRtcmDeparture,
    departures_capacity: usize,
    departures_written: *mut usize,
    departures_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_message_encode_with_policy";
    rtcm_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        c_try!(init_copy_counts(
            fn_name,
            departures_written,
            departures_required
        ));
        let selected_policy = match policy {
            value if value == SidereonRtcmPolicy::Strict as u32 => core_rtcm::RtcmPolicy::Strict,
            value if value == SidereonRtcmPolicy::Lenient as u32 => core_rtcm::RtcmPolicy::Lenient,
            _ => {
                set_last_error(format!("{fn_name}: unknown RTCM policy {policy}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let message = c_try!(rtcm_message_at(fn_name, messages, index));
        let (body, departures) = match message.encode_with_policy(selected_policy) {
            Ok(encoded) => encoded,
            Err(error) => return map_rtcm_error(fn_name, error),
        };
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            &body,
            out,
            len,
            out_written,
            out_required
        ));
        c_try!(copy_departures_to_c(
            fn_name,
            &departures,
            out_departures,
            departures_capacity,
            departures_written,
            departures_required
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_to_frame_with_policy(
    messages: *const SidereonRtcmMessages,
    index: usize,
    policy: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_departures: *mut SidereonRtcmDeparture,
    departures_capacity: usize,
    departures_written: *mut usize,
    departures_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_rtcm_message_to_frame_with_policy";
    rtcm_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        c_try!(init_copy_counts(
            fn_name,
            departures_written,
            departures_required
        ));
        let selected_policy = match policy {
            value if value == SidereonRtcmPolicy::Strict as u32 => core_rtcm::RtcmPolicy::Strict,
            value if value == SidereonRtcmPolicy::Lenient as u32 => core_rtcm::RtcmPolicy::Lenient,
            _ => {
                set_last_error(format!("{fn_name}: unknown RTCM policy {policy}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let message = c_try!(rtcm_message_at(fn_name, messages, index));
        let (body, departures) = match message.encode_with_policy(selected_policy) {
            Ok(encoded) => encoded,
            Err(error) => return map_rtcm_error(fn_name, error),
        };
        let frame = match core_rtcm::encode_frame(&body) {
            Ok(frame) => frame,
            Err(error) => return map_rtcm_error(fn_name, error),
        };
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            &frame,
            out,
            len,
            out_written,
            out_required
        ));
        c_try!(copy_departures_to_c(
            fn_name,
            &departures,
            out_departures,
            departures_capacity,
            departures_written,
            departures_required
        ));
        SidereonStatus::Ok
    })
}

/// Encode one decoded message into a complete RTCM transport frame (with a fresh
/// CRC-24Q). Variable-length output contract. Delegates to
/// sidereon_core::rtcm::Message::to_frame. A message body the encoder refuses,
/// as sidereon_rtcm_message_encode describes, or a body longer than the frame
/// length field is refused with SIDEREON_STATUS_INVALID_ARGUMENT.
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
    rtcm_operation_boundary(
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
    rtcm_operation_boundary("sidereon_rtcm_encode_frame", SidereonStatus::Panic, || {
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
    rtcm_operation_boundary("sidereon_rtcm_decode_frame", SidereonStatus::Panic, || {
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
    rtcm_operation_boundary("sidereon_rtcm_scan_frames", SidereonStatus::Panic, || {
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
                    *out = crate::ssr::ssr_info_to_c(ssr);
                    SidereonStatus::Ok
                }
                _ => rtcm_wrong_kind("sidereon_rtcm_message_ssr_info", "an SSR message"),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_vtec_info(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out_info: *mut SidereonRtcmSsrVtecInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_ssr_vtec_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let vtec = match message {
            RtcmMessage::SsrVtec(vtec) => vtec,
            _ => return rtcm_wrong_kind(FN_NAME, "an SSR VTEC message"),
        };
        *out = SidereonRtcmSsrVtecInfo {
            message_number: vtec.message_number,
            has_igs_ssr_version: vtec.igs_ssr_version.is_some(),
            igs_ssr_version: vtec.igs_ssr_version.unwrap_or(0),
            epoch_time_s: vtec.epoch_time_s,
            update_interval: vtec.update_interval,
            multiple_message: vtec.multiple_message,
            iod_ssr: vtec.iod_ssr,
            provider_id: vtec.provider_id,
            solution_id: vtec.solution_id,
            quality_indicator: vtec.quality_indicator,
            layer_count: vtec.layers.len(),
            trailing_bit_count: vtec.trailing_bits.len(),
        };
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_vtec_layers(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrVtecLayer,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_ssr_vtec_layers";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let vtec = match message {
            RtcmMessage::SsrVtec(vtec) => vtec,
            _ => return rtcm_wrong_kind(FN_NAME, "an SSR VTEC message"),
        };
        let layers = vtec
            .layers
            .iter()
            .map(|layer| SidereonRtcmSsrVtecLayer {
                height: layer.height,
                degree: layer.degree,
                order: layer.order,
                cosine_count: layer.cosine.len(),
                sine_count: layer.sine.len(),
            })
            .collect::<Vec<_>>();
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &layers,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_vtec_coefficients(
    messages: *const SidereonRtcmMessages,
    index: usize,
    layer_index: usize,
    cosine: bool,
    out: *mut i16,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_ssr_vtec_coefficients";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let vtec = match message {
            RtcmMessage::SsrVtec(vtec) => vtec,
            _ => return rtcm_wrong_kind(FN_NAME, "an SSR VTEC message"),
        };
        let layer = match vtec.layers.get(layer_index) {
            Some(layer) => layer,
            None => {
                set_last_error(format!(
                    "{FN_NAME}: layer index {layer_index} out of range ({} layers)",
                    vtec.layers.len()
                ));
                return SidereonStatus::InvalidArgument;
            }
        };
        let values = if cosine { &layer.cosine } else { &layer.sine };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            values,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_vtec_trailing_bits(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut bool,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_message_ssr_vtec_trailing_bits";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let message = c_try!(rtcm_message_at(FN_NAME, messages, index));
        let vtec = match message {
            RtcmMessage::SsrVtec(vtec) => vtec,
            _ => return rtcm_wrong_kind(FN_NAME, "an SSR VTEC message"),
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &vtec.trailing_bits,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_ssr_vtec(
    info: *const SidereonRtcmSsrVtecInfo,
    layers: *const SidereonRtcmSsrVtecLayer,
    layer_len: usize,
    cosine: *const i16,
    cosine_len: usize,
    sine: *const i16,
    sine_len: usize,
    trailing_bits: *const bool,
    trailing_bit_len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_ssr_vtec";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out = ptr::null_mut();
        let info = c_try!(require_ref(info, FN_NAME, "info"));
        let layer_records = c_try!(require_slice(layers, layer_len, FN_NAME, "layers"));
        let cosine_values = c_try!(require_slice(cosine, cosine_len, FN_NAME, "cosine"));
        let sine_values = c_try!(require_slice(sine, sine_len, FN_NAME, "sine"));
        let trailing_values = c_try!(require_slice(
            trailing_bits,
            trailing_bit_len,
            FN_NAME,
            "trailing_bits"
        ));
        if info.layer_count != layer_len || info.trailing_bit_count != trailing_bit_len {
            set_last_error(format!(
                "{FN_NAME}: declared layer/trailing-bit counts do not match the supplied arrays"
            ));
            return SidereonStatus::InvalidArgument;
        }
        let expected_cosine = layer_records.iter().try_fold(0_usize, |total, layer| {
            total.checked_add(layer.cosine_count)
        });
        let expected_sine = layer_records
            .iter()
            .try_fold(0_usize, |total, layer| total.checked_add(layer.sine_count));
        if expected_cosine != Some(cosine_len) || expected_sine != Some(sine_len) {
            set_last_error(format!(
                "{FN_NAME}: coefficient counts do not match the supplied arrays"
            ));
            return SidereonStatus::InvalidArgument;
        }
        let mut cosine_offset = 0;
        let mut sine_offset = 0;
        let mut core_layers = Vec::with_capacity(layer_records.len());
        for layer in layer_records {
            let cosine_end = cosine_offset + layer.cosine_count;
            let sine_end = sine_offset + layer.sine_count;
            core_layers.push(core_rtcm::SsrVtecLayer {
                height: layer.height,
                degree: layer.degree,
                order: layer.order,
                cosine: cosine_values[cosine_offset..cosine_end].to_vec(),
                sine: sine_values[sine_offset..sine_end].to_vec(),
            });
            cosine_offset = cosine_end;
            sine_offset = sine_end;
        }
        let message = core_rtcm::SsrVtecMessage {
            message_number: info.message_number,
            igs_ssr_version: info.has_igs_ssr_version.then_some(info.igs_ssr_version),
            epoch_time_s: info.epoch_time_s,
            update_interval: info.update_interval,
            multiple_message: info.multiple_message,
            iod_ssr: info.iod_ssr,
            provider_id: info.provider_id,
            solution_id: info.solution_id,
            quality_indicator: info.quality_indicator,
            layers: core_layers,
            trailing_bits: trailing_values.to_vec(),
        };
        rtcm_build(out, RtcmMessage::SsrVtec(message));
        SidereonStatus::Ok
    })
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
            c_try!(crate::ssr::ssr_copy_orbits(
                "sidereon_rtcm_message_ssr_orbits",
                ssr,
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
            c_try!(crate::ssr::ssr_copy_clocks(
                "sidereon_rtcm_message_ssr_clocks",
                ssr,
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
            c_try!(crate::ssr::ssr_copy_ura(
                "sidereon_rtcm_message_ssr_ura",
                ssr,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy an SSR message's per-satellite code-bias records. Each row exposes its
/// nested signal count. Values are raw wire integers. Variable-length output
/// contract.
///
/// Safety: messages is a live handle; out points to len
/// SidereonRtcmSsrCodeBiasRecord values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_code_biases(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrCodeBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_code_biases",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_code_biases",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_code_biases",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_ssr_code_biases",
                        "an SSR message",
                    );
                }
            };
            c_try!(crate::ssr::ssr_copy_code_biases(
                "sidereon_rtcm_message_ssr_code_biases",
                ssr,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the code-bias signal rows for one SSR satellite record. Values are raw
/// wire integers. Variable-length output contract.
///
/// Safety: messages is a live handle; out points to len
/// SidereonRtcmSsrCodeBiasSignal values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_code_bias_signals(
    messages: *const SidereonRtcmMessages,
    index: usize,
    record_index: usize,
    out: *mut SidereonRtcmSsrCodeBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_code_bias_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_code_bias_signals",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_code_bias_signals",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_ssr_code_bias_signals",
                        "an SSR message",
                    );
                }
            };
            c_try!(crate::ssr::ssr_copy_code_bias_signals(
                "sidereon_rtcm_message_ssr_code_bias_signals",
                ssr,
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

/// Copy an SSR message's per-satellite phase-bias records. Each row exposes
/// its raw yaw fields and nested signal count. Values are raw wire integers.
/// Variable-length output contract.
///
/// Safety: messages is a live handle; out points to len
/// SidereonRtcmSsrPhaseBiasRecord values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_phase_biases(
    messages: *const SidereonRtcmMessages,
    index: usize,
    out: *mut SidereonRtcmSsrPhaseBiasRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_phase_biases",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_phase_biases",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_phase_biases",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_ssr_phase_biases",
                        "an SSR message",
                    );
                }
            };
            c_try!(crate::ssr::ssr_copy_phase_biases(
                "sidereon_rtcm_message_ssr_phase_biases",
                ssr,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the phase-bias signal rows for one SSR satellite record. Values are raw
/// wire integers. Variable-length output contract.
///
/// Safety: messages is a live handle; out points to len
/// SidereonRtcmSsrPhaseBiasSignal values or is NULL when len is 0; out_written
/// and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_message_ssr_phase_bias_signals(
    messages: *const SidereonRtcmMessages,
    index: usize,
    record_index: usize,
    out: *mut SidereonRtcmSsrPhaseBiasSignal,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_rtcm_message_ssr_phase_bias_signals",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_rtcm_message_ssr_phase_bias_signals",
                out_written,
                out_required
            ));
            let message = c_try!(rtcm_message_at(
                "sidereon_rtcm_message_ssr_phase_bias_signals",
                messages,
                index
            ));
            let ssr = match message {
                RtcmMessage::Ssr(ssr) => ssr,
                _ => {
                    return rtcm_wrong_kind(
                        "sidereon_rtcm_message_ssr_phase_bias_signals",
                        "an SSR message",
                    );
                }
            };
            c_try!(crate::ssr::ssr_copy_phase_bias_signals(
                "sidereon_rtcm_message_ssr_phase_bias_signals",
                ssr,
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
    rtcm_operation_boundary(
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
    rtcm_operation_boundary(
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
                // A message built by hand carries no bits after its last field.
                trailing_bits: Vec::new(),
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
    rtcm_operation_boundary("sidereon_rtcm_build_msm", SidereonStatus::Panic, || {
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
            info.system,
        ));
        let kind = c_try!(rtcm_msm_kind_from_c_code(
            "sidereon_rtcm_build_msm",
            "info.kind",
            info.kind
        ));
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
        let signals: Vec<_> = raw_signals.iter().map(rtcm_msm_signal_from_c).collect();
        let signal_mask = if info.signal_mask == 0 {
            core_rtcm::msm_signal_mask(&signals)
        } else {
            info.signal_mask
        };
        let message = RtcmMsmMessage {
            message_number: info.message_number,
            system,
            kind,
            header: rtcm_msm_header_from_c(&info.header),
            signal_mask,
            satellites: raw_satellites
                .iter()
                .map(rtcm_msm_satellite_from_c)
                .collect(),
            signals,
            trailing_bits: Vec::new(),
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
    rtcm_operation_boundary(
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
    rtcm_operation_boundary(
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

/// Build a 1042 BeiDou broadcast ephemeris message from raw transmitted-integer
/// fields and wrap it in a single-element SidereonRtcmMessages handle. Release
/// with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmBeidouEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_beidou_ephemeris(
    eph: *const SidereonRtcmBeidouEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_beidou_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_beidou_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(
                eph,
                "sidereon_rtcm_build_beidou_ephemeris",
                "eph"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::BeidouEphemeris(rtcm_beidou_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a 1044 QZSS broadcast ephemeris message from raw transmitted-integer
/// fields and wrap it in a single-element SidereonRtcmMessages handle. Release
/// with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmQzssEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_qzss_ephemeris(
    eph: *const SidereonRtcmQzssEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_qzss_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_qzss_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(
                eph,
                "sidereon_rtcm_build_qzss_ephemeris",
                "eph"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::QzssEphemeris(rtcm_qzss_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a 1045 Galileo F/NAV broadcast ephemeris message from raw
/// transmitted-integer fields and wrap it in a single-element
/// SidereonRtcmMessages handle. Release with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmGalileoFnavEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_galileo_fnav_ephemeris(
    eph: *const SidereonRtcmGalileoFnavEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_galileo_fnav_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_galileo_fnav_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(
                eph,
                "sidereon_rtcm_build_galileo_fnav_ephemeris",
                "eph"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::GalileoFnavEphemeris(rtcm_galileo_fnav_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a 1046 Galileo I/NAV broadcast ephemeris message from raw
/// transmitted-integer fields and wrap it in a single-element
/// SidereonRtcmMessages handle. Release with sidereon_rtcm_messages_free.
///
/// Safety: eph points to a SidereonRtcmGalileoInavEphemeris; out_messages to a
/// SidereonRtcmMessages*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_galileo_inav_ephemeris(
    eph: *const SidereonRtcmGalileoInavEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    rtcm_operation_boundary(
        "sidereon_rtcm_build_galileo_inav_ephemeris",
        SidereonStatus::Panic,
        || {
            let out_messages = c_try!(require_out(
                out_messages,
                "sidereon_rtcm_build_galileo_inav_ephemeris",
                "out_messages"
            ));
            *out_messages = ptr::null_mut();
            let eph = c_try!(require_ref(
                eph,
                "sidereon_rtcm_build_galileo_inav_ephemeris",
                "eph"
            ));
            rtcm_build(
                out_messages,
                RtcmMessage::GalileoInavEphemeris(rtcm_galileo_inav_ephemeris_from_c(eph)),
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_navic_ephemeris(
    eph: *const SidereonRtcmNavicEphemeris,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_navic_ephemeris";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out_messages = ptr::null_mut();
        let eph = c_try!(require_ref(eph, FN_NAME, "eph"));
        rtcm_build(
            out_messages,
            RtcmMessage::NavicEphemeris(rtcm_navic_ephemeris_from_c(eph)),
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_build_navic_ephemeris_with_trailing_bits(
    eph: *const SidereonRtcmNavicEphemeris,
    trailing_bits: *const bool,
    trailing_bit_len: usize,
    out_messages: *mut *mut SidereonRtcmMessages,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_build_navic_ephemeris_with_trailing_bits";
    rtcm_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_messages = c_try!(require_out(out_messages, FN_NAME, "out_messages"));
        *out_messages = ptr::null_mut();
        let eph = c_try!(require_ref(eph, FN_NAME, "eph"));
        let trailing_bits = c_try!(require_slice(
            trailing_bits,
            trailing_bit_len,
            FN_NAME,
            "trailing_bits"
        ));
        let mut core_eph = rtcm_navic_ephemeris_from_c(eph);
        core_eph.trailing_bits = trailing_bits.to_vec();
        rtcm_build(out_messages, RtcmMessage::NavicEphemeris(core_eph));
        SidereonStatus::Ok
    })
}

// ============================================================================
// Universal-parity additions: capabilities the core exposes that this binding
// had not yet surfaced. Every function below is a thin extern-C wrapper that
// marshals C input into the cited sidereon-core / sidereon type, calls the
// reference entry point, and copies the result back. No modeling logic lives
// here.

fn map_rtcm_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    record_rtcm_typed_error(&err);
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        // The typed encoder, conversion and SBAS encoder refusals refuse the
        // caller's input, as the InvalidInput text they replace did; their
        // detail is in sidereon_rtcm_last_error_info and its payload.
        CoreError::InvalidInput(_)
        | CoreError::RtcmEncode(_)
        | CoreError::RtcmConversion(_)
        | CoreError::SbasEncode(_) => SidereonStatus::InvalidArgument,
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
    }
}

pub(crate) fn record_rtcm_typed_error(err: &CoreError) {
    let typed = match err {
        CoreError::RtcmEncode(error) => (
            SidereonRtcmErrorClass::Encode,
            rtcm_encode_error_kind(error) as u32,
        ),
        CoreError::RtcmConversion(error) => (
            SidereonRtcmErrorClass::Conversion,
            rtcm_conversion_error_kind(error),
        ),
        CoreError::SbasEncode(error) => (
            SidereonRtcmErrorClass::SbasEncode,
            sbas_encode_error_kind(error) as u32,
        ),
        _ => (SidereonRtcmErrorClass::Other, 0),
    };
    LAST_RTCM_TYPED_ERROR.with(|slot| {
        *slot.borrow_mut() = Some((typed.0, typed.1, rtcm_error_payload_json(err)));
    });
}

fn rtcm_error_payload_json(err: &CoreError) -> String {
    use core_rtcm::{RtcmConversionError as Conversion, RtcmEncodeError as Encode};
    let payload = match err {
        CoreError::RtcmEncode(error) => match error.as_ref() {
            Encode::FieldOutOfRange {
                message_number,
                field,
                value,
                width,
                encoding,
            } => {
                serde_json::json!({"variant":"FieldOutOfRange","message_number":message_number.to_string(),"field":field,"value":value.to_string(),"width":width.to_string(),"encoding":field_encoding_payload(*encoding)})
            }
            Encode::NegativeZeroWithValue {
                message_number,
                field,
                value,
            } => {
                serde_json::json!({"variant":"NegativeZeroWithValue","message_number":message_number.to_string(),"field":field,"value":value.to_string()})
            }
            Encode::NegativeZeroMask {
                message_number,
                mask,
            } => {
                serde_json::json!({"variant":"NegativeZeroMask","message_number":message_number.to_string(),"mask":mask.to_string()})
            }
            Encode::MessageNumber {
                message_number,
                record,
            } => {
                serde_json::json!({"variant":"MessageNumber","message_number":message_number.to_string(),"record":record_kind_payload(*record)})
            }
            Encode::FieldPresence {
                message_number,
                record,
                field,
                carried,
            } => {
                serde_json::json!({"variant":"FieldPresence","message_number":message_number.to_string(),"record":record_kind_payload(*record),"field":field,"carried":carried})
            }
            Encode::SatelliteFieldPresence {
                message_number,
                record,
                satellite,
                field,
                carried,
            } => {
                serde_json::json!({"variant":"SatelliteFieldPresence","message_number":message_number.to_string(),"record":record_kind_payload(*record),"satellite":satellite.to_string(),"field":field,"carried":carried})
            }
            Encode::CountMismatch {
                message_number,
                field,
                expected,
                actual,
            } => {
                serde_json::json!({"variant":"CountMismatch","message_number":message_number.to_string(),"field":field,"expected":expected.to_string(),"actual":actual.to_string()})
            }
            Encode::ValueOutOfRange {
                message_number,
                field,
                value,
                minimum,
                maximum,
            } => {
                serde_json::json!({"variant":"ValueOutOfRange","message_number":message_number.to_string(),"field":field,"value":value.to_string(),"minimum":minimum.to_string(),"maximum":maximum.to_string()})
            }
            Encode::NonLatin1Character { field, character } => {
                serde_json::json!({"variant":"NonLatin1Character","field":field,"character":character.to_string(),"codepoint":u32::from(*character).to_string()})
            }
            Encode::SatelliteIdOutOfRange {
                message_number,
                field,
                value,
                width,
            } => {
                serde_json::json!({"variant":"SatelliteIdOutOfRange","message_number":message_number.to_string(),"field":field,"value":value.to_string(),"width":width.to_string()})
            }
            Encode::SsrSatelliteIdOutOfRange {
                message_number,
                value,
                width,
            } => {
                serde_json::json!({"variant":"SsrSatelliteIdOutOfRange","message_number":message_number.to_string(),"value":value.to_string(),"width":width.to_string()})
            }
            Encode::SsrRecordsNotCarried {
                message_number,
                kind,
                records,
                count,
            } => {
                serde_json::json!({"variant":"SsrRecordsNotCarried","message_number":message_number.to_string(),"kind":ssr_kind_payload(*kind),"records":records,"count":count.to_string()})
            }
            Encode::SsrCombinedRecordCounts {
                message_number,
                orbit,
                clock,
            } => {
                serde_json::json!({"variant":"SsrCombinedRecordCounts","message_number":message_number.to_string(),"orbit":orbit.to_string(),"clock":clock.to_string()})
            }
            Encode::SsrCombinedSatelliteMismatch {
                message_number,
                index,
                orbit_satellite,
                clock_satellite,
            } => {
                serde_json::json!({"variant":"SsrCombinedSatelliteMismatch","message_number":message_number.to_string(),"index":index.to_string(),"orbit_satellite":orbit_satellite.to_string(),"clock_satellite":clock_satellite.to_string()})
            }
            Encode::SsrHighRateClockTerms {
                message_number,
                satellite,
                c1,
                c2,
            } => {
                serde_json::json!({"variant":"SsrHighRateClockTerms","message_number":message_number.to_string(),"satellite":satellite.to_string(),"c1":c1.to_string(),"c2":c2.to_string()})
            }
            Encode::SsrSatelliteCount {
                message_number,
                declared,
                records,
            } => {
                serde_json::json!({"variant":"SsrSatelliteCount","message_number":message_number.to_string(),"declared":declared.to_string(),"records":records.to_string()})
            }
            Encode::MsmMask {
                message_number,
                problem,
            } => {
                serde_json::json!({"variant":"MsmMask","message_number":message_number.to_string(),"problem":msm_mask_problem_payload(*problem)})
            }
            Encode::MsmOptional {
                message_number,
                kind,
                satellite,
                signal,
                field,
                problem,
            } => {
                serde_json::json!({"variant":"MsmOptional","message_number":message_number.to_string(),"kind":msm_kind_payload(*kind),"satellite":satellite.to_string(),"signal":signal.map(|value| value.to_string()),"field":msm_optional_field_payload(*field),"problem":msm_optional_problem_payload(*problem)})
            }
            Encode::TrailingZeroBits {
                message_number,
                bits,
            } => {
                serde_json::json!({"variant":"TrailingZeroBits","message_number":message_number.to_string(),"bits":bits.to_string()})
            }
            Encode::StrictDeparture(departure) => {
                serde_json::json!({"variant":"StrictDeparture","departure":departure_payload(departure)})
            }
            Encode::UnsupportedBodyTooShort { message_number } => {
                serde_json::json!({"variant":"UnsupportedBodyTooShort","message_number":message_number.to_string()})
            }
            Encode::UnsupportedBodyNumber {
                message_number,
                carried,
            } => {
                serde_json::json!({"variant":"UnsupportedBodyNumber","message_number":message_number.to_string(),"carried":carried.to_string()})
            }
            Encode::UnsupportedDecodedNumber { message_number } => {
                serde_json::json!({"variant":"UnsupportedDecodedNumber","message_number":message_number.to_string()})
            }
            Encode::FrameBodyTooLong { len } => {
                serde_json::json!({"variant":"FrameBodyTooLong","len":len.to_string()})
            }
            Encode::FrameReservedOutOfRange { value } => {
                serde_json::json!({"variant":"FrameReservedOutOfRange","value":value.to_string()})
            }
            _ => serde_json::json!({"variant":"UnknownEncodeError"}),
        },
        CoreError::RtcmConversion(error) => match error.as_ref() {
            Conversion::SatelliteIdOutOfRange {
                message_number,
                field,
                value,
                width,
            } => {
                serde_json::json!({"variant":"SatelliteIdOutOfRange","message_number":message_number.to_string(),"field":field,"value":value.to_string(),"width":width.to_string()})
            }
            Conversion::InvalidSatellite {
                message_number,
                field,
                value,
                error,
            } => {
                serde_json::json!({"variant":"InvalidSatellite","message_number":message_number.to_string(),"field":field,"value":value.to_string(),"error":satellite_id_error_payload(*error)})
            }
            Conversion::SbasPrnOutsideWindow {
                value,
                broadcast_prn,
            } => {
                serde_json::json!({"variant":"SbasPrnOutsideWindow","value":value.to_string(),"broadcast_prn":broadcast_prn.to_string()})
            }
            Conversion::NoLnavRecord { value, satellite } => {
                serde_json::json!({"variant":"NoLnavRecord","value":value.to_string(),"satellite":satellite_payload(*satellite)})
            }
            Conversion::WeekMismatch {
                message_number,
                full_week,
                week,
            } => {
                serde_json::json!({"variant":"WeekMismatch","message_number":message_number.to_string(),"full_week":full_week.to_string(),"week":week.to_string()})
            }
            Conversion::NavicWeekMismatch { full_week, week } => {
                serde_json::json!({"variant":"NavicWeekMismatch","full_week":full_week.to_string(),"week":week.to_string()})
            }
            Conversion::TimeNotRepresentable { field } => {
                serde_json::json!({"variant":"TimeNotRepresentable","field":field})
            }
            Conversion::GalileoWeekOverflow => serde_json::json!({"variant":"GalileoWeekOverflow"}),
            Conversion::SisaSpare { index } => {
                serde_json::json!({"variant":"SisaSpare","index":index.to_string()})
            }
            Conversion::SisaNoPrediction => serde_json::json!({"variant":"SisaNoPrediction"}),
            Conversion::UraOutOfRange { system, index } => {
                serde_json::json!({"variant":"UraOutOfRange","system":system.as_str(),"index":index.to_string()})
            }
            Conversion::UraNoPrediction { system, index } => {
                serde_json::json!({"variant":"UraNoPrediction","system":system.as_str(),"index":index.to_string()})
            }
            Conversion::FitInterval(error) => {
                serde_json::json!({"variant":"FitInterval","error":lnav_record_error_payload(*error)})
            }
            Conversion::VtecEvaluation(problem) => {
                serde_json::json!({"variant":"VtecEvaluation","problem":vtec_evaluation_problem_payload(problem)})
            }
            _ => serde_json::json!({"variant":"UnknownConversionError"}),
        },
        CoreError::SbasEncode(error) => sbas_encode_error_payload(error),
        _ => serde_json::json!({"variant":"OtherError"}),
    };
    serde_json::json!({"schema_version":1,"error":payload}).to_string()
}

fn sbas_encode_error_kind(
    error: &sidereon_core::sbas::SbasEncodeError,
) -> SidereonSbasEncodeErrorKind {
    use sidereon_core::sbas::SbasEncodeError as Error;
    match error {
        Error::FieldOutOfRange { .. } => SidereonSbasEncodeErrorKind::FieldOutOfRange,
        Error::UnrecognizedPreamble { .. } => SidereonSbasEncodeErrorKind::UnrecognizedPreamble,
        Error::MessageType { .. } => SidereonSbasEncodeErrorKind::MessageType,
        Error::RawPayload { .. } => SidereonSbasEncodeErrorKind::RawPayload,
        Error::ReservedLayout { .. } => SidereonSbasEncodeErrorKind::ReservedLayout,
        Error::LongTermRecordCount { .. } => SidereonSbasEncodeErrorKind::LongTermRecordCount,
        Error::LongTermFieldNotCarried { .. } => {
            SidereonSbasEncodeErrorKind::LongTermFieldNotCarried
        }
        Error::LongTermMissingTimeOfDay { .. } => {
            SidereonSbasEncodeErrorKind::LongTermMissingTimeOfDay
        }
        Error::PadBits { .. } => SidereonSbasEncodeErrorKind::PadBits,
        _ => SidereonSbasEncodeErrorKind::Unknown,
    }
}

fn sbas_encode_error_payload(error: &sidereon_core::sbas::SbasEncodeError) -> serde_json::Value {
    use sidereon_core::sbas::SbasEncodeError as Error;
    match error {
        Error::FieldOutOfRange {
            message_type,
            field,
            index,
            value,
            width,
            signed,
        } => {
            serde_json::json!({"variant":"FieldOutOfRange","message_type":message_type.to_string(),"field":field,"index":index.map(|value| value.to_string()),"value":value.to_string(),"width":width.to_string(),"signed":signed})
        }
        Error::UnrecognizedPreamble { preamble } => {
            serde_json::json!({"variant":"UnrecognizedPreamble","preamble":preamble.to_string()})
        }
        Error::MessageType {
            message_type,
            reason,
        } => {
            serde_json::json!({"variant":"MessageType","message_type":message_type.to_string(),"reason":reason})
        }
        Error::RawPayload {
            message_type,
            bytes,
            bits_past_payload,
        } => {
            serde_json::json!({"variant":"RawPayload","message_type":message_type.to_string(),"bytes":bytes.to_string(),"bits_past_payload":bits_past_payload})
        }
        Error::ReservedLayout {
            message_type,
            part,
            expected,
            found,
        } => {
            serde_json::json!({"variant":"ReservedLayout","message_type":message_type.to_string(),"part":part,"expected":expected.iter().map(u8::to_string).collect::<Vec<_>>(),"found":found.iter().map(u8::to_string).collect::<Vec<_>>()})
        }
        Error::LongTermRecordCount {
            message_type,
            half,
            velocity_code,
            expected,
            found,
        } => {
            serde_json::json!({"variant":"LongTermRecordCount","message_type":message_type.to_string(),"half":half.to_string(),"velocity_code":velocity_code,"expected":expected.to_string(),"found":found.to_string()})
        }
        Error::LongTermFieldNotCarried {
            message_type,
            half,
            record,
            field,
        } => {
            serde_json::json!({"variant":"LongTermFieldNotCarried","message_type":message_type.to_string(),"half":half.to_string(),"record":record.to_string(),"field":field})
        }
        Error::LongTermMissingTimeOfDay { message_type, half } => {
            serde_json::json!({"variant":"LongTermMissingTimeOfDay","message_type":message_type.to_string(),"half":half.to_string()})
        }
        Error::PadBits { value } => {
            serde_json::json!({"variant":"PadBits","value":value.to_string()})
        }
        other => {
            serde_json::json!({"variant":"UnknownSbasEncodeError","message":other.to_string()})
        }
    }
}

pub(crate) fn clear_rtcm_typed_error() {
    LAST_RTCM_TYPED_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Start a new RTCM-producing operation with no typed result from an earlier
/// operation on this thread. Error-info and payload accessors use the ordinary
/// panic boundary so callers can inspect the last recorded typed error.
fn rtcm_operation_boundary<T>(fn_name: &str, panic_value: T, body: impl FnOnce() -> T) -> T {
    clear_rtcm_typed_error();
    ffi_boundary(fn_name, panic_value, body)
}

fn field_encoding_payload(encoding: core_rtcm::RtcmFieldEncoding) -> &'static str {
    use core_rtcm::RtcmFieldEncoding as Encoding;
    match encoding {
        Encoding::Unsigned => "Unsigned",
        Encoding::TwosComplement => "TwosComplement",
        Encoding::SignMagnitude => "SignMagnitude",
        _ => "Unknown",
    }
}

fn record_kind_payload(record: core_rtcm::RtcmRecordKind) -> serde_json::Value {
    use core_rtcm::RtcmRecordKind as Record;
    match record {
        Record::StationCoordinates => serde_json::json!({"variant":"StationCoordinates"}),
        Record::AntennaDescriptor => serde_json::json!({"variant":"AntennaDescriptor"}),
        Record::Msm { system, kind } => {
            serde_json::json!({"variant":"Msm","system":system.as_str(),"kind":msm_kind_payload(kind)})
        }
        Record::Ssr { system, kind } => {
            serde_json::json!({"variant":"Ssr","system":system.as_str(),"kind":ssr_kind_payload(kind)})
        }
        Record::LegacyObservations => serde_json::json!({"variant":"LegacyObservations"}),
        Record::SystemParameters => serde_json::json!({"variant":"SystemParameters"}),
        Record::Text => serde_json::json!({"variant":"Text"}),
        Record::Network { family } => {
            serde_json::json!({"variant":"Network","family":family})
        }
        Record::Transformation { family } => {
            serde_json::json!({"variant":"Transformation","family":family})
        }
        Record::GlonassCodePhaseBiases => {
            serde_json::json!({"variant":"GlonassCodePhaseBiases"})
        }
        Record::SsrVtec { message_number } => {
            serde_json::json!({"variant":"SsrVtec","message_number":message_number.to_string()})
        }
        _ => serde_json::json!({"variant":"UnknownRecordKind"}),
    }
}

fn msm_kind_payload(kind: core_rtcm::MsmKind) -> &'static str {
    match kind {
        core_rtcm::MsmKind::Msm1 => "MSM1",
        core_rtcm::MsmKind::Msm2 => "MSM2",
        core_rtcm::MsmKind::Msm3 => "MSM3",
        core_rtcm::MsmKind::Msm4 => "MSM4",
        core_rtcm::MsmKind::Msm5 => "MSM5",
        core_rtcm::MsmKind::Msm6 => "MSM6",
        core_rtcm::MsmKind::Msm7 => "MSM7",
    }
}

fn ssr_kind_payload(kind: core_rtcm::SsrKind) -> &'static str {
    match kind {
        core_rtcm::SsrKind::Orbit => "Orbit",
        core_rtcm::SsrKind::Clock => "Clock",
        core_rtcm::SsrKind::CombinedOrbitClock => "CombinedOrbitClock",
        core_rtcm::SsrKind::CodeBias => "CodeBias",
        core_rtcm::SsrKind::PhaseBias => "PhaseBias",
        core_rtcm::SsrKind::Ura => "Ura",
        core_rtcm::SsrKind::HighRateClock => "HighRateClock",
    }
}

fn msm_optional_field_payload(field: core_rtcm::MsmOptionalField) -> &'static str {
    use core_rtcm::MsmOptionalField as Field;
    match field {
        Field::ExtendedInfo => "ExtendedInfo",
        Field::RoughPhaseRangeRate => "RoughPhaseRangeRate",
        Field::FinePhaseRangeRate => "FinePhaseRangeRate",
        _ => "Unknown",
    }
}

fn msm_optional_problem_payload(problem: core_rtcm::MsmOptionalProblem) -> serde_json::Value {
    use core_rtcm::MsmOptionalProblem as Problem;
    match problem {
        Problem::Missing => serde_json::json!({"variant":"Missing"}),
        Problem::NotCarried => serde_json::json!({"variant":"NotCarried"}),
        Problem::InvalidValue(value) => {
            serde_json::json!({"variant":"InvalidValue","value":value.to_string()})
        }
        _ => serde_json::json!({"variant":"Unknown"}),
    }
}

fn msm_mask_problem_payload(problem: core_rtcm::MsmMaskProblem) -> serde_json::Value {
    use core_rtcm::MsmMaskProblem as Problem;
    match problem {
        Problem::SatelliteOutsideMask { satellite } => {
            serde_json::json!({"variant":"SatelliteOutsideMask","satellite":satellite.to_string()})
        }
        Problem::SatelliteListedTwice { satellite } => {
            serde_json::json!({"variant":"SatelliteListedTwice","satellite":satellite.to_string()})
        }
        Problem::SignalOutsideMask { signal } => {
            serde_json::json!({"variant":"SignalOutsideMask","signal":signal.to_string()})
        }
        Problem::SignalNotInMask { signal, mask } => {
            serde_json::json!({"variant":"SignalNotInMask","signal":signal.to_string(),"mask":mask.to_string()})
        }
        Problem::SignalSatelliteNotListed { signal, satellite } => {
            serde_json::json!({"variant":"SignalSatelliteNotListed","signal":signal.to_string(),"satellite":satellite.to_string()})
        }
        Problem::CellListedTwice { satellite, signal } => {
            serde_json::json!({"variant":"CellListedTwice","satellite":satellite.to_string(),"signal":signal.to_string()})
        }
        _ => serde_json::json!({"variant":"Unknown"}),
    }
}

fn departure_payload(departure: &core_rtcm::RtcmDeparture) -> serde_json::Value {
    use core_rtcm::RtcmDeparture as Departure;
    match departure {
        Departure::FrameReservedBits { reserved } => {
            serde_json::json!({"variant":"FrameReservedBits","reserved":reserved.to_string()})
        }
        Departure::TrailingBits {
            message_number,
            bits,
        } => {
            serde_json::json!({"variant":"TrailingBits","message_number":message_number.to_string(),"bits":bits})
        }
        Departure::MsmCellMaskOver64 {
            message_number,
            cells,
        } => {
            serde_json::json!({"variant":"MsmCellMaskOver64","message_number":message_number.to_string(),"cells":cells.to_string()})
        }
        Departure::OrderExceedsDegree {
            message_number,
            layer_index,
            degree,
            order,
        } => {
            serde_json::json!({"variant":"OrderExceedsDegree","message_number":message_number.to_string(),"layer_index":layer_index.to_string(),"degree":degree.to_string(),"order":order.to_string()})
        }
        Departure::SsrRecordsShort {
            message_number,
            declared,
            read,
        } => {
            serde_json::json!({"variant":"SsrRecordsShort","message_number":message_number.to_string(),"declared":declared.to_string(),"read":read.to_string()})
        }
        Departure::RecordsShort {
            message_number,
            declared,
            read,
        } => {
            serde_json::json!({"variant":"RecordsShort","message_number":message_number.to_string(),"declared":declared.to_string(),"read":read.to_string()})
        }
        _ => serde_json::json!({"variant":"UnknownDeparture"}),
    }
}

fn satellite_payload(satellite: sidereon_core::GnssSatelliteId) -> serde_json::Value {
    serde_json::json!({"system":satellite.system.as_str(),"prn":satellite.prn.to_string()})
}

fn satellite_id_error_payload(error: sidereon_core::SatelliteIdError) -> serde_json::Value {
    match error {
        sidereon_core::SatelliteIdError::InvalidInput { field, reason } => {
            serde_json::json!({"variant":"InvalidInput","field":field,"reason":reason})
        }
    }
}

fn lnav_record_error_payload(
    error: sidereon_core::ephemeris::LnavRecordError,
) -> serde_json::Value {
    use sidereon_core::ephemeris::LnavRecordError as Error;
    match error {
        Error::NotGps(satellite) => {
            serde_json::json!({"variant":"NotGps","satellite":satellite_payload(satellite)})
        }
        Error::InvalidEpoch(field) => serde_json::json!({"variant":"InvalidEpoch","field":field}),
        Error::WeekMismatch {
            full_week,
            decoded_week,
        } => {
            serde_json::json!({"variant":"WeekMismatch","full_week":full_week.to_string(),"decoded_week":decoded_week.to_string()})
        }
        Error::NoUraPrediction(index) => {
            serde_json::json!({"variant":"NoUraPrediction","index":index.to_string()})
        }
        Error::FitIntervalUnsupported {
            fit_interval_flag,
            iode,
            iodc,
        } => {
            serde_json::json!({"variant":"FitIntervalUnsupported","fit_interval_flag":fit_interval_flag.to_string(),"iode":iode.to_string(),"iodc":iodc.to_string()})
        }
    }
}

fn vtec_evaluation_problem_payload(
    problem: &core_rtcm::VtecEvaluationProblem,
) -> serde_json::Value {
    use core_rtcm::VtecEvaluationProblem as Problem;
    match problem {
        Problem::ComputationTime => serde_json::json!({"variant":"ComputationTime"}),
        Problem::Frequency => serde_json::json!({"variant":"Frequency"}),
        Problem::NonFiniteCoordinates => serde_json::json!({"variant":"NonFiniteCoordinates"}),
        Problem::MessageIdentity { message_number } => {
            serde_json::json!({"variant":"MessageIdentity","message_number":message_number.to_string()})
        }
        Problem::LayerCount { layers } => {
            serde_json::json!({"variant":"LayerCount","layers":layers.to_string()})
        }
        Problem::InvalidGeometry => serde_json::json!({"variant":"InvalidGeometry"}),
        Problem::BelowHorizon => serde_json::json!({"variant":"BelowHorizon"}),
        Problem::LayerDegreeOrder {
            layer_index,
            degree,
            order,
        } => {
            serde_json::json!({"variant":"LayerDegreeOrder","layer_index":layer_index.to_string(),"degree":degree.to_string(),"order":order.to_string()})
        }
        Problem::CoefficientCounts {
            layer_index,
            cosine_expected,
            cosine_actual,
            sine_expected,
            sine_actual,
        } => {
            serde_json::json!({"variant":"CoefficientCounts","layer_index":layer_index.to_string(),"cosine_expected":cosine_expected.to_string(),"cosine_actual":cosine_actual.to_string(),"sine_expected":sine_expected.to_string(),"sine_actual":sine_actual.to_string()})
        }
        Problem::UnavailableCoefficient { layer_index } => {
            serde_json::json!({"variant":"UnavailableCoefficient","layer_index":layer_index.to_string()})
        }
        Problem::ShellNotAboveReceiver { layer_index } => {
            serde_json::json!({"variant":"ShellNotAboveReceiver","layer_index":layer_index.to_string()})
        }
        Problem::MissingCoefficient {
            layer_index,
            field,
            index,
        } => {
            serde_json::json!({"variant":"MissingCoefficient","layer_index":layer_index.to_string(),"field":field,"index":index.to_string()})
        }
        Problem::InvalidMappingFactor { layer_index } => {
            serde_json::json!({"variant":"InvalidMappingFactor","layer_index":layer_index.to_string()})
        }
        Problem::PhysicalResultOutOfRange { field } => {
            serde_json::json!({"variant":"PhysicalResultOutOfRange","field":field})
        }
        _ => serde_json::json!({"variant":"UnknownVtecEvaluationProblem"}),
    }
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_last_error_info(
    out: *mut SidereonRtcmErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_last_error_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        LAST_RTCM_TYPED_ERROR.with(|slot| {
            if let Some((class, kind, payload)) = slot.borrow().as_ref() {
                *out = SidereonRtcmErrorInfo {
                    class: *class,
                    kind: *kind,
                    payload_len: payload.len(),
                };
            } else {
                *out = SidereonRtcmErrorInfo {
                    class: SidereonRtcmErrorClass::None,
                    kind: 0,
                    payload_len: 0,
                };
            }
        });
        SidereonStatus::Ok
    })
}

/// Copies the JSON payload for the most recent thread-local RTCM encode/conversion or SBAS encode error.
/// The payload belongs to the calling OS thread; querying it does not clear it.
///
/// The payload is UTF-8 JSON with root object
/// `{"schema_version":1,"error":{"variant":"...","field":"..."}}`. The `error` value is a structured
/// object whose `variant` string identifies its case and whose other fields
/// carry that variant's data; nested errors are structured objects, not
/// debug-formatted text. Every integer-valued payload field is a base-10
/// decimal string, preserving its exact value. Floating-point quantities are
/// JSON numbers with units documented by their field names and error variant.
/// Booleans and textual fields use
/// their corresponding JSON types. The version and field encodings are part
/// of this C API. The payload is not NUL terminated; callers can query the
/// required byte count with a null output and zero length, allocate storage,
/// and call again.
///
/// # Safety
/// `out_written` and `out_required` must be valid writable pointers. `out` must
/// point to `len` writable bytes unless `len` is zero.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtcm_last_error_payload(
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rtcm_last_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let payload = LAST_RTCM_TYPED_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(String::new, |(_, _, payload)| payload.clone())
        });
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            payload.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn rtcm_encode_error_kind(error: &core_rtcm::RtcmEncodeError) -> SidereonRtcmEncodeErrorKind {
    use core_rtcm::RtcmEncodeError as Error;
    match error {
        Error::FieldOutOfRange { .. } => SidereonRtcmEncodeErrorKind::FieldOutOfRange,
        Error::NegativeZeroWithValue { .. } => SidereonRtcmEncodeErrorKind::NegativeZeroWithValue,
        Error::NegativeZeroMask { .. } => SidereonRtcmEncodeErrorKind::NegativeZeroMask,
        Error::MessageNumber { .. } => SidereonRtcmEncodeErrorKind::MessageNumber,
        Error::FieldPresence { .. } => SidereonRtcmEncodeErrorKind::FieldPresence,
        Error::SatelliteFieldPresence { .. } => SidereonRtcmEncodeErrorKind::SatelliteFieldPresence,
        Error::CountMismatch { .. } => SidereonRtcmEncodeErrorKind::CountMismatch,
        Error::ValueOutOfRange { .. } => SidereonRtcmEncodeErrorKind::ValueOutOfRange,
        Error::NonLatin1Character { .. } => SidereonRtcmEncodeErrorKind::NonLatin1Character,
        Error::SatelliteIdOutOfRange { .. } => SidereonRtcmEncodeErrorKind::SatelliteIdOutOfRange,
        Error::SsrSatelliteIdOutOfRange { .. } => {
            SidereonRtcmEncodeErrorKind::SsrSatelliteIdOutOfRange
        }
        Error::SsrRecordsNotCarried { .. } => SidereonRtcmEncodeErrorKind::SsrRecordsNotCarried,
        Error::SsrCombinedRecordCounts { .. } => {
            SidereonRtcmEncodeErrorKind::SsrCombinedRecordCounts
        }
        Error::SsrCombinedSatelliteMismatch { .. } => {
            SidereonRtcmEncodeErrorKind::SsrCombinedSatelliteMismatch
        }
        Error::SsrHighRateClockTerms { .. } => SidereonRtcmEncodeErrorKind::SsrHighRateClockTerms,
        Error::SsrSatelliteCount { .. } => SidereonRtcmEncodeErrorKind::SsrSatelliteCount,
        Error::MsmMask { .. } => SidereonRtcmEncodeErrorKind::MsmMask,
        Error::MsmOptional { .. } => SidereonRtcmEncodeErrorKind::MsmOptional,
        Error::TrailingZeroBits { .. } => SidereonRtcmEncodeErrorKind::TrailingZeroBits,
        Error::StrictDeparture(_) => SidereonRtcmEncodeErrorKind::StrictDeparture,
        Error::UnsupportedBodyTooShort { .. } => {
            SidereonRtcmEncodeErrorKind::UnsupportedBodyTooShort
        }
        Error::UnsupportedBodyNumber { .. } => SidereonRtcmEncodeErrorKind::UnsupportedBodyNumber,
        Error::UnsupportedDecodedNumber { .. } => {
            SidereonRtcmEncodeErrorKind::UnsupportedDecodedNumber
        }
        Error::FrameBodyTooLong { .. } => SidereonRtcmEncodeErrorKind::FrameBodyTooLong,
        Error::FrameReservedOutOfRange { .. } => {
            SidereonRtcmEncodeErrorKind::FrameReservedOutOfRange
        }
        _ => SidereonRtcmEncodeErrorKind::Other,
    }
}

fn rtcm_conversion_error_kind(error: &core_rtcm::RtcmConversionError) -> u32 {
    use core_rtcm::RtcmConversionError as Error;
    match error {
        Error::SatelliteIdOutOfRange { .. } => {
            SidereonRtcmConversionErrorKind::SatelliteIdOutOfRange as u32
        }
        Error::InvalidSatellite { .. } => SidereonRtcmConversionErrorKind::InvalidSatellite as u32,
        Error::SbasPrnOutsideWindow { .. } => {
            SidereonRtcmConversionErrorKind::SbasPrnOutsideWindow as u32
        }
        Error::NoLnavRecord { .. } => SidereonRtcmConversionErrorKind::NoLnavRecord as u32,
        Error::WeekMismatch { .. } => SidereonRtcmConversionErrorKind::WeekMismatch as u32,
        Error::NavicWeekMismatch { .. } => {
            SidereonRtcmConversionErrorKind::NavicWeekMismatch as u32
        }
        Error::TimeNotRepresentable { .. } => {
            SidereonRtcmConversionErrorKind::TimeNotRepresentable as u32
        }
        Error::GalileoWeekOverflow => SidereonRtcmConversionErrorKind::GalileoWeekOverflow as u32,
        Error::SisaSpare { .. } => SidereonRtcmConversionErrorKind::SisaSpare as u32,
        Error::SisaNoPrediction => SidereonRtcmConversionErrorKind::SisaNoPrediction as u32,
        Error::UraOutOfRange { .. } => SidereonRtcmConversionErrorKind::UraOutOfRange as u32,
        Error::UraNoPrediction { .. } => SidereonRtcmConversionErrorKind::UraNoPrediction as u32,
        Error::FitInterval(_) => SidereonRtcmConversionErrorKind::FitInterval as u32,
        Error::VtecEvaluation(_) => SidereonRtcmConversionErrorKind::VtecEvaluation as u32,
        _ => SidereonRtcmConversionErrorKind::Other as u32,
    }
}

unsafe fn copy_departures_to_c(
    fn_name: &str,
    departures: &[core_rtcm::RtcmDeparture],
    out: *mut SidereonRtcmDeparture,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let count = departures.len();
    super::validate_element_count::<SidereonRtcmDeparture>(fn_name, "departures", count)?;
    if !out_written.is_null() && !out_required.is_null() {
        let written = super::checked_output_range(fn_name, out_written, 1, "departures_written")?;
        let required =
            super::checked_output_range(fn_name, out_required, 1, "departures_required")?;
        super::reject_overlapping_outputs(
            fn_name,
            written,
            required,
            "departures_written",
            "departures_required",
        )?;
    }
    let capacity_is_valid =
        capacity <= isize::MAX as usize / size_of::<SidereonRtcmDeparture>().max(1);
    if count != 0 && capacity_is_valid && capacity >= count && !out.is_null() {
        let data = super::checked_output_range(fn_name, out, count, "out_departures")?;
        if !out_written.is_null() {
            let written =
                super::checked_output_range(fn_name, out_written, 1, "departures_written")?;
            super::reject_overlapping_outputs(
                fn_name,
                written,
                data,
                "departures_written",
                "out_departures",
            )?;
        }
        if !out_required.is_null() {
            let required =
                super::checked_output_range(fn_name, out_required, 1, "departures_required")?;
            super::reject_overlapping_outputs(
                fn_name,
                required,
                data,
                "departures_required",
                "out_departures",
            )?;
        }
    }
    // Do not form mutable references until every output range has been checked.
    let written = require_out(out_written, fn_name, "departures_written")?;
    let required = require_out(out_required, fn_name, "departures_required")?;
    *written = 0;
    *required = count;
    super::validate_element_count::<SidereonRtcmDeparture>(fn_name, "capacity", capacity)?;
    if capacity < count {
        set_last_error(format!(
            "{fn_name}: out_departures needs room for {count} entries"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    if count != 0 && out.is_null() {
        set_last_error(format!("{fn_name}: null out_departures"));
        return Err(SidereonStatus::InvalidArgument);
    }
    for (index, departure) in departures.iter().enumerate() {
        let mapped = match departure {
            core_rtcm::RtcmDeparture::FrameReservedBits { reserved } => SidereonRtcmDeparture {
                kind: 0,
                reserved: *reserved,
                ..SidereonRtcmDeparture::default()
            },
            core_rtcm::RtcmDeparture::TrailingBits {
                message_number,
                bits,
            } => SidereonRtcmDeparture {
                kind: 1,
                message_number: *message_number,
                bit_count: bits.len(),
                ..SidereonRtcmDeparture::default()
            },
            core_rtcm::RtcmDeparture::MsmCellMaskOver64 {
                message_number,
                cells,
            } => SidereonRtcmDeparture {
                kind: 2,
                message_number: *message_number,
                cells: *cells,
                ..SidereonRtcmDeparture::default()
            },
            core_rtcm::RtcmDeparture::OrderExceedsDegree {
                message_number,
                layer_index,
                degree,
                order,
            } => SidereonRtcmDeparture {
                kind: 3,
                message_number: *message_number,
                layer_index: *layer_index,
                degree: *degree,
                order: *order,
                ..SidereonRtcmDeparture::default()
            },
            core_rtcm::RtcmDeparture::SsrRecordsShort {
                message_number,
                declared,
                read,
            } => SidereonRtcmDeparture {
                kind: 4,
                message_number: *message_number,
                declared: *declared,
                read: *read,
                ..SidereonRtcmDeparture::default()
            },
            core_rtcm::RtcmDeparture::RecordsShort {
                message_number,
                declared,
                read,
            } => SidereonRtcmDeparture {
                kind: 5,
                message_number: *message_number,
                declared: *declared,
                read: *read,
                ..SidereonRtcmDeparture::default()
            },
            _ => {
                set_last_error(format!("{fn_name}: unrepresentable RTCM departure"));
                return Err(SidereonStatus::InvalidArgument);
            }
        };
        unsafe { out.add(index).write(mapped) };
    }
    *written = departures.len();
    Ok(())
}

fn rtcm_wrong_kind(fn_name: &str, expected: &str) -> SidereonStatus {
    set_last_error(format!("{fn_name}: message is not {expected}"));
    SidereonStatus::InvalidArgument
}

fn rtcm_message_kind_of(message: &RtcmMessage) -> SidereonRtcmMessageKind {
    match message {
        RtcmMessage::Msm(_) => SidereonRtcmMessageKind::Msm,
        RtcmMessage::LegacyObservations(_) => SidereonRtcmMessageKind::LegacyObservations,
        RtcmMessage::StationCoordinates(_) => SidereonRtcmMessageKind::StationCoordinates,
        RtcmMessage::AntennaDescriptor(_) => SidereonRtcmMessageKind::AntennaDescriptor,
        RtcmMessage::SystemParameters(_) => SidereonRtcmMessageKind::SystemParameters,
        RtcmMessage::Text(_) => SidereonRtcmMessageKind::Text,
        RtcmMessage::NetworkAuxiliaryStation(_)
        | RtcmMessage::NetworkCorrectionDifferences(_)
        | RtcmMessage::NetworkResiduals(_)
        | RtcmMessage::PhysicalReferenceStation(_)
        | RtcmMessage::FkpGradients(_) => SidereonRtcmMessageKind::Network,
        RtcmMessage::HelmertTransformation(_)
        | RtcmMessage::ResidualGrid(_)
        | RtcmMessage::Projection(_) => SidereonRtcmMessageKind::Transformation,
        RtcmMessage::GpsEphemeris(_) => SidereonRtcmMessageKind::GpsEphemeris,
        RtcmMessage::GlonassEphemeris(_) => SidereonRtcmMessageKind::GlonassEphemeris,
        RtcmMessage::NavicEphemeris(_) => SidereonRtcmMessageKind::NavicEphemeris,
        RtcmMessage::BeidouEphemeris(_) => SidereonRtcmMessageKind::BeidouEphemeris,
        RtcmMessage::QzssEphemeris(_) => SidereonRtcmMessageKind::QzssEphemeris,
        RtcmMessage::GalileoFnavEphemeris(_) => SidereonRtcmMessageKind::GalileoFnavEphemeris,
        RtcmMessage::GalileoInavEphemeris(_) => SidereonRtcmMessageKind::GalileoInavEphemeris,
        RtcmMessage::Ssr(_) => SidereonRtcmMessageKind::Ssr,
        RtcmMessage::GlonassCodePhaseBiases(_) => SidereonRtcmMessageKind::GlonassCodePhaseBiases,
        RtcmMessage::SsrVtec(_) => SidereonRtcmMessageKind::SsrVtec,
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
        has_rough_range_ms: satellite.rough_range_ms.is_some(),
        rough_range_ms: satellite.rough_range_ms.unwrap_or(0),
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
        has_fine_pseudorange: signal.fine_pseudorange.is_some(),
        fine_pseudorange: signal.fine_pseudorange.unwrap_or(0),
        has_fine_phase_range: signal.fine_phase_range.is_some(),
        fine_phase_range: signal.fine_phase_range.unwrap_or(0),
        has_lock_time_indicator: signal.lock_time_indicator.is_some(),
        lock_time_indicator: signal.lock_time_indicator.unwrap_or(0),
        has_half_cycle_ambiguity: signal.half_cycle_ambiguity.is_some(),
        half_cycle_ambiguity: signal.half_cycle_ambiguity.unwrap_or(false),
        has_cnr: signal.cnr.is_some(),
        cnr: signal.cnr.unwrap_or(0),
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
        value if value == SidereonRtcmMsmKind::Msm1 as u32 => Ok(RtcmMsmKind::Msm1),
        value if value == SidereonRtcmMsmKind::Msm2 as u32 => Ok(RtcmMsmKind::Msm2),
        value if value == SidereonRtcmMsmKind::Msm3 as u32 => Ok(RtcmMsmKind::Msm3),
        value if value == SidereonRtcmMsmKind::Msm4 as u32 => Ok(RtcmMsmKind::Msm4),
        value if value == SidereonRtcmMsmKind::Msm5 as u32 => Ok(RtcmMsmKind::Msm5),
        value if value == SidereonRtcmMsmKind::Msm6 as u32 => Ok(RtcmMsmKind::Msm6),
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
            core_rtcm::FrameSkipReason::Departure(_) => SidereonRtcmFrameSkipReason::Departure,
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

fn rtcm_galileo_fnav_ephemeris_to_c(
    eph: &RtcmGalileoFnavEphemeris,
) -> SidereonRtcmGalileoFnavEphemeris {
    SidereonRtcmGalileoFnavEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        iod_nav: eph.iod_nav,
        sisa: eph.sisa,
        idot: eph.idot,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
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
        bgd_e5a_e1: eph.bgd_e5a_e1,
        e5a_signal_health: eph.e5a_signal_health,
        e5a_data_validity: eph.e5a_data_validity,
        reserved: eph.reserved,
    }
}

fn rtcm_galileo_inav_ephemeris_to_c(
    eph: &RtcmGalileoInavEphemeris,
) -> SidereonRtcmGalileoInavEphemeris {
    SidereonRtcmGalileoInavEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        iod_nav: eph.iod_nav,
        sisa_index: eph.sisa_index,
        idot: eph.idot,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
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
        bgd_e5a_e1: eph.bgd_e5a_e1,
        bgd_e5b_e1: eph.bgd_e5b_e1,
        e5b_signal_health: eph.e5b_signal_health,
        e5b_data_validity: eph.e5b_data_validity,
        e1b_signal_health: eph.e1b_signal_health,
        e1b_data_validity: eph.e1b_data_validity,
        reserved: eph.reserved,
    }
}

fn rtcm_beidou_ephemeris_to_c(eph: &RtcmBeidouEphemeris) -> SidereonRtcmBeidouEphemeris {
    SidereonRtcmBeidouEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        sv_urai: eph.sv_urai,
        idot: eph.idot,
        aode: eph.aode,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        aodc: eph.aodc,
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
        t_gd1: eph.t_gd1,
        t_gd2: eph.t_gd2,
        sv_health: eph.sv_health,
    }
}

fn rtcm_qzss_ephemeris_to_c(eph: &RtcmQzssEphemeris) -> SidereonRtcmQzssEphemeris {
    SidereonRtcmQzssEphemeris {
        satellite_id: eph.satellite_id,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        iode: eph.iode,
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
        idot: eph.idot,
        codes_on_l2: eph.codes_on_l2,
        week_number: eph.week_number,
        ura: eph.ura,
        sv_health: eph.sv_health,
        t_gd: eph.t_gd,
        iodc: eph.iodc,
        fit_interval: eph.fit_interval,
    }
}

fn rtcm_navic_ephemeris_to_c(eph: &RtcmNavicEphemeris) -> SidereonRtcmNavicEphemeris {
    SidereonRtcmNavicEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        a_f0: eph.a_f0,
        a_f1: eph.a_f1,
        a_f2: eph.a_f2,
        ura: eph.ura,
        t_oc: eph.t_oc,
        t_gd: eph.t_gd,
        delta_n: eph.delta_n,
        iodec: eph.iodec,
        reserved: eph.reserved,
        l5_flag: eph.l5_flag,
        s_flag: eph.s_flag,
        c_uc: eph.c_uc,
        c_us: eph.c_us,
        c_ic: eph.c_ic,
        c_is: eph.c_is,
        c_rc: eph.c_rc,
        c_rs: eph.c_rs,
        idot: eph.idot,
        m0: eph.m0,
        t_oe: eph.t_oe,
        eccentricity: eph.eccentricity,
        sqrt_a: eph.sqrt_a,
        omega0: eph.omega0,
        omega: eph.omega,
        omega_dot: eph.omega_dot,
        i0: eph.i0,
        spare_df544: eph.spare_df544,
        spare_df545: eph.spare_df545,
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
        negative_zero: eph.negative_zero,
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
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
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
        rough_range_ms: satellite
            .has_rough_range_ms
            .then_some(satellite.rough_range_ms),
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
        fine_pseudorange: signal
            .has_fine_pseudorange
            .then_some(signal.fine_pseudorange),
        fine_phase_range: signal
            .has_fine_phase_range
            .then_some(signal.fine_phase_range),
        lock_time_indicator: signal
            .has_lock_time_indicator
            .then_some(signal.lock_time_indicator),
        half_cycle_ambiguity: signal
            .has_half_cycle_ambiguity
            .then_some(signal.half_cycle_ambiguity),
        cnr: signal.has_cnr.then_some(signal.cnr),
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
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

fn rtcm_galileo_fnav_ephemeris_from_c(
    eph: &SidereonRtcmGalileoFnavEphemeris,
) -> RtcmGalileoFnavEphemeris {
    RtcmGalileoFnavEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        iod_nav: eph.iod_nav,
        sisa: eph.sisa,
        idot: eph.idot,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
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
        bgd_e5a_e1: eph.bgd_e5a_e1,
        e5a_signal_health: eph.e5a_signal_health,
        e5a_data_validity: eph.e5a_data_validity,
        reserved: eph.reserved,
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

fn rtcm_galileo_inav_ephemeris_from_c(
    eph: &SidereonRtcmGalileoInavEphemeris,
) -> RtcmGalileoInavEphemeris {
    RtcmGalileoInavEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        iod_nav: eph.iod_nav,
        sisa_index: eph.sisa_index,
        idot: eph.idot,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
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
        bgd_e5a_e1: eph.bgd_e5a_e1,
        bgd_e5b_e1: eph.bgd_e5b_e1,
        e5b_signal_health: eph.e5b_signal_health,
        e5b_data_validity: eph.e5b_data_validity,
        e1b_signal_health: eph.e1b_signal_health,
        e1b_data_validity: eph.e1b_data_validity,
        reserved: eph.reserved,
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

fn rtcm_beidou_ephemeris_from_c(eph: &SidereonRtcmBeidouEphemeris) -> RtcmBeidouEphemeris {
    RtcmBeidouEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        sv_urai: eph.sv_urai,
        idot: eph.idot,
        aode: eph.aode,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        aodc: eph.aodc,
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
        t_gd1: eph.t_gd1,
        t_gd2: eph.t_gd2,
        sv_health: eph.sv_health,
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

fn rtcm_qzss_ephemeris_from_c(eph: &SidereonRtcmQzssEphemeris) -> RtcmQzssEphemeris {
    RtcmQzssEphemeris {
        satellite_id: eph.satellite_id,
        t_oc: eph.t_oc,
        a_f2: eph.a_f2,
        a_f1: eph.a_f1,
        a_f0: eph.a_f0,
        iode: eph.iode,
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
        idot: eph.idot,
        codes_on_l2: eph.codes_on_l2,
        week_number: eph.week_number,
        ura: eph.ura,
        sv_health: eph.sv_health,
        t_gd: eph.t_gd,
        iodc: eph.iodc,
        fit_interval: eph.fit_interval,
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

fn rtcm_navic_ephemeris_from_c(eph: &SidereonRtcmNavicEphemeris) -> RtcmNavicEphemeris {
    RtcmNavicEphemeris {
        satellite_id: eph.satellite_id,
        week_number: eph.week_number,
        a_f0: eph.a_f0,
        a_f1: eph.a_f1,
        a_f2: eph.a_f2,
        ura: eph.ura,
        t_oc: eph.t_oc,
        t_gd: eph.t_gd,
        delta_n: eph.delta_n,
        iodec: eph.iodec,
        reserved: eph.reserved,
        l5_flag: eph.l5_flag,
        s_flag: eph.s_flag,
        c_uc: eph.c_uc,
        c_us: eph.c_us,
        c_ic: eph.c_ic,
        c_is: eph.c_is,
        c_rc: eph.c_rc,
        c_rs: eph.c_rs,
        idot: eph.idot,
        m0: eph.m0,
        t_oe: eph.t_oe,
        eccentricity: eph.eccentricity,
        sqrt_a: eph.sqrt_a,
        omega0: eph.omega0,
        omega: eph.omega,
        omega_dot: eph.omega_dot,
        i0: eph.i0,
        spare_df544: eph.spare_df544,
        spare_df545: eph.spare_df545,
        trailing_bits: Vec::new(),
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
        negative_zero: eph.negative_zero,
        // A message built by hand carries no bits after its last field.
        trailing_bits: Vec::new(),
    }
}

unsafe fn rtcm_build(out_messages: *mut *mut SidereonRtcmMessages, message: RtcmMessage) {
    out_messages.write(ptr::null_mut());
    write_boxed_handle(
        out_messages,
        SidereonRtcmMessages {
            messages: vec![message],
        },
    );
}

#[cfg(test)]
mod rtcm_encode_refusal_tests {
    use super::*;

    fn msm_with_satellite(id: u8) -> *mut SidereonRtcmMessages {
        let info = SidereonRtcmMsmInfo {
            message_number: 1077,
            system: SidereonGnssSystem::Gps as u32,
            kind: SidereonRtcmMsmKind::Msm7 as u32,
            header: SidereonRtcmMsmHeader {
                reference_station_id: 0,
                epoch_time: 0,
                multiple_message: false,
                iods: 0,
                reserved: 0,
                clock_steering: 0,
                external_clock: 0,
                divergence_free_smoothing: false,
                smoothing_interval: 0,
            },
            satellite_count: 1,
            signal_count: 0,
            signal_mask: 0,
        };
        let satellite = SidereonRtcmMsmSatellite {
            id,
            has_rough_range_ms: true,
            rough_range_ms: 70,
            rough_range_mod1: 0,
            has_extended_info: true,
            extended_info: 0,
            has_rough_phase_range_rate: true,
            rough_phase_range_rate_m_s: 0,
        };
        let mut messages = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_rtcm_build_msm(&info, &satellite, 1, ptr::null(), 0, &mut messages) },
            SidereonStatus::Ok
        );
        messages
    }

    fn record_frame_too_long_error() {
        let body = vec![0_u8; 1024];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_encode_frame(
                    body.as_ptr(),
                    body.len(),
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let mut info = SidereonRtcmErrorInfo {
            class: SidereonRtcmErrorClass::None,
            kind: 0,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::Encode);
        assert_eq!(
            info.kind,
            SidereonRtcmEncodeErrorKind::FrameBodyTooLong as u32
        );
        assert!(info.payload_len > 0);
    }

    #[test]
    fn typed_rtcm_error_is_scoped_to_the_producing_operation() {
        let messages = msm_with_satellite(64);
        record_frame_too_long_error();
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_last_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
            },
            SidereonStatus::Ok
        );
        assert!(
            required > 0,
            "payload query must preserve the recorded error"
        );
        let mut payload = vec![0_u8; required];
        assert_eq!(
            unsafe {
                sidereon_rtcm_last_error_payload(
                    payload.as_mut_ptr(),
                    payload.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, payload.len());
        let payload: serde_json::Value = serde_json::from_slice(&payload).expect("JSON payload");
        assert_eq!(payload["error"]["variant"], "FrameBodyTooLong");
        let mut info = SidereonRtcmErrorInfo {
            class: SidereonRtcmErrorClass::None,
            kind: 0,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::Encode);

        let mut output_required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_encode(
                    messages,
                    1,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut output_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let mut info = SidereonRtcmErrorInfo {
            class: SidereonRtcmErrorClass::Encode,
            kind: u32::MAX,
            payload_len: usize::MAX,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::None);
        assert_eq!((info.kind, info.payload_len), (0, 0));
        written = 0;
        required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_rtcm_last_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
            },
            SidereonStatus::Ok
        );
        assert_eq!((written, required), (0, 0));
        let needed = unsafe { sidereon_last_error_message(ptr::null_mut(), 0) };
        let mut message = vec![0_i8; needed + 1];
        unsafe { sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        let message = unsafe { CStr::from_ptr(message.as_ptr()) }.to_string_lossy();
        assert!(message.contains("index 1 out of range"), "{message}");
        unsafe { sidereon_rtcm_messages_free(messages) };

        let messages = msm_with_satellite(64);
        record_frame_too_long_error();
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_encode(
                    messages,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut output_required,
                )
            },
            SidereonStatus::Ok
        );
        info.class = SidereonRtcmErrorClass::Encode;
        info.kind = u32::MAX;
        info.payload_len = usize::MAX;
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::None);
        assert_eq!((info.kind, info.payload_len), (0, 0));
        unsafe { sidereon_rtcm_messages_free(messages) };
    }

    #[test]
    fn retention_operations_clear_stale_typed_errors() {
        let messages = msm_with_satellite(64);
        record_frame_too_long_error();
        let mut output = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_with_trailing_bits(messages, 1, ptr::null(), 0, &mut output)
            },
            SidereonStatus::InvalidArgument
        );
        let mut info = SidereonRtcmErrorInfo {
            class: SidereonRtcmErrorClass::Encode,
            kind: u32::MAX,
            payload_len: usize::MAX,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::None);
        assert_eq!((info.kind, info.payload_len), (0, 0));
        unsafe { sidereon_rtcm_messages_free(messages) };

        let messages = msm_with_satellite(64);
        record_frame_too_long_error();
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_with_trailing_bits(messages, 0, ptr::null(), 0, &mut output)
            },
            SidereonStatus::Ok
        );
        info.class = SidereonRtcmErrorClass::Encode;
        info.kind = u32::MAX;
        info.payload_len = usize::MAX;
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::None);
        assert_eq!((info.kind, info.payload_len), (0, 0));
        unsafe {
            sidereon_rtcm_messages_free(messages);
            sidereon_rtcm_messages_free(output);
        }
    }

    #[test]
    fn an_msm_the_masks_cannot_state_is_refused_not_written_as_another_satellite() {
        let refused = msm_with_satellite(65);
        // sidereon-core refuses to encode satellite 65, with a typed encoder
        // refusal, and encodes 64.
        let core_refused = unsafe { (&(*refused).messages)[0].encode() };
        let Err(CoreError::RtcmEncode(core_encode_error)) = core_refused else {
            panic!("sidereon-core refuses satellite 65 with a typed encoder error");
        };
        assert!(unsafe { (&(*refused).messages)[0].to_frame() }.is_err());
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_encode(
                    refused,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));
        let mut info = SidereonRtcmErrorInfo {
            class: SidereonRtcmErrorClass::None,
            kind: 0,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_last_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.class, SidereonRtcmErrorClass::Encode);
        assert_eq!(info.kind, rtcm_encode_error_kind(&core_encode_error) as u32);
        assert_eq!(
            info.payload_len,
            rtcm_error_payload_json(&CoreError::RtcmEncode(core_encode_error)).len()
        );
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_to_frame(
                    refused,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));

        let accepted = msm_with_satellite(64);
        let core_body = unsafe { (&(*accepted).messages)[0].encode() }.expect("core encodes 64");
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_encode(
                    accepted,
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(required, core_body.len());
        unsafe {
            sidereon_rtcm_messages_free(refused);
            sidereon_rtcm_messages_free(accepted);
        }
    }

    #[test]
    fn legacy_system_text_and_policy_encode_apis_preserve_core_fields() {
        let legacy_satellite = SidereonRtcmLegacySatellite {
            satellite_id: 7,
            has_frequency_channel: false,
            frequency_channel: 0,
            l1: SidereonRtcmLegacyL1 {
                code_indicator: true,
                pseudorange: 123,
                phase_range_minus_pseudorange: -9,
                lock_time_indicator: 4,
                has_pseudorange_modulus_ambiguity: true,
                pseudorange_modulus_ambiguity: 3,
                has_cnr: true,
                cnr: 40,
            },
            has_l2: false,
            l2: SidereonRtcmLegacyL2 {
                code_indicator: 0,
                pseudorange_difference: 0,
                phase_range_minus_l1_pseudorange: 0,
                lock_time_indicator: 0,
                has_cnr: false,
                cnr: 0,
            },
        };
        let mut legacy = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_rtcm_build_legacy(
                    1002,
                    12,
                    345,
                    false,
                    1,
                    false,
                    0,
                    &legacy_satellite,
                    1,
                    ptr::null(),
                    0,
                    &mut legacy,
                )
            },
            SidereonStatus::Ok
        );
        let mut header = SidereonRtcmLegacyHeader {
            message_number: 0,
            reference_station_id: 0,
            epoch_time: 0,
            synchronous_gnss: false,
            satellite_count: 0,
            divergence_free_smoothing: false,
            smoothing_interval: 0,
            satellite_records: 0,
            trailing_bit_count: 0,
        };
        let mut satellite = legacy_satellite;
        assert_eq!(
            unsafe { sidereon_rtcm_message_legacy_header(legacy, 0, &mut header) },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_rtcm_message_legacy_satellite(legacy, 0, 0, &mut satellite) },
            SidereonStatus::Ok
        );
        assert_eq!((header.message_number, header.satellite_records), (1002, 1));
        assert_eq!((satellite.satellite_id, satellite.l1.pseudorange), (7, 123));
        let mut encoded_len = 0;
        let mut required_len = 0;
        let mut departure_len = 0;
        let mut departure_required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_encode_with_policy(
                    legacy,
                    0,
                    SidereonRtcmPolicy::Strict as u32,
                    ptr::null_mut(),
                    0,
                    &mut encoded_len,
                    &mut required_len,
                    ptr::null_mut(),
                    0,
                    &mut departure_len,
                    &mut departure_required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(required_len > 0);
        assert_eq!((departure_len, departure_required), (0, 0));

        let announcement = SidereonRtcmMessageAnnouncement {
            message_number: 1077,
            synchronous: true,
            interval: 10,
        };
        let mut parameters = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_rtcm_build_system_parameters(
                    1,
                    60000,
                    1234,
                    1,
                    18,
                    &announcement,
                    1,
                    ptr::null(),
                    0,
                    &mut parameters,
                )
            },
            SidereonStatus::Ok
        );
        let mut parameter_info = SidereonRtcmSystemParameters {
            reference_station_id: 0,
            mjd: 0,
            seconds_of_day: 0,
            announcement_count: 0,
            leap_seconds: 0,
            announcements: 0,
            trailing_bit_count: 0,
        };
        assert_eq!(
            unsafe { sidereon_rtcm_message_system_parameters(parameters, 0, &mut parameter_info) },
            SidereonStatus::Ok
        );
        assert_eq!(
            (parameter_info.mjd, parameter_info.announcements),
            (60000, 1)
        );

        let code_units = [0xc3, 0xa9];
        let mut text = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_rtcm_build_text(
                    2,
                    60001,
                    42,
                    1,
                    code_units.as_ptr(),
                    code_units.len(),
                    ptr::null(),
                    0,
                    &mut text,
                )
            },
            SidereonStatus::Ok
        );
        let mut text_info = SidereonRtcmTextMessage {
            reference_station_id: 0,
            mjd: 0,
            seconds_of_day: 0,
            character_count: 0,
            code_unit_count: 0,
            trailing_bit_count: 0,
        };
        let mut returned_units = [0; 2];
        let mut units_written = 0;
        let mut units_required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_text(
                    text,
                    0,
                    &mut text_info,
                    returned_units.as_mut_ptr(),
                    returned_units.len(),
                    &mut units_written,
                    &mut units_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            (
                text_info.character_count,
                returned_units,
                units_written,
                units_required
            ),
            (1, code_units, 2, 2)
        );
        unsafe {
            sidereon_rtcm_messages_free(legacy);
            sidereon_rtcm_messages_free(parameters);
            sidereon_rtcm_messages_free(text);
        }
    }

    #[test]
    fn navic_payload_accessors_and_builder_preserve_transmitted_fields() {
        let eph = SidereonRtcmNavicEphemeris {
            satellite_id: 9,
            week_number: 389,
            a_f0: -1_234_567,
            a_f1: -12_345,
            a_f2: -3,
            ura: 2,
            t_oc: 10_821,
            t_gd: -5,
            delta_n: 1_234_567,
            iodec: 161,
            reserved: 0x2A5,
            l5_flag: true,
            s_flag: false,
            c_uc: -16_000,
            c_us: 15_000,
            c_ic: -1,
            c_is: 2,
            c_rc: 16_383,
            c_rs: -16_384,
            idot: -8_000,
            m0: -2_000_000_000,
            t_oe: 10_821,
            eccentricity: 3_000_000,
            sqrt_a: 3_404_000_000,
            omega0: 1_500_000_000,
            omega: -1_000_000_000,
            omega_dot: -2_000_000,
            i0: 400_000_000,
            spare_df544: 3,
            spare_df545: 1,
        };
        let trailing_bits = [true, false, true, true];
        let mut messages = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_rtcm_build_navic_ephemeris_with_trailing_bits(
                    &eph,
                    trailing_bits.as_ptr(),
                    trailing_bits.len(),
                    &mut messages,
                )
            },
            SidereonStatus::Ok
        );
        let mut output = eph;
        assert_eq!(
            unsafe { sidereon_rtcm_message_navic_ephemeris(messages, 0, &mut output) },
            SidereonStatus::Ok
        );
        assert_eq!(
            (
                output.satellite_id,
                output.week_number,
                output.a_f0,
                output.a_f1,
                output.a_f2,
                output.ura,
                output.t_oc,
                output.t_gd,
                output.delta_n,
                output.iodec,
            ),
            (
                eph.satellite_id,
                eph.week_number,
                eph.a_f0,
                eph.a_f1,
                eph.a_f2,
                eph.ura,
                eph.t_oc,
                eph.t_gd,
                eph.delta_n,
                eph.iodec,
            )
        );
        assert_eq!(
            (
                output.reserved,
                output.l5_flag,
                output.s_flag,
                output.c_uc,
                output.c_us,
                output.c_ic,
                output.c_is,
                output.c_rc,
                output.c_rs,
                output.idot,
            ),
            (
                eph.reserved,
                eph.l5_flag,
                eph.s_flag,
                eph.c_uc,
                eph.c_us,
                eph.c_ic,
                eph.c_is,
                eph.c_rc,
                eph.c_rs,
                eph.idot,
            )
        );
        assert_eq!(
            (
                output.m0,
                output.t_oe,
                output.eccentricity,
                output.sqrt_a,
                output.omega0,
                output.omega,
                output.omega_dot,
                output.i0,
                output.spare_df544,
                output.spare_df545,
            ),
            (
                eph.m0,
                eph.t_oe,
                eph.eccentricity,
                eph.sqrt_a,
                eph.omega0,
                eph.omega,
                eph.omega_dot,
                eph.i0,
                eph.spare_df544,
                eph.spare_df545,
            )
        );
        let mut returned_trailing_bits = [false; 4];
        let mut trailing_written = 0;
        let mut trailing_required = 0;
        assert_eq!(
            unsafe {
                sidereon_rtcm_message_navic_trailing_bits(
                    messages,
                    0,
                    returned_trailing_bits.as_mut_ptr(),
                    returned_trailing_bits.len(),
                    &mut trailing_written,
                    &mut trailing_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(returned_trailing_bits, trailing_bits);
        assert_eq!((trailing_written, trailing_required), (4, 4));
        unsafe { sidereon_rtcm_messages_free(messages) };
    }

    #[test]
    fn sbas_encode_errors_keep_all_typed_payload_variants() {
        use sidereon_core::sbas::SbasEncodeError as Error;
        let cases = [
            (
                Error::FieldOutOfRange {
                    message_type: 1,
                    field: "delta",
                    index: Some(usize::MAX),
                    value: i128::MIN,
                    width: 1,
                    signed: true,
                },
                SidereonSbasEncodeErrorKind::FieldOutOfRange,
                serde_json::json!({"variant":"FieldOutOfRange","message_type":"1","field":"delta","index":usize::MAX.to_string(),"value":i128::MIN.to_string(),"width":"1","signed":true}),
            ),
            (
                Error::UnrecognizedPreamble { preamble: 0xA9 },
                SidereonSbasEncodeErrorKind::UnrecognizedPreamble,
                serde_json::json!({"variant":"UnrecognizedPreamble","preamble":"169"}),
            ),
            (
                Error::MessageType {
                    message_type: 64,
                    reason: "outside field width",
                },
                SidereonSbasEncodeErrorKind::MessageType,
                serde_json::json!({"variant":"MessageType","message_type":"64","reason":"outside field width"}),
            ),
            (
                Error::RawPayload {
                    message_type: 0,
                    bytes: usize::MAX,
                    bits_past_payload: true,
                },
                SidereonSbasEncodeErrorKind::RawPayload,
                serde_json::json!({"variant":"RawPayload","message_type":"0","bytes":usize::MAX.to_string(),"bits_past_payload":true}),
            ),
            (
                Error::ReservedLayout {
                    message_type: 0,
                    part: "reserved",
                    expected: vec![1, 2],
                    found: vec![3],
                },
                SidereonSbasEncodeErrorKind::ReservedLayout,
                serde_json::json!({"variant":"ReservedLayout","message_type":"0","part":"reserved","expected":["1","2"],"found":["3"]}),
            ),
            (
                Error::LongTermRecordCount {
                    message_type: 24,
                    half: usize::MAX,
                    velocity_code: true,
                    expected: 1,
                    found: 2,
                },
                SidereonSbasEncodeErrorKind::LongTermRecordCount,
                serde_json::json!({"variant":"LongTermRecordCount","message_type":"24","half":usize::MAX.to_string(),"velocity_code":true,"expected":"1","found":"2"}),
            ),
            (
                Error::LongTermFieldNotCarried {
                    message_type: 24,
                    half: usize::MAX,
                    record: usize::MAX,
                    field: "clock_drift",
                },
                SidereonSbasEncodeErrorKind::LongTermFieldNotCarried,
                serde_json::json!({"variant":"LongTermFieldNotCarried","message_type":"24","half":usize::MAX.to_string(),"record":usize::MAX.to_string(),"field":"clock_drift"}),
            ),
            (
                Error::LongTermMissingTimeOfDay {
                    message_type: 24,
                    half: usize::MAX,
                },
                SidereonSbasEncodeErrorKind::LongTermMissingTimeOfDay,
                serde_json::json!({"variant":"LongTermMissingTimeOfDay","message_type":"24","half":usize::MAX.to_string()}),
            ),
            (
                Error::PadBits { value: u8::MAX },
                SidereonSbasEncodeErrorKind::PadBits,
                serde_json::json!({"variant":"PadBits","value":"255"}),
            ),
        ];

        for (error, expected_kind, expected_payload) in cases {
            let core_error = CoreError::SbasEncode(Box::new(error));
            record_rtcm_typed_error(&core_error);
            let (class, kind, payload) =
                LAST_RTCM_TYPED_ERROR.with(|slot| slot.borrow().as_ref().cloned().unwrap());
            assert_eq!(class, SidereonRtcmErrorClass::SbasEncode);
            assert_eq!(kind, expected_kind as u32);
            let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["error"], expected_payload);
        }

        clear_rtcm_typed_error();
        assert!(LAST_RTCM_TYPED_ERROR.with(|slot| slot.borrow().is_none()));
    }

    #[test]
    fn typed_error_payload_is_json_with_variant_fields() {
        let error = CoreError::RtcmEncode(Box::new(core_rtcm::RtcmEncodeError::ValueOutOfRange {
            message_number: 1041,
            field: "week".to_owned(),
            value: i128::MIN,
            minimum: i128::MIN,
            maximum: i128::MAX,
        }));
        record_rtcm_typed_error(&error);
        let payload = LAST_RTCM_TYPED_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|(_, _, payload)| payload.clone())
                .unwrap()
        });
        let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["error"]["variant"], "ValueOutOfRange");
        assert_eq!(parsed["error"]["message_number"], "1041");
        assert_eq!(parsed["error"]["field"], "week");
        assert_eq!(parsed["error"]["value"], i128::MIN.to_string());
        assert_eq!(parsed["error"]["minimum"], i128::MIN.to_string());
        assert_eq!(parsed["error"]["maximum"], i128::MAX.to_string());

        let maximum_error =
            CoreError::RtcmEncode(Box::new(core_rtcm::RtcmEncodeError::FieldOutOfRange {
                message_number: 1041,
                field: "coefficient".to_owned(),
                value: i128::MAX,
                width: 128,
                encoding: core_rtcm::RtcmFieldEncoding::TwosComplement,
            }));
        let maximum_payload = rtcm_error_payload_json(&maximum_error);
        let parsed_maximum: serde_json::Value = serde_json::from_str(&maximum_payload).unwrap();
        assert_eq!(parsed_maximum["error"]["value"], i128::MAX.to_string());
        assert_eq!(parsed_maximum["error"]["encoding"], "TwosComplement");

        let record_error =
            CoreError::RtcmEncode(Box::new(core_rtcm::RtcmEncodeError::MessageNumber {
                message_number: 4076,
                record: core_rtcm::RtcmRecordKind::Msm {
                    system: sidereon_core::GnssSystem::Navic,
                    kind: core_rtcm::MsmKind::Msm7,
                },
            }));
        let record_payload: serde_json::Value =
            serde_json::from_str(&rtcm_error_payload_json(&record_error)).unwrap();
        assert_eq!(record_payload["error"]["record"]["variant"], "Msm");
        assert_eq!(record_payload["error"]["record"]["system"], "NavIC");
        assert_eq!(record_payload["error"]["record"]["kind"], "MSM7");

        let nested_error =
            CoreError::RtcmConversion(Box::new(core_rtcm::RtcmConversionError::VtecEvaluation(
                core_rtcm::VtecEvaluationProblem::LayerDegreeOrder {
                    layer_index: usize::MAX,
                    degree: 12,
                    order: 14,
                },
            )));
        let nested_payload: serde_json::Value =
            serde_json::from_str(&rtcm_error_payload_json(&nested_error)).unwrap();
        assert_eq!(
            nested_payload["error"]["problem"]["variant"],
            "LayerDegreeOrder"
        );
        assert_eq!(
            nested_payload["error"]["problem"]["layer_index"],
            usize::MAX.to_string()
        );
        assert_eq!(nested_payload["error"]["problem"]["degree"], "12");
        assert_eq!(nested_payload["error"]["problem"]["order"], "14");
    }
}

#[cfg(test)]
mod departure_output_alias_tests {
    use super::*;

    #[test]
    fn custom_departure_copy_checks_overlap_and_preserves_empty_short_semantics() {
        let departures = [core_rtcm::RtcmDeparture::FrameReservedBits { reserved: 1 }];

        let mut aliased_count = 0x1234usize;
        let count_ptr = &mut aliased_count as *mut usize;
        assert_eq!(
            unsafe {
                copy_departures_to_c(
                    "test",
                    &departures,
                    ptr::null_mut(),
                    departures.len(),
                    count_ptr,
                    count_ptr,
                )
            },
            Err(SidereonStatus::InvalidArgument)
        );
        assert_eq!(aliased_count, 0x1234);

        let mut overlapping_storage = [0usize; 8];
        overlapping_storage[0] = 0x5A5A;
        let written_ptr = overlapping_storage.as_mut_ptr();
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                copy_departures_to_c(
                    "test",
                    &departures,
                    written_ptr.cast::<SidereonRtcmDeparture>(),
                    departures.len(),
                    written_ptr,
                    &mut required,
                )
            },
            Err(SidereonStatus::InvalidArgument)
        );
        assert_eq!(overlapping_storage[0], 0x5A5A);
        assert_eq!(required, 0);

        let mut written = 99usize;
        let mut required = 99usize;
        assert_eq!(
            unsafe {
                copy_departures_to_c("test", &[], ptr::null_mut(), 0, &mut written, &mut required)
            },
            Ok(())
        );
        assert_eq!((written, required), (0, 0));

        assert_eq!(
            unsafe {
                copy_departures_to_c(
                    "test",
                    &departures,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            Err(SidereonStatus::InvalidArgument)
        );
        assert_eq!((written, required), (0, 1));

        let mut output = SidereonRtcmDeparture::default();
        assert_eq!(
            unsafe {
                copy_departures_to_c(
                    "test",
                    &departures,
                    &mut output,
                    1,
                    &mut written,
                    &mut required,
                )
            },
            Ok(())
        );
        assert_eq!((written, required), (1, 1));
        assert_eq!(output.kind, 0);
        assert_eq!(output.reserved, 1);
    }
}
