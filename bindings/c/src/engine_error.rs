use crate::{copy_prefix_to_c, ffi_boundary, init_copy_counts, require_out, SidereonStatus};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::ptr;

/// Stable family identifier for the versioned engine-error detail record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonEngineErrorFamily {
    /// No engine error is retained on this thread.
    None = 0,
    /// Sequential, static, or dual-frequency RTK arc failure.
    Rtk = 1,
    /// Static reference-station solve failure.
    StaticReference = 2,
    /// Trust-region least-squares solve failure.
    Trls = 3,
    /// Integer least-squares search failure.
    Ils = 4,
    /// Orbit ephemeris SPK file parsing or evaluation failure.
    Spk = 5,
    /// Conjunction data message (CDM) parsing or evaluation failure.
    Cdm = 6,
    /// Tracking data message (TDM) parsing or evaluation failure.
    Tdm = 7,
    /// GNSS/INS fusion filter failure.
    Fusion = 8,
    /// Fusion state binary serialization or deserialization failure.
    FusionStateCodec = 9,
    /// Clock Allan-family stability estimator failure.
    Allan = 10,
    /// Clock power-law noise identification or fit failure.
    PowerLawNoise = 11,
    /// Terrestrial reference-frame catalog and propagation failure.
    FrameCatalog = 12,
    /// Sidereal filtering and repeating multipath error.
    Sidereal = 13,
    /// Neutral-atmosphere density model evaluation failure.
    Atmosphere = 14,
    /// Source localization solve failure.
    SourceLocalization = 15,
    /// Geodetic time series analysis failure.
    GeodeticTimeSeries = 16,
    /// Statistical normality testing failure.
    Normality = 17,
    /// Track filtering or covariance update failure.
    Track = 18,
    /// Precise ephemeris samples building and validation failure.
    PreciseSamples = 19,
    /// Precise ephemeris interpolant construction or evaluation failure.
    PreciseInterpolant = 20,
    /// Space weather data parsing or evaluation failure.
    SpaceWeather = 21,
    /// Advanced RAIM integrity calculation failure.
    Araim = 22,
    /// Reduced orbit propagation or state failure.
    ReducedOrbit = 23,
    /// Reduced orbit source failure.
    ReducedOrbitSource = 24,
    /// Piecewise orbit evaluation failure.
    PiecewiseOrbit = 25,
    /// Orbit determination fit failure.
    OrbitFit = 26,
    /// Orbital elements conversion failure.
    Elements = 27,
    /// Equinoctial elements conversion failure.
    Equinoctial = 28,
    /// RTN frame transformation failure.
    RtnFrame = 29,
    /// Orbit anomaly conversion failure.
    Anomaly = 30,
    /// Numerical orbit propagation or state transition failure.
    Propagation = 31,
    /// Satellite orbit decay lifetime prediction failure.
    Decay = 32,
    /// DGNSS position orchestration failure.
    Dgnss = 33,
    /// Synthetic-observable scenario simulation failure.
    Scenario = 34,
    /// Data product catalog operations and queries.
    Catalog = 35,
    /// Exact product cache operations, storage, and publishing.
    ExactCache = 36,
    /// Time of closest approach calculation failure.
    Tca = 37,
    /// Astronomical almanac event evaluation failure.
    Almanac = 38,
    /// Topocentric and celestial body observation failure.
    Observe = 39,
    /// Celestial body observation failure.
    BodyObservation = 40,
    /// Ground station look angle prediction failure.
    LookAngle = 41,
    /// Satellite pass prediction failure.
    Pass = 42,
    /// Astronomical event finder failure.
    EventFinder = 43,
    /// Reference frame transformation failure.
    FrameTransform = 44,
    /// Close-approach conjunction evaluation failure.
    Conjunction = 45,
    /// Unified ergonomic facade product parsing or solve failure.
    Facade = 46,
    /// Single-point positioning core solve failure.
    Spp = 47,
    /// Single-point positioning policy or solution validation failure.
    SppPolicy = 48,
    /// Low-precision Sun and Moon position computation failure.
    SunMoon = 49,
    /// RINEX observation to SPP input assembly failure.
    RinexSpp = 50,
    /// Receiver solution validation failure.
    SolutionValidation = 51,
    /// Radio-frequency link budget input validation failure.
    Rf = 52,
    /// Ionosphere-free GNSS observable combination failure.
    IonosphereFree = 53,
    /// Satellite-ground Doppler calculation failure.
    Doppler = 54,
    /// CCSDS Orbit Ephemeris Message codec failure.
    Oem = 55,
    /// CCSDS Orbit Parameter Message codec failure.
    Opm = 56,
    /// CCSDS Orbit Mean-Elements Message codec failure.
    Omm = 57,
    /// Dilution-of-precision and error-ellipse failure.
    Dop = 58,
    /// Geodesic fence construction and evaluation failure.
    Geofence = 59,
    /// Position-error metrics and percentile calculations.
    ErrorMetrics = 63,
    /// GPS C/A signal generation, correlation, or acquisition failure.
    Signal = 60,
    /// Carrier-phase combination or cycle-slip failure.
    CarrierPhase = 61,
    /// Signal spectral or discriminator analysis failure.
    SignalAnalysis = 62,
    /// GNSS observable prediction and state evaluation failure.
    Observables = 64,
    /// NMEA sentence writing failure.
    Nmea = 65,
    /// SGP4 TLE fitting failure, including a complete best-effort fit when available.
    TleFit = 66,
    /// Initial-orbit-determination geometry or convergence failure.
    Iod = 67,
    /// Product staleness selection failure with complete SelectionError fields.
    Selection = 68,
    /// PPP auto-initialization failure, including its nested SPP/float/fixed cause.
    PppAutoInit = 69,
    /// Static-positioning solve failure with complete typed causes.
    StaticPositioning = 70,
    /// Inter-system time-scale offset failure with complete variant and scale.
    TimeOffset = 71,
    /// Time-model construction failure with exact input field and reason.
    TimeModel = 72,
    /// A family introduced by a later library version.
    Unknown = 999,
}

/// Summary for the retained versioned engine-error JSON payload.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonEngineErrorInfo {
    /// Error family for this operation.
    pub family: SidereonEngineErrorFamily,
    /// UTF-8 JSON payload size, excluding a terminator.
    pub payload_len: usize,
}

thread_local! {
    static LAST_ENGINE_ERROR: RefCell<Option<(SidereonEngineErrorInfo, String)>> = const { RefCell::new(None) };
    static LAST_OBSERVABLE_ROW_ERRORS: RefCell<Vec<(usize, SidereonStatus, String)>> = const { RefCell::new(Vec::new()) };
}

/// Owned summary for one retained observable batch-row failure.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SidereonObservableRowErrorInfo {
    /// Zero-based input row for this error.
    pub row_index: usize,
    /// Legacy C result status associated with this row.
    pub status: SidereonStatus,
    /// UTF-8 JSON payload size, excluding a terminator.
    pub payload_len: usize,
}

/// Caller-owned snapshot of every typed error produced by the most recent
/// observable batch on this OS thread. Release with
/// `sidereon_observable_row_errors_free`.
pub struct SidereonObservableRowErrors {
    rows: Vec<(usize, SidereonStatus, String)>,
}

impl SidereonEngineErrorFamily {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Rtk => "rtk",
            Self::StaticReference => "static_reference",
            Self::Trls => "trls",
            Self::Ils => "ils",
            Self::Spk => "spk",
            Self::Cdm => "cdm",
            Self::Tdm => "tdm",
            Self::Fusion => "fusion",
            Self::FusionStateCodec => "fusion_state_codec",
            Self::Allan => "allan",
            Self::PowerLawNoise => "power_law_noise",
            Self::FrameCatalog => "frame_catalog",
            Self::Sidereal => "sidereal",
            Self::Atmosphere => "atmosphere",
            Self::SourceLocalization => "source_localization",
            Self::GeodeticTimeSeries => "geodetic_time_series",
            Self::Normality => "normality",
            Self::Track => "track",
            Self::PreciseSamples => "precise_samples",
            Self::PreciseInterpolant => "precise_interpolant",
            Self::SpaceWeather => "space_weather",
            Self::Araim => "araim",
            Self::ReducedOrbit => "reduced_orbit",
            Self::ReducedOrbitSource => "reduced_orbit_source",
            Self::PiecewiseOrbit => "piecewise_orbit",
            Self::OrbitFit => "orbit_fit",
            Self::Elements => "elements",
            Self::Equinoctial => "equinoctial",
            Self::RtnFrame => "rtn_frame",
            Self::Anomaly => "anomaly",
            Self::Propagation => "propagation",
            Self::Decay => "decay",
            Self::Dgnss => "dgnss",
            Self::Scenario => "scenario",
            Self::Catalog => "catalog",
            Self::ExactCache => "exact_cache",
            Self::Tca => "tca",
            Self::Almanac => "almanac",
            Self::Observe => "observe",
            Self::BodyObservation => "body_observation",
            Self::LookAngle => "look_angle",
            Self::Pass => "pass",
            Self::EventFinder => "event_finder",
            Self::FrameTransform => "frame_transform",
            Self::Conjunction => "conjunction",
            Self::Facade => "facade",
            Self::Spp => "spp",
            Self::SppPolicy => "spp_policy",
            Self::SunMoon => "sun_moon",
            Self::RinexSpp => "rinex_spp",
            Self::SolutionValidation => "solution_validation",
            Self::Rf => "rf",
            Self::IonosphereFree => "ionosphere_free",
            Self::Doppler => "doppler",
            Self::Oem => "oem",
            Self::Opm => "opm",
            Self::Omm => "omm",
            Self::Dop => "dop",
            Self::Geofence => "geofence",
            Self::ErrorMetrics => "error_metrics",
            Self::Signal => "signal",
            Self::CarrierPhase => "carrier_phase",
            Self::SignalAnalysis => "signal_analysis",
            Self::Observables => "observables",
            Self::Nmea => "nmea",
            Self::TleFit => "tle_fit",
            Self::Iod => "iod",
            Self::Selection => "selection",
            Self::PppAutoInit => "ppp_auto_init",
            Self::StaticPositioning => "static_positioning",
            Self::TimeOffset => "time_offset",
            Self::TimeModel => "time_model",
            Self::Unknown => "unknown",
        }
    }
}

pub(crate) fn nmea_error_value(error: &sidereon_core::nmea::NmeaError) -> Value {
    use sidereon_core::nmea::NmeaError as E;
    match error {
        E::NotFramed { reason } => rtk_node("not_framed", json!({"reason": reason})),
        E::ChecksumMismatch { computed, stated } => rtk_node(
            "checksum_mismatch",
            json!({"computed": computed, "stated": stated}),
        ),
        E::UnsupportedType { address } => rtk_node("unsupported_type", json!({"address": address})),
        E::Proprietary { address } => rtk_node("proprietary", json!({"address": address})),
        E::MalformedField(error) => rtk_node(
            "malformed_field",
            json!({"cause": nmea_field_error_value(error)}),
        ),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
    }
}

fn nmea_field_error_value(error: &sidereon_core::nmea::FieldError) -> Value {
    use sidereon_core::nmea::FieldError as E;
    match error {
        E::Missing { field } => rtk_node("missing", json!({"field": field})),
        E::NonFinite { field } => rtk_node("non_finite", json!({"field": field})),
        E::NotPositive { field } => rtk_node("not_positive", json!({"field": field})),
        E::Negative { field } => rtk_node("negative", json!({"field": field})),
        E::OutOfRange {
            field,
            min,
            max,
            upper_inclusive,
        } => rtk_node(
            "out_of_range",
            json!({
                "field": field,
                "min": engine_f64(*min),
                "max": engine_f64(*max),
                "upper_inclusive": upper_inclusive,
            }),
        ),
        E::FloatParse { field, value } => {
            rtk_node("float_parse", json!({"field": field, "value": value}))
        }
        E::IntParse { field, value } => {
            rtk_node("int_parse", json!({"field": field, "value": value}))
        }
        E::InvalidCivilDate {
            field,
            year,
            month,
            day,
        } => rtk_node(
            "invalid_civil_date",
            json!({"field": field, "year": year, "month": month, "day": day}),
        ),
        E::InvalidCivilTime {
            field,
            hour,
            minute,
            second,
        } => rtk_node(
            "invalid_civil_time",
            json!({
                "field": field,
                "hour": hour,
                "minute": minute,
                "second": engine_f64(*second),
            }),
        ),
    }
}

pub(crate) fn nmea_skip_value(skip: &sidereon_core::nmea::Skip) -> Value {
    use sidereon_core::nmea::SkipReason as R;
    let reason = match &skip.reason {
        R::UnrepresentableSatellite => rtk_node("unrepresentable_satellite", json!({})),
        R::UnsupportedRecordType(record_type) => rtk_node(
            "unsupported_record_type",
            json!({"record_type": record_type}),
        ),
        R::MalformedField(error) => rtk_node(
            "malformed_field",
            json!({"cause": nmea_field_error_value(error)}),
        ),
        R::OutOfRangeEpoch => rtk_node("out_of_range_epoch", json!({})),
        R::Truncated => rtk_node("truncated", json!({})),
        R::UnsupportedUnit(unit) => rtk_node("unsupported_unit", json!({"unit": unit})),
        R::UnknownBlock(block) => rtk_node("unknown_block", json!({"block": block})),
        R::InconsistentRecord(reason) => rtk_node("inconsistent_record", json!({"reason": reason})),
    };
    rtk_node(
        "skip",
        json!({"at": nmea_record_ref_value(&skip.at), "reason": reason}),
    )
}

pub(crate) fn nmea_warning_value(warning: &sidereon_core::nmea::Warning) -> Value {
    use sidereon_core::nmea::WarningKind as W;
    let kind = match warning.kind {
        W::Checksum => "checksum",
        W::Clamped => "clamped",
        W::Degraded => "degraded",
        W::Mismatch => "mismatch",
        W::Overlap => "overlap",
        W::MissingMetadata => "missing_metadata",
    };
    rtk_node(
        "warning",
        json!({"at": nmea_record_ref_value(&warning.at), "warning_kind": kind}),
    )
}

fn nmea_record_ref_value(at: &sidereon_core::nmea::RecordRef) -> Value {
    json!({
        "line": at.line,
        "record_index": at.record_index,
        "satellite": at.satellite,
    })
}

/// Clear retained engine detail at a producing operation boundary.
pub(crate) fn clear_engine_error() {
    LAST_ENGINE_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Clear per-row observable diagnostics at an observable producer boundary.
pub(crate) fn clear_observable_row_errors() {
    LAST_OBSERVABLE_ROW_ERRORS.with(|rows| rows.borrow_mut().clear());
}

/// Retain a full typed error for one observable batch row.
pub(crate) fn record_observable_row_error(
    operation: &str,
    row_index: usize,
    status: SidereonStatus,
    error: &sidereon_core::observables::ObservablesError,
) {
    let tree = observables_error_value(error);
    record_engine_error(
        SidereonEngineErrorFamily::Observables,
        operation,
        tree.clone(),
    );
    let payload = json!({
        "schema_version": 1,
        "family": SidereonEngineErrorFamily::Observables.name(),
        "operation": operation,
        "error": tree,
    })
    .to_string();
    LAST_OBSERVABLE_ROW_ERRORS.with(|rows| {
        rows.borrow_mut().push((row_index, status, payload));
    });
}

/// Apply the panic boundary and clear only the engine-detail slot before any
/// argument validation. Error-message compatibility and other typed slots are
/// left to their existing operation boundaries.
pub(crate) fn engine_error_operation_boundary<T>(
    fn_name: &str,
    panic_value: T,
    body: impl FnOnce() -> T,
) -> T {
    clear_engine_error();
    ffi_boundary(fn_name, panic_value, body)
}

/// Apply the producer boundary for observable APIs, clearing both typed slots.
pub(crate) fn observables_error_operation_boundary<T>(
    fn_name: &str,
    panic_value: T,
    body: impl FnOnce() -> T,
) -> T {
    clear_observable_row_errors();
    engine_error_operation_boundary(fn_name, panic_value, body)
}

/// Record an owned, versioned JSON tree for a public core-domain failure.
pub(crate) fn record_engine_error(
    family: SidereonEngineErrorFamily,
    operation: &str,
    error: Value,
) {
    let payload = json!({
        "schema_version": 1,
        "family": family.name(),
        "operation": operation,
        "error": error,
    })
    .to_string();
    let info = SidereonEngineErrorInfo {
        family,
        payload_len: payload.len(),
    };
    LAST_ENGINE_ERROR.with(|slot| *slot.borrow_mut() = Some((info, payload)));
}

/// Record a time-model refusal while preserving the caller-specific legacy
/// message and status.
pub(crate) fn time_model_error(
    fn_name: &str,
    error: sidereon_core::astro::time::model::TimeModelError,
    legacy_message: String,
) -> SidereonStatus {
    use sidereon_core::astro::time::model::TimeModelError as E;
    let message = error.to_string();
    match error {
        E::InvalidInput { field, reason } => {
            record_engine_error(
                SidereonEngineErrorFamily::TimeModel,
                fn_name,
                json!({
                    "kind": "invalid_input",
                    "message": message,
                    "fields": {"field": field, "reason": reason},
                }),
            );
            crate::set_last_error(legacy_message);
            SidereonStatus::InvalidArgument
        }
    }
}

/// Record a time-offset refusal while preserving its existing C status/message.
pub(crate) fn time_offset_error(
    fn_name: &str,
    error: sidereon_core::astro::time::TimeOffsetError,
    legacy_message: String,
) -> SidereonStatus {
    use sidereon_core::astro::time::TimeOffsetError as E;
    let message = error.to_string();
    let (kind, scale) = match error {
        E::EpochRequired(scale) => ("epoch_required", scale),
        E::Unsupported(scale) => ("unsupported", scale),
        E::NonFiniteEpoch(scale) => ("non_finite_epoch", scale),
    };
    record_engine_error(
        SidereonEngineErrorFamily::TimeOffset,
        fn_name,
        json!({
            "kind": kind,
            "message": message,
            "fields": {"scale": scale},
        }),
    );
    crate::set_last_error(legacy_message);
    SidereonStatus::InvalidArgument
}

/// Preserve a binary64 value as a decimal string, including signed zero and
/// non-finite values, in a JSON payload.
pub(crate) fn engine_f64(value: f64) -> Value {
    json!({
        "decimal": value.to_string(),
        "bits_hex": format!("{:016x}", value.to_bits()),
    })
}

pub(crate) fn signal_error_value(error: &sidereon_core::signal::SignalError) -> Value {
    use sidereon_core::signal::SignalError;
    match error {
        SignalError::UnsupportedPrn(prn) => rtk_node("unsupported_prn", json!({"prn":prn})),
        SignalError::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field":field,"reason":reason}))
        }
        SignalError::EmptySamples => rtk_node("empty_samples", json!({})),
        SignalError::TooShort => rtk_node("too_short", json!({})),
    }
}

pub(crate) fn signal_analysis_error_value(
    error: &sidereon_core::signal::analysis::SignalAnalysisError,
) -> Value {
    use sidereon_core::signal::analysis::SignalAnalysisError;
    match error {
        SignalAnalysisError::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field":field,"reason":reason}))
        }
        SignalAnalysisError::EmptyComponents => rtk_node("empty_components", json!({})),
        SignalAnalysisError::NoDiscriminatorRoot {
            delay_chips,
            phase_sign,
        } => rtk_node(
            "no_discriminator_root",
            json!({
                "delay_chips":engine_f64(*delay_chips),
                "phase_sign":engine_f64(*phase_sign),
            }),
        ),
    }
}

fn rtk_node(kind: &str, fields: Value) -> Value {
    json!({"kind": kind, "fields": fields})
}

fn rtk_input_kind_name(kind: sidereon_core::rtk_filter::RtkInputErrorKind) -> &'static str {
    use sidereon_core::rtk_filter::RtkInputErrorKind as K;
    match kind {
        K::NonFinite => "non_finite",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::Missing => "missing",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

fn double_difference_error_value(error: &sidereon_core::rtk::DoubleDifferenceError) -> Value {
    use sidereon_core::rtk::DoubleDifferenceError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::DuplicateObservation(satellite_id) => rtk_node(
            "duplicate_observation",
            json!({"satellite_id": satellite_id}),
        ),
        E::TooFewCommonSatellites { count, minimum } => rtk_node(
            "too_few_common_satellites",
            json!({"count": count, "minimum": minimum}),
        ),
        E::NoCommonReferenceSatellite(system) => {
            rtk_node("no_common_reference_satellite", json!({"system": system}))
        }
        E::MissingSatellitePosition(satellite_id) => rtk_node(
            "missing_satellite_position",
            json!({"satellite_id": satellite_id}),
        ),
        E::ReferenceSatelliteMissing(satellite_id) => rtk_node(
            "reference_satellite_missing",
            json!({"satellite_id": satellite_id}),
        ),
        E::ReferenceSatelliteSingleSystem(satellite_id) => rtk_node(
            "reference_satellite_single_system",
            json!({"satellite_id": satellite_id}),
        ),
        E::ReferenceSatelliteMissingSystem(system) => rtk_node(
            "reference_satellite_missing_system",
            json!({"system": system}),
        ),
        E::InvalidReferenceOption => rtk_node("invalid_reference_option", json!({})),
    }
}

fn receiver_antenna_error_value(error: sidereon_core::rtk_filter::ReceiverAntennaError) -> Value {
    use sidereon_core::rtk_filter::ReceiverAntennaError as E;
    rtk_node(
        match error {
            E::MissingPcv => "missing_pcv",
            E::InvalidGeometry => "invalid_geometry",
        },
        json!({}),
    )
}

fn cycle_slip_reason_name(reason: sidereon_core::carrier_phase::SlipReason) -> &'static str {
    use sidereon_core::carrier_phase::SlipReason as R;
    match reason {
        R::Lli => "lli",
        R::DataGap => "data_gap",
        R::GeometryFree => "geometry_free",
        R::MelbourneWubbena => "melbourne_wubbena",
    }
}

fn cycle_slip_error_value(error: &sidereon_core::rtk::CycleSlipPrepError) -> Value {
    use sidereon_core::rtk::CycleSlipPrepError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::CycleSlipDetected {
            receiver,
            satellite_id,
            epoch_index,
            reasons,
        } => rtk_node(
            "cycle_slip_detected",
            json!({
                "receiver": match receiver { sidereon_core::rtk::CycleSlipReceiver::Base => "base", sidereon_core::rtk::CycleSlipReceiver::Rover => "rover" },
                "satellite_id": satellite_id,
                "epoch_index": epoch_index,
                "reasons": reasons.iter().map(|reason| cycle_slip_reason_name(*reason)).collect::<Vec<_>>(),
            }),
        ),
    }
}

fn invalid_state_kind_value(kind: &sidereon_core::rtk_filter::InvalidStateKind) -> Value {
    use sidereon_core::rtk_filter::InvalidStateKind as K;
    match kind {
        K::Length { expected, actual } => {
            rtk_node("length", json!({"expected": expected, "actual": actual}))
        }
        K::DimensionOverflow => rtk_node("dimension_overflow", json!({})),
        K::NonFinite => rtk_node("non_finite", json!({})),
        K::NotPositive => rtk_node("not_positive", json!({})),
        K::NotSymmetric => rtk_node("not_symmetric", json!({})),
        K::NotPositiveSemidefinite => rtk_node("not_positive_semidefinite", json!({})),
    }
}

fn filter_state_kind_value(kind: &sidereon_core::rtk_filter::FilterStateValidationKind) -> Value {
    use sidereon_core::rtk_filter::FilterStateValidationKind as K;
    match kind {
        K::Length { expected, actual } => {
            rtk_node("length", json!({"expected": expected, "actual": actual}))
        }
        K::DimensionOverflow => rtk_node("dimension_overflow", json!({})),
        K::NonFinite => rtk_node("non_finite", json!({})),
        K::NotPositive => rtk_node("not_positive", json!({})),
        K::NotSymmetric => rtk_node("not_symmetric", json!({})),
        K::NotPositiveSemidefinite => rtk_node("not_positive_semidefinite", json!({})),
    }
}

pub(crate) fn ils_error_value(error: &sidereon_core::ils::IlsError) -> Value {
    use sidereon_core::ils::IlsError as E;
    match error {
        E::Singular => rtk_node("singular", json!({})),
        E::NoCandidates(count) => rtk_node("no_candidates", json!({"count": count})),
        E::TooManyCandidates { evaluated, limit } => rtk_node(
            "too_many_candidates",
            json!({"evaluated": evaluated, "limit": limit}),
        ),
        E::InvalidDimensions { n, rows } => {
            rtk_node("invalid_dimensions", json!({"n": n, "rows": rows}))
        }
        E::NonFinite => rtk_node("non_finite", json!({})),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::SearchLimitExceeded => rtk_node("search_limit_exceeded", json!({})),
    }
}

fn update_error_value(error: &sidereon_core::rtk_filter::UpdateError) -> Value {
    use sidereon_core::rtk_filter::UpdateError as E;
    match error {
        E::InvalidState { field, kind } => rtk_node(
            "invalid_state",
            json!({"field": field, "cause": invalid_state_kind_value(kind)}),
        ),
        E::ReferenceChanged {
            system,
            expected,
            actual,
        } => rtk_node(
            "reference_changed",
            json!({"system": system, "expected": expected, "actual": actual}),
        ),
        E::UnknownReferenceSystem(system) => {
            rtk_node("unknown_reference_system", json!({"system": system}))
        }
        E::MissingSystemReference(system) => {
            rtk_node("missing_system_reference", json!({"system": system}))
        }
        E::MissingAmbiguityColumn(id) => {
            rtk_node("missing_ambiguity_column", json!({"ambiguity_id": id}))
        }
        E::MissingWavelength(id) => rtk_node("missing_wavelength", json!({"ambiguity_id": id})),
        E::MissingOffset(id) => rtk_node("missing_offset", json!({"ambiguity_id": id})),
        E::InvalidInput { field, kind } => rtk_node(
            "invalid_input",
            json!({"field": field, "kind": rtk_input_kind_name(*kind)}),
        ),
        E::SingularGeometry => rtk_node("singular_geometry", json!({})),
        E::ReceiverAntenna(source) => rtk_node(
            "receiver_antenna",
            json!({"cause": receiver_antenna_error_value(*source)}),
        ),
        E::Ils(source) => rtk_node("ils", json!({"cause": ils_error_value(source)})),
    }
}

/// Complete current `RtkArcError` tree with every present scalar field.
pub(crate) fn rtk_arc_error_value(error: &sidereon_core::rtk_filter::RtkArcError) -> Value {
    use sidereon_core::rtk_filter::RtkArcError as E;
    match error {
        E::EmptyEpochs => rtk_node("empty_epochs", json!({})),
        E::TooFewSatellites { count, minimum } => rtk_node(
            "too_few_satellites",
            json!({"count": count, "minimum": minimum}),
        ),
        E::Reference(source) => rtk_node(
            "reference",
            json!({"cause": double_difference_error_value(source)}),
        ),
        E::FilterState(source) => rtk_node(
            "filter_state",
            json!({"cause": rtk_node("filter_state_validation", json!({"field": source.field, "kind": filter_state_kind_value(&source.kind)}))}),
        ),
        E::Update {
            epoch_index,
            source,
        } => rtk_node(
            "update",
            json!({"epoch_index": epoch_index, "cause": update_error_value(source)}),
        ),
        E::InvalidEpochTime { epoch_index } => {
            rtk_node("invalid_epoch_time", json!({"epoch_index": epoch_index}))
        }
        E::MissingPosition {
            epoch_index,
            satellite_id,
        } => rtk_node(
            "missing_position",
            json!({"epoch_index": epoch_index, "satellite_id": satellite_id}),
        ),
        E::CycleSlipPrep(source) => rtk_node(
            "cycle_slip_prep",
            json!({"cause": cycle_slip_error_value(source)}),
        ),
        E::CodeSmoothing(source) => rtk_node(
            "code_smoothing",
            json!({"cause": match source { sidereon_core::rtk::CodeSmoothingError::InvalidWindowCap => rtk_node("invalid_window_cap", json!({})) }}),
        ),
        E::ElevationMask(source) => rtk_node(
            "elevation_mask",
            json!({"cause": double_difference_error_value(source)}),
        ),
    }
}

fn float_solve_error_value(error: &sidereon_core::rtk_filter::FloatSolveError) -> Value {
    use sidereon_core::rtk_filter::FloatSolveError as E;
    match error {
        E::MissingSystemReference(system) => {
            rtk_node("missing_system_reference", json!({"system": system}))
        }
        E::MissingAmbiguityColumn(id) => {
            rtk_node("missing_ambiguity_column", json!({"ambiguity_id": id}))
        }
        E::InvalidInput { field, kind } => rtk_node(
            "invalid_input",
            json!({"field": field, "kind": rtk_input_kind_name(*kind)}),
        ),
        E::SingularGeometry => rtk_node("singular_geometry", json!({})),
        E::IncompleteResidualPair => rtk_node("incomplete_residual_pair", json!({})),
        E::ReceiverAntenna(source) => rtk_node(
            "receiver_antenna",
            json!({"cause": receiver_antenna_error_value(*source)}),
        ),
    }
}

