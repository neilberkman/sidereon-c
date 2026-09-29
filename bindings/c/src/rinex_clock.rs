use super::*;
use sidereon_core::rinex::clock::{
    civil_to_clock_instant, ClockEpoch, ClockHeaderField, ClockHeaderReading, ClockHeaderRecord,
    ClockLayout, ClockPoint, ClockRecord, ClockRecordReading, ClockRecordType, ClockTimeSystem,
    ClockTimeSystemStatus, ClockWriteDeparture, ClockWriteLeniency, ClockWritePolicy, RinexClock,
    RinexClockError, RinexClockNotice,
};

// --- RINEX clock (sidereon_core::rinex::clock) -------------------------------
//
// A product read from text keeps every line it read as its authority: every
// header line with its exact label and payload, and every body line in order,
// including blank lines, `AR`, `AS`, `CR`, `DR` and `MS` records, continuation
// lines and, after a lossy read, lines that do not read as a record. The typed
// views below (header records, data records, the per-satellite series, skipped
// records, diagnostics and notices) are derived from those lines. Writing an
// unedited product restates its input byte for byte.

/// Most values a RINEX clock record declares: the bias and up to five further
/// values in the Table A16 order.
pub const SIDEREON_CLOCK_MAX_VALUES: usize = 6;

/// Most values a RINEX clock sample carries after its bias: bias sigma, rate,
/// rate sigma, acceleration and acceleration sigma.
pub const SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES: usize = 5;

/// Most values a RINEX clock record can carry beyond its declared count. A
/// surplus value sits at a distinct position 1..=5 of the value sequence.
pub const SIDEREON_CLOCK_MAX_SURPLUS_VALUES: usize = 5;

/// A RINEX clock product. Opaque to C. Create with sidereon_rinex_clock_parse,
/// sidereon_rinex_clock_parse_lossy, sidereon_rinex_clock_parse_result or
/// sidereon_rinex_clock_from_points; release with sidereon_rinex_clock_free.
///
/// The edit routes (sidereon_rinex_clock_set_time_system,
/// sidereon_rinex_clock_set_record_values, sidereon_rinex_clock_insert_record,
/// sidereon_rinex_clock_remove_record, sidereon_rinex_clock_remove_records and
/// sidereon_rinex_clock_set_records_values) change the handle in place; no
/// other call may use the handle while one runs.
pub struct SidereonRinexClock {
    pub(crate) inner: RinexClock,
}

/// Representation tag for a scale-tagged RINEX clock instant.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexClockInstantRepresentation {
    /// The instant is represented by `jd_whole` plus `jd_fraction`.
    JulianDate = 0,
    /// The instant is represented by the signed 128-bit nanosecond pair.
    Nanos = 1,
}

/// Full precision, scale-tagged clock epoch. For `JulianDate`, the nanosecond
/// pair is zero; for `Nanos`, the Julian fields are zero. The signed 128-bit
/// value is transported losslessly as two's-complement high/low words. Where a
/// record has no instant (its product's time system resolves to no time scale),
/// the owning struct's presence flag is false and both Julian fields are NaN.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockEpoch {
    /// TimeScale code.
    pub scale: u32,
    /// SidereonRinexClockInstantRepresentation code.
    pub representation: u32,
    /// Whole Julian date for the JulianDate representation.
    pub jd_whole: f64,
    /// Residual Julian-day fraction for the JulianDate representation.
    pub jd_fraction: f64,
    /// Signed high 64 bits of the Nanos representation.
    pub nanos_high: i64,
    /// Low 64 bits of the Nanos representation.
    pub nanos_low: u64,
}

/// Civil epoch fields of a RINEX clock record, in the product's time system.
/// A UTC product accepts a second of 60.x on a day that ends with a positive
/// leap second.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockCivilEpoch {
    /// Calendar year.
    pub year: i32,
    /// Calendar month, 1..=12.
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Hour of day.
    pub hour: u8,
    /// Minute of hour.
    pub minute: u8,
    /// Seconds of minute, including the fraction. A record's epoch keeps every
    /// digit its seconds field states; this is the nearest double to it, and
    /// sidereon_rinex_clock_source_line returns the field as written.
    pub second: f64,
}

/// One complete RINEX clock series sample.
///
/// additional_values holds the declared values after the bias, in the Table A16
/// order (bias sigma in seconds, clock rate, clock rate sigma, clock
/// acceleration in 1/s, clock acceleration sigma in 1/s); entries at and past
/// additional_value_count are NaN. Values a record carries beyond its declared
/// count are not part of the sample; the record reports them as surplus values.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockPoint {
    /// Scale-tagged sample epoch.
    pub epoch: SidereonClockEpoch,
    /// Satellite clock bias, seconds.
    pub bias_s: f64,
    /// Number of declared values after the bias, 0..=5.
    pub additional_value_count: usize,
    /// The declared values after the bias.
    pub additional_values: [f64; SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES],
}

/// One sample of a product built with sidereon_rinex_clock_from_points.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockSatellitePoint {
    /// Satellite name, null-terminated, exactly as the product holds it.
    pub satellite: SidereonSatelliteToken,
    /// The sample, every declared value included.
    pub point: SidereonClockPoint,
}

/// An owned per-satellite RINEX clock series. Samples remain valid until this
/// handle is released.
pub struct SidereonClockSeries {
    pub(crate) satellite: SidereonSatelliteToken,
    pub(crate) samples: Vec<ClockPoint>,
}

/// Column layout of a RINEX clock file.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockLayout {
    /// The 80-column layout of versions before 3.04.
    V300 = 0,
    /// The 85-column layout of version 3.04 and later.
    V304 = 1,
}

/// A time system a RINEX clock `TIME SYSTEM ID` record names.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockTimeSystem {
    /// `GPS`: GPS system time.
    Gps = 0,
    /// `GLO`: GLONASS time as RINEX reports it, with the hours of UTC. Its
    /// epochs are read in UTC, so a 23:59:60 label on a leap-second day is an
    /// epoch.
    Glo = 1,
    /// `GAL`: Galileo system time.
    Gal = 2,
    /// `QZS`: QZSS system time.
    Qzs = 3,
    /// `BDS`: BeiDou system time (the spelling `BDT` is also read).
    Bds = 4,
    /// `IRN`: IRNSS system time. No core time scale: epochs keep their civil
    /// fields and have no instant.
    Irn = 5,
    /// `UTC`: Coordinated Universal Time.
    Utc = 6,
    /// `TAI`: International Atomic Time.
    Tai = 7,
    /// A system this binding does not yet name.
    Unknown = 999,
}

/// How a RINEX clock product's time system was established.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockTimeSystemStatus {
    /// A `TIME SYSTEM ID` record declares it.
    Declared = 0,
    /// No `TIME SYSTEM ID` record is present; the RINEX clock 3.00 default
    /// applies (`GLO` for a pure GLONASS file, `GAL` for a pure Galileo file,
    /// otherwise `GPS`).
    Defaulted = 1,
    /// A `TIME SYSTEM ID` label this reader does not know; the label is read
    /// with sidereon_rinex_clock_time_system_label.
    Unrecognized = 2,
    /// Several `TIME SYSTEM ID` records naming different systems; the distinct
    /// labels are read with sidereon_rinex_clock_time_system_label.
    Conflicting = 3,
    /// The product was built from points in a stated time scale.
    Constructed = 4,
    /// A status this binding does not yet name.
    Unknown = 999,
}

/// Fixed-width summary of a RINEX clock product.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexClockInfo {
    /// Whether version carries the declared format version.
    pub has_version: bool,
    /// Declared format version; for a built product, the version it is written
    /// in.
    pub version: f64,
    /// Whether layout carries the column layout.
    pub has_layout: bool,
    /// Column layout records are read and written in, as SidereonClockLayout.
    pub layout: u32,
    /// Whether satellite_system carries the `RINEX VERSION / TYPE` system code.
    pub has_satellite_system: bool,
    /// Satellite system code point (`G`, `R`, `E`, `C`, `I`, `J`, `S` or `M`).
    pub satellite_system: u32,
    /// Whether time_system carries the product's time system.
    pub has_time_system: bool,
    /// The declared, defaulted or built time system, as
    /// SidereonClockTimeSystem.
    pub time_system: u32,
    /// How the time system was established, as SidereonClockTimeSystemStatus.
    pub time_system_status: u32,
    /// Number of labels an Unrecognized or Conflicting status carries.
    pub time_system_label_count: usize,
    /// Whether time_scale carries the scale record epochs are read in. False
    /// when the time system is missing, unrecognized, conflicting or has no
    /// core scale (`IRN`).
    pub has_time_scale: bool,
    /// The time scale, as SidereonTimeScale.
    pub time_scale: u32,
    /// Number of header lines; a built product has none.
    pub header_record_count: usize,
    /// Number of data records of every type.
    pub record_count: usize,
    /// Number of satellites with a clock series.
    pub series_count: usize,
    /// Number of samples across every satellite series.
    pub sample_count: usize,
    /// Number of records outside the satellite series.
    pub skipped_record_count: usize,
    /// Number of lines a lossy read kept without reading them, plus header
    /// time-system errors.
    pub diagnostic_count: usize,
    /// Number of non-fatal findings about how the product was read.
    pub notice_count: usize,
    /// The engine's name for `time_system` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub time_system_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    /// The engine's name for `time_system_status` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub time_system_status_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

/// A RINEX clock data record type (Table A16).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockRecordType {
    /// `AR`: analysis result for a receiver clock.
    Ar = 0,
    /// `AS`: analysis result for a satellite clock.
    As = 1,
    /// `CR`: calibration measurement for a receiver.
    Cr = 2,
    /// `DR`: discontinuity measurement for a receiver.
    Dr = 3,
    /// `MS`: monitor measurement for a broadcast satellite clock.
    Ms = 4,
}

/// How a RINEX clock data record line was read.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockRecordReading {
    /// Read at the columns of the 80-column layout.
    ColumnsV300 = 0,
    /// Read at the columns of the 85-column layout.
    ColumnsV304 = 1,
    /// Read as whitespace-separated values.
    Whitespace = 2,
    /// Built or edited through the typed API; written in the product's layout.
    Edited = 3,
    /// A reading this binding does not yet name.
    Unknown = 999,
}

/// A value present in a record beyond its declared count.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockSurplusValue {
    /// Zero-based position in the record's value sequence: 1 bias sigma,
    /// 2 rate, 3 rate sigma, 4 acceleration, 5 acceleration sigma.
    pub position: usize,
    /// The value.
    pub value: f64,
}

/// One RINEX clock data record with its typed reading. The record's name, as
/// written and trimmed, is read with sidereon_clock_records_name.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockRecord {
    /// Record type, as SidereonClockRecordType.
    pub record_type: u32,
    /// Whether satellite carries the canonical identifier of an `AS` record.
    pub has_satellite: bool,
    /// Canonical satellite identifier of an `AS` record.
    pub satellite: SidereonSatelliteToken,
    /// Civil epoch in the product's time system.
    pub civil_epoch: SidereonClockCivilEpoch,
    /// Whether epoch carries an instant; false when the product's time system
    /// resolves to no time scale.
    pub has_epoch: bool,
    /// The epoch as an instant in the product's time scale.
    pub epoch: SidereonClockEpoch,
    /// Number of declared values, bias first, 1..=6.
    pub value_count: usize,
    /// Declared values, bias first; entries at and past value_count are NaN.
    pub values: [f64; SIDEREON_CLOCK_MAX_VALUES],
    /// Number of values present beyond the declared count.
    pub surplus_count: usize,
    /// Values present beyond the declared count, in file order; entries at and
    /// past surplus_count have position 0 and value NaN.
    pub surplus: [SidereonClockSurplusValue; SIDEREON_CLOCK_MAX_SURPLUS_VALUES],
    /// Whether line carries the record's first line number in the source.
    pub has_line: bool,
    /// One-based line number of the record's first line; absent for a record
    /// built or edited through the typed API.
    pub line: usize,
    /// Number of physical lines the record spans in the source.
    pub line_count: usize,
    /// How the first line was read, as SidereonClockRecordReading.
    pub reading: u32,
    /// Whether continuation_reading carries how a continuation line was read.
    pub has_continuation_reading: bool,
    /// How the continuation line was read, as SidereonClockRecordReading.
    pub continuation_reading: u32,
    /// The engine's name for `reading` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub reading_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    /// The engine's name for `continuation_reading` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub continuation_reading_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

/// Values for one record in a sidereon_rinex_clock_set_records_values batch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockRecordValues {
    /// Index of the record, in record order.
    pub index: usize,
    /// Number of values in values, bias first, 1..=6.
    pub value_count: usize,
    /// The new declared values, bias first; entries past value_count are not
    /// read.
    pub values: [f64; SIDEREON_CLOCK_MAX_VALUES],
}

/// A data record read from the source that is not part of the satellite series.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockSkip {
    /// One-based line number where the record appears.
    pub line: usize,
    /// Record type, as SidereonClockRecordType.
    pub record_type: u32,
}

/// How a RINEX clock header record's fields were read.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockHeaderReading {
    /// Read at the columns of the file's version.
    Columns = 0,
    /// Read at the columns of the other layout.
    OtherVersionColumns = 1,
    /// Read as whitespace-separated values.
    Whitespace = 2,
    /// The label is known but the fields do not read in any supported way.
    Uninterpreted = 3,
    /// The label is not a RINEX clock header label.
    UnknownLabel = 4,
    /// A reading this binding does not yet name.
    Unknown = 999,
}

/// The typed reading of a RINEX clock header record. The text parts of each
/// kind, read with sidereon_clock_header_records_field_text, are listed per
/// kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockHeaderFieldKind {
    /// The fields do not read; the record has no typed reading.
    None = 0,
    /// `RINEX VERSION / TYPE`: version; text parts file type, satellite system,
    /// each as written.
    VersionType = 1,
    /// `PGM / RUN BY / DATE`: text parts program, run by, date.
    ProgramRunByDate = 2,
    /// `COMMENT`: text part the comment.
    Comment = 3,
    /// `SYS / # / OBS TYPES`, a first line (system_code and count present) or a
    /// continuation line (both absent): text parts the descriptors on the line.
    ObservationTypes = 4,
    /// `TIME SYSTEM ID`: text part the label, trimmed.
    TimeSystem = 5,
    /// `LEAP SECONDS`: integer. RINEX clock 3.00 defines it as GPS - UTC; 3.04
    /// as TAI - UTC.
    LeapSeconds = 6,
    /// `LEAP SECONDS GNSS` (3.04): integer, GNSS time - UTC.
    LeapSecondsGnss = 7,
    /// `SYS / DCBS APPLIED`: text parts system, program, source.
    DcbsApplied = 8,
    /// `SYS / PCVS APPLIED`: text parts system, program, source.
    PcvsApplied = 9,
    /// `# / TYPES OF DATA`: count; text parts the data type codes on the line.
    TypesOfData = 10,
    /// `STATION NAME / NUM`: text parts name, identifier.
    StationNameNum = 11,
    /// `STATION CLK REF`: text part the reference.
    StationClockRef = 12,
    /// `ANALYSIS CENTER`: text parts designator, name.
    AnalysisCenter = 13,
    /// `# OF CLK REF`: count; start and stop when the record states them.
    ClockRefCount = 14,
    /// `ANALYSIS CLK REF`: text parts name, identifier; constraint_s when
    /// stated.
    AnalysisClockRef = 15,
    /// `# OF SOLN STA / TRF`: count; text part the frame.
    SolutionStationCount = 16,
    /// `SOLN STA NAME / NUM`: text parts name, identifier; xyz_mm.
    SolutionStation = 17,
    /// `# OF SOLN SATS`: count.
    SolutionSatelliteCount = 18,
    /// `PRN LIST`: text parts the satellites on the line.
    PrnList = 19,
    /// `END OF HEADER`.
    EndOfHeader = 20,
    /// A reading this binding does not yet name. The line is read with
    /// sidereon_clock_header_records_text.
    Unknown = 999,
}

