use super::*;
use sidereon_core::data::{ArchiveCompression, DistributionSource, ProductDate};
use sidereon_core::ephemeris::{
    check_continuity, parse_exact_sp3 as core_parse_exact_sp3,
    validate_exact_sp3 as core_validate_exact_sp3, ContinuityDefect, ContinuityOptionRejection,
    ContinuityOptions, ContinuityOptionsError, EpochWindow,
    ExactSp3Coverage as CoreExactSp3Coverage, ExactSp3Request as CoreExactSp3Request,
    ExactSp3ValidationError, MergeContinuityViolation, MergeToleranceError, MergeToleranceField,
    OrbitClass, Sp3EpochIntervalError, Sp3EpochIntervalRejection, Sp3WriteError, SpeedBound,
    StencilExtent, UnusableSampleReason, WindowContinuityDecision, WindowContinuityVerdict,
};

const SP3_FRAME_LABEL_MAX_BYTES: usize = 64;
pub const SP3_ARTIFACT_SHA256_C_BYTES: usize = 65;
pub const SP3_ARTIFACT_FILENAME_C_BYTES: usize = 160;

/// Category of the latest structured SP3 validation failure on this thread.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3ErrorKind {
    /// No structured SP3 failure is recorded.
    None = 0,
    /// Exact SP3 content validation refused a product.
    ExactValidation = 1,
    /// An SP3 epoch interval was invalid or unwritable.
    EpochInterval = 2,
    /// A merge tolerance was negative or non-finite.
    MergeTolerance = 3,
    /// A continuity bound was negative or non-finite.
    ContinuityOptions = 4,
}

/// Stable field discriminant in [`SidereonSp3ErrorInfo`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3ErrorField {
    /// No field applies.
    None = 0,
    /// The declared line-1 start differs from the requested start.
    DeclaredStart = 1,
    /// The explicit merge target interval.
    TargetEpochInterval = 2,
    /// The cadence derived for the merged output.
    MergedEpochInterval = 3,
    /// Position consensus tolerance.
    PositionTolerance = 4,
    /// Clock consensus tolerance.
    ClockTolerance = 5,
    /// Outlier position tolerance.
    OutlierPositionTolerance = 6,
    /// Outlier clock tolerance.
    OutlierClockTolerance = 7,
    /// Explicit continuity speed bound.
    SpeedBound = 8,
    /// Continuity hold-out residual tolerance.
    ResidualToleranceM = 9,
    /// SP3 bytes or logical records.
    Sp3Content = 10,
    /// Catalog identity fields.
    CatalogIdentity = 11,
    /// Product family.
    ProductFamily = 12,
    /// Product issue token.
    IssueToken = 13,
    /// Requested product span token.
    SpanToken = 14,
    /// Requested sampling token.
    SampleToken = 15,
    /// Expected producing agency.
    ExpectedAgency = 16,
    /// Parsed producing agency.
    ProducingAgency = 17,
    /// Terminal EOF record.
    TerminalRecord = 18,
    /// Mandatory header record counts.
    HeaderRecordCount = 19,
    /// Declared satellite count.
    SatelliteCount = 20,
    /// Satellite declarations in the header.
    SatelliteDeclarations = 21,
    /// Per-epoch satellite record sequence.
    SatelliteRecordSequence = 22,
    /// Per-epoch P/V body ordering.
    BodyRecordOrder = 23,
    /// Declared header cadence.
    HeaderCadence = 24,
    /// Declared epoch count.
    DeclaredEpochCount = 25,
    /// GPS start metadata.
    GpsStart = 26,
    /// SP3 line-2 start metadata.
    HeaderStartMetadata = 27,
    /// Parsed epoch grid.
    EpochGrid = 28,
    /// Requested span and parsed span coverage.
    RequestedSpan = 29,
    /// Requested SP3 format revision.
    FormatVersion = 30,
    /// A field this binding does not name.
    Other = 255,
}

/// Stable validation reason discriminant in [`SidereonSp3ErrorInfo`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3ErrorReason {
    /// No reason applies.
    None = 0,
    /// Declared and requested start ticks differ.
    Mismatch = 1,
    /// The supplied value was NaN or infinite.
    NotFinite = 2,
    /// The supplied bound or tolerance was negative.
    Negative = 3,
    /// The supplied epoch interval was zero or negative.
    NotPositive = 4,
    /// The supplied interval was not an integral number of 10 ns ticks.
    NotWholeTicks = 5,
    /// f64 spacing cannot identify one whole tick at this magnitude.
    BeyondTickResolution = 6,
    /// The interval was at or above the SP3 100000 s limit.
    OutsideSpecificationRange = 7,
    /// A token, identity or header value was invalid.
    Invalid = 8,
    /// Required input or record was missing.
    Missing = 9,
    /// A valid value is unsupported by the exact SP3 contract.
    Unsupported = 10,
    /// SP3 content is malformed.
    Malformed = 11,
    /// A valid token was not written in canonical form.
    NonCanonical = 12,
    /// A satellite declaration is duplicated.
    Duplicate = 13,
    /// Content follows the terminal EOF marker.
    TrailingContent = 14,
    /// A required collection or grid is empty.
    Empty = 15,
    /// Requested span is not a multiple of its cadence.
    NotMultiple = 16,
    /// The requested start predates the GPS week-numbering epoch.
    BeforeGpsEpoch = 17,
    /// A reason this binding does not name.
    Other = 255,
}

/// Structured summary of the latest SP3 validation failure on this thread.
/// Exact i128 tick values and string-safe offending values are in the JSON
/// returned by `sidereon_sp3_last_error_payload`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ErrorInfo {
    /// Stable error category.
    pub kind: SidereonSp3ErrorKind,
    /// Stable field discriminator.
    pub field: SidereonSp3ErrorField,
    /// Stable rejection reason.
    pub reason: SidereonSp3ErrorReason,
    /// Whether value carries the rejected floating-point input.
    pub has_value: bool,
    /// Rejected value; may be NaN or infinite when present.
    pub value: f64,
    /// Whether the payload carries requested_tick.
    pub has_requested_tick: bool,
    /// Whether the payload carries a decimal declared_tick string.
    pub has_declared_tick: bool,
    /// Whether the payload carries requested_j2000_s.
    pub has_requested_j2000_s: bool,
    /// Whether the payload carries declared_j2000_s.
    pub has_declared_j2000_s: bool,
    /// Exact f64 diagnostic value for requested_j2000_s.
    pub requested_j2000_s: f64,
    /// Exact f64 diagnostic value for declared_j2000_s.
    pub declared_j2000_s: f64,
    /// UTF-8 payload length in bytes, excluding a terminator.
    pub payload_len: usize,
}

thread_local! {
    static LAST_SP3_TYPED_ERROR: RefCell<Option<(SidereonSp3ErrorInfo, String)>> = const { RefCell::new(None) };
}

/// A parsed SP3 precise-ephemeris product. Opaque to C. Create with
/// sidereon_sp3_load, sidereon_sp3_load_exact, or sidereon_sp3_merge and
/// release with sidereon_sp3_free.
pub struct SidereonSp3 {
    pub(crate) inner: Sp3,
}

/// Validated, source-independent requirements for one exact SP3 product.
///
/// Create with `sidereon_sp3_exact_request_new` or
/// `sidereon_sp3_exact_request_from_identity` and release with
/// `sidereon_sp3_exact_request_free`.
pub struct SidereonExactSp3Request {
    inner: CoreExactSp3Request,
}

/// Which regular epoch-grid boundary representation an exact SP3 used.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonExactSp3Coverage {
    /// The declared boundary is excluded.
    HalfOpen = 0,
    /// The declared boundary epoch is present.
    Inclusive = 1,
}

/// An SP3 merge audit report. Opaque to C. Create with sidereon_sp3_merge and
/// release with sidereon_sp3_merge_report_free.
pub struct SidereonSp3MergeReport {
    pub(crate) inner: MergeReport,
    /// Per-epoch agreement aggregate, computed once at merge time so the C
    /// accessors are O(1) lookups rather than recomputing the rollup per call.
    pub(crate) epoch_agreement: Vec<EpochAgreement>,
}

/// Canonical, versioned identity of exact SP3 merge inputs and policy.
///
/// The handle retains both the distributor-independent canonical contributor
/// order and, for precedence merges, the caller's semantic priority order.
/// Release it with `sidereon_sp3_merge_input_identity_free`.
pub struct SidereonSp3MergeInputIdentity {
    inner: Sp3MergeInputIdentity,
}

/// Exact parsed state of one satellite at one SP3 epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3State {
    /// ECEF position in meters.
    pub position_m: [f64; 3],
    /// Whether clock_s is present.
    pub has_clock_s: bool,
    /// Clock offset in seconds when has_clock_s is true.
    pub clock_s: f64,
    /// Whether velocity_m_s is present.
    pub has_velocity_m_s: bool,
    /// ECEF velocity in meters per second when has_velocity_m_s is true.
    pub velocity_m_s: [f64; 3],
    /// Whether clock_rate_s_s is present.
    pub has_clock_rate_s_s: bool,
    /// Clock rate in seconds per second when has_clock_rate_s_s is true.
    pub clock_rate_s_s: f64,
    /// Clock discontinuity flag.
    pub clock_event: bool,
    /// Clock prediction flag.
    pub clock_predicted: bool,
    /// Satellite maneuver flag.
    pub maneuver: bool,
    /// Orbit prediction flag.
    pub orbit_predicted: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonSp3AccuracyValueKind {
    Known = 0,
    Unknown = 1,
    TooLarge = 2,
    InvalidBase = 3,
    Overflow = 4,
    Other = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3AccuracyValue {
    pub kind: u32,
    pub value: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3AccuracyCodeGroup {
    pub has_axis_exponents: [bool; 3],
    pub axis_exponents: [i16; 3],
    pub has_clock_exponent: bool,
    pub clock_exponent: i16,
    pub has_position_velocity_base: bool,
    pub position_velocity_base: f64,
    pub has_clock_rate_base: bool,
    pub clock_rate_base: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3RawRecordAccuracy {
    pub has_p: bool,
    pub p: SidereonSp3AccuracyCodeGroup,
    pub has_v: bool,
    pub v: SidereonSp3AccuracyCodeGroup,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3PositionClockAccuracy {
    pub position_sigma_m: [SidereonSp3AccuracyValue; 3],
    pub clock_sigma_m: SidereonSp3AccuracyValue,
    pub position_variance_m2: [SidereonSp3AccuracyValue; 3],
    pub clock_variance_m2: SidereonSp3AccuracyValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3VelocityAccuracy {
    pub velocity_sigma_m_s: [SidereonSp3AccuracyValue; 3],
    pub clock_rate_sigma_m_s: SidereonSp3AccuracyValue,
    pub velocity_variance_m2_s2: [SidereonSp3AccuracyValue; 3],
    pub clock_rate_variance_m2_s2: SidereonSp3AccuracyValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3RecordAccuracy {
    pub has_p: bool,
    pub p: SidereonSp3PositionClockAccuracy,
    pub has_v: bool,
    pub v: SidereonSp3VelocityAccuracy,
}

/// How agreeing SP3 sources are combined during merge.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3MergeCombine {
    /// Arithmetic mean of agreeing sources.
    Mean = 0,
    /// Component-wise median of agreeing sources.
    Median = 1,
    /// Highest-precedence agreeing source, using input order.
    Precedence = 2,
}

/// Scope used by precedence-mode SP3 source selection.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3MergePrecedenceScope {
    /// Select the highest-precedence source present in each cell.
    Cell = 0,
    /// Keep one source owner for an entire satellite arc.
    SatelliteArc = 1,
}

/// Controls for merging SP3 products. Initialize with
/// sidereon_sp3_merge_options_init before overriding fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeOptions {
    /// Maximum agreeing-source 3D position difference, meters. Must be finite
    /// and non-negative.
    pub position_tolerance_m: f64,
    /// Maximum agreeing-source clock difference after datum alignment, seconds.
    /// Must be finite and non-negative.
    pub clock_tolerance_s: f64,
    /// Minimum agreeing sources required when several sources cover one cell.
    pub min_agree: usize,
    /// Minimum common clocked satellites for clock-datum alignment.
    pub clock_min_common: usize,
    /// One of SidereonSp3MergeCombine_*.
    pub combine: u32,
    /// One of SidereonSp3MergePrecedenceScope_*.
    pub precedence_scope: u32,
    /// Enable contested-cell outlier rejection.
    /// Exactly 0 or 1.
    pub outlier_reject_enabled: u8,
    /// Position tolerance for the outlier guard, meters.
    pub outlier_reject_position_tolerance_m: f64,
    /// Clock tolerance for the outlier guard, seconds.
    pub outlier_reject_clock_tolerance_s: f64,
    /// Whether target_epoch_interval_s is supplied.
    /// Exactly 0 or 1.
    pub target_epoch_interval_s_enabled: u8,
    /// Output epoch spacing in seconds when enabled.
    pub target_epoch_interval_s: f64,
    /// Optional array of SidereonGnssSystem_* values encoded as uint32_t.
    pub systems: *const u32,
    /// Number of entries in systems. Zero means no system filter.
    pub system_count: usize,
    /// Optional array of asserted coordinate-label sets.
    pub asserted_frame_label_sets: *const SidereonSp3FrameLabelSet,
    /// Number of entries in asserted_frame_label_sets.
    pub asserted_frame_label_set_count: usize,
    /// Enable catalog Helmert reconciliation between known ITRF/IGS labels.
    /// Exactly 0 or 1.
    pub helmert_frame_reconciliation: u8,
    /// Per-epoch provenance to record, a SidereonSp3ProvenanceMode value. Off
    /// (the default) records none. Recording never changes the merged product.
    pub provenance_mode: u32,
    /// Whether to verify the merged product's continuity as a post-condition
    /// under verify_continuity. Exactly 0 or 1; 0 (the default) runs no check.
    /// Verification never changes the product or fails the merge; its findings
    /// are read with sidereon_sp3_merge_report_continuity_json and the window
    /// verdicts.
    pub verify_continuity_enabled: u8,
    /// Continuity checks for the post-condition, read when
    /// verify_continuity_enabled is 1. sidereon_sp3_merge_options_init fills
    /// the MEO GNSS settings (sidereon_sp3_continuity_options_for_orbit_class).
    pub verify_continuity: SidereonSp3ContinuityOptions,
}

/// Orbit class whose earth-fixed speed bound a continuity check applies
/// (sidereon_core::ephemeris::OrbitClass).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3OrbitClass {
    /// GNSS medium earth orbit.
    MeoGnss = 0,
    /// Geosynchronous orbit.
    Geosynchronous = 1,
    /// Low earth orbit.
    Leo = 2,
}

/// Source of a continuity check's adjacent-pair speed bound
/// (sidereon_core::ephemeris::SpeedBound, or none).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3SpeedBoundKind {
    /// No speed gate.
    None = 0,
    /// The bound of orbit_class.
    OrbitClass = 1,
    /// The caller's explicit_max_speed_m_s.
    ExplicitMaxSpeed = 2,
}

/// Continuity checks (sidereon_core::ephemeris::ContinuityOptions). Fill with
/// sidereon_sp3_continuity_options_for_orbit_class, then override fields.
/// Invalid bounds return SIDEREON_STATUS_INVALID_ARGUMENT; the thread-local
/// error message and sidereon_sp3_last_error_info/payload identify the field,
/// supplied value and rejection reason.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ContinuityOptions {
    /// The speed gate, a SidereonSp3SpeedBoundKind value.
    pub speed_bound_kind: u32,
    /// The orbit class, a SidereonSp3OrbitClass value, read when
    /// speed_bound_kind is OrbitClass.
    pub orbit_class: u32,
    /// Explicit earth-fixed speed bound, meters per second, read when
    /// speed_bound_kind is ExplicitMaxSpeed; finite and non-negative.
    pub explicit_max_speed_m_s: f64,
    /// Whether the hold-out residual check runs. Exactly 0 or 1.
    pub residual_tolerance_enabled: u8,
    /// Hold-out residual tolerance, meters, read when
    /// residual_tolerance_enabled is 1; finite and non-negative.
    pub residual_tolerance_m: f64,
    /// Multiple of the nominal spacing above which a node gap is a coverage
    /// gap for the hold-out replay; as for the other gap_threshold_factor
    /// arguments, a value <= 0.0 selects the default 1.5 and NaN, infinity or
    /// a value in (0.0, 1.0] is refused.
    pub gap_threshold_factor: f64,
}

/// Complete, verified identity of one exact artifact supplied to an SP3 merge.
///
/// Retrieval timestamps, URLs, HTTP metadata, credentials, cache paths, and
/// retry history are deliberately absent because they are observational facts,
/// not reproducible merge inputs. Fixed text buffers must be null-terminated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ArtifactIdentity {
    /// Exact identity requested from the selected distributor.
    pub requested_identity: SidereonProductIdentity,
    /// Identity resolved by parsing and validating the product bytes.
    pub resolved_identity: SidereonProductIdentity,
    /// Explicit distributor that supplied the artifact.
    /// One of SidereonDistributionSource_*, encoded as uint32_t.
    pub distribution_source: u32,
    /// Official decompressed product filename.
    pub official_filename: [c_char; SP3_ARTIFACT_FILENAME_C_BYTES],
    /// Lower-case SHA-256 of the validated decompressed product bytes.
    pub product_sha256: [c_char; SP3_ARTIFACT_SHA256_C_BYTES],
    /// Length of the validated decompressed product bytes.
    pub product_byte_length: u64,
    /// Lower-case SHA-256 of the exact distributor archive bytes.
    pub archive_sha256: [c_char; SP3_ARTIFACT_SHA256_C_BYTES],
    /// Length of the exact distributor archive bytes.
    pub archive_byte_length: u64,
    /// Compression applied to the distributor archive.
    /// One of SidereonArchiveCompression_*, encoded as uint32_t.
    pub compression: u32,
}

/// One caller-asserted set of SP3 coordinate labels.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3FrameLabelSet {
    /// UTF-8 label pointers.
    pub labels: *const *const c_char,
    /// Number of labels. Must be at least two.
    pub label_count: usize,
}

/// Method used to reconcile one SP3 source coordinate label.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3FrameReconciliationMethod {
    /// Caller asserted the labels are equivalent; no math was applied.
    AssertedEquivalence = 0,
    /// Catalog Helmert reconciliation, or exact identity for the same realization.
    Helmert = 1,
}

/// One SP3 coordinate-label reconciliation report row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3FrameReconciliation {
    /// Source index in the sidereon_sp3_merge input array.
    pub source_index: usize,
    /// Source label byte length, copied separately.
    pub source_label_len: usize,
    /// Target label byte length, copied separately.
    pub target_label_len: usize,
    /// Reconciliation method.
    pub method: SidereonSp3FrameReconciliationMethod,
    /// Number of labels in the caller assertion set.
    pub asserted_label_count: usize,
    /// Whether source_frame is present.
    pub source_frame_present: bool,
    /// Resolved source frame as SidereonTerrestrialFrame.
    pub source_frame: u32,
    /// Whether target_frame is present.
    pub target_frame_present: bool,
    /// Resolved target frame as SidereonTerrestrialFrame.
    pub target_frame: u32,
    /// Whether catalog_source_frame and catalog_target_frame are present.
    pub catalog_frame_present: bool,
    /// Published catalog row source as SidereonTerrestrialFrame.
    pub catalog_source_frame: u32,
    /// Published catalog row target as SidereonTerrestrialFrame.
    pub catalog_target_frame: u32,
    /// Whether the published catalog row was applied in reverse.
    pub catalog_inverse: bool,
    /// Whether reference_epoch_year is present.
    pub reference_epoch_year_present: bool,
    /// Published transform reference epoch.
    pub reference_epoch_year: f64,
    /// Whether parameters are present.
    pub parameters_present: bool,
    /// Published translation parameters in millimetres.
    pub translation_mm: [f64; 3],
    /// Published scale parameter in parts per billion.
    pub scale_ppb: f64,
    /// Published rotation parameters in milliarcseconds.
    pub rotation_mas: [f64; 3],
    /// Whether rates are present.
    pub rates_present: bool,
    /// Published translation rates in millimetres per year.
    pub translation_mm_per_year: [f64; 3],
    /// Published scale rate in parts per billion per year.
    pub scale_ppb_per_year: f64,
    /// Published rotation rates in milliarcseconds per year.
    pub rotation_mas_per_year: [f64; 3],
    /// Provenance byte length, copied separately.
    pub provenance_len: usize,
    /// Whether epoch_year_start and epoch_year_end are present.
    pub epoch_year_span_present: bool,
    /// First affected decimal year.
    pub epoch_year_start: f64,
    /// Last affected decimal year.
    pub epoch_year_end: f64,
    /// Number of satellite position records covered by the reconciliation.
    pub records_affected: usize,
    /// True when both labels resolved to the same terrestrial realization.
    pub identity: bool,
}

/// Which merge report flag list to query.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3MergeFlagKind {
    /// Cells omitted because sources disagreed beyond tolerance.
    Quarantined = 0,
    /// Cells carried from one source because no cross-check was possible.
    SingleSource = 1,
    /// Cells where an accepted consensus rejected source outliers.
    PositionOutlier = 2,
    /// Clock contributors rejected from an accepted consensus or guard.
    ClockOutlier = 3,
    /// Cells whose position some source carried but satellite-arc precedence
    /// did not write, because the arc owner carried none there. Each flag's
    /// sources are the sources whose positions were withheld.
    ArcWithheld = 4,
}

/// Product-wide SP3 observed/predicted boundary metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3PredictionSummary {
    /// Number of per-epoch prediction rows.
    pub epoch_count: usize,
    /// Whether observed_through_j2000_seconds is present.
    pub observed_through_present: bool,
    /// Last contiguous observed epoch as seconds since J2000 when present.
    pub observed_through_j2000_seconds: f64,
}

/// Prediction status aggregated over every satellite record at one SP3 epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3EpochPrediction {
    /// Epoch as seconds since J2000 in the product time scale.
    pub epoch_j2000_seconds: f64,
    /// True when no orbit or clock record at this epoch is predicted.
    pub observed: bool,
    /// Number of orbit-predicted satellites. Query exact ids through
    /// sidereon_sp3_state and its orbit_predicted flag.
    pub orbit_predicted_satellite_count: usize,
    /// Number of clock-predicted satellites. Query exact ids through
    /// sidereon_sp3_state and its clock_predicted flag.
    pub clock_predicted_satellite_count: usize,
}

/// One SP3 merge audit flag. Source indices are copied with
/// sidereon_sp3_merge_report_flag_sources.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeFlag {
    /// Flagged epoch as seconds since J2000 in the product time scale; NaN
    /// when the exact epoch has no such reading.
    pub epoch_j2000_seconds: f64,
    /// Flagged epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Number of source indices attached to this flag.
    pub source_count: usize,
}

/// Per-epoch aggregate of the merge agreement metric: how tightly the consensus
/// sources clustered about the combined value written to the merged product,
/// pooled over the multi-source satellites at one output epoch. Mirrors
/// sidereon_core::ephemeris::EpochAgreement. Copied with
/// sidereon_sp3_merge_report_epoch_agreement; the entries are in output-epoch
/// order (count from sidereon_sp3_merge_report_epoch_agreement_count), one per
/// output epoch: the engine groups the cells of one epoch by its exact instant.
///
/// Every spread carries a present flag. An absent spread is NaN, never 0: a
/// zero spread is a measured agreement of two or more sources, and an epoch
/// whose cells were all single-source, or whose accepted cells carry only a
/// clock and no orbit, has no position spread at all.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3EpochAgreement {
    /// Output epoch as seconds since J2000 in the product time scale; NaN when
    /// the exact epoch has no such reading.
    pub epoch_j2000_seconds: f64,
    /// Output epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// Satellites at this epoch with a multi-source position consensus. Zero when
    /// no cell at the epoch had two or more position consensus members; the
    /// position present flags are then false.
    pub satellites: usize,
    /// True when position_rms_m carries a value: some cell at this epoch had
    /// two or more position consensus members.
    pub position_rms_present: bool,
    /// Member-count-weighted pooled RMS of the per-cell position dispersion over
    /// the multi-source satellites at this epoch, meters. NaN when
    /// position_rms_present is false.
    pub position_rms_m: f64,
    /// True when position_max_m carries a value, under the same condition as
    /// position_rms_present.
    pub position_max_present: bool,
    /// Worst per-cell position dispersion over the multi-source satellites at
    /// this epoch, meters. NaN when position_max_present is false.
    pub position_max_m: f64,
    /// True when clock_rms_s carries a value (a multi-source clock consensus
    /// existed at this epoch).
    pub clock_rms_present: bool,
    /// Pooled RMS of the per-cell clock dispersion at this epoch, seconds. NaN
    /// when clock_rms_present is false.
    pub clock_rms_s: f64,
    /// True when clock_max_s carries a value.
    pub clock_max_present: bool,
    /// Worst per-cell clock dispersion at this epoch, seconds. NaN when
    /// clock_max_present is false.
    pub clock_max_s: f64,
}

/// Per-epoch clock-reference offset of one SP3 product relative to another.
/// Copied by sidereon_sp3_clock_reference_offsets.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ClockReferenceOffset {
    /// Matched epoch as seconds since J2000 in the product time scale.
    pub epoch_j2000_seconds: f64,
    /// Other minus reference clock datum, seconds.
    pub offset_s: f64,
    /// Number of common clocked satellites used by the median estimate.
    pub satellites: usize,
}

/// Whole-product rollup of the merge agreement metric: the pooled position/clock
/// dispersion of the consensus members about the combined values. Each scalar
/// mirrors a sidereon_core::ephemeris::MergeReport agreement method and carries a
/// present flag, but the present condition differs by field (see each below): the
/// pooled RMS fields are present only when some accepted cell had a multi-source
/// consensus on that channel, whereas the max fields are present whenever some
/// accepted cell carried that channel (a single-source cell has zero dispersion,
/// not an absent max). An accepted cell that carries only a clock has no
/// position, so a product whose accepted cells are all clock-only reports both
/// position fields absent. An absent scalar is NaN, never 0. Written by
/// sidereon_sp3_merge_report_agreement_summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3AgreementSummary {
    /// True when position_rms_m carries a value (some cell had >= 2 position
    /// consensus members).
    pub position_rms_present: bool,
    /// Member-count-weighted pooled RMS of the per-cell position dispersion over
    /// the whole product, meters. NaN when position_rms_present is false.
    pub position_rms_m: f64,
    /// True when position_max_m carries a value (at least one accepted cell
    /// carried a position).
    pub position_max_present: bool,
    /// Largest single-cell position dispersion over the whole product, meters.
    /// NaN when position_max_present is false.
    pub position_max_m: f64,
    /// True when clock_rms_s carries a value (some accepted cell had >= 2 clock
    /// consensus members).
    pub clock_rms_present: bool,
    /// Member-count-weighted pooled RMS of the per-cell clock dispersion over the
    /// whole product, seconds. NaN when clock_rms_present is false.
    pub clock_rms_s: f64,
    /// True when clock_max_s carries a value (there was at least one accepted cell
    /// carrying a clock).
    pub clock_max_present: bool,
    /// Largest single-cell clock dispersion over the whole product, seconds. NaN
    /// when clock_max_present is false.
    pub clock_max_s: f64,
    /// True when single_source_fraction carries a value (some cell was
    /// accepted).
    pub single_source_fraction_present: bool,
    /// Fraction of accepted cells carried from a single source, in 0..=1: the
    /// share of the product no second source cross-checked, which the
    /// dispersion fields do not cover. NaN when
    /// single_source_fraction_present is false.
    pub single_source_fraction: f64,
}

/// Bytes of the SP3 time-system label field of SidereonSp3WriteError: the
/// three-character label and its terminator.
pub const SIDEREON_SP3_TIME_SYSTEM_C_BYTES: usize = 4;

/// Which refusal an SP3 write reported. Every named kind other than None and
/// Unknown corresponds to a `Sp3WriteError` variant of the engine; discriminant
/// values 1 and 12 are reserved.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3WriteErrorKind {
    /// No refusal is recorded.
    None = 0,
    /// A header text field carries a line break, another control byte, or a
    /// non-ASCII byte. Carries the field and the text value.
    TextNotColumnSafe = 2,
    /// A header text field carries leading or trailing whitespace the reader
    /// trims. Carries the field and the text value.
    TextNotColumnStable = 3,
    /// An optional descriptor holds a blank string, which its columns read back
    /// as absent. Carries the field and the text value.
    BlankDescriptor = 4,
    /// A comment holds no text, which its record reads back as padding.
    /// Carries comment_index and the text value.
    EmptyComment = 5,
    /// A text field is wider than its columns. Carries the field, columns and
    /// the text value.
    TextTooWide = 6,
    /// An integer field is wider than its columns. Carries the field, columns
    /// and integer_value.
    IntegerTooWide = 7,
    /// A header numeric field is not finite. Carries the field.
    NonFinite = 8,
    /// A header number's F{columns}.{decimals} form is wider than its columns.
    /// Carries the field, columns, decimals and number.
    NumberTooWide = 9,
    /// A header number carries more precision than its F{columns}.{decimals}
    /// field states. Carries the field, columns, decimals and number.
    PrecisionNotRepresentable = 10,
    /// An epoch's calendar year falls outside four digits. Carries epoch_index
    /// and year.
    YearNotRepresentable = 11,
    /// An epoch record would state a different instant from the one the
    /// product holds. Carries epoch_index, field_seconds and, when the engine
    /// measured it, residual_s.
    EpochNotRestatable = 13,
    /// An epoch is tagged with a different time scale than the header states.
    /// Carries epoch_index, epoch_time_scale and header_time_scale.
    EpochTimeScaleMismatch = 14,
    /// The header's SP3 time system and core time scale disagree. Carries
    /// time_system and header_time_scale.
    HeaderTimeScaleMismatch = 15,
    /// The header epoch count differs from the epochs the product holds.
    /// Carries declared_epochs and epochs.
    EpochCountMismatch = 16,
    /// The accuracy codes are not index-aligned with the header satellite list.
    /// Carries satellites and codes.
    AccuracyCodeCountMismatch = 17,
    /// The header satellite list names one satellite twice. Carries sat_id.
    DuplicateSatellite = 18,
    /// A per-epoch array is not parallel to the epoch list. Carries the field,
    /// epochs and entries.
    EpochArrayLengthMismatch = 19,
    /// A record belongs to a satellite the header does not declare. Carries
    /// sat_id and epoch_index.
    UndeclaredSatelliteRecord = 20,
    /// A satellite holds both a state and a clock-only record at one epoch.
    /// Carries sat_id and epoch_index.
    ConflictingRecords = 21,
    /// A position product holds velocity or clock-rate state. Carries the
    /// field, sat_id and epoch_index.
    VelocityStateInPositionProduct = 22,
    /// A record field is not finite. Carries the field, sat_id and epoch_index.
    RecordValueNonFinite = 23,
    /// A record field's F{columns}.{decimals} form is wider than its columns.
    /// Carries the field, sat_id, epoch_index, columns, decimals and
    /// column_value.
    RecordValueTooWide = 24,
    /// A record field's column reads back as a different value from the one
    /// the product holds. Carries the field, sat_id, epoch_index, columns,
    /// decimals, stored and column_value.
    RecordValueNotRepresentable = 25,
    /// A record value would be written as one of the format's absence
    /// sentinels. Carries the field, sat_id, epoch_index and column_value.
    RecordReadsAsAbsent = 26,
    /// A record holds a value in product units without its native-unit value,
    /// or the other way round. Carries the field, sat_id, epoch_index and
    /// whichever of stored and native the record holds.
    RecordFieldsDisagree = 27,
    /// A header satellite has no 01..99 token that reads back as itself: a
    /// satellite number of 0 or of 100 and above, which only an identifier
    /// built without the constructor can hold. Carries sat_id, the satellite's
    /// system letter followed by its number as held.
    SatelliteNotRepresentable = 28,
    AccuracyNotRepresentable = 29,
    AccuracyRecordMismatch = 30,
    AccuracyBasisMissing = 31,
    /// A refusal this binding does not yet name. The engine's text is in the
    /// message.
    Unknown = 999,
}