fn fixed_solve_error_value(error: &sidereon_core::rtk_filter::FixedSolveError) -> Value {
    use sidereon_core::rtk_filter::FixedSolveError as E;
    match error {
        E::Float(source) => rtk_node("float", json!({"cause": float_solve_error_value(source)})),
        E::Ils(source) => rtk_node("ils", json!({"cause": ils_error_value(source)})),
        E::MissingAmbiguity(id) => rtk_node("missing_ambiguity", json!({"ambiguity_id": id})),
        E::MissingWavelength(id) => rtk_node("missing_wavelength", json!({"ambiguity_id": id})),
        E::MissingOffset(id) => rtk_node("missing_offset", json!({"ambiguity_id": id})),
        E::InvalidCovarianceDimensions => rtk_node("invalid_covariance_dimensions", json!({})),
        E::InvalidInput { field, kind } => rtk_node(
            "invalid_input",
            json!({"field": field, "kind": rtk_input_kind_name(*kind)}),
        ),
        E::SingularGeometry => rtk_node("singular_geometry", json!({})),
        E::IncompleteResidualPair => rtk_node("incomplete_residual_pair", json!({})),
        E::ReceiverAntenna(source) => rtk_node(
            "receiver_antenna",
            json!({"cause": receiver_antenna_error_value(*source)}),
        ),
    }
}

fn residual_outlier_value(outlier: &sidereon_core::rtk_filter::ResidualValidationOutlier) -> Value {
    json!({
        "epoch_index": outlier.epoch_index,
        "satellite_id": outlier.satellite_id,
        "reference_satellite_id": outlier.reference_satellite_id,
        "ambiguity_id": outlier.ambiguity_id,
        "component": match outlier.kind {
            sidereon_core::rtk_filter::ResidualComponentKind::Code => "code",
            sidereon_core::rtk_filter::ResidualComponentKind::Phase => "phase",
        },
        "residual_m": engine_f64(outlier.residual_m),
        "sigma_m": engine_f64(outlier.sigma_m),
        "normalized_residual": engine_f64(outlier.normalized_residual),
        "threshold_sigma": engine_f64(outlier.threshold_sigma),
    })
}

fn validated_fixed_solve_error_value(
    error: &sidereon_core::rtk_filter::ValidatedFixedSolveError,
) -> Value {
    use sidereon_core::rtk_filter::ValidatedFixedSolveError as E;
    match error {
        E::Fixed(source) => rtk_node("fixed", json!({"cause": fixed_solve_error_value(source)})),
        E::ResidualValidationFailed {
            outlier,
            exclusions,
        } => rtk_node(
            "residual_validation_failed",
            json!({
                "outlier": residual_outlier_value(outlier),
                "exclusions": exclusions.iter().map(residual_outlier_value).collect::<Vec<_>>(),
            }),
        ),
        E::DuplicateAmbiguityId {
            ambiguity_id,
            first_satellite_id,
            second_satellite_id,
        } => rtk_node(
            "duplicate_ambiguity_id",
            json!({
                "ambiguity_id": ambiguity_id,
                "first_satellite_id": first_satellite_id,
                "second_satellite_id": second_satellite_id,
            }),
        ),
        E::Underdetermined {
            row_count,
            unknown_count,
        } => rtk_node(
            "underdetermined",
            json!({"row_count": row_count, "unknown_count": unknown_count}),
        ),
    }
}

/// Complete current `RtkStaticArcError` tree.
pub(crate) fn rtk_static_arc_error_value(
    error: &sidereon_core::rtk_filter::RtkStaticArcError,
) -> Value {
    use sidereon_core::rtk_filter::RtkStaticArcError as E;
    match error {
        E::Arc(source) => rtk_node("arc", json!({"cause": rtk_arc_error_value(source)})),
        E::Float(source) => rtk_node("float", json!({"cause": float_solve_error_value(source)})),
        E::Fixed(source) => rtk_node(
            "fixed",
            json!({"cause": validated_fixed_solve_error_value(source)}),
        ),
    }
}

pub(crate) fn degrade_reason_name(
    reason: sidereon_core::astro::time::DegradeReason,
) -> &'static str {
    use sidereon_core::astro::time::DegradeReason as D;
    match reason {
        D::BeforeCoverage => "before_coverage",
        D::AfterCoverage => "after_coverage",
    }
}

pub(crate) fn doppler_error_value(error: &sidereon_core::astro::doppler::DopplerError) -> Value {
    use sidereon_core::astro::doppler::DopplerError as E;
    match error {
        E::FrameTransform(cause) => rtk_node(
            "frame_transform",
            json!({"cause": crate::orbit_fit::frame_transform_error_value(cause)}),
        ),
    }
}

pub(crate) fn carrier_phase_error_value(
    error: sidereon_core::carrier_phase::CarrierPhaseError,
) -> Value {
    use sidereon_core::carrier_phase::CarrierPhaseError as E;
    rtk_node(
        match error {
            E::EqualFrequencies => "equal_frequencies",
            E::InvalidFrequency => "invalid_frequency",
            E::InvalidObservation => "invalid_observation",
            E::InvalidThreshold => "invalid_threshold",
        },
        json!({}),
    )
}

pub(crate) fn ionosphere_free_error_value(
    error: &sidereon_core::combinations::IonosphereFreeError,
) -> Value {
    use sidereon_core::combinations::IonosphereFreeError as E;
    match error {
        E::UnknownSystem(system) => {
            rtk_node("unknown_system", json!({"system": system.to_string()}))
        }
        E::UnknownBand { system, band } => rtk_node(
            "unknown_band",
            json!({"system": system.to_string(), "band": band}),
        ),
        E::EqualFrequencies => rtk_node("equal_frequencies", json!({})),
        E::InvalidFrequency => rtk_node("invalid_frequency", json!({})),
        E::InvalidObservation => rtk_node("invalid_observation", json!({})),
    }
}

fn wide_lane_error_value(error: &sidereon_core::rtk::WideLaneError) -> Value {
    use sidereon_core::rtk::WideLaneError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::ReferenceSatelliteMissing(satellite_id) => rtk_node(
            "reference_satellite_missing",
            json!({"satellite_id": satellite_id}),
        ),
        E::WideLaneFailed {
            satellite_id,
            reason,
        } => rtk_node(
            "wide_lane_failed",
            json!({
                "satellite_id": satellite_id,
                "cause": carrier_phase_error_value(*reason),
            }),
        ),
        E::TooFewWideLaneEpochs {
            ambiguity_id,
            count,
            minimum,
        } => rtk_node(
            "too_few_wide_lane_epochs",
            json!({"ambiguity_id": ambiguity_id, "count": count, "minimum": minimum}),
        ),
        E::WideLaneNotInteger {
            ambiguity_id,
            mean_cycles,
            fixed_cycles,
        } => rtk_node(
            "wide_lane_not_integer",
            json!({
                "ambiguity_id": ambiguity_id,
                "mean_cycles": engine_f64(*mean_cycles),
                "fixed_cycles": fixed_cycles,
            }),
        ),
    }
}

fn ionosphere_free_baseline_error_value(
    error: &sidereon_core::rtk::IonosphereFreeBaselineError,
) -> Value {
    use sidereon_core::rtk::IonosphereFreeBaselineError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::NoEpochs => rtk_node("no_epochs", json!({})),
        E::InconsistentFrequencies(satellite_id) => rtk_node(
            "inconsistent_frequencies",
            json!({"satellite_id": satellite_id}),
        ),
        E::NarrowLaneFailed(source) => rtk_node(
            "narrow_lane_failed",
            json!({"cause": ionosphere_free_error_value(source)}),
        ),
        E::IonosphereFreeFailed {
            satellite_id,
            reason,
        } => rtk_node(
            "ionosphere_free_failed",
            json!({
                "satellite_id": satellite_id,
                "cause": ionosphere_free_error_value(reason),
            }),
        ),
    }
}

pub(crate) fn rtk_wide_lane_arc_error_value(
    error: &sidereon_core::rtk_filter::RtkWideLaneArcError,
) -> Value {
    use sidereon_core::rtk_filter::RtkWideLaneArcError as E;
    match error {
        E::EmptyEpochs => rtk_node("empty_epochs", json!({})),
        E::Reference(source) => rtk_node(
            "reference",
            json!({"cause": double_difference_error_value(source)}),
        ),
        E::CycleSlipPrep(source) => rtk_node(
            "cycle_slip_prep",
            json!({"cause": cycle_slip_error_value(source)}),
        ),
        E::WideLane(source) => {
            rtk_node("wide_lane", json!({"cause": wide_lane_error_value(source)}))
        }
    }
}

pub(crate) fn rtk_ionosphere_free_arc_error_value(
    error: &sidereon_core::rtk_filter::RtkIonosphereFreeArcError,
) -> Value {
    use sidereon_core::rtk_filter::RtkIonosphereFreeArcError as E;
    match error {
        E::EmptyEpochs => rtk_node("empty_epochs", json!({})),
        E::Reference(source) => rtk_node(
            "reference",
            json!({"cause": double_difference_error_value(source)}),
        ),
        E::IonosphereFree(source) => rtk_node(
            "ionosphere_free",
            json!({"cause": ionosphere_free_baseline_error_value(source)}),
        ),
    }
}

pub(crate) fn rtk_wide_lane_fixed_arc_error_value(
    error: &sidereon_core::rtk_filter::RtkWideLaneFixedArcError,
) -> Value {
    use sidereon_core::rtk_filter::RtkWideLaneFixedArcError as E;
    match error {
        E::UnsupportedMultiGnss => rtk_node("unsupported_multi_gnss", json!({})),
        E::WideLane(source) => rtk_node(
            "wide_lane",
            json!({"cause": rtk_wide_lane_arc_error_value(source)}),
        ),
        E::IonosphereFree(source) => rtk_node(
            "ionosphere_free",
            json!({"cause": rtk_ionosphere_free_arc_error_value(source)}),
        ),
        E::Static(source) => rtk_node(
            "static",
            json!({"cause": rtk_static_arc_error_value(source)}),
        ),
        E::Sequential(source) => {
            rtk_node("sequential", json!({"cause": rtk_arc_error_value(source)}))
        }
    }
}

fn dted_horizontal_datum_value(datum: &sidereon_core::terrain::DtedHorizontalDatum) -> Value {
    use sidereon_core::terrain::DtedHorizontalDatum as D;
    match datum {
        D::Wgs84 => rtk_node("wgs84", json!({})),
        D::Wgs72 => rtk_node("wgs72", json!({})),
        D::Unstated => rtk_node("unstated", json!({})),
        D::Other(name) => rtk_node("other", json!({"name": name})),
        _ => rtk_node("unknown", json!({"message": datum.to_string()})),
    }
}

fn dted_tile_error_value(error: &sidereon_core::terrain::DtedTileError) -> Value {
    use sidereon_core::terrain::DtedTileError as E;
    match error {
        E::Io { path, message } => rtk_node("io", json!({"path": path, "message": message})),
        E::TooShort { path } => rtk_node("too_short", json!({"path": path})),
        E::MissingUhl1 { path } => rtk_node("missing_uhl1", json!({"path": path})),
        E::InvalidEncoding(message) => rtk_node("invalid_encoding", json!({"message": message})),
        E::InvalidField(message) => rtk_node("invalid_field", json!({"message": message})),
        E::InvalidDimensions {
            path,
            lon_count,
            lat_count,
        } => rtk_node(
            "invalid_dimensions",
            json!({
                "path": path,
                "lon_count": lon_count,
                "lat_count": lat_count,
            }),
        ),
        E::Truncated {
            path,
            actual,
            expected,
        } => rtk_node(
            "truncated",
            json!({
                "path": path,
                "actual": actual,
                "expected": expected,
            }),
        ),
        E::Outside {
            longitude,
            latitude,
            origin_longitude,
            origin_latitude,
        } => rtk_node(
            "outside",
            json!({
                "longitude": engine_f64(*longitude),
                "latitude": engine_f64(*latitude),
                "origin_longitude": engine_f64(*origin_longitude),
                "origin_latitude": engine_f64(*origin_latitude),
            }),
        ),
        E::PostingIndexOutOfBounds {
            longitude_index,
            latitude_index,
        } => rtk_node(
            "posting_index_out_of_bounds",
            json!({
                "longitude_index": longitude_index,
                "latitude_index": latitude_index,
            }),
        ),
        E::MissingDataSentinel { longitude_index } => rtk_node(
            "missing_data_sentinel",
            json!({"longitude_index": longitude_index}),
        ),
        E::Checksum {
            longitude_index,
            checksum,
            sum,
        } => rtk_node(
            "checksum",
            json!({
                "longitude_index": longitude_index,
                "checksum": checksum,
                "sum": sum,
            }),
        ),
        E::EmptyCoordinate => rtk_node("empty_coordinate", json!({})),
        E::InvalidHemisphere { hemisphere } => rtk_node(
            "invalid_hemisphere",
            json!({"hemisphere": hemisphere.to_string()}),
        ),
        E::NegativePostingIndex { index } => {
            rtk_node("negative_posting_index", json!({"index": index}))
        }
        E::CoordinateOutOfRange { field, text } => rtk_node(
            "coordinate_out_of_range",
            json!({"field": field, "text": text}),
        ),
        E::WrongHemisphere {
            field,
            hemisphere,
            expected,
        } => rtk_node(
            "wrong_hemisphere",
            json!({
                "field": field,
                "hemisphere": hemisphere.to_string(),
                "expected": expected,
            }),
        ),
        E::OriginNotWholeDegree { field, text } => rtk_node(
            "origin_not_whole_degree",
            json!({"field": field, "text": text}),
        ),
        E::IntervalCountMismatch {
            field,
            interval_tenths_arcsec,
            count,
        } => rtk_node(
            "interval_count_mismatch",
            json!({
                "field": field,
                "interval_tenths_arcsec": interval_tenths_arcsec,
                "count": count,
            }),
        ),
        E::ProfileLongitudeCountMismatch {
            longitude_index,
            declared,
        } => rtk_node(
            "profile_longitude_count_mismatch",
            json!({
                "longitude_index": longitude_index,
                "declared": declared,
            }),
        ),
        E::UnsupportedPartialProfile {
            longitude_index,
            first_latitude_index,
        } => rtk_node(
            "unsupported_partial_profile",
            json!({
                "longitude_index": longitude_index,
                "first_latitude_index": first_latitude_index,
            }),
        ),
        E::NullPosting {
            longitude_index,
            latitude_index,
        } => rtk_node(
            "null_posting",
            json!({
                "longitude_index": longitude_index,
                "latitude_index": latitude_index,
            }),
        ),
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

fn ionex_coverage_error_value(
    error: &sidereon_core::atmosphere::ionosphere::IonexCoverageError,
) -> Value {
    use sidereon_core::atmosphere::ionosphere::IonexCoverageError as E;
    let kind = match error {
        E::EpochBeforeFirstMap => "epoch_before_first_map",
        E::EpochAfterLastMap => "epoch_after_last_map",
        E::LatitudeOutOfRange => "latitude_out_of_range",
        E::LongitudeOutOfRange => "longitude_out_of_range",
    };
    rtk_node(kind, json!({}))
}

fn ionex_missing_nodes_value(
    nodes: &sidereon_core::atmosphere::ionosphere::IonexMissingNodes,
) -> Value {
    rtk_node(
        "missing_nodes",
        json!({
            "map_number": nodes.map_number,
            "lat_index": nodes.lat_index,
            "lon_index": nodes.lon_index,
            "lon_index_next": nodes.lon_index_next,
            "missing": nodes.missing,
        }),
    )
}

fn ionex_node_gap_value(gap: &sidereon_core::atmosphere::ionosphere::IonexNodeGap) -> Value {
    rtk_node(
        "node_gap",
        json!({
            "earlier": gap.earlier.as_ref().map(ionex_missing_nodes_value),
            "later": gap.later.as_ref().map(ionex_missing_nodes_value),
        }),
    )
}

fn ionex_mapping_declaration_value(
    decl: &sidereon_core::atmosphere::ionosphere::IonexMappingDeclaration,
) -> Value {
    use sidereon_core::atmosphere::ionosphere::IonexMappingDeclaration as D;
    match decl {
        D::Declared(func) => rtk_node("declared", json!({"code": func.code()})),
        D::Absent => rtk_node("absent", json!({})),
    }
}

fn ionex_slant_refusal_value(
    refusal: &sidereon_core::atmosphere::ionosphere::IonexSlantRefusal,
) -> Value {
    use sidereon_core::atmosphere::ionosphere::IonexSlantRefusal as R;
    match refusal {
        R::VaryingHeights {
            map_number,
            lat_index,
            lon_index,
        } => rtk_node(
            "varying_heights",
            json!({
                "map_number": map_number,
                "lat_index": lat_index,
                "lon_index": lon_index,
            }),
        ),
        R::HeightNotAvailable {
            map_number,
            lat_index,
            lon_index,
        } => rtk_node(
            "height_not_available",
            json!({
                "map_number": map_number,
                "lat_index": lat_index,
                "lon_index": lon_index,
            }),
        ),
        R::MappingFunction(decl) => rtk_node(
            "mapping_function",
            json!({"declaration": ionex_mapping_declaration_value(decl)}),
        ),
        _ => rtk_node("unknown", json!({"message": refusal.to_string()})),
    }
}

fn ionex_epoch_error_value(
    error: &sidereon_core::atmosphere::ionosphere::IonexEpochError,
) -> Value {
    use sidereon_core::atmosphere::ionosphere::IonexEpochError as E;
    match error {
        E::NotWholeSecond { scale } => {
            rtk_node("not_whole_second", json!({"scale": scale.abbrev()}))
        }
        E::FractionalUtcSecond { scale } => {
            rtk_node("fractional_utc_second", json!({"scale": scale.abbrev()}))
        }
        E::NoExactUtcOffset { scale } => {
            rtk_node("no_exact_utc_offset", json!({"scale": scale.abbrev()}))
        }
        E::InsertedLeapSecond { scale } => {
            rtk_node("inserted_leap_second", json!({"scale": scale.abbrev()}))
        }
        E::BeforeIntegerLeapSeconds { scale } => rtk_node(
            "before_integer_leap_seconds",
            json!({"scale": scale.abbrev()}),
        ),
        E::OutOfRange { scale } => rtk_node("out_of_range", json!({"scale": scale.abbrev()})),
        E::YearOutOfField { utc_j2000_s } => {
            rtk_node("year_out_of_field", json!({"utc_j2000_s": utc_j2000_s}))
        }
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

fn sp3_epoch_interval_rejection_name(
    reason: sidereon_core::ephemeris::Sp3EpochIntervalRejection,
) -> &'static str {
    use sidereon_core::ephemeris::Sp3EpochIntervalRejection as R;
    match reason {
        R::NotFinite => "not_finite",
        R::NotPositive => "not_positive",
        R::NotWholeTicks => "not_whole_ticks",
        R::BeyondTickResolution => "beyond_tick_resolution",
        R::OutsideSpecificationRange => "outside_specification_range",
        _ => "unknown",
    }
}

fn sp3_epoch_interval_error_value(
    error: &sidereon_core::ephemeris::Sp3EpochIntervalError,
) -> Value {
    rtk_node(
        "sp3_epoch_interval",
        json!({
            "field": error.field,
            "value": engine_f64(error.value),
            "reason": sp3_epoch_interval_rejection_name(error.reason),
        }),
    )
}

fn merge_tolerance_field_name(
    field: sidereon_core::ephemeris::MergeToleranceField,
) -> &'static str {
    use sidereon_core::ephemeris::MergeToleranceField as F;
    match field {
        F::Position => "position",
        F::Clock => "clock",
        F::OutlierPosition => "outlier_position",
        F::OutlierClock => "outlier_clock",
        _ => "unknown",
    }
}

fn sp3_merge_tolerance_error_value(error: &sidereon_core::ephemeris::MergeToleranceError) -> Value {
    rtk_node(
        "sp3_merge_tolerance",
        json!({
            "field": merge_tolerance_field_name(error.field),
            "value": engine_f64(error.value),
        }),
    )
}

fn continuity_option_rejection_name(
    reason: sidereon_core::ephemeris::ContinuityOptionRejection,
) -> &'static str {
    use sidereon_core::ephemeris::ContinuityOptionRejection as R;
    match reason {
        R::NotFinite => "not_finite",
        R::Negative => "negative",
        _ => "unknown",
    }
}

fn continuity_options_error_value(
    error: &sidereon_core::ephemeris::ContinuityOptionsError,
) -> Value {
    rtk_node(
        "continuity_options",
        json!({
            "field": error.field,
            "value": engine_f64(error.value),
            "reason": continuity_option_rejection_name(error.reason),
        }),
    )
}

fn sbas_encode_error_value(error: &sidereon_core::sbas::SbasEncodeError) -> Value {
    use sidereon_core::sbas::SbasEncodeError as E;
    match error {
        E::FieldOutOfRange {
            message_type,
            field,
            index,
            value,
            width,
            signed,
        } => rtk_node(
            "field_out_of_range",
            json!({
                "message_type": message_type,
                "field": field,
                "index": index,
                "value": value.to_string(),
                "width": width,
                "signed": signed,
            }),
        ),
        E::UnrecognizedPreamble { preamble } => {
            rtk_node("unrecognized_preamble", json!({"preamble": preamble}))
        }
        E::MessageType {
            message_type,
            reason,
        } => rtk_node(
            "message_type",
            json!({
                "message_type": message_type,
                "reason": reason,
            }),
        ),
        E::RawPayload {
            message_type,
            bytes,
            bits_past_payload,
        } => rtk_node(
            "raw_payload",
            json!({
                "message_type": message_type,
                "bytes": bytes,
                "bits_past_payload": bits_past_payload,
            }),
        ),
        E::ReservedLayout {
            message_type,
            part,
            expected,
            found,
        } => rtk_node(
            "reserved_layout",
            json!({
                "message_type": message_type,
                "part": part,
                "expected": expected,
                "found": found,
            }),
        ),
        E::LongTermRecordCount {
            message_type,
            half,
            velocity_code,
            expected,
            found,
        } => rtk_node(
            "long_term_record_count",
            json!({
                "message_type": message_type,
                "half": half,
                "velocity_code": velocity_code,
                "expected": expected,
                "found": found,
            }),
        ),
        E::LongTermFieldNotCarried {
            message_type,
            half,
            record,
            field,
        } => rtk_node(
            "long_term_field_not_carried",
            json!({
                "message_type": message_type,
                "half": half,
                "record": record,
                "field": field,
            }),
        ),
        E::LongTermMissingTimeOfDay { message_type, half } => rtk_node(
            "long_term_missing_time_of_day",
            json!({
                "message_type": message_type,
                "half": half,
            }),
        ),
        E::PadBits { value } => rtk_node("pad_bits", json!({"value": value})),
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

fn rtcm_field_encoding_name(encoding: sidereon_core::rtcm::RtcmFieldEncoding) -> &'static str {
    use sidereon_core::rtcm::RtcmFieldEncoding as E;
    match encoding {
        E::Unsigned => "unsigned",
        E::TwosComplement => "twos_complement",
        E::SignMagnitude => "sign_magnitude",
        _ => "unknown",
    }
}

fn rtcm_record_kind_value(record: &sidereon_core::rtcm::RtcmRecordKind) -> Value {
    use sidereon_core::rtcm::RtcmRecordKind as R;
    match record {
        R::StationCoordinates => rtk_node("station_coordinates", json!({})),
        R::AntennaDescriptor => rtk_node("antenna_descriptor", json!({})),
        R::Msm { system, kind } => rtk_node(
            "msm",
            json!({
                "system": format!("{system:?}"),
                "kind": format!("{kind:?}"),
            }),
        ),
        R::Ssr { system, kind } => rtk_node(
            "ssr",
            json!({
                "system": format!("{system:?}"),
                "kind": format!("{kind:?}"),
            }),
        ),
        R::LegacyObservations => rtk_node("legacy_observations", json!({})),
        R::SystemParameters => rtk_node("system_parameters", json!({})),
        R::Text => rtk_node("text", json!({})),
        R::Network { family } => rtk_node("network", json!({"family": family})),
        R::Transformation { family } => rtk_node("transformation", json!({"family": family})),
        R::GlonassCodePhaseBiases => rtk_node("glonass_code_phase_biases", json!({})),
        R::SsrVtec { message_number } => {
            rtk_node("ssr_vtec", json!({"message_number": message_number}))
        }
        _ => rtk_node("other", json!({"debug": format!("{record:?}")})),
    }
}

fn msm_optional_problem_value(problem: &sidereon_core::rtcm::MsmOptionalProblem) -> Value {
    use sidereon_core::rtcm::MsmOptionalProblem as P;
    match problem {
        P::Missing => rtk_node("missing", json!({})),
        P::NotCarried => rtk_node("not_carried", json!({})),
        P::InvalidValue(value) => rtk_node("invalid_value", json!({"value": value})),
        _ => rtk_node("other", json!({"debug": format!("{problem:?}")})),
    }
}

fn msm_mask_problem_value(problem: &sidereon_core::rtcm::MsmMaskProblem) -> Value {
    use sidereon_core::rtcm::MsmMaskProblem as P;
    match problem {
        P::SatelliteOutsideMask { satellite } => {
            rtk_node("satellite_outside_mask", json!({"satellite": satellite}))
        }
        P::SatelliteListedTwice { satellite } => {
            rtk_node("satellite_listed_twice", json!({"satellite": satellite}))
        }
        P::SignalOutsideMask { signal } => {
            rtk_node("signal_outside_mask", json!({"signal": signal}))
        }
        P::SignalNotInMask { signal, mask } => rtk_node(
            "signal_not_in_mask",
            json!({"signal": signal, "mask": mask}),
        ),
        P::SignalSatelliteNotListed { signal, satellite } => rtk_node(
            "signal_satellite_not_listed",
            json!({"signal": signal, "satellite": satellite}),
        ),
        P::CellListedTwice { satellite, signal } => rtk_node(
            "cell_listed_twice",
            json!({"satellite": satellite, "signal": signal}),
        ),
        _ => rtk_node("other", json!({"debug": format!("{problem:?}")})),
    }
}

fn rtcm_departure_value(departure: &sidereon_core::rtcm::RtcmDeparture) -> Value {
    use sidereon_core::rtcm::RtcmDeparture as D;
    match departure {
        D::FrameReservedBits { reserved } => {
            rtk_node("frame_reserved_bits", json!({"reserved": reserved}))
        }
        D::TrailingBits {
            message_number,
            bits,
        } => rtk_node(
            "trailing_bits",
            json!({"message_number": message_number, "bits": bits}),
        ),
        D::MsmCellMaskOver64 {
            message_number,
            cells,
        } => rtk_node(
            "msm_cell_mask_over_64",
            json!({"message_number": message_number, "cells": cells}),
        ),
        D::OrderExceedsDegree {
            message_number,
            layer_index,
            degree,
            order,
        } => rtk_node(
            "order_exceeds_degree",
            json!({
                "message_number": message_number,
                "layer_index": layer_index,
                "degree": degree,
                "order": order,
            }),
        ),
        D::SsrRecordsShort {
            message_number,
            declared,
            read,
        } => rtk_node(
            "ssr_records_short",
            json!({
                "message_number": message_number,
                "declared": declared,
                "read": read,
            }),
        ),
        D::RecordsShort {
            message_number,
            declared,
            read,
        } => rtk_node(
            "records_short",
            json!({
                "message_number": message_number,
                "declared": declared,
                "read": read,
            }),
        ),
        _ => rtk_node("other", json!({"debug": format!("{departure:?}")})),
    }
}

fn rtcm_encode_error_value(error: &sidereon_core::rtcm::RtcmEncodeError) -> Value {
    use sidereon_core::rtcm::RtcmEncodeError as E;
    match error {
        E::FieldOutOfRange {
            message_number,
            field,
            value,
            width,
            encoding,
        } => rtk_node(
            "field_out_of_range",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value.to_string(),
                "width": width,
                "encoding": rtcm_field_encoding_name(*encoding),
            }),
        ),
        E::NegativeZeroWithValue {
            message_number,
            field,
            value,
        } => rtk_node(
            "negative_zero_with_value",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value,
            }),
        ),
        E::NegativeZeroMask {
            message_number,
            mask,
        } => rtk_node(
            "negative_zero_mask",
            json!({
                "message_number": message_number,
                "mask": mask,
            }),
        ),
        E::MessageNumber {
            message_number,
            record,
        } => rtk_node(
            "message_number",
            json!({
                "message_number": message_number,
                "record": rtcm_record_kind_value(record),
            }),
        ),
        E::FieldPresence {
            message_number,
            record,
            field,
            carried,
        } => rtk_node(
            "field_presence",
            json!({
                "message_number": message_number,
                "record": rtcm_record_kind_value(record),
                "field": field,
                "carried": carried,
            }),
        ),
        E::SatelliteFieldPresence {
            message_number,
            record,
            satellite,
            field,
            carried,
        } => rtk_node(
            "satellite_field_presence",
            json!({
                "message_number": message_number,
                "record": rtcm_record_kind_value(record),
                "satellite": satellite,
                "field": field,
                "carried": carried,
            }),
        ),
        E::CountMismatch {
            message_number,
            field,
            expected,
            actual,
        } => rtk_node(
            "count_mismatch",
            json!({
                "message_number": message_number,
                "field": field,
                "expected": expected,
                "actual": actual,
            }),
        ),
        E::ValueOutOfRange {
            message_number,
            field,
            value,
            minimum,
            maximum,
        } => rtk_node(
            "value_out_of_range",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value.to_string(),
                "minimum": minimum.to_string(),
                "maximum": maximum.to_string(),
            }),
        ),
        E::NonLatin1Character { field, character } => rtk_node(
            "non_latin1_character",
            json!({
                "field": field,
                "character": character.to_string(),
            }),
        ),
        E::SatelliteIdOutOfRange {
            message_number,
            field,
            value,
            width,
        } => rtk_node(
            "satellite_id_out_of_range",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value,
                "width": width,
            }),
        ),
        E::SsrSatelliteIdOutOfRange {
            message_number,
            value,
            width,
        } => rtk_node(
            "ssr_satellite_id_out_of_range",
            json!({
                "message_number": message_number,
                "value": value,
                "width": width,
            }),
        ),
        E::SsrRecordsNotCarried {
            message_number,
            kind,
            records,
            count,
        } => rtk_node(
            "ssr_records_not_carried",
            json!({
                "message_number": message_number,
                "kind": format!("{kind:?}"),
                "records": records,
                "count": count,
            }),
        ),
        E::SsrCombinedRecordCounts {
            message_number,
            orbit,
            clock,
        } => rtk_node(
            "ssr_combined_record_counts",
            json!({
                "message_number": message_number,
                "orbit": orbit,
                "clock": clock,
            }),
        ),
        E::SsrCombinedSatelliteMismatch {
            message_number,
            index,
            orbit_satellite,
            clock_satellite,
        } => rtk_node(
            "ssr_combined_satellite_mismatch",
            json!({
                "message_number": message_number,
                "index": index,
                "orbit_satellite": orbit_satellite,
                "clock_satellite": clock_satellite,
            }),
        ),
        E::SsrHighRateClockTerms {
            message_number,
            satellite,
            c1,
            c2,
        } => rtk_node(
            "ssr_high_rate_clock_terms",
            json!({
                "message_number": message_number,
                "satellite": satellite,
                "c1": c1,
                "c2": c2,
            }),
        ),
        E::SsrSatelliteCount {
            message_number,
            declared,
            records,
        } => rtk_node(
            "ssr_satellite_count",
            json!({
                "message_number": message_number,
                "declared": declared,
                "records": records,
            }),
        ),
        E::MsmMask {
            message_number,
            problem,
        } => rtk_node(
            "msm_mask",
            json!({
                "message_number": message_number,
                "problem": msm_mask_problem_value(problem),
            }),
        ),
        E::MsmOptional {
            message_number,
            kind,
            satellite,
            signal,
            field,
            problem,
        } => rtk_node(
            "msm_optional",
            json!({
                "message_number": message_number,
                "kind": format!("{kind:?}"),
                "satellite": satellite,
                "signal": signal,
                "field": format!("{field:?}"),
                "problem": msm_optional_problem_value(problem),
            }),
        ),
        E::TrailingZeroBits {
            message_number,
            bits,
        } => rtk_node(
            "trailing_zero_bits",
            json!({
                "message_number": message_number,
                "bits": bits,
            }),
        ),
        E::StrictDeparture(departure) => rtk_node(
            "strict_departure",
            json!({"departure": rtcm_departure_value(departure)}),
        ),
        E::UnsupportedBodyTooShort { message_number } => rtk_node(
            "unsupported_body_too_short",
            json!({"message_number": message_number}),
        ),
        E::UnsupportedBodyNumber {
            message_number,
            carried,
        } => rtk_node(
            "unsupported_body_number",
            json!({
                "message_number": message_number,
                "carried": carried,
            }),
        ),
        E::UnsupportedDecodedNumber { message_number } => rtk_node(
            "unsupported_decoded_number",
            json!({"message_number": message_number}),
        ),
        E::FrameBodyTooLong { len } => rtk_node("frame_body_too_long", json!({"len": len})),
        E::FrameReservedOutOfRange { value } => {
            rtk_node("frame_reserved_out_of_range", json!({"value": value}))
        }
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