/// One RINEX clock header line with its typed reading. The line, label and
/// payload text are read with sidereon_clock_header_records_text, and the
/// typed field's text parts with sidereon_clock_header_records_field_text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockHeaderRecord {
    /// Whether line carries the line number in the source.
    pub has_line: bool,
    /// One-based line number; absent for a line written by an edit.
    pub line: usize,
    /// Zero-based column where the label starts.
    pub label_column: usize,
    /// How the fields were read, as SidereonClockHeaderReading.
    pub reading: u32,
    /// The typed reading, as SidereonClockHeaderFieldKind.
    pub field_kind: u32,
    /// Number of text parts the typed reading carries.
    pub text_part_count: usize,
    /// Whether version carries a VersionType version.
    pub has_version: bool,
    /// Format version.
    pub version: f64,
    /// Whether system_code carries an ObservationTypes system code point.
    pub has_system_code: bool,
    /// Satellite system code point.
    pub system_code: u32,
    /// Whether count carries a declared count.
    pub has_count: bool,
    /// Declared count (ObservationTypes, TypesOfData, ClockRefCount,
    /// SolutionStationCount, SolutionSatelliteCount).
    pub count: usize,
    /// Whether integer carries a LeapSeconds or LeapSecondsGnss value.
    pub has_integer: bool,
    /// Leap seconds.
    pub integer: i64,
    /// Whether start carries a ClockRefCount start epoch.
    pub has_start: bool,
    /// Start epoch.
    pub start: SidereonClockCivilEpoch,
    /// Whether stop carries a ClockRefCount stop epoch.
    pub has_stop: bool,
    /// Stop epoch.
    pub stop: SidereonClockCivilEpoch,
    /// Whether constraint_s carries an AnalysisClockRef a priori constraint.
    pub has_constraint_s: bool,
    /// A priori clock constraint, seconds.
    pub constraint_s: f64,
    /// Whether xyz_mm carries SolutionStation coordinates.
    pub has_xyz_mm: bool,
    /// Geocentric X, Y, Z in millimetres.
    pub xyz_mm: [i64; 3],
    /// The engine's name for `reading` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub reading_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    /// The engine's name for `field_kind` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub field_kind_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

/// Which text of a header record sidereon_clock_header_records_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockHeaderText {
    /// The complete line without its terminator, exactly as written.
    Line = 0,
    /// The header label, or for an unknown label the text in the label columns.
    Label = 1,
    /// The text before the label.
    Payload = 2,
}

/// An owned snapshot of a RINEX clock product's header records. Create with
/// sidereon_rinex_clock_header_records and release with
/// sidereon_clock_header_records_free. The snapshot does not change when the
/// product is edited afterwards.
pub struct SidereonClockHeaderRecords {
    pub(crate) records: Vec<ClockHeaderRecord>,
}

/// An owned snapshot of a RINEX clock product's data records, in file order,
/// including duplicate records for one name and epoch. Create with
/// sidereon_rinex_clock_records and release with sidereon_clock_records_free.
/// The snapshot does not change when the product is edited afterwards.
pub struct SidereonClockRecords {
    pub(crate) records: Vec<ClockRecord>,
}

/// Which failure a RINEX clock read, edit, build or write reported. Every kind
/// but None names a `RinexClockError` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexClockErrorKind {
    /// No failure is recorded.
    None = 0,
    /// An `AS` record is too short to carry its bias. Carries line, reason and
    /// record.
    MalformedAsRecord = 1,
    /// A declared continuation line is missing. Carries line and record_type.
    MissingContinuation = 2,
    /// A continuation line is malformed or truncated. Carries line, reason and
    /// record.
    MalformedContinuation = 3,
    /// A record or header field could not be read or is out of range. Carries
    /// line, field and value.
    BadField = 4,
    /// A caller input or query parameter is invalid, or a value, name or epoch
    /// cannot be written exactly. Carries field and reason.
    InvalidInput = 5,
    /// The product names a time scale no RINEX clock time system states, such
    /// as GLONASS system time. Carries time_scale.
    UnsupportedTimeScale = 6,
}

/// Which text part of a RINEX clock failure a text route copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonRinexClockErrorText {
    /// The complete failure text.
    Message = 0,
    /// The field name a BadField or InvalidInput failure names.
    Field = 1,
    /// The reason a MalformedAsRecord, MalformedContinuation or InvalidInput
    /// failure gives.
    Reason = 2,
    /// The record text a MalformedAsRecord or MalformedContinuation failure
    /// carries.
    Record = 3,
    /// The record type a MissingContinuation failure names.
    RecordType = 4,
    /// The field value a BadField failure carries.
    Value = 5,
}

/// Typed detail of a RINEX clock failure. Only the fields the kind names carry
/// meaning. Text parts are read with the text route of the owner (a
/// SidereonRinexClockResult or a diagnostic); each has_ flag says whether the
/// failure carries that part, since a part can itself be empty.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexClockError {
    /// Which failure was reported.
    pub kind: SidereonRinexClockErrorKind,
    /// Whether line carries a line number.
    pub has_line: bool,
    /// One-based input line number.
    pub line: usize,
    /// Whether time_scale carries the refused scale.
    pub has_time_scale: bool,
    /// The refused time scale, as SidereonTimeScale.
    pub time_scale: u32,
    /// Whether the failure names a field.
    pub has_field: bool,
    /// Whether the failure gives a reason.
    pub has_reason: bool,
    /// Whether the failure carries record text.
    pub has_record: bool,
    /// Whether the failure names a record type.
    pub has_record_type: bool,
    /// Whether the failure carries a field value.
    pub has_value: bool,
}

/// A line a lossy read kept without reading it as a record, or a header
/// time-system error, with the typed failure. Its text parts are read with
/// sidereon_rinex_clock_diagnostic_text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockDiagnostic {
    /// One-based line number.
    pub line: usize,
    /// The failure.
    pub error: SidereonRinexClockError,
}

/// A finding about how a RINEX clock product was read that does not stop it
/// being read.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockNoticeKind {
    /// No `TIME SYSTEM ID` record; the default carried in time_system applies.
    TimeSystemDefaulted = 1,
    /// A version 3.04 or later file has no `TIME SYSTEM ID` record, which its
    /// version requires.
    TimeSystemMissing = 2,
    /// The time system, carried in time_system, has no core time scale; record
    /// epochs keep their civil fields and have no instant.
    TimeSystemWithoutScale = 3,
    /// A header record, at line, was read at the other layout's columns or as
    /// whitespace-separated values.
    HeaderRecordNonconforming = 4,
    /// A header record, at line, has a known label whose fields do not read.
    HeaderRecordUninterpreted = 5,
    /// A header line, at line, has no RINEX clock header label.
    HeaderRecordUnknownLabel = 6,
    /// records records carry values beyond their declared count, the first at
    /// first_line.
    SurplusValues = 7,
    /// records records follow the columns of the layout the file does not
    /// declare, the first at first_line.
    OtherLayoutRecords = 8,
    /// records records were read as whitespace-separated values, the first at
    /// first_line.
    WhitespaceRecords = 9,
    /// A finding this binding does not yet name.
    Unknown = 999,
}

/// One notice about how a RINEX clock product was read.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockNotice {
    /// Which finding, as SidereonClockNoticeKind.
    pub kind: u32,
    /// Whether time_system carries a system.
    pub has_time_system: bool,
    /// The system, as SidereonClockTimeSystem.
    pub time_system: u32,
    /// Whether line carries a header line number.
    pub has_line: bool,
    /// One-based header line number.
    pub line: usize,
    /// Whether records and first_line carry a record count.
    pub has_records: bool,
    /// Number of records.
    pub records: usize,
    /// One-based line number of the first.
    pub first_line: usize,
    /// The engine's name for `kind` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub kind_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    /// The engine's name for `time_system` when it reads UNKNOWN: a value a later
    /// engine adds that this binding has no code for yet. Empty otherwise.
    pub time_system_unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

/// Whether the RINEX clock writer may emit one kind of departure from what a
/// product states, or refuses to write it.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockWriteLeniency {
    /// Refuse to write, naming what cannot be stated.
    Strict = 0,
    /// Write, and report the departure.
    Allow = 1,
}

/// The departures the RINEX clock writer may emit. Values are never
/// approximated under any policy: a value no 19-column field states exactly is
/// refused. Initialize with sidereon_clock_write_policy_init, which allows no
/// departure.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockWritePolicy {
    /// Epochs that no microsecond text states exactly, written as the nearest
    /// microsecond text when Allow, as SidereonClockWriteLeniency.
    pub nearest_microsecond_epochs: u32,
}

/// Which departure the RINEX clock writer emitted under a policy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockWriteDepartureKind {
    /// An epoch no microsecond text states exactly, written as the nearest
    /// microsecond text.
    EpochAtNearestMicrosecond = 1,
    /// A departure this binding does not yet name. Its text is in the name
    /// part.
    Unknown = 999,
}

/// One departure the RINEX clock writer emitted. The record's name and the
/// epoch fields as written are read with
/// sidereon_rinex_clock_result_departure_text.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonClockWriteDeparture {
    /// Which departure, as SidereonClockWriteDepartureKind.
    pub kind: u32,
    /// Index of the record, in record order.
    pub record: usize,
    /// Whether epoch carries the epoch the product holds.
    pub has_epoch: bool,
    /// The epoch the product holds.
    pub epoch: SidereonClockEpoch,
    /// The engine's name for the first value in this struct that reads
    /// UNKNOWN: a value a later engine adds that this binding has no code for
    /// yet. Empty when no value reads UNKNOWN.
    pub unknown_variant: [c_char; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
}

/// Which text of a departure sidereon_rinex_clock_result_departure_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockDepartureText {
    /// The satellite or receiver name the record is written with.
    Name = 0,
    /// The epoch fields as written: year, month, day, hour, minute and
    /// seconds, separated by single blanks.
    Written = 1,
}

/// The fixed-width outcome of one RINEX clock operation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRinexClockOutcome {
    /// Whether the operation succeeded.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, otherwise
    /// SIDEREON_STATUS_INVALID_ARGUMENT, which every failure maps to.
    pub status: SidereonStatus,
    /// The typed failure; kind is None when is_ok is true.
    pub error: SidereonRinexClockError,
}

/// An owned record of one RINEX clock operation: the outcome with its owned
/// text parts, and whatever the operation produced (the written text and its
/// departures, or a removed record). It owns every string it reports, so they
/// stay readable after the product is freed and after later calls overwrite
/// the thread-local message. Release with sidereon_rinex_clock_result_free.
pub struct SidereonRinexClockResult {
    pub(crate) outcome: SidereonRinexClockOutcome,
    pub(crate) message: String,
    pub(crate) error: Option<RinexClockError>,
    pub(crate) text: String,
    pub(crate) departures: Vec<ClockWriteDeparture>,
    pub(crate) record: Option<ClockRecord>,
}

// --- conversions -------------------------------------------------------------

pub(crate) fn absent_clock_epoch() -> SidereonClockEpoch {
    SidereonClockEpoch {
        scale: 0,
        representation: SidereonRinexClockInstantRepresentation::JulianDate as u32,
        jd_whole: f64::NAN,
        jd_fraction: f64::NAN,
        nanos_high: 0,
        nanos_low: 0,
    }
}

pub(crate) fn instant_to_clock_epoch(epoch: &Instant) -> SidereonClockEpoch {
    let (representation, jd_whole, jd_fraction, nanos_high, nanos_low) = match epoch.repr {
        InstantRepr::JulianDate(jd) => (
            SidereonRinexClockInstantRepresentation::JulianDate as u32,
            jd.jd_whole,
            jd.fraction,
            0,
            0,
        ),
        InstantRepr::Nanos(nanos) => {
            let bits = nanos as u128;
            (
                SidereonRinexClockInstantRepresentation::Nanos as u32,
                0.0,
                0.0,
                (bits >> 64) as u64 as i64,
                bits as u64,
            )
        }
    };
    SidereonClockEpoch {
        scale: time_scale_to_c_code(epoch.scale),
        representation,
        jd_whole,
        jd_fraction,
        nanos_high,
        nanos_low,
    }
}