/// Typed detail of a refused SP3 write.
///
/// Only the fields the kind names carry meaning, and each carries a present
/// flag. An absent number is NaN and an absent count or index is 0. The two
/// text parts -- the field name and the text value the product holds -- are
/// read from the owned SidereonSp3WriteResult with
/// sidereon_sp3_write_result_get_field and
/// sidereon_sp3_write_result_get_text_value; has_field and has_text_value say
/// whether the refusal carries them, since a text value can itself be empty.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3WriteError {
    /// Which refusal the writer reported.
    pub kind: SidereonSp3WriteErrorKind,
    /// Whether the refusal names a header or record field.
    pub has_field: bool,
    /// Whether the refusal carries the text the product holds.
    pub has_text_value: bool,
    /// Whether sat_id names the satellite the refusal concerns.
    pub has_sat_id: bool,
    /// The satellite the refusal concerns.
    pub sat_id: SidereonSatelliteToken,
    /// Whether epoch_index names an epoch.
    pub has_epoch_index: bool,
    /// Zero-based index of the epoch the refusal concerns.
    pub epoch_index: usize,
    /// Whether comment_index names a comment.
    pub has_comment_index: bool,
    /// Zero-based index of the refused comment.
    pub comment_index: usize,
    /// Whether columns carries the width of the refused field.
    pub has_columns: bool,
    /// Columns the refused field occupies.
    pub columns: usize,
    /// Whether decimals carries the decimal places of the refused field.
    pub has_decimals: bool,
    /// Decimal places the refused field carries.
    pub decimals: usize,
    /// Whether integer_value carries the refused integer.
    pub has_integer_value: bool,
    /// The integer an IntegerTooWide refusal names, as the product holds it.
    pub integer_value: u64,
    /// Whether number carries the refused header number.
    pub has_number: bool,
    /// The header number a NumberTooWide or PrecisionNotRepresentable refusal
    /// names, as the product holds it.
    pub number: f64,
    /// Whether year carries the refused calendar year.
    pub has_year: bool,
    /// The calendar year the epoch converts to.
    pub year: i64,
    /// Whether field_seconds carries the seconds the epoch record would state.
    pub has_field_seconds: bool,
    /// Seconds of minute the epoch record would state.
    pub field_seconds: f64,
    /// Whether residual_s carries a measured residual. False when the engine
    /// could read no candidate record back, which leaves no instant to measure
    /// against.
    pub has_residual_s: bool,
    /// Stored epoch minus the instant the record would state, seconds.
    pub residual_s: f64,
    /// Whether epoch_time_scale carries the epoch's scale.
    pub has_epoch_time_scale: bool,
    /// The scale the epoch is tagged with, as SidereonTimeScale.
    pub epoch_time_scale: u32,
    /// Whether header_time_scale carries the header's scale.
    pub has_header_time_scale: bool,
    /// The core time scale the header states, as SidereonTimeScale.
    pub header_time_scale: u32,
    /// Whether time_system carries the header's SP3 time-system label.
    pub has_time_system: bool,
    /// The SP3 time-system label the header states, such as "GPS",
    /// null-terminated.
    pub time_system: [c_char; SIDEREON_SP3_TIME_SYSTEM_C_BYTES],
    /// Whether declared_epochs carries the header epoch count.
    pub has_declared_epochs: bool,
    /// The epoch count header line 1 states.
    pub declared_epochs: u64,
    /// Whether epochs carries the epoch count the product holds.
    pub has_epochs: bool,
    /// Epochs the product holds.
    pub epochs: usize,
    /// Whether entries carries the length of the refused per-epoch array.
    pub has_entries: bool,
    /// Entries the refused per-epoch array holds.
    pub entries: usize,
    /// Whether satellites carries the header satellite count.
    pub has_satellites: bool,
    /// Satellites the header declares.
    pub satellites: usize,
    /// Whether codes carries the accuracy-code count.
    pub has_codes: bool,
    /// Accuracy codes held against the header satellites.
    pub codes: usize,
    /// Whether stored carries the record value in the product's own units.
    pub has_stored: bool,
    /// The record value as the product holds it, in meters, seconds, meters
    /// per second or seconds per second.
    pub stored: f64,
    /// Whether native carries the retained native-unit record value.
    pub has_native: bool,
    /// The retained native-unit record value.
    pub native: f64,
    /// Whether column_value carries the number the column would state.
    pub has_column_value: bool,
    /// The number the record column would carry, in the format's own units.
    pub column_value: f64,
    pub has_exponent: bool,
    pub exponent: i16,
}

/// The complete outcome of one SP3 write: the fixed-width part of an owned
/// SidereonSp3WriteResult.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3WriteOutcome {
    /// Whether the product was written.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK when written, otherwise
    /// SIDEREON_STATUS_INVALID_ARGUMENT, which every refusal maps to.
    pub status: SidereonStatus,
    /// The typed refusal; kind is None when is_ok is true.
    pub error: SidereonSp3WriteError,
}

/// An owned record of one SP3 write: the text on success, or the typed refusal
/// with its owned text parts. The result owns every string it reports, so they
/// stay readable after the product is freed and after later failing calls have
/// overwritten the thread-local message. Create with
/// sidereon_sp3_to_sp3_text_result and release with
/// sidereon_sp3_write_result_free.
pub struct SidereonSp3WriteResult {
    pub(crate) outcome: SidereonSp3WriteOutcome,
    /// The SP3 text on success; empty on a refusal.
    pub(crate) text: String,
    /// The refusal text, prefixed with the route that produced it; empty on
    /// success.
    pub(crate) message: String,
    /// The field a refusal names; empty when it names none.
    pub(crate) field: String,
    /// The text value a refusal carries; empty when it carries none.
    pub(crate) text_value: String,
}

fn exact_sp3_coverage_to_c(coverage: CoreExactSp3Coverage) -> SidereonExactSp3Coverage {
    match coverage {
        CoreExactSp3Coverage::HalfOpen => SidereonExactSp3Coverage::HalfOpen,
        CoreExactSp3Coverage::Inclusive => SidereonExactSp3Coverage::Inclusive,
    }
}

fn sp3_error_field_from_name(field: &str) -> SidereonSp3ErrorField {
    match field {
        "declared_start" => SidereonSp3ErrorField::DeclaredStart,
        "target_epoch_interval_s" => SidereonSp3ErrorField::TargetEpochInterval,
        "merged_epoch_interval_s" => SidereonSp3ErrorField::MergedEpochInterval,
        "position_tolerance_m" => SidereonSp3ErrorField::PositionTolerance,
        "clock_tolerance_s" => SidereonSp3ErrorField::ClockTolerance,
        "outlier_position_tolerance_m" => SidereonSp3ErrorField::OutlierPositionTolerance,
        "outlier_clock_tolerance_s" => SidereonSp3ErrorField::OutlierClockTolerance,
        "speed_bound" => SidereonSp3ErrorField::SpeedBound,
        "residual_tolerance_m" => SidereonSp3ErrorField::ResidualToleranceM,
        _ => SidereonSp3ErrorField::Other,
    }
}

fn sp3_error_kind_name(kind: SidereonSp3ErrorKind) -> &'static str {
    match kind {
        SidereonSp3ErrorKind::None => "none",
        SidereonSp3ErrorKind::ExactValidation => "exact_validation",
        SidereonSp3ErrorKind::EpochInterval => "epoch_interval",
        SidereonSp3ErrorKind::MergeTolerance => "merge_tolerance",
        SidereonSp3ErrorKind::ContinuityOptions => "continuity_options",
    }
}

fn sp3_epoch_reason(reason: Sp3EpochIntervalRejection) -> SidereonSp3ErrorReason {
    match reason {
        Sp3EpochIntervalRejection::NotFinite => SidereonSp3ErrorReason::NotFinite,
        Sp3EpochIntervalRejection::NotPositive => SidereonSp3ErrorReason::NotPositive,
        Sp3EpochIntervalRejection::NotWholeTicks => SidereonSp3ErrorReason::NotWholeTicks,
        Sp3EpochIntervalRejection::BeyondTickResolution => {
            SidereonSp3ErrorReason::BeyondTickResolution
        }
        Sp3EpochIntervalRejection::OutsideSpecificationRange => {
            SidereonSp3ErrorReason::OutsideSpecificationRange
        }
        _ => SidereonSp3ErrorReason::Other,
    }
}

fn sp3_continuity_reason(reason: ContinuityOptionRejection) -> SidereonSp3ErrorReason {
    match reason {
        ContinuityOptionRejection::NotFinite => SidereonSp3ErrorReason::NotFinite,
        ContinuityOptionRejection::Negative => SidereonSp3ErrorReason::Negative,
        _ => SidereonSp3ErrorReason::Other,
    }
}

fn record_sp3_epoch_interval_error(error: Sp3EpochIntervalError) {
    let reason = sp3_epoch_reason(error.reason);
    let reason_name = match reason {
        SidereonSp3ErrorReason::NotFinite => "not_finite",
        SidereonSp3ErrorReason::NotPositive => "not_positive",
        SidereonSp3ErrorReason::NotWholeTicks => "not_whole_ticks",
        SidereonSp3ErrorReason::BeyondTickResolution => "beyond_tick_resolution",
        SidereonSp3ErrorReason::OutsideSpecificationRange => "outside_specification_range",
        _ => "other",
    };
    let diagnostic = (reason == SidereonSp3ErrorReason::Other).then(|| error.reason.to_string());
    record_sp3_typed_error(Sp3TypedErrorDetails {
        kind: SidereonSp3ErrorKind::EpochInterval,
        field: sp3_error_field_from_name(error.field),
        reason,
        field_name: error.field,
        reason_name,
        value: Some(error.value),
        requested_tick: None,
        declared_tick: None,
        requested_j2000_s: None,
        declared_j2000_s: None,
        details: None,
        diagnostic: diagnostic.as_deref(),
    });
}

fn record_sp3_merge_tolerance_error(error: MergeToleranceError) {
    let (field, field_name) = match error.field {
        MergeToleranceField::Position => (
            SidereonSp3ErrorField::PositionTolerance,
            "position_tolerance_m",
        ),
        MergeToleranceField::Clock => (SidereonSp3ErrorField::ClockTolerance, "clock_tolerance_s"),
        MergeToleranceField::OutlierPosition => (
            SidereonSp3ErrorField::OutlierPositionTolerance,
            "outlier_position_tolerance_m",
        ),
        MergeToleranceField::OutlierClock => (
            SidereonSp3ErrorField::OutlierClockTolerance,
            "outlier_clock_tolerance_s",
        ),
        _ => (SidereonSp3ErrorField::Other, "other_tolerance"),
    };
    let reason = if error.value.is_finite() {
        SidereonSp3ErrorReason::Negative
    } else {
        SidereonSp3ErrorReason::NotFinite
    };
    let reason_name = match reason {
        SidereonSp3ErrorReason::Negative => "negative",
        SidereonSp3ErrorReason::NotFinite => "not_finite",
        _ => "other",
    };
    record_sp3_typed_error(Sp3TypedErrorDetails {
        kind: SidereonSp3ErrorKind::MergeTolerance,
        field,
        reason,
        field_name,
        reason_name,
        value: Some(error.value),
        requested_tick: None,
        declared_tick: None,
        requested_j2000_s: None,
        declared_j2000_s: None,
        details: None,
        diagnostic: None,
    });
}

fn record_sp3_continuity_options_error(error: ContinuityOptionsError) {
    let reason = sp3_continuity_reason(error.reason);
    let reason_name = match reason {
        SidereonSp3ErrorReason::NotFinite => "not_finite",
        SidereonSp3ErrorReason::Negative => "negative",
        _ => "other",
    };
    let diagnostic = (reason == SidereonSp3ErrorReason::Other).then(|| error.reason.to_string());
    record_sp3_typed_error(Sp3TypedErrorDetails {
        kind: SidereonSp3ErrorKind::ContinuityOptions,
        field: sp3_error_field_from_name(error.field),
        reason,
        field_name: error.field,
        reason_name,
        value: Some(error.value),
        requested_tick: None,
        declared_tick: None,
        requested_j2000_s: None,
        declared_j2000_s: None,
        details: None,
        diagnostic: diagnostic.as_deref(),
    });
}

struct Sp3TypedErrorDetails<'a> {
    kind: SidereonSp3ErrorKind,
    field: SidereonSp3ErrorField,
    reason: SidereonSp3ErrorReason,
    field_name: &'a str,
    reason_name: &'a str,
    value: Option<f64>,
    requested_tick: Option<i128>,
    declared_tick: Option<i128>,
    requested_j2000_s: Option<f64>,
    declared_j2000_s: Option<f64>,
    details: Option<serde_json::Value>,
    diagnostic: Option<&'a str>,
}

fn record_sp3_typed_error(details: Sp3TypedErrorDetails<'_>) {
    let Sp3TypedErrorDetails {
        kind,
        field,
        reason,
        field_name,
        reason_name,
        value,
        requested_tick,
        declared_tick,
        requested_j2000_s,
        declared_j2000_s,
        details,
        diagnostic,
    } = details;
    let payload = serde_json::json!({
        "schema_version": 1,
        "error": {
            "kind": sp3_error_kind_name(kind),
            "field": field_name,
            "reason": reason_name,
            "value": value.map(|number| number.to_string()),
            "requested_tick": requested_tick.map(|tick| tick.to_string()),
            "declared_tick": declared_tick.map(|tick| tick.to_string()),
            "requested_j2000_s": requested_j2000_s.map(|value| value.to_string()),
            "declared_j2000_s": declared_j2000_s.map(|value| value.to_string()),
            "details": details,
            "diagnostic": diagnostic,
        }
    })
    .to_string();
    let info = SidereonSp3ErrorInfo {
        kind,
        field,
        reason,
        has_value: value.is_some(),
        value: value.unwrap_or(f64::NAN),
        has_requested_tick: requested_tick.is_some(),
        has_declared_tick: declared_tick.is_some(),
        has_requested_j2000_s: requested_j2000_s.is_some(),
        has_declared_j2000_s: declared_j2000_s.is_some(),
        requested_j2000_s: requested_j2000_s.unwrap_or(f64::NAN),
        declared_j2000_s: declared_j2000_s.unwrap_or(f64::NAN),
        payload_len: payload.len(),
    };
    LAST_SP3_TYPED_ERROR.with(|slot| *slot.borrow_mut() = Some((info, payload)));
}

pub(crate) fn clear_sp3_typed_error() {
    LAST_SP3_TYPED_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Clear SP3 typed failure data at the start of each producer operation;
/// read-only info and payload accessors do not use this boundary.
pub(crate) fn sp3_operation_boundary<T>(
    fn_name: &str,
    panic_value: T,
    body: impl FnOnce() -> T,
) -> T {
    clear_sp3_typed_error();
    clear_engine_error();
    ffi_boundary(fn_name, panic_value, body)
}

/// Copy the structured summary of the latest SP3 validation failure, or a
/// `None` record after success or before any matching failure. Reading it does
/// not clear the per-thread error.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_last_error_info(
    out: *mut SidereonSp3ErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_last_error_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        *out = LAST_SP3_TYPED_ERROR.with(|slot| {
            slot.borrow().as_ref().map_or(
                SidereonSp3ErrorInfo {
                    kind: SidereonSp3ErrorKind::None,
                    field: SidereonSp3ErrorField::None,
                    reason: SidereonSp3ErrorReason::None,
                    has_value: false,
                    value: f64::NAN,
                    has_requested_tick: false,
                    has_declared_tick: false,
                    has_requested_j2000_s: false,
                    has_declared_j2000_s: false,
                    requested_j2000_s: f64::NAN,
                    declared_j2000_s: f64::NAN,
                    payload_len: 0,
                },
                |(info, _)| *info,
            )
        });
        SidereonStatus::Ok
    })
}

/// Copy the latest structured SP3 validation failure as UTF-8 JSON. Integer
/// tick counts and offending floating-point values are decimal strings, so no
/// precision is lost and non-finite values remain valid JSON. Use a null
/// output and zero length to query the required byte count. Reading the
/// payload does not clear it.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_last_error_payload(
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_last_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let payload = LAST_SP3_TYPED_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(String::new, |(_, payload)| payload.clone())
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

fn exact_sp3_error_classification(
    error: &ExactSp3ValidationError,
) -> (
    SidereonSp3ErrorField,
    &'static str,
    SidereonSp3ErrorReason,
    &'static str,
    Option<f64>,
) {
    use ExactSp3ValidationError as E;
    use SidereonSp3ErrorField as F;
    use SidereonSp3ErrorReason as R;

    match error {
        E::Parse(_) => (
            F::Sp3Content,
            "sp3_content",
            R::Malformed,
            "malformed",
            None,
        ),
        E::Catalog(_) => (
            F::CatalogIdentity,
            "catalog_identity",
            R::Invalid,
            "invalid",
            None,
        ),
        E::WrongProductFamily { .. } => (
            F::ProductFamily,
            "product_family",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::InvalidIssue { .. } => (F::IssueToken, "issue_token", R::Invalid, "invalid", None),
        E::UnsupportedSpanToken { .. } => (
            F::SpanToken,
            "span_token",
            R::Unsupported,
            "unsupported",
            None,
        ),
        E::UnsupportedSampleToken { .. } => (
            F::SampleToken,
            "sample_token",
            R::Unsupported,
            "unsupported",
            None,
        ),
        E::NonCanonicalSpanToken { .. } => (
            F::SpanToken,
            "span_token",
            R::NonCanonical,
            "non_canonical",
            None,
        ),
        E::NonCanonicalSampleToken { .. } => (
            F::SampleToken,
            "sample_token",
            R::NonCanonical,
            "non_canonical",
            None,
        ),
        E::InvalidExpectedAgency { .. } => (
            F::ExpectedAgency,
            "expected_agency",
            R::Invalid,
            "invalid",
            None,
        ),
        E::AgencyMismatch { .. } => (
            F::ProducingAgency,
            "producing_agency",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::MissingEof => (
            F::TerminalRecord,
            "terminal_record",
            R::Missing,
            "missing",
            None,
        ),
        E::MalformedEofRecord { .. } => (
            F::TerminalRecord,
            "terminal_record",
            R::Malformed,
            "malformed",
            None,
        ),
        E::TrailingContentAfterEof => (
            F::TerminalRecord,
            "terminal_record",
            R::TrailingContent,
            "trailing_content",
            None,
        ),
        E::MandatoryHeaderRecordCount { .. } => (
            F::HeaderRecordCount,
            "mandatory_header_record_count",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::MissingDeclaredSatelliteCount => (
            F::SatelliteCount,
            "satellite_count",
            R::Missing,
            "missing",
            None,
        ),
        E::DeclaredSatelliteCountMismatch { .. } => (
            F::SatelliteCount,
            "satellite_count",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::DuplicateDeclaredSatellite { .. } => (
            F::SatelliteDeclarations,
            "satellite_declarations",
            R::Duplicate,
            "duplicate",
            None,
        ),
        E::NoDeclaredSatellites => (
            F::SatelliteDeclarations,
            "satellite_declarations",
            R::Empty,
            "empty",
            None,
        ),
        E::SatelliteRecordSequenceMismatch { .. } => (
            F::SatelliteRecordSequence,
            "satellite_record_sequence",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::BodyRecordInterleavingMismatch { .. } => (
            F::BodyRecordOrder,
            "body_record_order",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::NonFiniteHeaderCadence => (
            F::HeaderCadence,
            "header_cadence",
            R::NotFinite,
            "not_finite",
            Some(f64::NAN),
        ),
        E::NonPositiveHeaderCadence { actual_s } => (
            F::HeaderCadence,
            "header_cadence",
            R::NotPositive,
            "not_positive",
            Some(*actual_s),
        ),
        E::UnsupportedHeaderCadence { actual_s } => (
            F::HeaderCadence,
            "header_cadence",
            R::Unsupported,
            "unsupported",
            Some(*actual_s),
        ),
        E::CadenceMismatch { header_s, .. } => (
            F::HeaderCadence,
            "header_cadence",
            R::Mismatch,
            "mismatch",
            Some(*header_s),
        ),
        E::DeclaredEpochCountMismatch { .. } => (
            F::DeclaredEpochCount,
            "declared_epoch_count",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::MissingDeclaredStart => (
            F::DeclaredStart,
            "declared_start",
            R::Missing,
            "missing",
            None,
        ),
        E::DeclaredStartMismatch { .. } => (
            F::DeclaredStart,
            "declared_start",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::RequestBeforeGpsEpoch => (
            F::GpsStart,
            "gps_start",
            R::BeforeGpsEpoch,
            "before_gps_epoch",
            None,
        ),
        E::NonFiniteHeaderStartMetadata { .. } => (
            F::HeaderStartMetadata,
            "header_start_metadata",
            R::NotFinite,
            "not_finite",
            Some(f64::NAN),
        ),
        E::InvalidHeaderStartMetadata { actual, .. } => (
            F::HeaderStartMetadata,
            "header_start_metadata",
            R::Invalid,
            "invalid",
            Some(*actual),
        ),
        E::HeaderStartMetadataMismatch { actual, .. } => (
            F::HeaderStartMetadata,
            "header_start_metadata",
            R::Mismatch,
            "mismatch",
            Some(*actual),
        ),
        E::EmptyEpochGrid => (F::EpochGrid, "epoch_grid", R::Empty, "empty", None),
        E::FirstEpochMismatch { actual_j2000_s, .. } => (
            F::EpochGrid,
            "epoch_grid",
            R::Mismatch,
            "mismatch",
            Some(*actual_j2000_s),
        ),
        E::IrregularEpochGrid { actual_s, .. } => (
            F::EpochGrid,
            "epoch_grid",
            R::Mismatch,
            "mismatch",
            Some(*actual_s),
        ),
        E::SpanNotMultipleOfCadence { .. } => (
            F::RequestedSpan,
            "requested_span",
            R::NotMultiple,
            "not_multiple",
            None,
        ),
        E::SpanMismatch { .. } => (
            F::RequestedSpan,
            "requested_span",
            R::Mismatch,
            "mismatch",
            None,
        ),
        E::FormatVersionMismatch { .. } => (
            F::FormatVersion,
            "format_version",
            R::Mismatch,
            "mismatch",
            None,
        ),
        _ => (F::Other, "other_exact_validation", R::Other, "other", None),
    }
}

fn exact_sp3_error_details(error: &ExactSp3ValidationError) -> serde_json::Value {
    use ExactSp3ValidationError as E;
    match error {
        E::Parse(error) => {
            serde_json::json!({"variant":"parse", "error":error.to_string(), "debug":format!("{error:?}")})
        }
        E::Catalog(error) => {
            serde_json::json!({"variant":"catalog", "error":error.to_string(), "debug":format!("{error:?}")})
        }
        E::WrongProductFamily { actual } => {
            serde_json::json!({"variant":"wrong_product_family", "actual":actual.code()})
        }
        E::InvalidIssue { issue } => serde_json::json!({"variant":"invalid_issue", "issue":issue}),
        E::UnsupportedSpanToken { token } => {
            serde_json::json!({"variant":"unsupported_span_token", "token":token})
        }
        E::UnsupportedSampleToken { token } => {
            serde_json::json!({"variant":"unsupported_sample_token", "token":token})
        }
        E::NonCanonicalSpanToken { token, canonical } => {
            serde_json::json!({"variant":"non_canonical_span_token", "token":token, "canonical":canonical})
        }
        E::NonCanonicalSampleToken { token, canonical } => {
            serde_json::json!({"variant":"non_canonical_sample_token", "token":token, "canonical":canonical})
        }
        E::InvalidExpectedAgency { agency } => {
            serde_json::json!({"variant":"invalid_expected_agency", "agency":agency})
        }
        E::AgencyMismatch { expected, actual } => {
            serde_json::json!({"variant":"agency_mismatch", "expected":expected, "actual":actual})
        }
        E::MissingEof => serde_json::json!({"variant":"missing_eof"}),
        E::MalformedEofRecord {
            line_number,
            record_length,
        } => {
            serde_json::json!({"variant":"malformed_eof_record", "line_number":line_number.to_string(), "record_length":record_length.to_string()})
        }
        E::TrailingContentAfterEof => serde_json::json!({"variant":"trailing_content_after_eof"}),
        E::MandatoryHeaderRecordCount {
            record,
            expected,
            actual,
        } => {
            serde_json::json!({"variant":"mandatory_header_record_count", "record":record, "expected":expected.to_string(), "actual":actual.to_string()})
        }
        E::MissingDeclaredSatelliteCount => {
            serde_json::json!({"variant":"missing_declared_satellite_count"})
        }
        E::DeclaredSatelliteCountMismatch { declared, tokens } => {
            serde_json::json!({"variant":"declared_satellite_count_mismatch", "declared":declared.to_string(), "tokens":tokens.to_string()})
        }
        E::DuplicateDeclaredSatellite {
            token,
            first_index,
            duplicate_index,
        } => {
            serde_json::json!({"variant":"duplicate_declared_satellite", "token":token, "first_index":first_index.to_string(), "duplicate_index":duplicate_index.to_string()})
        }
        E::NoDeclaredSatellites => serde_json::json!({"variant":"no_declared_satellites"}),
        E::SatelliteRecordSequenceMismatch {
            record,
            epoch_index,
            expected,
            actual,
        } => {
            serde_json::json!({"variant":"satellite_record_sequence_mismatch", "record":record, "epoch_index":epoch_index.to_string(), "expected":expected, "actual":actual})
        }
        E::BodyRecordInterleavingMismatch {
            epoch_index,
            expected,
            actual,
        } => {
            serde_json::json!({"variant":"body_record_interleaving_mismatch", "epoch_index":epoch_index.to_string(), "expected":expected, "actual":actual})
        }
        E::NonFiniteHeaderCadence => serde_json::json!({"variant":"non_finite_header_cadence"}),
        E::NonPositiveHeaderCadence { actual_s } => {
            serde_json::json!({"variant":"non_positive_header_cadence", "actual_s":actual_s.to_string()})
        }
        E::UnsupportedHeaderCadence { actual_s } => {
            serde_json::json!({"variant":"unsupported_header_cadence", "actual_s":actual_s.to_string()})
        }
        E::CadenceMismatch {
            requested_s,
            header_s,
        } => {
            serde_json::json!({"variant":"cadence_mismatch", "requested_s":requested_s.to_string(), "header_s":header_s.to_string()})
        }
        E::DeclaredEpochCountMismatch { declared, parsed } => {
            serde_json::json!({"variant":"declared_epoch_count_mismatch", "declared":declared.to_string(), "parsed":parsed.to_string()})
        }
        E::MissingDeclaredStart => serde_json::json!({"variant":"missing_declared_start"}),
        E::DeclaredStartMismatch {
            requested_j2000_s,
            declared_j2000_s,
            requested_tick,
            declared_tick,
        } => {
            serde_json::json!({"variant":"declared_start_mismatch", "requested_j2000_s":requested_j2000_s.to_string(), "declared_j2000_s":declared_j2000_s.to_string(), "requested_tick":requested_tick.to_string(), "declared_tick":declared_tick.map(|value| value.to_string())})
        }
        E::RequestBeforeGpsEpoch => serde_json::json!({"variant":"request_before_gps_epoch"}),
        E::NonFiniteHeaderStartMetadata { field } => {
            serde_json::json!({"variant":"non_finite_header_start_metadata", "field":field})
        }
        E::InvalidHeaderStartMetadata { field, actual } => {
            serde_json::json!({"variant":"invalid_header_start_metadata", "field":field, "actual":actual.to_string()})
        }
        E::HeaderStartMetadataMismatch {
            field,
            requested,
            actual,
        } => {
            serde_json::json!({"variant":"header_start_metadata_mismatch", "field":field, "requested":requested.to_string(), "actual":actual.to_string()})
        }
        E::EmptyEpochGrid => serde_json::json!({"variant":"empty_epoch_grid"}),
        E::FirstEpochMismatch {
            requested_j2000_s,
            actual_j2000_s,
        } => {
            serde_json::json!({"variant":"first_epoch_mismatch", "requested_j2000_s":requested_j2000_s.to_string(), "actual_j2000_s":actual_j2000_s.to_string()})
        }
        E::IrregularEpochGrid {
            epoch_index,
            requested_s,
            actual_s,
        } => {
            serde_json::json!({"variant":"irregular_epoch_grid", "epoch_index":epoch_index.to_string(), "requested_s":requested_s.to_string(), "actual_s":actual_s.to_string()})
        }
        E::SpanNotMultipleOfCadence { span_s, cadence_s } => {
            serde_json::json!({"variant":"span_not_multiple_of_cadence", "span_s":span_s.to_string(), "cadence_s":cadence_s.to_string()})
        }
        E::SpanMismatch {
            parsed,
            half_open,
            inclusive,
        } => {
            serde_json::json!({"variant":"span_mismatch", "parsed":parsed.to_string(), "half_open":half_open.to_string(), "inclusive":inclusive.to_string()})
        }
        E::FormatVersionMismatch { requested, actual } => {
            serde_json::json!({"variant":"format_version_mismatch", "requested":requested, "actual":actual})
        }
        other => {
            serde_json::json!({"variant":"other", "diagnostic":other.to_string(), "debug":format!("{other:?}")})
        }
    }
}

fn map_exact_sp3_error(fn_name: &str, error: ExactSp3ValidationError) -> SidereonStatus {
    let status = if matches!(error, ExactSp3ValidationError::Parse(_)) {
        SidereonStatus::Sp3Parse
    } else {
        SidereonStatus::InvalidArgument
    };
    let message = match &error {
        ExactSp3ValidationError::DeclaredStartMismatch {
            requested_tick,
            declared_tick,
            ..
        } => format!(
            "{fn_name}: {error}; requested_tick={requested_tick}; declared_tick={}",
            (*declared_tick)
                .map(|tick| tick.to_string())
                .unwrap_or_else(|| "unplaced".to_string())
        ),
        _ => format!("{fn_name}: {error}"),
    };
    let (field, field_name, reason, reason_name, value) = exact_sp3_error_classification(&error);
    let (requested_tick, declared_tick, requested_j2000_s, declared_j2000_s) = match &error {
        ExactSp3ValidationError::DeclaredStartMismatch {
            requested_j2000_s,
            declared_j2000_s,
            requested_tick,
            declared_tick,
        } => (
            Some(*requested_tick),
            *declared_tick,
            Some(*requested_j2000_s),
            Some(*declared_j2000_s),
        ),
        _ => (None, None, None, None),
    };
    let details = exact_sp3_error_details(&error);
    let diagnostic = error.to_string();
    record_sp3_typed_error(Sp3TypedErrorDetails {
        kind: SidereonSp3ErrorKind::ExactValidation,
        field,
        reason,
        field_name,
        reason_name,
        value,
        requested_tick,
        declared_tick,
        requested_j2000_s,
        declared_j2000_s,
        details: Some(details),
        diagnostic: Some(&diagnostic),
    });
    set_last_error(message);
    status
}

/// Create a validated source-independent exact-SP3 request.
///
/// `issue` may be NULL for midnight or point to an `HHMM` token. `span` and
/// `sample` must be non-null IGS period tokens. `expected_agency` may be NULL;
/// when present it must be the one-to-four-character upper-case agency field
/// required from SP3 header line 1. This constructor does not constrain an SP3
/// revision; use `sidereon_sp3_exact_request_from_identity` to inherit both the
/// catalog identity's format revision and official producing agency.
///
/// Safety: each non-null text pointer must reference a null-terminated UTF-8
/// string and `out_request` must reference writable handle storage. On success
/// the caller owns the returned request.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_exact_request_new(
    year: i32,
    month: u8,
    day: u8,
    issue: *const c_char,
    span: *const c_char,
    sample: *const c_char,
    expected_agency: *const c_char,
    out_request: *mut *mut SidereonExactSp3Request,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_exact_request_new";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_request = c_try!(require_out(out_request, FN_NAME, "out_request"));
        *out_request = ptr::null_mut();
        let date = c_try!(ProductDate::new(year, month, day).map_err(|error| {
            set_last_error(format!("{FN_NAME}: {error}"));
            SidereonStatus::InvalidArgument
        }));
        let issue = if issue.is_null() {
            None
        } else {
            Some(c_try!(parse_bounded_c_string(FN_NAME, "issue", issue, 16)))
        };
        let span = c_try!(parse_bounded_c_string(FN_NAME, "span", span, 16));
        let sample = c_try!(parse_bounded_c_string(FN_NAME, "sample", sample, 16));
        let mut request = c_try!(
            CoreExactSp3Request::new(date, issue.as_deref(), &span, &sample)
                .map_err(|error| map_exact_sp3_error(FN_NAME, error))
        );
        if !expected_agency.is_null() {
            let agency = c_try!(parse_bounded_c_string(
                FN_NAME,
                "expected_agency",
                expected_agency,
                8,
            ));
            request = c_try!(request
                .with_expected_agency(&agency)
                .map_err(|error| map_exact_sp3_error(FN_NAME, error)));
        }
        write_boxed_handle(out_request, SidereonExactSp3Request { inner: request });
        SidereonStatus::Ok
    })
}

/// Create an exact-SP3 request from a complete catalog identity.
///
/// The core validates every identity field, requires the SP3 family, and binds
/// the request to the identity's date, issue, span, cadence, optional format
/// revision, and official producing-agency code.
///
/// Safety: `identity` must reference a live `SidereonProductIdentity` and
/// `out_request` must reference writable handle storage. On success the caller
/// owns the returned request.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_exact_request_from_identity(
    identity: *const SidereonProductIdentity,
    out_request: *mut *mut SidereonExactSp3Request,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_exact_request_from_identity";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_request = c_try!(require_out(out_request, FN_NAME, "out_request"));
        *out_request = ptr::null_mut();
        let identity = c_try!(require_ref(identity, FN_NAME, "identity")
            .and_then(|identity| data_distribution::identity_from_c(FN_NAME, identity)));
        let request = c_try!(CoreExactSp3Request::from_identity(&identity)
            .map_err(|error| map_exact_sp3_error(FN_NAME, error)));
        write_boxed_handle(out_request, SidereonExactSp3Request { inner: request });
        SidereonStatus::Ok
    })
}

/// Release an exact-SP3 request handle. NULL is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_exact_request_free(request: *mut SidereonExactSp3Request) {
    free_boxed(request);
}

/// Parse and validate bytes as one exact SP3 request.
///
/// The request supplies the trusted cadence and span. The core requires a
/// regular grid, matching header cadence/count/start metadata and identity,
/// mandatory SP3 structure, and either the half-open or inclusive boundary
/// representation. Any content-integrity failure is terminal and no handle is
/// returned.
///
/// Safety: `data` must reference `len` readable bytes; `request` must be a live
/// exact-request handle; both output pointers must reference writable storage.
/// On success the caller owns `*out_sp3`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_load_exact(
    data: *const u8,
    len: usize,
    request: *const SidereonExactSp3Request,
    out_sp3: *mut *mut SidereonSp3,
    out_coverage: *mut SidereonExactSp3Coverage,
) -> SidereonStatus {
    sidereon_sp3_load_exact_with_gap_threshold_factor(
        data,
        len,
        request,
        0.0,
        out_sp3,
        out_coverage,
    )
}