fn satellite_id_error_value(error: &sidereon_core::SatelliteIdError) -> Value {
    use sidereon_core::SatelliteIdError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
    }
}

fn lnav_record_error_value(error: &sidereon_core::ephemeris::LnavRecordError) -> Value {
    use sidereon_core::ephemeris::LnavRecordError as E;
    match error {
        E::NotGps(sat) => rtk_node("not_gps", json!({"satellite_id": sat.to_string()})),
        E::InvalidEpoch(reason) => rtk_node("invalid_epoch", json!({"reason": reason})),
        E::WeekMismatch {
            full_week,
            decoded_week,
        } => rtk_node(
            "week_mismatch",
            json!({
                "full_week": full_week,
                "decoded_week": decoded_week,
            }),
        ),
        E::NoUraPrediction(index) => rtk_node("no_ura_prediction", json!({"index": index})),
        E::FitIntervalUnsupported {
            fit_interval_flag,
            iode,
            iodc,
        } => rtk_node(
            "fit_interval_unsupported",
            json!({
                "fit_interval_flag": fit_interval_flag,
                "iode": iode,
                "iodc": iodc,
            }),
        ),
    }
}

fn vtec_evaluation_problem_value(problem: &sidereon_core::rtcm::VtecEvaluationProblem) -> Value {
    use sidereon_core::rtcm::VtecEvaluationProblem as P;
    match problem {
        P::ComputationTime => rtk_node("computation_time", json!({})),
        P::Frequency => rtk_node("frequency", json!({})),
        P::NonFiniteCoordinates => rtk_node("non_finite_coordinates", json!({})),
        P::MessageIdentity { message_number } => rtk_node(
            "message_identity",
            json!({"message_number": message_number}),
        ),
        P::LayerCount { layers } => rtk_node("layer_count", json!({"layers": layers})),
        P::InvalidGeometry => rtk_node("invalid_geometry", json!({})),
        P::BelowHorizon => rtk_node("below_horizon", json!({})),
        P::LayerDegreeOrder {
            layer_index,
            degree,
            order,
        } => rtk_node(
            "layer_degree_order",
            json!({
                "layer_index": layer_index,
                "degree": degree,
                "order": order,
            }),
        ),
        P::CoefficientCounts {
            layer_index,
            cosine_expected,
            cosine_actual,
            sine_expected,
            sine_actual,
        } => rtk_node(
            "coefficient_counts",
            json!({
                "layer_index": layer_index,
                "cosine_expected": cosine_expected,
                "cosine_actual": cosine_actual,
                "sine_expected": sine_expected,
                "sine_actual": sine_actual,
            }),
        ),
        P::UnavailableCoefficient { layer_index } => rtk_node(
            "unavailable_coefficient",
            json!({"layer_index": layer_index}),
        ),
        P::ShellNotAboveReceiver { layer_index } => rtk_node(
            "shell_not_above_receiver",
            json!({"layer_index": layer_index}),
        ),
        P::MissingCoefficient {
            layer_index,
            field,
            index,
        } => rtk_node(
            "missing_coefficient",
            json!({
                "layer_index": layer_index,
                "field": field,
                "index": index,
            }),
        ),
        P::InvalidMappingFactor { layer_index } => rtk_node(
            "invalid_mapping_factor",
            json!({"layer_index": layer_index}),
        ),
        P::PhysicalResultOutOfRange { field } => {
            rtk_node("physical_result_out_of_range", json!({"field": field}))
        }
        _ => rtk_node("other", json!({"debug": format!("{problem:?}")})),
    }
}

fn rtcm_conversion_error_value(error: &sidereon_core::rtcm::RtcmConversionError) -> Value {
    use sidereon_core::rtcm::RtcmConversionError as E;
    match error {
        E::SatelliteIdOutOfRange {
            message_number,
            field,
            value,
            width,
        } => rtk_node(
            "satellite_id_out_of_range",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value,
                "width": width,
            }),
        ),
        E::InvalidSatellite {
            message_number,
            field,
            value,
            error,
        } => rtk_node(
            "invalid_satellite",
            json!({
                "message_number": message_number,
                "field": field,
                "value": value,
                "error": satellite_id_error_value(error),
            }),
        ),
        E::SbasPrnOutsideWindow {
            value,
            broadcast_prn,
        } => rtk_node(
            "sbas_prn_outside_window",
            json!({
                "value": value,
                "broadcast_prn": broadcast_prn,
            }),
        ),
        E::NoLnavRecord { value, satellite } => rtk_node(
            "no_lnav_record",
            json!({
                "value": value,
                "satellite_id": satellite.to_string(),
            }),
        ),
        E::WeekMismatch {
            message_number,
            full_week,
            week,
        } => rtk_node(
            "week_mismatch",
            json!({
                "message_number": message_number,
                "full_week": full_week,
                "week": week,
            }),
        ),
        E::NavicWeekMismatch { full_week, week } => rtk_node(
            "navic_week_mismatch",
            json!({
                "full_week": full_week,
                "week": week,
            }),
        ),
        E::TimeNotRepresentable { field } => {
            rtk_node("time_not_representable", json!({"field": field}))
        }
        E::GalileoWeekOverflow => rtk_node("galileo_week_overflow", json!({})),
        E::SisaSpare { index } => rtk_node("sisa_spare", json!({"index": index})),
        E::SisaNoPrediction => rtk_node("sisa_no_prediction", json!({})),
        E::UraOutOfRange { system, index } => rtk_node(
            "ura_out_of_range",
            json!({
                "system": format!("{system:?}"),
                "index": index,
            }),
        ),
        E::UraNoPrediction { system, index } => rtk_node(
            "ura_no_prediction",
            json!({
                "system": format!("{system:?}"),
                "index": index,
            }),
        ),
        E::FitInterval(err) => rtk_node(
            "fit_interval",
            json!({"cause": lnav_record_error_value(err)}),
        ),
        E::VtecEvaluation(problem) => rtk_node(
            "vtec_evaluation",
            json!({"cause": vtec_evaluation_problem_value(problem)}),
        ),
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

pub(crate) fn core_error_value(error: &sidereon_core::Error) -> Value {
    use sidereon_core::Error as E;
    match error {
        E::Parse(message) => rtk_node("parse", json!({"message": message})),
        E::InvalidInput(message) => rtk_node("invalid_input", json!({"message": message})),
        E::UnknownSatellite(sat) => rtk_node(
            "unknown_satellite",
            json!({"satellite_id": sat.to_string()}),
        ),
        E::MissingGlonassChannel => rtk_node("missing_glonass_channel", json!({})),
        E::EpochOutOfRange => rtk_node("epoch_out_of_range", json!({})),
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
        E::MissingTerrainTile {
            lat_index,
            lon_index,
        } => rtk_node(
            "missing_terrain_tile",
            json!({"lat_index": lat_index, "lon_index": lon_index}),
        ),
        E::UnknownTerrainElevation {
            lat_index,
            lon_index,
            latitude_posting,
            longitude_posting,
        } => rtk_node(
            "unknown_terrain_elevation",
            json!({
                "lat_index": lat_index,
                "lon_index": lon_index,
                "latitude_posting": latitude_posting,
                "longitude_posting": longitude_posting,
            }),
        ),
        E::NonWgs84TerrainTile {
            lat_index,
            lon_index,
            datum,
        } => rtk_node(
            "non_wgs84_terrain_tile",
            json!({
                "lat_index": lat_index,
                "lon_index": lon_index,
                "datum": dted_horizontal_datum_value(datum),
            }),
        ),
        E::TerrainTile {
            lat_index,
            lon_index,
            error,
        } => rtk_node(
            "terrain_tile",
            json!({
                "lat_index": lat_index,
                "lon_index": lon_index,
                "cause": dted_tile_error_value(error),
            }),
        ),
        E::TerrainTileOrigin {
            path,
            lat_index,
            lon_index,
            origin_latitude,
            origin_longitude,
        } => rtk_node(
            "terrain_tile_origin",
            json!({
                "path": path.to_string_lossy(),
                "lat_index": lat_index,
                "lon_index": lon_index,
                "origin_latitude": origin_latitude,
                "origin_longitude": origin_longitude,
            }),
        ),
        E::IonexOutOfCoverage(err) => rtk_node(
            "ionex_out_of_coverage",
            json!({"cause": ionex_coverage_error_value(err)}),
        ),
        E::IonexNodesNotAvailable(err) => rtk_node(
            "ionex_nodes_not_available",
            json!({"cause": ionex_node_gap_value(err)}),
        ),
        E::IonexSlantUnavailable(err) => rtk_node(
            "ionex_slant_unavailable",
            json!({"cause": ionex_slant_refusal_value(err)}),
        ),
        E::IonexEpoch(err) => rtk_node(
            "ionex_epoch",
            json!({"cause": ionex_epoch_error_value(err)}),
        ),
        E::InsufficientPreciseNodes {
            sat,
            nodes,
            required,
        } => rtk_node(
            "insufficient_precise_nodes",
            json!({
                "satellite_id": sat.to_string(),
                "nodes": nodes,
                "required": required,
            }),
        ),
        E::Sp3EpochInterval(err) => rtk_node(
            "sp3_epoch_interval",
            json!({"cause": sp3_epoch_interval_error_value(err)}),
        ),
        E::Sp3MergeTolerance(err) => rtk_node(
            "sp3_merge_tolerance",
            json!({"cause": sp3_merge_tolerance_error_value(err)}),
        ),
        E::ContinuityOptions(err) => rtk_node(
            "continuity_options",
            json!({"cause": continuity_options_error_value(err)}),
        ),
        E::SbasEncode(err) => rtk_node(
            "sbas_encode",
            json!({"cause": sbas_encode_error_value(err)}),
        ),
        E::RtcmEncode(err) => rtk_node(
            "rtcm_encode",
            json!({"cause": rtcm_encode_error_value(err)}),
        ),
        E::RtcmConversion(err) => rtk_node(
            "rtcm_conversion",
            json!({"cause": rtcm_conversion_error_value(err)}),
        ),
        other => rtk_node(
            "other",
            json!({
                "message": other.to_string(),
                "debug": format!("{other:?}"),
            }),
        ),
    }
}

fn spp_input_error_kind_name(kind: sidereon_core::positioning::SppInputErrorKind) -> &'static str {
    use sidereon_core::positioning::SppInputErrorKind as K;
    match kind {
        K::NonFinite => "non_finite",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::Missing => "missing",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

pub(crate) fn least_squares_solve_error_value(
    error: &sidereon_core::astro::math::least_squares::SolveError,
) -> Value {
    use sidereon_core::astro::math::least_squares::SolveError as E;
    match error {
        E::SingularJacobian => rtk_node("singular_jacobian", json!({})),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
    }
}

pub(crate) fn spp_error_value(error: &sidereon_core::positioning::SppError) -> Value {
    use sidereon_core::positioning::SppError as E;
    match error {
        E::InvalidInput { field, kind } => rtk_node(
            "invalid_input",
            json!({
                "field": field,
                "kind": spp_input_error_kind_name(*kind),
            }),
        ),
        E::TooFewSatellites { used, required } => rtk_node(
            "too_few_satellites",
            json!({
                "used": used,
                "required": required,
            }),
        ),
        E::Singular(source) => rtk_node(
            "singular",
            json!({"cause": least_squares_solve_error_value(source)}),
        ),
        E::DuplicateObservation { satellite } => rtk_node(
            "duplicate_observation",
            json!({"satellite_id": satellite.to_string()}),
        ),
        E::EphemerisLost { satellite } => rtk_node(
            "ephemeris_lost",
            json!({"satellite_id": satellite.to_string()}),
        ),
        E::SelectionUnsettled { passes } => {
            rtk_node("selection_unsettled", json!({"passes": passes}))
        }
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
    }
}

fn observables_input_error_kind_name(
    kind: sidereon_core::observables::ObservablesInputErrorKind,
) -> &'static str {
    use sidereon_core::observables::ObservablesInputErrorKind as K;
    match kind {
        K::NonFinite => "non_finite",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::Missing => "missing",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

pub(crate) fn observables_error_value(
    error: &sidereon_core::observables::ObservablesError,
) -> Value {
    use sidereon_core::observables::ObservablesError as E;
    match error {
        E::InvalidInput { field, kind } => rtk_node(
            "invalid_input",
            json!({
                "field": field,
                "kind": observables_input_error_kind_name(*kind),
            }),
        ),
        E::NoEphemeris => rtk_node("no_ephemeris", json!({})),
        E::Ephemeris(source) => rtk_node("ephemeris", json!({"cause": core_error_value(source)})),
        E::Media(source) => rtk_node("media", json!({"cause": core_error_value(source)})),
    }
}

pub(crate) fn dgnss_error_value(error: &sidereon_core::dgnss::DgnssError) -> Value {
    use sidereon_core::dgnss::DgnssError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::Spp(source) => rtk_node("spp", json!({"cause": spp_error_value(source)})),
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
    }
}

pub(crate) fn scenario_error_value(error: &sidereon_core::scenario::ScenarioError) -> Value {
    use sidereon_core::scenario::ScenarioError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::ExternalSourceRequired => rtk_node("external_source_required", json!({})),
        E::ExternalSourceMismatch {
            field,
            expected,
            actual,
        } => rtk_node(
            "external_source_mismatch",
            json!({
                "field": field,
                "expected": expected,
                "actual": actual,
            }),
        ),
        E::ExternalIonosphereRequired => rtk_node("external_ionosphere_required", json!({})),
        E::Ionosphere(message) => rtk_node("ionosphere", json!({"message": message})),
        E::NoEphemeris { satellite } => rtk_node(
            "no_ephemeris",
            json!({"satellite_id": satellite.to_string()}),
        ),
        E::Ut1OutsideCoverage { satellite, reason } => rtk_node(
            "ut1_outside_coverage",
            json!({
                "satellite_id": satellite.to_string(),
                "reason": degrade_reason_name(*reason),
            }),
        ),
        E::Observable(source) => rtk_node(
            "observable",
            json!({"cause": observables_error_value(source)}),
        ),
        E::Frame(message) => rtk_node("frame", json!({"message": message})),
    }
}

pub(crate) fn rtk_rinex_arc_error_value(
    error: &sidereon_core::rtk_filter::RtkRinexArcError,
) -> Value {
    use sidereon_core::rtk_filter::RtkRinexArcError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::Observation(source) => {
            rtk_node("observation", json!({"cause": core_error_value(source)}))
        }
        E::Ephemeris {
            satellite_id,
            epoch_j2000_s,
            reason,
        } => rtk_node(
            "ephemeris",
            json!({
                "satellite_id": satellite_id,
                "epoch_j2000_s": engine_f64(*epoch_j2000_s),
                "reason": reason,
            }),
        ),
        E::NoSignalPairs => rtk_node("no_signal_pairs", json!({})),
        E::NoUsableEpochs => rtk_node("no_usable_epochs", json!({})),
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
    }
}

fn static_reference_mode_name(
    mode: sidereon_core::positioning::StaticReferenceStationMode,
) -> &'static str {
    use sidereon_core::positioning::StaticReferenceStationMode as M;
    match mode {
        M::CodeDgnss => "code_dgnss",
        M::CarrierFloat => "carrier_float",
        M::CarrierFixed => "carrier_fixed",
    }
}

fn static_reference_mode_status_name(
    status: sidereon_core::positioning::StaticReferenceModeStatus,
) -> &'static str {
    use sidereon_core::positioning::StaticReferenceModeStatus as S;
    match status {
        S::Solved => "solved",
        S::Failed => "failed",
    }
}

fn static_reference_mode_error_value(
    error: &sidereon_core::positioning::StaticReferenceModeError,
) -> Value {
    use sidereon_core::positioning::StaticReferenceModeError as E;
    match error {
        E::RinexAssembly { side, reason } => {
            rtk_node("rinex_assembly", json!({"side": side, "reason": reason}))
        }
        E::NoMatchedCodeEpochs => rtk_node("no_matched_code_epochs", json!({})),
        E::CodeDgnss { reason } => rtk_node("code_dgnss", json!({"reason": reason})),
        E::StaticSolve { reason } => rtk_node("static_solve", json!({"reason": reason})),
        E::CarrierArc { reason } => rtk_node("carrier_arc", json!({"reason": reason})),
        E::CarrierSolve { reason } => rtk_node("carrier_solve", json!({"reason": reason})),
        E::Frame { field, reason } => rtk_node("frame", json!({"field": field, "reason": reason})),
        E::CorrectedObservation { reason } => {
            rtk_node("corrected_observation", json!({"reason": reason}))
        }
        E::InvalidCorrectedSatelliteId { satellite_id } => rtk_node(
            "invalid_corrected_satellite_id",
            json!({"satellite_id": satellite_id}),
        ),
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
    }
}

fn unresolved_carrier_value(c: &sidereon_core::rtk_filter::RtkRinexUnresolvedCarrier) -> Value {
    json!({
        "receiver": match c.receiver {
            sidereon_core::rtk_filter::RtkRinexReceiver::Base => "base",
            sidereon_core::rtk_filter::RtkRinexReceiver::Rover => "rover",
        },
        "epoch_index": c.epoch_index,
        "satellite_id": c.satellite_id,
        "observable_code": c.observable_code,
    })
}

fn static_reference_mode_report_value(
    report: &sidereon_core::positioning::StaticReferenceModeReport,
) -> Value {
    json!({
        "mode": static_reference_mode_name(report.mode),
        "status": static_reference_mode_status_name(report.status),
        "used_epochs": report.used_epochs,
        "skipped_epochs": report.skipped_epochs,
        "used_measurements": report.used_measurements,
        "error": report.error.as_ref().map(static_reference_mode_error_value),
        "unresolved_carriers": report
            .unresolved_carriers
            .iter()
            .map(unresolved_carrier_value)
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn static_reference_station_error_value(
    error: &sidereon_core::positioning::StaticReferenceStationError,
) -> Value {
    use sidereon_core::positioning::StaticReferenceStationError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::NoEnabledModes => rtk_node("no_enabled_modes", json!({})),
        E::AllModesFailed { mode_reports } => rtk_node(
            "all_modes_failed",
            json!({
                "mode_reports": mode_reports
                    .iter()
                    .map(static_reference_mode_report_value)
                    .collect::<Vec<_>>(),
            }),
        ),
    }
}

fn tle_fit_integer_value<T: Into<Value>>(value: T) -> Value {
    value.into()
}

fn tle_fit_optional_f64(value: Option<f64>) -> Value {
    value.map(engine_f64).unwrap_or(Value::Null)
}

fn tle_fit_optional_integer<T: Into<Value>>(value: Option<T>) -> Value {
    value.map_or(Value::Null, tle_fit_integer_value)
}

fn tle_fit_exact_julian_date(value: sidereon_core::astro::sgp4::JulianDate) -> Value {
    json!([engine_f64(value.0), engine_f64(value.1)])
}

fn tle_fit_elements_value(value: &sidereon_core::astro::sgp4::ElementSet) -> Value {
    json!({
        "epoch": tle_fit_exact_julian_date(value.epoch),
        "bstar": engine_f64(value.bstar),
        "mean_motion_dot": tle_fit_optional_f64(value.mean_motion_dot),
        "mean_motion_double_dot": tle_fit_optional_f64(value.mean_motion_double_dot),
        "eccentricity": engine_f64(value.eccentricity),
        "argument_of_perigee_deg": engine_f64(value.argument_of_perigee_deg),
        "inclination_deg": engine_f64(value.inclination_deg),
        "mean_anomaly_deg": engine_f64(value.mean_anomaly_deg),
        "mean_motion_rev_per_day": engine_f64(value.mean_motion_rev_per_day),
        "right_ascension_deg": engine_f64(value.right_ascension_deg),
        "catalog_number": tle_fit_optional_integer(value.catalog_number),
        "omm_epoch_days": tle_fit_optional_f64(value.omm_epoch_days),
    })
}

fn tle_fit_omm_epoch_value(value: &sidereon_core::astro::omm::OmmEpoch) -> Value {
    json!({
        "year": tle_fit_integer_value(value.year),
        "month": tle_fit_integer_value(value.month),
        "day": tle_fit_integer_value(value.day),
        "hour": tle_fit_integer_value(value.hour),
        "minute": tle_fit_integer_value(value.minute),
        "second": tle_fit_integer_value(value.second),
        "microsecond": tle_fit_integer_value(value.microsecond),
        "femtosecond": tle_fit_integer_value(value.femtosecond),
    })
}

fn tle_fit_omm_comments_value(value: &sidereon_core::astro::omm::OmmComments) -> Value {
    json!({
        "header": value.header,
        "metadata": value.metadata,
        "mean_elements": value.mean_elements,
        "tle_parameters": value.tle_parameters,
        "user_defined": value.user_defined,
    })
}

fn tle_fit_omm_spacecraft_value(value: &sidereon_core::astro::omm::OmmSpacecraft) -> Value {
    json!({
        "comments": value.comments,
        "mass_kg": tle_fit_optional_f64(value.mass_kg),
        "solar_rad_area_m2": tle_fit_optional_f64(value.solar_rad_area_m2),
        "solar_rad_coeff": tle_fit_optional_f64(value.solar_rad_coeff),
        "drag_area_m2": tle_fit_optional_f64(value.drag_area_m2),
        "drag_coeff": tle_fit_optional_f64(value.drag_coeff),
    })
}

fn tle_fit_omm_covariance_value(value: &sidereon_core::astro::omm::OmmCovariance) -> Value {
    json!({
        "comments": value.comments,
        "cov_ref_frame": value.cov_ref_frame,
        "lower_triangle": value.lower_triangle.map(engine_f64),
    })
}

fn tle_fit_omm_value(value: &sidereon_core::astro::omm::Omm) -> Value {
    json!({
        "ccsds_omm_vers": value.ccsds_omm_vers,
        "classification": value.classification,
        "creation_date": value.creation_date,
        "originator": value.originator,
        "message_id": value.message_id,
        "object_name": value.object_name,
        "object_id": value.object_id,
        "center_name": value.center_name,
        "ref_frame": value.ref_frame,
        "ref_frame_epoch": value.ref_frame_epoch,
        "time_system": value.time_system,
        "mean_element_theory": value.mean_element_theory,
        "epoch": tle_fit_omm_epoch_value(&value.epoch),
        "mean_motion": tle_fit_optional_f64(value.mean_motion),
        "semi_major_axis_km": tle_fit_optional_f64(value.semi_major_axis_km),
        "eccentricity": engine_f64(value.eccentricity),
        "inclination_deg": engine_f64(value.inclination_deg),
        "ra_of_asc_node_deg": engine_f64(value.ra_of_asc_node_deg),
        "arg_of_pericenter_deg": engine_f64(value.arg_of_pericenter_deg),
        "mean_anomaly_deg": engine_f64(value.mean_anomaly_deg),
        "gm_km3_s2": tle_fit_optional_f64(value.gm_km3_s2),
        "spacecraft": value.spacecraft.as_ref().map(tle_fit_omm_spacecraft_value),
        "ephemeris_type": tle_fit_optional_integer(value.ephemeris_type),
        "classification_type": value.classification_type,
        "norad_cat_id": tle_fit_optional_integer(value.norad_cat_id),
        "element_set_no": tle_fit_optional_integer(value.element_set_no),
        "rev_at_epoch": tle_fit_optional_integer(value.rev_at_epoch),
        "bstar": tle_fit_optional_f64(value.bstar),
        "bterm_m2_kg": tle_fit_optional_f64(value.bterm_m2_kg),
        "mean_motion_dot": tle_fit_optional_f64(value.mean_motion_dot),
        "mean_motion_ddot": tle_fit_optional_f64(value.mean_motion_ddot),
        "agom_m2_kg": tle_fit_optional_f64(value.agom_m2_kg),
        "covariance": value.covariance.as_ref().map(tle_fit_omm_covariance_value),
        "user_defined": value.user_defined.iter().map(|item| json!({
            "parameter": item.parameter,
            "value": item.value,
        })).collect::<Vec<_>>(),
        "comments": tle_fit_omm_comments_value(&value.comments),
        "exact_sgp4_epoch": value.exact_sgp4_epoch.map(tle_fit_exact_julian_date),
        "quantize_tle_derived_fields": value.quantize_tle_derived_fields,
    })
}

fn tle_fit_statistics_value(value: &sidereon_core::astro::sgp4::FitStatistics) -> Value {
    json!({
        "rms_position_km": engine_f64(value.rms_position_km),
        "max_position_km": engine_f64(value.max_position_km),
        "rms_position_axes_km": value.rms_position_axes_km.map(engine_f64),
        "rms_velocity_km_s": tle_fit_optional_f64(value.rms_velocity_km_s),
        "tle_rms_position_km": engine_f64(value.tle_rms_position_km),
        "status": tle_fit_integer_value(value.status),
        "nfev": tle_fit_integer_value(value.nfev),
        "njev": tle_fit_integer_value(value.njev),
        "cost": engine_f64(value.cost),
        "optimality": engine_f64(value.optimality),
        "bstar_observable": value.bstar_observable,
        "seed_refine_passes": value.seed_refine_passes,
    })
}

pub(crate) fn tle_fit_result_value(value: &sidereon_core::astro::sgp4::TleFit) -> Value {
    json!({
        "elements": tle_fit_elements_value(&value.elements),
        "line1": value.line1,
        "line2": value.line2,
        "omm": tle_fit_omm_value(&value.omm),
        "stats": tle_fit_statistics_value(&value.stats),
    })
}

fn tle_error_value(error: &sidereon_core::astro::tle::TleError) -> Value {
    use sidereon_core::astro::tle::TleError as E;
    match error {
        E::NonAscii => rtk_node("non_ascii", json!({})),
        E::Format => rtk_node("format", json!({})),
        E::SatelliteMismatch => rtk_node("satellite_mismatch", json!({})),
        E::InvalidCatalogNumber { value, reason } => rtk_node(
            "invalid_catalog_number",
            json!({"value": value, "reason": reason}),
        ),
        E::CatalogNumberOutOfRange { catalog_number } => rtk_node(
            "catalog_number_out_of_range",
            json!({"catalog_number": tle_fit_integer_value(*catalog_number)}),
        ),
        E::InvalidField { field, reason } => {
            rtk_node("invalid_field", json!({"field": field, "reason": reason}))
        }
        E::Field(value) => rtk_node("field", json!({"value": value})),
        E::ChecksumMismatch {
            line_label,
            expected,
            computed,
        } => rtk_node(
            "checksum_mismatch",
            json!({"line_label": line_label, "expected": tle_fit_integer_value(*expected), "computed": tle_fit_integer_value(*computed)}),
        ),
        E::ChecksumNotDigit {
            line_label,
            found,
            computed,
        } => rtk_node(
            "checksum_not_digit",
            json!({"line_label": line_label, "found": found.to_string(), "computed": tle_fit_integer_value(*computed)}),
        ),
    }
}

pub(crate) fn tle_fit_error_value(error: &sidereon_core::astro::sgp4::TleFitError) -> Value {
    use sidereon_core::astro::sgp4::TleFitError as E;
    match error {
        E::ArcTooShort { samples, needed } => rtk_node(
            "arc_too_short",
            json!({"samples": tle_fit_integer_value(*samples), "needed": tle_fit_integer_value(*needed)}),
        ),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::EpochsNotIncreasing { index } => rtk_node(
            "epochs_not_increasing",
            json!({"index": tle_fit_integer_value(*index)}),
        ),
        E::EpochOutsideArc => rtk_node("epoch_outside_arc", json!({})),
        E::MixedVelocityPresence => rtk_node("mixed_velocity_presence", json!({})),
        E::NotElliptical => rtk_node("not_elliptical", json!({})),
        E::InclinationNearRetrograde { inclination_deg } => rtk_node(
            "inclination_near_retrograde",
            json!({"inclination_deg": engine_f64(*inclination_deg)}),
        ),
        E::SeedPropagation {
            epoch_index,
            source,
        } => rtk_node(
            "seed_propagation",
            json!({"epoch_index": tle_fit_integer_value(*epoch_index), "message": source.to_string(), "cause": crate::tle::sgp4_error_value(source)}),
        ),
        E::Solver(source) => rtk_node(
            "solver",
            json!({"message": source.to_string(), "cause": trls_error_value(source)}),
        ),
        E::SolutionInfeasible => rtk_node("solution_infeasible", json!({})),
        E::DidNotConverge { result } => rtk_node(
            "did_not_converge",
            json!({"best_effort_fit": tle_fit_result_value(result)}),
        ),
        E::FinalElements(source) => rtk_node(
            "final_elements",
            json!({"message": source.to_string(), "cause": crate::tle::sgp4_error_value(source)}),
        ),
        E::TleEncode(source) => rtk_node(
            "tle_encode",
            json!({"message": source.to_string(), "cause": tle_error_value(source)}),
        ),
    }
}

pub(crate) fn iod_error_value(error: &sidereon_core::astro::iod::IodError) -> Value {
    use sidereon_core::astro::iod::IodError as E;
    let kind = match error {
        E::DeterminantTooSmall => "determinant_too_small",
        E::OrbitNotPossible => "orbit_not_possible",
        E::ZeroVector => "zero_vector",
        E::CollinearVectors => "collinear_vectors",
        E::NotCoplanar => "not_coplanar",
        E::InvalidTimeGeometry => "invalid_time_geometry",
        E::NoPositiveRoot => "no_positive_root",
        E::RootSolveFailed => "root_solve_failed",
        E::NonFiniteValue => "non_finite_value",
    };
    rtk_node(kind, json!({}))
}

fn backend_error_value(error: &trust_region_least_squares::trf::BackendError) -> Value {
    use trust_region_least_squares::trf::BackendError as E;
    match error {
        E::Failed(reason) => rtk_node("failed", json!({"reason": reason})),
        E::BadDimensions {
            expected_m,
            expected_n,
            got,
        } => rtk_node(
            "bad_dimensions",
            json!({
                "expected_m": expected_m,
                "expected_n": expected_n,
                "got": got,
            }),
        ),
    }
}

pub(crate) fn trls_error_value(error: &trust_region_least_squares::trf::TrfError) -> Value {
    use trust_region_least_squares::trf::TrfError as E;
    match error {
        E::EmptyResidual => rtk_node("empty_residual", json!({})),
        E::EmptyParameters => rtk_node("empty_parameters", json!({})),
        E::NonFiniteParameters => rtk_node("non_finite_parameters", json!({})),
        E::NonFiniteInitialResidual => rtk_node("non_finite_initial_residual", json!({})),
        E::InsufficientRows { m, n } => rtk_node("insufficient_rows", json!({"m": m, "n": n})),
        E::SizeOverflow { m, n } => rtk_node("size_overflow", json!({"m": m, "n": n})),
        E::DegreeOverflow { degree } => rtk_node("degree_overflow", json!({"degree": degree})),
        E::InvalidMaxNfev => rtk_node("invalid_max_nfev", json!({})),
        E::InvalidFScale { f_scale } => {
            rtk_node("invalid_f_scale", json!({"f_scale": engine_f64(*f_scale)}))
        }
        E::InvalidXScaleLength { expected, got } => rtk_node(
            "invalid_x_scale_length",
            json!({"expected": expected, "got": got}),
        ),
        E::InvalidXScaleValue { index, value } => rtk_node(
            "invalid_x_scale_value",
            json!({"index": index, "value": engine_f64(*value)}),
        ),
        E::InvalidJacobianLength { expected, got } => rtk_node(
            "invalid_jacobian_length",
            json!({"expected": expected, "got": got}),
        ),
        E::InvalidResidualLength { expected, got } => rtk_node(
            "invalid_residual_length",
            json!({"expected": expected, "got": got}),
        ),
        E::InvalidSliceLength {
            what,
            expected,
            got,
        } => rtk_node(
            "invalid_slice_length",
            json!({"what": what, "expected": expected, "got": got}),
        ),
        E::InvalidSvdOutput(reason) => rtk_node("invalid_svd_output", json!({"reason": reason})),
        E::Backend(source) => rtk_node("backend", json!({"cause": backend_error_value(source)})),
    }
}

pub(crate) fn dop_error_value(error: &sidereon_core::dop::DopError) -> Value {
    use sidereon_core::dop::DopError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::TooFewSatellites => rtk_node("too_few_satellites", json!({})),
        E::Singular => rtk_node("singular", json!({})),
    }
}

pub(crate) fn geofence_error_value(error: &sidereon_core::geofence::GeofenceError) -> Value {
    use sidereon_core::geofence::GeofenceError as E;
    match error {
        E::TooFewVertices => rtk_node("too_few_vertices", json!({})),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::Geodesic(cause) => rtk_node("geodesic", json!({"cause": geodesic_error_value(cause)})),
        E::Dop(cause) => rtk_node("dop", json!({"cause": dop_error_value(cause)})),
        E::ErrorMetrics(cause) => rtk_node(
            "error_metrics",
            json!({"cause": error_metrics_error_value(cause)}),
        ),
    }
}

fn geodesic_error_value(error: &sidereon_core::geodesic::GeodesicError) -> Value {
    use sidereon_core::geodesic::GeodesicError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
    }
}