/// Read a caller clock epoch as an instant without checking its values; the
/// engine validates the instant where it takes one.
pub(crate) fn clock_epoch_from_c(
    fn_name: &str,
    arg_name: &str,
    epoch: &SidereonClockEpoch,
) -> Result<Instant, SidereonStatus> {
    let scale = time_scale_from_c_code(fn_name, arg_name, epoch.scale)?;
    let repr = match epoch.representation {
        value if value == SidereonRinexClockInstantRepresentation::JulianDate as u32 => {
            InstantRepr::JulianDate(JulianDateSplit {
                jd_whole: epoch.jd_whole,
                fraction: epoch.jd_fraction,
            })
        }
        value if value == SidereonRinexClockInstantRepresentation::Nanos as u32 => {
            let bits = (u128::from(epoch.nanos_high as u64) << 64) | u128::from(epoch.nanos_low);
            InstantRepr::Nanos(bits as i128)
        }
        other => {
            set_last_error(format!(
                "{fn_name}: invalid {arg_name} representation {other}"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(Instant { scale, repr })
}

fn civil_epoch_to_c(epoch: ClockEpoch) -> SidereonClockCivilEpoch {
    SidereonClockCivilEpoch {
        year: epoch.year,
        month: epoch.month,
        day: epoch.day,
        hour: epoch.hour,
        minute: epoch.minute,
        second: epoch.second,
    }
}

fn absent_civil_epoch() -> SidereonClockCivilEpoch {
    SidereonClockCivilEpoch {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: f64::NAN,
    }
}

fn civil_epoch_from_c(epoch: &SidereonClockCivilEpoch) -> ClockEpoch {
    ClockEpoch {
        year: epoch.year,
        month: epoch.month,
        day: epoch.day,
        hour: epoch.hour,
        minute: epoch.minute,
        second: epoch.second,
    }
}

/// Marshal a sample. A sample carries at most five values after its bias; one
/// with more cannot come from the engine, and is refused by name rather than
/// cut short.
fn clock_point_to_c(
    fn_name: &str,
    point: &ClockPoint,
) -> Result<SidereonClockPoint, SidereonStatus> {
    if point.additional_values.len() > SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES {
        set_last_error(format!(
            "{fn_name}: a sample carries {} values after its bias, more than {SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES}",
            point.additional_values.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let mut additional_values = [f64::NAN; SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES];
    additional_values[..point.additional_values.len()].copy_from_slice(&point.additional_values);
    Ok(SidereonClockPoint {
        epoch: instant_to_clock_epoch(&point.epoch),
        bias_s: point.bias_s,
        additional_value_count: point.additional_values.len(),
        additional_values,
    })
}

fn clock_point_from_c(
    fn_name: &str,
    arg_name: &str,
    point: &SidereonClockPoint,
) -> Result<ClockPoint, SidereonStatus> {
    if point.additional_value_count > SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES {
        set_last_error(format!(
            "{fn_name}: {arg_name}.additional_value_count {} exceeds {SIDEREON_CLOCK_MAX_ADDITIONAL_VALUES}",
            point.additional_value_count
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(ClockPoint::new(
        clock_epoch_from_c(fn_name, &format!("{arg_name}.epoch"), &point.epoch)?,
        point.bias_s,
        point.additional_values[..point.additional_value_count].to_vec(),
    ))
}

fn clock_layout_to_c(layout: ClockLayout) -> u32 {
    match layout {
        ClockLayout::V300 => SidereonClockLayout::V300 as u32,
        ClockLayout::V304 => SidereonClockLayout::V304 as u32,
    }
}

fn clock_time_system_to_c(system: ClockTimeSystem) -> u32 {
    let value = match system {
        ClockTimeSystem::Gps => SidereonClockTimeSystem::Gps,
        ClockTimeSystem::Glo => SidereonClockTimeSystem::Glo,
        ClockTimeSystem::Gal => SidereonClockTimeSystem::Gal,
        ClockTimeSystem::Qzs => SidereonClockTimeSystem::Qzs,
        ClockTimeSystem::Bds => SidereonClockTimeSystem::Bds,
        ClockTimeSystem::Irn => SidereonClockTimeSystem::Irn,
        ClockTimeSystem::Utc => SidereonClockTimeSystem::Utc,
        ClockTimeSystem::Tai => SidereonClockTimeSystem::Tai,
        _ => SidereonClockTimeSystem::Unknown,
    };
    value as u32
}

fn clock_time_system_from_c(fn_name: &str, system: u32) -> Result<ClockTimeSystem, SidereonStatus> {
    match system {
        value if value == SidereonClockTimeSystem::Gps as u32 => Ok(ClockTimeSystem::Gps),
        value if value == SidereonClockTimeSystem::Glo as u32 => Ok(ClockTimeSystem::Glo),
        value if value == SidereonClockTimeSystem::Gal as u32 => Ok(ClockTimeSystem::Gal),
        value if value == SidereonClockTimeSystem::Qzs as u32 => Ok(ClockTimeSystem::Qzs),
        value if value == SidereonClockTimeSystem::Bds as u32 => Ok(ClockTimeSystem::Bds),
        value if value == SidereonClockTimeSystem::Irn as u32 => Ok(ClockTimeSystem::Irn),
        value if value == SidereonClockTimeSystem::Utc as u32 => Ok(ClockTimeSystem::Utc),
        value if value == SidereonClockTimeSystem::Tai as u32 => Ok(ClockTimeSystem::Tai),
        other => {
            set_last_error(format!("{fn_name}: invalid time system {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn clock_time_system_status_to_c(status: &ClockTimeSystemStatus) -> (u32, Vec<String>) {
    let (value, labels) = match status {
        ClockTimeSystemStatus::Declared => (SidereonClockTimeSystemStatus::Declared, Vec::new()),
        ClockTimeSystemStatus::Defaulted => (SidereonClockTimeSystemStatus::Defaulted, Vec::new()),
        ClockTimeSystemStatus::Unrecognized { label } => (
            SidereonClockTimeSystemStatus::Unrecognized,
            vec![label.clone()],
        ),
        ClockTimeSystemStatus::Conflicting { labels } => {
            (SidereonClockTimeSystemStatus::Conflicting, labels.clone())
        }
        ClockTimeSystemStatus::Constructed => {
            (SidereonClockTimeSystemStatus::Constructed, Vec::new())
        }
        _ => (SidereonClockTimeSystemStatus::Unknown, Vec::new()),
    };
    (value as u32, labels)
}

fn clock_record_type_to_c(record_type: ClockRecordType) -> u32 {
    let value = match record_type {
        ClockRecordType::Ar => SidereonClockRecordType::Ar,
        ClockRecordType::As => SidereonClockRecordType::As,
        ClockRecordType::Cr => SidereonClockRecordType::Cr,
        ClockRecordType::Dr => SidereonClockRecordType::Dr,
        ClockRecordType::Ms => SidereonClockRecordType::Ms,
    };
    value as u32
}

fn clock_record_type_from_c(
    fn_name: &str,
    record_type: u32,
) -> Result<ClockRecordType, SidereonStatus> {
    match record_type {
        value if value == SidereonClockRecordType::Ar as u32 => Ok(ClockRecordType::Ar),
        value if value == SidereonClockRecordType::As as u32 => Ok(ClockRecordType::As),
        value if value == SidereonClockRecordType::Cr as u32 => Ok(ClockRecordType::Cr),
        value if value == SidereonClockRecordType::Dr as u32 => Ok(ClockRecordType::Dr),
        value if value == SidereonClockRecordType::Ms as u32 => Ok(ClockRecordType::Ms),
        other => {
            set_last_error(format!("{fn_name}: invalid record type {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn clock_record_reading_to_c(reading: ClockRecordReading) -> u32 {
    let value = match reading {
        ClockRecordReading::Columns(ClockLayout::V300) => SidereonClockRecordReading::ColumnsV300,
        ClockRecordReading::Columns(ClockLayout::V304) => SidereonClockRecordReading::ColumnsV304,
        ClockRecordReading::Whitespace => SidereonClockRecordReading::Whitespace,
        ClockRecordReading::Edited => SidereonClockRecordReading::Edited,
        _ => SidereonClockRecordReading::Unknown,
    };
    value as u32
}

/// Marshal a record. A record declares at most six values and carries at most
/// five surplus values, one per position 1..=5; a record past either bound
/// cannot come from the engine, and is refused by name rather than cut short.
fn clock_record_to_c(
    fn_name: &str,
    record: &ClockRecord,
) -> Result<SidereonClockRecord, SidereonStatus> {
    let values = record.values();
    let surplus_values = record.surplus_values();
    if values.len() > SIDEREON_CLOCK_MAX_VALUES
        || surplus_values.len() > SIDEREON_CLOCK_MAX_SURPLUS_VALUES
    {
        set_last_error(format!(
            "{fn_name}: a record carries {} declared and {} surplus values, more than its fields hold",
            values.len(),
            surplus_values.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let mut declared = [f64::NAN; SIDEREON_CLOCK_MAX_VALUES];
    declared[..values.len()].copy_from_slice(values);
    let mut surplus = [SidereonClockSurplusValue {
        position: 0,
        value: f64::NAN,
    }; SIDEREON_CLOCK_MAX_SURPLUS_VALUES];
    for (slot, value) in surplus.iter_mut().zip(surplus_values) {
        *slot = SidereonClockSurplusValue {
            position: value.position,
            value: value.value,
        };
    }
    let satellite = record.satellite();
    if satellite.is_some_and(|text| text.len() > MAX_SATELLITE_TOKEN_BYTES) {
        set_last_error(format!(
            "{fn_name}: satellite {:?} does not fit a satellite token",
            satellite.unwrap_or_default()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    let epoch = record.epoch();
    let continuation = record.continuation_reading();
    let mut out = SidereonClockRecord {
        record_type: clock_record_type_to_c(record.record_type()),
        has_satellite: satellite.is_some(),
        satellite: satellite_token_from_text(satellite.unwrap_or("")),
        civil_epoch: civil_epoch_to_c(record.civil_epoch()),
        has_epoch: epoch.is_some(),
        epoch: epoch
            .as_ref()
            .map_or_else(absent_clock_epoch, instant_to_clock_epoch),
        value_count: values.len(),
        values: declared,
        surplus_count: surplus_values.len(),
        surplus,
        has_line: record.line().is_some(),
        line: record.line().unwrap_or(0),
        line_count: record.line_count(),
        reading: clock_record_reading_to_c(record.reading()),
        has_continuation_reading: continuation.is_some(),
        continuation_reading: continuation.map_or(
            SidereonClockRecordReading::Unknown as u32,
            clock_record_reading_to_c,
        ),
        reading_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        continuation_reading_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    };
    let unknown = SidereonClockRecordReading::Unknown as u32;
    note_unknown_variant(
        &mut out.reading_unknown_variant,
        out.reading,
        unknown,
        &record.reading(),
    );
    if let Some(reading) = continuation {
        note_unknown_variant(
            &mut out.continuation_reading_unknown_variant,
            out.continuation_reading,
            unknown,
            &reading,
        );
    }
    Ok(out)
}

fn clock_header_reading_to_c(reading: ClockHeaderReading) -> u32 {
    let value = match reading {
        ClockHeaderReading::Columns => SidereonClockHeaderReading::Columns,
        ClockHeaderReading::OtherVersionColumns => SidereonClockHeaderReading::OtherVersionColumns,
        ClockHeaderReading::Whitespace => SidereonClockHeaderReading::Whitespace,
        ClockHeaderReading::Uninterpreted => SidereonClockHeaderReading::Uninterpreted,
        ClockHeaderReading::UnknownLabel => SidereonClockHeaderReading::UnknownLabel,
        _ => SidereonClockHeaderReading::Unknown,
    };
    value as u32
}

/// The fixed-width reading of a header record and the text parts of its typed
/// field, in the order SidereonClockHeaderFieldKind lists for each kind.
fn clock_header_record_parts(
    record: &ClockHeaderRecord,
) -> (SidereonClockHeaderRecord, Vec<String>) {
    use SidereonClockHeaderFieldKind as Kind;
    let mut out = SidereonClockHeaderRecord {
        has_line: record.line().is_some(),
        line: record.line().unwrap_or(0),
        label_column: record.label_column(),
        reading: clock_header_reading_to_c(record.reading()),
        field_kind: Kind::None as u32,
        reading_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        field_kind_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        text_part_count: 0,
        has_version: false,
        version: f64::NAN,
        has_system_code: false,
        system_code: 0,
        has_count: false,
        count: 0,
        has_integer: false,
        integer: 0,
        has_start: false,
        start: absent_civil_epoch(),
        has_stop: false,
        stop: absent_civil_epoch(),
        has_constraint_s: false,
        constraint_s: f64::NAN,
        has_xyz_mm: false,
        xyz_mm: [0; 3],
    };
    let mut parts: Vec<String> = Vec::new();
    let kind = match record.field() {
        None => Kind::None,
        Some(ClockHeaderField::VersionType {
            version,
            file_type,
            satellite_system,
        }) => {
            out.has_version = true;
            out.version = *version;
            parts.extend([file_type.clone(), satellite_system.clone()]);
            Kind::VersionType
        }
        Some(ClockHeaderField::ProgramRunByDate {
            program,
            run_by,
            date,
        }) => {
            parts.extend([program.clone(), run_by.clone(), date.clone()]);
            Kind::ProgramRunByDate
        }
        Some(ClockHeaderField::Comment(text)) => {
            parts.push(text.clone());
            Kind::Comment
        }
        Some(ClockHeaderField::ObservationTypes {
            system,
            count,
            descriptors,
        }) => {
            if let Some(system) = system {
                out.has_system_code = true;
                out.system_code = u32::from(*system);
            }
            if let Some(count) = count {
                out.has_count = true;
                out.count = *count;
            }
            parts.extend(descriptors.iter().cloned());
            Kind::ObservationTypes
        }
        Some(ClockHeaderField::TimeSystem { label }) => {
            parts.push(label.clone());
            Kind::TimeSystem
        }
        Some(ClockHeaderField::LeapSeconds(value)) => {
            out.has_integer = true;
            out.integer = *value;
            Kind::LeapSeconds
        }
        Some(ClockHeaderField::LeapSecondsGnss(value)) => {
            out.has_integer = true;
            out.integer = *value;
            Kind::LeapSecondsGnss
        }
        Some(ClockHeaderField::DcbsApplied {
            system,
            program,
            source,
        }) => {
            parts.extend([system.clone(), program.clone(), source.clone()]);
            Kind::DcbsApplied
        }
        Some(ClockHeaderField::PcvsApplied {
            system,
            program,
            source,
        }) => {
            parts.extend([system.clone(), program.clone(), source.clone()]);
            Kind::PcvsApplied
        }
        Some(ClockHeaderField::TypesOfData { count, types }) => {
            out.has_count = true;
            out.count = *count;
            parts.extend(types.iter().cloned());
            Kind::TypesOfData
        }
        Some(ClockHeaderField::StationNameNum { name, identifier }) => {
            parts.extend([name.clone(), identifier.clone()]);
            Kind::StationNameNum
        }
        Some(ClockHeaderField::StationClockRef(text)) => {
            parts.push(text.clone());
            Kind::StationClockRef
        }
        Some(ClockHeaderField::AnalysisCenter { designator, name }) => {
            parts.extend([designator.clone(), name.clone()]);
            Kind::AnalysisCenter
        }
        Some(ClockHeaderField::ClockRefCount { count, start, stop }) => {
            out.has_count = true;
            out.count = *count;
            if let Some(start) = start {
                out.has_start = true;
                out.start = civil_epoch_to_c(*start);
            }
            if let Some(stop) = stop {
                out.has_stop = true;
                out.stop = civil_epoch_to_c(*stop);
            }
            Kind::ClockRefCount
        }
        Some(ClockHeaderField::AnalysisClockRef {
            name,
            identifier,
            constraint_s,
        }) => {
            parts.extend([name.clone(), identifier.clone()]);
            if let Some(constraint_s) = constraint_s {
                out.has_constraint_s = true;
                out.constraint_s = *constraint_s;
            }
            Kind::AnalysisClockRef
        }
        Some(ClockHeaderField::SolutionStationCount { count, frame }) => {
            out.has_count = true;
            out.count = *count;
            parts.push(frame.clone());
            Kind::SolutionStationCount
        }
        Some(ClockHeaderField::SolutionStation {
            name,
            identifier,
            xyz_mm,
        }) => {
            parts.extend([name.clone(), identifier.clone()]);
            out.has_xyz_mm = true;
            out.xyz_mm = *xyz_mm;
            Kind::SolutionStation
        }
        Some(ClockHeaderField::SolutionSatelliteCount(count)) => {
            out.has_count = true;
            out.count = *count;
            Kind::SolutionSatelliteCount
        }
        Some(ClockHeaderField::PrnList(satellites)) => {
            parts.extend(satellites.iter().cloned());
            Kind::PrnList
        }
        Some(ClockHeaderField::EndOfHeader) => Kind::EndOfHeader,
        // `ClockHeaderField` is non-exhaustive: a reading a later engine adds
        // reads as Unknown; the line itself is still available.
        Some(_) => Kind::Unknown,
    };
    out.field_kind = kind as u32;
    out.text_part_count = parts.len();
    note_unknown_variant(
        &mut out.reading_unknown_variant,
        out.reading,
        SidereonClockHeaderReading::Unknown as u32,
        &record.reading(),
    );
    if let Some(field) = record.field() {
        note_unknown_variant(
            &mut out.field_kind_unknown_variant,
            out.field_kind,
            Kind::Unknown as u32,
            field,
        );
    }
    (out, parts)
}

fn no_clock_error() -> SidereonRinexClockError {
    SidereonRinexClockError {
        kind: SidereonRinexClockErrorKind::None,
        has_line: false,
        line: 0,
        has_time_scale: false,
        time_scale: 0,
        has_field: false,
        has_reason: false,
        has_record: false,
        has_record_type: false,
        has_value: false,
    }
}

fn clock_error_to_c(err: &RinexClockError) -> SidereonRinexClockError {
    use SidereonRinexClockErrorKind as Kind;
    let mut out = no_clock_error();
    match err {
        RinexClockError::MalformedAsRecord { line, .. } => {
            out.kind = Kind::MalformedAsRecord;
            out.has_line = true;
            out.line = *line;
            out.has_reason = true;
            out.has_record = true;
        }
        RinexClockError::MissingContinuation { line, .. } => {
            out.kind = Kind::MissingContinuation;
            out.has_line = true;
            out.line = *line;
            out.has_record_type = true;
        }
        RinexClockError::MalformedContinuation { line, .. } => {
            out.kind = Kind::MalformedContinuation;
            out.has_line = true;
            out.line = *line;
            out.has_reason = true;
            out.has_record = true;
        }
        RinexClockError::BadField { line, .. } => {
            out.kind = Kind::BadField;
            out.has_line = true;
            out.line = *line;
            out.has_field = true;
            out.has_value = true;
        }
        RinexClockError::InvalidInput { .. } => {
            out.kind = Kind::InvalidInput;
            out.has_field = true;
            out.has_reason = true;
        }
        RinexClockError::UnsupportedTimeScale { scale } => {
            out.kind = Kind::UnsupportedTimeScale;
            out.has_time_scale = true;
            out.time_scale = time_scale_to_c_code(*scale);
        }
    }
    out
}

/// One text part of a clock failure; empty when the failure carries none.
fn clock_error_text(err: &RinexClockError, part: u32) -> Option<String> {
    use SidereonRinexClockErrorText as Part;
    let text = match (part, err) {
        (p, _) if p == Part::Message as u32 => err.to_string(),
        (p, RinexClockError::BadField { field, .. })
        | (p, RinexClockError::InvalidInput { field, .. })
            if p == Part::Field as u32 =>
        {
            (*field).to_owned()
        }
        (p, RinexClockError::MalformedAsRecord { reason, .. })
        | (p, RinexClockError::MalformedContinuation { reason, .. })
        | (p, RinexClockError::InvalidInput { reason, .. })
            if p == Part::Reason as u32 =>
        {
            (*reason).to_owned()
        }
        (p, RinexClockError::MalformedAsRecord { record, .. })
        | (p, RinexClockError::MalformedContinuation { record, .. })
            if p == Part::Record as u32 =>
        {
            record.clone()
        }
        (p, RinexClockError::MissingContinuation { record_type, .. })
            if p == Part::RecordType as u32 =>
        {
            record_type.clone()
        }
        (p, RinexClockError::BadField { value, .. }) if p == Part::Value as u32 => value.clone(),
        (p, _) if p <= Part::Value as u32 => String::new(),
        _ => return None,
    };
    Some(text)
}

fn clock_notice_to_c(notice: &RinexClockNotice) -> SidereonClockNotice {
    use SidereonClockNoticeKind as Kind;
    let mut out = SidereonClockNotice {
        kind: Kind::Unknown as u32,
        has_time_system: false,
        time_system: SidereonClockTimeSystem::Unknown as u32,
        has_line: false,
        line: 0,
        has_records: false,
        records: 0,
        first_line: 0,
        kind_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        time_system_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
    };
    let kind = match notice {
        RinexClockNotice::TimeSystemDefaulted { system } => {
            out.has_time_system = true;
            out.time_system = clock_time_system_to_c(*system);
            Kind::TimeSystemDefaulted
        }
        RinexClockNotice::TimeSystemMissing => Kind::TimeSystemMissing,
        RinexClockNotice::TimeSystemWithoutScale { system } => {
            out.has_time_system = true;
            out.time_system = clock_time_system_to_c(*system);
            Kind::TimeSystemWithoutScale
        }
        RinexClockNotice::HeaderRecordNonconforming { line } => {
            out.has_line = true;
            out.line = *line;
            Kind::HeaderRecordNonconforming
        }
        RinexClockNotice::HeaderRecordUninterpreted { line } => {
            out.has_line = true;
            out.line = *line;
            Kind::HeaderRecordUninterpreted
        }
        RinexClockNotice::HeaderRecordUnknownLabel { line } => {
            out.has_line = true;
            out.line = *line;
            Kind::HeaderRecordUnknownLabel
        }
        RinexClockNotice::SurplusValues {
            records,
            first_line,
        } => {
            out.has_records = true;
            out.records = *records;
            out.first_line = *first_line;
            Kind::SurplusValues
        }
        RinexClockNotice::OtherLayoutRecords {
            records,
            first_line,
        } => {
            out.has_records = true;
            out.records = *records;
            out.first_line = *first_line;
            Kind::OtherLayoutRecords
        }
        RinexClockNotice::WhitespaceRecords {
            records,
            first_line,
        } => {
            out.has_records = true;
            out.records = *records;
            out.first_line = *first_line;
            Kind::WhitespaceRecords
        }
        _ => Kind::Unknown,
    };
    out.kind = kind as u32;
    note_unknown_variant(
        &mut out.kind_unknown_variant,
        out.kind,
        Kind::Unknown as u32,
        notice,
    );
    match notice {
        RinexClockNotice::TimeSystemDefaulted { system }
        | RinexClockNotice::TimeSystemWithoutScale { system } => note_unknown_variant(
            &mut out.time_system_unknown_variant,
            out.time_system,
            SidereonClockTimeSystem::Unknown as u32,
            system,
        ),
        _ => {}
    }
    out
}

fn clock_write_policy_from_c(
    fn_name: &str,
    policy: &SidereonClockWritePolicy,
) -> Result<ClockWritePolicy, SidereonStatus> {
    let leniency = match policy.nearest_microsecond_epochs {
        value if value == SidereonClockWriteLeniency::Strict as u32 => ClockWriteLeniency::Strict,
        value if value == SidereonClockWriteLeniency::Allow as u32 => ClockWriteLeniency::Allow,
        other => {
            set_last_error(format!(
                "{fn_name}: invalid policy.nearest_microsecond_epochs {other}"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(ClockWritePolicy::strict().with_nearest_microsecond_epochs(leniency))
}

fn clock_departure_to_c(departure: &ClockWriteDeparture) -> SidereonClockWriteDeparture {
    match departure {
        ClockWriteDeparture::EpochAtNearestMicrosecond { record, epoch, .. } => {
            SidereonClockWriteDeparture {
                kind: SidereonClockWriteDepartureKind::EpochAtNearestMicrosecond as u32,
                record: *record,
                has_epoch: epoch.is_some(),
                epoch: epoch
                    .as_ref()
                    .map_or_else(absent_clock_epoch, instant_to_clock_epoch),
                unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
            }
        }
        other => SidereonClockWriteDeparture {
            kind: SidereonClockWriteDepartureKind::Unknown as u32,
            record: 0,
            has_epoch: false,
            epoch: absent_clock_epoch(),
            unknown_variant: unknown_variant_name(other),
        },
    }
}

fn clock_departure_text(departure: &ClockWriteDeparture, part: u32) -> Option<String> {
    match departure {
        ClockWriteDeparture::EpochAtNearestMicrosecond { name, written, .. } => match part {
            p if p == SidereonClockDepartureText::Name as u32 => Some(name.clone()),
            p if p == SidereonClockDepartureText::Written as u32 => Some(written.clone()),
            _ => None,
        },
        other => match part {
            p if p == SidereonClockDepartureText::Name as u32 => Some(other.to_string()),
            p if p == SidereonClockDepartureText::Written as u32 => Some(String::new()),
            _ => None,
        },
    }
}

/// Read a caller satellite token exactly as it holds it, empty included; the
/// product keeps the name as given.
fn clock_satellite_name_from_c(
    fn_name: &str,
    arg_name: &str,
    token: &SidereonSatelliteToken,
) -> Result<String, SidereonStatus> {
    let Some(len) = token.bytes.iter().position(|byte| *byte == 0) else {
        set_last_error(format!("{fn_name}: {arg_name} is not null-terminated"));
        return Err(SidereonStatus::InvalidArgument);
    };
    let raw: Vec<u8> = token.bytes[..len].iter().map(|byte| *byte as u8).collect();
    String::from_utf8(raw).map_err(|_| {
        set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
        SidereonStatus::InvalidToken
    })
}

// --- owned operation results -------------------------------------------------

fn clock_result_ok() -> SidereonRinexClockResult {
    SidereonRinexClockResult {
        outcome: SidereonRinexClockOutcome {
            is_ok: true,
            status: SidereonStatus::Ok,
            error: no_clock_error(),
        },
        message: String::new(),
        error: None,
        text: String::new(),
        departures: Vec::new(),
        record: None,
    }
}

fn clock_result_refused(fn_name: &str, err: RinexClockError) -> SidereonRinexClockResult {
    SidereonRinexClockResult {
        outcome: SidereonRinexClockOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: clock_error_to_c(&err),
        },
        message: format!("{fn_name}: {err}"),
        error: Some(err),
        text: String::new(),
        departures: Vec::new(),
        record: None,
    }
}

/// Clear an optional result slot before any work.
unsafe fn clear_clock_result_slot(out_result: *mut *mut SidereonRinexClockResult) {
    if !out_result.is_null() {
        *out_result = ptr::null_mut();
    }
}

/// Finish an edit or build: set the thread-local message on a refusal, hand
/// the owned result to an optional slot, and return the operation's status.
unsafe fn finish_clock_operation(
    out_result: *mut *mut SidereonRinexClockResult,
    result: SidereonRinexClockResult,
) -> SidereonStatus {
    let status = result.outcome.status;
    if !result.outcome.is_ok {
        set_last_error(result.message.clone());
    }
    if !out_result.is_null() {
        write_boxed_handle(out_result, result);
    }
    status
}

fn invalid_clock_input(field: &'static str, reason: &'static str) -> RinexClockError {
    RinexClockError::InvalidInput { field, reason }
}

// --- read --------------------------------------------------------------------

unsafe fn clock_text_from_c<'a>(
    fn_name: &str,
    text: *const u8,
    len: usize,
) -> Result<&'a str, SidereonStatus> {
    let bytes = require_slice(text, len, fn_name, "text")?;
    str::from_utf8(bytes).map_err(|_| {
        set_last_error(format!("{fn_name}: text is not valid UTF-8"));
        SidereonStatus::InvalidToken
    })
}

/// Parse RINEX clock text, failing on the first line that does not read. On
/// success writes a newly owned handle to *out_clock. Every line is retained,
/// and sidereon_rinex_clock_to_text restates an unedited product byte for
/// byte. A failure returns SIDEREON_STATUS_INVALID_ARGUMENT with the engine's
/// text in the thread-local message; sidereon_rinex_clock_parse_result returns
/// it typed.
///
/// Safety: text points to len readable bytes; out_clock points to a
/// SidereonRinexClock*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_parse(
    text: *const u8,
    len: usize,
    out_clock: *mut *mut SidereonRinexClock,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_parse";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_clock = c_try!(require_out(out_clock, FN_NAME, "out_clock"));
        let text = clock_text_from_c(FN_NAME, text, len).map(str::to_owned);
        *out_clock = ptr::null_mut();
        let text = c_try!(text);
        match RinexClock::parse(&text) {
            Ok(inner) => {
                write_boxed_handle(out_clock, SidereonRinexClock { inner });
                SidereonStatus::Ok
            }
            Err(err) => {
                set_last_error(format!("{FN_NAME}: {err}"));
                SidereonStatus::InvalidArgument
            }
        }
    })
}

/// Parse RINEX clock text strictly and take an owned record of the attempt.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonRinexClockResult; on a refusal of the text the
/// typed failure is inside it and `*out_clock` stays NULL. A structural failure
/// (a null pointer, text that is not UTF-8) leaves both outputs NULL and
/// returns a status that is not OK.
///
/// Safety: text points to len readable bytes; out_clock points to a
/// SidereonRinexClock*; out_result points to a SidereonRinexClockResult*. Both
/// output slots are set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_parse_result(
    text: *const u8,
    len: usize,
    out_clock: *mut *mut SidereonRinexClock,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_parse_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if out_clock.is_null() || out_result.is_null() {
            if !out_clock.is_null() {
                *out_clock = ptr::null_mut();
            }
            clear_clock_result_slot(out_result);
            c_try!(require_out(out_clock, FN_NAME, "out_clock"));
            c_try!(require_out(out_result, FN_NAME, "out_result"));
        }
        let out_clock = c_try!(require_out(out_clock, FN_NAME, "out_clock"));
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        {
            let clock_range = c_try!(checked_output_range(FN_NAME, out_clock, 1, "out_clock"));
            let result_range = c_try!(checked_output_range(FN_NAME, out_result, 1, "out_result"));
            c_try!(reject_overlapping_outputs(
                FN_NAME,
                clock_range,
                result_range,
                "out_clock",
                "out_result"
            ));
        }
        let text = clock_text_from_c(FN_NAME, text, len).map(str::to_owned);
        *out_clock = ptr::null_mut();
        *out_result = ptr::null_mut();
        let text = c_try!(text);
        match RinexClock::parse(&text) {
            Ok(inner) => {
                write_boxed_handle(out_clock, SidereonRinexClock { inner });
                write_boxed_handle(out_result, clock_result_ok());
            }
            Err(err) => write_boxed_handle(out_result, clock_result_refused(FN_NAME, err)),
        }
        SidereonStatus::Ok
    })
}

/// Parse RINEX clock text, keeping every line that does not read verbatim with
/// a typed diagnostic (sidereon_rinex_clock_diagnostics) and reading the rest.
/// Nothing is dropped: sidereon_rinex_clock_to_text restates the input
/// exactly. An unrecognized `TIME SYSTEM ID` leaves the time system unresolved
/// rather than assuming one.
///
/// Safety: text points to len readable UTF-8 bytes; out_clock points to an
/// owned clock handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_parse_lossy(
    text: *const u8,
    len: usize,
    out_clock: *mut *mut SidereonRinexClock,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_parse_lossy";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_clock = c_try!(require_out(out_clock, FN_NAME, "out_clock"));
        let text = clock_text_from_c(FN_NAME, text, len).map(str::to_owned);
        *out_clock = ptr::null_mut();
        let text = c_try!(text);
        let inner = RinexClock::parse_lossy(&text);
        write_boxed_handle(out_clock, SidereonRinexClock { inner });
        SidereonStatus::Ok
    })
}

/// Build a product from per-satellite samples, keeping every declared value of
/// each sample.
///
/// A run of consecutive entries naming one satellite forms one series row, and
/// each row's samples must be strictly increasing in time; a satellite named
/// by more than one run keeps the samples of every run. The satellite names are
/// taken as written. The records are written in the layout the time scale
/// needs (3.00 for GPST, GST, UTC and TAI; 3.04 for QZSST and BDT); a scale no
/// RINEX clock time system names (GLONASS system time among them, since `GLO`
/// names UTC hours) is refused when the product is written, and a value or
/// epoch the writer cannot state exactly is refused by name.
///
/// Returns SIDEREON_STATUS_OK and a newly owned handle in *out_clock, or
/// SIDEREON_STATUS_INVALID_ARGUMENT with *out_clock NULL. When out_result is
/// not NULL it receives an owned SidereonRinexClockResult recording the
/// outcome of a well-formed call.
///
/// Safety: points points to count readable SidereonClockSatellitePoint values
/// or is NULL when count is 0; out_clock points to a SidereonRinexClock*;
/// out_result is NULL or points to a SidereonRinexClockResult*. Both output
/// slots are set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_from_points(
    time_scale: u32,
    points: *const SidereonClockSatellitePoint,
    count: usize,
    out_clock: *mut *mut SidereonRinexClock,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_from_points";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if out_clock.is_null() {
            clear_clock_result_slot(out_result);
            c_try!(require_out(out_clock, FN_NAME, "out_clock"));
        }
        let out_clock = c_try!(require_out(out_clock, FN_NAME, "out_clock"));
        if !out_result.is_null() {
            let clock_range = c_try!(checked_output_range(FN_NAME, out_clock, 1, "out_clock"));
            let result_range = c_try!(checked_output_range(FN_NAME, out_result, 1, "out_result"));
            c_try!(reject_overlapping_outputs(
                FN_NAME,
                clock_range,
                result_range,
                "out_clock",
                "out_result"
            ));
        }
        let entries =
            require_slice(points, count, FN_NAME, "points").map(|entries| entries.to_vec());
        *out_clock = ptr::null_mut();
        clear_clock_result_slot(out_result);
        let scale = c_try!(time_scale_from_c_code(FN_NAME, "time_scale", time_scale));
        let entries = c_try!(entries);
        let mut rows: Vec<(String, Vec<ClockPoint>)> = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let name = c_try!(clock_satellite_name_from_c(
                FN_NAME,
                &format!("points[{index}].satellite"),
                &entry.satellite
            ));
            let point = c_try!(clock_point_from_c(
                FN_NAME,
                &format!("points[{index}].point"),
                &entry.point
            ));
            let continues_row = rows.last().is_some_and(|(last, _)| *last == name);
            if continues_row {
                if let Some((_, samples)) = rows.last_mut() {
                    samples.push(point);
                }
            } else {
                rows.push((name, vec![point]));
            }
        }
        let result = match RinexClock::from_clock_points(scale, rows) {
            Ok(inner) => {
                write_boxed_handle(out_clock, SidereonRinexClock { inner });
                clock_result_ok()
            }
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        finish_clock_operation(out_result, result)
    })
}

/// Release a RINEX clock handle. Passing NULL is a no-op.
///
/// Safety: clock must be a handle from a RINEX clock creation route or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_free(clock: *mut SidereonRinexClock) {
    free_boxed(clock);
}

// --- product views -----------------------------------------------------------

/// Copy the fixed-width summary of a RINEX clock product.
///
/// Safety: clock is a live handle; out_info points to a SidereonRinexClockInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_info(
    clock: *const SidereonRinexClock,
    out_info: *mut SidereonRinexClockInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        if !clock.is_null() {
            let output_range = c_try!(super::checked_output_range(FN_NAME, out, 1, "out_info"));
            let clock_range = c_try!(super::checked_output_range(
                FN_NAME,
                clock as *mut SidereonRinexClock,
                1,
                "clock"
            ));
            c_try!(super::reject_overlapping_outputs(
                FN_NAME,
                output_range,
                clock_range,
                "out_info",
                "clock"
            ));
        }
        *out = SidereonRinexClockInfo {
            has_version: false,
            version: f64::NAN,
            has_layout: false,
            layout: 0,
            has_satellite_system: false,
            satellite_system: 0,
            has_time_system: false,
            time_system: SidereonClockTimeSystem::Unknown as u32,
            time_system_status: SidereonClockTimeSystemStatus::Unknown as u32,
            time_system_label_count: 0,
            has_time_scale: false,
            time_scale: 0,
            header_record_count: 0,
            record_count: 0,
            series_count: 0,
            sample_count: 0,
            skipped_record_count: 0,
            diagnostic_count: 0,
            notice_count: 0,
            time_system_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
            time_system_status_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        };
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let (status, labels) = clock_time_system_status_to_c(clock.time_system_status());
        let series = clock.series();
        *out = SidereonRinexClockInfo {
            has_version: clock.version().is_some(),
            version: clock.version().unwrap_or(f64::NAN),
            has_layout: clock.layout().is_some(),
            layout: clock.layout().map_or(0, clock_layout_to_c),
            has_satellite_system: clock.satellite_system().is_some(),
            satellite_system: clock.satellite_system().map_or(0, u32::from),
            has_time_system: clock.time_system().is_some(),
            time_system: clock.time_system().map_or(
                SidereonClockTimeSystem::Unknown as u32,
                clock_time_system_to_c,
            ),
            time_system_status: status,
            time_system_label_count: labels.len(),
            has_time_scale: clock.time_scale().is_some(),
            time_scale: clock.time_scale().map_or(0, time_scale_to_c_code),
            header_record_count: clock.header_records().len(),
            record_count: clock.record_count(),
            series_count: series.len(),
            sample_count: series.values().map(Vec::len).sum(),
            skipped_record_count: clock.skipped_records().len(),
            diagnostic_count: clock.diagnostics().len(),
            notice_count: clock.notices().len(),
            time_system_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
            time_system_status_unknown_variant: [0; SIDEREON_UNKNOWN_VARIANT_C_BYTES],
        };
        let out_info = &mut *out;
        if let Some(system) = clock.time_system() {
            note_unknown_variant(
                &mut out_info.time_system_unknown_variant,
                out_info.time_system,
                SidereonClockTimeSystem::Unknown as u32,
                &system,
            );
        }
        note_unknown_variant(
            &mut out_info.time_system_status_unknown_variant,
            out_info.time_system_status,
            SidereonClockTimeSystemStatus::Unknown as u32,
            clock.time_system_status(),
        );
        SidereonStatus::Ok
    })
}

/// Copy one label an Unrecognized or Conflicting time-system status carries,
/// by index in file order. Uses the variable-length output contract; the bytes
/// are not null-terminated.
///
/// Safety: clock is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_time_system_label(
    clock: *const SidereonRinexClock,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_time_system_label";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let (_, labels) = clock_time_system_status_to_c(clock.time_system_status());
        let Some(label) = labels.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            label.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy one line of the text the product was read from, by one-based line
/// number, without its terminator. *out_present is false, with nothing
/// copied, for a line number the text does not have and for a built product.
/// Uses the variable-length output contract; the bytes are not
/// null-terminated.
///
/// Safety: clock is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written, out_required and out_present point to writable
/// values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_source_line(
    clock: *const SidereonRinexClock,
    line: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
    out_present: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_source_line";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_present,
            "out_present",
            clock,
            "clock"
        ));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_written,
            "out_written",
            clock,
            "clock"
        ));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_required,
            "out_required",
            clock,
            "clock"
        ));
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_present.cast(), size_of::<bool>(), 1, "out_present"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        if !out_present.is_null() {
            *out_present = false;
        }
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let out_present = c_try!(require_out(out_present, FN_NAME, "out_present"));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let Some(text) = clock.source_line(line) else {
            return SidereonStatus::Ok;
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
        *out_present = true;
        SidereonStatus::Ok
    })
}