/// Parse and validate bytes as one exact SP3 request with an explicit
/// coverage-gap threshold factor. When gap_threshold_factor is <= 0.0, the core
/// default of 1.5 is used.
///
/// Safety: `data` must reference `len` readable bytes; `request` must be a live
/// exact-request handle; both output pointers must reference writable storage.
/// On success the caller owns `*out_sp3`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_load_exact_with_gap_threshold_factor(
    data: *const u8,
    len: usize,
    request: *const SidereonExactSp3Request,
    gap_threshold_factor: f64,
    out_sp3: *mut *mut SidereonSp3,
    out_coverage: *mut SidereonExactSp3Coverage,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_load_exact_with_gap_threshold_factor";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_sp3 = c_try!(require_out(out_sp3, FN_NAME, "out_sp3"));
        *out_sp3 = ptr::null_mut();
        let out_coverage = c_try!(require_out(out_coverage, FN_NAME, "out_coverage"));
        *out_coverage = SidereonExactSp3Coverage::HalfOpen;
        let request = c_try!(require_ref(request, FN_NAME, "request"));
        let bytes = c_try!(require_slice(data, len, FN_NAME, "data"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let (inner, coverage) = c_try!(core_parse_exact_sp3(bytes, &request.inner)
            .map_err(|error| map_exact_sp3_error(FN_NAME, error)));
        let inner = inner.with_interpolation_options(options);
        write_boxed_handle(out_sp3, SidereonSp3 { inner });
        *out_coverage = exact_sp3_coverage_to_c(coverage);
        SidereonStatus::Ok
    })
}

/// Validate an already parsed SP3 product against an exact request.
///
/// This applies the same integrity gate as `sidereon_sp3_load_exact` without
/// reparsing the original byte stream.
///
/// Safety: `sp3` and `request` must be live handles and `out_coverage` must
/// reference writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_validate_exact(
    sp3: *const SidereonSp3,
    request: *const SidereonExactSp3Request,
    out_coverage: *mut SidereonExactSp3Coverage,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_validate_exact";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_coverage = c_try!(require_out(out_coverage, FN_NAME, "out_coverage"));
        *out_coverage = SidereonExactSp3Coverage::HalfOpen;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let request = c_try!(require_ref(request, FN_NAME, "request"));
        let coverage = c_try!(core_validate_exact_sp3(&sp3.inner, &request.inner)
            .map_err(|error| map_exact_sp3_error(FN_NAME, error)));
        *out_coverage = exact_sp3_coverage_to_c(coverage);
        SidereonStatus::Ok
    })
}

/// Parse an SP3-c or SP3-d byte buffer into a precise-ephemeris product. On
/// success writes a newly owned handle to *out_sp3. Release it with
/// sidereon_sp3_free.
///
/// Safety: data must point to len readable bytes; out_sp3 must point to storage
/// for a SidereonSp3*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_load(
    data: *const u8,
    len: usize,
    out_sp3: *mut *mut SidereonSp3,
) -> SidereonStatus {
    sidereon_sp3_load_with_gap_threshold_factor(data, len, 0.0, out_sp3)
}

/// Parse an SP3-c or SP3-d byte buffer into a precise-ephemeris product with an
/// explicit coverage-gap threshold factor. When gap_threshold_factor is <= 0.0,
/// the core default of 1.5 is used. On success writes a newly owned handle to
/// *out_sp3. Release it with sidereon_sp3_free.
///
/// Safety: data must point to len readable bytes; out_sp3 must point to storage
/// for a SidereonSp3*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_load_with_gap_threshold_factor(
    data: *const u8,
    len: usize,
    gap_threshold_factor: f64,
    out_sp3: *mut *mut SidereonSp3,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_load_with_gap_threshold_factor";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_sp3 = c_try!(require_out(out_sp3, FN_NAME, "out_sp3"));
        *out_sp3 = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, FN_NAME, "data"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let inner = c_try!(guard(FN_NAME, SidereonStatus::Sp3Parse, || {
            sidereon::load_sp3(bytes)
        }))
        .with_interpolation_options(options);
        write_boxed_handle(out_sp3, SidereonSp3 { inner });
        SidereonStatus::Ok
    })
}

/// Write the SP3 interpolation gap threshold factor carried by this product to
/// *out_gap_threshold_factor.
///
/// Safety: sp3 must be a live SP3 handle; out_gap_threshold_factor must point
/// to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_gap_threshold_factor(
    sp3: *const SidereonSp3,
    out_gap_threshold_factor: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_gap_threshold_factor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_gap_threshold_factor = c_try!(require_out(
            out_gap_threshold_factor,
            FN_NAME,
            "out_gap_threshold_factor"
        ));
        *out_gap_threshold_factor = 0.0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        *out_gap_threshold_factor = sp3.inner.interpolation_options().gap_threshold_factor();
        SidereonStatus::Ok
    })
}

/// Write the number of epochs in the product to *out_count.
///
/// Safety: sp3 must be a live SP3 handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_epoch_count(
    sp3: *const SidereonSp3,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_epoch_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_sp3_epoch_count",
            "out_count"
        ));
        *out_count = 0;
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_epoch_count", "sp3"));
        *out_count = sp3.inner.epoch_count();
        SidereonStatus::Ok
    })
}

/// Attest that the product is physically continuous, writing summary counts.
///
/// Two checks run with different jobs: a physical earth-fixed speed gate whose
/// bound is a true upper bound for the orbit class, so it cannot false-positive
/// and catches gross corruption; and a hold-out interpolation residual, which
/// supplies the sensitivity a speed gate structurally cannot (adjacent GNSS MEO
/// epochs are hundreds of kilometres apart, so a metre-scale splice moves the
/// implied speed by a fraction of a percent).
///
/// `orbit_class` is 0 for MEO GNSS, 1 for geosynchronous, 2 for LEO, or -1 to
/// disable the speed gate. A negative `residual_tolerance_m` disables the
/// residual check. `out_defects` receives the number of violations found;
/// `out_residuals_checked` and `out_residuals_skipped` let a caller tell
/// "checked and clean" from "not checked". Reports rather than refuses.
///
/// Safety: `sp3` must be a live handle and each out pointer must reference
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_check_continuity(
    sp3: *const SidereonSp3,
    orbit_class: i32,
    residual_tolerance_m: f64,
    out_defects: *mut usize,
    out_residuals_checked: *mut usize,
    out_residuals_skipped: *mut usize,
) -> SidereonStatus {
    sidereon_sp3_check_continuity_with_gap_threshold_factor(
        sp3,
        orbit_class,
        residual_tolerance_m,
        0.0,
        out_defects,
        out_residuals_checked,
        out_residuals_skipped,
    )
}

/// Run the product-wide continuity pre-check over every satellite series in an
/// SP3 product with an explicit coverage-gap threshold factor. When
/// `gap_threshold_factor` is <= 0.0, the core default of 1.5 is used.
///
/// `orbit_class` is 0 for MEO GNSS, 1 for geosynchronous, 2 for LEO, or -1 to
/// disable the speed gate. A negative `residual_tolerance_m` disables the
/// residual check. `out_defects` receives the number of violations found;
/// `out_residuals_checked` and `out_residuals_skipped` let a caller tell
/// "checked and clean" from "not checked". Reports rather than refuses.
///
/// Safety: `sp3` must be a live handle and each out pointer must reference
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_check_continuity_with_gap_threshold_factor(
    sp3: *const SidereonSp3,
    orbit_class: i32,
    residual_tolerance_m: f64,
    gap_threshold_factor: f64,
    out_defects: *mut usize,
    out_residuals_checked: *mut usize,
    out_residuals_skipped: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_check_continuity_with_gap_threshold_factor";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_defects = c_try!(require_out(out_defects, FN_NAME, "out_defects"));
        *out_defects = 0;
        let out_residuals_checked = c_try!(require_out(
            out_residuals_checked,
            FN_NAME,
            "out_residuals_checked"
        ));
        *out_residuals_checked = 0;
        let out_residuals_skipped = c_try!(require_out(
            out_residuals_skipped,
            FN_NAME,
            "out_residuals_skipped"
        ));
        *out_residuals_skipped = 0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));

        let options = c_try!(continuity_options_from_c(
            FN_NAME,
            orbit_class,
            residual_tolerance_m,
            gap_threshold_factor,
        ));
        let report = c_try!(
            check_continuity(&sp3.inner.precise_ephemeris_samples(), &options)
                .map_err(|error| map_continuity_options_error(FN_NAME, error))
        );

        *out_defects = report.defects.len();
        *out_residuals_checked = report.residuals_checked;
        *out_residuals_skipped = report.residuals_skipped;
        SidereonStatus::Ok
    })
}

/// Write the time reach of the SP3 position interpolator before and after a
/// query, in seconds.
///
/// The core derives both values from this product's declared epoch interval and
/// interpolation-node count. The caller never supplies a stencil duration.
///
/// Safety: `sp3` must be a live handle and both output pointers must reference
/// writable doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_stencil_extent(
    sp3: *const SidereonSp3,
    out_before_s: *mut f64,
    out_after_s: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_stencil_extent";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_before_s = c_try!(require_out(out_before_s, FN_NAME, "out_before_s"));
        *out_before_s = 0.0;
        let out_after_s = c_try!(require_out(out_after_s, FN_NAME, "out_after_s"));
        *out_after_s = 0.0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let stencil = c_try!(StencilExtent::for_sp3(&sp3.inner)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        *out_before_s = stencil.before_s();
        *out_after_s = stencil.after_s();
        SidereonStatus::Ok
    })
}

/// Decide whether product-wide continuity findings can influence an inclusive
/// evaluation window through this product's derived interpolation stencil.
///
/// The JSON object contains `decision` (`"accept"` or `"refuse"`), `accepted`,
/// the influencing defect and splice arrays (`influencing_defects`,
/// `influencing_splices`), and the complete defect and splice arrays
/// (`all_defects`, `all_splices`). Standalone checks always have empty splice
/// arrays. `orbit_class` and `residual_tolerance_m` use the same selectors as
/// `sidereon_sp3_check_continuity`.
///
/// A defect object has `kind` (`"duplicate_epoch"`, `"single_sample_series"`,
/// `"unusable_sample"`, `"speed_bound"` or `"hold_out_residual"`), `satellite`, the summary
/// `from_j2000_s`, `to_j2000_s`, `magnitude` and `bound` (null where the kind
/// has none), and every field of its kind under the engine's name:
/// `epoch_j2000_s` and `occurrences`; `sample_index`, `epoch_j2000_s` and
/// `reason` for unusable samples; `interval_s`, `displacement_m`,
/// `implied_speed_m_s` and `bound_m_s`; or `epoch_j2000_s`,
/// `preceding_j2000_s`, `residual_m`, `tolerance_m` and `node_epochs_j2000_s`.
/// A splice object has `defect`, `from_sources`, `to_sources`, `sources`,
/// `crosses_contributors` and `cells`: each cell has `epoch_j2000_s`, `role`
/// (`"held_out"`, `"interpolation_node"`, `"pair_end"` or `"repeated_epoch"`)
/// and `selection`, null or an object whose `kind` is `"single_source"`
/// (with `source`), `"precedence"` (with `source` and `members`) or
/// `"combined"` (with `rule`, `"mean"`, `"median"` or `"precedence"`, and
/// `members`).
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `sp3` must be a live handle; `out` must reference `out_len` writable
/// bytes, or be NULL when `out_len` is zero; both count pointers must reference
/// writable size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_continuity_verdict_json(
    sp3: *const SidereonSp3,
    orbit_class: i32,
    residual_tolerance_m: f64,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
        sp3,
        orbit_class,
        residual_tolerance_m,
        0.0,
        from_j2000_s,
        through_j2000_s,
        out,
        out_len,
        out_written,
        out_required,
    )
}

/// Decide whether product-wide continuity findings can influence an inclusive
/// evaluation window through this product's derived interpolation stencil, with
/// an explicit coverage-gap threshold factor. When `gap_threshold_factor` is <= 0.0,
/// the core default of 1.5 is used.
///
/// The JSON object contains `decision` (`"accept"` or `"refuse"`), `accepted`,
/// the influencing defect and splice arrays (`influencing_defects`,
/// `influencing_splices`), and the complete defect and splice arrays
/// (`all_defects`, `all_splices`). Standalone checks always have empty splice
/// arrays. `orbit_class` and `residual_tolerance_m` use the same selectors as
/// `sidereon_sp3_check_continuity`.
///
/// A defect object has `kind` (`"duplicate_epoch"`, `"single_sample_series"`,
/// `"speed_bound"` or `"hold_out_residual"`), `satellite`, the summary
/// `from_j2000_s`, `to_j2000_s`, `magnitude` and `bound` (null where the kind
/// has none), and every field of its kind under the engine's name:
/// `epoch_j2000_s` and `occurrences`; `interval_s`, `displacement_m`,
/// `implied_speed_m_s` and `bound_m_s`; or `epoch_j2000_s`,
/// `preceding_j2000_s`, `residual_m`, `tolerance_m` and `node_epochs_j2000_s`.
/// A splice object has `defect`, `from_sources`, `to_sources`, `sources`,
/// `crosses_contributors` and `cells`: each cell has `epoch_j2000_s`, `role`
/// (`"held_out"`, `"interpolation_node"`, `"pair_end"` or `"repeated_epoch"`)
/// and `selection`, null or an object whose `kind` is `"single_source"`
/// (with `source`), `"precedence"` (with `source` and `members`) or
/// `"combined"` (with `rule`, `"mean"`, `"median"` or `"precedence"`, and
/// `members`).
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `sp3` must be a live handle; `out` must reference `out_len` writable
/// bytes, or be NULL when `out_len` is zero; both count pointers must reference
/// writable size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
    sp3: *const SidereonSp3,
    orbit_class: i32,
    residual_tolerance_m: f64,
    gap_threshold_factor: f64,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let options = c_try!(continuity_options_from_c(
            FN_NAME,
            orbit_class,
            residual_tolerance_m,
            gap_threshold_factor,
        ));
        let window = c_try!(EpochWindow::new(from_j2000_s, through_j2000_s)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        let stencil = c_try!(StencilExtent::for_sp3(&sp3.inner)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        let report = c_try!(
            check_continuity(&sp3.inner.precise_ephemeris_samples(), &options)
                .map_err(|error| map_continuity_options_error(FN_NAME, error))
        );
        let value = window_continuity_verdict_json(report.verdict_for_window(window, stencil));
        c_try!(copy_json_value_to_c(
            FN_NAME,
            &value,
            out,
            out_len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Check the product's continuity under `options` (a
/// SidereonSp3ContinuityOptions; see
/// sidereon_sp3_continuity_options_for_orbit_class) and copy the whole report
/// as JSON: `attested`, `defects` (each as the verdict JSON describes it, ordered
/// by satellite then epoch), `pairs_checked`, `residuals_checked` and
/// `residuals_skipped`. Reports rather than refuses.
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `sp3` must be a live handle; `options` must point to a
/// SidereonSp3ContinuityOptions; `out` must reference `out_len` writable bytes,
/// or be NULL when `out_len` is zero; both count pointers must reference
/// writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_continuity_report_json(
    sp3: *const SidereonSp3,
    options: *const SidereonSp3ContinuityOptions,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_continuity_report_json";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let options = c_try!(require_ref(options, FN_NAME, "options"));
        let options = c_try!(continuity_options_struct_from_c(FN_NAME, options));
        let report = c_try!(
            check_continuity(&sp3.inner.precise_ephemeris_samples(), &options)
                .map_err(|error| map_continuity_options_error(FN_NAME, error))
        );
        c_try!(copy_json_value_to_c(
            FN_NAME,
            &continuity_report_json(&report),
            out,
            out_len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// As sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor, with the
/// checks stated by a SidereonSp3ContinuityOptions record, which can also set
/// an explicit speed bound.
///
/// Safety: `sp3` must be a live handle; `options` must point to a
/// SidereonSp3ContinuityOptions; `out` must reference `out_len` writable bytes,
/// or be NULL when `out_len` is zero; both count pointers must reference
/// writable size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_continuity_verdict_json_with_options(
    sp3: *const SidereonSp3,
    options: *const SidereonSp3ContinuityOptions,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_continuity_verdict_json_with_options";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let options = c_try!(require_ref(options, FN_NAME, "options"));
        let options = c_try!(continuity_options_struct_from_c(FN_NAME, options));
        let window = c_try!(EpochWindow::new(from_j2000_s, through_j2000_s)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        let stencil = c_try!(StencilExtent::for_sp3(&sp3.inner)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        let report = c_try!(
            check_continuity(&sp3.inner.precise_ephemeris_samples(), &options)
                .map_err(|error| map_continuity_options_error(FN_NAME, error))
        );
        let value = window_continuity_verdict_json(report.verdict_for_window(window, stencil));
        c_try!(copy_json_value_to_c(
            FN_NAME,
            &value,
            out,
            out_len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Write the epoch count declared on SP3 header line 1.
///
/// This may differ from `sidereon_sp3_epoch_count` for a truncated product
/// accepted by the deliberately permissive base parser. Exact validation
/// requires the declared and parsed counts to agree.
///
/// Safety: `sp3` must be a live handle and `out_count` must reference writable
/// uint64_t storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_declared_epoch_count(
    sp3: *const SidereonSp3,
    out_count: *mut u64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_declared_epoch_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        *out_count = sp3.inner.declared_epoch_count();
        SidereonStatus::Ok
    })
}

/// Read the start epoch declared on SP3 header line 1 as J2000 seconds in the
/// product time scale.
///
/// The permissive parser represents a malformed line-1 civil epoch as absent.
/// Exact validation requires it to be present and equal both the request and
/// first parsed epoch. `out_present` is exactly 0 or 1; `out_seconds` is zero
/// when the field is absent.
///
/// Safety: `sp3` must be a live handle and both output pointers must reference
/// disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_declared_start_j2000_seconds(
    sp3: *const SidereonSp3,
    out_present: *mut u8,
    out_seconds: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_declared_start_j2000_seconds";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_present.is_null() && !out_seconds.is_null() {
            let outputs = [
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_present,
                        1,
                        "out_present"
                    )),
                    "out_present",
                )),
                Some((
                    c_try!(super::checked_output_range(
                        FN_NAME,
                        out_seconds,
                        1,
                        "out_seconds"
                    )),
                    "out_seconds",
                )),
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                FN_NAME, &outputs
            ));
        }
        let out_present = c_try!(require_out(out_present, FN_NAME, "out_present"));
        *out_present = 0;
        let out_seconds = c_try!(require_out(out_seconds, FN_NAME, "out_seconds"));
        *out_seconds = 0.0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let declared = sp3.inner.declared_start_j2000_s();
        *out_present = u8::from(declared.is_some());
        *out_seconds = declared.unwrap_or(0.0);
        SidereonStatus::Ok
    })
}

/// Copy satellite tokens present in the product. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_satellites(
    sp3: *const SidereonSp3,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_satellites", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_sp3_satellites",
            out_written,
            out_required
        ));
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_satellites", "sp3"));
        let values: Vec<SidereonSatelliteToken> = sp3
            .inner
            .satellites()
            .iter()
            .copied()
            .map(satellite_token)
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_sp3_satellites",
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

/// Copy parsed SP3 epoch nodes as seconds since J2000, in the product time
/// scale. Uses the variable-length output contract documented at the top of the
/// header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_epochs_j2000_seconds(
    sp3: *const SidereonSp3,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_epochs_j2000_seconds",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_epochs_j2000_seconds",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_epochs_j2000_seconds", "sp3"));
            let epochs = sp3.inner.epochs_j2000_seconds();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_epochs_j2000_seconds",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Read the product-wide observed/predicted boundary derived from SP3 record
/// flags. Exact per-satellite flags remain available through sidereon_sp3_state.
///
/// Safety: sp3 must be a live handle and out_summary must point to writable
/// SidereonSp3PredictionSummary storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_prediction_summary(
    sp3: *const SidereonSp3,
    out_summary: *mut SidereonSp3PredictionSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_prediction_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_sp3_prediction_summary",
                "out_summary"
            ));
            *out_summary = SidereonSp3PredictionSummary {
                epoch_count: 0,
                observed_through_present: false,
                observed_through_j2000_seconds: 0.0,
            };
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_prediction_summary", "sp3"));
            let summary = sp3.inner.prediction_summary();
            let observed_through = summary
                .observed_through
                .as_ref()
                .and_then(instant_to_j2000_seconds);
            *out_summary = SidereonSp3PredictionSummary {
                epoch_count: summary.epochs.len(),
                observed_through_present: observed_through.is_some(),
                observed_through_j2000_seconds: observed_through.unwrap_or(0.0),
            };
            SidereonStatus::Ok
        },
    )
}

/// Read one per-epoch observed/predicted aggregate by parsed epoch index.
///
/// Safety: sp3 must be a live handle and out_prediction must point to writable
/// SidereonSp3EpochPrediction storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_epoch_prediction(
    sp3: *const SidereonSp3,
    epoch_index: usize,
    out_prediction: *mut SidereonSp3EpochPrediction,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_epoch_prediction",
        SidereonStatus::Panic,
        || {
            let out_prediction = c_try!(require_out(
                out_prediction,
                "sidereon_sp3_epoch_prediction",
                "out_prediction"
            ));
            *out_prediction = SidereonSp3EpochPrediction {
                epoch_j2000_seconds: 0.0,
                observed: false,
                orbit_predicted_satellite_count: 0,
                clock_predicted_satellite_count: 0,
            };
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_epoch_prediction", "sp3"));
            let summary = sp3.inner.prediction_summary();
            let Some(epoch) = summary.epochs.get(epoch_index) else {
                set_last_error(format!(
                    "sidereon_sp3_epoch_prediction: epoch index {epoch_index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_prediction = SidereonSp3EpochPrediction {
                epoch_j2000_seconds: instant_to_j2000_seconds(&epoch.epoch).unwrap_or(f64::NAN),
                observed: epoch.is_observed(),
                orbit_predicted_satellite_count: epoch.orbit_predicted_satellites.len(),
                clock_predicted_satellite_count: epoch.clock_predicted_satellites.len(),
            };
            SidereonStatus::Ok
        },
    )
}