pub(crate) fn error_metrics_error_value(
    error: &sidereon_core::error_metrics::ErrorMetricsError,
) -> Value {
    use sidereon_core::error_metrics::ErrorMetricsError as E;
    match error {
        E::NonFinite => rtk_node("non_finite", json!({})),
        E::NotPositiveSemidefinite => rtk_node("not_positive_semidefinite", json!({})),
        E::InvalidProbability => rtk_node("invalid_probability", json!({})),
        E::Rotation(cause) => rtk_node("rotation", json!({"cause": dop_error_value(cause)})),
    }
}

pub(crate) fn antex_error_value(error: &sidereon_core::antex::AntexError) -> Value {
    use sidereon_core::antex::AntexError as E;
    match error {
        E::InvalidDateTime => rtk_node("invalid_date_time", json!({})),
        E::InvalidField {
            antenna_id,
            record,
            field,
            value,
        } => rtk_node(
            "invalid_field",
            json!({
                "antenna_id": antenna_id,
                "record": record,
                "field": field,
                "value": value,
            }),
        ),
        E::RepeatedRecord { antenna_id, record } => rtk_node(
            "repeated_record",
            json!({
                "antenna_id": antenna_id,
                "record": record,
            }),
        ),
        E::DegenerateGrid {
            antenna_id,
            frequency,
            reason,
        } => rtk_node(
            "degenerate_grid",
            json!({
                "antenna_id": antenna_id,
                "frequency": frequency,
                "reason": reason,
            }),
        ),
        E::InvalidInput { field, reason } => rtk_node(
            "invalid_input",
            json!({
                "field": field,
                "reason": reason,
            }),
        ),
        E::UnknownFrequency {
            antenna_id,
            frequency,
        } => rtk_node(
            "unknown_frequency",
            json!({
                "antenna_id": antenna_id,
                "frequency": frequency,
            }),
        ),
        E::AmbiguousFrequency {
            antenna_id,
            frequency,
            sections,
        } => rtk_node(
            "ambiguous_frequency",
            json!({
                "antenna_id": antenna_id,
                "frequency": frequency,
                "sections": sections,
            }),
        ),
        E::MissingPco {
            antenna_id,
            frequency,
        } => rtk_node(
            "missing_pco",
            json!({
                "antenna_id": antenna_id,
                "frequency": frequency,
            }),
        ),
        E::EmptyPcvGrid {
            antenna_id,
            frequency,
        } => rtk_node(
            "empty_pcv_grid",
            json!({
                "antenna_id": antenna_id,
                "frequency": frequency,
            }),
        ),
        E::Unwritable { field, reason } => rtk_node(
            "unwritable",
            json!({
                "field": field,
                "reason": reason,
            }),
        ),
    }
}

pub(crate) fn nav_parse_error_value(error: &sidereon_core::rinex::nav::NavParseError) -> Value {
    use sidereon_core::rinex::nav::NavParseError as E;
    match error {
        E::UnsupportedHeader(message) => {
            rtk_node("unsupported_header", json!({"message": message}))
        }
        E::MissingHeaderEnd => rtk_node("missing_header_end", json!({})),
        E::TruncatedRecord(satellite) => {
            rtk_node("truncated_record", json!({"satellite": satellite}))
        }
        E::BadField { satellite, field } => {
            rtk_node("bad_field", json!({"satellite": satellite, "field": field}))
        }
        E::BadHeaderField { field } => rtk_node("bad_header_field", json!({"field": field})),
        E::UnexpectedLine { line } => rtk_node("unexpected_line", json!({"line": line})),
        E::ExtraRecordLines { satellite } => {
            rtk_node("extra_record_lines", json!({"satellite": satellite}))
        }
    }
}

pub(crate) fn rinex_clock_error_value(
    error: &sidereon_core::rinex::clock::RinexClockError,
) -> Value {
    use sidereon_core::rinex::clock::RinexClockError as E;
    match error {
        E::MalformedAsRecord {
            line,
            reason,
            record,
        } => rtk_node(
            "malformed_as_record",
            json!({"line": line, "reason": reason, "record": record}),
        ),
        E::MissingContinuation { line, record_type } => rtk_node(
            "missing_continuation",
            json!({"line": line, "record_type": record_type}),
        ),
        E::MalformedContinuation {
            line,
            reason,
            record,
        } => rtk_node(
            "malformed_continuation",
            json!({"line": line, "reason": reason, "record": record}),
        ),
        E::BadField { line, field, value } => rtk_node(
            "bad_field",
            json!({"line": line, "field": field, "value": value}),
        ),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::UnsupportedTimeScale { scale } => {
            rtk_node("unsupported_time_scale", json!({"scale": scale.abbrev()}))
        }
    }
}

pub(crate) fn bias_departure_value(departure: &sidereon_core::bias::BiasDeparture) -> Value {
    use sidereon_core::bias::BiasDeparture as D;
    match departure {
        D::HeaderLayout { reason } => rtk_node("header_layout", json!({"reason": reason})),
        D::OtherVersion { version } => rtk_node("other_version", json!({"version": version})),
        D::MissingFooter => rtk_node("missing_footer", json!({})),
        D::ContentAfterFooter { line } => rtk_node("content_after_footer", json!({"line": line})),
        D::UnexpectedControlLine { line } => {
            rtk_node("unexpected_control_line", json!({"line": line}))
        }
        D::UnclosedBlock { name, line } => {
            rtk_node("unclosed_block", json!({"name": name, "line": line}))
        }
        D::UnopenedBlockEnd { name, line } => {
            rtk_node("unopened_block_end", json!({"name": name, "line": line}))
        }
        D::MismatchedBlockEnd { open, close, line } => rtk_node(
            "mismatched_block_end",
            json!({"open": open, "close": close, "line": line}),
        ),
        D::NestedBlock { open, inner, line } => rtk_node(
            "nested_block",
            json!({"open": open, "inner": inner, "line": line}),
        ),
        D::MissingBlock { name } => rtk_node("missing_block", json!({"name": name})),
        D::UnknownBlock { name, line } => {
            rtk_node("unknown_block", json!({"name": name, "line": line}))
        }
        D::BlockStartSuffix { line } => rtk_node("block_start_suffix", json!({"line": line})),
        D::DataOutsideBlock { line } => rtk_node("data_outside_block", json!({"line": line})),
        D::MissingDeclaration { keyword } => {
            rtk_node("missing_declaration", json!({"keyword": keyword}))
        }
        D::UnsupportedBiasMode { line, label } => rtk_node(
            "unsupported_bias_mode",
            json!({"line": line, "label": label}),
        ),
        D::NonStandardTimeSystem { line, label } => rtk_node(
            "non_standard_time_system",
            json!({"line": line, "label": label}),
        ),
        D::HeaderModeMismatch {
            header,
            description,
        } => {
            let desc = match description {
                sidereon_core::bias::BiasMode::Absolute => "Absolute",
                sidereon_core::bias::BiasMode::Relative => "Relative",
                sidereon_core::bias::BiasMode::Unspecified => "Unspecified",
            };
            rtk_node(
                "header_mode_mismatch",
                json!({"header": header, "description": desc}),
            )
        }
        D::UnknownDcbTimeSystem { line, label } => rtk_node(
            "unknown_dcb_time_system",
            json!({"line": line, "label": label}),
        ),
        D::EstimateCountMismatch {
            declared,
            solution_rows,
        } => rtk_node(
            "estimate_count_mismatch",
            json!({"declared": declared, "solution_rows": solution_rows}),
        ),
        other => rtk_node("unknown", json!({"debug": format!("{other:?}")})),
    }
}

pub(crate) fn bias_error_value(error: &sidereon_core::bias::BiasError) -> Value {
    use sidereon_core::bias::BiasError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::InvalidEpoch => rtk_node("invalid_epoch", json!({})),
        E::UnknownObservable { code } => rtk_node("unknown_observable", json!({"code": code})),
        E::UnsupportedVersion { version } => {
            rtk_node("unsupported_version", json!({"version": version}))
        }
        E::MissingDcbMetadata => rtk_node("missing_dcb_metadata", json!({})),
        E::MissingClockReference => rtk_node("missing_clock_reference", json!({})),
        E::MissingWriterMetadata { field } => {
            rtk_node("missing_writer_metadata", json!({"field": field}))
        }
        E::Utf8 => rtk_node("utf8", json!({})),
        E::Departure { departure } => rtk_node(
            "departure",
            json!({"departure": bias_departure_value(departure)}),
        ),
        E::InvalidUtf8Line { line } => rtk_node("invalid_utf8_line", json!({"line": line})),
        E::UnsupportedTimeSystem { scale } => rtk_node(
            "unsupported_time_system",
            json!({"scale": scale.map(|s| s.abbrev())}),
        ),
        E::DcbRecordMismatch { record, field } => rtk_node(
            "dcb_record_mismatch",
            json!({"record": record, "field": field}),
        ),
    }
}

pub(crate) fn solution_validation_error_value(
    error: &sidereon_core::quality::SolutionValidationError,
) -> Value {
    use sidereon_core::quality::SolutionValidationError as E;
    match error {
        E::InvalidOptions { field, reason } => {
            rtk_node("invalid_options", json!({"field": field, "reason": reason}))
        }
        E::DegenerateGeometryRankDeficient => {
            rtk_node("degenerate_geometry_rank_deficient", json!({}))
        }
        E::DegenerateGeometryPdop(pdop) => rtk_node(
            "degenerate_geometry_pdop",
            json!({"pdop": engine_f64(*pdop)}),
        ),
        E::ImplausiblePosition(radius_m) => rtk_node(
            "implausible_position",
            json!({"radius_m": engine_f64(*radius_m)}),
        ),
        E::InvalidResiduals => rtk_node("invalid_residuals", json!({})),
        E::NoConvergence(rms_m) => rtk_node("no_convergence", json!({"rms_m": engine_f64(*rms_m)})),
    }
}

pub(crate) fn rf_error_value(error: &sidereon_core::astro::rf::RfError) -> Value {
    use sidereon_core::astro::rf::RfError as E;
    match error {
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
    }
}

pub(crate) fn rinex_spp_error_value(error: &sidereon_core::positioning::RinexSppError) -> Value {
    use sidereon_core::positioning::RinexSppError as E;
    match error {
        E::Observation(cause) => rtk_node("observation", json!({"cause": core_error_value(cause)})),
        E::MissingApproxPosition => rtk_node("missing_approx_position", json!({})),
        _ => rtk_node("unknown", json!({"message": error.to_string()})),
    }
}

pub(crate) fn solve_policy_error_value(
    error: &sidereon_core::positioning::SolvePolicyError,
) -> Value {
    use sidereon_core::positioning::SolvePolicyError as E;
    match error {
        E::Solve(spp_err) => rtk_node("solve", json!({"cause": spp_error_value(spp_err)})),
        E::Validation(val_err) => rtk_node(
            "validation",
            json!({"cause": solution_validation_error_value(val_err)}),
        ),
        E::NoCoarseSolution => rtk_node("no_coarse_solution", json!({})),
    }
}

pub(crate) fn velocity_error_value(error: &sidereon_core::velocity::VelocityError) -> Value {
    use sidereon_core::velocity::VelocityError as E;
    match error {
        E::NoObservations => rtk_node("no_observations", json!({})),
        E::TooFewSatellites { used, required } => rtk_node(
            "too_few_satellites",
            json!({"used": used, "required": required}),
        ),
        E::SingularGeometry => rtk_node("singular_geometry", json!({})),
        E::DuplicateObservation { satellite_id } => rtk_node(
            "duplicate_observation",
            json!({"satellite_id": satellite_id.to_string()}),
        ),
        E::InvalidCarrier { satellite_id } => rtk_node(
            "invalid_carrier",
            json!({"satellite_id": satellite_id.to_string()}),
        ),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::InvalidObservation { satellite_id } => rtk_node(
            "invalid_observation",
            json!({"satellite_id": satellite_id.to_string()}),
        ),
        E::InvalidReceiverState => rtk_node("invalid_receiver_state", json!({})),
    }
}

pub(crate) fn ppp_no_ephemeris_reason_value(
    reason: &sidereon_core::precise_positioning::NoEphemerisReason,
) -> Value {
    use sidereon_core::precise_positioning::NoEphemerisReason as R;
    match reason {
        R::NoEphemeris => rtk_node("no_ephemeris", json!({})),
        R::MissingSatelliteClock => rtk_node("missing_satellite_clock", json!({})),
        R::Reason(msg) => rtk_node("reason", json!({"message": msg})),
    }
}

pub(crate) fn ppp_missing_correction_value(
    correction: &sidereon_core::precise_positioning::MissingCorrection,
) -> Value {
    use sidereon_core::precise_positioning::MissingCorrection as C;
    match correction {
        C::SolidEarthTide => rtk_node("solid_earth_tide", json!({})),
        C::PoleTide => rtk_node("pole_tide", json!({})),
        C::OceanLoading => rtk_node("ocean_loading", json!({})),
        C::PhaseWindup => rtk_node("phase_windup", json!({})),
        C::SatelliteAntennaPco => rtk_node("satellite_antenna_pco", json!({})),
        C::SatelliteAntennaPcv => rtk_node("satellite_antenna_pcv", json!({})),
        C::CodeBias => rtk_node("code_bias", json!({})),
        C::SsrCodeBias => rtk_node("ssr_code_bias", json!({})),
        C::PhaseBias => rtk_node("phase_bias", json!({})),
        C::ReceiverAntennaFrequency(label) => {
            rtk_node("receiver_antenna_frequency", json!({"label": label}))
        }
        C::ReceiverAntennaPcv(label) => rtk_node("receiver_antenna_pcv", json!({"label": label})),
        C::ReceiverAntennaGeometry => rtk_node("receiver_antenna_geometry", json!({})),
    }
}

pub(crate) fn ppp_float_solve_error_value(
    error: &sidereon_core::precise_positioning::FloatSolveError,
) -> Value {
    use sidereon_core::precise_positioning::FloatSolveError as E;
    match error {
        E::NoEphemeris {
            satellite_id,
            reason,
        } => rtk_node(
            "no_ephemeris",
            json!({
                "satellite_id": satellite_id,
                "reason": ppp_no_ephemeris_reason_value(reason),
            }),
        ),
        E::SingularGeometry => rtk_node("singular_geometry", json!({})),
        E::InvalidClockCount { expected, actual } => rtk_node(
            "invalid_clock_count",
            json!({"expected": expected, "actual": actual}),
        ),
        E::InvalidSolveOption { field, reason } => rtk_node(
            "invalid_solve_option",
            json!({"field": field, "reason": reason}),
        ),
        E::InvalidInput { field, reason } => {
            rtk_node("invalid_input", json!({"field": field, "reason": reason}))
        }
        E::InsufficientObservationsAfterElevationCutoff {
            cutoff_deg,
            retained_observations,
            required_observations,
        } => rtk_node(
            "insufficient_observations_after_elevation_cutoff",
            json!({
                "cutoff_deg": engine_f64(*cutoff_deg),
                "retained_observations": retained_observations,
                "required_observations": required_observations,
            }),
        ),
        E::InsufficientObservationsAfterSsrBiasExclusion {
            excluded_observations,
            retained_observations,
            required_observations,
        } => rtk_node(
            "insufficient_observations_after_ssr_bias_exclusion",
            json!({
                "excluded_observations": excluded_observations,
                "retained_observations": retained_observations,
                "required_observations": required_observations,
            }),
        ),
        E::MissingAmbiguity(ambiguity_id) => {
            rtk_node("missing_ambiguity", json!({"ambiguity_id": ambiguity_id}))
        }
        E::MissingCorrection {
            satellite_id,
            correction,
        } => rtk_node(
            "missing_correction",
            json!({
                "satellite_id": satellite_id,
                "correction": ppp_missing_correction_value(correction),
            }),
        ),
        E::Ut1OutsideCoverage(reason) => rtk_node(
            "ut1_outside_coverage",
            json!({"reason": degrade_reason_name(*reason)}),
        ),
    }
}

pub(crate) fn ppp_fixed_solve_error_value(
    error: &sidereon_core::precise_positioning::FixedSolveError,
) -> Value {
    use sidereon_core::precise_positioning::FixedSolveError as E;
    match error {
        E::Float(float_err) => rtk_node(
            "float",
            json!({"cause": ppp_float_solve_error_value(float_err)}),
        ),
        E::Integer(ils_err) => rtk_node("integer", json!({"cause": ils_error_value(ils_err)})),
        E::MissingWavelength(ambiguity_id) => {
            rtk_node("missing_wavelength", json!({"ambiguity_id": ambiguity_id}))
        }
        E::MissingOffset(ambiguity_id) => {
            rtk_node("missing_offset", json!({"ambiguity_id": ambiguity_id}))
        }
        E::MissingFixedAmbiguity(ambiguity_id) => rtk_node(
            "missing_fixed_ambiguity",
            json!({"ambiguity_id": ambiguity_id}),
        ),
    }
}

pub(crate) fn facade_error_value(error: &sidereon::Error) -> Value {
    use sidereon::Error as E;
    match error {
        E::Sp3(e) => rtk_node("sp3", json!({"cause": core_error_value(e)})),
        E::Antex(e) => rtk_node("antex", json!({"cause": antex_error_value(e)})),
        E::RinexNav(e) => rtk_node("rinex_nav", json!({"cause": nav_parse_error_value(e)})),
        E::RinexObs(e) => rtk_node("rinex_obs", json!({"cause": core_error_value(e)})),
        E::RinexClock(e) => rtk_node("rinex_clock", json!({"cause": rinex_clock_error_value(e)})),
        E::Bias(e) => rtk_node("bias", json!({"cause": bias_error_value(e)})),
        E::Ssr(e) => rtk_node("ssr", json!({"cause": core_error_value(e)})),
        E::Crinex(e) => rtk_node("crinex", json!({"cause": core_error_value(e)})),
        E::Io(e) => rtk_node(
            "io",
            json!({
                "kind": format!("{:?}", e.kind()),
                "raw_os_error": e.raw_os_error(),
                "message": e.to_string(),
            }),
        ),
        E::Spp(e) => rtk_node("spp", json!({"cause": solve_policy_error_value(e)})),
        E::Velocity(e) => rtk_node("velocity", json!({"cause": velocity_error_value(e)})),
        E::RtkFloat(e) => rtk_node("rtk_float", json!({"cause": float_solve_error_value(e)})),
        E::RtkFixed(e) => rtk_node(
            "rtk_fixed",
            json!({"cause": validated_fixed_solve_error_value(e)}),
        ),
        E::PppFloat(e) => rtk_node(
            "ppp_float",
            json!({"cause": ppp_float_solve_error_value(e)}),
        ),
        E::PppFixed(e) => rtk_node(
            "ppp_fixed",
            json!({"cause": ppp_fixed_solve_error_value(e)}),
        ),
    }
}

pub(crate) trait RecordEngineError {
    fn record_engine_error(fn_name: &str, err: &Self);
}

impl RecordEngineError for sidereon::Error {
    fn record_engine_error(fn_name: &str, err: &Self) {
        record_engine_error(
            SidereonEngineErrorFamily::Facade,
            fn_name,
            facade_error_value(err),
        );
    }
}

impl RecordEngineError for sidereon_core::positioning::SppError {
    fn record_engine_error(fn_name: &str, err: &Self) {
        record_engine_error(
            SidereonEngineErrorFamily::Spp,
            fn_name,
            spp_error_value(err),
        );
    }
}

impl RecordEngineError for sidereon_core::positioning::SolvePolicyError {
    fn record_engine_error(fn_name: &str, err: &Self) {
        record_engine_error(
            SidereonEngineErrorFamily::SppPolicy,
            fn_name,
            solve_policy_error_value(err),
        );
    }
}

/// Read the typed error summary from the most recent matching producer on this
/// OS thread. Reading does not clear the record.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_engine_error_info(
    out: *mut SidereonEngineErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_engine_error_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out, FN_NAME, "out") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = LAST_ENGINE_ERROR.with(|slot| {
            slot.borrow().as_ref().map_or(
                SidereonEngineErrorInfo {
                    family: SidereonEngineErrorFamily::None,
                    payload_len: 0,
                },
                |(info, _)| *info,
            )
        });
        SidereonStatus::Ok
    })
}

/// Copy the retained versioned engine-error JSON. Query the required size with
/// a null output and zero length; reads and short-buffer retries retain it.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_engine_error_payload(
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_engine_error_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let payload = LAST_ENGINE_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(String::new, |(_, payload)| payload.clone())
        });
        if let Err(status) = copy_prefix_to_c(
            FN_NAME,
            "out",
            payload.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ) {
            return status;
        }
        SidereonStatus::Ok
    })
}

/// Snapshot the most recent observable batch's thread-local row errors into an
/// independently owned handle. The thread-local source is replaced at the next
/// observable producer, but this snapshot survives later operations and source
/// handle frees. Release it with `sidereon_observable_row_errors_free`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_row_errors_snapshot(
    out: *mut *mut SidereonObservableRowErrors,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_observable_row_errors_snapshot";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = match require_out(out, FN_NAME, "out") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out = ptr::null_mut();
        let rows = LAST_OBSERVABLE_ROW_ERRORS.with(|rows| rows.borrow().clone());
        *out = Box::into_raw(Box::new(SidereonObservableRowErrors { rows }));
        SidereonStatus::Ok
    })
}

/// Read the number of typed row errors in an owned observable-error snapshot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_row_errors_count(
    errors: *const SidereonObservableRowErrors,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_observable_row_errors_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let errors = match crate::require_ref(errors, FN_NAME, "errors") {
            Ok(errors) => errors,
            Err(status) => return status,
        };
        let out_count = match require_out(out_count, FN_NAME, "out_count") {
            Ok(out) => out,
            Err(status) => return status,
        };
        *out_count = errors.rows.len();
        SidereonStatus::Ok
    })
}

/// Read row index, legacy status, and payload length from an owned snapshot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_row_errors_info(
    errors: *const SidereonObservableRowErrors,
    index: usize,
    out: *mut SidereonObservableRowErrorInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_observable_row_errors_info";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let errors = match crate::require_ref(errors, FN_NAME, "errors") {
            Ok(errors) => errors,
            Err(status) => return status,
        };
        let out = match require_out(out, FN_NAME, "out") {
            Ok(out) => out,
            Err(status) => return status,
        };
        let Some((row_index, status, payload)) = errors.rows.get(index) else {
            crate::set_last_error(format!("{FN_NAME}: index {index} is out of range"));
            return SidereonStatus::InvalidArgument;
        };
        *out = SidereonObservableRowErrorInfo {
            row_index: *row_index,
            status: *status,
            payload_len: payload.len(),
        };
        SidereonStatus::Ok
    })
}

/// Copy one owned row-error JSON payload. Short-buffer retries retain the
/// snapshot; caller storage is owned by the caller and requires no library free.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_row_errors_payload(
    errors: *const SidereonObservableRowErrors,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_observable_row_errors_payload";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if let Err(status) = init_copy_counts(FN_NAME, out_written, out_required) {
            return status;
        }
        let errors = match crate::require_ref(errors, FN_NAME, "errors") {
            Ok(errors) => errors,
            Err(status) => return status,
        };
        let Some((_, _, payload)) = errors.rows.get(index) else {
            crate::set_last_error(format!("{FN_NAME}: index {index} is out of range"));
            return SidereonStatus::InvalidArgument;
        };
        if let Err(status) = copy_prefix_to_c(
            FN_NAME,
            "out",
            payload.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ) {
            return status;
        }
        SidereonStatus::Ok
    })
}