/// Take an owned snapshot of the product's header records, every header line
/// in order with its typed reading. A built product has none.
///
/// Safety: clock is a live handle; out_records points to a
/// SidereonClockHeaderRecords*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_header_records(
    clock: *const SidereonRinexClock,
    out_records: *mut *mut SidereonClockHeaderRecords,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_header_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_records = c_try!(require_out(out_records, FN_NAME, "out_records"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_records,
            "out_records",
            clock,
            "clock"
        ));
        *out_records = ptr::null_mut();
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        write_boxed_handle(
            out_records,
            SidereonClockHeaderRecords {
                records: clock.header_records(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Write the number of records in a header-record snapshot.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_header_records_count(
    records: *const SidereonClockHeaderRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_header_records_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            records,
            "records"
        ));
        *out = 0;
        let records = c_try!(require_ref(records, FN_NAME, "records"));
        *out = records.records.len();
        SidereonStatus::Ok
    })
}

unsafe fn clock_header_record_at<'a>(
    fn_name: &str,
    records: *const SidereonClockHeaderRecords,
    index: usize,
) -> Result<&'a ClockHeaderRecord, SidereonStatus> {
    let records = require_ref(records, fn_name, "records")?;
    records.records.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: index {index} out of range"));
        SidereonStatus::InvalidArgument
    })
}