/// Copy the exact parsed state of satellite sat_id at epoch_index into
/// *out_state.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated
/// satellite token whose terminator appears within 16 bytes; out_state must
/// point to a SidereonSp3State.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_state(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    epoch_index: usize,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_state", SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, "sidereon_sp3_state", "out_state"));
        *out_state = empty_sp3_state();
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_state", "sp3"));
        let sat = c_try!(parse_satellite_token("sidereon_sp3_state", sat_id));
        let state = c_try!(guard_core(
            || sp3.inner.state(sat, epoch_index),
            |err| map_sp3_argument_error("sidereon_sp3_state", err),
        ));
        *out_state = sp3_state_to_c(state);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_state_at_epoch_query(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    query: *const SidereonExactEpochQuery,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_state_at_epoch_query";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN_NAME, "out_state"));
        *out_state = empty_sp3_state();
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let query = c_try!(require_ref(query, FN_NAME, "query"));
        let state = c_try!(guard_core(
            || sp3.inner.position_at_epoch_query(sat, &query.inner),
            |error| { map_sp3_interpolation_error(FN_NAME, query.inner.j2000_seconds(), error) },
        ));
        *out_state = sp3_state_to_c(state);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_source_state_at_epoch_queries(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out: *mut SidereonEphemerisSourceState,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_source_state_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        *out = SidereonEphemerisSourceState::default();
        record_degrade_reason(None);
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let state = c_try!(guard_core(
            || {
                sidereon_core::positioning::EphemerisSource::try_position_clock_group_delay_selected_at_epoch_query(
                &sp3.inner, satellite, &state_epoch.inner, &selection_epoch.inner,
            )
            },
            |error| crate::precise::precise_source_error_to_status(FN_NAME, error),
        ));
        if let Some(state) = state {
            record_degrade_reason(state.degraded);
            *out = SidereonEphemerisSourceState {
                has_state: true,
                position_ecef_m: state.value.0,
                clock_s: state.value.1,
                has_group_delay: state.value.2.is_some(),
                group_delay_s: state.value.2.unwrap_or_default(),
                degraded: state.degraded.is_some(),
            };
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_source_transmit_epoch_clock_at_epoch_queries(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    transmit_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_has_clock: *mut bool,
    out_clock_s: *mut f64,
    out_degraded: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_source_transmit_epoch_clock_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_has_clock.is_null() && !out_clock_s.is_null() && !out_degraded.is_null() {
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
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                FN_NAME, &outputs
            ));
        }
        let out_has_clock = c_try!(require_out(out_has_clock, FN_NAME, "out_has_clock"));
        let out_clock_s = c_try!(require_out(out_clock_s, FN_NAME, "out_clock_s"));
        let out_degraded = c_try!(require_out(out_degraded, FN_NAME, "out_degraded"));
        *out_has_clock = false;
        *out_clock_s = 0.0;
        *out_degraded = false;
        record_degrade_reason(None);
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let transmit_epoch = c_try!(require_ref(transmit_epoch, FN_NAME, "transmit_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let clock = c_try!(guard_core(
            || sidereon_core::positioning::EphemerisSource::try_transmit_epoch_clock_at_epoch_query(
                &sp3.inner,
                satellite,
                &transmit_epoch.inner,
                &selection_epoch.inner,
            ),
            |error| crate::precise::precise_source_error_to_status(FN_NAME, error),
        ));
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
pub unsafe extern "C" fn sidereon_sp3_source_clock_relativity_at_epoch_query(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    epoch: *const SidereonExactEpochQuery,
    position_ecef_m: *const f64,
    out_kind: *mut SidereonClockRelativityKind,
    out_term_s: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_source_clock_relativity_at_epoch_query";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_kind.is_null() && !out_term_s.is_null() {
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
            ];
            c_try!(super::reject_overlapping_optional_outputs(
                FN_NAME, &outputs
            ));
        }
        let out_kind = c_try!(require_out(out_kind, FN_NAME, "out_kind"));
        let out_term_s = c_try!(require_out(out_term_s, FN_NAME, "out_term_s"));
        *out_kind = SidereonClockRelativityKind::NotApplicable;
        *out_term_s = 0.0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let epoch = c_try!(require_ref(epoch, FN_NAME, "epoch"));
        let position = c_try!(require_slice(
            position_ecef_m,
            3,
            FN_NAME,
            "position_ecef_m"
        ));
        let position = [position[0], position[1], position[2]];
        match sidereon_core::positioning::EphemerisSource::clock_relativity_for_state_at_epoch_query(
            &sp3.inner,
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
pub unsafe extern "C" fn sidereon_sp3_source_ephemeris_variance_at_epoch_queries(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_variance_m2: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_source_ephemeris_variance_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_variance_m2 = c_try!(require_out(out_variance_m2, FN_NAME, "out_variance_m2"));
        *out_variance_m2 = 0.0;
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        *out_variance_m2 =
            sidereon_core::positioning::EphemerisSource::ephemeris_variance_at_epoch_query(
                &sp3.inner,
                satellite,
                &state_epoch.inner,
                &selection_epoch.inner,
            );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_record_accuracy_codes(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    epoch_index: usize,
    out_accuracy: *mut SidereonSp3RawRecordAccuracy,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_record_accuracy_codes";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_accuracy = c_try!(require_out(out_accuracy, FN_NAME, "out_accuracy"));
        *out_accuracy = empty_sp3_raw_accuracy();
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let accuracy = c_try!(guard_core(
            || sp3.inner.record_accuracy_codes(sat, epoch_index),
            |error| map_sp3_argument_error(FN_NAME, error),
        ));
        *out_accuracy = raw_sp3_accuracy_to_c(accuracy);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_record_accuracy(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    epoch_index: usize,
    out_accuracy: *mut SidereonSp3RecordAccuracy,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_record_accuracy";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_accuracy = c_try!(require_out(out_accuracy, FN_NAME, "out_accuracy"));
        *out_accuracy = empty_sp3_record_accuracy();
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let accuracy = c_try!(guard_core(
            || sp3.inner.record_accuracy(sat, epoch_index),
            |error| map_sp3_argument_error(FN_NAME, error),
        ));
        *out_accuracy = record_sp3_accuracy_to_c(accuracy);
        SidereonStatus::Ok
    })
}

fn empty_sp3_accuracy_value() -> SidereonSp3AccuracyValue {
    SidereonSp3AccuracyValue {
        kind: SidereonSp3AccuracyValueKind::Unknown as u32,
        value: 0.0,
    }
}

fn empty_sp3_code_group() -> SidereonSp3AccuracyCodeGroup {
    SidereonSp3AccuracyCodeGroup {
        has_axis_exponents: [false; 3],
        axis_exponents: [0; 3],
        has_clock_exponent: false,
        clock_exponent: 0,
        has_position_velocity_base: false,
        position_velocity_base: 0.0,
        has_clock_rate_base: false,
        clock_rate_base: 0.0,
    }
}

fn empty_sp3_raw_accuracy() -> SidereonSp3RawRecordAccuracy {
    SidereonSp3RawRecordAccuracy {
        has_p: false,
        p: empty_sp3_code_group(),
        has_v: false,
        v: empty_sp3_code_group(),
    }
}

fn empty_sp3_position_clock_accuracy() -> SidereonSp3PositionClockAccuracy {
    SidereonSp3PositionClockAccuracy {
        position_sigma_m: [empty_sp3_accuracy_value(); 3],
        clock_sigma_m: empty_sp3_accuracy_value(),
        position_variance_m2: [empty_sp3_accuracy_value(); 3],
        clock_variance_m2: empty_sp3_accuracy_value(),
    }
}

fn empty_sp3_velocity_accuracy() -> SidereonSp3VelocityAccuracy {
    SidereonSp3VelocityAccuracy {
        velocity_sigma_m_s: [empty_sp3_accuracy_value(); 3],
        clock_rate_sigma_m_s: empty_sp3_accuracy_value(),
        velocity_variance_m2_s2: [empty_sp3_accuracy_value(); 3],
        clock_rate_variance_m2_s2: empty_sp3_accuracy_value(),
    }
}

fn empty_sp3_record_accuracy() -> SidereonSp3RecordAccuracy {
    SidereonSp3RecordAccuracy {
        has_p: false,
        p: empty_sp3_position_clock_accuracy(),
        has_v: false,
        v: empty_sp3_velocity_accuracy(),
    }
}

pub(crate) fn sp3_accuracy_value_to_c(
    value: sidereon_core::ephemeris::Sp3AccuracyValue,
) -> SidereonSp3AccuracyValue {
    use sidereon_core::ephemeris::Sp3AccuracyValue as CoreValue;
    match value {
        CoreValue::Known(value) => SidereonSp3AccuracyValue {
            kind: SidereonSp3AccuracyValueKind::Known as u32,
            value,
        },
        CoreValue::Unknown => empty_sp3_accuracy_value(),
        CoreValue::TooLarge => SidereonSp3AccuracyValue {
            kind: SidereonSp3AccuracyValueKind::TooLarge as u32,
            value: 0.0,
        },
        CoreValue::InvalidBase => SidereonSp3AccuracyValue {
            kind: SidereonSp3AccuracyValueKind::InvalidBase as u32,
            value: 0.0,
        },
        CoreValue::Overflow => SidereonSp3AccuracyValue {
            kind: SidereonSp3AccuracyValueKind::Overflow as u32,
            value: 0.0,
        },
        _ => SidereonSp3AccuracyValue {
            kind: SidereonSp3AccuracyValueKind::Other as u32,
            value: 0.0,
        },
    }
}

fn sp3_accuracy_code_group_to_c(
    group: sidereon_core::ephemeris::Sp3AccuracyCodeGroup,
) -> SidereonSp3AccuracyCodeGroup {
    let mut output = empty_sp3_code_group();
    for (index, exponent) in group.axis_exponents.into_iter().enumerate() {
        if let Some(exponent) = exponent {
            output.has_axis_exponents[index] = true;
            output.axis_exponents[index] = exponent;
        }
    }
    if let Some(exponent) = group.clock_exponent {
        output.has_clock_exponent = true;
        output.clock_exponent = exponent;
    }
    if let Some(base) = group.position_velocity_base {
        output.has_position_velocity_base = true;
        output.position_velocity_base = base;
    }
    if let Some(base) = group.clock_rate_base {
        output.has_clock_rate_base = true;
        output.clock_rate_base = base;
    }
    output
}

fn raw_sp3_accuracy_to_c(
    accuracy: sidereon_core::ephemeris::Sp3RawRecordAccuracy,
) -> SidereonSp3RawRecordAccuracy {
    let mut output = empty_sp3_raw_accuracy();
    if let Some(group) = accuracy.p {
        output.has_p = true;
        output.p = sp3_accuracy_code_group_to_c(group);
    }
    if let Some(group) = accuracy.v {
        output.has_v = true;
        output.v = sp3_accuracy_code_group_to_c(group);
    }
    output
}

fn position_clock_accuracy_to_c(
    accuracy: sidereon_core::ephemeris::Sp3PositionClockAccuracy,
) -> SidereonSp3PositionClockAccuracy {
    SidereonSp3PositionClockAccuracy {
        position_sigma_m: accuracy.position_sigma_m.map(sp3_accuracy_value_to_c),
        clock_sigma_m: sp3_accuracy_value_to_c(accuracy.clock_sigma_m),
        position_variance_m2: accuracy.position_variance_m2().map(sp3_accuracy_value_to_c),
        clock_variance_m2: sp3_accuracy_value_to_c(accuracy.clock_variance_m2()),
    }
}

fn velocity_accuracy_to_c(
    accuracy: sidereon_core::ephemeris::Sp3VelocityAccuracy,
) -> SidereonSp3VelocityAccuracy {
    SidereonSp3VelocityAccuracy {
        velocity_sigma_m_s: accuracy.velocity_sigma_m_s.map(sp3_accuracy_value_to_c),
        clock_rate_sigma_m_s: sp3_accuracy_value_to_c(accuracy.clock_rate_sigma_m_s),
        velocity_variance_m2_s2: accuracy
            .velocity_variance_m2_s2()
            .map(sp3_accuracy_value_to_c),
        clock_rate_variance_m2_s2: sp3_accuracy_value_to_c(accuracy.clock_rate_variance_m2_s2()),
    }
}

fn record_sp3_accuracy_to_c(
    accuracy: sidereon_core::ephemeris::Sp3RecordAccuracy,
) -> SidereonSp3RecordAccuracy {
    let mut output = empty_sp3_record_accuracy();
    if let Some(value) = accuracy.p {
        output.has_p = true;
        output.p = position_clock_accuracy_to_c(value);
    }
    if let Some(value) = accuracy.v {
        output.has_v = true;
        output.v = velocity_accuracy_to_c(value);
    }
    output
}

/// Interpolate a satellite at each query epoch. out_position_m receives
/// epoch_count rows of 3 ECEF meters, and out_clock_s receives epoch_count
/// clock offsets in seconds, using NaN when the engine reports no clock.
///
/// Safety: sp3 must be a live handle; sat_id must be a bounded null-terminated
/// token; j2000_seconds must point to epoch_count readable doubles; output
/// buffers must have room for epoch_count*3 and epoch_count doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_interpolate(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    j2000_seconds: *const f64,
    epoch_count: usize,
    out_position_m: *mut f64,
    position_len: usize,
    out_clock_s: *mut f64,
    clock_len: usize,
    out_written: *mut usize,
) -> SidereonStatus {
    sp3_operation_boundary("sidereon_sp3_interpolate", SidereonStatus::Panic, || {
        if out_written.is_null() {
            set_last_error("sidereon_sp3_interpolate: null out_written");
            return SidereonStatus::NullPointer;
        }
        // Validate and retain the live source handle, and parse/copy the
        // caller-owned token and epoch values before touching any output,
        // including out_written. Preserve first-input-error ordering and the
        // legacy zero count on input refusal.
        let inputs_result = (|| {
            let sp3 = require_ref(sp3, "sidereon_sp3_interpolate", "sp3")?;
            let sat = parse_satellite_token("sidereon_sp3_interpolate", sat_id)?;
            let queries = require_slice(
                j2000_seconds,
                epoch_count,
                "sidereon_sp3_interpolate",
                "j2000_seconds",
            )?
            .to_vec();
            Ok((sp3, sat, queries))
        })();
        let inputs = match inputs_result {
            Ok(value) => value,
            Err(status) => {
                out_written.write(0);
                return status;
            }
        };
        let (sp3, sat, queries) = inputs;
        out_written.write(0);
        if queries.is_empty() {
            set_last_error("sidereon_sp3_interpolate: j2000_seconds array is empty");
            return SidereonStatus::InvalidArgument;
        }
        c_try!(validate_element_count::<[f64; 3]>(
            "sidereon_sp3_interpolate",
            "epoch_count",
            queries.len(),
        ));
        c_try!(validate_element_count::<f64>(
            "sidereon_sp3_interpolate",
            "position_len",
            position_len,
        ));
        c_try!(validate_element_count::<f64>(
            "sidereon_sp3_interpolate",
            "clock_len",
            clock_len,
        ));
        let required_position_len = queries.len() * 3;
        if position_len < required_position_len {
            set_last_error(format!(
                "sidereon_sp3_interpolate: out_position_m needs room for {required_position_len} doubles"
            ));
            return SidereonStatus::InvalidArgument;
        }
        if clock_len < queries.len() {
            set_last_error(format!(
                "sidereon_sp3_interpolate: out_clock_s needs room for {} doubles",
                queries.len()
            ));
            return SidereonStatus::InvalidArgument;
        }
        if out_position_m.is_null() {
            set_last_error("sidereon_sp3_interpolate: null out_position_m");
            return SidereonStatus::NullPointer;
        }
        if out_clock_s.is_null() {
            set_last_error("sidereon_sp3_interpolate: null out_clock_s");
            return SidereonStatus::NullPointer;
        }
        let written_range = c_try!(checked_output_range(
            "sidereon_sp3_interpolate",
            out_written,
            1,
            "out_written",
        ));
        let position_range = c_try!(checked_output_range(
            "sidereon_sp3_interpolate",
            out_position_m,
            required_position_len,
            "out_position_m",
        ));
        let clock_range = c_try!(checked_output_range(
            "sidereon_sp3_interpolate",
            out_clock_s,
            queries.len(),
            "out_clock_s",
        ));
        c_try!(reject_overlapping_outputs(
            "sidereon_sp3_interpolate",
            position_range,
            clock_range,
            "out_position_m",
            "out_clock_s",
        ));
        c_try!(reject_overlapping_outputs(
            "sidereon_sp3_interpolate",
            written_range,
            position_range,
            "out_written",
            "out_position_m",
        ));
        c_try!(reject_overlapping_outputs(
            "sidereon_sp3_interpolate",
            written_range,
            clock_range,
            "out_written",
            "out_clock_s",
        ));

        let mut positions = Vec::with_capacity(queries.len());
        let mut clocks = Vec::with_capacity(queries.len());
        for &query in &queries {
            let state = c_try!(guard_core(
                || sp3.inner.position_at_j2000_seconds(sat, query),
                |err| map_sp3_interpolation_error("sidereon_sp3_interpolate", query, err),
            ));
            positions.push(state.position.as_array());
            clocks.push(state.clock_s.unwrap_or(f64::NAN));
        }
        for (idx, position) in positions.iter().enumerate() {
            ptr::copy_nonoverlapping(position.as_ptr(), out_position_m.add(idx * 3), 3);
        }
        ptr::copy_nonoverlapping(clocks.as_ptr(), out_clock_s, clocks.len());
        out_written.write(queries.len());
        SidereonStatus::Ok
    })
}

/// Export the product as SP3 text bytes. The output is not null-terminated.
/// Uses the variable-length output contract documented at the top of the
/// header.
///
/// The write is refused when the product holds something SP3 text cannot state
/// exactly: a value its column would read back as a different number or as an
/// absence sentinel, a field wider than its columns, an epoch no record
/// restates, or a header that disagrees with the records. A refusal returns
/// SIDEREON_STATUS_INVALID_ARGUMENT with the engine's text in the thread-local
/// message, reports a required length of zero and writes nothing, so it never
/// reads as a successful empty file. sidereon_sp3_to_sp3_text_result returns
/// the same refusal typed, with owned text.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_to_sp3_text(
    sp3: *const SidereonSp3,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_operation_boundary("sidereon_sp3_to_sp3_text", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_sp3_to_sp3_text",
            out_written,
            out_required
        ));
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_to_sp3_text", "sp3"));
        let text = match sp3.inner.to_sp3_string() {
            Ok(text) => text,
            Err(err) => {
                set_last_error(format!("sidereon_sp3_to_sp3_text: {err}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            "sidereon_sp3_to_sp3_text",
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

/// Write the product as SP3 text and take an owned record of the attempt.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonSp3WriteResult: the SP3 text when the product was
/// written, or the typed refusal sidereon_sp3_to_sp3_text reports only as text.
/// Read it with sidereon_sp3_write_result_get_outcome, _get_text,
/// _get_message, _get_field and _get_text_value. A null argument leaves
/// `*out_result` NULL, returns SIDEREON_STATUS_NULL_POINTER and allocates
/// nothing.
///
/// Safety: `sp3` must be a live SidereonSp3 handle, not freed for the duration
/// of the call; `out_result` must point to writable, aligned storage for one
/// `SidereonSp3WriteResult *`, which is set to NULL before any work and
/// receives a newly owned handle only on SIDEREON_STATUS_OK; release it with
/// sidereon_sp3_write_result_free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_to_sp3_text_result(
    sp3: *const SidereonSp3,
    out_result: *mut *mut SidereonSp3WriteResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_to_sp3_text_result";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        write_boxed_handle(out_result, sp3_write_result(FN_NAME, &sp3.inner));
        SidereonStatus::Ok
    })
}

/// Release an owned SP3 write result. Passing NULL is a no-op.
///
/// Safety: `result` may be NULL; otherwise it must be a live
/// SidereonSp3WriteResult handle this binding produced, passed here exactly
/// once. The handle is invalid afterwards.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_free(result: *mut SidereonSp3WriteResult) {
    ffi_boundary("sidereon_sp3_write_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned SP3 write result.
///
/// `*out_outcome` is written before the result pointer is validated, so it
/// never keeps whatever the caller left in it.
///
/// Safety: `result` must be a live SidereonSp3WriteResult handle;
/// `out_outcome` must point to one writable, aligned SidereonSp3WriteOutcome
/// that no other argument aliases.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_get_outcome(
    result: *const SidereonSp3WriteResult,
    out_outcome: *mut SidereonSp3WriteOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_write_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_outcome = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        *out_outcome = SidereonSp3WriteOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_sp3_write_error(),
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out_outcome = result.outcome;
        SidereonStatus::Ok
    })
}

/// Copy the SP3 text of an owned write result. Uses the variable-length output
/// contract; the bytes are not null-terminated.
///
/// A refused result has no text: the call returns
/// SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero, writes
/// nothing, and sets the thread-local message to the result's own refusal text.
///
/// Safety: `result` must be a live SidereonSp3WriteResult handle; `out` may be
/// NULL only when `len` is 0; otherwise it must point to `len` writable bytes;
/// `out_written` and `out_required` must each point to a writable size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_get_text(
    result: *const SidereonSp3WriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_write_result_get_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
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

/// Copy the refusal text of an owned SP3 write result, prefixed with the route
/// that produced it. A written result reports a required length of zero. Uses
/// the variable-length output contract; the bytes are not null-terminated.
///
/// Safety: as sidereon_sp3_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_get_message(
    result: *const SidereonSp3WriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_write_result_get_message";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.message.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the name of the header or record field a refused SP3 write names,
/// exactly as the engine names it, such as "pos/vel base". The error's
/// has_field says whether the refusal names one; when it does not, the
/// required length is zero. Uses the variable-length output contract; the
/// bytes are not null-terminated.
///
/// Safety: as sidereon_sp3_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_get_field(
    result: *const SidereonSp3WriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_write_result_get_field";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.field.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the text a refused SP3 write names, as the product holds it: the
/// header text or comment the columns could not carry. The error's
/// has_text_value says whether the refusal carries one; the text itself may
/// be empty, as for a blank descriptor. Uses the variable-length output
/// contract; the bytes are copied verbatim and not null-terminated.
///
/// Safety: as sidereon_sp3_write_result_get_text.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_write_result_get_text_value(
    result: *const SidereonSp3WriteResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_write_result_get_text_value";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.text_value.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Estimate the per-epoch clock-reference offset of `other` relative to
/// `reference`. Delegates to sidereon_core::ephemeris::clock_reference_offset.
/// Uses the variable-length output contract documented at the top of the
/// header.
///
/// Safety: reference and other must be live SP3 handles; out must point to at
/// least len writable SidereonSp3ClockReferenceOffset entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_clock_reference_offsets(
    reference: *const SidereonSp3,
    other: *const SidereonSp3,
    min_common: usize,
    out: *mut SidereonSp3ClockReferenceOffset,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_clock_reference_offsets",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_clock_reference_offsets",
                out_written,
                out_required
            ));
            let reference = c_try!(require_ref(
                reference,
                "sidereon_sp3_clock_reference_offsets",
                "reference"
            ));
            let other = c_try!(require_ref(
                other,
                "sidereon_sp3_clock_reference_offsets",
                "other"
            ));
            let values: Vec<SidereonSp3ClockReferenceOffset> =
                clock_reference_offset(&reference.inner, &other.inner, min_common)
                    .into_iter()
                    .map(|offset| SidereonSp3ClockReferenceOffset {
                        epoch_j2000_seconds: instant_to_j2000_seconds(&offset.epoch)
                            .unwrap_or(f64::NAN),
                        offset_s: offset.offset_s,
                        satellites: offset.satellites,
                    })
                    .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_clock_reference_offsets",
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

/// Return a copy of `other` with its clocks shifted onto `reference`'s clock
/// datum. Delegates to sidereon_core::ephemeris::align_clock_reference.
///
/// Safety: reference and other must be live SP3 handles; out_sp3 must point to
/// storage for a SidereonSp3*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_align_clock_reference(
    reference: *const SidereonSp3,
    other: *const SidereonSp3,
    min_common: usize,
    out_sp3: *mut *mut SidereonSp3,
) -> SidereonStatus {
    sp3_operation_boundary(
        "sidereon_sp3_align_clock_reference",
        SidereonStatus::Panic,
        || {
            let out_sp3 = c_try!(require_out(
                out_sp3,
                "sidereon_sp3_align_clock_reference",
                "out_sp3"
            ));
            *out_sp3 = ptr::null_mut();
            let reference = c_try!(require_ref(
                reference,
                "sidereon_sp3_align_clock_reference",
                "reference"
            ));
            let other = c_try!(require_ref(
                other,
                "sidereon_sp3_align_clock_reference",
                "other"
            ));
            let inner = align_clock_reference(&reference.inner, &other.inner, min_common);
            write_boxed_handle(out_sp3, SidereonSp3 { inner });
            SidereonStatus::Ok
        },
    )
}

/// Initialize SP3 merge options with engine defaults.
///
/// Safety: out_options must point to a SidereonSp3MergeOptions.
/// Fill *out_options with sidereon-core's ContinuityOptions::for_orbit_class:
/// the class speed bound, a 1 m hold-out residual tolerance and the default
/// gap threshold. orbit_class is a SidereonSp3OrbitClass value.
///
/// Safety: out_options must point to a SidereonSp3ContinuityOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_continuity_options_for_orbit_class(
    orbit_class: u32,
    out_options: *mut SidereonSp3ContinuityOptions,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_continuity_options_for_orbit_class";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_options, FN_NAME, "out_options"));
        *out = continuity_options_to_c(&ContinuityOptions::for_orbit_class(OrbitClass::MeoGnss));
        let class = c_try!(orbit_class_from_c(FN_NAME, "orbit_class", orbit_class));
        *out = continuity_options_to_c(&ContinuityOptions::for_orbit_class(class));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_options_init(
    out_options: *mut SidereonSp3MergeOptions,
) -> SidereonStatus {
    sp3_operation_boundary(
        "sidereon_sp3_merge_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_sp3_merge_options_init",
                "out_options"
            ));
            *out_options = default_sp3_merge_options();
            SidereonStatus::Ok
        },
    )
}

/// Build the canonical, versioned identity of a complete exact SP3 artifact
/// set and the full merge policy.
///
/// Contributor enumeration order does not affect mean or median identities.
/// Precedence identities bind the original order because it determines source
/// priority. Unordered policy fields are canonicalized. Every artifact field is
/// validated by the core; empty, duplicate, incomplete, malformed, non-SP3, or
/// mismatched records fail closed. The returned handle exposes the stable ID,
/// canonical contributors, and ordered precedence contributors without
/// discarding any part of the core result.
///
/// Safety: `contributors` must reference `contributor_count` readable records
/// (and may be NULL only when the count is zero); `options` may be NULL for
/// defaults; `out_identity` must reference writable storage. On success the
/// caller owns the returned handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity(
    contributors: *const SidereonSp3ArtifactIdentity,
    contributor_count: usize,
    options: *const SidereonSp3MergeOptions,
    out_identity: *mut *mut SidereonSp3MergeInputIdentity,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity";
    sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_identity = c_try!(require_out(out_identity, FN_NAME, "out_identity"));
        *out_identity = ptr::null_mut();
        let contributors = c_try!(require_slice(
            contributors,
            contributor_count,
            FN_NAME,
            "contributors"
        ));
        let contributors = c_try!(contributors
            .iter()
            .enumerate()
            .map(|(index, contributor)| {
                sp3_artifact_identity_from_c(FN_NAME, index, contributor)
            })
            .collect::<Result<Vec<_>, SidereonStatus>>());
        let options = c_try!(sp3_merge_options_from_c(FN_NAME, options));
        let identity = c_try!(Sp3MergeInputIdentity::new(&contributors, &options).map_err(
            |error| {
                set_last_error(format!(
                    "{FN_NAME}: {}",
                    sp3_merge_identity_error_detail(&error)
                ));
                SidereonStatus::InvalidArgument
            }
        ));
        write_boxed_handle(
            out_identity,
            SidereonSp3MergeInputIdentity { inner: identity },
        );
        SidereonStatus::Ok
    })
}

/// Release a merged-SP3 input identity handle. NULL is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_free(
    identity: *mut SidereonSp3MergeInputIdentity,
) {
    free_boxed(identity);
}

/// Read the canonical encoding version.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_schema_version(
    identity: *const SidereonSp3MergeInputIdentity,
    out_schema_version: *mut u8,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_schema_version";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        let output = c_try!(require_out(
            out_schema_version,
            FN_NAME,
            "out_schema_version"
        ));
        *output = identity.inner.schema_version;
        SidereonStatus::Ok
    })
}

/// Copy the versioned stable identity. The bytes are not null-terminated; pass
/// NULL with `out_stable_id_len == 0` to query the required length.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_stable_id(
    identity: *const SidereonSp3MergeInputIdentity,
    out_stable_id: *mut u8,
    out_stable_id_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_stable_id";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out_stable_id",
            identity.inner.stable_id.as_bytes(),
            out_stable_id,
            out_stable_id_len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Number of distributor-independent canonical contributors.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_contributor_count(
    identity: *const SidereonSp3MergeInputIdentity,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_contributor_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        let output = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *output = identity.inner.contributors.len();
        SidereonStatus::Ok
    })
}

/// Copy one distributor-independent canonical contributor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_contributor(
    identity: *const SidereonSp3MergeInputIdentity,
    index: usize,
    out_contributor: *mut SidereonSp3ArtifactIdentity,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_contributor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        let contributor = c_try!(identity.inner.contributors.get(index).ok_or_else(|| {
            set_last_error(format!("{FN_NAME}: index {index} is out of range"));
            SidereonStatus::InvalidArgument
        }));
        let output = c_try!(require_out(out_contributor, FN_NAME, "out_contributor"));
        *output = c_try!(sp3_artifact_identity_to_c(FN_NAME, contributor));
        SidereonStatus::Ok
    })
}

/// Report whether precedence ordering is present and its contributor count.
/// `out_present` is exactly 0 for mean/median and 1 for precedence.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_precedence_contributor_count(
    identity: *const SidereonSp3MergeInputIdentity,
    out_present: *mut u8,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_precedence_contributor_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        let present = c_try!(require_out(out_present, FN_NAME, "out_present"));
        let count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        let contributors = identity.inner.precedence_contributors.as_deref();
        *present = u8::from(contributors.is_some());
        *count = contributors.map_or(0, <[_]>::len);
        SidereonStatus::Ok
    })
}

/// Copy one ordered precedence contributor. This fails for mean/median results.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_input_identity_precedence_contributor(
    identity: *const SidereonSp3MergeInputIdentity,
    index: usize,
    out_contributor: *mut SidereonSp3ArtifactIdentity,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_input_identity_precedence_contributor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let identity = c_try!(require_ref(identity, FN_NAME, "identity"));
        let contributors =
            c_try!(identity
                .inner
                .precedence_contributors
                .as_ref()
                .ok_or_else(|| {
                    set_last_error(format!("{FN_NAME}: precedence ordering is absent"));
                    SidereonStatus::InvalidArgument
                }));
        let contributor = c_try!(contributors.get(index).ok_or_else(|| {
            set_last_error(format!("{FN_NAME}: index {index} is out of range"));
            SidereonStatus::InvalidArgument
        }));
        let output = c_try!(require_out(out_contributor, FN_NAME, "out_contributor"));
        *output = c_try!(sp3_artifact_identity_to_c(FN_NAME, contributor));
        SidereonStatus::Ok
    })
}

/// Merge SP3 products using the engine consensus merge path. On success writes
/// newly owned handles to *out_sp3 and *out_report. Release them with
/// sidereon_sp3_free and sidereon_sp3_merge_report_free.
///
/// Safety: sources must point to source_count live SidereonSp3* handles when
/// source_count is nonzero; options may be NULL for defaults; out_sp3 and
/// out_report must point to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge(
    sources: *const *const SidereonSp3,
    source_count: usize,
    options: *const SidereonSp3MergeOptions,
    out_sp3: *mut *mut SidereonSp3,
    out_report: *mut *mut SidereonSp3MergeReport,
) -> SidereonStatus {
    sp3_operation_boundary("sidereon_sp3_merge", SidereonStatus::Panic, || {
        if !out_sp3.is_null() && !out_report.is_null() {
            let outputs = [
                Some((
                    c_try!(checked_output_range(
                        "sidereon_sp3_merge",
                        out_sp3,
                        1,
                        "out_sp3"
                    )),
                    "out_sp3",
                )),
                Some((
                    c_try!(checked_output_range(
                        "sidereon_sp3_merge",
                        out_report,
                        1,
                        "out_report"
                    )),
                    "out_report",
                )),
            ];
            c_try!(reject_overlapping_optional_outputs(
                "sidereon_sp3_merge",
                &outputs
            ));
        }
        let out_sp3 = c_try!(require_out(out_sp3, "sidereon_sp3_merge", "out_sp3"));
        *out_sp3 = ptr::null_mut();
        let out_report = c_try!(require_out(out_report, "sidereon_sp3_merge", "out_report"));
        *out_report = ptr::null_mut();
        c_try!(validate_element_count::<Sp3>(
            "sidereon_sp3_merge",
            "source_count",
            source_count
        ));
        let source_handles = c_try!(require_slice(
            sources,
            source_count,
            "sidereon_sp3_merge",
            "sources"
        ));
        let mut core_sources = Vec::with_capacity(source_handles.len());
        for (idx, source) in source_handles.iter().copied().enumerate() {
            let source = c_try!(require_ref(
                source,
                "sidereon_sp3_merge",
                &format!("sources[{idx}]")
            ));
            core_sources.push(source.inner.clone());
        }
        let options = c_try!(sp3_merge_options_from_c("sidereon_sp3_merge", options));
        let (inner, report) = c_try!(guard_core(
            || merge(&core_sources, &options),
            |err| map_sp3_argument_error("sidereon_sp3_merge", err),
        ));
        let sp3_handle = Box::new(SidereonSp3 { inner });
        let epoch_agreement = report.per_epoch_agreement();
        let report_handle = Box::new(SidereonSp3MergeReport {
            inner: report,
            epoch_agreement,
        });
        *out_sp3 = Box::into_raw(sp3_handle);
        *out_report = Box::into_raw(report_handle);
        SidereonStatus::Ok
    })
}

/// Decide whether an optional merge continuity post-condition influences an
/// inclusive evaluation window.
///
/// The result is the same JSON object returned by
/// `sidereon_sp3_continuity_verdict_json`, or JSON `null` when continuity
/// verification was not requested for the merge. The report carries the
/// merged product's interpolation nodes: a violation influences the window when
/// the nodes the window's interpolations select include its held-out, repeated
/// or pair-end record, or straddle a handover between two of its records
/// written from different sources. A single-sample series influences every
/// window. As in sidereon-core, the verdict takes the window alone: no merged
/// product and no caller-provided stencil value.
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `report` must be a live handle; `out` must reference
/// `out_len` writable bytes, or be NULL when `out_len` is zero; both count
/// pointers must reference writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_continuity_verdict_json(
    report: *const SidereonSp3MergeReport,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_continuity_verdict_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let window = c_try!(EpochWindow::new(from_j2000_s, through_j2000_s)
            .map_err(|error| map_sp3_argument_error(FN_NAME, error)));
        let value = report
            .inner
            .continuity_verdict_for_window(window)
            .map(window_continuity_verdict_json)
            .unwrap_or(serde_json::Value::Null);
        c_try!(copy_json_value_to_c(
            FN_NAME,
            &value,
            out,
            out_len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the merge's continuity post-condition as JSON: `null` when the merge
/// did not verify continuity, otherwise an object with `attested`, `defects`
/// (every defect over the merged product, as the verdict JSON describes each),
/// `pairs_checked`, `residuals_checked`, `residuals_skipped` and `violations`:
/// every violation attributed to the contributors it rests on, splices and
/// single-contributor discontinuities alike, each as a splice object of the
/// verdict JSON (`defect`, `from_sources`, `to_sources`, `sources`,
/// `crosses_contributors`, `cells`).
///
/// Uses the standard variable-length byte-output contract; JSON bytes are not
/// null-terminated.
///
/// Safety: `report` must be a live handle; `out` must reference `out_len`
/// writable bytes, or be NULL when `out_len` is zero; both count pointers must
/// reference writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_continuity_json(
    report: *const SidereonSp3MergeReport,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_continuity_json";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let value = report
            .inner
            .continuity
            .as_ref()
            .map(merge_continuity_report_json)
            .unwrap_or(serde_json::Value::Null);
        c_try!(copy_json_value_to_c(
            FN_NAME,
            &value,
            out,
            out_len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Write the number of coordinate-label reconciliation rows in a merge report.
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_count(
    report: *const SidereonSp3MergeReport,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_frame_reconciliation_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_frame_reconciliation_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_frame_reconciliation_count",
                "report"
            ));
            *out_count = report.inner.frame_reconciliations.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one coordinate-label reconciliation row by index.
///
/// Safety: report must be a live merge report handle; out_reconciliation must
/// point to a SidereonSp3FrameReconciliation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out_reconciliation: *mut SidereonSp3FrameReconciliation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_frame_reconciliation",
        SidereonStatus::Panic,
        || {
            let out_reconciliation = c_try!(require_out(
                out_reconciliation,
                "sidereon_sp3_merge_report_frame_reconciliation",
                "out_reconciliation"
            ));
            *out_reconciliation = zero_sp3_frame_reconciliation();
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_frame_reconciliation",
                "report"
            ));
            let Some(reconciliation) = report.inner.frame_reconciliations.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_frame_reconciliation: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_reconciliation = sp3_frame_reconciliation_to_c(reconciliation);
            SidereonStatus::Ok
        },
    )
}

/// Copy a reconciliation source label as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_source_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_source_label",
        report,
        index,
        |row| row.source_label.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy a reconciliation target label as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_target_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_target_label",
        report,
        index,
        |row| row.target_label.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy one asserted-label-set item as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_asserted_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    label_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_asserted_label",
        report,
        index,
        |row| {
            row.asserted_label_set
                .as_ref()
                .and_then(|labels| labels.get(label_index))
                .map(String::as_bytes)
                .unwrap_or(&[])
        },
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy the published-table provenance as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_provenance(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_provenance",
        report,
        index,
        |row| row.provenance.as_deref().unwrap_or("").as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Write the number of flags in a merge report flag list to *out_count. kind is
/// one of SidereonSp3MergeFlagKind_*.
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag_count(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_flag_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag_count",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag_count",
                &report.inner,
                kind,
            ));
            *out_count = flags.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one merge report flag by index. kind is one of
/// SidereonSp3MergeFlagKind_*.
///
/// Safety: report must be a live merge report handle; out_flag must point to a
/// SidereonSp3MergeFlag.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    index: usize,
    out_flag: *mut SidereonSp3MergeFlag,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag",
        SidereonStatus::Panic,
        || {
            let out_flag = c_try!(require_out(
                out_flag,
                "sidereon_sp3_merge_report_flag",
                "out_flag"
            ));
            *out_flag = SidereonSp3MergeFlag {
                epoch_j2000_seconds: f64::NAN,
                epoch: crate::rinex_clock::absent_clock_epoch(),
                sat_id: SidereonSatelliteToken {
                    bytes: [0; SATELLITE_TOKEN_C_BYTES],
                },
                source_count: 0,
            };
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag",
                &report.inner,
                kind,
            ));
            let Some(flag) = flags.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_flag: flag index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_flag = sp3_merge_flag_to_c(flag);
            SidereonStatus::Ok
        },
    )
}

/// Copy the source indices for one merge report flag. kind is one of
/// SidereonSp3MergeFlagKind_* and the output uses the variable-length contract
/// documented at the top of the header.
///
/// Safety: report must be a live merge report handle; out must point to at
/// least len writable size_t values or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag_sources(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    index: usize,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag_sources",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_merge_report_flag_sources",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag_sources",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag_sources",
                &report.inner,
                kind,
            ));
            let Some(flag) = flags.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_flag_sources: flag index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_merge_report_flag_sources",
                "out",
                &flag.sources,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the number of per-epoch agreement entries to *out_count. This is the
/// length of the list copied element-wise by
/// sidereon_sp3_merge_report_epoch_agreement (one entry per output epoch).
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_epoch_agreement_count(
    report: *const SidereonSp3MergeReport,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_epoch_agreement_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_epoch_agreement_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_epoch_agreement_count",
                "report"
            ));
            *out_count = report.epoch_agreement.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one per-epoch agreement entry (by zero-based output-epoch index) into
/// *out_agreement. Fails with SIDEREON_STATUS_INVALID_ARGUMENT if index is out of
/// range (see sidereon_sp3_merge_report_epoch_agreement_count).
///
/// Safety: report must be a live merge report handle; out_agreement must point to
/// a SidereonSp3EpochAgreement.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_epoch_agreement(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out_agreement: *mut SidereonSp3EpochAgreement,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_epoch_agreement",
        SidereonStatus::Panic,
        || {
            let out_agreement = c_try!(require_out(
                out_agreement,
                "sidereon_sp3_merge_report_epoch_agreement",
                "out_agreement"
            ));
            *out_agreement = empty_sp3_epoch_agreement();
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_epoch_agreement",
                "report"
            ));
            let Some(agg) = report.epoch_agreement.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_epoch_agreement: index {index} out of range ({} epochs)",
                    report.epoch_agreement.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_agreement = sp3_epoch_agreement_to_c(agg);
            SidereonStatus::Ok
        },
    )
}