/// Free an owned observable-row-error snapshot; NULL is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observable_row_errors_free(
    errors: *mut SidereonObservableRowErrors,
) {
    const FN_NAME: &str = "sidereon_observable_row_errors_free";
    ffi_boundary(FN_NAME, (), || {
        if !errors.is_null() {
            drop(Box::from_raw(errors));
        }
    });
}

/// Clear the versioned engine-error detail retained on the current OS thread.
///
/// Resets only the typed generic engine TLS slot (`LAST_ENGINE_ERROR`). All
/// existing diagnostic text (`sidereon_last_error_message`) and other
/// family-specific slots remain unchanged; ordinary readers and free functions
/// continue retaining their slots until a producer or explicit reset runs.
///
/// This explicit reset is needed for foreign language bindings (such as Go)
/// to prevent stale generic detail on independent operation entry.
#[no_mangle]
pub extern "C" fn sidereon_clear_engine_error() {
    const FN_NAME: &str = "sidereon_clear_engine_error";
    ffi_boundary(FN_NAME, (), clear_engine_error);
}

fn ndm_text_issue_name(issue: sidereon_core::astro::ndm::TextIssue) -> &'static str {
    use sidereon_core::astro::ndm::TextIssue as I;
    match issue {
        I::LineBreak => "line_break",
        I::SurroundingWhitespace => "surrounding_whitespace",
        I::InteriorWhitespace => "interior_whitespace",
        I::KeywordSeparator => "keyword_separator",
        I::XmlIllegalCharacter => "xml_illegal_character",
        I::Empty => "empty",
        I::DetachedComment => "detached_comment",
        I::RepeatedParameter => "repeated_parameter",
        I::CommentNotCarried => "comment_not_carried",
    }
}