/// Copy the fixed-width reading of one header record.
///
/// Safety: records is a live handle; out_record points to a
/// SidereonClockHeaderRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_header_records_get(
    records: *const SidereonClockHeaderRecords,
    index: usize,
    out_record: *mut SidereonClockHeaderRecord,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_header_records_get";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_record, FN_NAME, "out_record"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_record",
            records,
            "records"
        ));
        let record = c_try!(clock_header_record_at(FN_NAME, records, index));
        *out = clock_header_record_parts(record).0;
        SidereonStatus::Ok
    })
}

/// Copy the line, label or payload text of one header record, selected by a
/// SidereonClockHeaderText value. Uses the variable-length output contract; the
/// bytes are copied verbatim and not null-terminated.
///
/// Safety: records is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_header_records_text(
    records: *const SidereonClockHeaderRecords,
    index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_header_records_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            records,
            "records"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let record = c_try!(clock_header_record_at(FN_NAME, records, index));
        let text = match part {
            value if value == SidereonClockHeaderText::Line as u32 => record.text(),
            value if value == SidereonClockHeaderText::Label as u32 => record.label(),
            value if value == SidereonClockHeaderText::Payload as u32 => record.payload(),
            other => {
                set_last_error(format!("{FN_NAME}: invalid part {other}"));
                return SidereonStatus::InvalidArgument;
            }
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

/// Copy one text part of a header record's typed reading, by part index below
/// the record's text_part_count, in the order SidereonClockHeaderFieldKind
/// lists for its kind. Uses the variable-length output contract; the bytes are
/// copied verbatim and not null-terminated.
///
/// Safety: records is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_header_records_field_text(
    records: *const SidereonClockHeaderRecords,
    index: usize,
    part_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_header_records_field_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            records,
            "records"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let record = c_try!(clock_header_record_at(FN_NAME, records, index));
        let (_, parts) = clock_header_record_parts(record);
        let Some(text) = parts.get(part_index) else {
            set_last_error(format!(
                "{FN_NAME}: part_index {part_index} out of range for {} parts",
                parts.len()
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

/// Release a header-record snapshot. Passing NULL is a no-op.
///
/// Safety: records must be NULL or a live handle from
/// sidereon_rinex_clock_header_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_header_records_free(
    records: *mut SidereonClockHeaderRecords,
) {
    free_boxed(records);
}

/// Write the number of data records of every type.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_record_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_record_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            clock,
            "clock"
        ));
        *out = 0;
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        *out = clock.inner.record_count();
        SidereonStatus::Ok
    })
}

/// Take an owned snapshot of every data record in file order, including
/// duplicate records for one name and epoch (RINEX clock section 4 uses two `AR`
/// records at one epoch to state a discontinuity). Lines a lossy read could not
/// read are not records; sidereon_rinex_clock_diagnostics names them.
///
/// Safety: clock is a live handle; out_records points to a
/// SidereonClockRecords*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_records(
    clock: *const SidereonRinexClock,
    out_records: *mut *mut SidereonClockRecords,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_records = c_try!(require_out(out_records, FN_NAME, "out_records"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_records,
            "out_records",
            clock,
            "clock"
        ));
        *out_records = ptr::null_mut();
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        write_boxed_handle(
            out_records,
            SidereonClockRecords {
                records: clock.records().collect(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Write the number of records in a record snapshot.
///
/// Safety: records is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_records_count(
    records: *const SidereonClockRecords,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_records_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            records,
            "records"
        ));
        *out = 0;
        let records = c_try!(require_ref(records, FN_NAME, "records"));
        *out = records.records.len();
        SidereonStatus::Ok
    })
}

/// Copy the fixed-width records of a snapshot using the standard caller-buffer
/// convention.
///
/// Safety: records is a live handle; out points to len writable
/// SidereonClockRecord values or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_records_copy(
    records: *const SidereonClockRecords,
    out: *mut SidereonClockRecord,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_records_copy";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonClockRecord>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            records,
            "records"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let records = c_try!(require_ref(records, FN_NAME, "records"));
        let values: Vec<SidereonClockRecord> = c_try!(records
            .records
            .iter()
            .map(|record| clock_record_to_c(FN_NAME, record))
            .collect());
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

/// Copy one fixed-width record of a snapshot.
///
/// Safety: records is a live handle; out_record points to a
/// SidereonClockRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_records_get(
    records: *const SidereonClockRecords,
    index: usize,
    out_record: *mut SidereonClockRecord,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_records_get";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_record, FN_NAME, "out_record"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_record",
            records,
            "records"
        ));
        let records = c_try!(require_ref(records, FN_NAME, "records"));
        let Some(record) = records.records.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *out = c_try!(clock_record_to_c(FN_NAME, record));
        SidereonStatus::Ok
    })
}

/// Copy the receiver or satellite name of one record, as written and trimmed.
/// Uses the variable-length output contract; the bytes are not
/// null-terminated.
///
/// Safety: records is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_records_name(
    records: *const SidereonClockRecords,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_records_name";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            records,
            "records"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let records = c_try!(require_ref(records, FN_NAME, "records"));
        let Some(record) = records.records.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            record.name().as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Release a record snapshot. Passing NULL is a no-op.
///
/// Safety: records must be NULL or a live handle from
/// sidereon_rinex_clock_records.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_records_free(records: *mut SidereonClockRecords) {
    free_boxed(records);
}

/// Copy the records read from the source that are not in the satellite
/// series, every `AR`, `CR`, `DR` and `MS` record, in line order. The records
/// themselves are in sidereon_rinex_clock_records. Uses the standard
/// caller-buffer convention.
///
/// Safety: clock is a live handle; out points to len writable SidereonClockSkip
/// values or is NULL when len is 0; out_written and out_required point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_skipped_records(
    clock: *const SidereonRinexClock,
    out: *mut SidereonClockSkip,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_skipped_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonClockSkip>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let mut values = Vec::with_capacity(clock.skipped_records().len());
        for skip in clock.skipped_records() {
            let Some(record_type) = ClockRecordType::from_code(&skip.record_type) else {
                set_last_error(format!(
                    "{FN_NAME}: skipped record at line {} has record type {:?}, which this binding does not name",
                    skip.line, skip.record_type
                ));
                return SidereonStatus::InvalidArgument;
            };
            values.push(SidereonClockSkip {
                line: skip.line,
                record_type: clock_record_type_to_c(record_type),
            });
        }
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