/// Write the whole-product agreement rollup (pooled position/clock dispersion of
/// the consensus members about the combined values) to *out_summary. Each scalar
/// carries a present flag; an absent value sets the flag false and the scalar to
/// NaN. See SidereonSp3AgreementSummary for when each one is absent.
///
/// Safety: report must be a live merge report handle; out_summary must point to a
/// SidereonSp3AgreementSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_agreement_summary(
    report: *const SidereonSp3MergeReport,
    out_summary: *mut SidereonSp3AgreementSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_agreement_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_sp3_merge_report_agreement_summary",
                "out_summary"
            ));
            *out_summary = empty_sp3_agreement_summary();
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_agreement_summary",
                "report"
            ));
            *out_summary = sp3_agreement_summary_to_c(&report.inner);
            SidereonStatus::Ok
        },
    )
}

/// Release an SP3 merge report handle. Null is a no-op. A non-null handle must
/// come from sidereon_sp3_merge and must be freed exactly once with this
/// function.
///
/// Safety: report must be NULL or a live handle from sidereon_sp3_merge. Passing
/// a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_free(report: *mut SidereonSp3MergeReport) {
    ffi_boundary("sidereon_sp3_merge_report_free", (), || {
        free_boxed(report);
    });
}

/// Release an SP3 handle. Null is a no-op. A non-null handle must come from
/// sidereon_sp3_load, sidereon_sp3_load_exact, or sidereon_sp3_merge and must
/// be freed exactly once with this function.
///
/// Safety: sp3 must be NULL or a live handle from sidereon_sp3_load,
/// sidereon_sp3_load_exact, or sidereon_sp3_merge. Passing a handle after it
/// has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_free(sp3: *mut SidereonSp3) {
    ffi_boundary("sidereon_sp3_free", (), || {
        free_boxed(sp3);
    });
}

/// List satellites visible from a static receiver at one epoch, scanning the SP3
/// product's own satellites. Delegates to sidereon_core::geometry::visible. Uses
/// the variable-length output contract documented at the top of the header.
///
/// `systems` may be NULL with systems_len 0 to keep every constellation, or
/// point to systems_len SidereonGnssSystem codes (cast to uint32_t) to filter.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems must point to systems_len readable uint32_t (or be
/// NULL when systems_len is 0); out must point to at least len writable
/// SidereonGeometryVisible or be NULL when len is 0; out_written and out_required
/// must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_visible(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonGeometryVisible,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_sp3_geometry_visible",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_visible",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_geometry_visible", "sp3"));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_visible",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_visible",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let rows = c_try!(guard_dop("sidereon_sp3_geometry_visible", || {
                geometry_visible(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    t_rx_j2000_s,
                    &options,
                )
            }));
            let values: Vec<SidereonGeometryVisible> =
                rows.iter().map(geometry_visible_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_visible",
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

/// Count visible satellites over an inclusive sampled window. Delegates to
/// sidereon_core::geometry::visibility_series. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems follows sidereon_sp3_geometry_visible; out must
/// point to at least len writable SidereonVisibilitySeriesPoint or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_visibility_series(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    window_start_j2000_s: f64,
    window_end_j2000_s: f64,
    step_seconds: u64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonVisibilitySeriesPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_sp3_geometry_visibility_series",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_visibility_series",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_geometry_visibility_series",
                "sp3"
            ));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_visibility_series",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_visibility_series",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let points = c_try!(guard_dop("sidereon_sp3_geometry_visibility_series", || {
                geometry_visibility_series(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    (window_start_j2000_s, window_end_j2000_s),
                    step_seconds,
                    &options,
                )
            }));
            let values: Vec<SidereonVisibilitySeriesPoint> =
                points.iter().map(visibility_series_point_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_visibility_series",
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

/// Build sampled rise/set/peak visibility passes over an inclusive window.
/// Delegates to sidereon_core::geometry::passes. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems follows sidereon_sp3_geometry_visible; out must
/// point to at least len writable SidereonVisibilityPass or be NULL when len is
/// 0; out_written and out_required must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_passes(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    window_start_j2000_s: f64,
    window_end_j2000_s: f64,
    step_seconds: u64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonVisibilityPass,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    crate::engine_error::engine_error_operation_boundary(
        "sidereon_sp3_geometry_passes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_passes",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_geometry_passes", "sp3"));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_passes",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_passes",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let found = c_try!(guard_dop("sidereon_sp3_geometry_passes", || {
                geometry_passes(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    (window_start_j2000_s, window_end_j2000_s),
                    step_seconds,
                    &options,
                )
            }));
            let values: Vec<SidereonVisibilityPass> =
                found.iter().map(visibility_pass_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_passes",
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

// ===========================================================================
// Predicted observables (line of sight, range rate, Doppler, az/el) from an SP3
// or broadcast source. Delegates to sidereon_core::observables::predict.

/// Predict one satellite's observables from a loaded SP3 product. Delegates to
/// sidereon_core::observables::predict. options may be NULL for the engine
/// defaults.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated token;
/// receiver_ecef_m must point to three readable doubles; options must be NULL or
/// point to a SidereonObservablesOptions; out must point to a
/// SidereonPredictedObservables.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observables(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_observables",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(out, "sidereon_sp3_observables", "out"));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observables", "sp3"));
            let satellite = c_try!(parse_satellite_token("sidereon_sp3_observables", sat_id));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_observables",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let opts = c_try!(predict_options_from_c("sidereon_sp3_observables", options));
            let obs = match observables_predict(
                &sp3.inner,
                satellite,
                receiver_ecef_m,
                t_rx_j2000_s,
                opts,
            ) {
                Ok(obs) => obs,
                Err(err) => return map_observables_error("sidereon_sp3_observables", err),
            };
            *out = predicted_observables_to_c(&obs);
            SidereonStatus::Ok
        },
    )
}

/// Evaluate an SP3 precise product at a J2000 second for one satellite via the
/// same ObservableEphemerisSource contract as the broadcast path. Delegates to
/// sidereon_core::observables::ObservableEphemerisSource::observable_state_at_j2000_s.
///
/// Safety: as sidereon_broadcast_observable_state with an SP3 handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_state(
    sp3: *const SidereonSp3,
    satellite_id: *const c_char,
    t_j2000_s: f64,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
    out_has_clock: *mut bool,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_observable_state",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observable_state", "sp3"));
            observable_state_common(
                "sidereon_sp3_observable_state",
                &sp3.inner,
                satellite_id,
                t_j2000_s,
                out_position_ecef_m,
                out_clock_s,
                out_has_clock,
            )
        },
    )
}

/// Predict observables for many `(satellite, receiver, epoch)` requests from a
/// loaded SP3 product in one call. Delegates to the core serial `predict_batch`.
/// `out` and `out_ok` are caller arrays of `count` entries: `out[i]` holds the
/// observables for `requests[i]` and `out_ok[i]` is true on success. A
/// per-request failure (e.g. no ephemeris) sets `out_ok[i]` false and zeroes
/// `out[i]`; the call still returns SIDEREON_STATUS_OK. options may be NULL for
/// the engine defaults.
///
/// Safety: sp3 must be a live handle; requests must point to count entries (each
/// with a valid sat_id); out and out_ok must each point to count writable
/// entries (or be NULL when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observables_batch(
    sp3: *const SidereonSp3,
    requests: *const SidereonPredictRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
    out_ok: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_observables_batch",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observables_batch", "sp3"));
            let raw = c_try!(require_slice(
                requests,
                count,
                "sidereon_sp3_observables_batch",
                "requests"
            ));
            // Guard the caller's output arrays (non-null when count > 0, no element
            // overflow) before writing them element-by-element below.
            c_try!(require_slice(
                out as *const SidereonPredictedObservables,
                count,
                "sidereon_sp3_observables_batch",
                "out"
            ));
            c_try!(require_slice(
                out_ok as *const bool,
                count,
                "sidereon_sp3_observables_batch",
                "out_ok"
            ));
            let opts = c_try!(predict_options_from_c(
                "sidereon_sp3_observables_batch",
                options
            ));
            let parsed = c_try!(predict_requests_from_c(
                "sidereon_sp3_observables_batch",
                raw
            ));
            let results = core_predict_batch(&sp3.inner, &parsed, opts);
            write_predict_batch_results(&results, out, out_ok);
            SidereonStatus::Ok
        },
    )
}

/// Extract a loaded SP3 product as its canonical precise-ephemeris samples, in
/// SI units, one per real position record in ascending epoch order. Round-tripping
/// through sidereon_precise_ephemeris_samples_from_samples rebuilds the same
/// interpolatable source. Uses the variable-length output contract documented at
/// the top of the header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// SidereonPreciseEphemerisSample or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_precise_ephemeris_samples(
    sp3: *const SidereonSp3,
    out: *mut SidereonPreciseEphemerisSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_precise_ephemeris_samples",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_precise_ephemeris_samples",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_precise_ephemeris_samples",
                "sp3"
            ));
            let samples = sp3.inner.precise_ephemeris_samples();
            let values: Vec<SidereonPreciseEphemerisSample> =
                samples.iter().map(precise_sample_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_precise_ephemeris_samples",
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
pub unsafe extern "C" fn sidereon_sp3_precise_ephemeris_samples_v2(
    sp3: *const SidereonSp3,
    out: *mut SidereonPreciseEphemerisSampleV2,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_precise_ephemeris_samples_v2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let values = sp3
            .inner
            .precise_ephemeris_samples()
            .into_iter()
            .map(|sample| SidereonPreciseEphemerisSampleV2 {
                sat: satellite_token(sample.sat),
                epoch: crate::rinex_clock::instant_to_clock_epoch(&sample.epoch),
                position_ecef_m: sample.position_ecef_m,
                has_clock_s: sample.clock_s.is_some(),
                clock_s: sample.clock_s.unwrap_or_default(),
                clock_event: sample.clock_event,
            })
            .collect::<Vec<_>>();
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &values,
            out,
            len,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Sample a loaded SP3 source over a regular satellite/epoch grid.
///
/// Safety: sp3 must be a live handle; satellites points to satellite_count
/// null-terminated tokens; out points to len SidereonEphemerisSampleRow or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_ephemeris_sample(
    sp3: *const SidereonSp3,
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
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_ephemeris_sample",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_ephemeris_sample", "sp3"));
            ephemeris_sample_common(
                "sidereon_sp3_ephemeris_sample",
                &sp3.inner,
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

/// Predict geometric ranges for many (satellite, receiver, epoch) requests from a
/// loaded SP3 product in one call, writing `out[i]` for `requests[i]`. Delegates to
/// sidereon_core::observables::predict_ranges. options may be NULL for the engine
/// defaults.
///
/// Safety: sp3 must be a live handle; requests must point to count entries (each
/// with a valid sat_id); out must point to count writable entries (or be NULL
/// when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_predict_ranges(
    sp3: *const SidereonSp3,
    requests: *const SidereonRangePredictionRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonRangePrediction,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_predict_ranges",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_predict_ranges", "sp3"));
            predict_ranges_into(
                "sidereon_sp3_predict_ranges",
                &sp3.inner,
                requests,
                count,
                options,
                out,
            )
        },
    )
}

/// Evaluate many SP3 observable states with per-satellite epochs.
///
/// out_positions_ecef_m receives count triples in row-major XYZ order, meters.
/// out_clocks_s and out_has_clocks_s receive count satellite-clock entries in
/// seconds. out_element_statuses receives Valid/Gap/Error. out_result_statuses
/// receives SIDEREON_STATUS_OK for valid elements, SIDEREON_STATUS_SOLVE for
/// gaps/source errors, and SIDEREON_STATUS_INVALID_ARGUMENT for invalid scalar
/// inputs. Failed elements receive the missing-position sentinel.
///
/// Safety: sp3 is a live handle; satellites and epochs_j2000_s point to count
/// entries; output arrays point to count entries except out_positions_ecef_m,
/// which points to count * 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_states_at_j2000_s(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    epochs_j2000_s: *const f64,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_observable_states_at_j2000_s",
                "sp3"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_sp3_observable_states_at_j2000_s",
                &sp3.inner,
                satellites,
                epochs_j2000_s,
                count,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

/// Evaluate many SP3 observable states at one shared epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_states_at_shared_j2000_s(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    satellite_count: usize,
    epoch_j2000_s: f64,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_sp3_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_observable_states_at_shared_j2000_s",
                "sp3"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_sp3_observable_states_at_shared_j2000_s",
                &sp3.inner,
                satellites,
                satellite_count,
                epoch_j2000_s,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

pub(crate) fn interpolation_options_from_c(
    fn_name: &str,
    gap_threshold_factor: f64,
) -> Result<Sp3InterpolationOptions, SidereonStatus> {
    if !gap_threshold_factor.is_nan() && gap_threshold_factor <= 0.0 {
        Ok(Sp3InterpolationOptions::default())
    } else {
        Sp3InterpolationOptions::new(gap_threshold_factor)
            .map_err(|error| map_sp3_argument_error(fn_name, error))
    }
}