fn oem_input_error_kind_name(kind: sidereon_core::astro::oem::OemInputErrorKind) -> &'static str {
    use sidereon_core::astro::oem::OemInputErrorKind as K;
    match kind {
        K::Missing => "missing",
        K::NonFinite => "non_finite",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

fn opm_input_error_kind_name(kind: sidereon_core::astro::opm::OpmInputErrorKind) -> &'static str {
    use sidereon_core::astro::opm::OpmInputErrorKind as K;
    match kind {
        K::Missing => "missing",
        K::NonFinite => "non_finite",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

fn omm_input_error_kind_name(kind: sidereon_core::astro::omm::OmmInputErrorKind) -> &'static str {
    use sidereon_core::astro::omm::OmmInputErrorKind as K;
    match kind {
        K::Missing => "missing",
        K::NonFinite => "non_finite",
        K::FloatParse => "float_parse",
        K::IntParse => "int_parse",
        K::NotPositive => "not_positive",
        K::Negative => "negative",
        K::OutOfRange => "out_of_range",
        K::InvalidCivilDate => "invalid_civil_date",
        K::InvalidCivilTime => "invalid_civil_time",
    }
}

pub(crate) fn oem_state_line_error_value(
    error: &sidereon_core::astro::oem::OemStateLineError,
) -> Value {
    use sidereon_core::astro::oem::OemStateLineError as E;
    match error {
        E::ItemCount(found) => rtk_node("item_count", json!({"found": found})),
        E::InvalidField { field, kind } => rtk_node(
            "invalid_field",
            json!({"field": field, "kind": oem_input_error_kind_name(*kind)}),
        ),
    }
}

pub(crate) fn oem_error_value(error: &sidereon_core::astro::oem::OemError) -> Value {
    use sidereon_core::astro::oem::OemError as E;
    match error {
        E::MissingField(field) => rtk_node("missing_field", json!({"field": field})),
        E::InvalidField { field, kind } => rtk_node(
            "invalid_field",
            json!({"field": field, "kind": oem_input_error_kind_name(*kind)}),
        ),
        E::Field(message) => rtk_node("field", json!({"message": message})),
        E::DuplicateField {
            field,
            first,
            second,
        } => rtk_node(
            "duplicate_field",
            json!({"field": field, "first": first, "second": second}),
        ),
        E::UnitMismatch {
            field,
            unit,
            expected,
        } => rtk_node(
            "unit_mismatch",
            json!({"field": field, "unit": unit, "expected": expected}),
        ),
        E::MultipleMessages { count } => rtk_node("multiple_messages", json!({"count": count})),
        E::UnknownField(field) => rtk_node("unknown_field", json!({"field": field})),
        E::MalformedLine { line, text } => {
            rtk_node("malformed_line", json!({"line": line, "text": text}))
        }
        E::UnwritableText {
            field,
            value,
            issue,
        } => rtk_node(
            "unwritable_text",
            json!({"field": field, "value": value, "issue": ndm_text_issue_name(*issue)}),
        ),
    }
}

pub(crate) fn opm_error_value(error: &sidereon_core::astro::opm::OpmError) -> Value {
    use sidereon_core::astro::opm::OpmError as E;
    match error {
        E::MissingField(field) => rtk_node("missing_field", json!({"field": field})),
        E::InvalidField { field, kind } => rtk_node(
            "invalid_field",
            json!({"field": field, "kind": opm_input_error_kind_name(*kind)}),
        ),
        E::Field(message) => rtk_node("field", json!({"message": message})),
        E::DuplicateField {
            field,
            first,
            second,
        } => rtk_node(
            "duplicate_field",
            json!({"field": field, "first": first, "second": second}),
        ),
        E::UnitMismatch {
            field,
            unit,
            expected,
        } => rtk_node(
            "unit_mismatch",
            json!({"field": field, "unit": unit, "expected": expected}),
        ),
        E::MultipleMessages { count } => rtk_node("multiple_messages", json!({"count": count})),
        E::UnknownField(field) => rtk_node("unknown_field", json!({"field": field})),
        E::MalformedLine { line, text } => {
            rtk_node("malformed_line", json!({"line": line, "text": text}))
        }
        E::UnwritableText {
            field,
            value,
            issue,
        } => rtk_node(
            "unwritable_text",
            json!({"field": field, "value": value, "issue": ndm_text_issue_name(*issue)}),
        ),
    }
}

pub(crate) fn omm_error_value(error: &sidereon_core::astro::omm::OmmError) -> Value {
    use sidereon_core::astro::omm::OmmError as E;
    match error {
        E::MissingField(field) => rtk_node("missing_field", json!({"field": field})),
        E::InvalidField { field, kind } => rtk_node(
            "invalid_field",
            json!({"field": field, "kind": omm_input_error_kind_name(*kind)}),
        ),
        E::Field(message) => rtk_node("field", json!({"message": message})),
        E::Epoch(message) => rtk_node("epoch", json!({"message": message})),
        E::DuplicateField {
            field,
            first,
            second,
        } => rtk_node(
            "duplicate_field",
            json!({"field": field, "first": first, "second": second}),
        ),
        E::UnknownField(field) => rtk_node("unknown_field", json!({"field": field})),
        E::CsvColumnCount { found, expected } => rtk_node(
            "csv_column_count",
            json!({"found": found, "expected": expected}),
        ),
        E::CsvEmptyBlock(block) => rtk_node("csv_empty_block", json!({"block": block})),
        E::MalformedLine { line, text } => {
            rtk_node("malformed_line", json!({"line": line, "text": text}))
        }
        E::UnitMismatch {
            field,
            unit,
            expected,
        } => rtk_node(
            "unit_mismatch",
            json!({"field": field, "unit": unit, "expected": expected}),
        ),
        E::MultipleMessages { count } => rtk_node("multiple_messages", json!({"count": count})),
        E::InRecord { index, source } => rtk_node(
            "in_record",
            json!({"index": index, "source": omm_error_value(source)}),
        ),
        E::CsvColumnOrder { first, second } => rtk_node(
            "csv_column_order",
            json!({"first": first, "second": second}),
        ),
        E::IncompatibleMetadata { field, value } => rtk_node(
            "incompatible_metadata",
            json!({"field": field, "value": value}),
        ),
        E::UnwritableText {
            field,
            value,
            issue,
        } => rtk_node(
            "unwritable_text",
            json!({"field": field, "value": value, "issue": ndm_text_issue_name(*issue)}),
        ),
    }
}

#[cfg(test)]
mod ndm_error_mapper_tests {
    use super::*;

    #[test]
    fn ndm_nested_and_writer_errors_keep_complete_payload_data() {
        let issue = sidereon_core::astro::ndm::TextIssue::RepeatedParameter;
        let oem = sidereon_core::astro::oem::OemError::UnwritableText {
            field: "COMMENT".to_string(),
            value: "keep this value".to_string(),
            issue,
        };
        assert_eq!(
            oem_error_value(&oem),
            json!({
                "kind": "unwritable_text",
                "fields": {
                    "field": "COMMENT",
                    "value": "keep this value",
                    "issue": "repeated_parameter"
                }
            })
        );

        let opm = sidereon_core::astro::opm::OpmError::UnitMismatch {
            field: "X".to_string(),
            unit: "m".to_string(),
            expected: Some("km"),
        };
        assert_eq!(
            opm_error_value(&opm),
            json!({
                "kind": "unit_mismatch",
                "fields": {"field": "X", "unit": "m", "expected": "km"}
            })
        );

        let nested = sidereon_core::astro::omm::OmmError::InRecord {
            index: 4,
            source: Box::new(sidereon_core::astro::omm::OmmError::UnwritableText {
                field: "USER_DEFINED_X".to_string(),
                value: "bad\ntext".to_string(),
                issue: sidereon_core::astro::ndm::TextIssue::LineBreak,
            }),
        };
        assert_eq!(
            omm_error_value(&nested),
            json!({
                "kind": "in_record",
                "fields": {
                    "index": 4,
                    "source": {
                        "kind": "unwritable_text",
                        "fields": {
                            "field": "USER_DEFINED_X",
                            "value": "bad\ntext",
                            "issue": "line_break"
                        }
                    }
                }
            })
        );
    }
}

#[cfg(test)]
pub(crate) fn snapshot_engine_error_for_test() -> Option<(SidereonEngineErrorInfo, String)> {
    let mut info = std::mem::MaybeUninit::<SidereonEngineErrorInfo>::uninit();
    let status = unsafe { sidereon_last_engine_error_info(info.as_mut_ptr()) };
    assert_eq!(status, SidereonStatus::Ok);
    let info = unsafe { info.assume_init() };
    if info.family == SidereonEngineErrorFamily::None && info.payload_len == 0 {
        return None;
    }
    let mut written = usize::MAX;
    let mut required = 0usize;
    let status = unsafe {
        sidereon_last_engine_error_payload(std::ptr::null_mut(), 0, &mut written, &mut required)
    };
    assert_eq!(status, SidereonStatus::Ok);
    assert_eq!(written, 0);
    assert_eq!(required, info.payload_len);
    if required == 0 {
        return Some((info, String::new()));
    }
    let mut buf = vec![0u8; required];
    let mut written2 = 0usize;
    let mut required2 = 0usize;
    let status = unsafe {
        sidereon_last_engine_error_payload(
            buf.as_mut_ptr(),
            buf.len(),
            &mut written2,
            &mut required2,
        )
    };
    assert_eq!(status, SidereonStatus::Ok);
    assert_eq!(written2, required);
    assert_eq!(required2, required);
    let payload = String::from_utf8(buf).expect("valid utf-8 payload");
    Some((info, payload))
}

#[cfg(test)]
mod tests {
    use super::{
        clear_engine_error, core_error_value, degrade_reason_name, dgnss_error_value,
        dop_error_value, engine_error_operation_boundary, engine_f64, iod_error_value,
        nmea_error_value, observables_error_value, record_engine_error, rtk_arc_error_value,
        rtk_ionosphere_free_arc_error_value, rtk_rinex_arc_error_value, rtk_static_arc_error_value,
        rtk_wide_lane_arc_error_value, rtk_wide_lane_fixed_arc_error_value, scenario_error_value,
        sidereon_clear_engine_error, sidereon_last_engine_error_info,
        sidereon_last_engine_error_payload, spp_error_value, static_reference_station_error_value,
        tle_fit_error_value, tle_fit_integer_value, tle_fit_optional_integer, trls_error_value,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo, LAST_ENGINE_ERROR,
    };
    use crate::SidereonStatus;
    use serde_json::{json, Value};
    use std::ptr;

    #[test]
    fn nmea_error_mapper_preserves_each_core_variant_and_payload() {
        use sidereon_core::nmea::{FieldError, NmeaError as E};

        let not_framed = nmea_error_value(&E::NotFramed {
            reason: "malformed checksum",
        });
        assert_eq!(
            not_framed,
            json!({"kind":"not_framed","fields":{"reason":"malformed checksum"}})
        );

        let checksum = nmea_error_value(&E::ChecksumMismatch {
            computed: 0x31,
            stated: 0x35,
        });
        assert_eq!(checksum["kind"], "checksum_mismatch");
        assert_eq!(checksum["fields"]["computed"], 0x31);
        assert_eq!(checksum["fields"]["stated"], 0x35);

        let unsupported = nmea_error_value(&E::UnsupportedType {
            address: "GPGSV".to_owned(),
        });
        assert_eq!(unsupported["fields"]["address"], "GPGSV");
        assert_eq!(
            nmea_error_value(&E::Proprietary {
                address: "PTEST".to_owned(),
            })["kind"],
            "proprietary"
        );

        let malformed = nmea_error_value(&E::MalformedField(FieldError::Missing {
            field: "latitude",
        }));
        assert_eq!(malformed["kind"], "malformed_field");
        assert_eq!(malformed["fields"]["cause"]["kind"], "missing");
        assert_eq!(malformed["fields"]["cause"]["fields"]["field"], "latitude");

        let invalid = nmea_error_value(&E::InvalidInput {
            field: "time",
            reason: "seconds of day is out of range",
        });
        assert_eq!(invalid["kind"], "invalid_input");
        assert_eq!(invalid["fields"]["field"], "time");
        assert_eq!(
            invalid["fields"]["reason"],
            "seconds of day is out of range"
        );
    }

    #[test]
    fn operation_boundary_clears_and_payload_retains_duplicate_nested_data() {
        record_engine_error(
            SidereonEngineErrorFamily::Rtk,
            "test_producer",
            json!({"kind": "test", "causes": [{"kind": "one"}, {"kind": "one"}]}),
        );
        engine_error_operation_boundary("test_success", 0, || 0);
        assert!(LAST_ENGINE_ERROR.with(|slot| slot.borrow().is_none()));

        let value = engine_f64(f64::NAN);
        assert_eq!(value["decimal"], json!("NaN"));
        assert_eq!(
            value["bits_hex"],
            json!(format!("{:016x}", f64::NAN.to_bits()))
        );
        record_engine_error(
            SidereonEngineErrorFamily::StaticReference,
            "failed",
            json!({"value": value}),
        );
        let retained = LAST_ENGINE_ERROR.with(|slot| slot.borrow().clone());
        assert!(retained.is_some());
        clear_engine_error();
    }

    #[test]
    fn engine_f64_preserves_exact_representation() {
        let cases = [
            (0.0, "0", 0.0f64.to_bits()),
            (-0.0, "-0", (-0.0f64).to_bits()),
            (1.5, "1.5", 1.5f64.to_bits()),
            (f64::INFINITY, "inf", f64::INFINITY.to_bits()),
            (f64::NEG_INFINITY, "-inf", f64::NEG_INFINITY.to_bits()),
            (f64::NAN, "NaN", f64::NAN.to_bits()),
        ];
        for (val, dec, bits) in cases {
            let v = engine_f64(val);
            assert_eq!(v["decimal"], json!(dec));
            assert_eq!(v["bits_hex"], json!(format!("{:016x}", bits)));
        }
    }

    #[test]
    fn table_driven_rtk_arc_error_mapping() {
        use sidereon_core::rtk_filter::RtkArcError as E;
        let cases: Vec<(E, &'static str)> = vec![
            (E::EmptyEpochs, "empty_epochs"),
            (
                E::TooFewSatellites {
                    count: 3,
                    minimum: 4,
                },
                "too_few_satellites",
            ),
            (E::InvalidEpochTime { epoch_index: 5 }, "invalid_epoch_time"),
            (
                E::MissingPosition {
                    epoch_index: 2,
                    satellite_id: "G12".into(),
                },
                "missing_position",
            ),
            (
                E::Reference(
                    sidereon_core::rtk::DoubleDifferenceError::TooFewCommonSatellites {
                        count: 1,
                        minimum: 2,
                    },
                ),
                "reference",
            ),
            (
                E::CodeSmoothing(sidereon_core::rtk::CodeSmoothingError::InvalidWindowCap),
                "code_smoothing",
            ),
        ];

        for (err, kind) in cases {
            let v = rtk_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
        }

        // Direct mapper verification for MissingPosition fields
        let missing_pos = E::MissingPosition {
            epoch_index: 2,
            satellite_id: "G12".into(),
        };
        let v_missing = rtk_arc_error_value(&missing_pos);
        assert_eq!(v_missing["kind"], json!("missing_position"));
        assert_eq!(v_missing["fields"]["epoch_index"], json!(2));
        assert_eq!(v_missing["fields"]["satellite_id"], json!("G12"));

        // Direct mapper verification for InvalidEpochTime fields
        let invalid_time = E::InvalidEpochTime { epoch_index: 5 };
        let v_time = rtk_arc_error_value(&invalid_time);
        assert_eq!(v_time["kind"], json!("invalid_epoch_time"));
        assert_eq!(v_time["fields"]["epoch_index"], json!(5));
    }

    #[test]
    fn table_driven_rtk_wide_lane_error_mapping() {
        use sidereon_core::rtk_filter::RtkWideLaneArcError as E;
        let cases: Vec<(E, &'static str, Option<&'static str>)> = vec![
            (E::EmptyEpochs, "empty_epochs", None),
            (
                E::WideLane(sidereon_core::rtk::WideLaneError::WideLaneNotInteger {
                    ambiguity_id: "G01".into(),
                    mean_cycles: 1.25,
                    fixed_cycles: 1,
                }),
                "wide_lane",
                Some("wide_lane_not_integer"),
            ),
            (
                E::WideLane(sidereon_core::rtk::WideLaneError::TooFewWideLaneEpochs {
                    ambiguity_id: "G02".into(),
                    count: 2,
                    minimum: 5,
                }),
                "wide_lane",
                Some("too_few_wide_lane_epochs"),
            ),
            (
                E::WideLane(sidereon_core::rtk::WideLaneError::WideLaneFailed {
                    satellite_id: "G03".into(),
                    reason: sidereon_core::carrier_phase::CarrierPhaseError::InvalidFrequency,
                }),
                "wide_lane",
                Some("wide_lane_failed"),
            ),
            (
                E::WideLane(
                    sidereon_core::rtk::WideLaneError::ReferenceSatelliteMissing("G04".into()),
                ),
                "wide_lane",
                Some("reference_satellite_missing"),
            ),
        ];

        for (err, kind, cause_kind) in cases {
            let v = rtk_wide_lane_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if let Some(ck) = cause_kind {
                assert_eq!(v["fields"]["cause"]["kind"], json!(ck));
            }
        }
    }

    #[test]
    fn table_driven_rtk_ionosphere_free_error_mapping() {
        use sidereon_core::rtk_filter::RtkIonosphereFreeArcError as E;
        let cases: Vec<(E, &'static str, Option<&'static str>)> = vec![
            (E::EmptyEpochs, "empty_epochs", None),
            (
                E::IonosphereFree(
                    sidereon_core::rtk::IonosphereFreeBaselineError::InconsistentFrequencies(
                        "G05".into(),
                    ),
                ),
                "ionosphere_free",
                Some("inconsistent_frequencies"),
            ),
            (
                E::IonosphereFree(
                    sidereon_core::rtk::IonosphereFreeBaselineError::NarrowLaneFailed(
                        sidereon_core::combinations::IonosphereFreeError::UnknownSystem('E'),
                    ),
                ),
                "ionosphere_free",
                Some("narrow_lane_failed"),
            ),
            (
                E::IonosphereFree(
                    sidereon_core::rtk::IonosphereFreeBaselineError::IonosphereFreeFailed {
                        satellite_id: "G06".into(),
                        reason: sidereon_core::combinations::IonosphereFreeError::EqualFrequencies,
                    },
                ),
                "ionosphere_free",
                Some("ionosphere_free_failed"),
            ),
        ];

        for (err, kind, cause_kind) in cases {
            let v = rtk_ionosphere_free_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if let Some(ck) = cause_kind {
                assert_eq!(v["fields"]["cause"]["kind"], json!(ck));
            }
        }
    }

    #[test]
    fn table_driven_rtk_wide_lane_fixed_error_mapping() {
        use sidereon_core::rtk_filter::RtkWideLaneFixedArcError as E;
        let cases: Vec<(E, &'static str)> = vec![
            (E::UnsupportedMultiGnss, "unsupported_multi_gnss"),
            (
                E::WideLane(sidereon_core::rtk_filter::RtkWideLaneArcError::EmptyEpochs),
                "wide_lane",
            ),
            (
                E::IonosphereFree(
                    sidereon_core::rtk_filter::RtkIonosphereFreeArcError::EmptyEpochs,
                ),
                "ionosphere_free",
            ),
            (
                E::Static(sidereon_core::rtk_filter::RtkStaticArcError::Arc(
                    sidereon_core::rtk_filter::RtkArcError::EmptyEpochs,
                )),
                "static",
            ),
            (
                E::Sequential(sidereon_core::rtk_filter::RtkArcError::EmptyEpochs),
                "sequential",
            ),
        ];

        for (err, kind) in cases {
            let v = rtk_wide_lane_fixed_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
        }
    }

    #[test]
    fn table_driven_rtk_static_arc_error_mapping() {
        use sidereon_core::rtk_filter::{FloatSolveError, RtkArcError, RtkStaticArcError as E};
        let cases = vec![
            (E::Arc(RtkArcError::EmptyEpochs), "arc"),
            (E::Float(FloatSolveError::SingularGeometry), "float"),
        ];
        for (err, kind) in cases {
            let v = rtk_static_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
        }
    }

    #[test]
    fn table_driven_rtk_rinex_error_mapping() {
        use sidereon_core::rtk_filter::RtkRinexArcError as E;
        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "min_common_satellites",
                    reason: "must be positive",
                },
                "invalid_input",
            ),
            (
                E::Observation(sidereon_core::Error::Parse("corrupt rinex bytes".into())),
                "observation",
            ),
            (
                E::Observation(sidereon_core::Error::InvalidInput(
                    "timeline discontinuity".into(),
                )),
                "observation",
            ),
            (
                E::Observation(sidereon_core::Error::MissingGlonassChannel),
                "observation",
            ),
            (
                E::Ephemeris {
                    satellite_id: "G07".into(),
                    epoch_j2000_s: 642_000.5,
                    reason: "unavailable state gap".into(),
                },
                "ephemeris",
            ),
            (E::NoSignalPairs, "no_signal_pairs"),
            (E::NoUsableEpochs, "no_usable_epochs"),
            (
                E::Ut1OutsideCoverage(sidereon_core::astro::time::DegradeReason::AfterCoverage),
                "ut1_outside_coverage",
            ),
        ];

        for (err, kind) in cases {
            let v = rtk_rinex_arc_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if kind == "observation" {
                match &err {
                    E::Observation(sidereon_core::Error::Parse(msg)) => {
                        assert_eq!(v["fields"]["cause"]["kind"], json!("parse"));
                        assert_eq!(v["fields"]["cause"]["fields"]["message"], json!(msg));
                    }
                    E::Observation(sidereon_core::Error::InvalidInput(msg)) => {
                        assert_eq!(v["fields"]["cause"]["kind"], json!("invalid_input"));
                        assert_eq!(v["fields"]["cause"]["fields"]["message"], json!(msg));
                    }
                    E::Observation(sidereon_core::Error::MissingGlonassChannel) => {
                        assert_eq!(
                            v["fields"]["cause"]["kind"],
                            json!("missing_glonass_channel")
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn table_driven_static_reference_error_mapping() {
        use sidereon_core::positioning::{
            StaticReferenceModeError, StaticReferenceModeReport, StaticReferenceModeStatus,
            StaticReferenceStationError as E, StaticReferenceStationMode,
        };

        let report1 = StaticReferenceModeReport {
            mode: StaticReferenceStationMode::CodeDgnss,
            status: StaticReferenceModeStatus::Failed,
            used_epochs: 0,
            skipped_epochs: 5,
            used_measurements: 0,
            error: Some(StaticReferenceModeError::NoMatchedCodeEpochs),
            unresolved_carriers: Vec::new(),
        };

        let report2 = StaticReferenceModeReport {
            mode: StaticReferenceStationMode::CarrierFixed,
            status: StaticReferenceModeStatus::Failed,
            used_epochs: 10,
            skipped_epochs: 2,
            used_measurements: 20,
            error: Some(StaticReferenceModeError::CarrierArc {
                reason: "cycle slip threshold exceeded".into(),
            }),
            unresolved_carriers: vec![sidereon_core::rtk_filter::RtkRinexUnresolvedCarrier {
                receiver: sidereon_core::rtk_filter::RtkRinexReceiver::Rover,
                epoch_index: 3,
                satellite_id: "R28".into(),
                observable_code: "L1C".into(),
            }],
        };

        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "reference_position_m",
                    reason: "must be finite",
                },
                "invalid_input",
            ),
            (E::NoEnabledModes, "no_enabled_modes"),
            (
                E::AllModesFailed {
                    mode_reports: vec![report1, report2],
                },
                "all_modes_failed",
            ),
        ];

        for (err, kind) in cases {
            let v = static_reference_station_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if kind == "all_modes_failed" {
                let reps = v["fields"]["mode_reports"].as_array().unwrap();
                assert_eq!(reps.len(), 2);
                assert_eq!(reps[0]["mode"], json!("code_dgnss"));
                assert_eq!(reps[0]["status"], json!("failed"));
                assert_eq!(reps[0]["skipped_epochs"], json!(5));
                assert_eq!(reps[0]["error"]["kind"], json!("no_matched_code_epochs"));

                assert_eq!(reps[1]["mode"], json!("carrier_fixed"));
                assert_eq!(reps[1]["status"], json!("failed"));
                assert_eq!(reps[1]["used_epochs"], json!(10));
                assert_eq!(reps[1]["used_measurements"], json!(20));
                assert_eq!(reps[1]["error"]["kind"], json!("carrier_arc"));
                let unres = reps[1]["unresolved_carriers"].as_array().unwrap();
                assert_eq!(unres.len(), 1);
                assert_eq!(unres[0]["receiver"], json!("rover"));
                assert_eq!(unres[0]["epoch_index"], json!(3));
                assert_eq!(unres[0]["satellite_id"], json!("R28"));
                assert_eq!(unres[0]["observable_code"], json!("L1C"));
            }
        }
    }

    #[test]
    fn table_driven_trls_error_mapping() {
        use trust_region_least_squares::trf::{BackendError, TrfError as E};

        let cases: Vec<(E, &'static str)> = vec![
            (E::EmptyResidual, "empty_residual"),
            (E::EmptyParameters, "empty_parameters"),
            (E::NonFiniteParameters, "non_finite_parameters"),
            (E::NonFiniteInitialResidual, "non_finite_initial_residual"),
            (E::InsufficientRows { m: 1, n: 3 }, "insufficient_rows"),
            (
                E::SizeOverflow {
                    m: usize::MAX,
                    n: 2,
                },
                "size_overflow",
            ),
            (E::DegreeOverflow { degree: usize::MAX }, "degree_overflow"),
            (E::InvalidMaxNfev, "invalid_max_nfev"),
            (E::InvalidFScale { f_scale: -0.5 }, "invalid_f_scale"),
            (
                E::InvalidXScaleLength {
                    expected: 4,
                    got: 2,
                },
                "invalid_x_scale_length",
            ),
            (
                E::InvalidXScaleValue {
                    index: 0,
                    value: -1.0,
                },
                "invalid_x_scale_value",
            ),
            (
                E::InvalidJacobianLength {
                    expected: 6,
                    got: 5,
                },
                "invalid_jacobian_length",
            ),
            (
                E::InvalidResidualLength {
                    expected: 4,
                    got: 3,
                },
                "invalid_residual_length",
            ),
            (
                E::InvalidSliceLength {
                    what: "a",
                    expected: 10,
                    got: 8,
                },
                "invalid_slice_length",
            ),
            (
                E::InvalidSvdOutput("singular values unordered".into()),
                "invalid_svd_output",
            ),
            (
                E::Backend(BackendError::BadDimensions {
                    expected_m: 2,
                    expected_n: 2,
                    got: 3,
                }),
                "backend",
            ),
        ];

        for (err, kind) in cases {
            let v = trls_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
        }
    }

    #[test]
    fn c_api_query_short_buffer_and_two_pass_retention() {
        clear_engine_error();

        unsafe {
            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::Rtk,
                payload_len: 999,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);

            // Record a TRLS error
            record_engine_error(
                SidereonEngineErrorFamily::Trls,
                "test_trls_op",
                json!({"kind": "insufficient_rows", "fields": {"m": 1, "n": 2}}),
            );

            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Trls);
            assert!(info.payload_len > 0);
            let expected_len = info.payload_len;

            // Pass 1: Query with null buffer
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required,),
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

            let parsed: Value = serde_json::from_slice(&buf).expect("valid JSON payload");
            assert_eq!(parsed["schema_version"], 1);
            assert_eq!(parsed["family"], "trls");
            assert_eq!(parsed["operation"], "test_trls_op");
            assert_eq!(parsed["error"]["kind"], "insufficient_rows");
            assert_eq!(parsed["error"]["fields"]["m"], 1);
            assert_eq!(parsed["error"]["fields"]["n"], 2);

            // Check that it still retains
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::Trls);
            assert_eq!(info.payload_len, expected_len);

            clear_engine_error();
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }

    #[test]
    fn tle_fit_integer_and_float_values_keep_full_boundaries() {
        assert_eq!(tle_fit_integer_value(i64::MIN).as_i64(), Some(i64::MIN));
        assert_eq!(tle_fit_integer_value(u64::MAX).as_u64(), Some(u64::MAX));
        assert_eq!(tle_fit_optional_integer(Some(i64::MIN)), json!(i64::MIN));
        assert_eq!(
            engine_f64(f64::from_bits(0x7ff8_0000_0000_0042)),
            json!({
                "decimal": "NaN",
                "bits_hex": "7ff8000000000042"
            })
        );
        assert_eq!(
            engine_f64(f64::NEG_INFINITY),
            json!({"decimal": "-inf", "bits_hex": "fff0000000000000"})
        );
    }

    #[test]
    fn tle_fit_arc_too_short_mapper_keeps_all_direct_fields() {
        let error = sidereon_core::astro::sgp4::TleFitError::ArcTooShort {
            samples: usize::MAX,
            needed: 3,
        };
        assert_eq!(
            tle_fit_error_value(&error),
            json!({
                "kind": "arc_too_short",
                "fields": {"samples": usize::MAX, "needed": 3}
            })
        );
    }

    #[test]
    fn engine_error_family_names() {
        assert_eq!(SidereonEngineErrorFamily::None.name(), "none");
        assert_eq!(SidereonEngineErrorFamily::Rtk.name(), "rtk");
        assert_eq!(
            SidereonEngineErrorFamily::StaticReference.name(),
            "static_reference"
        );
        assert_eq!(SidereonEngineErrorFamily::Trls.name(), "trls");
        assert_eq!(SidereonEngineErrorFamily::Ils.name(), "ils");
        assert_eq!(SidereonEngineErrorFamily::Spk.name(), "spk");
        assert_eq!(SidereonEngineErrorFamily::Cdm.name(), "cdm");
        assert_eq!(SidereonEngineErrorFamily::Tdm.name(), "tdm");
        assert_eq!(SidereonEngineErrorFamily::Fusion.name(), "fusion");
        assert_eq!(
            SidereonEngineErrorFamily::FusionStateCodec.name(),
            "fusion_state_codec"
        );
        assert_eq!(SidereonEngineErrorFamily::Allan.name(), "allan");
        assert_eq!(
            SidereonEngineErrorFamily::PowerLawNoise.name(),
            "power_law_noise"
        );
        assert_eq!(
            SidereonEngineErrorFamily::FrameCatalog.name(),
            "frame_catalog"
        );
        assert_eq!(SidereonEngineErrorFamily::Sidereal.name(), "sidereal");
        assert_eq!(SidereonEngineErrorFamily::Atmosphere.name(), "atmosphere");
        assert_eq!(
            SidereonEngineErrorFamily::SourceLocalization.name(),
            "source_localization"
        );
        assert_eq!(
            SidereonEngineErrorFamily::GeodeticTimeSeries.name(),
            "geodetic_time_series"
        );
        assert_eq!(SidereonEngineErrorFamily::Normality.name(), "normality");
        assert_eq!(SidereonEngineErrorFamily::Track.name(), "track");
        assert_eq!(
            SidereonEngineErrorFamily::PreciseSamples.name(),
            "precise_samples"
        );
        assert_eq!(
            SidereonEngineErrorFamily::PreciseInterpolant.name(),
            "precise_interpolant"
        );
        assert_eq!(
            SidereonEngineErrorFamily::SpaceWeather.name(),
            "space_weather"
        );
        assert_eq!(SidereonEngineErrorFamily::Araim.name(), "araim");
        assert_eq!(
            SidereonEngineErrorFamily::ReducedOrbit.name(),
            "reduced_orbit"
        );
        assert_eq!(
            SidereonEngineErrorFamily::ReducedOrbitSource.name(),
            "reduced_orbit_source"
        );
        assert_eq!(
            SidereonEngineErrorFamily::PiecewiseOrbit.name(),
            "piecewise_orbit"
        );
        assert_eq!(SidereonEngineErrorFamily::OrbitFit.name(), "orbit_fit");
        assert_eq!(SidereonEngineErrorFamily::Elements.name(), "elements");
        assert_eq!(SidereonEngineErrorFamily::Equinoctial.name(), "equinoctial");
        assert_eq!(SidereonEngineErrorFamily::RtnFrame.name(), "rtn_frame");
        assert_eq!(SidereonEngineErrorFamily::Anomaly.name(), "anomaly");
        assert_eq!(SidereonEngineErrorFamily::Propagation.name(), "propagation");
        assert_eq!(SidereonEngineErrorFamily::Decay.name(), "decay");
        assert_eq!(SidereonEngineErrorFamily::Dgnss.name(), "dgnss");
        assert_eq!(SidereonEngineErrorFamily::Scenario.name(), "scenario");
        assert_eq!(SidereonEngineErrorFamily::Catalog.name(), "catalog");
        assert_eq!(SidereonEngineErrorFamily::ExactCache.name(), "exact_cache");
        assert_eq!(SidereonEngineErrorFamily::Unknown.name(), "unknown");
        assert_eq!(SidereonEngineErrorFamily::Observables.name(), "observables");

        assert_eq!(SidereonEngineErrorFamily::None as u32, 0);
        assert_eq!(SidereonEngineErrorFamily::Rtk as u32, 1);
        assert_eq!(SidereonEngineErrorFamily::StaticReference as u32, 2);
        assert_eq!(SidereonEngineErrorFamily::Trls as u32, 3);
        assert_eq!(SidereonEngineErrorFamily::Ils as u32, 4);
        assert_eq!(SidereonEngineErrorFamily::Spk as u32, 5);
        assert_eq!(SidereonEngineErrorFamily::Cdm as u32, 6);
        assert_eq!(SidereonEngineErrorFamily::Tdm as u32, 7);
        assert_eq!(SidereonEngineErrorFamily::Fusion as u32, 8);
        assert_eq!(SidereonEngineErrorFamily::FusionStateCodec as u32, 9);
        assert_eq!(SidereonEngineErrorFamily::Allan as u32, 10);
        assert_eq!(SidereonEngineErrorFamily::PowerLawNoise as u32, 11);
        assert_eq!(SidereonEngineErrorFamily::FrameCatalog as u32, 12);
        assert_eq!(SidereonEngineErrorFamily::Sidereal as u32, 13);
        assert_eq!(SidereonEngineErrorFamily::Atmosphere as u32, 14);
        assert_eq!(SidereonEngineErrorFamily::SourceLocalization as u32, 15);
        assert_eq!(SidereonEngineErrorFamily::GeodeticTimeSeries as u32, 16);
        assert_eq!(SidereonEngineErrorFamily::Normality as u32, 17);
        assert_eq!(SidereonEngineErrorFamily::Track as u32, 18);
        assert_eq!(SidereonEngineErrorFamily::PreciseSamples as u32, 19);
        assert_eq!(SidereonEngineErrorFamily::PreciseInterpolant as u32, 20);
        assert_eq!(SidereonEngineErrorFamily::SpaceWeather as u32, 21);
        assert_eq!(SidereonEngineErrorFamily::Araim as u32, 22);
        assert_eq!(SidereonEngineErrorFamily::ReducedOrbit as u32, 23);
        assert_eq!(SidereonEngineErrorFamily::ReducedOrbitSource as u32, 24);
        assert_eq!(SidereonEngineErrorFamily::PiecewiseOrbit as u32, 25);
        assert_eq!(SidereonEngineErrorFamily::OrbitFit as u32, 26);
        assert_eq!(SidereonEngineErrorFamily::Elements as u32, 27);
        assert_eq!(SidereonEngineErrorFamily::Equinoctial as u32, 28);
        assert_eq!(SidereonEngineErrorFamily::RtnFrame as u32, 29);
        assert_eq!(SidereonEngineErrorFamily::Anomaly as u32, 30);
        assert_eq!(SidereonEngineErrorFamily::Propagation as u32, 31);
        assert_eq!(SidereonEngineErrorFamily::Decay as u32, 32);
        assert_eq!(SidereonEngineErrorFamily::Dgnss as u32, 33);
        assert_eq!(SidereonEngineErrorFamily::Scenario as u32, 34);
        assert_eq!(SidereonEngineErrorFamily::Catalog as u32, 35);
        assert_eq!(SidereonEngineErrorFamily::ExactCache as u32, 36);
        assert_eq!(SidereonEngineErrorFamily::Unknown as u32, 999);
    }

    #[test]
    fn table_driven_dgnss_error_mapping() {
        use sidereon_core::astro::time::DegradeReason;
        use sidereon_core::dgnss::DgnssError as E;
        use sidereon_core::positioning::{SppError, SppInputErrorKind};

        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "rover_observation.pseudorange_m",
                    reason: "not positive",
                },
                "invalid_input",
            ),
            (
                E::Spp(SppError::InvalidInput {
                    field: "receiver_clock_bias_s",
                    kind: SppInputErrorKind::NonFinite,
                }),
                "spp",
            ),
            (
                E::Ut1OutsideCoverage(DegradeReason::BeforeCoverage),
                "ut1_outside_coverage",
            ),
        ];

        for (err, kind) in cases {
            let v = dgnss_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            match &err {
                E::InvalidInput { field, reason } => {
                    assert_eq!(v["fields"]["field"], json!(*field));
                    assert_eq!(v["fields"]["reason"], json!(*reason));
                }
                E::Spp(_) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("invalid_input"));
                    assert_eq!(
                        v["fields"]["cause"]["fields"]["field"],
                        json!("receiver_clock_bias_s")
                    );
                    assert_eq!(v["fields"]["cause"]["fields"]["kind"], json!("non_finite"));
                }
                E::Ut1OutsideCoverage(reason) => {
                    assert_eq!(v["fields"]["reason"], json!(degrade_reason_name(*reason)));
                }
            }
        }
    }

    #[test]
    fn table_driven_spp_error_mapping() {
        use sidereon_core::astro::math::least_squares::SolveError;
        use sidereon_core::astro::time::DegradeReason;
        use sidereon_core::positioning::{SppError as E, SppInputErrorKind as K};
        use sidereon_core::{GnssSatelliteId, GnssSystem};

        let sat = GnssSatelliteId::new(GnssSystem::Gps, 1).expect("valid satellite");

        let input_kinds = [
            (K::NonFinite, "non_finite"),
            (K::NotPositive, "not_positive"),
            (K::Negative, "negative"),
            (K::OutOfRange, "out_of_range"),
            (K::Missing, "missing"),
            (K::FloatParse, "float_parse"),
            (K::IntParse, "int_parse"),
            (K::InvalidCivilDate, "invalid_civil_date"),
            (K::InvalidCivilTime, "invalid_civil_time"),
        ];

        for (kind_enum, kind_str) in input_kinds {
            let err = E::InvalidInput {
                field: "test_field",
                kind: kind_enum,
            };
            let v = spp_error_value(&err);
            assert_eq!(v["kind"], json!("invalid_input"));
            assert_eq!(v["fields"]["field"], json!("test_field"));
            assert_eq!(v["fields"]["kind"], json!(kind_str));
        }

        let cases: Vec<(E, &'static str)> = vec![
            (
                E::TooFewSatellites {
                    used: 3,
                    required: 4,
                },
                "too_few_satellites",
            ),
            (E::Singular(SolveError::SingularJacobian), "singular"),
            (
                E::Singular(SolveError::InvalidInput {
                    field: "jacobian",
                    reason: "empty",
                }),
                "singular",
            ),
            (
                E::DuplicateObservation { satellite: sat },
                "duplicate_observation",
            ),
            (E::EphemerisLost { satellite: sat }, "ephemeris_lost"),
            (E::SelectionUnsettled { passes: 10 }, "selection_unsettled"),
            (
                E::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
                "ut1_outside_coverage",
            ),
        ];

        for (err, kind) in cases {
            let v = spp_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            match &err {
                E::TooFewSatellites { used, required } => {
                    assert_eq!(v["fields"]["used"], json!(*used));
                    assert_eq!(v["fields"]["required"], json!(*required));
                }
                E::Singular(SolveError::SingularJacobian) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("singular_jacobian"));
                }
                E::Singular(SolveError::InvalidInput { field, reason }) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("invalid_input"));
                    assert_eq!(v["fields"]["cause"]["fields"]["field"], json!(*field));
                    assert_eq!(v["fields"]["cause"]["fields"]["reason"], json!(*reason));
                }
                E::DuplicateObservation { satellite } => {
                    assert_eq!(v["fields"]["satellite_id"], json!(satellite.to_string()));
                }
                E::EphemerisLost { satellite } => {
                    assert_eq!(v["fields"]["satellite_id"], json!(satellite.to_string()));
                }
                E::SelectionUnsettled { passes } => {
                    assert_eq!(v["fields"]["passes"], json!(*passes));
                }
                E::Ut1OutsideCoverage(reason) => {
                    assert_eq!(v["fields"]["reason"], json!(degrade_reason_name(*reason)));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn table_driven_scenario_error_mapping() {
        use sidereon_core::astro::time::DegradeReason;
        use sidereon_core::observables::{ObservablesError, ObservablesInputErrorKind};
        use sidereon_core::scenario::ScenarioError as E;
        use sidereon_core::{GnssSatelliteId, GnssSystem};

        let sat = GnssSatelliteId::new(GnssSystem::Galileo, 5).expect("valid satellite");

        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "schema_version",
                    reason: "unsupported schema version",
                },
                "invalid_input",
            ),
            (E::ExternalSourceRequired, "external_source_required"),
            (
                E::ExternalSourceMismatch {
                    field: "sp3",
                    expected: "sp3_exp".into(),
                    actual: "sp3_act".into(),
                },
                "external_source_mismatch",
            ),
            (
                E::ExternalIonosphereRequired,
                "external_ionosphere_required",
            ),
            (E::Ionosphere("grid truncated".into()), "ionosphere"),
            (E::NoEphemeris { satellite: sat }, "no_ephemeris"),
            (
                E::Ut1OutsideCoverage {
                    satellite: sat,
                    reason: DegradeReason::BeforeCoverage,
                },
                "ut1_outside_coverage",
            ),
            (
                E::Observable(ObservablesError::InvalidInput {
                    field: "carrier",
                    kind: ObservablesInputErrorKind::NotPositive,
                }),
                "observable",
            ),
            (E::Frame("unsupported frame".into()), "frame"),
        ];

        for (err, kind) in cases {
            let v = scenario_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            match &err {
                E::InvalidInput { field, reason } => {
                    assert_eq!(v["fields"]["field"], json!(*field));
                    assert_eq!(v["fields"]["reason"], json!(*reason));
                }
                E::ExternalSourceRequired | E::ExternalIonosphereRequired => {
                    assert_eq!(v["fields"], json!({}));
                }
                E::ExternalSourceMismatch {
                    field,
                    expected,
                    actual,
                } => {
                    assert_eq!(v["fields"]["field"], json!(*field));
                    assert_eq!(v["fields"]["expected"], json!(expected));
                    assert_eq!(v["fields"]["actual"], json!(actual));
                }
                E::Ionosphere(msg) => {
                    assert_eq!(v["fields"]["message"], json!(msg));
                }
                E::NoEphemeris { satellite } => {
                    assert_eq!(v["fields"]["satellite_id"], json!(satellite.to_string()));
                }
                E::Ut1OutsideCoverage { satellite, reason } => {
                    assert_eq!(v["fields"]["satellite_id"], json!(satellite.to_string()));
                    assert_eq!(v["fields"]["reason"], json!(degrade_reason_name(*reason)));
                }
                E::Observable(_) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("invalid_input"));
                    assert_eq!(v["fields"]["cause"]["fields"]["field"], json!("carrier"));
                    assert_eq!(
                        v["fields"]["cause"]["fields"]["kind"],
                        json!("not_positive")
                    );
                }
                E::Frame(msg) => {
                    assert_eq!(v["fields"]["message"], json!(msg));
                }
            }
        }
    }

    #[test]
    fn table_driven_observables_error_mapping() {
        use sidereon_core::observables::{ObservablesError as E, ObservablesInputErrorKind as K};
        use sidereon_core::{Error as CoreError, GnssSatelliteId, GnssSystem};
        let unknown = GnssSatelliteId::new(GnssSystem::Gps, 99).expect("valid token");

        let input_kinds = [
            (K::NonFinite, "non_finite"),
            (K::NotPositive, "not_positive"),
            (K::Negative, "negative"),
            (K::OutOfRange, "out_of_range"),
            (K::Missing, "missing"),
            (K::FloatParse, "float_parse"),
            (K::IntParse, "int_parse"),
            (K::InvalidCivilDate, "invalid_civil_date"),
            (K::InvalidCivilTime, "invalid_civil_time"),
        ];

        for (kind_enum, kind_str) in input_kinds {
            let err = E::InvalidInput {
                field: "obs_input",
                kind: kind_enum,
            };
            let v = observables_error_value(&err);
            assert_eq!(v["kind"], json!("invalid_input"));
            assert_eq!(v["fields"]["field"], json!("obs_input"));
            assert_eq!(v["fields"]["kind"], json!(kind_str));
        }

        let cases: Vec<(E, &'static str)> = vec![
            (E::NoEphemeris, "no_ephemeris"),
            (E::Ephemeris(CoreError::EpochOutOfRange), "ephemeris"),
            (
                E::Ephemeris(CoreError::UnknownSatellite(unknown)),
                "ephemeris",
            ),
            (E::Media(CoreError::MissingGlonassChannel), "media"),
        ];

        for (err, kind) in cases {
            let v = observables_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            match &err {
                E::NoEphemeris => assert_eq!(v["fields"], json!({})),
                E::Ephemeris(CoreError::EpochOutOfRange) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("epoch_out_of_range"));
                }
                E::Ephemeris(CoreError::UnknownSatellite(satellite)) => {
                    assert_eq!(v["fields"]["cause"]["kind"], json!("unknown_satellite"));
                    assert_eq!(
                        v["fields"]["cause"]["fields"]["satellite_id"],
                        json!(satellite.to_string())
                    );
                }
                E::Media(_) => {
                    assert_eq!(
                        v["fields"]["cause"]["kind"],
                        json!("missing_glonass_channel")
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn table_driven_core_error_lossless_mapping() {
        use sidereon_core::astro::time::TimeScale;
        use sidereon_core::atmosphere::ionosphere::{
            IonexCoverageError, IonexEpochError, IonexMappingDeclaration, IonexMissingNodes,
            IonexNodeGap, IonexSlantRefusal,
        };
        use sidereon_core::ephemeris::{
            ContinuityOptionRejection, ContinuityOptionsError, MergeToleranceError,
            MergeToleranceField, Sp3EpochIntervalError, Sp3EpochIntervalRejection,
        };
        use sidereon_core::rinex::observations::RinexObsWriteError;
        use sidereon_core::rtcm::{RtcmConversionError, RtcmEncodeError};
        use sidereon_core::sbas::SbasEncodeError;
        use sidereon_core::terrain::{DtedHorizontalDatum, DtedTileError};
        use sidereon_core::Error as E;

        // 1. Terrain datum variants
        let datums = [
            (DtedHorizontalDatum::Wgs84, "wgs84"),
            (DtedHorizontalDatum::Wgs72, "wgs72"),
            (DtedHorizontalDatum::Unstated, "unstated"),
            (DtedHorizontalDatum::Other("ED50".into()), "other"),
        ];
        for (datum, expected_kind) in datums {
            let err = E::NonWgs84TerrainTile {
                lat_index: 45,
                lon_index: -73,
                datum,
            };
            let v = core_error_value(&err);
            assert_eq!(v["kind"], json!("non_wgs84_terrain_tile"));
            assert_eq!(v["fields"]["lat_index"], json!(45));
            assert_eq!(v["fields"]["lon_index"], json!(-73));
            assert_eq!(v["fields"]["datum"]["kind"], json!(expected_kind));
        }

        // 2. DtedTileError variants
        let dted_errors: Vec<(DtedTileError, &'static str)> = vec![
            (
                DtedTileError::Io {
                    path: "/tiles/n45.dt2".into(),
                    message: "permission denied".into(),
                },
                "io",
            ),
            (
                DtedTileError::TooShort {
                    path: "/tiles/n45.dt2".into(),
                },
                "too_short",
            ),
            (
                DtedTileError::MissingUhl1 {
                    path: "/tiles/n45.dt2".into(),
                },
                "missing_uhl1",
            ),
            (
                DtedTileError::InvalidEncoding("bad utf-8".into()),
                "invalid_encoding",
            ),
            (
                DtedTileError::InvalidField("corrupt field".into()),
                "invalid_field",
            ),
            (
                DtedTileError::InvalidDimensions {
                    path: "/tiles/n45.dt2".into(),
                    lon_count: 1,
                    lat_count: 1,
                },
                "invalid_dimensions",
            ),
            (
                DtedTileError::Truncated {
                    path: "/tiles/n45.dt2".into(),
                    actual: 100,
                    expected: 200,
                },
                "truncated",
            ),
            (
                DtedTileError::Outside {
                    longitude: 10.5,
                    latitude: 20.5,
                    origin_longitude: 0.0,
                    origin_latitude: 0.0,
                },
                "outside",
            ),
            (
                DtedTileError::PostingIndexOutOfBounds {
                    longitude_index: 1000,
                    latitude_index: 2000,
                },
                "posting_index_out_of_bounds",
            ),
            (
                DtedTileError::MissingDataSentinel { longitude_index: 3 },
                "missing_data_sentinel",
            ),
            (
                DtedTileError::Checksum {
                    longitude_index: 3,
                    checksum: 1234,
                    sum: 5678,
                },
                "checksum",
            ),
            (DtedTileError::EmptyCoordinate, "empty_coordinate"),
            (
                DtedTileError::InvalidHemisphere { hemisphere: 'X' },
                "invalid_hemisphere",
            ),
            (
                DtedTileError::NegativePostingIndex { index: -5 },
                "negative_posting_index",
            ),
            (
                DtedTileError::CoordinateOutOfRange {
                    field: "lat",
                    text: "95".into(),
                },
                "coordinate_out_of_range",
            ),
            (
                DtedTileError::WrongHemisphere {
                    field: "lon",
                    hemisphere: 'N',
                    expected: "E/W",
                },
                "wrong_hemisphere",
            ),
            (
                DtedTileError::OriginNotWholeDegree {
                    field: "origin",
                    text: "45.5".into(),
                },
                "origin_not_whole_degree",
            ),
            (
                DtedTileError::IntervalCountMismatch {
                    field: "interval",
                    interval_tenths_arcsec: 30,
                    count: 100,
                },
                "interval_count_mismatch",
            ),
            (
                DtedTileError::ProfileLongitudeCountMismatch {
                    longitude_index: 2,
                    declared: 5,
                },
                "profile_longitude_count_mismatch",
            ),
            (
                DtedTileError::UnsupportedPartialProfile {
                    longitude_index: 2,
                    first_latitude_index: 10,
                },
                "unsupported_partial_profile",
            ),
            (
                DtedTileError::NullPosting {
                    longitude_index: 4,
                    latitude_index: 8,
                },
                "null_posting",
            ),
        ];

        for (tile_err, expected_kind) in dted_errors {
            let err = E::TerrainTile {
                lat_index: 45,
                lon_index: -73,
                error: Box::new(tile_err),
            };
            let v = core_error_value(&err);
            assert_eq!(v["kind"], json!("terrain_tile"));
            assert_eq!(v["fields"]["cause"]["kind"], json!(expected_kind));
        }

        // 3. IonexCoverageError
        let cov_cases = [
            (
                IonexCoverageError::EpochBeforeFirstMap,
                "epoch_before_first_map",
            ),
            (
                IonexCoverageError::EpochAfterLastMap,
                "epoch_after_last_map",
            ),
            (
                IonexCoverageError::LatitudeOutOfRange,
                "latitude_out_of_range",
            ),
            (
                IonexCoverageError::LongitudeOutOfRange,
                "longitude_out_of_range",
            ),
        ];
        for (cov_err, expected_kind) in cov_cases {
            let err = E::IonexOutOfCoverage(cov_err);
            let v = core_error_value(&err);
            assert_eq!(v["kind"], json!("ionex_out_of_coverage"));
            assert_eq!(v["fields"]["cause"]["kind"], json!(expected_kind));
        }

        // 4. IonexNodeGap
        let gap_err = E::IonexNodesNotAvailable(Box::new(IonexNodeGap {
            earlier: Some(IonexMissingNodes {
                map_number: 1,
                lat_index: 10,
                lon_index: 20,
                lon_index_next: 21,
                missing: [true, false, true, false],
            }),
            later: None,
        }));
        let v_gap = core_error_value(&gap_err);
        assert_eq!(v_gap["kind"], json!("ionex_nodes_not_available"));
        assert_eq!(v_gap["fields"]["cause"]["kind"], json!("node_gap"));
        assert_eq!(
            v_gap["fields"]["cause"]["fields"]["earlier"]["fields"]["map_number"],
            json!(1)
        );

        // 5. IonexSlantRefusal
        let slant_cases: Vec<(IonexSlantRefusal, &'static str)> = vec![
            (
                IonexSlantRefusal::VaryingHeights {
                    map_number: 2,
                    lat_index: 5,
                    lon_index: 6,
                },
                "varying_heights",
            ),
            (
                IonexSlantRefusal::HeightNotAvailable {
                    map_number: 2,
                    lat_index: 5,
                    lon_index: 6,
                },
                "height_not_available",
            ),
            (
                IonexSlantRefusal::MappingFunction(IonexMappingDeclaration::Absent),
                "mapping_function",
            ),
        ];
        for (slant_err, expected_kind) in slant_cases {
            let err = E::IonexSlantUnavailable(slant_err);
            let v = core_error_value(&err);
            assert_eq!(v["kind"], json!("ionex_slant_unavailable"));
            assert_eq!(v["fields"]["cause"]["kind"], json!(expected_kind));
        }

        // 6. IonexEpochError
        let epoch_cases = [
            (
                IonexEpochError::NotWholeSecond {
                    scale: TimeScale::Utc,
                },
                "not_whole_second",
                "UTC",
            ),
            (
                IonexEpochError::FractionalUtcSecond {
                    scale: TimeScale::Tt,
                },
                "fractional_utc_second",
                "TT",
            ),
            (
                IonexEpochError::NoExactUtcOffset {
                    scale: TimeScale::Tcg,
                },
                "no_exact_utc_offset",
                "TCG",
            ),
            (
                IonexEpochError::InsertedLeapSecond {
                    scale: TimeScale::Utc,
                },
                "inserted_leap_second",
                "UTC",
            ),
            (
                IonexEpochError::BeforeIntegerLeapSeconds {
                    scale: TimeScale::Utc,
                },
                "before_integer_leap_seconds",
                "UTC",
            ),
            (
                IonexEpochError::OutOfRange {
                    scale: TimeScale::Tai,
                },
                "out_of_range",
                "TAI",
            ),
        ];
        for (epoch_err, expected_kind, expected_scale) in epoch_cases {
            let err = E::IonexEpoch(epoch_err);
            let v = core_error_value(&err);
            assert_eq!(v["kind"], json!("ionex_epoch"));
            assert_eq!(v["fields"]["cause"]["kind"], json!(expected_kind));
            assert_eq!(
                v["fields"]["cause"]["fields"]["scale"],
                json!(expected_scale)
            );
        }

        let year_err = E::IonexEpoch(IonexEpochError::YearOutOfField {
            utc_j2000_s: 1_000_000_000,
        });
        let v_year = core_error_value(&year_err);
        assert_eq!(v_year["kind"], json!("ionex_epoch"));
        assert_eq!(
            v_year["fields"]["cause"]["kind"],
            json!("year_out_of_field")
        );
        assert_eq!(
            v_year["fields"]["cause"]["fields"]["utc_j2000_s"],
            json!(1_000_000_000)
        );

        // 7. Sp3EpochIntervalError
        let sp3_int_err = E::Sp3EpochInterval(Sp3EpochIntervalError {
            field: "interval",
            value: 30.0,
            reason: Sp3EpochIntervalRejection::NotWholeTicks,
        });
        let v_sp3_int = core_error_value(&sp3_int_err);
        assert_eq!(v_sp3_int["kind"], json!("sp3_epoch_interval"));
        assert_eq!(
            v_sp3_int["fields"]["cause"]["fields"]["reason"],
            json!("not_whole_ticks")
        );

        // 8. MergeToleranceError
        let merge_err = E::Sp3MergeTolerance(MergeToleranceError {
            field: MergeToleranceField::Clock,
            value: -0.001,
        });
        let v_merge = core_error_value(&merge_err);
        assert_eq!(v_merge["kind"], json!("sp3_merge_tolerance"));
        assert_eq!(
            v_merge["fields"]["cause"]["fields"]["field"],
            json!("clock")
        );

        // 9. ContinuityOptionsError
        let cont_err = E::ContinuityOptions(ContinuityOptionsError {
            field: "speed_bound",
            value: -1.0,
            reason: ContinuityOptionRejection::Negative,
        });
        let v_cont = core_error_value(&cont_err);
        assert_eq!(v_cont["kind"], json!("continuity_options"));
        assert_eq!(
            v_cont["fields"]["cause"]["fields"]["reason"],
            json!("negative")
        );

        // 10. SbasEncodeError
        let sbas_err = E::SbasEncode(Box::new(SbasEncodeError::UnrecognizedPreamble {
            preamble: 0x42,
        }));
        let v_sbas = core_error_value(&sbas_err);
        assert_eq!(v_sbas["kind"], json!("sbas_encode"));
        assert_eq!(
            v_sbas["fields"]["cause"]["kind"],
            json!("unrecognized_preamble")
        );
        assert_eq!(v_sbas["fields"]["cause"]["fields"]["preamble"], json!(0x42));

        // 11. RtcmEncodeError
        let rtcm_enc_err = E::RtcmEncode(Box::new(RtcmEncodeError::NegativeZeroWithValue {
            message_number: 1020,
            field: "df001".into(),
            value: 42,
        }));
        let v_rtcm_enc = core_error_value(&rtcm_enc_err);
        assert_eq!(v_rtcm_enc["kind"], json!("rtcm_encode"));
        assert_eq!(
            v_rtcm_enc["fields"]["cause"]["kind"],
            json!("negative_zero_with_value")
        );

        let rtcm_dep_ssr = E::RtcmEncode(Box::new(RtcmEncodeError::StrictDeparture(
            sidereon_core::rtcm::RtcmDeparture::SsrRecordsShort {
                message_number: 1057,
                declared: 10,
                read: 8,
            },
        )));
        let v_dep_ssr = core_error_value(&rtcm_dep_ssr);
        assert_eq!(v_dep_ssr["kind"], json!("rtcm_encode"));
        assert_eq!(
            v_dep_ssr["fields"]["cause"]["kind"],
            json!("strict_departure")
        );
        assert_eq!(
            v_dep_ssr["fields"]["cause"]["fields"]["departure"]["kind"],
            json!("ssr_records_short")
        );
        assert_eq!(
            v_dep_ssr["fields"]["cause"]["fields"]["departure"]["fields"]["message_number"],
            json!(1057)
        );
        assert_eq!(
            v_dep_ssr["fields"]["cause"]["fields"]["departure"]["fields"]["declared"],
            json!(10)
        );
        assert_eq!(
            v_dep_ssr["fields"]["cause"]["fields"]["departure"]["fields"]["read"],
            json!(8)
        );

        let rtcm_dep_rec = E::RtcmEncode(Box::new(RtcmEncodeError::StrictDeparture(
            sidereon_core::rtcm::RtcmDeparture::RecordsShort {
                message_number: 1004,
                declared: 6,
                read: 4,
            },
        )));
        let v_dep_rec = core_error_value(&rtcm_dep_rec);
        assert_eq!(v_dep_rec["kind"], json!("rtcm_encode"));
        assert_eq!(
            v_dep_rec["fields"]["cause"]["kind"],
            json!("strict_departure")
        );
        assert_eq!(
            v_dep_rec["fields"]["cause"]["fields"]["departure"]["kind"],
            json!("records_short")
        );
        assert_eq!(
            v_dep_rec["fields"]["cause"]["fields"]["departure"]["fields"]["message_number"],
            json!(1004)
        );
        assert_eq!(
            v_dep_rec["fields"]["cause"]["fields"]["departure"]["fields"]["declared"],
            json!(6)
        );
        assert_eq!(
            v_dep_rec["fields"]["cause"]["fields"]["departure"]["fields"]["read"],
            json!(4)
        );

        // 12. RtcmConversionError
        let rtcm_conv_err = E::RtcmConversion(Box::new(RtcmConversionError::GalileoWeekOverflow));
        let v_rtcm_conv = core_error_value(&rtcm_conv_err);
        assert_eq!(v_rtcm_conv["kind"], json!("rtcm_conversion"));
        assert_eq!(
            v_rtcm_conv["fields"]["cause"]["kind"],
            json!("galileo_week_overflow")
        );

        let vtec_missing = E::RtcmConversion(Box::new(RtcmConversionError::VtecEvaluation(
            sidereon_core::rtcm::VtecEvaluationProblem::MissingCoefficient {
                layer_index: 1,
                field: "cosine",
                index: 3,
            },
        )));
        let v_vtec_missing = core_error_value(&vtec_missing);
        assert_eq!(v_vtec_missing["kind"], json!("rtcm_conversion"));
        assert_eq!(
            v_vtec_missing["fields"]["cause"]["kind"],
            json!("vtec_evaluation")
        );
        assert_eq!(
            v_vtec_missing["fields"]["cause"]["fields"]["cause"]["kind"],
            json!("missing_coefficient")
        );
        assert_eq!(
            v_vtec_missing["fields"]["cause"]["fields"]["cause"]["fields"]["layer_index"],
            json!(1)
        );
        assert_eq!(
            v_vtec_missing["fields"]["cause"]["fields"]["cause"]["fields"]["field"],
            json!("cosine")
        );
        assert_eq!(
            v_vtec_missing["fields"]["cause"]["fields"]["cause"]["fields"]["index"],
            json!(3)
        );

        let vtec_factor = E::RtcmConversion(Box::new(RtcmConversionError::VtecEvaluation(
            sidereon_core::rtcm::VtecEvaluationProblem::InvalidMappingFactor { layer_index: 2 },
        )));
        let v_vtec_factor = core_error_value(&vtec_factor);
        assert_eq!(v_vtec_factor["kind"], json!("rtcm_conversion"));
        assert_eq!(
            v_vtec_factor["fields"]["cause"]["kind"],
            json!("vtec_evaluation")
        );
        assert_eq!(
            v_vtec_factor["fields"]["cause"]["fields"]["cause"]["kind"],
            json!("invalid_mapping_factor")
        );
        assert_eq!(
            v_vtec_factor["fields"]["cause"]["fields"]["cause"]["fields"]["layer_index"],
            json!(2)
        );

        let vtec_range = E::RtcmConversion(Box::new(RtcmConversionError::VtecEvaluation(
            sidereon_core::rtcm::VtecEvaluationProblem::PhysicalResultOutOfRange { field: "STEC" },
        )));
        let v_vtec_range = core_error_value(&vtec_range);
        assert_eq!(v_vtec_range["kind"], json!("rtcm_conversion"));
        assert_eq!(
            v_vtec_range["fields"]["cause"]["kind"],
            json!("vtec_evaluation")
        );
        assert_eq!(
            v_vtec_range["fields"]["cause"]["fields"]["cause"]["kind"],
            json!("physical_result_out_of_range")
        );
        assert_eq!(
            v_vtec_range["fields"]["cause"]["fields"]["cause"]["fields"]["field"],
            json!("STEC")
        );

        // 13. Nested Scenario -> Observable -> Core integration assertion
        let scen_err = sidereon_core::scenario::ScenarioError::Observable(
            sidereon_core::observables::ObservablesError::Ephemeris(
                sidereon_core::Error::IonexEpoch(IonexEpochError::OutOfRange {
                    scale: TimeScale::Gpst,
                }),
            ),
        );
        let v_scen = scenario_error_value(&scen_err);
        assert_eq!(v_scen["kind"], json!("observable"));
        assert_eq!(v_scen["fields"]["cause"]["kind"], json!("ephemeris"));
        assert_eq!(
            v_scen["fields"]["cause"]["fields"]["cause"]["kind"],
            json!("ionex_epoch")
        );
        assert_eq!(
            v_scen["fields"]["cause"]["fields"]["cause"]["fields"]["cause"]["kind"],
            json!("out_of_range")
        );
        assert_eq!(
            v_scen["fields"]["cause"]["fields"]["cause"]["fields"]["cause"]["fields"]["scale"],
            json!("GPST")
        );

        // Display and each public conversion retain the typed cause. The
        // truncated recognized body traverses both private RTCM conversions:
        // OutOfInput -> DecodeError -> Error.
        assert_eq!(
            E::Parse("bad line".into()).to_string(),
            "parse error: bad line"
        );

        let rinex_source = RinexObsWriteError::NotVersionTwo { version: 3.0 };
        let rinex_message = rinex_source.to_string();
        let rinex_error: E = rinex_source.into();
        assert_eq!(rinex_error, E::InvalidInput(rinex_message));

        let rtcm_encode_source = RtcmEncodeError::NegativeZeroWithValue {
            message_number: 1020,
            field: "df001".into(),
            value: 42,
        };
        let rtcm_encode_expected = rtcm_encode_source.clone();
        let rtcm_encode_error: E = rtcm_encode_source.into();
        assert_eq!(
            rtcm_encode_error,
            E::RtcmEncode(Box::new(rtcm_encode_expected))
        );

        let rtcm_conversion_source = RtcmConversionError::GalileoWeekOverflow;
        let rtcm_conversion_expected = rtcm_conversion_source.clone();
        let rtcm_conversion_error: E = rtcm_conversion_source.into();
        assert_eq!(
            rtcm_conversion_error,
            E::RtcmConversion(Box::new(rtcm_conversion_expected))
        );

        let sbas_source = SbasEncodeError::UnrecognizedPreamble { preamble: 0x42 };
        let sbas_expected = sbas_source.clone();
        let sbas_error: E = sbas_source.into();
        assert_eq!(sbas_error, E::SbasEncode(Box::new(sbas_expected)));

        let truncated = sidereon_core::rtcm::Message::decode(&[0x3e, 0xd0])
            .expect_err("recognized RTCM 1005 body must be truncated");
        match truncated {
            E::Parse(message) => assert!(
                message.contains("RTCM body truncated"),
                "unexpected truncated-body message: {message}"
            ),
            other => panic!("expected parse error from truncated RTCM body, got {other:?}"),
        }
    }

    #[test]
    fn table_driven_dop_error_mapping() {
        use sidereon_core::dop::DopError as E;
        let cases: Vec<(E, &'static str)> = vec![
            (
                E::InvalidInput {
                    field: "positions",
                    reason: "non-finite",
                },
                "invalid_input",
            ),
            (E::TooFewSatellites, "too_few_satellites"),
            (E::Singular, "singular"),
        ];
        for (err, kind) in cases {
            let v = dop_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if kind == "invalid_input" {
                assert_eq!(v["fields"]["field"], json!("positions"));
                assert_eq!(v["fields"]["reason"], json!("non-finite"));
            }
        }
    }

    #[test]
    fn clear_engine_error_export_resets_only_generic_tls_slot() {
        use std::ffi::CStr;
        use std::os::raw::c_char;

        const LEGACY_MSG: &str = "independently seeded legacy diagnostic text";
        crate::set_last_error(LEGACY_MSG);

        record_engine_error(
            SidereonEngineErrorFamily::Propagation,
            "focused_control_operation",
            json!({"cause": "numerical_failure"}),
        );

        let mut info = SidereonEngineErrorInfo {
            family: SidereonEngineErrorFamily::None,
            payload_len: 0,
        };
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Propagation);
        assert!(info.payload_len > 0);

        sidereon_clear_engine_error();

        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::None);
        assert_eq!(info.payload_len, 0);

        let mut written = 999;
        let mut required = 999;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required)
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        assert_eq!(required, 0);

        let mut legacy_buf = [0 as c_char; 128];
        let needed = unsafe {
            crate::sidereon_last_error_message(legacy_buf.as_mut_ptr(), legacy_buf.len())
        };
        assert_eq!(needed, LEGACY_MSG.len());
        let legacy_str = unsafe { CStr::from_ptr(legacy_buf.as_ptr()) }
            .to_str()
            .expect("valid utf-8");
        assert_eq!(legacy_str, LEGACY_MSG);
    }

    #[test]
    fn all_registered_families_match_frozen_ids_and_names() {
        let expected = [
            (SidereonEngineErrorFamily::None, 0, "none"),
            (SidereonEngineErrorFamily::Rtk, 1, "rtk"),
            (
                SidereonEngineErrorFamily::StaticReference,
                2,
                "static_reference",
            ),
            (SidereonEngineErrorFamily::Trls, 3, "trls"),
            (SidereonEngineErrorFamily::Ils, 4, "ils"),
            (SidereonEngineErrorFamily::Spk, 5, "spk"),
            (SidereonEngineErrorFamily::Cdm, 6, "cdm"),
            (SidereonEngineErrorFamily::Tdm, 7, "tdm"),
            (SidereonEngineErrorFamily::Fusion, 8, "fusion"),
            (
                SidereonEngineErrorFamily::FusionStateCodec,
                9,
                "fusion_state_codec",
            ),
            (SidereonEngineErrorFamily::Allan, 10, "allan"),
            (
                SidereonEngineErrorFamily::PowerLawNoise,
                11,
                "power_law_noise",
            ),
            (SidereonEngineErrorFamily::FrameCatalog, 12, "frame_catalog"),
            (SidereonEngineErrorFamily::Sidereal, 13, "sidereal"),
            (SidereonEngineErrorFamily::Atmosphere, 14, "atmosphere"),
            (
                SidereonEngineErrorFamily::SourceLocalization,
                15,
                "source_localization",
            ),
            (
                SidereonEngineErrorFamily::GeodeticTimeSeries,
                16,
                "geodetic_time_series",
            ),
            (SidereonEngineErrorFamily::Normality, 17, "normality"),
            (SidereonEngineErrorFamily::Track, 18, "track"),
            (
                SidereonEngineErrorFamily::PreciseSamples,
                19,
                "precise_samples",
            ),
            (
                SidereonEngineErrorFamily::PreciseInterpolant,
                20,
                "precise_interpolant",
            ),
            (SidereonEngineErrorFamily::SpaceWeather, 21, "space_weather"),
            (SidereonEngineErrorFamily::Araim, 22, "araim"),
            (SidereonEngineErrorFamily::ReducedOrbit, 23, "reduced_orbit"),
            (
                SidereonEngineErrorFamily::ReducedOrbitSource,
                24,
                "reduced_orbit_source",
            ),
            (
                SidereonEngineErrorFamily::PiecewiseOrbit,
                25,
                "piecewise_orbit",
            ),
            (SidereonEngineErrorFamily::OrbitFit, 26, "orbit_fit"),
            (SidereonEngineErrorFamily::Elements, 27, "elements"),
            (SidereonEngineErrorFamily::Equinoctial, 28, "equinoctial"),
            (SidereonEngineErrorFamily::RtnFrame, 29, "rtn_frame"),
            (SidereonEngineErrorFamily::Anomaly, 30, "anomaly"),
            (SidereonEngineErrorFamily::Propagation, 31, "propagation"),
            (SidereonEngineErrorFamily::Decay, 32, "decay"),
            (SidereonEngineErrorFamily::Dgnss, 33, "dgnss"),
            (SidereonEngineErrorFamily::Scenario, 34, "scenario"),
            (SidereonEngineErrorFamily::Catalog, 35, "catalog"),
            (SidereonEngineErrorFamily::ExactCache, 36, "exact_cache"),
            (SidereonEngineErrorFamily::Tca, 37, "tca"),
            (SidereonEngineErrorFamily::Almanac, 38, "almanac"),
            (SidereonEngineErrorFamily::Observe, 39, "observe"),
            (
                SidereonEngineErrorFamily::BodyObservation,
                40,
                "body_observation",
            ),
            (SidereonEngineErrorFamily::LookAngle, 41, "look_angle"),
            (SidereonEngineErrorFamily::Pass, 42, "pass"),
            (SidereonEngineErrorFamily::EventFinder, 43, "event_finder"),
            (
                SidereonEngineErrorFamily::FrameTransform,
                44,
                "frame_transform",
            ),
            (SidereonEngineErrorFamily::Conjunction, 45, "conjunction"),
            (SidereonEngineErrorFamily::Facade, 46, "facade"),
            (SidereonEngineErrorFamily::Spp, 47, "spp"),
            (SidereonEngineErrorFamily::SppPolicy, 48, "spp_policy"),
            (SidereonEngineErrorFamily::SunMoon, 49, "sun_moon"),
            (SidereonEngineErrorFamily::RinexSpp, 50, "rinex_spp"),
            (
                SidereonEngineErrorFamily::SolutionValidation,
                51,
                "solution_validation",
            ),
            (SidereonEngineErrorFamily::Rf, 52, "rf"),
            (
                SidereonEngineErrorFamily::IonosphereFree,
                53,
                "ionosphere_free",
            ),
            (SidereonEngineErrorFamily::Doppler, 54, "doppler"),
            (SidereonEngineErrorFamily::Oem, 55, "oem"),
            (SidereonEngineErrorFamily::Opm, 56, "opm"),
            (SidereonEngineErrorFamily::Omm, 57, "omm"),
            (SidereonEngineErrorFamily::Dop, 58, "dop"),
            (SidereonEngineErrorFamily::Geofence, 59, "geofence"),
            (SidereonEngineErrorFamily::Observables, 64, "observables"),
            (SidereonEngineErrorFamily::Nmea, 65, "nmea"),
            (SidereonEngineErrorFamily::TleFit, 66, "tle_fit"),
            (SidereonEngineErrorFamily::Iod, 67, "iod"),
            (SidereonEngineErrorFamily::Unknown, 999, "unknown"),
        ];

        for (family, id, name) in expected {
            assert_eq!(family as u32, id);
            assert_eq!(family.name(), name);
        }
    }

    #[test]
    fn iod_error_mapper_covers_all_core_variants() {
        use sidereon_core::astro::iod::IodError as E;

        let cases = [
            (E::DeterminantTooSmall, "determinant_too_small"),
            (E::OrbitNotPossible, "orbit_not_possible"),
            (E::ZeroVector, "zero_vector"),
            (E::CollinearVectors, "collinear_vectors"),
            (E::NotCoplanar, "not_coplanar"),
            (E::InvalidTimeGeometry, "invalid_time_geometry"),
            (E::NoPositiveRoot, "no_positive_root"),
            (E::RootSolveFailed, "root_solve_failed"),
            (E::NonFiniteValue, "non_finite_value"),
        ];
        for (error, kind) in cases {
            assert_eq!(iod_error_value(&error), json!({"kind":kind,"fields":{}}));
        }
    }

    #[test]
    fn facade_all_15_variants_serialize_with_exact_schema_and_fields() {
        use super::facade_error_value;

        // 1. Sp3
        let sp3_err = sidereon::Error::Sp3(sidereon_core::Error::Parse("bad header".into()));
        let v = facade_error_value(&sp3_err);
        assert_eq!(v["kind"], "sp3");
        assert_eq!(v["fields"]["cause"]["kind"], "parse");
        assert_eq!(v["fields"]["cause"]["fields"]["message"], "bad header");

        // 2. Antex
        let antex_err = sidereon::Error::Antex(sidereon_core::antex::AntexError::InvalidDateTime);
        let v = facade_error_value(&antex_err);
        assert_eq!(v["kind"], "antex");
        assert_eq!(v["fields"]["cause"]["kind"], "invalid_date_time");

        // 3. RinexNav
        let rinex_nav_err =
            sidereon::Error::RinexNav(sidereon_core::rinex::nav::NavParseError::MissingHeaderEnd);
        let v = facade_error_value(&rinex_nav_err);
        assert_eq!(v["kind"], "rinex_nav");
        assert_eq!(v["fields"]["cause"]["kind"], "missing_header_end");

        // 4. RinexObs
        let rinex_obs_err = sidereon::Error::RinexObs(sidereon_core::Error::EpochOutOfRange);
        let v = facade_error_value(&rinex_obs_err);
        assert_eq!(v["kind"], "rinex_obs");
        assert_eq!(v["fields"]["cause"]["kind"], "epoch_out_of_range");

        // 5. RinexClock
        let rinex_clock_err = sidereon::Error::RinexClock(
            sidereon_core::rinex::clock::RinexClockError::InvalidInput {
                field: "scale",
                reason: "unsupported",
            },
        );
        let v = facade_error_value(&rinex_clock_err);
        assert_eq!(v["kind"], "rinex_clock");
        assert_eq!(v["fields"]["cause"]["kind"], "invalid_input");
        assert_eq!(v["fields"]["cause"]["fields"]["field"], "scale");
        assert_eq!(v["fields"]["cause"]["fields"]["reason"], "unsupported");

        // 6. Bias
        let bias_err = sidereon::Error::Bias(sidereon_core::bias::BiasError::Departure {
            departure: sidereon_core::bias::BiasDeparture::HeaderModeMismatch {
                header: "REL".into(),
                description: sidereon_core::bias::BiasMode::Absolute,
            },
        });
        let v = facade_error_value(&bias_err);
        assert_eq!(v["kind"], "bias");
        assert_eq!(v["fields"]["cause"]["kind"], "departure");
        assert_eq!(
            v["fields"]["cause"]["fields"]["departure"]["kind"],
            "header_mode_mismatch"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["departure"]["fields"]["header"],
            "REL"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["departure"]["fields"]["description"],
            "Absolute"
        );

        // 7. Ssr
        let ssr_err = sidereon::Error::Ssr(sidereon_core::Error::Parse("missing ephemeris".into()));
        let v = facade_error_value(&ssr_err);
        assert_eq!(v["kind"], "ssr");
        assert_eq!(v["fields"]["cause"]["kind"], "parse");
        assert_eq!(
            v["fields"]["cause"]["fields"]["message"],
            "missing ephemeris"
        );

        // 8. Crinex
        let crinex_err =
            sidereon::Error::Crinex(sidereon_core::Error::Parse("invalid civil date".into()));
        let v = facade_error_value(&crinex_err);
        assert_eq!(v["kind"], "crinex");
        assert_eq!(v["fields"]["cause"]["kind"], "parse");
        assert_eq!(
            v["fields"]["cause"]["fields"]["message"],
            "invalid civil date"
        );

        // 9. Io
        let io_err = sidereon::Error::Io(std::io::Error::from_raw_os_error(2));
        let v = facade_error_value(&io_err);
        assert_eq!(v["kind"], "io");
        assert_eq!(v["fields"]["kind"], "NotFound");
        assert_eq!(v["fields"]["raw_os_error"], 2);
        assert!(!v["fields"]["message"].as_str().unwrap().is_empty());

        // 10. Spp
        let spp_err = sidereon::Error::Spp(sidereon_core::positioning::SolvePolicyError::Solve(
            sidereon_core::positioning::SppError::TooFewSatellites {
                used: 2,
                required: 4,
            },
        ));
        let v = facade_error_value(&spp_err);
        assert_eq!(v["kind"], "spp");
        assert_eq!(v["fields"]["cause"]["kind"], "solve");
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["kind"],
            "too_few_satellites"
        );
        assert_eq!(v["fields"]["cause"]["fields"]["cause"]["fields"]["used"], 2);
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["fields"]["required"],
            4
        );

        // 11. Velocity
        let vel_err =
            sidereon::Error::Velocity(sidereon_core::velocity::VelocityError::TooFewSatellites {
                used: 2,
                required: 4,
            });
        let v = facade_error_value(&vel_err);
        assert_eq!(v["kind"], "velocity");
        assert_eq!(v["fields"]["cause"]["kind"], "too_few_satellites");
        assert_eq!(v["fields"]["cause"]["fields"]["used"], 2);
        assert_eq!(v["fields"]["cause"]["fields"]["required"], 4);

        // 12. RtkFloat
        let rtk_float_err =
            sidereon::Error::RtkFloat(sidereon_core::rtk_filter::FloatSolveError::SingularGeometry);
        let v = facade_error_value(&rtk_float_err);
        assert_eq!(v["kind"], "rtk_float");
        assert_eq!(v["fields"]["cause"]["kind"], "singular_geometry");

        // 13. RtkFixed
        let rtk_fixed_err =
            sidereon::Error::RtkFixed(sidereon_core::rtk_filter::ValidatedFixedSolveError::Fixed(
                sidereon_core::rtk_filter::FixedSolveError::Float(
                    sidereon_core::rtk_filter::FloatSolveError::SingularGeometry,
                ),
            ));
        let v = facade_error_value(&rtk_fixed_err);
        assert_eq!(v["kind"], "rtk_fixed");
        assert_eq!(v["fields"]["cause"]["kind"], "fixed");
        assert_eq!(v["fields"]["cause"]["fields"]["cause"]["kind"], "float");
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["fields"]["cause"]["kind"],
            "singular_geometry"
        );

        // 14. PppFloat (lossless NaN and -0.0 test)
        let nan_val = f64::from_bits(0x7ff8_0000_0000_1234);
        let ppp_float_err = sidereon::Error::PppFloat(
            sidereon_core::precise_positioning::FloatSolveError::InsufficientObservationsAfterElevationCutoff {
                cutoff_deg: nan_val,
                retained_observations: 3,
                required_observations: 4,
            },
        );
        let v = facade_error_value(&ppp_float_err);
        assert_eq!(v["kind"], "ppp_float");
        assert_eq!(
            v["fields"]["cause"]["kind"],
            "insufficient_observations_after_elevation_cutoff"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["cutoff_deg"]["decimal"],
            "NaN"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["cutoff_deg"]["bits_hex"],
            "7ff8000000001234"
        );
        assert_eq!(v["fields"]["cause"]["fields"]["retained_observations"], 3);
        assert_eq!(v["fields"]["cause"]["fields"]["required_observations"], 4);

        let ppp_float_neg0 = sidereon::Error::PppFloat(
            sidereon_core::precise_positioning::FloatSolveError::InsufficientObservationsAfterElevationCutoff {
                cutoff_deg: -0.0,
                retained_observations: 3,
                required_observations: 4,
            },
        );
        let v_neg0 = facade_error_value(&ppp_float_neg0);
        assert_eq!(v_neg0["kind"], "ppp_float");
        assert_eq!(
            v_neg0["fields"]["cause"]["kind"],
            "insufficient_observations_after_elevation_cutoff"
        );
        assert_eq!(
            v_neg0["fields"]["cause"]["fields"]["cutoff_deg"]["decimal"],
            "-0"
        );
        assert_eq!(
            v_neg0["fields"]["cause"]["fields"]["cutoff_deg"]["bits_hex"],
            "8000000000000000"
        );
        assert_eq!(
            v_neg0["fields"]["cause"]["fields"]["retained_observations"],
            3
        );
        assert_eq!(
            v_neg0["fields"]["cause"]["fields"]["required_observations"],
            4
        );

        // 15. PppFixed
        let ppp_fixed_err =
            sidereon::Error::PppFixed(sidereon_core::precise_positioning::FixedSolveError::Float(
                sidereon_core::precise_positioning::FloatSolveError::InvalidInput {
                    field: "epochs",
                    reason: "empty",
                },
            ));
        let v = facade_error_value(&ppp_fixed_err);
        assert_eq!(v["kind"], "ppp_fixed");
        assert_eq!(v["fields"]["cause"]["kind"], "float");
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["kind"],
            "invalid_input"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["fields"]["field"],
            "epochs"
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["fields"]["reason"],
            "empty"
        );
    }

    #[test]
    fn record_engine_error_trait_records_correct_family_and_payload() {
        use super::RecordEngineError;

        // 1. Facade with lossless f64 (-0.0)
        clear_engine_error();
        let err = sidereon::Error::PppFloat(
            sidereon_core::precise_positioning::FloatSolveError::InsufficientObservationsAfterElevationCutoff {
                cutoff_deg: -0.0,
                retained_observations: 3,
                required_observations: 4,
            },
        );
        sidereon::Error::record_engine_error("test_op", &err);
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

        let mut buf = vec![0u8; info.payload_len];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, info.payload_len);
        let payload: Value = serde_json::from_slice(&buf).expect("valid JSON payload");
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["family"], "facade");
        assert_eq!(payload["operation"], "test_op");
        assert_eq!(payload["error"]["kind"], "ppp_float");
        assert_eq!(
            payload["error"]["fields"]["cause"]["kind"],
            "insufficient_observations_after_elevation_cutoff"
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["cutoff_deg"]["decimal"],
            "-0"
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["cutoff_deg"]["bits_hex"],
            "8000000000000000"
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["retained_observations"],
            3
        );
        assert_eq!(
            payload["error"]["fields"]["cause"]["fields"]["required_observations"],
            4
        );

        // 2. Spp with least_squares SolveError
        clear_engine_error();
        let spp_err = sidereon_core::positioning::SppError::Singular(
            sidereon_core::astro::math::least_squares::SolveError::SingularJacobian,
        );
        sidereon_core::positioning::SppError::record_engine_error("spp_op", &spp_err);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::Spp);
        assert!(info.payload_len > 0);

        let mut spp_buf = vec![0u8; info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    spp_buf.as_mut_ptr(),
                    spp_buf.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let spp_payload: Value = serde_json::from_slice(&spp_buf).expect("valid JSON payload");
        assert_eq!(spp_payload["schema_version"], 1);
        assert_eq!(spp_payload["family"], "spp");
        assert_eq!(spp_payload["operation"], "spp_op");
        assert_eq!(spp_payload["error"]["kind"], "singular");
        assert_eq!(
            spp_payload["error"]["fields"]["cause"]["kind"],
            "singular_jacobian"
        );

        // 3. SppPolicy
        clear_engine_error();
        let policy_err = sidereon_core::positioning::SolvePolicyError::NoCoarseSolution;
        sidereon_core::positioning::SolvePolicyError::record_engine_error("policy_op", &policy_err);
        assert_eq!(
            unsafe { sidereon_last_engine_error_info(&mut info) },
            SidereonStatus::Ok
        );
        assert_eq!(info.family, SidereonEngineErrorFamily::SppPolicy);
        assert!(info.payload_len > 0);

        let mut policy_buf = vec![0u8; info.payload_len];
        assert_eq!(
            unsafe {
                sidereon_last_engine_error_payload(
                    policy_buf.as_mut_ptr(),
                    policy_buf.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let policy_payload: Value =
            serde_json::from_slice(&policy_buf).expect("valid JSON payload");
        assert_eq!(policy_payload["schema_version"], 1);
        assert_eq!(policy_payload["family"], "spp_policy");
        assert_eq!(policy_payload["operation"], "policy_op");
        assert_eq!(policy_payload["error"]["kind"], "no_coarse_solution");

        clear_engine_error();
    }

    #[test]
    fn test_nav_parse_error_all_variants_mapped() {
        use super::nav_parse_error_value;
        use sidereon_core::rinex::nav::NavParseError as E;

        let cases: Vec<(E, &'static str, Value)> = vec![
            (
                E::UnsupportedHeader("unsupported header text".to_string()),
                "unsupported_header",
                json!({"message": "unsupported header text"}),
            ),
            (E::MissingHeaderEnd, "missing_header_end", json!({})),
            (
                E::TruncatedRecord("G01".to_string()),
                "truncated_record",
                json!({"satellite": "G01"}),
            ),
            (
                E::BadField {
                    satellite: "G02".to_string(),
                    field: "eccentricity",
                },
                "bad_field",
                json!({"satellite": "G02", "field": "eccentricity"}),
            ),
            (
                E::BadHeaderField {
                    field: "leap_seconds",
                },
                "bad_header_field",
                json!({"field": "leap_seconds"}),
            ),
            (
                E::UnexpectedLine { line: 42 },
                "unexpected_line",
                json!({"line": 42}),
            ),
            (
                E::ExtraRecordLines {
                    satellite: "E05".to_string(),
                },
                "extra_record_lines",
                json!({"satellite": "E05"}),
            ),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let v = nav_parse_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }

    #[test]
    fn test_rinex_clock_error_all_variants_mapped() {
        use super::rinex_clock_error_value;
        use sidereon_core::astro::time::TimeScale;
        use sidereon_core::rinex::clock::RinexClockError as E;

        let cases: Vec<(E, &'static str, Value)> = vec![
            (
                E::MalformedAsRecord {
                    line: 10,
                    reason: "too short",
                    record: "AS G01 ...".to_string(),
                },
                "malformed_as_record",
                json!({"line": 10, "reason": "too short", "record": "AS G01 ..."}),
            ),
            (
                E::MissingContinuation {
                    line: 20,
                    record_type: "CR".to_string(),
                },
                "missing_continuation",
                json!({"line": 20, "record_type": "CR"}),
            ),
            (
                E::MalformedContinuation {
                    line: 30,
                    reason: "bad continuation line",
                    record: "continuation text".to_string(),
                },
                "malformed_continuation",
                json!({"line": 30, "reason": "bad continuation line", "record": "continuation text"}),
            ),
            (
                E::BadField {
                    line: 40,
                    field: "bias",
                    value: "NaN_string".to_string(),
                },
                "bad_field",
                json!({"line": 40, "field": "bias", "value": "NaN_string"}),
            ),
            (
                E::InvalidInput {
                    field: "epoch",
                    reason: "negative",
                },
                "invalid_input",
                json!({"field": "epoch", "reason": "negative"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Utc,
                },
                "unsupported_time_scale",
                json!({"scale": "UTC"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Gpst,
                },
                "unsupported_time_scale",
                json!({"scale": "GPST"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Gst,
                },
                "unsupported_time_scale",
                json!({"scale": "GST"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Bdt,
                },
                "unsupported_time_scale",
                json!({"scale": "BDT"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Glonasst,
                },
                "unsupported_time_scale",
                json!({"scale": "GLONASST"}),
            ),
            (
                E::UnsupportedTimeScale {
                    scale: TimeScale::Qzsst,
                },
                "unsupported_time_scale",
                json!({"scale": "QZSST"}),
            ),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let v = rinex_clock_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }

    #[test]
    fn test_bias_departure_all_19_variants_mapped() {
        use super::bias_departure_value;
        use sidereon_core::bias::{BiasDeparture as D, BiasMode};

        let cases: Vec<(D, &'static str, Value)> = vec![
            (
                D::HeaderLayout {
                    reason: "short layout",
                },
                "header_layout",
                json!({"reason": "short layout"}),
            ),
            (
                D::OtherVersion {
                    version: "2.0".to_string(),
                },
                "other_version",
                json!({"version": "2.0"}),
            ),
            (D::MissingFooter, "missing_footer", json!({})),
            (
                D::ContentAfterFooter { line: 100 },
                "content_after_footer",
                json!({"line": 100}),
            ),
            (
                D::UnexpectedControlLine { line: 101 },
                "unexpected_control_line",
                json!({"line": 101}),
            ),
            (
                D::UnclosedBlock {
                    name: "BLOCK".to_string(),
                    line: 102,
                },
                "unclosed_block",
                json!({"name": "BLOCK", "line": 102}),
            ),
            (
                D::UnopenedBlockEnd {
                    name: "END".to_string(),
                    line: 103,
                },
                "unopened_block_end",
                json!({"name": "END", "line": 103}),
            ),
            (
                D::MismatchedBlockEnd {
                    open: "OPEN_B".to_string(),
                    close: "CLOSE_B".to_string(),
                    line: 104,
                },
                "mismatched_block_end",
                json!({"open": "OPEN_B", "close": "CLOSE_B", "line": 104}),
            ),
            (
                D::NestedBlock {
                    open: "PARENT".to_string(),
                    inner: "CHILD".to_string(),
                    line: 105,
                },
                "nested_block",
                json!({"open": "PARENT", "inner": "CHILD", "line": 105}),
            ),
            (
                D::MissingBlock {
                    name: "MANDATORY_BLOCK",
                },
                "missing_block",
                json!({"name": "MANDATORY_BLOCK"}),
            ),
            (
                D::UnknownBlock {
                    name: "MY_BLOCK".to_string(),
                    line: 106,
                },
                "unknown_block",
                json!({"name": "MY_BLOCK", "line": 106}),
            ),
            (
                D::BlockStartSuffix { line: 107 },
                "block_start_suffix",
                json!({"line": 107}),
            ),
            (
                D::DataOutsideBlock { line: 108 },
                "data_outside_block",
                json!({"line": 108}),
            ),
            (
                D::MissingDeclaration {
                    keyword: "BIAS_MODE",
                },
                "missing_declaration",
                json!({"keyword": "BIAS_MODE"}),
            ),
            (
                D::UnsupportedBiasMode {
                    line: 109,
                    label: "UNKNOWN_MODE".to_string(),
                },
                "unsupported_bias_mode",
                json!({"line": 109, "label": "UNKNOWN_MODE"}),
            ),
            (
                D::NonStandardTimeSystem {
                    line: 110,
                    label: "CUSTOM_TIME".to_string(),
                },
                "non_standard_time_system",
                json!({"line": 110, "label": "CUSTOM_TIME"}),
            ),
            (
                D::HeaderModeMismatch {
                    header: "ABSOLUTE".to_string(),
                    description: BiasMode::Relative,
                },
                "header_mode_mismatch",
                json!({"header": "ABSOLUTE", "description": "Relative"}),
            ),
            (
                D::HeaderModeMismatch {
                    header: "RELATIVE".to_string(),
                    description: BiasMode::Absolute,
                },
                "header_mode_mismatch",
                json!({"header": "RELATIVE", "description": "Absolute"}),
            ),
            (
                D::HeaderModeMismatch {
                    header: "OTHER".to_string(),
                    description: BiasMode::Unspecified,
                },
                "header_mode_mismatch",
                json!({"header": "OTHER", "description": "Unspecified"}),
            ),
            (
                D::UnknownDcbTimeSystem {
                    line: 111,
                    label: "BAD_DCB_SCALE".to_string(),
                },
                "unknown_dcb_time_system",
                json!({"line": 111, "label": "BAD_DCB_SCALE"}),
            ),
            (
                D::EstimateCountMismatch {
                    declared: 100,
                    solution_rows: 95,
                },
                "estimate_count_mismatch",
                json!({"declared": 100, "solution_rows": 95}),
            ),
        ];

        for (dep, expected_kind, expected_fields) in cases {
            let v = bias_departure_value(&dep);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }

    #[test]
    fn test_bias_error_all_12_variants_mapped() {
        use super::bias_error_value;
        use sidereon_core::astro::time::TimeScale;
        use sidereon_core::bias::{BiasDeparture, BiasError as E};

        let cases: Vec<(E, &'static str, Value)> = vec![
            (
                E::InvalidInput {
                    field: "month",
                    reason: "out of range",
                },
                "invalid_input",
                json!({"field": "month", "reason": "out of range"}),
            ),
            (E::InvalidEpoch, "invalid_epoch", json!({})),
            (
                E::UnknownObservable {
                    code: "C9X".to_string(),
                },
                "unknown_observable",
                json!({"code": "C9X"}),
            ),
            (
                E::UnsupportedVersion {
                    version: "2.0".to_string(),
                },
                "unsupported_version",
                json!({"version": "2.0"}),
            ),
            (E::MissingDcbMetadata, "missing_dcb_metadata", json!({})),
            (
                E::MissingClockReference,
                "missing_clock_reference",
                json!({}),
            ),
            (
                E::MissingWriterMetadata { field: "agency" },
                "missing_writer_metadata",
                json!({"field": "agency"}),
            ),
            (E::Utf8, "utf8", json!({})),
            (
                E::Departure {
                    departure: BiasDeparture::MissingFooter,
                },
                "departure",
                json!({"departure": {"kind": "missing_footer", "fields": {}}}),
            ),
            (
                E::InvalidUtf8Line { line: 15 },
                "invalid_utf8_line",
                json!({"line": 15}),
            ),
            (
                E::UnsupportedTimeSystem { scale: None },
                "unsupported_time_system",
                json!({"scale": Value::Null}),
            ),
            (
                E::UnsupportedTimeSystem {
                    scale: Some(TimeScale::Gpst),
                },
                "unsupported_time_system",
                json!({"scale": "GPST"}),
            ),
            (
                E::UnsupportedTimeSystem {
                    scale: Some(TimeScale::Bdt),
                },
                "unsupported_time_system",
                json!({"scale": "BDT"}),
            ),
            (
                E::DcbRecordMismatch {
                    record: 7,
                    field: "prn",
                },
                "dcb_record_mismatch",
                json!({"record": 7, "field": "prn"}),
            ),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let v = bias_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }

    #[test]
    fn test_solve_policy_and_solution_validation_all_variants_mapped() {
        use super::{solution_validation_error_value, solve_policy_error_value};
        use sidereon_core::astro::math::least_squares::SolveError;
        use sidereon_core::positioning::{SolvePolicyError, SppError};
        use sidereon_core::quality::SolutionValidationError as V;

        // SolutionValidationError: all 6 variants
        let val_cases: Vec<(V, &'static str, Value)> = vec![
            (
                V::InvalidOptions {
                    field: "pdop",
                    reason: "too high",
                },
                "invalid_options",
                json!({"field": "pdop", "reason": "too high"}),
            ),
            (
                V::DegenerateGeometryRankDeficient,
                "degenerate_geometry_rank_deficient",
                json!({}),
            ),
            (
                V::DegenerateGeometryPdop(5.5),
                "degenerate_geometry_pdop",
                json!({
                    "pdop": {
                        "decimal": "5.5",
                        "bits_hex": "4016000000000000"
                    }
                }),
            ),
            (
                V::ImplausiblePosition(100.0),
                "implausible_position",
                json!({
                    "radius_m": {
                        "decimal": "100",
                        "bits_hex": "4059000000000000"
                    }
                }),
            ),
            (V::InvalidResiduals, "invalid_residuals", json!({})),
            (
                V::NoConvergence(0.0),
                "no_convergence",
                json!({
                    "rms_m": {
                        "decimal": "0",
                        "bits_hex": "0000000000000000"
                    }
                }),
            ),
        ];

        for (err, expected_kind, expected_fields) in val_cases {
            let v = solution_validation_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }

        // SolvePolicyError: all 3 variants
        let policy_solve =
            SolvePolicyError::Solve(SppError::Singular(SolveError::SingularJacobian));
        let v = solve_policy_error_value(&policy_solve);
        assert_eq!(v["kind"], "solve");
        assert_eq!(v["fields"]["cause"]["kind"], "singular");
        assert_eq!(
            v["fields"]["cause"]["fields"]["cause"]["kind"],
            "singular_jacobian"
        );

        let policy_val = SolvePolicyError::Validation(V::DegenerateGeometryRankDeficient);
        let v = solve_policy_error_value(&policy_val);
        assert_eq!(v["kind"], "validation");
        assert_eq!(
            v["fields"]["cause"]["kind"],
            "degenerate_geometry_rank_deficient"
        );

        let policy_coarse = SolvePolicyError::NoCoarseSolution;
        let v = solve_policy_error_value(&policy_coarse);
        assert_eq!(v["kind"], "no_coarse_solution");
        assert_eq!(v["fields"], json!({}));
    }

    #[test]
    fn rinex_spp_error_variants_map_with_complete_nested_cause() {
        use super::rinex_spp_error_value;
        use sidereon_core::positioning::RinexSppError as E;

        let missing = rinex_spp_error_value(&E::MissingApproxPosition);
        assert_eq!(missing["kind"], "missing_approx_position");
        assert_eq!(missing["fields"], json!({}));

        let cause = sidereon_core::Error::InvalidInput("bad epoch value".into());
        let observation = rinex_spp_error_value(&E::Observation(cause));
        assert_eq!(observation["kind"], "observation");
        assert_eq!(
            observation["fields"]["cause"],
            json!({"kind":"invalid_input", "fields":{"message":"bad epoch value"}})
        );
    }

    #[test]
    fn test_velocity_error_all_8_variants_mapped() {
        use super::velocity_error_value;
        use sidereon_core::velocity::VelocityError as E;
        use sidereon_core::{GnssSatelliteId, GnssSystem};

        let sat1 = GnssSatelliteId::new(GnssSystem::Gps, 1).expect("sat 1");
        let sat2 = GnssSatelliteId::new(GnssSystem::Gps, 2).expect("sat 2");
        let sat3 = GnssSatelliteId::new(GnssSystem::Galileo, 5).expect("sat 3");

        let cases: Vec<(E, &'static str, Value)> = vec![
            (E::NoObservations, "no_observations", json!({})),
            (
                E::TooFewSatellites {
                    used: 3,
                    required: 4,
                },
                "too_few_satellites",
                json!({"used": 3, "required": 4}),
            ),
            (E::SingularGeometry, "singular_geometry", json!({})),
            (
                E::DuplicateObservation { satellite_id: sat1 },
                "duplicate_observation",
                json!({"satellite_id": "G01"}),
            ),
            (
                E::InvalidCarrier { satellite_id: sat2 },
                "invalid_carrier",
                json!({"satellite_id": "G02"}),
            ),
            (
                E::InvalidInput {
                    field: "doppler_hz",
                    reason: "not finite",
                },
                "invalid_input",
                json!({"field": "doppler_hz", "reason": "not finite"}),
            ),
            (
                E::InvalidObservation { satellite_id: sat3 },
                "invalid_observation",
                json!({"satellite_id": "E05"}),
            ),
            (E::InvalidReceiverState, "invalid_receiver_state", json!({})),
        ];

        for (err, expected_kind, expected_fields) in cases {
            let v = velocity_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }

    #[test]
    fn test_ppp_nested_mappers_all_variants() {
        use super::{
            ppp_fixed_solve_error_value, ppp_float_solve_error_value, ppp_missing_correction_value,
            ppp_no_ephemeris_reason_value,
        };
        use sidereon_core::astro::time::DegradeReason;
        use sidereon_core::ils::IlsError;
        use sidereon_core::precise_positioning::{
            FixedSolveError, FloatSolveError, MissingCorrection, NoEphemerisReason,
        };

        // 1. NoEphemerisReason: all 3 variants
        let r_cases: Vec<(NoEphemerisReason, &'static str, Value)> = vec![
            (NoEphemerisReason::NoEphemeris, "no_ephemeris", json!({})),
            (
                NoEphemerisReason::MissingSatelliteClock,
                "missing_satellite_clock",
                json!({}),
            ),
            (
                NoEphemerisReason::Reason("unhealthy satellite".to_string()),
                "reason",
                json!({"message": "unhealthy satellite"}),
            ),
        ];
        for (reason, expected_kind, expected_fields) in r_cases {
            let v = ppp_no_ephemeris_reason_value(&reason);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }

        // 2. MissingCorrection: all 12 variants
        let c_cases: Vec<(MissingCorrection, &'static str, Value)> = vec![
            (
                MissingCorrection::SolidEarthTide,
                "solid_earth_tide",
                json!({}),
            ),
            (MissingCorrection::PoleTide, "pole_tide", json!({})),
            (MissingCorrection::OceanLoading, "ocean_loading", json!({})),
            (MissingCorrection::PhaseWindup, "phase_windup", json!({})),
            (
                MissingCorrection::SatelliteAntennaPco,
                "satellite_antenna_pco",
                json!({}),
            ),
            (
                MissingCorrection::SatelliteAntennaPcv,
                "satellite_antenna_pcv",
                json!({}),
            ),
            (MissingCorrection::CodeBias, "code_bias", json!({})),
            (MissingCorrection::SsrCodeBias, "ssr_code_bias", json!({})),
            (MissingCorrection::PhaseBias, "phase_bias", json!({})),
            (
                MissingCorrection::ReceiverAntennaFrequency("L1_FREQ".to_string()),
                "receiver_antenna_frequency",
                json!({"label": "L1_FREQ"}),
            ),
            (
                MissingCorrection::ReceiverAntennaPcv("L1_PCV".to_string()),
                "receiver_antenna_pcv",
                json!({"label": "L1_PCV"}),
            ),
            (
                MissingCorrection::ReceiverAntennaGeometry,
                "receiver_antenna_geometry",
                json!({}),
            ),
        ];
        for (corr, expected_kind, expected_fields) in c_cases {
            let v = ppp_missing_correction_value(&corr);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }

        // 3. FloatSolveError: all 10 variants
        let f_cases: Vec<(FloatSolveError, &'static str, Value)> = vec![
            (
                FloatSolveError::NoEphemeris {
                    satellite_id: "E12".to_string(),
                    reason: NoEphemerisReason::Reason("satellite clock unhealthy".to_string()),
                },
                "no_ephemeris",
                json!({
                    "satellite_id": "E12",
                    "reason": {
                        "kind": "reason",
                        "fields": { "message": "satellite clock unhealthy" }
                    }
                }),
            ),
            (
                FloatSolveError::SingularGeometry,
                "singular_geometry",
                json!({}),
            ),
            (
                FloatSolveError::InvalidClockCount {
                    expected: 7,
                    actual: 3,
                },
                "invalid_clock_count",
                json!({ "expected": 7, "actual": 3 }),
            ),
            (
                FloatSolveError::InvalidSolveOption {
                    field: "phase_tolerance_m",
                    reason: "must be positive",
                },
                "invalid_solve_option",
                json!({ "field": "phase_tolerance_m", "reason": "must be positive" }),
            ),
            (
                FloatSolveError::InvalidInput {
                    field: "float_position_m",
                    reason: "not finite",
                },
                "invalid_input",
                json!({ "field": "float_position_m", "reason": "not finite" }),
            ),
            (
                FloatSolveError::InsufficientObservationsAfterElevationCutoff {
                    cutoff_deg: -0.0,
                    retained_observations: 7,
                    required_observations: 11,
                },
                "insufficient_observations_after_elevation_cutoff",
                json!({
                    "cutoff_deg": { "decimal": "-0", "bits_hex": "8000000000000000" },
                    "retained_observations": 7,
                    "required_observations": 11
                }),
            ),
            (
                FloatSolveError::InsufficientObservationsAfterSsrBiasExclusion {
                    excluded_observations: 5,
                    retained_observations: 9,
                    required_observations: 12,
                },
                "insufficient_observations_after_ssr_bias_exclusion",
                json!({
                    "excluded_observations": 5,
                    "retained_observations": 9,
                    "required_observations": 12
                }),
            ),
            (
                FloatSolveError::MissingAmbiguity("G07-L1C".to_string()),
                "missing_ambiguity",
                json!({ "ambiguity_id": "G07-L1C" }),
            ),
            (
                FloatSolveError::MissingCorrection {
                    satellite_id: "E12".to_string(),
                    correction: MissingCorrection::ReceiverAntennaPcv("L2_PCV".to_string()),
                },
                "missing_correction",
                json!({
                    "satellite_id": "E12",
                    "correction": {
                        "kind": "receiver_antenna_pcv",
                        "fields": { "label": "L2_PCV" }
                    }
                }),
            ),
            (
                FloatSolveError::Ut1OutsideCoverage(DegradeReason::AfterCoverage),
                "ut1_outside_coverage",
                json!({ "reason": "after_coverage" }),
            ),
        ];
        for (err, expected_kind, expected_fields) in f_cases {
            let v = ppp_float_solve_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }

        // 4. FixedSolveError: all 5 variants
        let fix_cases: Vec<(FixedSolveError, &'static str, Value)> = vec![
            (
                FixedSolveError::Float(FloatSolveError::InvalidInput {
                    field: "fixed_seed_position_m",
                    reason: "not finite",
                }),
                "float",
                json!({
                    "cause": {
                        "kind": "invalid_input",
                        "fields": {
                            "field": "fixed_seed_position_m",
                            "reason": "not finite"
                        }
                    }
                }),
            ),
            (
                FixedSolveError::Integer(IlsError::InvalidInput {
                    field: "ambiguity_covariance",
                    reason: "not symmetric",
                }),
                "integer",
                json!({
                    "cause": {
                        "kind": "invalid_input",
                        "fields": {
                            "field": "ambiguity_covariance",
                            "reason": "not symmetric"
                        }
                    }
                }),
            ),
            (
                FixedSolveError::MissingWavelength("E11-L1C".to_string()),
                "missing_wavelength",
                json!({ "ambiguity_id": "E11-L1C" }),
            ),
            (
                FixedSolveError::MissingOffset("G12-L2W".to_string()),
                "missing_offset",
                json!({ "ambiguity_id": "G12-L2W" }),
            ),
            (
                FixedSolveError::MissingFixedAmbiguity("R20-L2C".to_string()),
                "missing_fixed_ambiguity",
                json!({ "ambiguity_id": "R20-L2C" }),
            ),
        ];
        for (err, expected_kind, expected_fields) in fix_cases {
            let v = ppp_fixed_solve_error_value(&err);
            assert_eq!(v["kind"], expected_kind);
            assert_eq!(v["fields"], expected_fields);
        }
    }
}

#[cfg(test)]
mod dop_geofence_mapper_tests {
    use super::*;

    #[test]
    fn dop_mapper_covers_each_core_variant() {
        use sidereon_core::dop::DopError as E;
        assert_eq!(
            dop_error_value(&E::InvalidInput {
                field: "los",
                reason: "not unit length",
            }),
            json!({"kind":"invalid_input","fields":{"field":"los","reason":"not unit length"}})
        );
        assert_eq!(
            dop_error_value(&E::TooFewSatellites),
            json!({"kind":"too_few_satellites","fields":{}})
        );
        assert_eq!(
            dop_error_value(&E::Singular),
            json!({"kind":"singular","fields":{}})
        );
    }

    #[test]
    fn geofence_mapper_recursively_covers_all_core_variants() {
        use sidereon_core::dop::DopError as D;
        use sidereon_core::error_metrics::ErrorMetricsError as M;
        use sidereon_core::geodesic::GeodesicError as G;
        use sidereon_core::geofence::GeofenceError as E;

        assert_eq!(
            geofence_error_value(&E::TooFewVertices),
            json!({"kind":"too_few_vertices","fields":{}})
        );
        assert_eq!(
            geofence_error_value(&E::InvalidInput {
                field: "vertices",
                reason: "duplicate"
            }),
            json!({"kind":"invalid_input","fields":{"field":"vertices","reason":"duplicate"}})
        );
        assert_eq!(
            geofence_error_value(&E::Geodesic(G::InvalidInput {
                field: "latitude",
                reason: "out of range"
            })),
            json!({"kind":"geodesic","fields":{"cause":{"kind":"invalid_input","fields":{"field":"latitude","reason":"out of range"}}}})
        );
        assert_eq!(
            geofence_error_value(&E::Dop(D::Singular)),
            json!({"kind":"dop","fields":{"cause":{"kind":"singular","fields":{}}}})
        );
        assert_eq!(
            geofence_error_value(&E::ErrorMetrics(M::NonFinite)),
            json!({"kind":"error_metrics","fields":{"cause":{"kind":"non_finite","fields":{}}}})
        );
        assert_eq!(
            geofence_error_value(&E::ErrorMetrics(M::NotPositiveSemidefinite)),
            json!({"kind":"error_metrics","fields":{"cause":{"kind":"not_positive_semidefinite","fields":{}}}})
        );
        assert_eq!(
            geofence_error_value(&E::ErrorMetrics(M::InvalidProbability)),
            json!({"kind":"error_metrics","fields":{"cause":{"kind":"invalid_probability","fields":{}}}})
        );
        assert_eq!(
            geofence_error_value(&E::ErrorMetrics(M::Rotation(D::InvalidInput {
                field: "receiver",
                reason: "not finite"
            }))),
            json!({"kind":"error_metrics","fields":{"cause":{"kind":"rotation","fields":{"cause":{"kind":"invalid_input","fields":{"field":"receiver","reason":"not finite"}}}}}})
        );
    }
}

#[cfg(test)]
mod signal_error_mapper_tests {
    use super::*;
    use sidereon_core::carrier_phase::CarrierPhaseError as C;
    use sidereon_core::signal::analysis::SignalAnalysisError as A;
    use sidereon_core::signal::SignalError as S;

    #[test]
    fn new_signal_error_mappers_cover_every_variant_and_exact_float_bits() {
        assert_eq!(SidereonEngineErrorFamily::Signal as u32, 60);
        assert_eq!(SidereonEngineErrorFamily::CarrierPhase as u32, 61);
        assert_eq!(SidereonEngineErrorFamily::SignalAnalysis as u32, 62);
        assert_eq!(
            signal_error_value(&S::UnsupportedPrn(-7)),
            rtk_node("unsupported_prn", json!({"prn":-7}))
        );
        assert_eq!(
            signal_error_value(&S::InvalidInput {
                field: "sample_rate_hz",
                reason: "out of range"
            }),
            rtk_node(
                "invalid_input",
                json!({"field":"sample_rate_hz","reason":"out of range"})
            )
        );
        assert_eq!(
            signal_error_value(&S::EmptySamples),
            rtk_node("empty_samples", json!({}))
        );
        assert_eq!(
            signal_error_value(&S::TooShort),
            rtk_node("too_short", json!({}))
        );
        assert_eq!(
            carrier_phase_error_value(C::EqualFrequencies),
            rtk_node("equal_frequencies", json!({}))
        );
        assert_eq!(
            carrier_phase_error_value(C::InvalidFrequency),
            rtk_node("invalid_frequency", json!({}))
        );
        assert_eq!(
            carrier_phase_error_value(C::InvalidObservation),
            rtk_node("invalid_observation", json!({}))
        );
        assert_eq!(
            carrier_phase_error_value(C::InvalidThreshold),
            rtk_node("invalid_threshold", json!({}))
        );
        assert_eq!(
            signal_analysis_error_value(&A::InvalidInput {
                field: "offset_hz",
                reason: "not finite"
            }),
            rtk_node(
                "invalid_input",
                json!({"field":"offset_hz","reason":"not finite"})
            )
        );
        assert_eq!(
            signal_analysis_error_value(&A::EmptyComponents),
            rtk_node("empty_components", json!({}))
        );
        let delay = f64::from_bits(0x3fd0_0000_0000_0001);
        let phase = f64::from_bits(0xbff0_0000_0000_0000);
        assert_eq!(
            signal_analysis_error_value(&A::NoDiscriminatorRoot {
                delay_chips: delay,
                phase_sign: phase
            }),
            rtk_node(
                "no_discriminator_root",
                json!({
                    "delay_chips":{"decimal":"0.25000000000000006","bits_hex":"3fd0000000000001"},
                    "phase_sign":{"decimal":"-1","bits_hex":"bff0000000000000"}
                })
            )
        );
    }
}

#[cfg(test)]
mod error_metrics_family_tests {
    use super::*;
    use sidereon_core::dop::DopError as D;
    use sidereon_core::error_metrics::ErrorMetricsError as E;

    #[test]
    fn error_metrics_family_reserves_63_and_preserves_recursive_dop_data() {
        assert_eq!(SidereonEngineErrorFamily::ErrorMetrics as u32, 63);
        assert_eq!(
            SidereonEngineErrorFamily::ErrorMetrics.name(),
            "error_metrics"
        );
        assert_eq!(
            error_metrics_error_value(&E::NonFinite),
            rtk_node("non_finite", json!({}))
        );
        assert_eq!(
            error_metrics_error_value(&E::NotPositiveSemidefinite),
            rtk_node("not_positive_semidefinite", json!({}))
        );
        assert_eq!(
            error_metrics_error_value(&E::InvalidProbability),
            rtk_node("invalid_probability", json!({}))
        );
        assert_eq!(
            error_metrics_error_value(&E::Rotation(D::InvalidInput {
                field: "position_m",
                reason: "geodetic conversion failed",
            })),
            rtk_node(
                "rotation",
                json!({
                    "cause": rtk_node("invalid_input", json!({
                        "field":"position_m",
                        "reason":"geodetic conversion failed"
                    }))
                })
            )
        );
        assert_eq!(
            error_metrics_error_value(&E::Rotation(D::TooFewSatellites)),
            rtk_node(
                "rotation",
                json!({"cause":rtk_node("too_few_satellites", json!({}))})
            )
        );
        assert_eq!(
            error_metrics_error_value(&E::Rotation(D::Singular)),
            rtk_node("rotation", json!({"cause":rtk_node("singular", json!({}))}))
        );
    }
}