/// Copy the typed diagnostics: every line a lossy read kept without reading it
/// as a record, and header time-system errors. Text parts are read with
/// sidereon_rinex_clock_diagnostic_text. Uses the standard caller-buffer
/// convention.
///
/// Safety: clock is a live handle; out points to len writable
/// SidereonClockDiagnostic values or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_diagnostics(
    clock: *const SidereonRinexClock,
    out: *mut SidereonClockDiagnostic,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_diagnostics";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonClockDiagnostic>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let values: Vec<SidereonClockDiagnostic> = clock
            .diagnostics()
            .iter()
            .map(|diagnostic| SidereonClockDiagnostic {
                line: diagnostic.line,
                error: clock_error_to_c(&diagnostic.error),
            })
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

/// Copy one text part of a diagnostic, selected by a SidereonRinexClockErrorText
/// value. A part the failure does not carry copies nothing. Uses the
/// variable-length output contract; the bytes are copied verbatim and not
/// null-terminated.
///
/// Safety: clock is a live handle; out points to len writable bytes or is NULL
/// when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_diagnostic_text(
    clock: *const SidereonRinexClock,
    index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_diagnostic_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let Some(diagnostic) = clock.diagnostics().get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        let Some(text) = clock_error_text(&diagnostic.error, part) else {
            set_last_error(format!("{FN_NAME}: invalid part {part}"));
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

/// Copy the notices: findings about how the product was read that do not stop
/// it being read. Uses the standard caller-buffer convention.
///
/// Safety: clock is a live handle; out points to len writable
/// SidereonClockNotice values or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_notices(
    clock: *const SidereonRinexClock,
    out: *mut SidereonClockNotice,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_notices";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonClockNotice>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = &c_try!(require_ref(clock, FN_NAME, "clock")).inner;
        let values: Vec<SidereonClockNotice> =
            clock.notices().iter().map(clock_notice_to_c).collect();
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

// --- satellite series --------------------------------------------------------

/// Copy the deterministic satellite-token enumeration of the satellite series,
/// every satellite with an `AS` record whose epoch resolves to an instant.
///
/// Safety: clock is a live handle; out points to len writable tokens or is
/// NULL when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_satellites(
    clock: *const SidereonRinexClock,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_satellites";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonSatelliteToken>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        let values: Vec<_> = clock
            .inner
            .series()
            .keys()
            .map(|satellite| satellite_token_from_text(satellite))
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

/// Write the number of satellite series in a RINEX clock product.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            clock,
            "clock"
        ));
        *out = 0;
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        *out = clock.inner.series().len();
        SidereonStatus::Ok
    })
}

/// Write the total number of complete scale-tagged samples in a RINEX clock.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_sample_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_sample_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            clock,
            "clock"
        ));
        *out = 0;
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        *out = clock.inner.series().values().map(Vec::len).sum();
        SidereonStatus::Ok
    })
}

/// Return one complete RINEX clock series by deterministic satellite-order
/// index. The returned handle owns a clone of the core series, every declared
/// value of each sample included.
///
/// Safety: clock is a live handle; out_series points to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series(
    clock: *const SidereonRinexClock,
    index: usize,
    out_series: *mut *mut SidereonClockSeries,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_series = c_try!(require_out(out_series, FN_NAME, "out_series"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_series,
            "out_series",
            clock,
            "clock"
        ));
        *out_series = ptr::null_mut();
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        let Some((satellite, samples)) = clock.inner.series().iter().nth(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        write_boxed_handle(
            out_series,
            SidereonClockSeries {
                satellite: satellite_token_from_text(satellite),
                samples: samples.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Return one complete RINEX clock series for a satellite, or a null output
/// handle with status Ok when the satellite has no series.
///
/// Safety: clock is a live handle; satellite_id is a null-terminated UTF-8 string;
/// out_series points to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_for(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    out_series: *mut *mut SidereonClockSeries,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series_for";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_series = c_try!(require_out(out_series, FN_NAME, "out_series"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_series,
            "out_series",
            clock,
            "clock"
        ));
        if clock.is_null() {
            *out_series = ptr::null_mut();
            c_try!(require_ref(clock, FN_NAME, "clock"));
        }
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        let satellite = parse_c_string_allow_empty(FN_NAME, "satellite_id", satellite_id);
        *out_series = ptr::null_mut();
        let satellite = c_try!(satellite);
        let Some(samples) = clock.inner.series().get(&satellite) else {
            return SidereonStatus::Ok;
        };
        write_boxed_handle(
            out_series,
            SidereonClockSeries {
                satellite: satellite_token_from_text(&satellite),
                samples: samples.clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Release a RINEX clock series handle. Passing NULL is a no-op.
///
/// Safety: series is NULL or a live handle returned by a clock-series route.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_free(series: *mut SidereonClockSeries) {
    free_boxed(series);
}

/// Write the number of samples in one complete RINEX clock series.
///
/// Safety: series is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_sample_count(
    series: *const SidereonClockSeries,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series_sample_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_count, FN_NAME, "out_count"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_count",
            series,
            "series"
        ));
        *out = 0;
        let series = c_try!(require_ref(series, FN_NAME, "series"));
        *out = series.samples.len();
        SidereonStatus::Ok
    })
}

/// Copy the satellite identity carried by a series handle.
///
/// Safety: series is a live handle; out_satellite points to a writable token.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_satellite(
    series: *const SidereonClockSeries,
    out_satellite: *mut SidereonSatelliteToken,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series_satellite";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_satellite, FN_NAME, "out_satellite"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_satellite",
            series,
            "series"
        ));
        *out = satellite_token_from_text("");
        let series = c_try!(require_ref(series, FN_NAME, "series"));
        *out = series.satellite;
        SidereonStatus::Ok
    })
}

/// Copy complete scale-tagged samples, every declared value included, from one
/// clock series using the standard caller-buffer convention.
///
/// Safety: series is a live handle; out points to len writable samples or is
/// NULL when len is zero; count pointers point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_series_samples(
    series: *const SidereonClockSeries,
    out: *mut SidereonClockPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_series_samples";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<SidereonClockPoint>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            series,
            "series"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let series = c_try!(require_ref(series, FN_NAME, "series"));
        let values: Vec<SidereonClockPoint> = c_try!(series
            .samples
            .iter()
            .map(|point| clock_point_to_c(FN_NAME, point))
            .collect());
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

/// Write the number of satellites with a clock series to *out_count.
///
/// Safety: clock is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_satellite_count(
    clock: *const SidereonRinexClock,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_satellite_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        *out_count = clock.inner.series().len();
        SidereonStatus::Ok
    })
}

// --- interpolation -----------------------------------------------------------

/// A bias query's engine answer, or a status from reading the caller's query.
type ClockBiasAnswer = Result<Result<Option<f64>, RinexClockError>, SidereonStatus>;

unsafe fn clock_bias_query(
    fn_name: &str,
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    out_bias_s: *mut f64,
    out_available: *mut bool,
    query: impl FnOnce(&RinexClock, &str) -> ClockBiasAnswer,
) -> SidereonStatus {
    c_try!(reject_output_overlaps_handle(
        fn_name,
        out_bias_s,
        "out_bias_s",
        clock,
        "clock"
    ));
    c_try!(reject_output_overlaps_handle(
        fn_name,
        out_available,
        "out_available",
        clock,
        "clock"
    ));
    if !out_bias_s.is_null() && !out_available.is_null() {
        let bias = c_try!(checked_output_range(fn_name, out_bias_s, 1, "out_bias_s"));
        let available = c_try!(checked_output_range(
            fn_name,
            out_available,
            1,
            "out_available"
        ));
        c_try!(reject_overlapping_outputs(
            fn_name,
            bias,
            available,
            "out_bias_s",
            "out_available"
        ));
    }
    let satellite = if clock.is_null() {
        Err(SidereonStatus::NullPointer)
    } else {
        parse_c_string_allow_empty(fn_name, "satellite_id", satellite_id)
    };
    if !out_bias_s.is_null() {
        *out_bias_s = f64::NAN;
    }
    if !out_available.is_null() {
        *out_available = false;
    }
    let out_bias_s = c_try!(require_out(out_bias_s, fn_name, "out_bias_s"));
    let out_available = c_try!(require_out(out_available, fn_name, "out_available"));
    let clock = c_try!(require_ref(clock, fn_name, "clock"));
    let satellite = c_try!(satellite);
    match c_try!(query(&clock.inner, &satellite)) {
        Ok(Some(bias)) => {
            *out_bias_s = bias;
            *out_available = true;
            SidereonStatus::Ok
        }
        Ok(None) => SidereonStatus::Ok,
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        }
    }
}

/// Interpolate a satellite clock bias (seconds) at a GPS-seconds epoch. GPST and
/// QZSST series answer; *out_available is false, with *out_bias_s NaN, when
/// the satellite has no usable value there. GPS seconds outside the civil years
/// 1 through 9999 are refused with SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: clock is a live handle; satellite_id is a null-terminated token;
/// out_bias_s points to a double; out_available points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_bias_at_gps_seconds(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    gps_seconds: f64,
    out_bias_s: *mut f64,
    out_available: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_bias_at_gps_seconds";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        clock_bias_query(
            FN_NAME,
            clock,
            satellite_id,
            out_bias_s,
            out_available,
            |clock, satellite| Ok(clock.clock_s_at_gps_seconds(satellite, gps_seconds)),
        )
    })
}

/// Interpolate a satellite clock bias (seconds) at a civil epoch in the
/// product's own time scale. The second is read as the shortest decimal of the
/// double given, every digit kept, so a query at a record's stated epoch lands
/// on that record. On a UTC product (including one whose time system is `GLO`)
/// a 23:59:60.x label on a leap-second day is a valid query, and interpolation
/// across a leap second uses elapsed time. A product whose time system resolves
/// to no time scale is refused with SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: clock is a live handle; satellite_id is a null-terminated token;
/// epoch points to a SidereonClockCivilEpoch; out_bias_s points to a double;
/// out_available points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_bias_at_civil(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    epoch: *const SidereonClockCivilEpoch,
    out_bias_s: *mut f64,
    out_available: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_bias_at_civil";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        clock_bias_query(
            FN_NAME,
            clock,
            satellite_id,
            out_bias_s,
            out_available,
            |clock, satellite| {
                let epoch = require_ref(epoch, FN_NAME, "epoch")?;
                Ok(clock.clock_s(satellite, civil_epoch_from_c(epoch)))
            },
        )
    })
}

/// Interpolate a satellite clock bias (seconds) at a scale-tagged instant.
///
/// Safety: clock is a live handle; satellite_id is a null-terminated token;
/// epoch points to a SidereonClockEpoch; out_bias_s points to a double;
/// out_available points to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_bias_at_epoch(
    clock: *const SidereonRinexClock,
    satellite_id: *const c_char,
    epoch: *const SidereonClockEpoch,
    out_bias_s: *mut f64,
    out_available: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_bias_at_epoch";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        clock_bias_query(
            FN_NAME,
            clock,
            satellite_id,
            out_bias_s,
            out_available,
            |clock, satellite| {
                let epoch = require_ref(epoch, FN_NAME, "epoch")?;
                let epoch = clock_epoch_from_c(FN_NAME, "epoch", epoch)?;
                Ok(clock.clock_s_at_instant(satellite, epoch))
            },
        )
    })
}

/// Convert a civil epoch in a time scale into the scale-tagged instant a RINEX
/// clock record at that epoch holds. The second is read as the shortest
/// decimal of the double given, every digit kept. A UTC second of 60.x is
/// accepted on a day that ends with a positive leap second. *out_available is
/// false, with *out_epoch's Julian fields NaN, when the fields name no epoch in
/// that scale. Delegates to sidereon_core::rinex::clock::civil_to_clock_instant.
///
/// Safety: out_epoch points to a SidereonClockEpoch; out_available points to a
/// bool.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_civil_to_clock_epoch(
    time_scale: u32,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
    out_epoch: *mut SidereonClockEpoch,
    out_available: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_civil_to_clock_epoch";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_epoch.is_null() {
            *out_epoch = absent_clock_epoch();
        }
        if !out_available.is_null() {
            *out_available = false;
        }
        let out_epoch = c_try!(require_out(out_epoch, FN_NAME, "out_epoch"));
        let out_available = c_try!(require_out(out_available, FN_NAME, "out_available"));
        let scale = c_try!(time_scale_from_c_code(FN_NAME, "time_scale", time_scale));
        if let Some(instant) = civil_to_clock_instant(scale, year, month, day, hour, minute, second)
        {
            *out_epoch = instant_to_clock_epoch(&instant);
            *out_available = true;
        }
        SidereonStatus::Ok
    })
}

// --- edits -------------------------------------------------------------------

/// Declare the product's time system. Every `TIME SYSTEM ID` record is replaced
/// by one written at the `3X,A3` columns of the product's layout, or one is
/// inserted before the first header record Table A15 orders after it. Every
/// record epoch is checked in the new system first; if one does not convert (a
/// 23:59:60 label in a continuous scale), nothing changes. A product with no
/// header section, or one built from points, is refused.
///
/// Returns SIDEREON_STATUS_OK, or SIDEREON_STATUS_INVALID_ARGUMENT with the
/// product unchanged. When out_result is not NULL it receives an owned
/// SidereonRinexClockResult recording the outcome of a well-formed call.
///
/// Safety: clock is a live handle no other call uses meanwhile; out_result is
/// NULL or points to a SidereonRinexClockResult*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_set_time_system(
    clock: *mut SidereonRinexClock,
    time_system: u32,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_set_time_system";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        clear_clock_result_slot(out_result);
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let system = c_try!(clock_time_system_from_c(FN_NAME, time_system));
        let result = match clock.inner.set_time_system(system) {
            Ok(()) => clock_result_ok(),
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        finish_clock_operation(out_result, result)
    })
}

/// Replace the declared values of the record at index (in record order), bias
/// first. The record keeps its type, name and epoch, including the exact text
/// of its seconds field, and is then written in the product's layout. The edit
/// is refused, and nothing changes, when the record could not then be written
/// (a value no 19-column field states exactly, a name or year the layout
/// cannot hold, an epoch the seconds field cannot state), or when the source
/// record carries values beyond its declared count that the new values do not
/// restate: those values are never dropped.
///
/// Returns SIDEREON_STATUS_OK, or SIDEREON_STATUS_INVALID_ARGUMENT with the
/// product unchanged. When out_result is not NULL it receives an owned result.
///
/// Safety: clock is a live handle no other call uses meanwhile; values points
/// to value_count readable doubles or is NULL when value_count is 0; out_result
/// is NULL or points to a SidereonRinexClockResult*, set to NULL before any
/// work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_set_record_values(
    clock: *mut SidereonRinexClock,
    index: usize,
    values: *const f64,
    value_count: usize,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_set_record_values";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        let values =
            require_slice(values, value_count, FN_NAME, "values").map(|values| values.to_vec());
        clear_clock_result_slot(out_result);
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let values = c_try!(values);
        let result = match clock.inner.set_record_values(index, values) {
            Ok(()) => clock_result_ok(),
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        finish_clock_operation(out_result, result)
    })
}