fn orbit_class_from_c(
    fn_name: &str,
    arg_name: &str,
    value: u32,
) -> Result<OrbitClass, SidereonStatus> {
    match value {
        v if v == SidereonSp3OrbitClass::MeoGnss as u32 => Ok(OrbitClass::MeoGnss),
        v if v == SidereonSp3OrbitClass::Geosynchronous as u32 => Ok(OrbitClass::Geosynchronous),
        v if v == SidereonSp3OrbitClass::Leo as u32 => Ok(OrbitClass::Leo),
        other => {
            set_last_error(format!("{fn_name}: invalid {arg_name} {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn orbit_class_to_c(class: OrbitClass) -> SidereonSp3OrbitClass {
    match class {
        OrbitClass::MeoGnss => SidereonSp3OrbitClass::MeoGnss,
        OrbitClass::Geosynchronous => SidereonSp3OrbitClass::Geosynchronous,
        OrbitClass::Leo => SidereonSp3OrbitClass::Leo,
    }
}

fn continuity_options_to_c(options: &ContinuityOptions) -> SidereonSp3ContinuityOptions {
    let (speed_bound_kind, orbit_class, explicit_max_speed_m_s) = match options.speed_bound {
        None => (
            SidereonSp3SpeedBoundKind::None,
            SidereonSp3OrbitClass::MeoGnss,
            f64::NAN,
        ),
        Some(SpeedBound::OrbitClass(class)) => (
            SidereonSp3SpeedBoundKind::OrbitClass,
            orbit_class_to_c(class),
            f64::NAN,
        ),
        Some(SpeedBound::ExplicitMaxSpeed(bound)) => (
            SidereonSp3SpeedBoundKind::ExplicitMaxSpeed,
            SidereonSp3OrbitClass::MeoGnss,
            bound,
        ),
    };
    SidereonSp3ContinuityOptions {
        speed_bound_kind: speed_bound_kind as u32,
        orbit_class: orbit_class as u32,
        explicit_max_speed_m_s,
        residual_tolerance_enabled: u8::from(options.residual_tolerance_m.is_some()),
        residual_tolerance_m: options.residual_tolerance_m.unwrap_or(f64::NAN),
        gap_threshold_factor: options.interpolation.gap_threshold_factor(),
    }
}

/// The engine continuity options a caller record states. Invalid bounds are
/// refused with the core's typed field, value and reason.
pub(crate) fn continuity_options_struct_from_c(
    fn_name: &str,
    options: &SidereonSp3ContinuityOptions,
) -> Result<ContinuityOptions, SidereonStatus> {
    let speed_bound = match options.speed_bound_kind {
        v if v == SidereonSp3SpeedBoundKind::None as u32 => None,
        v if v == SidereonSp3SpeedBoundKind::OrbitClass as u32 => Some(SpeedBound::OrbitClass(
            orbit_class_from_c(fn_name, "orbit_class", options.orbit_class)?,
        )),
        v if v == SidereonSp3SpeedBoundKind::ExplicitMaxSpeed as u32 => {
            Some(SpeedBound::ExplicitMaxSpeed(options.explicit_max_speed_m_s))
        }
        other => {
            set_last_error(format!("{fn_name}: invalid speed_bound_kind {other}"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let residual_tolerance_m = if data_distribution::bool_from_c(
        fn_name,
        "residual_tolerance_enabled",
        options.residual_tolerance_enabled,
    )? {
        Some(options.residual_tolerance_m)
    } else {
        None
    };
    let interpolation = interpolation_options_from_c(fn_name, options.gap_threshold_factor)?;
    ContinuityOptions::new(speed_bound, residual_tolerance_m)
        .map_err(|error| map_continuity_options_error(fn_name, error))
        .map(|options| options.with_interpolation_options(interpolation))
}

fn continuity_options_from_c(
    fn_name: &str,
    orbit_class: i32,
    residual_tolerance_m: f64,
    gap_threshold_factor: f64,
) -> Result<ContinuityOptions, SidereonStatus> {
    let speed_bound = match orbit_class {
        -1 => None,
        0 => Some(SpeedBound::OrbitClass(OrbitClass::MeoGnss)),
        1 => Some(SpeedBound::OrbitClass(OrbitClass::Geosynchronous)),
        2 => Some(SpeedBound::OrbitClass(OrbitClass::Leo)),
        _ => {
            set_last_error(format!("{fn_name}: orbit_class must be -1, 0, 1, or 2"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let interpolation = interpolation_options_from_c(fn_name, gap_threshold_factor)?;
    let residual_tolerance = if residual_tolerance_m.is_finite() && residual_tolerance_m < 0.0 {
        None
    } else {
        Some(residual_tolerance_m)
    };
    ContinuityOptions::new(speed_bound, residual_tolerance)
        .map_err(|error| map_continuity_options_error(fn_name, error))
        .map(|options| options.with_interpolation_options(interpolation))
}

fn map_continuity_options_error(fn_name: &str, error: ContinuityOptionsError) -> SidereonStatus {
    if matches!(
        fn_name,
        "sidereon_sp3_check_continuity_with_gap_threshold_factor"
            | "sidereon_sp3_merge"
            | "sidereon_sp3_merge_input_identity"
            | "sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor"
            | "sidereon_sp3_continuity_report_json"
            | "sidereon_sp3_continuity_verdict_json_with_options"
    ) {
        record_sp3_continuity_options_error(error);
    }
    set_last_error(format!(
        "{fn_name}: continuity option field={} value={} reason={}",
        error.field, error.value, error.reason
    ));
    SidereonStatus::InvalidArgument
}

fn continuity_defect_json(defect: &ContinuityDefect) -> serde_json::Value {
    // `from_j2000_s`, `to_j2000_s`, `magnitude` and `bound` summarize every
    // kind alike; the kind's own fields follow under their core names.
    let satellite = defect.satellite().to_string();
    match defect {
        ContinuityDefect::DuplicateEpoch {
            epoch_j2000_s,
            occurrences,
            ..
        } => serde_json::json!({
            "kind": "duplicate_epoch",
            "satellite": satellite,
            "from_j2000_s": epoch_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": *occurrences as f64,
            "bound": serde_json::Value::Null,
            "epoch_j2000_s": epoch_j2000_s,
            "occurrences": occurrences,
        }),
        ContinuityDefect::SingleSampleSeries { .. } => serde_json::json!({
            "kind": "single_sample_series",
            "satellite": satellite,
            "from_j2000_s": serde_json::Value::Null,
            "to_j2000_s": serde_json::Value::Null,
            "magnitude": serde_json::Value::Null,
            "bound": serde_json::Value::Null,
        }),
        ContinuityDefect::UnusableSample {
            sample_index,
            epoch_j2000_s,
            reason,
            ..
        } => serde_json::json!({
            "kind": "unusable_sample",
            "satellite": satellite,
            "from_j2000_s": epoch_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": serde_json::Value::Null,
            "bound": serde_json::Value::Null,
            "sample_index": sample_index,
            "epoch_j2000_s": epoch_j2000_s,
            "reason": unusable_sample_reason_name(*reason),
        }),
        ContinuityDefect::SpeedBound {
            from_j2000_s,
            to_j2000_s,
            interval_s,
            displacement_m,
            implied_speed_m_s,
            bound_m_s,
            ..
        } => serde_json::json!({
            "kind": "speed_bound",
            "satellite": satellite,
            "from_j2000_s": from_j2000_s,
            "to_j2000_s": to_j2000_s,
            "magnitude": implied_speed_m_s,
            "bound": bound_m_s,
            "interval_s": interval_s,
            "displacement_m": displacement_m,
            "implied_speed_m_s": implied_speed_m_s,
            "bound_m_s": bound_m_s,
        }),
        ContinuityDefect::HoldOutResidual {
            epoch_j2000_s,
            preceding_j2000_s,
            residual_m,
            tolerance_m,
            node_epochs_j2000_s,
            ..
        } => serde_json::json!({
            "kind": "hold_out_residual",
            "satellite": satellite,
            "from_j2000_s": preceding_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": residual_m,
            "bound": tolerance_m,
            "epoch_j2000_s": epoch_j2000_s,
            "preceding_j2000_s": preceding_j2000_s,
            "residual_m": residual_m,
            "tolerance_m": tolerance_m,
            "node_epochs_j2000_s": node_epochs_j2000_s,
        }),
    }
}

fn unusable_sample_reason_name(reason: UnusableSampleReason) -> &'static str {
    match reason {
        UnusableSampleReason::EpochNotPlaced => "epoch_not_placed",
        UnusableSampleReason::NonFinitePosition => "non_finite_position",
        _ => "unknown",
    }
}

fn merge_cell_selection_json(
    selection: &sidereon_core::ephemeris::CellSelection,
) -> serde_json::Value {
    use sidereon_core::ephemeris::{CellSelection, MergeCombine};
    match selection {
        CellSelection::SingleSource { source } => serde_json::json!({
            "kind": "single_source",
            "source": source,
        }),
        CellSelection::Precedence { source, members } => serde_json::json!({
            "kind": "precedence",
            "source": source,
            "members": members,
        }),
        CellSelection::Combined { rule, members } => serde_json::json!({
            "kind": "combined",
            "rule": match rule {
                MergeCombine::Mean => "mean",
                MergeCombine::Median => "median",
                MergeCombine::Precedence => "precedence",
            },
            "members": members,
        }),
    }
}

fn merge_continuity_cell_json(
    cell: &sidereon_core::ephemeris::MergeContinuityCell,
) -> serde_json::Value {
    use sidereon_core::ephemeris::MergeContinuityCellRole as Role;
    serde_json::json!({
        "epoch_j2000_s": cell.epoch_j2000_s,
        "role": match cell.role {
            Role::HeldOut => "held_out",
            Role::InterpolationNode => "interpolation_node",
            Role::PairEnd => "pair_end",
            Role::RepeatedEpoch => "repeated_epoch",
        },
        "selection": cell
            .selection
            .as_ref()
            .map_or(serde_json::Value::Null, merge_cell_selection_json),
    })
}

fn merge_continuity_violation_json(violation: &MergeContinuityViolation) -> serde_json::Value {
    serde_json::json!({
        "defect": continuity_defect_json(&violation.defect),
        "from_sources": violation.from_sources,
        "to_sources": violation.to_sources,
        "cells": violation
            .cells
            .iter()
            .map(merge_continuity_cell_json)
            .collect::<Vec<_>>(),
        "sources": violation.sources,
        "crosses_contributors": violation.crosses_contributors,
    })
}

pub(crate) fn continuity_report_json(
    report: &sidereon_core::ephemeris::ContinuityReport,
) -> serde_json::Value {
    serde_json::json!({
        "attested": report.attested(),
        "defects": report
            .defects
            .iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "pairs_checked": report.pairs_checked,
        "residuals_checked": report.residuals_checked,
        "residuals_skipped": report.residuals_skipped,
    })
}

pub(crate) fn merge_continuity_report_json(
    report: &sidereon_core::ephemeris::MergeContinuityReport,
) -> serde_json::Value {
    serde_json::json!({
        "attested": report.attested(),
        "defects": report
            .report
            .defects
            .iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "pairs_checked": report.report.pairs_checked,
        "residuals_checked": report.report.residuals_checked,
        "residuals_skipped": report.report.residuals_skipped,
        "violations": report
            .violations
            .iter()
            .map(merge_continuity_violation_json)
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn window_continuity_verdict_json(
    verdict: WindowContinuityVerdict<'_>,
) -> serde_json::Value {
    let decision = match verdict.decision {
        WindowContinuityDecision::Accept => "accept",
        WindowContinuityDecision::Refuse => "refuse",
    };
    serde_json::json!({
        "decision": decision,
        "accepted": verdict.accepted(),
        "influencing_defects": verdict
            .influencing_defects
            .into_iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "influencing_splices": verdict
            .influencing_splices
            .into_iter()
            .map(merge_continuity_violation_json)
            .collect::<Vec<_>>(),
        "all_defects": verdict
            .all_defects
            .iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "all_splices": verdict
            .all_splices
            .into_iter()
            .map(merge_continuity_violation_json)
            .collect::<Vec<_>>(),
    })
}

unsafe fn copy_json_value_to_c(
    fn_name: &str,
    value: &serde_json::Value,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let json = serde_json::to_vec(value).map_err(|error| {
        set_last_error(format!("{fn_name}: failed to serialize JSON: {error}"));
        SidereonStatus::InvalidArgument
    })?;
    copy_prefix_to_c(
        fn_name,
        "out",
        &json,
        out,
        out_len,
        out_written,
        out_required,
    )
}

fn sp3_merge_identity_error_detail(
    error: &sidereon_core::ephemeris::Sp3MergeInputIdentityError,
) -> String {
    use sidereon_core::ephemeris::Sp3MergeInputIdentityError as IdentityError;

    match error {
        IdentityError::InvalidTolerance(error) => {
            record_sp3_merge_tolerance_error(*error);
            format!(
                "invalid merge tolerance: field={} value={} reason=must be finite and nonnegative",
                merge_tolerance_field_name(error.field),
                error.value
            )
        }
        IdentityError::TargetEpochInterval(error) => {
            record_sp3_epoch_interval_error(*error);
            format!(
                "invalid merge interval: field={} value={} reason={}",
                error.field, error.value, error.reason
            )
        }
        IdentityError::ContinuityOptions(error) => {
            record_sp3_continuity_options_error(*error);
            format!(
                "invalid continuity option: field={} value={} reason={}",
                error.field, error.value, error.reason
            )
        }
        other => other.to_string(),
    }
}

fn merge_tolerance_field_name(field: MergeToleranceField) -> &'static str {
    match field {
        MergeToleranceField::Position => "position_tolerance_m",
        MergeToleranceField::Clock => "clock_tolerance_s",
        MergeToleranceField::OutlierPosition => "outlier_position_tolerance_m",
        MergeToleranceField::OutlierClock => "outlier_clock_tolerance_s",
        _ => "unknown",
    }
}

pub(crate) fn map_sp3_argument_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    if matches!(
        fn_name,
        "sidereon_sp3_merge"
            | "sidereon_sp3_load_with_gap_threshold_factor"
            | "sidereon_sp3_load_exact_with_gap_threshold_factor"
            | "sidereon_sp3_check_continuity_with_gap_threshold_factor"
            | "sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor"
            | "sidereon_sp3_continuity_report_json"
            | "sidereon_sp3_continuity_verdict_json_with_options"
    ) {
        match &err {
            CoreError::Sp3EpochInterval(error) => record_sp3_epoch_interval_error(*error),
            CoreError::Sp3MergeTolerance(error) => record_sp3_merge_tolerance_error(*error),
            CoreError::ContinuityOptions(error) => record_sp3_continuity_options_error(*error),
            _ => {}
        }
    }
    let message = match &err {
        CoreError::Sp3EpochInterval(error) => format!(
            "{fn_name}: field={} value={} reason={}",
            error.field, error.value, error.reason
        ),
        CoreError::Sp3MergeTolerance(error) => format!(
            "{fn_name}: field={} value={} reason=must be finite and nonnegative",
            merge_tolerance_field_name(error.field),
            error.value
        ),
        CoreError::ContinuityOptions(error) => format!(
            "{fn_name}: field={} value={} reason={}",
            error.field, error.value, error.reason
        ),
        _ => format!("{fn_name}: {err}"),
    };
    set_last_error(message);
    match err {
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::UnknownSatellite(_)
        | CoreError::EpochOutOfRange
        | CoreError::InvalidInput(_)
        | CoreError::Sp3EpochInterval(_)
        | CoreError::Sp3MergeTolerance(_)
        | CoreError::ContinuityOptions(_) => SidereonStatus::InvalidArgument,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
    }
}

pub(crate) fn map_sp3_interpolation_error(
    fn_name: &str,
    query: f64,
    err: CoreError,
) -> SidereonStatus {
    match &err {
        CoreError::Sp3EpochInterval(error) => record_sp3_epoch_interval_error(*error),
        CoreError::Sp3MergeTolerance(error) => record_sp3_merge_tolerance_error(*error),
        CoreError::ContinuityOptions(error) => record_sp3_continuity_options_error(*error),
        _ => {}
    }
    set_last_error(format!(
        "{fn_name}: interpolation at j2000 second {query}: {err}"
    ));
    match err {
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::UnknownSatellite(_)
        | CoreError::InvalidInput(_)
        | CoreError::Sp3EpochInterval(_)
        | CoreError::Sp3MergeTolerance(_)
        | CoreError::ContinuityOptions(_) => SidereonStatus::InvalidArgument,
        CoreError::EpochOutOfRange => SidereonStatus::Solve,
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!(
                "{fn_name}: interpolation at j2000 second {query}: {other} ({other:?})"
            ));
            SidereonStatus::Solve
        }
    }
}

pub(crate) fn empty_sp3_state() -> SidereonSp3State {
    SidereonSp3State {
        position_m: [0.0; 3],
        has_clock_s: false,
        clock_s: 0.0,
        has_velocity_m_s: false,
        velocity_m_s: [0.0; 3],
        has_clock_rate_s_s: false,
        clock_rate_s_s: 0.0,
        clock_event: false,
        clock_predicted: false,
        maneuver: false,
        orbit_predicted: false,
    }
}

pub(crate) fn sp3_state_to_c(state: Sp3State) -> SidereonSp3State {
    SidereonSp3State {
        position_m: state.position.as_array(),
        has_clock_s: state.clock_s.is_some(),
        clock_s: state.clock_s.unwrap_or(0.0),
        has_velocity_m_s: state.velocity.is_some(),
        velocity_m_s: state
            .velocity
            .map(|velocity| velocity.as_array())
            .unwrap_or([0.0; 3]),
        has_clock_rate_s_s: state.clock_rate_s_s.is_some(),
        clock_rate_s_s: state.clock_rate_s_s.unwrap_or(0.0),
        clock_event: state.flags.clock_event,
        clock_predicted: state.flags.clock_predicted,
        maneuver: state.flags.maneuver,
        orbit_predicted: state.flags.orbit_predicted,
    }
}

fn default_sp3_merge_options() -> SidereonSp3MergeOptions {
    let options = MergeOptions::default();
    SidereonSp3MergeOptions {
        position_tolerance_m: options.position_tolerance_m,
        clock_tolerance_s: options.clock_tolerance_s,
        min_agree: options.min_agree,
        clock_min_common: options.clock_min_common,
        combine: SidereonSp3MergeCombine::Mean as u32,
        precedence_scope: SidereonSp3MergePrecedenceScope::Cell as u32,
        outlier_reject_enabled: u8::from(options.outlier_reject.is_some()),
        outlier_reject_position_tolerance_m: options
            .outlier_reject
            .map(|guard| guard.position_tolerance_m)
            .unwrap_or(0.0),
        outlier_reject_clock_tolerance_s: options
            .outlier_reject
            .map(|guard| guard.clock_tolerance_s)
            .unwrap_or(0.0),
        target_epoch_interval_s_enabled: u8::from(options.target_epoch_interval_s.is_some()),
        target_epoch_interval_s: options.target_epoch_interval_s.unwrap_or(0.0),
        systems: ptr::null(),
        system_count: 0,
        asserted_frame_label_sets: ptr::null(),
        asserted_frame_label_set_count: 0,
        helmert_frame_reconciliation: u8::from(options.frame_reconciliation.helmert),
        provenance_mode: SidereonSp3ProvenanceMode::Off as u32,
        verify_continuity_enabled: u8::from(options.verify_continuity.is_some()),
        verify_continuity: continuity_options_to_c(
            options
                .verify_continuity
                .as_ref()
                .unwrap_or(&ContinuityOptions::for_orbit_class(OrbitClass::MeoGnss)),
        ),
    }
}

fn sp3_artifact_identity_to_c(
    fn_name: &str,
    artifact: &Sp3ArtifactIdentity,
) -> Result<SidereonSp3ArtifactIdentity, SidereonStatus> {
    Ok(SidereonSp3ArtifactIdentity {
        requested_identity: data_distribution::identity_to_c(
            fn_name,
            &artifact.requested_identity,
        )?,
        resolved_identity: data_distribution::identity_to_c(fn_name, &artifact.resolved_identity)?,
        distribution_source: match artifact.distribution_source {
            DistributionSource::Direct => SidereonDistributionSource::Direct as u32,
            DistributionSource::NasaCddis => SidereonDistributionSource::NasaCddis as u32,
            DistributionSource::LocalFile => SidereonDistributionSource::LocalFile as u32,
            DistributionSource::InMemory => SidereonDistributionSource::InMemory as u32,
        },
        official_filename: data_distribution::fixed_text(
            fn_name,
            "artifact.official_filename",
            &artifact.official_filename,
        )?,
        product_sha256: data_distribution::fixed_text(
            fn_name,
            "artifact.product_sha256",
            &artifact.product_sha256,
        )?,
        product_byte_length: artifact.product_byte_length,
        archive_sha256: data_distribution::fixed_text(
            fn_name,
            "artifact.archive_sha256",
            &artifact.archive_sha256,
        )?,
        archive_byte_length: artifact.archive_byte_length,
        compression: match artifact.compression {
            ArchiveCompression::None => SidereonArchiveCompression::None as u32,
            ArchiveCompression::Gzip => SidereonArchiveCompression::Gzip as u32,
            ArchiveCompression::UnixCompress => SidereonArchiveCompression::UnixCompress as u32,
        },
    })
}

fn sp3_artifact_identity_from_c(
    fn_name: &str,
    index: usize,
    artifact: &SidereonSp3ArtifactIdentity,
) -> Result<Sp3ArtifactIdentity, SidereonStatus> {
    let field = |name: &str| format!("contributors[{index}].{name}");
    Ok(Sp3ArtifactIdentity {
        requested_identity: data_distribution::identity_from_c(
            fn_name,
            &artifact.requested_identity,
        )?,
        resolved_identity: data_distribution::identity_from_c(
            fn_name,
            &artifact.resolved_identity,
        )?,
        distribution_source: data_distribution::source_from_c(
            fn_name,
            &field("distribution_source"),
            artifact.distribution_source,
        )?,
        official_filename: data_distribution::fixed_text_from_c(
            fn_name,
            &field("official_filename"),
            &artifact.official_filename,
        )?,
        product_sha256: data_distribution::fixed_text_from_c(
            fn_name,
            &field("product_sha256"),
            &artifact.product_sha256,
        )?,
        product_byte_length: artifact.product_byte_length,
        archive_sha256: data_distribution::fixed_text_from_c(
            fn_name,
            &field("archive_sha256"),
            &artifact.archive_sha256,
        )?,
        archive_byte_length: artifact.archive_byte_length,
        compression: data_distribution::compression_from_c(
            fn_name,
            &field("compression"),
            artifact.compression,
        )?,
    })
}

pub(crate) unsafe fn sp3_merge_options_from_c(
    fn_name: &str,
    options: *const SidereonSp3MergeOptions,
) -> Result<MergeOptions, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(MergeOptions::default());
    };
    if options.min_agree == 0 {
        set_last_error(format!("{fn_name}: min_agree must be at least 1"));
        return Err(SidereonStatus::InvalidArgument);
    }
    if options.clock_min_common == 0 {
        set_last_error(format!("{fn_name}: clock_min_common must be at least 1"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let combine = match options.combine {
        value if value == SidereonSp3MergeCombine::Mean as u32 => MergeCombine::Mean,
        value if value == SidereonSp3MergeCombine::Median as u32 => MergeCombine::Median,
        value if value == SidereonSp3MergeCombine::Precedence as u32 => MergeCombine::Precedence,
        _ => {
            set_last_error(format!("{fn_name}: invalid merge combine selector"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let precedence_scope = match options.precedence_scope {
        value if value == SidereonSp3MergePrecedenceScope::Cell as u32 => {
            MergePrecedenceScope::Cell
        }
        value if value == SidereonSp3MergePrecedenceScope::SatelliteArc as u32 => {
            MergePrecedenceScope::SatelliteArc
        }
        _ => {
            set_last_error(format!(
                "{fn_name}: invalid merge precedence scope selector"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let outlier_reject_enabled = data_distribution::bool_from_c(
        fn_name,
        "outlier_reject_enabled",
        options.outlier_reject_enabled,
    )?;
    let target_epoch_interval_s_enabled = data_distribution::bool_from_c(
        fn_name,
        "target_epoch_interval_s_enabled",
        options.target_epoch_interval_s_enabled,
    )?;
    let helmert_frame_reconciliation = data_distribution::bool_from_c(
        fn_name,
        "helmert_frame_reconciliation",
        options.helmert_frame_reconciliation,
    )?;
    let outlier_reject = if outlier_reject_enabled {
        Some(OutlierRejectOptions::new(
            options.outlier_reject_position_tolerance_m,
            options.outlier_reject_clock_tolerance_s,
        ))
    } else {
        None
    };
    // The engine validates the target: positive, finite and a whole number of
    // the 10-nanosecond ticks an SP3 interval states.
    let target_epoch_interval_s =
        target_epoch_interval_s_enabled.then_some(options.target_epoch_interval_s);
    let provenance =
        crate::sp3_merge_report::provenance_mode_from_c(fn_name, options.provenance_mode)?;
    let verify_continuity = if data_distribution::bool_from_c(
        fn_name,
        "verify_continuity_enabled",
        options.verify_continuity_enabled,
    )? {
        Some(continuity_options_struct_from_c(
            fn_name,
            &options.verify_continuity,
        )?)
    } else {
        None
    };
    let systems = if options.system_count == 0 {
        None
    } else {
        let raw_systems = require_slice(options.systems, options.system_count, fn_name, "systems")?;
        let mut systems = BTreeSet::new();
        for (idx, system) in raw_systems.iter().copied().enumerate() {
            systems.insert(gnss_system_from_c_code(
                fn_name,
                &format!("systems[{idx}]"),
                system,
            )?);
        }
        Some(systems)
    };
    let asserted_equivalent_label_sets = if options.asserted_frame_label_set_count == 0 {
        Vec::new()
    } else {
        parse_sp3_asserted_frame_label_sets(
            fn_name,
            options.asserted_frame_label_sets,
            options.asserted_frame_label_set_count,
        )?
    };

    // Mutate-a-default rather than a struct literal: MergeOptions is
    // non-exhaustive.
    let mut merge_options = MergeOptions::default();
    merge_options.position_tolerance_m = options.position_tolerance_m;
    merge_options.clock_tolerance_s = options.clock_tolerance_s;
    merge_options.min_agree = options.min_agree;
    merge_options.clock_min_common = options.clock_min_common;
    merge_options.combine = combine;
    merge_options.precedence_scope = precedence_scope;
    merge_options.outlier_reject = outlier_reject;
    merge_options.target_epoch_interval_s = target_epoch_interval_s;
    merge_options.systems = systems;
    let mut frame_reconciliation = Sp3FrameReconciliationOptions::default();
    frame_reconciliation.asserted_equivalent_label_sets = asserted_equivalent_label_sets;
    frame_reconciliation.helmert = helmert_frame_reconciliation;
    merge_options.frame_reconciliation = frame_reconciliation;
    merge_options.provenance = provenance;
    merge_options.verify_continuity = verify_continuity;
    Ok(merge_options)
}

unsafe fn parse_sp3_asserted_frame_label_sets(
    fn_name: &str,
    label_sets: *const SidereonSp3FrameLabelSet,
    label_set_count: usize,
) -> Result<Vec<Sp3FrameLabelSet>, SidereonStatus> {
    let raw_sets = require_slice(
        label_sets,
        label_set_count,
        fn_name,
        "asserted_frame_label_sets",
    )?;
    let mut parsed = Vec::with_capacity(raw_sets.len());
    for (set_idx, set) in raw_sets.iter().copied().enumerate() {
        if set.label_count < 2 {
            set_last_error(format!(
                "{fn_name}: asserted_frame_label_sets[{set_idx}] must contain at least two labels"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        let raw_labels = require_slice(
            set.labels,
            set.label_count,
            fn_name,
            &format!("asserted_frame_label_sets[{set_idx}].labels"),
        )?;
        let mut labels = Vec::with_capacity(raw_labels.len());
        for (label_idx, label) in raw_labels.iter().copied().enumerate() {
            let label = parse_bounded_c_string(
                fn_name,
                &format!("asserted_frame_label_sets[{set_idx}].labels[{label_idx}]"),
                label,
                SP3_FRAME_LABEL_MAX_BYTES,
            )?;
            let label = label.trim().to_string();
            if label.is_empty() {
                set_last_error(format!(
                    "{fn_name}: asserted_frame_label_sets[{set_idx}].labels[{label_idx}] is empty"
                ));
                return Err(SidereonStatus::InvalidArgument);
            }
            labels.push(label);
        }
        parsed.push(Sp3FrameLabelSet::new(labels));
    }
    Ok(parsed)
}

fn sp3_merge_flag_to_c(flag: &MergeFlag) -> SidereonSp3MergeFlag {
    SidereonSp3MergeFlag {
        epoch_j2000_seconds: instant_to_j2000_seconds(&flag.epoch).unwrap_or(f64::NAN),
        epoch: crate::rinex_clock::instant_to_clock_epoch(&flag.epoch),
        sat_id: satellite_token(flag.satellite),
        source_count: flag.sources.len(),
    }
}

fn sp3_frame_reconciliation_to_c(
    value: &sidereon_core::ephemeris::Sp3FrameReconciliation,
) -> SidereonSp3FrameReconciliation {
    let parameters = value.parameters;
    let rates = value.rates;
    let epoch_year_span = value.epoch_year_span;
    SidereonSp3FrameReconciliation {
        source_index: value.source_index,
        source_label_len: value.source_label.len(),
        target_label_len: value.target_label.len(),
        method: match value.method {
            Sp3FrameReconciliationMethod::AssertedEquivalence => {
                SidereonSp3FrameReconciliationMethod::AssertedEquivalence
            }
            Sp3FrameReconciliationMethod::Helmert => SidereonSp3FrameReconciliationMethod::Helmert,
        },
        asserted_label_count: value.asserted_label_set.as_ref().map(Vec::len).unwrap_or(0),
        source_frame_present: value.source_frame.is_some(),
        source_frame: value
            .source_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        target_frame_present: value.target_frame.is_some(),
        target_frame: value
            .target_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_frame_present: value.catalog_source_frame.is_some()
            && value.catalog_target_frame.is_some(),
        catalog_source_frame: value
            .catalog_source_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_target_frame: value
            .catalog_target_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_inverse: value.catalog_inverse,
        reference_epoch_year_present: value.reference_epoch_year.is_some(),
        reference_epoch_year: value.reference_epoch_year.unwrap_or(0.0),
        parameters_present: parameters.is_some(),
        translation_mm: parameters
            .map(|parameters| parameters.translation_mm)
            .unwrap_or([0.0; 3]),
        scale_ppb: parameters
            .map(|parameters| parameters.scale_ppb)
            .unwrap_or(0.0),
        rotation_mas: parameters
            .map(|parameters| parameters.rotation_mas)
            .unwrap_or([0.0; 3]),
        rates_present: rates.is_some(),
        translation_mm_per_year: rates
            .map(|rates| rates.translation_mm_per_year)
            .unwrap_or([0.0; 3]),
        scale_ppb_per_year: rates.map(|rates| rates.scale_ppb_per_year).unwrap_or(0.0),
        rotation_mas_per_year: rates
            .map(|rates| rates.rotation_mas_per_year)
            .unwrap_or([0.0; 3]),
        provenance_len: value.provenance.as_ref().map(String::len).unwrap_or(0),
        epoch_year_span_present: epoch_year_span.is_some(),
        epoch_year_start: epoch_year_span.map(|span| span[0]).unwrap_or(0.0),
        epoch_year_end: epoch_year_span.map(|span| span[1]).unwrap_or(0.0),
        records_affected: value.records_affected,
        identity: value.identity,
    }
}

fn zero_sp3_frame_reconciliation() -> SidereonSp3FrameReconciliation {
    SidereonSp3FrameReconciliation {
        source_index: 0,
        source_label_len: 0,
        target_label_len: 0,
        method: SidereonSp3FrameReconciliationMethod::AssertedEquivalence,
        asserted_label_count: 0,
        source_frame_present: false,
        source_frame: 0,
        target_frame_present: false,
        target_frame: 0,
        catalog_frame_present: false,
        catalog_source_frame: 0,
        catalog_target_frame: 0,
        catalog_inverse: false,
        reference_epoch_year_present: false,
        reference_epoch_year: 0.0,
        parameters_present: false,
        translation_mm: [0.0; 3],
        scale_ppb: 0.0,
        rotation_mas: [0.0; 3],
        rates_present: false,
        translation_mm_per_year: [0.0; 3],
        scale_ppb_per_year: 0.0,
        rotation_mas_per_year: [0.0; 3],
        provenance_len: 0,
        epoch_year_span_present: false,
        epoch_year_start: 0.0,
        epoch_year_end: 0.0,
        records_affected: 0,
        identity: false,
    }
}

fn sp3_terrestrial_frame_to_c(value: sidereon_core::frame_catalog::TerrestrialFrame) -> u32 {
    match value {
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2020 => {
            SidereonTerrestrialFrame::Itrf2020 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2014 => {
            SidereonTerrestrialFrame::Itrf2014 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2008 => {
            SidereonTerrestrialFrame::Itrf2008 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Etrf2020 => {
            SidereonTerrestrialFrame::Etrf2020 as u32
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn sp3_merge_report_reconciliation_bytes<'a, F>(
    fn_name: &str,
    report: *const SidereonSp3MergeReport,
    index: usize,
    select: F,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus
where
    F: FnOnce(&'a sidereon_core::ephemeris::Sp3FrameReconciliation) -> &'a [u8],
{
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let report = c_try!(require_ref(report, fn_name, "report"));
        let Some(reconciliation) = report.inner.frame_reconciliations.get(index) else {
            set_last_error(format!("{fn_name}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            select(reconciliation),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn sp3_epoch_agreement_to_c(agg: &EpochAgreement) -> SidereonSp3EpochAgreement {
    SidereonSp3EpochAgreement {
        epoch_j2000_seconds: instant_to_j2000_seconds(&agg.epoch).unwrap_or(f64::NAN),
        epoch: crate::rinex_clock::instant_to_clock_epoch(&agg.epoch),
        satellites: agg.satellites,
        position_rms_present: agg.position_rms_m.is_some(),
        position_rms_m: agg.position_rms_m.unwrap_or(f64::NAN),
        position_max_present: agg.position_max_m.is_some(),
        position_max_m: agg.position_max_m.unwrap_or(f64::NAN),
        clock_rms_present: agg.clock_rms_s.is_some(),
        clock_rms_s: agg.clock_rms_s.unwrap_or(f64::NAN),
        clock_max_present: agg.clock_max_s.is_some(),
        clock_max_s: agg.clock_max_s.unwrap_or(f64::NAN),
    }
}

/// The agreement entry an unread or refused query leaves behind: nothing is
/// present, and every number is NaN rather than a zero that reads as a
/// measured agreement.
fn empty_sp3_epoch_agreement() -> SidereonSp3EpochAgreement {
    SidereonSp3EpochAgreement {
        epoch_j2000_seconds: f64::NAN,
        epoch: crate::rinex_clock::absent_clock_epoch(),
        satellites: 0,
        position_rms_present: false,
        position_rms_m: f64::NAN,
        position_max_present: false,
        position_max_m: f64::NAN,
        clock_rms_present: false,
        clock_rms_s: f64::NAN,
        clock_max_present: false,
        clock_max_s: f64::NAN,
    }
}

fn sp3_agreement_summary_to_c(report: &MergeReport) -> SidereonSp3AgreementSummary {
    let pos_rms = report.position_agreement_rms_m();
    let pos_max = report.position_agreement_max_m();
    let clk_rms = report.clock_agreement_rms_s();
    let clk_max = report.clock_agreement_max_s();
    let single_source_fraction = report.single_source_fraction();
    SidereonSp3AgreementSummary {
        position_rms_present: pos_rms.is_some(),
        position_rms_m: pos_rms.unwrap_or(f64::NAN),
        position_max_present: pos_max.is_some(),
        position_max_m: pos_max.unwrap_or(f64::NAN),
        clock_rms_present: clk_rms.is_some(),
        clock_rms_s: clk_rms.unwrap_or(f64::NAN),
        clock_max_present: clk_max.is_some(),
        clock_max_s: clk_max.unwrap_or(f64::NAN),
        single_source_fraction_present: single_source_fraction.is_some(),
        single_source_fraction: single_source_fraction.unwrap_or(f64::NAN),
    }
}

fn empty_sp3_agreement_summary() -> SidereonSp3AgreementSummary {
    SidereonSp3AgreementSummary {
        position_rms_present: false,
        position_rms_m: f64::NAN,
        position_max_present: false,
        position_max_m: f64::NAN,
        clock_rms_present: false,
        clock_rms_s: f64::NAN,
        clock_max_present: false,
        clock_max_s: f64::NAN,
        single_source_fraction_present: false,
        single_source_fraction: f64::NAN,
    }
}

fn no_sp3_write_error() -> SidereonSp3WriteError {
    SidereonSp3WriteError {
        kind: SidereonSp3WriteErrorKind::None,
        has_field: false,
        has_text_value: false,
        has_sat_id: false,
        sat_id: SidereonSatelliteToken {
            bytes: [0; SATELLITE_TOKEN_C_BYTES],
        },
        has_epoch_index: false,
        epoch_index: 0,
        has_comment_index: false,
        comment_index: 0,
        has_columns: false,
        columns: 0,
        has_decimals: false,
        decimals: 0,
        has_integer_value: false,
        integer_value: 0,
        has_number: false,
        number: f64::NAN,
        has_year: false,
        year: 0,
        has_field_seconds: false,
        field_seconds: f64::NAN,
        has_residual_s: false,
        residual_s: f64::NAN,
        has_epoch_time_scale: false,
        epoch_time_scale: 0,
        has_header_time_scale: false,
        header_time_scale: 0,
        has_time_system: false,
        time_system: [0; SIDEREON_SP3_TIME_SYSTEM_C_BYTES],
        has_declared_epochs: false,
        declared_epochs: 0,
        has_epochs: false,
        epochs: 0,
        has_entries: false,
        entries: 0,
        has_satellites: false,
        satellites: 0,
        has_codes: false,
        codes: 0,
        has_stored: false,
        stored: f64::NAN,
        has_native: false,
        native: f64::NAN,
        has_column_value: false,
        column_value: f64::NAN,
        has_exponent: false,
        exponent: 0,
    }
}

/// A refused SP3 write, split into its fixed-width detail and its two text
/// parts.
struct Sp3WriteRefusal {
    error: SidereonSp3WriteError,
    field: Option<&'static str>,
    text_value: Option<String>,
}

impl Sp3WriteRefusal {
    fn new(kind: SidereonSp3WriteErrorKind) -> Self {
        let mut error = no_sp3_write_error();
        error.kind = kind;
        Self {
            error,
            field: None,
            text_value: None,
        }
    }

    fn field(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    fn text_value(mut self, value: &str) -> Self {
        self.text_value = Some(value.to_owned());
        self
    }

    fn sat(mut self, sat: GnssSatelliteId) -> Self {
        self.error.has_sat_id = true;
        self.error.sat_id = satellite_token(sat);
        self
    }

    fn epoch_index(mut self, epoch_index: usize) -> Self {
        self.error.has_epoch_index = true;
        self.error.epoch_index = epoch_index;
        self
    }

    fn columns(mut self, columns: usize) -> Self {
        self.error.has_columns = true;
        self.error.columns = columns;
        self
    }

    fn decimals(mut self, decimals: usize) -> Self {
        self.error.has_decimals = true;
        self.error.decimals = decimals;
        self
    }

    fn column_value(mut self, column_value: f64) -> Self {
        self.error.has_column_value = true;
        self.error.column_value = column_value;
        self
    }
}

/// Map every field of an engine SP3 write refusal into the C record. Nothing is
/// rounded or defaulted: a field the variant does not carry keeps its absent
/// flag, and an engine residual of NaN, which means no candidate record could
/// be read back, stays absent rather than becoming a measured value.
fn sp3_write_refusal(error: &Sp3WriteError) -> Sp3WriteRefusal {
    use SidereonSp3WriteErrorKind as Kind;
    match error {
        Sp3WriteError::TextNotColumnSafe { field, value } => {
            Sp3WriteRefusal::new(Kind::TextNotColumnSafe)
                .field(field)
                .text_value(value)
        }
        Sp3WriteError::TextNotColumnStable { field, value } => {
            Sp3WriteRefusal::new(Kind::TextNotColumnStable)
                .field(field)
                .text_value(value)
        }
        Sp3WriteError::BlankDescriptor { field, value } => {
            Sp3WriteRefusal::new(Kind::BlankDescriptor)
                .field(field)
                .text_value(value)
        }
        Sp3WriteError::EmptyComment { index, value } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::EmptyComment).text_value(value);
            refusal.error.has_comment_index = true;
            refusal.error.comment_index = *index;
            refusal
        }
        Sp3WriteError::TextTooWide {
            field,
            columns,
            value,
        } => Sp3WriteRefusal::new(Kind::TextTooWide)
            .field(field)
            .columns(*columns)
            .text_value(value),
        Sp3WriteError::IntegerTooWide {
            field,
            columns,
            value,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::IntegerTooWide)
                .field(field)
                .columns(*columns);
            refusal.error.has_integer_value = true;
            refusal.error.integer_value = *value;
            refusal
        }
        Sp3WriteError::NonFinite { field } => Sp3WriteRefusal::new(Kind::NonFinite).field(field),
        Sp3WriteError::NumberTooWide {
            field,
            columns,
            decimals,
            value,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::NumberTooWide)
                .field(field)
                .columns(*columns)
                .decimals(*decimals);
            refusal.error.has_number = true;
            refusal.error.number = *value;
            refusal
        }
        Sp3WriteError::PrecisionNotRepresentable {
            field,
            columns,
            decimals,
            value,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::PrecisionNotRepresentable)
                .field(field)
                .columns(*columns)
                .decimals(*decimals);
            refusal.error.has_number = true;
            refusal.error.number = *value;
            refusal
        }
        Sp3WriteError::YearNotRepresentable { epoch_index, year } => {
            let mut refusal =
                Sp3WriteRefusal::new(Kind::YearNotRepresentable).epoch_index(*epoch_index);
            refusal.error.has_year = true;
            refusal.error.year = *year;
            refusal
        }
        Sp3WriteError::EpochNotRestatable {
            epoch_index,
            field_seconds,
            residual_s,
        } => {
            let mut refusal =
                Sp3WriteRefusal::new(Kind::EpochNotRestatable).epoch_index(*epoch_index);
            refusal.error.has_field_seconds = true;
            refusal.error.field_seconds = *field_seconds;
            if !residual_s.is_nan() {
                refusal.error.has_residual_s = true;
                refusal.error.residual_s = *residual_s;
            }
            refusal
        }
        Sp3WriteError::EpochTimeScaleMismatch {
            epoch_index,
            epoch_scale,
            header_scale,
        } => {
            let mut refusal =
                Sp3WriteRefusal::new(Kind::EpochTimeScaleMismatch).epoch_index(*epoch_index);
            refusal.error.has_epoch_time_scale = true;
            refusal.error.epoch_time_scale = time_scale_to_c_code(*epoch_scale);
            refusal.error.has_header_time_scale = true;
            refusal.error.header_time_scale = time_scale_to_c_code(*header_scale);
            refusal
        }
        Sp3WriteError::HeaderTimeScaleMismatch {
            time_system,
            time_scale,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::HeaderTimeScaleMismatch);
            refusal.error.has_time_system = true;
            refusal.error.time_system =
                fixed_c_chars::<SIDEREON_SP3_TIME_SYSTEM_C_BYTES>(time_system.label());
            refusal.error.has_header_time_scale = true;
            refusal.error.header_time_scale = time_scale_to_c_code(*time_scale);
            refusal
        }
        Sp3WriteError::EpochCountMismatch { declared, epochs } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::EpochCountMismatch);
            refusal.error.has_declared_epochs = true;
            refusal.error.declared_epochs = *declared;
            refusal.error.has_epochs = true;
            refusal.error.epochs = *epochs;
            refusal
        }
        Sp3WriteError::AccuracyCodeCountMismatch { satellites, codes } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::AccuracyCodeCountMismatch);
            refusal.error.has_satellites = true;
            refusal.error.satellites = *satellites;
            refusal.error.has_codes = true;
            refusal.error.codes = *codes;
            refusal
        }
        Sp3WriteError::DuplicateSatellite { sat } => {
            Sp3WriteRefusal::new(Kind::DuplicateSatellite).sat(*sat)
        }
        Sp3WriteError::EpochArrayLengthMismatch {
            field,
            epochs,
            entries,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::EpochArrayLengthMismatch).field(field);
            refusal.error.has_epochs = true;
            refusal.error.epochs = *epochs;
            refusal.error.has_entries = true;
            refusal.error.entries = *entries;
            refusal
        }
        Sp3WriteError::UndeclaredSatelliteRecord { sat, epoch_index } => {
            Sp3WriteRefusal::new(Kind::UndeclaredSatelliteRecord)
                .sat(*sat)
                .epoch_index(*epoch_index)
        }
        Sp3WriteError::ConflictingRecords { sat, epoch_index } => {
            Sp3WriteRefusal::new(Kind::ConflictingRecords)
                .sat(*sat)
                .epoch_index(*epoch_index)
        }
        Sp3WriteError::VelocityStateInPositionProduct {
            field,
            sat,
            epoch_index,
        } => Sp3WriteRefusal::new(Kind::VelocityStateInPositionProduct)
            .field(field)
            .sat(*sat)
            .epoch_index(*epoch_index),
        Sp3WriteError::RecordValueNonFinite {
            field,
            sat,
            epoch_index,
        } => Sp3WriteRefusal::new(Kind::RecordValueNonFinite)
            .field(field)
            .sat(*sat)
            .epoch_index(*epoch_index),
        Sp3WriteError::RecordValueTooWide {
            field,
            sat,
            epoch_index,
            columns,
            decimals,
            column_value,
        } => Sp3WriteRefusal::new(Kind::RecordValueTooWide)
            .field(field)
            .sat(*sat)
            .epoch_index(*epoch_index)
            .columns(*columns)
            .decimals(*decimals)
            .column_value(*column_value),
        Sp3WriteError::RecordValueNotRepresentable {
            field,
            sat,
            epoch_index,
            columns,
            decimals,
            stored,
            column_value,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::RecordValueNotRepresentable)
                .field(field)
                .sat(*sat)
                .epoch_index(*epoch_index)
                .columns(*columns)
                .decimals(*decimals)
                .column_value(*column_value);
            refusal.error.has_stored = true;
            refusal.error.stored = *stored;
            refusal
        }
        Sp3WriteError::RecordReadsAsAbsent {
            field,
            sat,
            epoch_index,
            column_value,
        } => Sp3WriteRefusal::new(Kind::RecordReadsAsAbsent)
            .field(field)
            .sat(*sat)
            .epoch_index(*epoch_index)
            .column_value(*column_value),
        Sp3WriteError::RecordFieldsDisagree {
            field,
            sat,
            epoch_index,
            stored,
            native,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::RecordFieldsDisagree)
                .field(field)
                .sat(*sat)
                .epoch_index(*epoch_index);
            if let Some(stored) = stored {
                refusal.error.has_stored = true;
                refusal.error.stored = *stored;
            }
            if let Some(native) = native {
                refusal.error.has_native = true;
                refusal.error.native = *native;
            }
            refusal
        }
        Sp3WriteError::SatelliteNotRepresentable { sat } => {
            Sp3WriteRefusal::new(Kind::SatelliteNotRepresentable).sat(*sat)
        }
        Sp3WriteError::AccuracyNotRepresentable {
            sat,
            epoch_index,
            component,
            exponent,
        } => {
            let mut refusal = Sp3WriteRefusal::new(Kind::AccuracyNotRepresentable)
                .sat(*sat)
                .epoch_index(*epoch_index)
                .field(component);
            if let Some(exponent) = exponent {
                refusal.error.has_exponent = true;
                refusal.error.exponent = *exponent;
            }
            refusal
        }
        Sp3WriteError::AccuracyRecordMismatch { sat, epoch_index } => {
            Sp3WriteRefusal::new(Kind::AccuracyRecordMismatch)
                .sat(*sat)
                .epoch_index(*epoch_index)
        }
        Sp3WriteError::AccuracyBasisMissing { sat, epoch_index } => {
            Sp3WriteRefusal::new(Kind::AccuracyBasisMissing)
                .sat(*sat)
                .epoch_index(*epoch_index)
        }
        // `Sp3WriteError` is non-exhaustive: a refusal a later engine adds
        // reads as Unknown, with the engine's text in the message.
        _ => Sp3WriteRefusal::new(Kind::Unknown),
    }
}

/// Write `sp3` and record the whole outcome: the text, or the typed refusal and
/// its text parts. Nothing partial is kept from a refused write.
fn sp3_write_result(fn_name: &str, sp3: &Sp3) -> SidereonSp3WriteResult {
    match sp3.to_sp3_string() {
        Ok(text) => SidereonSp3WriteResult {
            outcome: SidereonSp3WriteOutcome {
                is_ok: true,
                status: SidereonStatus::Ok,
                error: no_sp3_write_error(),
            },
            text,
            message: String::new(),
            field: String::new(),
            text_value: String::new(),
        },
        Err(err) => {
            let mut refusal = sp3_write_refusal(&err);
            refusal.error.has_field = refusal.field.is_some();
            refusal.error.has_text_value = refusal.text_value.is_some();
            SidereonSp3WriteResult {
                outcome: SidereonSp3WriteOutcome {
                    is_ok: false,
                    status: SidereonStatus::InvalidArgument,
                    error: refusal.error,
                },
                text: String::new(),
                message: format!("{fn_name}: {err}"),
                field: refusal.field.unwrap_or_default().to_owned(),
                text_value: refusal.text_value.unwrap_or_default(),
            }
        }
    }
}

unsafe fn visibility_options_from_c(
    fn_name: &str,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
) -> Result<VisibilityOptions, SidereonStatus> {
    let raw = require_slice(systems, systems_len, fn_name, "systems")?;
    let systems = if raw.is_empty() {
        None
    } else {
        let mut set = BTreeSet::new();
        for code in raw {
            set.insert(gnss_system_from_c_code(fn_name, "systems", *code)?);
        }
        Some(set)
    };
    let mut o = VisibilityOptions::default();
    o.elevation_mask_deg = elevation_mask_deg;
    o.systems = systems;
    Ok(o)
}

fn geometry_visible_to_c(sat: &GeometryVisibleSatellite) -> SidereonGeometryVisible {
    SidereonGeometryVisible {
        satellite: satellite_token(sat.satellite),
        elevation_deg: sat.elevation_deg,
        azimuth_deg: sat.azimuth_deg,
    }
}

fn visibility_series_point_to_c(point: &VisibilitySeriesPoint) -> SidereonVisibilitySeriesPoint {
    SidereonVisibilitySeriesPoint {
        step_index: point.step_index,
        n_visible: point.n_visible,
    }
}

fn visibility_pass_to_c(pass: &VisibilityPass) -> SidereonVisibilityPass {
    SidereonVisibilityPass {
        satellite: satellite_token(pass.satellite),
        rise_step_index: pass.rise_step_index,
        set_step_index: pass.set_step_index,
        peak_elevation_deg: pass.peak_elevation_deg,
        peak_step_index: pass.peak_step_index,
    }
}

fn precise_sample_to_c(sample: &PreciseEphemerisSample) -> SidereonPreciseEphemerisSample {
    SidereonPreciseEphemerisSample {
        sat: satellite_token(sample.sat),
        time_scale: time_scale_to_c_code(sample.epoch.scale),
        epoch_j2000_s: instant_to_j2000_seconds(&sample.epoch).unwrap_or(f64::NAN),
        position_ecef_m: sample.position_ecef_m,
        has_clock_s: sample.clock_s.is_some(),
        clock_s: sample.clock_s.unwrap_or(0.0),
        clock_event: sample.clock_event,
    }
}

#[cfg(test)]
mod exact_sp3_c_tests {
    use super::*;

    #[test]
    fn exact_coverage_discriminants_and_mapping_are_stable() {
        assert_eq!(SidereonExactSp3Coverage::HalfOpen as u32, 0);
        assert_eq!(SidereonExactSp3Coverage::Inclusive as u32, 1);
        assert_eq!(
            exact_sp3_coverage_to_c(CoreExactSp3Coverage::HalfOpen),
            SidereonExactSp3Coverage::HalfOpen
        );
        assert_eq!(
            exact_sp3_coverage_to_c(CoreExactSp3Coverage::Inclusive),
            SidereonExactSp3Coverage::Inclusive
        );
    }
}

#[cfg(test)]
mod window_continuity_c_tests {
    use std::ptr;

    use super::*;

    #[test]
    fn unusable_sample_json_retains_original_index_and_reason() {
        let sat = "G01".parse().expect("satellite");
        let defect = ContinuityDefect::UnusableSample {
            sat,
            sample_index: 7,
            epoch_j2000_s: None,
            reason: UnusableSampleReason::EpochNotPlaced,
        };
        let value = continuity_defect_json(&defect);
        assert_eq!(value["kind"], "unusable_sample");
        assert_eq!(value["sample_index"], 7);
        assert!(value["epoch_j2000_s"].is_null());
        assert_eq!(value["reason"], "epoch_not_placed");
    }

    fn product_with_seam_jump() -> (SidereonSp3, Vec<f64>, f64) {
        let product = Sp3::parse(include_bytes!(
            "../tests/fixtures/sp3/COD0MGXFIN_20201770000_01D_05M_ORB.SP3"
        ))
        .expect("parse established C SP3 fixture");
        let mut epoch_index = -1_i32;
        let mut shifted = 0_usize;
        let mut changed = String::new();
        let source_text = product
            .to_sp3_string()
            .expect("write established C SP3 fixture");
        for source_line in source_text.lines() {
            let mut line = source_line.to_string();
            if line.starts_with("* ") {
                epoch_index += 1;
            } else if epoch_index >= 145 && line.starts_with("PG01") {
                let x_km = line[4..18].trim().parse::<f64>().expect("G01 x value");
                line.replace_range(4..18, &format!("{:14.6}", x_km + 3_000.0));
                shifted += 1;
            }
            changed.push_str(&line);
            changed.push('\n');
        }
        assert!(shifted > 0);

        let inner = Sp3::parse(changed.as_bytes()).expect("parse seam-injected fixture");
        let epochs = inner.epochs_j2000_seconds();
        let seam = epochs[144];
        (SidereonSp3 { inner }, epochs, seam)
    }

    fn verdict_json(
        sp3: &SidereonSp3,
        from_j2000_s: f64,
        through_j2000_s: f64,
    ) -> serde_json::Value {
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json(
                    sp3,
                    0,
                    -1.0,
                    from_j2000_s,
                    through_j2000_s,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert!(required > 0);

        let mut bytes = vec![0_u8; required];
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json(
                    sp3,
                    0,
                    -1.0,
                    from_j2000_s,
                    through_j2000_s,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        serde_json::from_slice(&bytes[..written]).expect("verdict JSON")
    }

    /// sidereon-core's own verdict for the window, in the binding's JSON form.
    fn core_verdict_json(
        sp3: &SidereonSp3,
        from_j2000_s: f64,
        through_j2000_s: f64,
    ) -> serde_json::Value {
        let options = continuity_options_from_c("test", 0, -1.0, 0.0).expect("options");
        let window = EpochWindow::new(from_j2000_s, through_j2000_s).expect("window");
        let stencil = StencilExtent::for_sp3(&sp3.inner).expect("stencil");
        let report = check_continuity(&sp3.inner.precise_ephemeris_samples(), &options)
            .expect("valid continuity options");
        window_continuity_verdict_json(report.verdict_for_window(window, stencil))
    }

    fn merge_verdict_json(
        report: &SidereonSp3MergeReport,
        from_j2000_s: f64,
        through_j2000_s: f64,
    ) -> serde_json::Value {
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge_report_continuity_verdict_json(
                    report,
                    from_j2000_s,
                    through_j2000_s,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert!(required > 0);

        let mut bytes = vec![0_u8; required];
        assert_eq!(
            unsafe {
                sidereon_sp3_merge_report_continuity_verdict_json(
                    report,
                    from_j2000_s,
                    through_j2000_s,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        serde_json::from_slice(&bytes[..written]).expect("merge verdict JSON")
    }

    #[test]
    fn c_window_mapping_covers_inside_straddling_and_stencil_boundary_cases() {
        let (sp3, epochs, seam) = product_with_seam_jump();
        let mut before_s = f64::NAN;
        let mut after_s = f64::NAN;
        assert_eq!(
            unsafe { sidereon_sp3_stencil_extent(&sp3, &mut before_s, &mut after_s) },
            SidereonStatus::Ok
        );
        let core_stencil = StencilExtent::for_sp3(&sp3.inner).expect("core stencil");
        assert_eq!(before_s.to_bits(), core_stencil.before_s().to_bits());
        assert_eq!(after_s.to_bits(), core_stencil.after_s().to_bits());

        // A window well inside the arc, one straddling the injected seam, one
        // whose stencil just reaches it and one whose stencil just misses it:
        // each verdict is sidereon-core's own.
        for (from, through) in [
            (epochs[24], epochs[72]),
            (seam - 600.0, seam + 600.0),
            (seam - 7_200.0, seam - after_s),
            (seam - 7_200.0, seam - after_s - 0.001),
        ] {
            let verdict = verdict_json(&sp3, from, through);
            assert_eq!(verdict, core_verdict_json(&sp3, from, through));
        }
        // The seam window refuses and the inside window accepts, so the four
        // windows cover both decisions.
        assert_eq!(
            verdict_json(&sp3, seam - 600.0, seam + 600.0)["accepted"],
            false
        );
        assert_eq!(verdict_json(&sp3, epochs[24], epochs[72])["accepted"], true);
    }

    #[test]
    fn c_merge_window_mapping_retains_influencing_and_global_splices() {
        let (sp3, _, seam) = product_with_seam_jump();
        let mut options = MergeOptions::default();
        options.min_agree = 1;
        options.clock_min_common = 1;
        let (merged_inner, mut report_inner) =
            merge(&[sp3.inner], &options).expect("merge established fixture");
        let sat = merged_inner.satellites()[0];
        let defect = ContinuityDefect::SpeedBound {
            sat,
            from_j2000_s: seam,
            to_j2000_s: seam + 300.0,
            interval_s: 300.0,
            displacement_m: 3_000_000.0,
            implied_speed_m_s: 10_000.0,
            bound_m_s: 6_000.0,
        };
        report_inner.continuity = Some(sidereon_core::ephemeris::MergeContinuityReport {
            report: sidereon_core::ephemeris::ContinuityReport {
                defects: vec![defect.clone()],
                ..sidereon_core::ephemeris::ContinuityReport::default()
            },
            // A speed-bound finding rests on both ends of its pair, each with
            // the selection the merge recorded; sidereon-core decides influence
            // from these cells, so the violation names them as a merge does.
            violations: vec![MergeContinuityViolation {
                defect,
                from_sources: vec![0],
                to_sources: vec![1],
                cells: vec![
                    sidereon_core::ephemeris::MergeContinuityCell {
                        epoch_j2000_s: seam,
                        role: sidereon_core::ephemeris::MergeContinuityCellRole::PairEnd,
                        selection: Some(sidereon_core::ephemeris::CellSelection::SingleSource {
                            source: 0,
                        }),
                    },
                    sidereon_core::ephemeris::MergeContinuityCell {
                        epoch_j2000_s: seam + 300.0,
                        role: sidereon_core::ephemeris::MergeContinuityCellRole::PairEnd,
                        selection: Some(sidereon_core::ephemeris::CellSelection::SingleSource {
                            source: 1,
                        }),
                    },
                ],
                sources: vec![0, 1],
                crosses_contributors: true,
            }],
            nodes: sidereon_core::ephemeris::InterpolationNodes::for_sp3(&merged_inner),
        });
        let epoch_agreement = report_inner.per_epoch_agreement();
        let report = SidereonSp3MergeReport {
            inner: report_inner,
            epoch_agreement,
        };

        let verdict = merge_verdict_json(&report, seam - 600.0, seam + 600.0);
        // sidereon-core's own verdict for the same report and window.
        let expected = report
            .inner
            .continuity_verdict_for_window(
                EpochWindow::new(seam - 600.0, seam + 600.0).expect("window"),
            )
            .map(window_continuity_verdict_json)
            .expect("core verdict");
        assert_eq!(verdict, expected);
        // The splice the report carries crosses the window, so it influences.
        assert_eq!(verdict["influencing_splices"], verdict["all_splices"]);
        // Every field of the violation reaches the JSON.
        let splice = &verdict["all_splices"][0];
        assert_eq!(splice["defect"]["interval_s"], 300.0);
        assert_eq!(splice["defect"]["displacement_m"], 3_000_000.0);
        assert_eq!(splice["defect"]["implied_speed_m_s"], 10_000.0);
        assert_eq!(splice["defect"]["bound_m_s"], 6_000.0);
        assert_eq!(splice["sources"], serde_json::json!([0, 1]));
        assert_eq!(
            splice["cells"],
            serde_json::json!([
                {
                    "epoch_j2000_s": seam,
                    "role": "pair_end",
                    "selection": {"kind": "single_source", "source": 0},
                },
                {
                    "epoch_j2000_s": seam + 300.0,
                    "role": "pair_end",
                    "selection": {"kind": "single_source", "source": 1},
                },
            ])
        );
        assert_eq!(verdict["accepted"], false);
    }

    #[test]
    fn c_window_mapping_rejects_invalid_window_and_selector() {
        let (sp3, epochs, _) = product_with_seam_jump();
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json(
                    &sp3,
                    0,
                    -1.0,
                    epochs[1],
                    epochs[0],
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));

        written = usize::MAX;
        required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json(
                    &sp3,
                    99,
                    -1.0,
                    epochs[0],
                    epochs[1],
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((written, required), (0, 0));
    }
}

#[cfg(test)]
mod sp3_interpolation_c_tests {
    use std::ffi::CString;
    use std::ptr;

    use super::*;

    const GAPPED_SP3_BYTES: &[u8] =
        include_bytes!("../tests/fixtures/sp3/GAP_G01_20201760000_15M.sp3");
    const HOLE_MIDPOINT_J2000_S: f64 = 646_260_300.0;

    /// sidereon-core's default gap threshold factor.
    fn default_factor() -> f64 {
        Sp3InterpolationOptions::default().gap_threshold_factor()
    }

    fn g01() -> GnssSatelliteId {
        "G01".parse().expect("G01")
    }

    /// Compare one state the C route wrote with sidereon-core's own state
    /// batch from the same source at the hole midpoint.
    fn assert_state_matches(
        source: &dyn ObservableEphemerisSource,
        pos: &[f64; 3],
        clk: f64,
        has_clk: bool,
        elem_status: SidereonObservableStateElementStatus,
    ) {
        let batch = source.observable_states_at_shared_j2000_s(&[g01()], HOLE_MIDPOINT_J2000_S);
        let status = batch
            .element_status(0)
            .unwrap_or(CoreObservableStateElementStatus::Error);
        assert_eq!(elem_status, observable_state_element_status_to_c(status));
        assert_eq!(
            pos.map(f64::to_bits),
            batch.positions_ecef_m[0].map(f64::to_bits)
        );
        assert_eq!(has_clk, batch.clocks_s[0].is_some());
        assert_eq!(clk.to_bits(), batch.clocks_s[0].unwrap_or(0.0).to_bits());
    }

    #[test]
    fn test_sp3_load_and_interpolation_policy() {
        // sidereon-core refuses a gap threshold factor of 1.0.
        assert!(Sp3InterpolationOptions::new(1.0).is_err());
        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_load_with_gap_threshold_factor(
                    GAPPED_SP3_BYTES.as_ptr(),
                    GAPPED_SP3_BYTES.len(),
                    1.0,
                    &mut sp3,
                )
            },
            SidereonStatus::InvalidArgument
        );

        assert_eq!(
            unsafe {
                sidereon_sp3_load_with_gap_threshold_factor(
                    GAPPED_SP3_BYTES.as_ptr(),
                    GAPPED_SP3_BYTES.len(),
                    0.0,
                    &mut sp3,
                )
            },
            SidereonStatus::Ok
        );
        let mut factor = 0.0;
        assert_eq!(
            unsafe { sidereon_sp3_gap_threshold_factor(sp3, &mut factor) },
            SidereonStatus::Ok
        );
        assert_eq!(factor.to_bits(), default_factor().to_bits());

        let g01 = CString::new("G01").unwrap();
        let query_epochs = [HOLE_MIDPOINT_J2000_S];
        let mut pos = [0.0; 3];
        let mut clk = 0.0;
        let mut written = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_interpolate(
                    sp3,
                    g01.as_ptr(),
                    query_epochs.as_ptr(),
                    1,
                    pos.as_mut_ptr(),
                    3,
                    &mut clk,
                    1,
                    &mut written,
                )
            },
            SidereonStatus::Solve
        );
        // sidereon-core refuses the hole midpoint under the default factor.
        assert!(unsafe { &(*sp3).inner }
            .position_at_j2000_seconds(self::g01(), HOLE_MIDPOINT_J2000_S)
            .is_err());
        unsafe { sidereon_sp3_free(sp3) };

        assert_eq!(
            unsafe {
                sidereon_sp3_load_with_gap_threshold_factor(
                    GAPPED_SP3_BYTES.as_ptr(),
                    GAPPED_SP3_BYTES.len(),
                    13.0,
                    &mut sp3,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_sp3_gap_threshold_factor(sp3, &mut factor) },
            SidereonStatus::Ok
        );
        assert_eq!(factor, 13.0);

        assert_eq!(
            unsafe {
                sidereon_sp3_interpolate(
                    sp3,
                    g01.as_ptr(),
                    query_epochs.as_ptr(),
                    1,
                    pos.as_mut_ptr(),
                    3,
                    &mut clk,
                    1,
                    &mut written,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 1);
        let core_state = unsafe { &(*sp3).inner }
            .position_at_j2000_seconds(self::g01(), HOLE_MIDPOINT_J2000_S)
            .expect("core state across the hole");
        assert_eq!(
            pos.map(f64::to_bits),
            core_state.position.as_array().map(f64::to_bits)
        );
        assert_eq!(
            clk.to_bits(),
            core_state.clock_s.unwrap_or(f64::NAN).to_bits()
        );
        unsafe { sidereon_sp3_free(sp3) };
    }

    #[test]
    fn test_sp3_load_exact_with_gap_threshold_factor() {
        const EXACT_SP3_BYTES: &[u8] =
            include_bytes!("../tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
        let span = CString::new("01D").unwrap();
        let sample = CString::new("15M").unwrap();
        let agency = CString::new("GRGS").unwrap();
        let mut request: *mut SidereonExactSp3Request = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_exact_request_new(
                    2020,
                    6,
                    24,
                    ptr::null(),
                    span.as_ptr(),
                    sample.as_ptr(),
                    agency.as_ptr(),
                    &mut request,
                )
            },
            SidereonStatus::Ok
        );

        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        let mut coverage = SidereonExactSp3Coverage::Inclusive;

        // Invalid factor 1.0
        assert_eq!(
            unsafe {
                sidereon_sp3_load_exact_with_gap_threshold_factor(
                    EXACT_SP3_BYTES.as_ptr(),
                    EXACT_SP3_BYTES.len(),
                    request,
                    1.0,
                    &mut sp3,
                    &mut coverage,
                )
            },
            SidereonStatus::InvalidArgument
        );

        // Default factor 0.0 -> 1.5
        assert_eq!(
            unsafe {
                sidereon_sp3_load_exact_with_gap_threshold_factor(
                    EXACT_SP3_BYTES.as_ptr(),
                    EXACT_SP3_BYTES.len(),
                    request,
                    0.0,
                    &mut sp3,
                    &mut coverage,
                )
            },
            SidereonStatus::Ok
        );
        let mut factor = 0.0;
        assert_eq!(
            unsafe { sidereon_sp3_gap_threshold_factor(sp3, &mut factor) },
            SidereonStatus::Ok
        );
        assert_eq!(factor.to_bits(), default_factor().to_bits());
        unsafe { sidereon_sp3_free(sp3) };

        // Explicit factor 13.0
        assert_eq!(
            unsafe {
                sidereon_sp3_load_exact_with_gap_threshold_factor(
                    EXACT_SP3_BYTES.as_ptr(),
                    EXACT_SP3_BYTES.len(),
                    request,
                    13.0,
                    &mut sp3,
                    &mut coverage,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_sp3_gap_threshold_factor(sp3, &mut factor) },
            SidereonStatus::Ok
        );
        assert_eq!(factor, 13.0);

        unsafe {
            sidereon_sp3_free(sp3);
            sidereon_sp3_exact_request_free(request);
        }
    }

    #[test]
    fn test_continuity_with_gap_threshold_factor() {
        let core_report = |factor: f64| {
            let options = continuity_options_from_c("test", -1, 1.0, factor).expect("options");
            let core_sp3 = sidereon::load_sp3(GAPPED_SP3_BYTES).expect("core SP3");
            check_continuity(&core_sp3.precise_ephemeris_samples(), &options)
                .expect("valid continuity options")
        };
        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_load(GAPPED_SP3_BYTES.as_ptr(), GAPPED_SP3_BYTES.len(), &mut sp3)
            },
            SidereonStatus::Ok
        );

        let mut defects = 0;
        let mut checked = 0;
        let mut skipped = 0;

        assert_eq!(
            unsafe {
                sidereon_sp3_check_continuity_with_gap_threshold_factor(
                    sp3,
                    -1,
                    1.0,
                    1.0,
                    &mut defects,
                    &mut checked,
                    &mut skipped,
                )
            },
            SidereonStatus::InvalidArgument
        );

        assert_eq!(
            unsafe {
                sidereon_sp3_check_continuity_with_gap_threshold_factor(
                    sp3,
                    -1,
                    1.0,
                    0.0,
                    &mut defects,
                    &mut checked,
                    &mut skipped,
                )
            },
            SidereonStatus::Ok
        );
        let default_defects = defects;
        let core_default = core_report(0.0);
        assert_eq!(
            (defects, checked, skipped),
            (
                core_default.defects.len(),
                core_default.residuals_checked,
                core_default.residuals_skipped
            )
        );

        assert_eq!(
            unsafe {
                sidereon_sp3_check_continuity_with_gap_threshold_factor(
                    sp3,
                    -1,
                    1.0,
                    13.0,
                    &mut defects,
                    &mut checked,
                    &mut skipped,
                )
            },
            SidereonStatus::Ok
        );
        let core_wide = core_report(13.0);
        assert_eq!(
            (defects, checked, skipped),
            (
                core_wide.defects.len(),
                core_wide.residuals_checked,
                core_wide.residuals_skipped
            )
        );
        // A wider gap threshold bridges the hole, so fewer defects remain.
        assert!(defects < default_defects);

        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
                    sp3,
                    -1,
                    1.0,
                    1.0,
                    HOLE_MIDPOINT_J2000_S - 100.0,
                    HOLE_MIDPOINT_J2000_S + 100.0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );

        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
                    sp3,
                    -1,
                    1.0,
                    13.0,
                    HOLE_MIDPOINT_J2000_S - 100.0,
                    HOLE_MIDPOINT_J2000_S + 100.0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let window = EpochWindow::new(HOLE_MIDPOINT_J2000_S - 100.0, HOLE_MIDPOINT_J2000_S + 100.0)
            .expect("window");
        let stencil = StencilExtent::for_sp3(unsafe { &(*sp3).inner }).expect("stencil");
        let expected = serde_json::to_vec(&window_continuity_verdict_json(
            core_wide.verdict_for_window(window, stencil),
        ))
        .expect("core verdict JSON");
        assert_eq!(required, expected.len());

        unsafe { sidereon_sp3_free(sp3) };
    }

    #[test]
    fn test_samples_and_interpolant_and_artifact_with_gap_threshold_factor() {
        let mut sp3: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_load(GAPPED_SP3_BYTES.as_ptr(), GAPPED_SP3_BYTES.len(), &mut sp3)
            },
            SidereonStatus::Ok
        );

        let mut sample_count = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_precise_ephemeris_samples(
                    sp3,
                    ptr::null_mut(),
                    0,
                    &mut 0,
                    &mut sample_count,
                )
            },
            SidereonStatus::Ok
        );
        let core_samples = unsafe { &(*sp3).inner }.precise_ephemeris_samples();
        assert_eq!(sample_count, core_samples.len());
        let mut samples = vec![
            SidereonPreciseEphemerisSample {
                sat: SidereonSatelliteToken { bytes: [0; 17] },
                epoch_j2000_s: 0.0,
                time_scale: SidereonTimeScale::Gpst as u32,
                position_ecef_m: [0.0; 3],
                has_clock_s: false,
                clock_s: 0.0,
                clock_event: false,
            };
            sample_count
        ];
        let mut written = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_precise_ephemeris_samples(
                    sp3,
                    samples.as_mut_ptr(),
                    samples.len(),
                    &mut written,
                    &mut sample_count,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, sample_count);
        unsafe { sidereon_sp3_free(sp3) };

        let mut samples_handle: *mut SidereonPreciseEphemerisSamples = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    1.0,
                    &mut samples_handle,
                )
            },
            SidereonStatus::InvalidArgument
        );

        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    0.0,
                    &mut samples_handle,
                )
            },
            SidereonStatus::Ok
        );
        let mut factor = 0.0;
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_gap_threshold_factor(samples_handle, &mut factor)
            },
            SidereonStatus::Ok
        );
        assert_eq!(factor.to_bits(), default_factor().to_bits());

        let g01 = CString::new("G01").unwrap();
        let sat_ptrs = [g01.as_ptr()];
        let mut pos = [0.0; 3];
        let mut clk = 0.0;
        let mut has_clk = false;
        let mut elem_status = SidereonObservableStateElementStatus::Valid;
        let mut res_status = SidereonStatus::Ok;
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
                    samples_handle,
                    sat_ptrs.as_ptr(),
                    1,
                    HOLE_MIDPOINT_J2000_S,
                    pos.as_mut_ptr(),
                    &mut clk,
                    &mut has_clk,
                    &mut elem_status,
                    &mut res_status,
                )
            },
            SidereonStatus::Ok
        );
        assert_state_matches(
            unsafe { &(*samples_handle).inner },
            &pos,
            clk,
            has_clk,
            elem_status,
        );
        unsafe { sidereon_precise_ephemeris_samples_free(samples_handle) };

        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    13.0,
                    &mut samples_handle,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_gap_threshold_factor(samples_handle, &mut factor)
            },
            SidereonStatus::Ok
        );
        assert_eq!(factor, 13.0);
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
                    samples_handle,
                    sat_ptrs.as_ptr(),
                    1,
                    HOLE_MIDPOINT_J2000_S,
                    pos.as_mut_ptr(),
                    &mut clk,
                    &mut has_clk,
                    &mut elem_status,
                    &mut res_status,
                )
            },
            SidereonStatus::Ok
        );
        assert_state_matches(
            unsafe { &(*samples_handle).inner },
            &pos,
            clk,
            has_clk,
            elem_status,
        );
        unsafe { sidereon_precise_ephemeris_samples_free(samples_handle) };

        let mut interp_handle: *mut SidereonPreciseEphemerisInterpolant = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    1.0,
                    &mut interp_handle,
                )
            },
            SidereonStatus::InvalidArgument
        );

        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    0.0,
                    &mut interp_handle,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
                    interp_handle,
                    &mut factor,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(factor.to_bits(), default_factor().to_bits());
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
                    interp_handle,
                    sat_ptrs.as_ptr(),
                    1,
                    HOLE_MIDPOINT_J2000_S,
                    pos.as_mut_ptr(),
                    &mut clk,
                    &mut has_clk,
                    &mut elem_status,
                    &mut res_status,
                )
            },
            SidereonStatus::Ok
        );
        assert_state_matches(
            unsafe { &(*interp_handle).inner },
            &pos,
            clk,
            has_clk,
            elem_status,
        );
        unsafe { sidereon_precise_ephemeris_interpolant_free(interp_handle) };

        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    samples.len(),
                    13.0,
                    &mut interp_handle,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
                    interp_handle,
                    &mut factor,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(factor, 13.0);
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
                    interp_handle,
                    sat_ptrs.as_ptr(),
                    1,
                    HOLE_MIDPOINT_J2000_S,
                    pos.as_mut_ptr(),
                    &mut clk,
                    &mut has_clk,
                    &mut elem_status,
                    &mut res_status,
                )
            },
            SidereonStatus::Ok
        );
        assert_state_matches(
            unsafe { &(*interp_handle).inner },
            &pos,
            clk,
            has_clk,
            elem_status,
        );
        unsafe { sidereon_precise_ephemeris_interpolant_free(interp_handle) };

        let mut sp3_wide: *mut SidereonSp3 = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_load_with_gap_threshold_factor(
                    GAPPED_SP3_BYTES.as_ptr(),
                    GAPPED_SP3_BYTES.len(),
                    13.0,
                    &mut sp3_wide,
                )
            },
            SidereonStatus::Ok
        );
        let mut art_error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut art_len = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_precise_interpolant_artifact_bytes(
                    sp3_wide,
                    &mut art_error,
                    ptr::null_mut(),
                    0,
                    &mut 0,
                    &mut art_len,
                )
            },
            SidereonStatus::Ok
        );
        let core_artifact = unsafe { &(*sp3_wide).inner }
            .precise_interpolant_store_bytes()
            .expect("core artifact bytes");
        assert_eq!(art_len, core_artifact.len());
        let mut art_bytes = vec![0_u8; art_len];
        assert_eq!(
            unsafe {
                sidereon_sp3_precise_interpolant_artifact_bytes(
                    sp3_wide,
                    &mut art_error,
                    art_bytes.as_mut_ptr(),
                    art_bytes.len(),
                    &mut written,
                    &mut art_len,
                )
            },
            SidereonStatus::Ok
        );
        unsafe { sidereon_sp3_free(sp3_wide) };

        let mut artifact_handle: *mut SidereonPreciseInterpolantArtifact = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_owned(
                    art_bytes.as_ptr(),
                    art_bytes.len(),
                    &mut art_error,
                    &mut artifact_handle,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_gap_threshold_factor(
                    artifact_handle,
                    &mut factor,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(factor, 13.0);
        unsafe { sidereon_precise_interpolant_artifact_free(artifact_handle) };
    }
}