/// Insert a record before the record at index (in record order), or after the
/// last record when index equals the record count.
///
/// An `AS` name must be a satellite identifier and is stored in its canonical
/// spelling; other names must be a single ASCII token the product's layout can
/// hold. The epoch's second is taken as the shortest decimal of the double
/// given, and an epoch the seconds field cannot state (finer than a
/// microsecond) is refused. The record must be writable in the product's
/// layout and its epoch valid in the product's time system.
///
/// Returns SIDEREON_STATUS_OK, or SIDEREON_STATUS_INVALID_ARGUMENT with the
/// product unchanged. When out_result is not NULL it receives an owned result.
///
/// Safety: clock is a live handle no other call uses meanwhile; name is a
/// null-terminated UTF-8 string; epoch points to a SidereonClockCivilEpoch;
/// values points to value_count readable doubles or is NULL when value_count
/// is 0; out_result is NULL or points to a SidereonRinexClockResult*, set to
/// NULL before any work.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_rinex_clock_insert_record(
    clock: *mut SidereonRinexClock,
    index: usize,
    record_type: u32,
    name: *const c_char,
    epoch: *const SidereonClockCivilEpoch,
    values: *const f64,
    value_count: usize,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_insert_record";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        let inputs = (|| {
            let record_type = clock_record_type_from_c(FN_NAME, record_type)?;
            let name = parse_c_string_allow_empty(FN_NAME, "name", name)?;
            let epoch = civil_epoch_from_c(require_ref(epoch, FN_NAME, "epoch")?);
            let values = require_slice(values, value_count, FN_NAME, "values")?.to_vec();
            Ok::<_, SidereonStatus>((record_type, name, epoch, values))
        })();
        clear_clock_result_slot(out_result);
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let (record_type, name, epoch, values) = c_try!(inputs);
        let inserted = ClockRecord::new(record_type, &name, epoch, values)
            .and_then(|record| clock.inner.insert_record(index, record));
        let result = match inserted {
            Ok(()) => clock_result_ok(),
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        finish_clock_operation(out_result, result)
    })
}

/// Remove the record at index (in record order) with every line it spans. When
/// out_result is not NULL it receives an owned result carrying the removed
/// record (sidereon_rinex_clock_result_record).
///
/// Returns SIDEREON_STATUS_OK, or SIDEREON_STATUS_INVALID_ARGUMENT with the
/// product unchanged.
///
/// Safety: clock is a live handle no other call uses meanwhile; out_result is
/// NULL or points to a SidereonRinexClockResult*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_remove_record(
    clock: *mut SidereonRinexClock,
    index: usize,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_remove_record";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        clear_clock_result_slot(out_result);
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let result = match clock.inner.remove_record(index) {
            Ok(record) => {
                let mut result = clock_result_ok();
                result.record = Some(record);
                result
            }
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        finish_clock_operation(out_result, result)
    })
}

/// Remove the records at the given indices (in record order) with every line
/// they span, in one pass; blank and unread lines stay. An index given twice
/// removes its record once. An index past the last record is refused as
/// InvalidInput before anything changes. *out_removed receives the number of
/// records removed.
///
/// Safety: clock is a live handle no other call uses meanwhile; indices points
/// to count readable size_t values or is NULL when count is 0; out_removed
/// points to a size_t; out_result is NULL or points to a
/// SidereonRinexClockResult*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_remove_records(
    clock: *mut SidereonRinexClock,
    indices: *const usize,
    count: usize,
    out_removed: *mut usize,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_remove_records";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_removed,
            "out_removed",
            clock,
            "clock"
        ));
        let indices = require_slice(indices, count, FN_NAME, "indices").map(|items| items.to_vec());
        if !out_result.is_null() && !out_removed.is_null() {
            let removed_range =
                c_try!(checked_output_range(FN_NAME, out_removed, 1, "out_removed"));
            let result_range = c_try!(checked_output_range(FN_NAME, out_result, 1, "out_result"));
            c_try!(reject_overlapping_outputs(
                FN_NAME,
                removed_range,
                result_range,
                "out_removed",
                "out_result"
            ));
        }
        clear_clock_result_slot(out_result);
        let out_removed = c_try!(require_out(out_removed, FN_NAME, "out_removed"));
        *out_removed = 0;
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let indices = c_try!(indices);
        let record_count = clock.inner.record_count();
        let result = if indices.iter().any(|&index| index >= record_count) {
            clock_result_refused(
                FN_NAME,
                invalid_clock_input("indices", "an index names no record"),
            )
        } else {
            let remove: BTreeSet<usize> = indices.iter().copied().collect();
            let mut position = 0usize;
            *out_removed = clock.inner.retain_records(|_| {
                let keep = !remove.contains(&position);
                position += 1;
                keep
            });
            clock_result_ok()
        };
        finish_clock_operation(out_result, result)
    })
}

/// Replace the declared values of several records in one pass, each as
/// sidereon_rinex_clock_set_record_values describes. The whole batch is checked
/// before anything changes: if one edit is refused, none is applied. An index
/// past the last record, or one index given two different value lists, is
/// refused as InvalidInput; an index given the same values twice is edited
/// once. *out_edited receives the number of records edited.
///
/// Safety: clock is a live handle no other call uses meanwhile; edits points to
/// count readable SidereonClockRecordValues or is NULL when count is 0;
/// out_edited points to a size_t; out_result is NULL or points to a
/// SidereonRinexClockResult*, set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_set_records_values(
    clock: *mut SidereonRinexClock,
    edits: *const SidereonClockRecordValues,
    count: usize,
    out_edited: *mut usize,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_set_records_values";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_edited,
            "out_edited",
            clock,
            "clock"
        ));
        let edits = require_slice(edits, count, FN_NAME, "edits").map(|items| items.to_vec());
        if !out_result.is_null() && !out_edited.is_null() {
            let edited_range = c_try!(checked_output_range(FN_NAME, out_edited, 1, "out_edited"));
            let result_range = c_try!(checked_output_range(FN_NAME, out_result, 1, "out_result"));
            c_try!(reject_overlapping_outputs(
                FN_NAME,
                edited_range,
                result_range,
                "out_edited",
                "out_result"
            ));
        }
        clear_clock_result_slot(out_result);
        let out_edited = c_try!(require_out(out_edited, FN_NAME, "out_edited"));
        *out_edited = 0;
        let clock = c_try!(require_mut(clock, FN_NAME, "clock"));
        let edits = c_try!(edits);
        let record_count = clock.inner.record_count();
        let mut by_index: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        let mut refusal = None;
        for (position, edit) in edits.iter().enumerate() {
            if edit.value_count > SIDEREON_CLOCK_MAX_VALUES {
                set_last_error(format!(
                    "{FN_NAME}: edits[{position}].value_count {} exceeds {SIDEREON_CLOCK_MAX_VALUES}",
                    edit.value_count
                ));
                return SidereonStatus::InvalidArgument;
            }
            let values = edit.values[..edit.value_count].to_vec();
            if edit.index >= record_count {
                refusal = Some(invalid_clock_input("edits", "an index names no record"));
                break;
            }
            match by_index.entry(edit.index) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(values);
                }
                std::collections::btree_map::Entry::Occupied(slot) => {
                    let existing = slot.get();
                    let same = existing.len() == values.len()
                        && existing
                            .iter()
                            .zip(&values)
                            .all(|(a, b)| a.to_bits() == b.to_bits());
                    if !same {
                        refusal = Some(invalid_clock_input(
                            "edits",
                            "one record is given two different value lists",
                        ));
                        break;
                    }
                }
            }
        }
        let result = match refusal {
            Some(err) => clock_result_refused(FN_NAME, err),
            None => {
                let mut position = 0usize;
                let edited = clock.inner.edit_records(|_| {
                    let values = by_index.get(&position).cloned();
                    position += 1;
                    values
                });
                match edited {
                    Ok(edited) => {
                        *out_edited = edited;
                        clock_result_ok()
                    }
                    Err(err) => clock_result_refused(FN_NAME, err),
                }
            }
        };
        finish_clock_operation(out_result, result)
    })
}

// --- write -------------------------------------------------------------------

/// Initialize a RINEX clock write policy that allows no departure.
///
/// Safety: out_policy points to a SidereonClockWritePolicy.
#[no_mangle]
pub unsafe extern "C" fn sidereon_clock_write_policy_init(
    out_policy: *mut SidereonClockWritePolicy,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_clock_write_policy_init";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_policy, FN_NAME, "out_policy"));
        *out = SidereonClockWritePolicy {
            nearest_microsecond_epochs: SidereonClockWriteLeniency::Strict as u32,
        };
        SidereonStatus::Ok
    })
}

/// Serialize a RINEX clock product back to text (not null-terminated), allowing
/// no departure. A product read from text restates every retained line byte for
/// byte. Records held as typed values are written in the product's layout;
/// values are written only when their 19-column field reads back to the same
/// bits, and an epoch only when a microsecond text states it exactly. A refusal
/// returns SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero
/// and writes nothing; sidereon_rinex_clock_to_text_result returns it typed.
/// Variable-length output contract.
///
/// Safety: clock is a live handle; out points to len writable bytes or NULL when
/// len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_to_text(
    clock: *const SidereonRinexClock,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_to_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            clock,
            "clock"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        let text = c_try!(clock.inner.to_rinex_string().map_err(|err| {
            set_last_error(format!("{FN_NAME}: {err}"));
            SidereonStatus::InvalidArgument
        }));
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

/// Write the product under a policy and take an owned record of the attempt:
/// the text and every departure the policy allowed and the writer emitted, or
/// the typed refusal. With nearest_microsecond_epochs allowed, an epoch no
/// microsecond text states exactly is written at the nearest microsecond and
/// reported as a departure naming the record, its name, its epoch and the
/// epoch text written. A NULL policy allows no departure.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and
/// hands back a newly owned SidereonRinexClockResult.
///
/// Safety: clock is a live handle; policy is NULL or points to a
/// SidereonClockWritePolicy; out_result points to a SidereonRinexClockResult*,
/// set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_to_text_result(
    clock: *const SidereonRinexClock,
    policy: *const SidereonClockWritePolicy,
    out_result: *mut *mut SidereonRinexClockResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_to_text_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_result,
            "out_result",
            clock,
            "clock"
        ));
        let policy = match policy.as_ref() {
            Some(policy) => clock_write_policy_from_c(FN_NAME, policy),
            None => Ok(ClockWritePolicy::strict()),
        };
        *out_result = ptr::null_mut();
        let clock = c_try!(require_ref(clock, FN_NAME, "clock"));
        let policy = c_try!(policy);
        let result = match clock.inner.to_rinex_string_with_policy(policy) {
            Ok((text, departures)) => {
                let mut result = clock_result_ok();
                result.text = text;
                result.departures = departures;
                result
            }
            Err(err) => clock_result_refused(FN_NAME, err),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

// --- result accessors --------------------------------------------------------

/// Release an owned RINEX clock result. Passing NULL is a no-op.
///
/// Safety: result may be NULL; otherwise it must be a live
/// SidereonRinexClockResult handle this binding produced, passed here exactly
/// once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_free(result: *mut SidereonRinexClockResult) {
    ffi_boundary("sidereon_rinex_clock_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned RINEX clock result. *out_outcome is
/// written before the result pointer is validated.
///
/// Safety: result is a live handle; out_outcome points to a
/// SidereonRinexClockOutcome.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_get_outcome(
    result: *const SidereonRinexClockResult,
    out_outcome: *mut SidereonRinexClockOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out,
            "out_outcome",
            result,
            "result"
        ));
        *out = SidereonRinexClockOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_clock_error(),
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out = result.outcome;
        SidereonStatus::Ok
    })
}

/// Copy one text part of an owned result's failure, selected by a
/// SidereonRinexClockErrorText value; Message is prefixed with the route that
/// produced it. A successful result, or a part the failure does not carry,
/// copies nothing. Uses the variable-length output contract; the bytes are
/// copied verbatim and not null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_error_text(
    result: *const SidereonRinexClockResult,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            result,
            "result"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if part > SidereonRinexClockErrorText::Value as u32 {
            set_last_error(format!("{FN_NAME}: invalid part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = if part == SidereonRinexClockErrorText::Message as u32 {
            result.message.clone()
        } else {
            result
                .error
                .as_ref()
                .and_then(|err| clock_error_text(err, part))
                .unwrap_or_default()
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

/// Copy the text a write result holds. A refused result has no text: the call
/// returns SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero,
/// writes nothing, and sets the thread-local message to the result's own
/// failure text. A result of any other operation holds no text and reports a
/// required length of zero. Uses the variable-length output contract; the
/// bytes are not null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_get_text(
    result: *const SidereonRinexClockResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_get_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            result,
            "result"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if !result.outcome.is_ok {
            set_last_error(result.message.clone());
            return result.outcome.status;
        }
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the departures a write result reports, in record order. Uses the
/// standard caller-buffer convention.
///
/// Safety: result is a live handle; out points to len writable
/// SidereonClockWriteDeparture values or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_departures(
    result: *const SidereonRinexClockResult,
    out: *mut SidereonClockWriteDeparture,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_departures";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (
                    out.cast(),
                    size_of::<SidereonClockWriteDeparture>(),
                    len,
                    "out"
                ),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            result,
            "result"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let values: Vec<SidereonClockWriteDeparture> =
            result.departures.iter().map(clock_departure_to_c).collect();
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

/// Copy the name or the written epoch text of one departure, selected by a
/// SidereonClockDepartureText value. Uses the variable-length output contract;
/// the bytes are not null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_departure_text(
    result: *const SidereonRinexClockResult,
    index: usize,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_departure_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            result,
            "result"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let Some(departure) = result.departures.get(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        let Some(text) = clock_departure_text(departure, part) else {
            set_last_error(format!("{FN_NAME}: invalid part {part}"));
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

/// Copy the record a remove result carries. *out_present is false, and
/// *out_record is left as cleared, for a result of any other operation.
///
/// Safety: result is a live handle; out_present points to a bool; out_record
/// points to a SidereonClockRecord.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_record(
    result: *const SidereonRinexClockResult,
    out_present: *mut bool,
    out_record: *mut SidereonClockRecord,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_record";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_present.is_null() && !out_record.is_null() {
            let present_range =
                c_try!(checked_output_range(FN_NAME, out_present, 1, "out_present"));
            let record_range = c_try!(checked_output_range(FN_NAME, out_record, 1, "out_record"));
            c_try!(reject_overlapping_outputs(
                FN_NAME,
                present_range,
                record_range,
                "out_present",
                "out_record"
            ));
        }
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_present,
            "out_present",
            result,
            "result"
        ));
        c_try!(reject_output_overlaps_handle(
            FN_NAME,
            out_record,
            "out_record",
            result,
            "result"
        ));
        if !out_present.is_null() {
            *out_present = false;
        }
        let out_present = c_try!(require_out(out_present, FN_NAME, "out_present"));
        let out_record = c_try!(require_out(out_record, FN_NAME, "out_record"));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if let Some(record) = &result.record {
            *out_record = c_try!(clock_record_to_c(FN_NAME, record));
            *out_present = true;
        }
        SidereonStatus::Ok
    })
}

/// Copy the name of the record a remove result carries; a result without a
/// record reports a required length of zero. Uses the variable-length output
/// contract; the bytes are not null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rinex_clock_result_record_name(
    result: *const SidereonRinexClockResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_rinex_clock_result_record_name";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(reject_outputs_overlapping_handle(
            FN_NAME,
            &[
                (out.cast(), size_of::<u8>(), len, "out"),
                (out_written.cast(), size_of::<usize>(), 1, "out_written"),
                (out_required.cast(), size_of::<usize>(), 1, "out_required")
            ],
            result,
            "result"
        ));
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let name = result.record.as_ref().map_or("", ClockRecord::name);
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            name.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod rinex_clock_c_tests {
    use super::*;
    use std::mem::{ManuallyDrop, MaybeUninit};

    #[repr(C)]
    union ClockInfoStorage {
        clock: ManuallyDrop<SidereonRinexClock>,
        info: SidereonRinexClockInfo,
    }

    const CLOCK_FIXTURE: &str = include_str!("../tests/fixtures/clk/synthetic_rinex_clock.clk");

    #[test]
    fn parse_snapshots_text_before_overlapping_output_handle_reset() {
        const STORAGE_WORDS: usize = CLOCK_FIXTURE.len().div_ceil(size_of::<usize>());
        let mut storage = [0usize; STORAGE_WORDS];
        let bytes = storage.as_mut_ptr().cast::<u8>();
        unsafe {
            ptr::copy_nonoverlapping(CLOCK_FIXTURE.as_ptr(), bytes, CLOCK_FIXTURE.len());
        }
        assert_eq!(
            unsafe {
                sidereon_rinex_clock_parse(bytes, CLOCK_FIXTURE.len(), storage.as_mut_ptr().cast())
            },
            SidereonStatus::Ok
        );
        let handle = storage.as_mut_ptr().cast::<*mut SidereonRinexClock>();
        let handle = unsafe { handle.read() };
        assert!(!handle.is_null());
        unsafe { sidereon_rinex_clock_free(handle) };
    }

    #[test]
    fn parse_result_input_refusal_resets_both_output_slots() {
        let mut clock = ptr::dangling_mut::<SidereonRinexClock>();
        let mut result = ptr::dangling_mut::<SidereonRinexClockResult>();
        assert_eq!(
            unsafe { sidereon_rinex_clock_parse_result(ptr::null(), 1, &mut clock, &mut result) },
            SidereonStatus::NullPointer
        );
        assert!(clock.is_null());
        assert!(result.is_null());
    }

    fn lossy(text: &str) -> SidereonRinexClock {
        SidereonRinexClock {
            inner: RinexClock::parse_lossy(text),
        }
    }

    #[test]
    fn info_output_overlap_with_live_handle_is_refused_without_mutating_handle() {
        let mut storage = ClockInfoStorage {
            clock: ManuallyDrop::new(lossy(CLOCK_FIXTURE)),
        };
        let storage_ptr = &mut storage as *mut ClockInfoStorage;
        let clock_ptr =
            unsafe { std::ptr::addr_of_mut!((*storage_ptr).clock).cast::<SidereonRinexClock>() };
        let info_ptr = unsafe { std::ptr::addr_of_mut!((*storage_ptr).info) };
        assert_eq!(
            unsafe { sidereon_rinex_clock_info(clock_ptr, info_ptr) },
            SidereonStatus::InvalidArgument
        );

        let mut info = MaybeUninit::<SidereonRinexClockInfo>::uninit();
        assert_eq!(
            unsafe { sidereon_rinex_clock_info(clock_ptr, info.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let info = unsafe { info.assume_init() };
        assert!(info.record_count > 0);

        unsafe { ManuallyDrop::drop(&mut (*storage_ptr).clock) };
    }

    fn text_of(
        call: impl Fn(*mut u8, usize, *mut usize, *mut usize) -> SidereonStatus,
    ) -> (SidereonStatus, String) {
        let mut written = 0;
        let mut required = 0;
        let status = call(ptr::null_mut(), 0, &mut written, &mut required);
        if status != SidereonStatus::Ok {
            return (status, String::new());
        }
        let mut bytes = vec![0u8; required];
        let status = call(bytes.as_mut_ptr(), bytes.len(), &mut written, &mut required);
        bytes.truncate(written);
        (status, String::from_utf8(bytes).expect("UTF-8"))
    }

    fn assert_record_matches(got: &SidereonClockRecord, want: &SidereonClockRecord) {
        assert_eq!(got.record_type, want.record_type);
        assert_eq!(got.has_satellite, want.has_satellite);
        assert_eq!(got.satellite.bytes, want.satellite.bytes);
        assert_eq!(
            (
                got.civil_epoch.year,
                got.civil_epoch.month,
                got.civil_epoch.day,
                got.civil_epoch.hour,
                got.civil_epoch.minute,
                got.civil_epoch.second.to_bits()
            ),
            (
                want.civil_epoch.year,
                want.civil_epoch.month,
                want.civil_epoch.day,
                want.civil_epoch.hour,
                want.civil_epoch.minute,
                want.civil_epoch.second.to_bits()
            )
        );
        assert_eq!(got.value_count, want.value_count);
        for (got_value, want_value) in got.values.iter().zip(want.values) {
            assert_eq!(got_value.to_bits(), want_value.to_bits());
        }
        assert_eq!(got.surplus_count, want.surplus_count);
        assert_eq!(
            (got.has_line, got.line, got.line_count),
            (want.has_line, want.line, want.line_count)
        );
        assert_eq!(got.reading, want.reading);
        assert_eq!(got.has_continuation_reading, want.has_continuation_reading);
    }

    fn assert_notice_matches(got: &SidereonClockNotice, want: &SidereonClockNotice) {
        assert_eq!(got.kind, want.kind);
        assert_eq!(
            (got.has_time_system, got.time_system),
            (want.has_time_system, want.time_system)
        );
        assert_eq!((got.has_line, got.line), (want.has_line, want.line));
        assert_eq!(
            (got.has_records, got.records, got.first_line),
            (want.has_records, want.records, want.first_line)
        );
    }

    #[test]
    fn every_record_of_the_fixture_is_kept_and_restated() {
        let clock = lossy(CLOCK_FIXTURE);
        // Every expected value is sidereon-core's own lossy reading of the
        // fixture; enum values go through the binding's code mapping.
        let core = RinexClock::parse_lossy(CLOCK_FIXTURE);
        let core_series = core.series();
        unsafe {
            let mut info = std::mem::zeroed::<SidereonRinexClockInfo>();
            assert_eq!(
                sidereon_rinex_clock_info(&clock, &mut info),
                SidereonStatus::Ok
            );
            assert!(info.has_version);
            assert_eq!(
                info.version.to_bits(),
                core.version().expect("version").to_bits()
            );
            assert_eq!(
                info.layout,
                clock_layout_to_c(core.layout().expect("layout"))
            );
            assert_eq!(info.has_satellite_system, core.satellite_system().is_some());
            assert_eq!(
                info.time_system,
                clock_time_system_to_c(core.time_system().expect("time system"))
            );
            assert_eq!(
                info.time_system_status,
                clock_time_system_status_to_c(core.time_system_status()).0
            );
            assert_eq!(info.has_time_scale, core.time_scale().is_some());
            assert_eq!(
                info.time_scale,
                time_scale_to_c_code(core.time_scale().expect("time scale"))
            );
            assert_eq!(info.header_record_count, core.header_records().len());
            assert_eq!(info.record_count, core.record_count());
            assert_eq!(
                (info.series_count, info.sample_count),
                (
                    core_series.len(),
                    core_series.values().map(Vec::len).sum::<usize>()
                )
            );
            assert_eq!(
                (info.skipped_record_count, info.diagnostic_count),
                (core.skipped_records().len(), core.diagnostics().len())
            );

            let (status, text) = text_of(|out, len, written, required| {
                sidereon_rinex_clock_to_text(&clock, out, len, written, required)
            });
            assert_eq!(status, SidereonStatus::Ok);
            // An unedited product restates its text byte for byte.
            assert_eq!(text, CLOCK_FIXTURE);

            let core_skip = &core.skipped_records()[0];
            let mut skip = SidereonClockSkip {
                line: 0,
                record_type: 99,
            };
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_rinex_clock_skipped_records(
                    &clock,
                    &mut skip,
                    1,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                (skip.line, skip.record_type),
                (
                    core_skip.line,
                    clock_record_type_to_c(
                        ClockRecordType::from_code(&core_skip.record_type).expect("record type")
                    )
                )
            );

            let mut notices = vec![std::mem::zeroed::<SidereonClockNotice>(); info.notice_count];
            assert_eq!(
                sidereon_rinex_clock_notices(
                    &clock,
                    notices.as_mut_ptr(),
                    notices.len(),
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(notices.len(), core.notices().len());
            for (got, want) in notices.iter().zip(core.notices()) {
                assert_notice_matches(got, &clock_notice_to_c(want));
            }

            let mut records = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_records(&clock, &mut records),
                SidereonStatus::Ok
            );
            for (index, core_record) in core.records().enumerate() {
                let mut got = std::mem::zeroed::<SidereonClockRecord>();
                assert_eq!(
                    sidereon_clock_records_get(records, index, &mut got),
                    SidereonStatus::Ok
                );
                let want = clock_record_to_c("test", &core_record).expect("record");
                assert_record_matches(&got, &want);
            }
            let (_, name) = text_of(|out, len, written, required| {
                sidereon_clock_records_name(records, 0, out, len, written, required)
            });
            // synthetic_rinex_clock.clk line 4 names the receiver ONSA.
            assert_eq!(name, "ONSA");
            sidereon_clock_records_free(records);

            let (_, line) = text_of(|out, len, written, required| {
                let mut present = false;
                let status = sidereon_rinex_clock_source_line(
                    &clock,
                    4,
                    out,
                    len,
                    written,
                    required,
                    &mut present,
                );
                assert!(present);
                status
            });
            assert_eq!(line, CLOCK_FIXTURE.lines().nth(3).expect("line 4"));
        }
    }

    #[test]
    fn a_refused_batch_changes_nothing_and_a_removal_keeps_the_other_lines() {
        let mut clock = lossy(CLOCK_FIXTURE);
        // sidereon-core's own reading and removal, for the expected values.
        let mut core = RinexClock::parse_lossy(CLOCK_FIXTURE);
        let core_record_count = core.record_count();
        unsafe {
            let mut edits = [SidereonClockRecordValues {
                index: 1,
                value_count: 2,
                values: [-2.0e-4, 4.0e-11, 0.0, 0.0, 0.0, 0.0],
            }; 2];
            // One past the last record the product holds.
            edits[1].index = core_record_count;
            let mut edited = usize::MAX;
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_set_records_values(
                    &mut clock,
                    edits.as_ptr(),
                    edits.len(),
                    &mut edited,
                    &mut result
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(edited, 0);
            let outcome = (*result).outcome;
            assert_eq!(
                outcome.error.kind,
                SidereonRinexClockErrorKind::InvalidInput
            );
            let (_, field) = text_of(|out, len, written, required| {
                sidereon_rinex_clock_result_error_text(
                    result,
                    SidereonRinexClockErrorText::Field as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            assert_eq!(field, "edits");
            sidereon_rinex_clock_result_free(result);
            assert_eq!(
                clock.inner.to_rinex_string().expect("unchanged"),
                CLOCK_FIXTURE
            );

            let indices = [0usize, 0];
            let mut removed = usize::MAX;
            assert_eq!(
                sidereon_rinex_clock_remove_records(
                    &mut clock,
                    indices.as_ptr(),
                    indices.len(),
                    &mut removed,
                    ptr::null_mut()
                ),
                SidereonStatus::Ok
            );
            let mut position = 0usize;
            let core_removed = core.retain_records(|_| {
                let keep = position != 0;
                position += 1;
                keep
            });
            assert_eq!(removed, core_removed);
            assert_eq!(clock.inner.record_count(), core.record_count());
            assert_eq!(
                clock.inner.skipped_records().len(),
                core.skipped_records().len()
            );
            let written = clock.inner.to_rinex_string().expect("write");
            assert_eq!(written, core.to_rinex_string().expect("core write"));
            // The removal keeps every other line as read.
            let expected: String = CLOCK_FIXTURE
                .split_inclusive('\n')
                .enumerate()
                .filter(|(index, _)| *index != 3)
                .map(|(_, line)| line)
                .collect();
            assert_eq!(written, expected);
        }
    }

    #[test]
    fn a_line_a_lossy_read_cannot_read_is_kept_with_a_typed_diagnostic() {
        let text = concat!(
            "     3.00           C                   G                   RINEX VERSION / TYPE\n",
            "                                                            END OF HEADER\n",
            "AS G05  2026 05 13 00 00  bad-second  1   2.0e-04\n",
        );
        let clock = lossy(text);
        // sidereon-core's own lossy diagnostic and strict refusal.
        let core_lossy = RinexClock::parse_lossy(text);
        let core_diagnostic = &core_lossy.diagnostics()[0];
        let core_refusal = RinexClock::parse(text).expect_err("core refuses the line");
        unsafe {
            let mut diagnostic = std::mem::zeroed::<SidereonClockDiagnostic>();
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_rinex_clock_diagnostics(
                    &clock,
                    &mut diagnostic,
                    1,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            let count = core_lossy.diagnostics().len();
            assert_eq!((written, required), (count.min(1), count));
            assert_eq!(diagnostic.line, core_diagnostic.line);
            assert_eq!(
                diagnostic.error.kind,
                clock_error_to_c(&core_diagnostic.error).kind
            );
            let (_, message) = text_of(|out, len, written, required| {
                sidereon_rinex_clock_diagnostic_text(
                    &clock,
                    0,
                    SidereonRinexClockErrorText::Message as u32,
                    out,
                    len,
                    written,
                    required,
                )
            });
            assert_eq!(
                Some(message),
                clock_error_text(
                    &core_diagnostic.error,
                    SidereonRinexClockErrorText::Message as u32
                )
            );
            assert_eq!(clock.inner.to_rinex_string().expect("restated"), text);

            let mut parsed = ptr::null_mut();
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_rinex_clock_parse_result(
                    text.as_ptr(),
                    text.len(),
                    &mut parsed,
                    &mut result
                ),
                SidereonStatus::Ok
            );
            assert!(parsed.is_null());
            assert!(!(*result).outcome.is_ok);
            let core_error = clock_error_to_c(&core_refusal);
            assert_eq!((*result).outcome.error.kind, core_error.kind);
            assert_eq!(
                (
                    (*result).outcome.error.has_line,
                    (*result).outcome.error.line
                ),
                (core_error.has_line, core_error.line)
            );
            sidereon_rinex_clock_result_free(result);
        }
    }
}