#[cfg(test)]
mod sp3_write_and_agreement_c_tests {
    use std::ptr;

    use super::*;

    /// Two epochs, two satellites, with an absent clock and a clock-only record.
    const SP3C_FILE: &str = "\
#cP2020  6 24  0  0  0.00000000       2 ORBIT IGS14 FIT  TST
## 2111 432000.00000000   900.00000000 59024 0.0000000000000
+    2   G01G02  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc
%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc
%f  1.2500000  1.025000000  0.00000000000  0.000000000000000
%f  0.0000000  0.000000000  0.00000000000  0.000000000000000
%i    0    0    0    0      0      0      0      0         0
%i    0    0    0    0      0      0      0      0         0
/* TEST SP3-c FIXTURE
*  2020  6 24  0  0  0.00000000
PG01  15000.000000 -20000.000000   5000.000000    123.456789
PG02  -1234.567890   2345.678901  -3456.789012 999999.999999
*  2020  6 24  0 15  0.00000000
PG01  15100.000000 -20100.000000   5100.000000   -987.654321              E
PG02      0.000000      0.000000      0.000000    100.000000
EOF
";

    /// One epoch whose only record carries a clock and no orbit.
    const CLOCK_ONLY_FILE: &str = "\
#cP2020  6 24  0  0  0.00000000       1 ORBIT IGS14 FIT  TST
## 2111 259200.00000000   900.00000000 59024 0.0000000000000
+    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
++         5  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0
%c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc
%c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc
%f  1.2500000  1.025000000  0.00000000000  0.000000000000000
%f  0.0000000  0.000000000  0.00000000000  0.000000000000000
%i    0    0    0    0      0      0      0      0         0
%i    0    0    0    0      0      0      0      0         0
*  2020  6 24  0  0  0.00000000
PG01      0.000000      0.000000      0.000000    123.456789
EOF
";

    fn load(text: &str) -> SidereonSp3 {
        SidereonSp3 {
            inner: Sp3::parse(text.as_bytes()).expect("parse SP3 test product"),
        }
    }

    #[test]
    fn a_written_product_carries_its_text_and_no_refusal() {
        let sp3 = load(SP3C_FILE);
        unsafe {
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_sp3_to_sp3_text_result(&sp3, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            assert!(owned.outcome.is_ok);
            assert_eq!(owned.outcome.error.kind, SidereonSp3WriteErrorKind::None);
            assert_eq!(
                owned.text,
                sp3.inner.to_sp3_string().expect("the fixture writes back")
            );
            assert!(owned.message.is_empty());
            sidereon_sp3_write_result_free(result);
        }
    }

    #[test]
    fn a_header_base_finer_than_its_field_is_refused_with_every_field() {
        // The reader keeps a %f base the source spelled with more digits than
        // F10.7 holds; the writer cannot restate it and says so.
        let sp3 = load(&SP3C_FILE.replacen(" 1.2500000", "1.25000001", 1));
        // The base the test writes into the %f record reads back as written.
        assert_eq!(sp3.inner.header.pos_vel_base, Some(1.25000001));
        // sidereon-core's own refusal of the product.
        let Err(Sp3WriteError::PrecisionNotRepresentable {
            field: core_field,
            columns: core_columns,
            decimals: core_decimals,
            value: core_value,
        }) = sp3.inner.to_sp3_string()
        else {
            panic!("sidereon-core writes the finer base");
        };
        unsafe {
            let mut written = usize::MAX;
            let mut required = usize::MAX;
            assert_eq!(
                sidereon_sp3_to_sp3_text(&sp3, ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::InvalidArgument
            );
            assert_eq!((written, required), (0, 0));

            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_sp3_to_sp3_text_result(&sp3, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            let error = owned.outcome.error;
            assert!(!owned.outcome.is_ok);
            assert_eq!(owned.outcome.status, SidereonStatus::InvalidArgument);
            assert_eq!(
                error.kind,
                SidereonSp3WriteErrorKind::PrecisionNotRepresentable
            );
            assert!(error.has_field && !error.has_text_value);
            assert_eq!(owned.field, core_field);
            assert!(error.has_columns && error.columns == core_columns);
            assert!(error.has_decimals && error.decimals == core_decimals);
            assert!(error.has_number);
            assert_eq!(error.number.to_bits(), core_value.to_bits());
            assert!(!error.has_sat_id && !error.has_epoch_index);
            assert!(error.stored.is_nan() && error.column_value.is_nan());
            assert!(owned.text.is_empty());

            written = usize::MAX;
            required = usize::MAX;
            assert_eq!(
                sidereon_sp3_write_result_get_text(
                    result,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!((written, required), (0, 0));
            sidereon_sp3_write_result_free(result);
        }
    }

    #[test]
    fn a_header_satellite_with_no_two_digit_token_is_refused_by_name() {
        // The identifier fields are public, so a satellite number the
        // constructor refuses can reach the writer; its token would not read
        // back as itself.
        let mut sp3 = load(SP3C_FILE);
        let bypass = GnssSatelliteId {
            system: GnssSystem::Gps,
            prn: 100,
        };
        sp3.inner.header.satellites.push(bypass);
        sp3.inner.header.satellite_accuracy_codes.push(0);
        // sidereon-core's own refusal names the satellite.
        let Err(Sp3WriteError::SatelliteNotRepresentable { sat: core_sat }) =
            sp3.inner.to_sp3_string()
        else {
            panic!("sidereon-core writes satellite number 100");
        };
        unsafe {
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_sp3_to_sp3_text_result(&sp3, &mut result),
                SidereonStatus::Ok
            );
            let owned = &*result;
            let error = owned.outcome.error;
            assert!(!owned.outcome.is_ok);
            assert_eq!(
                error.kind,
                SidereonSp3WriteErrorKind::SatelliteNotRepresentable
            );
            assert!(error.has_sat_id);
            let end = error
                .sat_id
                .bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(error.sat_id.bytes.len());
            let token: String = error.sat_id.bytes[..end]
                .iter()
                .map(|byte| *byte as u8 as char)
                .collect();
            assert_eq!(token, core_sat.to_string());
            assert!(!error.has_field && !error.has_epoch_index);
            sidereon_sp3_write_result_free(result);
        }
    }

    #[test]
    fn a_clock_only_merge_reports_position_agreement_absent() {
        let sp3 = load(CLOCK_ONLY_FILE);
        let mut options = default_sp3_merge_options();
        options.min_agree = 1;
        options.clock_min_common = 1;
        let sources: [*const SidereonSp3; 2] = [&sp3, &sp3];
        unsafe {
            let mut merged = ptr::null_mut();
            let mut report = ptr::null_mut();
            assert_eq!(
                sidereon_sp3_merge(sources.as_ptr(), 2, &options, &mut merged, &mut report),
                SidereonStatus::Ok
            );

            let mut count = 0;
            assert_eq!(
                sidereon_sp3_merge_report_epoch_agreement_count(report, &mut count),
                SidereonStatus::Ok
            );
            // sidereon-core's own merge of the same sources and its rollups.
            let core_options = sp3_merge_options_from_c("test", &options).expect("options");
            let (_, core_report) =
                merge(&[sp3.inner.clone(), sp3.inner.clone()], &core_options).expect("core merge");
            let core_agreement = core_report.per_epoch_agreement();
            assert_eq!(count, core_agreement.len());
            let mut agreement = empty_sp3_epoch_agreement();
            assert_eq!(
                sidereon_sp3_merge_report_epoch_agreement(report, 0, &mut agreement),
                SidereonStatus::Ok
            );
            let want = sp3_epoch_agreement_to_c(&core_agreement[0]);
            assert_eq!(agreement.satellites, want.satellites);
            assert_eq!(
                (
                    agreement.position_rms_present,
                    agreement.position_rms_m.to_bits(),
                    agreement.position_max_present,
                    agreement.position_max_m.to_bits()
                ),
                (
                    want.position_rms_present,
                    want.position_rms_m.to_bits(),
                    want.position_max_present,
                    want.position_max_m.to_bits()
                )
            );
            assert_eq!(
                (
                    agreement.clock_rms_present,
                    agreement.clock_rms_s.to_bits(),
                    agreement.clock_max_present,
                    agreement.clock_max_s.to_bits()
                ),
                (
                    want.clock_rms_present,
                    want.clock_rms_s.to_bits(),
                    want.clock_max_present,
                    want.clock_max_s.to_bits()
                )
            );
            // A clock-only epoch has no position spread.
            assert!(!agreement.position_rms_present && agreement.position_rms_m.is_nan());

            let mut summary = empty_sp3_agreement_summary();
            assert_eq!(
                sidereon_sp3_merge_report_agreement_summary(report, &mut summary),
                SidereonStatus::Ok
            );
            let want = sp3_agreement_summary_to_c(&core_report);
            assert_eq!(
                (
                    summary.position_rms_present,
                    summary.position_rms_m.to_bits(),
                    summary.position_max_present,
                    summary.position_max_m.to_bits()
                ),
                (
                    want.position_rms_present,
                    want.position_rms_m.to_bits(),
                    want.position_max_present,
                    want.position_max_m.to_bits()
                )
            );
            assert_eq!(
                (
                    summary.clock_rms_present,
                    summary.clock_rms_s.to_bits(),
                    summary.clock_max_present,
                    summary.clock_max_s.to_bits()
                ),
                (
                    want.clock_rms_present,
                    want.clock_rms_s.to_bits(),
                    want.clock_max_present,
                    want.clock_max_s.to_bits()
                )
            );

            sidereon_sp3_merge_report_free(report);
            sidereon_sp3_free(merged);
        }
    }
}

#[cfg(test)]
mod sp3_typed_error_abi_tests {
    use super::*;
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::ptr;

    unsafe fn error_info() -> SidereonSp3ErrorInfo {
        let mut info = MaybeUninit::<SidereonSp3ErrorInfo>::uninit();
        assert_eq!(
            sidereon_sp3_last_error_info(info.as_mut_ptr()),
            SidereonStatus::Ok
        );
        info.assume_init()
    }

    unsafe fn error_payload(info: SidereonSp3ErrorInfo) -> serde_json::Value {
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            sidereon_sp3_last_error_payload(ptr::null_mut(), 0, &mut written, &mut required,),
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert_eq!(required, info.payload_len);
        let mut bytes = vec![0_u8; required];
        assert_eq!(
            sidereon_sp3_last_error_payload(
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
                &mut required,
            ),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        serde_json::from_slice(&bytes).expect("structured SP3 error JSON")
    }

    fn c_artifact_identity() -> SidereonSp3ArtifactIdentity {
        let center = CString::new("cod").expect("center");
        let mut product = MaybeUninit::<SidereonProductIdentity>::uninit();
        assert_eq!(
            unsafe {
                sidereon_data_product_identity(
                    center.as_ptr(),
                    SidereonProductFamily::Sp3 as u32,
                    2026,
                    7,
                    12,
                    ptr::null(),
                    ptr::null(),
                    product.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let product = unsafe { product.assume_init() };
        let mut resolved_identity = product;
        resolved_identity.has_format_version = 1;
        resolved_identity.format_version =
            fixed_c_chars::<{ data_distribution::FORMAT_VERSION_C_BYTES }>("SP3-d");
        SidereonSp3ArtifactIdentity {
            requested_identity: product,
            resolved_identity,
            distribution_source: SidereonDistributionSource::InMemory as u32,
            official_filename: product.official_filename,
            product_sha256: fixed_c_chars::<SP3_ARTIFACT_SHA256_C_BYTES>(&"a".repeat(64)),
            product_byte_length: 1,
            archive_sha256: fixed_c_chars::<SP3_ARTIFACT_SHA256_C_BYTES>(&"b".repeat(64)),
            archive_byte_length: 1,
            compression: SidereonArchiveCompression::None as u32,
        }
    }

    fn no_typed_error() {
        let info = unsafe { error_info() };
        assert_eq!(info.kind, SidereonSp3ErrorKind::None);
        assert_eq!(info.payload_len, 0);
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_last_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
            },
            SidereonStatus::Ok
        );
        assert_eq!((written, required), (0, 0));
    }

    fn seed_distinct_interval_error(sp3: *const SidereonSp3) {
        unsafe {
            let mut options = MaybeUninit::<SidereonSp3MergeOptions>::uninit();
            assert_eq!(
                sidereon_sp3_merge_options_init(options.as_mut_ptr()),
                SidereonStatus::Ok
            );
            let mut options = options.assume_init();
            options.target_epoch_interval_s_enabled = 1;
            options.target_epoch_interval_s = 0.0;
            let sources = [sp3];
            let mut merged = ptr::null_mut();
            let mut report = ptr::null_mut();
            assert_eq!(
                sidereon_sp3_merge(
                    sources.as_ptr(),
                    sources.len(),
                    &options,
                    &mut merged,
                    &mut report,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(merged.is_null() && report.is_null());
        }
    }

    fn assert_continuity_error(
        expected_field: SidereonSp3ErrorField,
        expected_reason: SidereonSp3ErrorReason,
        expected_value: &str,
    ) {
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::ContinuityOptions);
            assert_eq!(info.field, expected_field);
            assert_eq!(info.reason, expected_reason);
            assert_eq!(error_payload(info)["error"]["value"], expected_value);
        }
    }

    #[test]
    fn exact_error_mapping_preserves_each_current_variant_and_its_fields() {
        use sidereon_core::data::{DataCatalogError, ProductType};
        use ExactSp3ValidationError as E;
        use SidereonSp3ErrorField as F;
        use SidereonSp3ErrorReason as R;

        let cases = vec![
            (
                E::Parse(sidereon_core::Error::Parse("bad bytes".into())),
                "parse",
                F::Sp3Content,
                R::Malformed,
            ),
            (
                E::Catalog(DataCatalogError::UnknownCenter("bad".into())),
                "catalog",
                F::CatalogIdentity,
                R::Invalid,
            ),
            (
                E::WrongProductFamily {
                    actual: ProductType::Clk,
                },
                "wrong_product_family",
                F::ProductFamily,
                R::Mismatch,
            ),
            (
                E::InvalidIssue {
                    issue: "2460".into(),
                },
                "invalid_issue",
                F::IssueToken,
                R::Invalid,
            ),
            (
                E::UnsupportedSpanToken { token: "3D".into() },
                "unsupported_span_token",
                F::SpanToken,
                R::Unsupported,
            ),
            (
                E::UnsupportedSampleToken { token: "7S".into() },
                "unsupported_sample_token",
                F::SampleToken,
                R::Unsupported,
            ),
            (
                E::NonCanonicalSpanToken {
                    token: "24H".into(),
                    canonical: "1D".into(),
                },
                "non_canonical_span_token",
                F::SpanToken,
                R::NonCanonical,
            ),
            (
                E::NonCanonicalSampleToken {
                    token: "60S".into(),
                    canonical: "1M".into(),
                },
                "non_canonical_sample_token",
                F::SampleToken,
                R::NonCanonical,
            ),
            (
                E::InvalidExpectedAgency {
                    agency: "bad!".into(),
                },
                "invalid_expected_agency",
                F::ExpectedAgency,
                R::Invalid,
            ),
            (
                E::AgencyMismatch {
                    expected: "COD".into(),
                    actual: "GFZ".into(),
                },
                "agency_mismatch",
                F::ProducingAgency,
                R::Mismatch,
            ),
            (E::MissingEof, "missing_eof", F::TerminalRecord, R::Missing),
            (
                E::MalformedEofRecord {
                    line_number: usize::MAX,
                    record_length: 2048,
                },
                "malformed_eof_record",
                F::TerminalRecord,
                R::Malformed,
            ),
            (
                E::TrailingContentAfterEof,
                "trailing_content_after_eof",
                F::TerminalRecord,
                R::TrailingContent,
            ),
            (
                E::MandatoryHeaderRecordCount {
                    record: "%i",
                    expected: 2,
                    actual: 1,
                },
                "mandatory_header_record_count",
                F::HeaderRecordCount,
                R::Mismatch,
            ),
            (
                E::MissingDeclaredSatelliteCount,
                "missing_declared_satellite_count",
                F::SatelliteCount,
                R::Missing,
            ),
            (
                E::DeclaredSatelliteCountMismatch {
                    declared: usize::MAX,
                    tokens: 3,
                },
                "declared_satellite_count_mismatch",
                F::SatelliteCount,
                R::Mismatch,
            ),
            (
                E::DuplicateDeclaredSatellite {
                    token: "G01".into(),
                    first_index: 0,
                    duplicate_index: 1,
                },
                "duplicate_declared_satellite",
                F::SatelliteDeclarations,
                R::Duplicate,
            ),
            (
                E::NoDeclaredSatellites,
                "no_declared_satellites",
                F::SatelliteDeclarations,
                R::Empty,
            ),
            (
                E::SatelliteRecordSequenceMismatch {
                    record: "P",
                    epoch_index: 4,
                    expected: vec!["G01".into()],
                    actual: vec!["G02".into()],
                },
                "satellite_record_sequence_mismatch",
                F::SatelliteRecordSequence,
                R::Mismatch,
            ),
            (
                E::BodyRecordInterleavingMismatch {
                    epoch_index: 5,
                    expected: vec!["PG01".into(), "VG01".into()],
                    actual: vec!["VG01".into(), "PG01".into()],
                },
                "body_record_interleaving_mismatch",
                F::BodyRecordOrder,
                R::Mismatch,
            ),
            (
                E::NonFiniteHeaderCadence,
                "non_finite_header_cadence",
                F::HeaderCadence,
                R::NotFinite,
            ),
            (
                E::NonPositiveHeaderCadence { actual_s: 0.0 },
                "non_positive_header_cadence",
                F::HeaderCadence,
                R::NotPositive,
            ),
            (
                E::UnsupportedHeaderCadence { actual_s: 100000.0 },
                "unsupported_header_cadence",
                F::HeaderCadence,
                R::Unsupported,
            ),
            (
                E::CadenceMismatch {
                    requested_s: 300.0,
                    header_s: 600.0,
                },
                "cadence_mismatch",
                F::HeaderCadence,
                R::Mismatch,
            ),
            (
                E::DeclaredEpochCountMismatch {
                    declared: u64::MAX,
                    parsed: usize::MAX,
                },
                "declared_epoch_count_mismatch",
                F::DeclaredEpochCount,
                R::Mismatch,
            ),
            (
                E::MissingDeclaredStart,
                "missing_declared_start",
                F::DeclaredStart,
                R::Missing,
            ),
            (
                E::DeclaredStartMismatch {
                    requested_j2000_s: 1.0,
                    declared_j2000_s: 2.0,
                    requested_tick: i128::MAX,
                    declared_tick: Some(i128::MIN),
                },
                "declared_start_mismatch",
                F::DeclaredStart,
                R::Mismatch,
            ),
            (
                E::RequestBeforeGpsEpoch,
                "request_before_gps_epoch",
                F::GpsStart,
                R::BeforeGpsEpoch,
            ),
            (
                E::NonFiniteHeaderStartMetadata { field: "mjd" },
                "non_finite_header_start_metadata",
                F::HeaderStartMetadata,
                R::NotFinite,
            ),
            (
                E::InvalidHeaderStartMetadata {
                    field: "seconds_of_week",
                    actual: f64::INFINITY,
                },
                "invalid_header_start_metadata",
                F::HeaderStartMetadata,
                R::Invalid,
            ),
            (
                E::HeaderStartMetadataMismatch {
                    field: "mjd",
                    requested: 60000.0,
                    actual: 60001.0,
                },
                "header_start_metadata_mismatch",
                F::HeaderStartMetadata,
                R::Mismatch,
            ),
            (
                E::EmptyEpochGrid,
                "empty_epoch_grid",
                F::EpochGrid,
                R::Empty,
            ),
            (
                E::FirstEpochMismatch {
                    requested_j2000_s: 1.0,
                    actual_j2000_s: 2.0,
                },
                "first_epoch_mismatch",
                F::EpochGrid,
                R::Mismatch,
            ),
            (
                E::IrregularEpochGrid {
                    epoch_index: usize::MAX,
                    requested_s: 300.0,
                    actual_s: 299.99999999,
                },
                "irregular_epoch_grid",
                F::EpochGrid,
                R::Mismatch,
            ),
            (
                E::SpanNotMultipleOfCadence {
                    span_s: u64::MAX,
                    cadence_s: 300,
                },
                "span_not_multiple_of_cadence",
                F::RequestedSpan,
                R::NotMultiple,
            ),
            (
                E::SpanMismatch {
                    parsed: 20,
                    half_open: 21,
                    inclusive: 22,
                },
                "span_mismatch",
                F::RequestedSpan,
                R::Mismatch,
            ),
            (
                E::FormatVersionMismatch {
                    requested: "d".into(),
                    actual: "c".into(),
                },
                "format_version_mismatch",
                F::FormatVersion,
                R::Mismatch,
            ),
        ];

        for (error, variant, expected_field, expected_reason) in cases {
            let status =
                sp3_operation_boundary("test_exact_variant", SidereonStatus::Panic, || {
                    map_exact_sp3_error("test_exact_variant", error)
                });
            assert_eq!(
                status,
                if variant == "parse" {
                    SidereonStatus::Sp3Parse
                } else {
                    SidereonStatus::InvalidArgument
                },
                "{variant}"
            );
            let info = unsafe { error_info() };
            assert_eq!(
                info.kind,
                SidereonSp3ErrorKind::ExactValidation,
                "{variant}"
            );
            assert_eq!(info.field, expected_field, "{variant}");
            assert_eq!(info.reason, expected_reason, "{variant}");
            let payload = unsafe { error_payload(info) };
            assert_eq!(payload["error"]["details"]["variant"], variant, "{variant}");
            assert!(payload["error"]["diagnostic"].is_string(), "{variant}");
        }

        let special = serde_json::json!({
            "variant": "declared_satellite_count_mismatch",
            "declared": usize::MAX.to_string(),
            "tokens": "3",
        });
        let payload = exact_sp3_error_details(&E::DeclaredSatelliteCountMismatch {
            declared: usize::MAX,
            tokens: 3,
        });
        assert_eq!(payload, special);
        let nonfinite = exact_sp3_error_details(&E::NonPositiveHeaderCadence {
            actual_s: f64::INFINITY,
        });
        assert_eq!(nonfinite["actual_s"], "inf");
    }

    #[test]
    fn structured_errors_cover_identity_merge_exact_and_continuity_producers() {
        let artifact = c_artifact_identity();
        let mut options = MaybeUninit::<SidereonSp3MergeOptions>::uninit();
        assert_eq!(
            unsafe { sidereon_sp3_merge_options_init(options.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let mut options = unsafe { options.assume_init() };
        options.target_epoch_interval_s_enabled = 1;
        options.target_epoch_interval_s = 600.0000000001;
        let mut identity = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_sp3_merge_input_identity(&artifact, 1, &options, &mut identity,) },
            SidereonStatus::InvalidArgument
        );
        assert!(identity.is_null());
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::EpochInterval);
            assert_eq!(info.field, SidereonSp3ErrorField::TargetEpochInterval);
            assert_eq!(info.reason, SidereonSp3ErrorReason::NotWholeTicks);
            let payload = error_payload(info);
            assert_eq!(payload["error"]["value"], "600.0000000001");
            assert_eq!(payload["error"]["reason"], "not_whole_ticks");
        }

        options.target_epoch_interval_s_enabled = 0;
        options.position_tolerance_m = -1.0;
        assert_eq!(
            unsafe { sidereon_sp3_merge_input_identity(&artifact, 1, &options, &mut identity,) },
            SidereonStatus::InvalidArgument
        );
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::MergeTolerance);
            assert_eq!(info.field, SidereonSp3ErrorField::PositionTolerance);
            assert_eq!(info.reason, SidereonSp3ErrorReason::Negative);
            assert_eq!(error_payload(info)["error"]["value"], "-1");
        }
        options.position_tolerance_m = f64::NAN;
        assert_eq!(
            unsafe { sidereon_sp3_merge_input_identity(&artifact, 1, &options, &mut identity,) },
            SidereonStatus::InvalidArgument
        );
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::MergeTolerance);
            assert_eq!(info.field, SidereonSp3ErrorField::PositionTolerance);
            assert_eq!(info.reason, SidereonSp3ErrorReason::NotFinite);
            let payload = error_payload(info);
            assert_eq!(payload["error"]["value"], "NaN");
        }

        assert_eq!(
            unsafe { sidereon_sp3_merge_options_init(&mut options) },
            SidereonStatus::Ok
        );
        options.verify_continuity_enabled = 1;
        options.verify_continuity.speed_bound_kind =
            SidereonSp3SpeedBoundKind::ExplicitMaxSpeed as u32;
        options.verify_continuity.explicit_max_speed_m_s = -2.0;
        assert_eq!(
            unsafe { sidereon_sp3_merge_input_identity(&artifact, 1, &options, &mut identity,) },
            SidereonStatus::InvalidArgument
        );
        assert!(identity.is_null());
        assert_continuity_error(
            SidereonSp3ErrorField::SpeedBound,
            SidereonSp3ErrorReason::Negative,
            "-2",
        );

        options.verify_continuity.explicit_max_speed_m_s = 0.0;
        assert_eq!(
            unsafe { sidereon_sp3_merge_input_identity(&artifact, 1, &options, &mut identity,) },
            SidereonStatus::Ok
        );
        assert!(!identity.is_null());
        no_typed_error();
        unsafe { sidereon_sp3_merge_input_identity_free(identity) };

        let requested_tick = i128::MAX - 1;
        let declared_tick = Some(i128::MAX);
        let _ = sp3_operation_boundary("test_exact", SidereonStatus::Panic, || {
            map_exact_sp3_error(
                "test_exact",
                ExactSp3ValidationError::DeclaredStartMismatch {
                    requested_j2000_s: 1.25,
                    declared_j2000_s: 1.25000001,
                    requested_tick,
                    declared_tick,
                },
            )
        });
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::ExactValidation);
            assert_eq!(info.field, SidereonSp3ErrorField::DeclaredStart);
            assert_eq!(info.reason, SidereonSp3ErrorReason::Mismatch);
            assert!(info.has_requested_tick && info.has_declared_tick);
            assert!(info.has_requested_j2000_s && info.has_declared_j2000_s);
            assert_eq!(info.requested_j2000_s, 1.25);
            assert_eq!(info.declared_j2000_s, 1.25000001);
            let payload = error_payload(info);
            assert_eq!(
                payload["error"]["requested_tick"],
                requested_tick.to_string()
            );
            assert_eq!(payload["error"]["declared_tick"], i128::MAX.to_string());
            assert_eq!(payload["error"]["requested_j2000_s"], "1.25");
            assert_eq!(payload["error"]["declared_j2000_s"], "1.25000001");
        }
        let _ = sp3_operation_boundary("test_unplaced_exact", SidereonStatus::Panic, || {
            map_exact_sp3_error(
                "test_unplaced_exact",
                ExactSp3ValidationError::DeclaredStartMismatch {
                    requested_j2000_s: 1.0,
                    declared_j2000_s: f64::NAN,
                    requested_tick,
                    declared_tick: None,
                },
            )
        });
        unsafe {
            let info = error_info();
            assert!(info.has_requested_tick && !info.has_declared_tick);
            let payload = error_payload(info);
            assert_eq!(
                payload["error"]["requested_tick"],
                requested_tick.to_string()
            );
            assert!(payload["error"]["declared_tick"].is_null());
        }

        let sp3 = Box::into_raw(Box::new(SidereonSp3 {
            inner: Sp3::parse(include_bytes!(
                "../tests/fixtures/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3"
            ))
            .expect("fixture SP3"),
        }));
        let mut continuity = MaybeUninit::<SidereonSp3ContinuityOptions>::uninit();
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_options_for_orbit_class(
                    SidereonSp3OrbitClass::MeoGnss as u32,
                    continuity.as_mut_ptr(),
                )
            },
            SidereonStatus::Ok
        );
        let mut continuity = unsafe { continuity.assume_init() };
        continuity.speed_bound_kind = SidereonSp3SpeedBoundKind::ExplicitMaxSpeed as u32;
        continuity.explicit_max_speed_m_s = -0.0;
        continuity.residual_tolerance_enabled = 0;
        let mut written = usize::MAX;
        let mut required = 0;
        let size_status = unsafe {
            sidereon_sp3_continuity_report_json(
                sp3,
                &continuity,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        if size_status != SidereonStatus::Ok {
            let info = unsafe { error_info() };
            let payload = if info.payload_len > 0 {
                unsafe { error_payload(info) }
            } else {
                serde_json::Value::Null
            };
            let mut err_msg_buf = [0 as c_char; 512];
            let len = unsafe {
                crate::sidereon_last_error_message(err_msg_buf.as_mut_ptr(), err_msg_buf.len())
            };
            let msg = if len > 0 {
                unsafe {
                    std::ffi::CStr::from_ptr(err_msg_buf.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                }
            } else {
                String::new()
            };
            panic!(
                "sidereon_sp3_continuity_report_json sizing query failed: status={:?}, message={:?}, kind={:?}, field={:?}, reason={:?}, payload={}",
                size_status, msg, info.kind, info.field, info.reason, payload
            );
        }
        assert_eq!(size_status, SidereonStatus::Ok);
        assert_eq!(written, 0);
        assert!(required > 0);

        let mut bytes = vec![0_u8; required];
        let mut copy_written = 0;
        let mut copy_required = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_report_json(
                    sp3,
                    &continuity,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut copy_written,
                    &mut copy_required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(copy_written, required);
        assert_eq!(copy_required, required);
        let report: serde_json::Value =
            serde_json::from_slice(&bytes[..copy_written]).expect("continuity report JSON");
        assert!(!report["defects"].as_array().expect("defects").is_empty());
        no_typed_error();

        for (invalid_speed, reason) in [
            (-1.0, SidereonSp3ErrorReason::Negative),
            (f64::NAN, SidereonSp3ErrorReason::NotFinite),
            (f64::INFINITY, SidereonSp3ErrorReason::NotFinite),
        ] {
            continuity.explicit_max_speed_m_s = invalid_speed;
            seed_distinct_interval_error(sp3);
            assert_eq!(
                unsafe {
                    sidereon_sp3_continuity_report_json(
                        sp3,
                        &continuity,
                        bytes.as_mut_ptr(),
                        bytes.len(),
                        &mut written,
                        &mut required,
                    )
                },
                SidereonStatus::InvalidArgument
            );
            assert_continuity_error(
                SidereonSp3ErrorField::SpeedBound,
                reason,
                &invalid_speed.to_string(),
            );
        }

        continuity.explicit_max_speed_m_s = -1.0;
        seed_distinct_interval_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json_with_options(
                    sp3,
                    &continuity,
                    0.0,
                    1.0,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_continuity_error(
            SidereonSp3ErrorField::SpeedBound,
            SidereonSp3ErrorReason::Negative,
            "-1",
        );

        seed_distinct_interval_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor(
                    sp3,
                    -1,
                    f64::INFINITY,
                    0.0,
                    0.0,
                    1.0,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_continuity_error(
            SidereonSp3ErrorField::ResidualToleranceM,
            SidereonSp3ErrorReason::NotFinite,
            "inf",
        );

        seed_distinct_interval_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_verdict_json(
                    sp3,
                    -1,
                    f64::NAN,
                    0.0,
                    1.0,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_continuity_error(
            SidereonSp3ErrorField::ResidualToleranceM,
            SidereonSp3ErrorReason::NotFinite,
            "NaN",
        );

        let mut defects = 0;
        let mut residuals_checked = 0;
        let mut residuals_skipped = 0;
        seed_distinct_interval_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_check_continuity_with_gap_threshold_factor(
                    sp3,
                    -1,
                    -1.0,
                    0.0,
                    &mut defects,
                    &mut residuals_checked,
                    &mut residuals_skipped,
                )
            },
            SidereonStatus::Ok
        );
        no_typed_error();

        seed_distinct_interval_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_report_json(
                    ptr::null(),
                    &continuity,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::NullPointer
        );
        no_typed_error();

        let mut merge_options = MaybeUninit::<SidereonSp3MergeOptions>::uninit();
        assert_eq!(
            unsafe { sidereon_sp3_merge_options_init(merge_options.as_mut_ptr()) },
            SidereonStatus::Ok
        );
        let mut merge_options = unsafe { merge_options.assume_init() };
        merge_options.target_epoch_interval_s_enabled = 1;
        merge_options.target_epoch_interval_s = 0.0;
        let sources = [sp3 as *const SidereonSp3, sp3 as *const SidereonSp3];
        let mut merged = ptr::null_mut();
        let mut merge_report = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    sources.as_ptr(),
                    sources.len(),
                    &merge_options,
                    &mut merged,
                    &mut merge_report,
                )
            },
            SidereonStatus::InvalidArgument
        );
        let mut epoch_count = 0;
        assert_eq!(
            unsafe { sidereon_sp3_epoch_count(sp3, &mut epoch_count) },
            SidereonStatus::Ok
        );
        assert_eq!(epoch_count, unsafe { (*sp3).inner.epoch_count() });
        unsafe {
            let info = error_info();
            assert_eq!(info.kind, SidereonSp3ErrorKind::EpochInterval);
            assert_eq!(info.field, SidereonSp3ErrorField::TargetEpochInterval);
            assert_eq!(info.reason, SidereonSp3ErrorReason::NotPositive);
            assert_eq!(error_payload(info)["error"]["value"], "0");
        }

        merge_options.target_epoch_interval_s_enabled = 0;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    ptr::null(),
                    1,
                    &merge_options,
                    &mut merged,
                    &mut merge_report,
                )
            },
            SidereonStatus::NullPointer
        );
        no_typed_error();
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    sources.as_ptr(),
                    sources.len(),
                    &merge_options,
                    &mut merged,
                    &mut merge_report,
                )
            },
            SidereonStatus::Ok
        );
        assert!(!merged.is_null() && !merge_report.is_null());
        no_typed_error();
        unsafe {
            sidereon_sp3_merge_report_free(merge_report);
            sidereon_sp3_free(merged);
            sidereon_sp3_free(sp3);
        }
    }
}

#[cfg(test)]
mod sp3_geometry_engine_error_tests {
    use super::*;
    use std::ptr;

    const SP3_BYTES: &[u8] =
        include_bytes!("../tests/fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3");
    const RECEIVER_ECEF_M: [f64; 3] = [
        f64::from_bits(0x41511b0a63f49487),
        f64::from_bits(0x4120cd6408329d2f),
        f64::from_bits(0x41511e646761efc5),
    ];
    const T_RX_J2000_S: f64 = f64::from_bits(0x41c342aa00000000);

    fn loaded_sp3() -> *mut SidereonSp3 {
        let mut sp3 = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_sp3_load(SP3_BYTES.as_ptr(), SP3_BYTES.len(), &mut sp3) },
            SidereonStatus::Ok
        );
        assert!(!sp3.is_null());
        sp3
    }

    fn assert_dop_error(operation: &str) {
        let (info, payload) = crate::engine_error::snapshot_engine_error_for_test()
            .expect("SP3 geometry DOP refusal");
        assert_eq!(
            info.family,
            crate::engine_error::SidereonEngineErrorFamily::Dop
        );
        assert_eq!(info.payload_len, payload.len());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).expect("DOP payload"),
            serde_json::json!({
                "schema_version": 1,
                "family": "dop",
                "operation": operation,
                "error": {
                    "kind": "invalid_input",
                    "fields": {"field": "step_seconds", "reason": "not positive"}
                }
            })
        );
        let required = unsafe { crate::sidereon_last_error_message(ptr::null_mut(), 0) };
        let mut message = vec![0 as std::ffi::c_char; required + 1];
        unsafe { crate::sidereon_last_error_message(message.as_mut_ptr(), message.len()) };
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }
                .to_str()
                .expect("legacy utf-8"),
            format!("{operation}: invalid DOP input step_seconds: not positive")
        );
    }

    fn seed_dop_error(sp3: *const SidereonSp3) {
        let (mut written, mut required) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visibility_series(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    T_RX_J2000_S + 1800.0,
                    0,
                    10.0,
                    ptr::null(),
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
        assert_dop_error("sidereon_sp3_geometry_visibility_series");
    }

    #[test]
    fn sp3_geometry_guard_dop_callers_record_errors_and_reset_state() {
        use crate::engine_error::{clear_engine_error, snapshot_engine_error_for_test};

        clear_engine_error();
        let sp3 = loaded_sp3();
        let end = T_RX_J2000_S + 1800.0;
        let (mut written, mut required) = (0, 0);

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visible(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert!(snapshot_engine_error_for_test().is_none());

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visibility_series(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    600,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_passes(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    600,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert!(snapshot_engine_error_for_test().is_none());

        let (mut failed_written, mut failed_required) = (usize::MAX, usize::MAX);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visibility_series(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    0,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    &mut failed_written,
                    &mut failed_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((failed_written, failed_required), (0, 0));
        assert_dop_error("sidereon_sp3_geometry_visibility_series");

        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_passes(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    0,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    &mut failed_written,
                    &mut failed_required,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!((failed_written, failed_required), (0, 0));
        assert_dop_error("sidereon_sp3_geometry_passes");

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visible(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut failed_required,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_visibility_series(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    600,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut failed_required,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        seed_dop_error(sp3);
        assert_eq!(
            unsafe {
                sidereon_sp3_geometry_passes(
                    sp3,
                    RECEIVER_ECEF_M.as_ptr(),
                    T_RX_J2000_S,
                    end,
                    600,
                    10.0,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &mut failed_required,
                )
            },
            SidereonStatus::NullPointer
        );
        assert!(snapshot_engine_error_for_test().is_none());

        unsafe { sidereon_sp3_free(sp3) };
        clear_engine_error();
    }
}

#[cfg(test)]
mod sp3_write_error_epoch_mapping_tests {
    use super::*;

    #[test]
    fn current_epoch_writer_refusals_keep_named_kinds_and_index() {
        let satellite = "G07".parse::<GnssSatelliteId>().unwrap();
        let unrepresentable = sp3_write_refusal(&Sp3WriteError::AccuracyNotRepresentable {
            sat: satellite,
            epoch_index: 5,
            component: "position",
            exponent: Some(12),
        });
        assert_eq!(
            unrepresentable.error.kind,
            SidereonSp3WriteErrorKind::AccuracyNotRepresentable
        );
        assert!(unrepresentable.error.has_epoch_index);
        assert_eq!(unrepresentable.error.epoch_index, 5);
        assert!(unrepresentable.error.has_sat_id);
        assert_eq!(
            unrepresentable.error.sat_id.bytes,
            satellite_token(satellite).bytes
        );
        assert_eq!(unrepresentable.field, Some("position"));
        assert!(unrepresentable.error.has_exponent);
        assert_eq!(unrepresentable.error.exponent, 12);

        for (error, kind, epoch) in [
            (
                Sp3WriteError::AccuracyRecordMismatch {
                    sat: satellite,
                    epoch_index: 6,
                },
                SidereonSp3WriteErrorKind::AccuracyRecordMismatch,
                6,
            ),
            (
                Sp3WriteError::AccuracyBasisMissing {
                    sat: satellite,
                    epoch_index: 8,
                },
                SidereonSp3WriteErrorKind::AccuracyBasisMissing,
                8,
            ),
        ] {
            let mapped = sp3_write_refusal(&error);
            assert_eq!(mapped.error.kind, kind);
            assert_eq!(mapped.error.epoch_index, epoch);
            assert_eq!(mapped.error.sat_id.bytes, satellite_token(satellite).bytes);
            assert!(mapped.error.has_epoch_index);
            assert!(mapped.error.has_sat_id);
        }
    }
}
