//! C ABI bindings over the `sidereon` ergonomic engine surface.
//!
//! This crate is a thin INTERFACE in the C idiom: opaque handles, integer
//! status codes, and caller-allocated output buffers. It normalizes C input,
//! marshals it into the `sidereon` / `sidereon-core` types, calls the reference
//! solve, and copies the result into the caller's buffers. It contains ZERO
//! modeling logic of its own; the numbers it returns are exactly what
//! `sidereon-core` produces.
//!
//! Ownership: every `*_load` / `*_solve_*` that yields a handle transfers
//! ownership to the caller, who must release it with the matching `*_free`.
//! Reader functions borrow a handle and copy scalars/arrays into caller memory;
//! they never allocate on the caller's behalf. A failed call sets a thread-local
//! message retrievable with [`sidereon_last_error_message`].
//!
//! The header is generated from this source with cbindgen (see `cbindgen.toml`)
//! into `include/sidereon.h`.

// cbindgen exports these comments to C, where this header uses "Safety:".
#![allow(clippy::missing_safety_doc)]

use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_char, CStr, CString};
use std::mem::size_of;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;
use std::str::{self, FromStr};
use std::sync::Arc;

use sidereon::passes::{
    find_passes_for_satellite, ground_track, look_angle_arc, look_angle_batch_parallel,
    look_angle_batch_serial, propagate_teme_arc, propagate_teme_batch_parallel,
    propagate_teme_batch_serial, visible_from_satellites, GroundStation, LookAngle, LookAngleError,
    PassError, PassFinderOptions, UtcInstant, VisibleSatellite,
};
use sidereon::propagator::api::{IntegratorOptions, PropagationContext};
use sidereon::propagator::{
    ForceModelComponents, ForceModelKind, IntegratorKind, PropagationConfig, PropagationForceModel,
};
use sidereon::sgp4::{
    parse_tle_file_with_opsmode, DecayLatch, DecayLatchedError, Error as Sgp4Error,
    MinutesSinceEpoch, NamedSatellite, OpsMode, Prediction, Satellite,
};
use sidereon::state::CartesianState;
use sidereon::tle::{self as sidereon_tle, ChecksumWarning, TleElements};
use sidereon_core::antex::{Antenna, Antex, AntexError};
use sidereon_core::araim::{
    araim as core_araim, AraimError, AraimGeometry, AraimResult as CoreAraimResult, AraimRow,
    ConstellationIsm, IntegrityAllocation, Ism, SatelliteIsm, SatelliteIsmModel,
};
use sidereon_core::astro::atmosphere::{
    nrlmsise00_with_lst, ApArray, AtmosphereError, NrlmsiseInput, DEFAULT_AP, DEFAULT_F107,
    DEFAULT_F107A,
};
use sidereon_core::astro::constants::units::MICROSECONDS_PER_SECOND;
use sidereon_core::astro::constants::MU_EARTH;
use sidereon_core::astro::coverage::{
    access_counts as coverage_access_counts, look_angles_batch as coverage_look_angles_batch,
    max_elevation as coverage_max_elevation, visible_mask as coverage_visible_mask, LookAngleGrid,
};
use sidereon_core::astro::elements::{coe2rv, rv2coe, ClassicalElements, ElementsError, OrbitType};
use sidereon_core::astro::error::PropagationError;
use sidereon_core::astro::forces::drag::{DragForce, DragParameters, SpaceWeather};
use sidereon_core::astro::forces::SpaceWeatherSource;
use sidereon_core::astro::forces::{
    ForceModel, J2Gravity, SchwarzschildRelativity, SolarRadiationPressure,
    SolidEarthPoleTideGravity, SolidEarthTideGravity, SphericalHarmonicGravityConfig,
    ThirdBodyBodies, ThirdBodyGravity, TwoBodyGravity, ZonalCoefficients, ZonalDegrees,
    ZonalGravity,
};
use sidereon_core::astro::frames::transforms::FrameTransformError;
use sidereon_core::astro::frames::TdbEarthOrientationProvider;
use sidereon_core::astro::math::least_squares::Status;
use sidereon_core::astro::math::vec3::dot3;
use sidereon_core::astro::observation::{
    parallactic_angle_deg, satellite_visual_magnitude, sub_observer_point, sub_solar_point,
    terminator_latitude_deg, ObservationError, SurfacePoint,
};
use sidereon_core::astro::oem::{self as core_oem, Oem};
use sidereon_core::astro::omm::parse_json_array as parse_omm_json_array;
use sidereon_core::astro::opm::{self as core_opm, Opm};
use sidereon_core::astro::propagator::decay::{
    estimate_decay, DecayConfig, DecayError, DecayEstimate,
};
use sidereon_core::astro::propagator::StatePropagator;
use sidereon_core::astro::time::civil::j2000_seconds_from_split;
use sidereon_core::astro::time::gnss::{
    seconds_of_week_from_calendar, week_and_seconds_of_week, week_epoch_julian_day_number,
    week_from_calendar,
};
use sidereon_core::astro::time::scales::{
    find_leap_seconds, julian_day_number, leap_second_table, ut1_coverage,
};
use sidereon_core::astro::time::{
    split_julian_date_from_j2000_seconds, timescale_offset_at_s, timescale_offset_s, GnssWeekTow,
    Instant, InstantRepr, JulianDateSplit, TimeOffsetError, TimeScale,
};
use sidereon_core::astro::{Spk, SpkError, SpkState};
use sidereon_core::atmosphere::ionosphere::{
    galileo_nequick_g_native, ionex_slant_delay_with_policy, klobuchar_native, nequick_g_delay_m,
    nequick_g_stec_tecu, GalileoNequickCoeffs, GalileoNequickEval, Ionex, IonexCoverageError,
    IonexCoveragePolicy, IonexSlantDelayEvaluation, IonexSlantDelayStatus, KlobucharParams,
    NequickGRayEval, TecGridSamples as CoreTecGridSamples, TecSample as CoreTecSample,
    TecSamplesError,
};
use sidereon_core::atmosphere::troposphere::{MappingModel, Met};
use sidereon_core::bias::{
    bias_epoch_instant, BiasEpoch, BiasKind, BiasMode, BiasRecord, BiasSet, BiasTarget,
    ClockReferenceObservables, CodeDcbOptions,
};
use sidereon_core::clock_stability::{
    allan_deviation as core_allan_deviation,
    allan_deviation_power_law_slope as core_allan_deviation_power_law_slope,
    allan_variance_power_law_tau_exponent as core_allan_variance_power_law_tau_exponent,
    compute_allan_deviations as core_compute_allan_deviations,
    fit_power_law_noise as core_fit_power_law_noise, hadamard_deviation as core_hadamard_deviation,
    modified_adev as core_modified_adev,
    modified_allan_deviation_power_law_slope as core_modified_allan_deviation_power_law_slope,
    overlapping_adev as core_overlapping_adev,
    receiver_clock_phase_deviations as core_receiver_clock_phase_deviations,
    time_deviation as core_time_deviation, AllanDeviationCurves as CoreAllanDeviationCurves,
    AllanError, AllanEstimator as CoreAllanEstimator, AllanEstimatorSet, AllanInput, AllanOptions,
    AllanResult as CoreAllanResult, AllanSeries, GapPolicy, PowerLawNoiseError,
    PowerLawNoiseFit as CorePowerLawNoiseFit, PowerLawNoiseOptions, PowerLawNoiseRegion,
    PowerLawNoiseType, PowerLawOctave, PowerLawOctaveDominance, PowerLawOctaveFlag, TauGrid,
};
use sidereon_core::constants::SECONDS_PER_DAY;
use sidereon_core::constellation::{
    changed as constellation_changed, diff as constellation_diff, from_celestrak_omm,
    from_celestrak_omm_lenient, galileo_prn_for_gsat, glonass_fdma_channel,
    glonass_slot_for_number, gnss_sp3_id, is_valid as constellation_is_valid, merge_navcen,
    parse_navcen, to_csv, validate as constellation_validate, validate_against_sp3,
    validate_against_sp3_ids, validate_against_sp3_ids_strict, BoolStyle, Catalog as ConstCatalog,
    Diff as ConstDiff, FieldChange as ConstFieldChange, Record as ConstRecord,
    Validation as ConstValidation,
};
use sidereon_core::ephemeris::{
    align_clock_reference, clock_reference_offset, merge, precise_interpolant_store_checksum64,
    EpochAgreement, MergeCombine, MergeFlag, MergeOptions, MergeReport,
    MmapPreciseEphemerisInterpolant, PreciseEphemerisInterpolant, PreciseEphemerisSample,
    PreciseEphemerisSamples, PreciseInterpolantError, PreciseInterpolantStoreError,
    PreciseSamplesError, Sp3, Sp3FrameLabelSet, Sp3FrameReconciliationMethod,
    Sp3FrameReconciliationOptions, Sp3State,
};
use sidereon_core::ephemeris::{
    sample as ephemeris_sample, BroadcastEphemeris, EphemerisSampleRow, EphemerisSampleStatus,
};
use sidereon_core::frame::{
    geocentric_neu_basis, geodetic_to_itrf, itrf_to_geodetic, ItrfPositionM, Wgs84Geodetic,
};
use sidereon_core::geofence::{
    containment as geofence_containment, containment_probability as geofence_probability,
    containment_probability_with_options as geofence_probability_with_options,
    crossing_probability as geofence_crossing_probability,
    crossing_probability_with_options as geofence_crossing_probability_with_options,
    distance_to_boundary as geofence_distance_to_boundary, CrossingEvent as CoreGeofenceEvent,
    CrossingKind as CoreGeofenceCrossingKind, Fence as CoreGeofence,
    GeofenceError as CoreGeofenceError, GeofencePositionEstimate as CoreGeofencePositionEstimate,
    PositionUncertainty as CoreGeofenceUncertainty,
    ProbabilityHysteresis as CoreGeofenceHysteresis, ProbabilityMethod as CoreGeofenceMethod,
    ProbabilityOptions as CoreGeofenceOptions,
};
use sidereon_core::geoid::{
    ellipsoidal_height_m, geoid_undulation, orthometric_height_m, Egm2008GridSpacing,
    Egm2008RasterWindow, GeoidError, GeoidGrid,
};
use sidereon_core::geometry::{
    dop as core_dop, line_of_sight_from_az_el_deg, passes as geometry_passes,
    visibility_series as geometry_visibility_series, visible as geometry_visible, Dop, DopError,
    LineOfSight, VisibilityOptions, VisibilityPass, VisibilitySeriesPoint,
    VisibleSatellite as GeometryVisibleSatellite,
};
use sidereon_core::geometry_quality::{
    GeometryQuality as CoreGeometryQuality, ObservabilityTier as CoreObservabilityTier,
};
use sidereon_core::ils::{bounded_ils_search, lambda_ils_search, IlsError, IlsResult};
use sidereon_core::observables::{
    emission_media_batch_at_j2000_s as observables_emission_media_batch_at_j2000_s,
    predict as observables_predict, predict_ranges as observables_predict_ranges,
    EmissionMediaBatch, EmissionMediaBatchOptions, EmissionMediaStatus, ObservableEphemerisSource,
    ObservableIonosphereCorrection, ObservableMediaOptions, ObservableStateBatch,
    ObservableStateElementStatus as CoreObservableStateElementStatus,
    ObservableTroposphereCorrection, ObservablesError, PredictOptions, PredictedObservables,
    RangePrediction, RangePredictionRequest, OBSERVABLE_STATE_MISSING_POSITION_ECEF_M,
};
use sidereon_core::orbit::{
    drift as reduced_orbit_drift_core,
    drift_piecewise_reduced_orbit_source as reduced_orbit_piecewise_drift_source_core,
    drift_reduced_orbit_source as reduced_orbit_drift_source_core,
    fit_piecewise as reduced_orbit_fit_piecewise_core,
    fit_piecewise_reduced_orbit_source as reduced_orbit_piecewise_fit_source_core,
    fit_reduced_orbit_source as reduced_orbit_fit_source_core,
    fit_with_model as reduced_orbit_fit_core,
    piecewise_drift as reduced_orbit_piecewise_drift_core,
    piecewise_position as reduced_orbit_piecewise_position_core,
    piecewise_position_velocity as reduced_orbit_piecewise_position_velocity_core,
    position as reduced_orbit_position_core,
    position_velocity as reduced_orbit_position_velocity_core,
    select_piecewise_segment as reduced_orbit_select_piecewise_segment_core, CalendarEpoch,
    DriftEntry, DriftReport, EcefSample, Elements as ReducedOrbitElements,
    Frame as ReducedOrbitFrameInner, Model as ReducedOrbitModelInner,
    PiecewiseOrbit as ReducedOrbitPiecewise, PiecewiseOrbitError, PiecewiseOrbitSourceFitOptions,
    ReducedOrbit, ReducedOrbitError, ReducedOrbitSource, ReducedOrbitSourceDriftOptions,
    ReducedOrbitSourceError, ReducedOrbitSourceFitOptions, ReducedOrbitSourceSampling,
};
use sidereon_core::positioning::{
    solve_broadcast, solve_static as core_solve_static,
    solve_with_doppler_velocity as core_solve_with_doppler_velocity, solve_with_fallback,
    BroadcastReason, Corrections, DopplerObservation as CoreDopplerObservation, EphemerisSource,
    FallbackError, FixSource, KlobucharCoeffs, Observation, ReceiverSolution,
    RejectionReason as CoreSppRejectionReason, RinexSppEpochInputs, RinexSppEpochSolution,
    RinexSppError, RinexSppOptions, RobustConfig, SolveInputs, SolvePolicy,
    StaticClockBias as CoreStaticClockBias, StaticEpoch as CoreStaticEpoch,
    StaticEpochInfluence as CoreStaticEpochInfluence,
    StaticInfluenceStatus as CoreStaticInfluenceStatus, StaticResidual as CoreStaticResidual,
    StaticSatelliteBatchInfluence as CoreStaticSatelliteBatchInfluence,
    StaticSatelliteInfluence as CoreStaticSatelliteInfluence, StaticSolution as CoreStaticSolution,
    StaticSolutionMetadata as CoreStaticSolutionMetadata, StaticSolveError as CoreStaticSolveError,
    StaticSolveOptions as CoreStaticSolveOptions, SurfaceMet,
};
use sidereon_core::ppp_corrections::{CivilDateTime, CodeBiasOptions as PppCodeBiasOptions};
use sidereon_core::precise_positioning::defaults as ppp_defaults;
use sidereon_core::precise_positioning::{solve_ppp_auto_init_fixed, solve_ppp_auto_init_float};
use sidereon_core::precise_positioning::{
    FixedAmbiguityOptions as PppFixedAmbiguityOptionsInner, FixedSolution as PppFixedSolutionInner,
    FixedSolveConfig as PppFixedSolveConfigInner, FloatEpoch as PppFloatEpoch,
    FloatObservation as PppFloatObservation, FloatSolution as PppFloatSolutionInner,
    FloatSolveConfig as PppFloatSolveConfigInner, FloatSolveOptions as PppFloatSolveOptions,
    FloatState as PppFloatStateInner, FloatStatus as PppFloatStatus,
    IntegerStatus as PppIntegerStatusInner, MeasurementWeights as PppMeasurementWeightsInner,
    PcvSample as PppPcvSample, PppAutoInitOptions, PppInitialGuess, RangeCorrections,
    ReceiverAntennaFrequency as PppReceiverAntennaFrequencyInner,
    ReceiverAntennaOptions as PppReceiverAntennaOptionsInner, SatelliteClockCorrections,
    TropoMapping as PppTropoMapping, TroposphereOptions as PppTroposphereOptionsInner,
    VmfSiteSample as PppVmfSiteSample, VmfSiteSeries as PppVmfSiteSeries,
    VMF_SITE_MAX_SAMPLES as PPP_VMF_SITE_MAX_SAMPLES,
};
use sidereon_core::quality::{
    fde_spp, raim_fde_design, reliability_araim as core_reliability_araim,
    reliability_design as core_reliability_design, spp_robust_fde_driver,
    validate_receiver_solution,
    wtest_noncentrality_components as core_wtest_noncentrality_components, FdeError, FdeOptions,
    FdeSppError, FdeSppOptions, ObservationReliability as CoreObservationReliability, QualityError,
    RaimOptions, RaimWeights, RangeFdeOptions, RangeFdeResult, RangeFdeRow,
    RangeReliabilityRow as CoreRangeReliabilityRow, ReliabilityOptions as CoreReliabilityOptions,
    ReliabilityReport as CoreReliabilityReport, ReliabilitySummary as CoreReliabilitySummary,
    SolutionValidationOptions,
};
use sidereon_core::rinex::crinex::decode as crinex_decode;
use sidereon_core::rinex::observations::{
    carrier_phase_rows as rinex_obs_carrier_phase_rows,
    observation_values as rinex_obs_observation_values, pseudoranges as rinex_obs_pseudoranges,
    ObservationFilter as RinexObservationFilter, ObservationKind as RinexObservationKind, RinexObs,
    SignalPolicy as RinexSignalPolicy,
};
use sidereon_core::rtcm::{
    self as core_rtcm, AntennaDescriptor as RtcmAntennaDescriptor,
    BeidouEphemeris as RtcmBeidouEphemeris, GalileoFnavEphemeris as RtcmGalileoFnavEphemeris,
    GalileoInavEphemeris as RtcmGalileoInavEphemeris, GlonassEphemeris as RtcmGlonassEphemeris,
    GpsEphemeris as RtcmGpsEphemeris, LockTimeTracker as RtcmLockTimeTracker,
    Message as RtcmMessage, MsmKind as RtcmMsmKind, MsmMessage as RtcmMsmMessage,
    MsmSatellite as RtcmMsmSatellite, MsmSignal as RtcmMsmSignal, PreviousLock as RtcmPreviousLock,
    QzssEphemeris as RtcmQzssEphemeris, SsrClockRecord as RtcmSsrClockRecord,
    SsrHeader as RtcmSsrHeader, SsrKind as RtcmSsrKind, SsrMessage as RtcmSsrMessage,
    SsrOrbitRecord as RtcmSsrOrbitRecord, StationCoordinates as RtcmStationCoordinates,
    StreamDiagnostics as RtcmStreamDiagnostics, LLI_HALF_CYCLE as RTCM_LLI_HALF_CYCLE,
    LLI_LOSS_OF_LOCK as RTCM_LLI_LOSS_OF_LOCK,
};
use sidereon_core::rtk::{BaselineReferenceSelection, CycleSlipReceiver};
use sidereon_core::rtk_filter::defaults::{
    AMBIGUITY_TOL_M as RTK_AMBIGUITY_TOL_M, CODE_SIGMA_M as RTK_CODE_SIGMA_M,
    MAX_ITERATIONS as RTK_MAX_ITERATIONS, PARTIAL_MIN_AMBIGUITIES as RTK_PARTIAL_MIN_AMBIGUITIES,
    PHASE_SIGMA_M as RTK_PHASE_SIGMA_M, POSITION_TOL_M as RTK_POSITION_TOL_M,
    RATIO_THRESHOLD as RTK_RATIO_THRESHOLD,
};
use sidereon_core::rtk_filter::{
    build_dual_frequency_rinex_rtk_arc, build_rinex_rtk_arc, fix_wide_lane_rtk_arc,
    prepare_ionosphere_free_rtk_arc, solve_moving_baseline, solve_rtk_arc, solve_static_rtk_arc,
    solve_wide_lane_fixed_rtk_arc, AmbiguityScale, AmbiguitySet, CycleSlipPolicy,
    CycleSlipSplitArc, DynamicsModel, Epoch as RtkEpoch, FixedBaselineSolution, FixedSolveOpts,
    FloatBaselineSolution, FloatResidual, FloatSolveOpts, FloatSolveStatus,
    IntegerStatus as RtkIntegerStatus, MeasModel, MovingBaselineEpoch, MovingBaselineEpochSolution,
    MovingBaselineOpts, MovingBaselineStatus,
    ReceiverAntennaCalibration as RtkReceiverAntennaCalibrationInner,
    ReceiverAntennaCorrections as RtkReceiverAntennaCorrectionsInner, ResidualValidationOpts,
    RtkArcConfig, RtkArcEpoch, RtkArcEpochSolution, RtkArcError, RtkArcObservation,
    RtkArcPreprocessing, RtkArcSolution, RtkDualCycleSlipConfig, RtkDualFrequencyArcEpoch,
    RtkDualFrequencyObservation, RtkDualFrequencySatelliteObservation, RtkIonosphereFreeArcConfig,
    RtkIonosphereFreeArcError, RtkIonosphereFreeArcSolution, RtkRinexArcOptions,
    RtkRinexDualArcOptions, RtkRinexDualSignalPair, RtkRinexSignalPair, RtkStaticArcConfig,
    RtkStaticArcError, RtkStaticArcSolution, RtkWideLaneArcConfig, RtkWideLaneArcError,
    RtkWideLaneArcSolution, RtkWideLaneFixedArcConfig, RtkWideLaneFixedArcError,
    RtkWideLaneFixedArcMetadata, RtkWideLaneFixedArcSolution, RtkWideLaneFixedArcSolveConfig,
    RtkWideLaneFixedStaticArcSolution, SatMeas, SearchOpts, StochasticModel, UpdateOpts,
    ValidatedFixedBaselineSolution, ValidatedFixedSolveOpts, WideLaneError, WideLaneOptions,
};
use sidereon_core::sbas::{
    SbasBlock, SbasCorrectedEphemeris, SbasCorrectionStore, SbasFastCorrection,
    SbasFastCorrections, SbasFastDegradation, SbasGeoNav, SbasGeoState, SbasIgp, SbasIgpDelay,
    SbasIgpMask, SbasIntegrity, SbasIonoDelays, SbasLongTermCorrection, SbasLongTermHalf,
    SbasLongTermRecord, SbasMessage, SbasMixedFastCorrections, SbasPrnMask, SbasSolveMode,
    SbasWireForm,
};
use sidereon_core::sbas_pl::{
    sbas_protection_levels as core_sbas_protection_levels, AirborneModel as CoreAirborneModel,
    DegradationParams as CoreDegradationParams, SbasErrorModel as CoreSbasErrorModel,
    SbasKMultipliers as CoreSbasKMultipliers, SbasPlError as CoreSbasPlError,
    SbasProtection as CoreSbasProtection, SbasSisError as CoreSbasSisError,
};
use sidereon_core::ssr::{
    MissingCorrectionAction, OrbitReferencePoint, RegionalPolicy, SsrClockCorrection,
    SsrCorrectedEphemeris, SsrCorrectionStore, SsrFallbackPolicy, SsrOrbitCorrection,
};
use sidereon_core::staleness::{
    select_ionex_over_range, select_sp3_over_range, DegradationKind, SelectionError,
    StalenessMetadata, StalenessPolicy,
};
use sidereon_core::terrain::{DtedInterpolation, DtedLookupOptions, DtedTerrain, DtedTile};
use sidereon_core::terrain_store::{
    dted_tree_to_mmap_store as core_dted_tree_to_mmap_store,
    terrain_store_checksum64 as core_terrain_store_checksum64,
    write_dted_tree_to_mmap_store as core_write_dted_tree_to_mmap_store,
    Egm96FifteenMinuteGeoid as CoreEgm96FifteenMinuteGeoid, MmapTerrain as CoreMmapTerrain,
    OrthometricHeightM as CoreOrthometricHeightM, TerrainDatumError,
    TerrainGeoidModel as CoreTerrainGeoidModel, TerrainStoreError,
    TerrainStoreTileIndex as CoreTerrainStoreTileIndex, VerticalDatum as CoreVerticalDatum,
};
use sidereon_core::velocity::{
    VelocityError, VelocityObservable, VelocityObservation, VelocitySolution, VelocitySolveOptions,
};
use sidereon_core::{Error as CoreError, GnssSatelliteId, GnssSystem};

/// Integer status returned by every fallible entry point. SIDEREON_STATUS_OK
/// is the only success value; any other value means the call failed and a
/// human-readable reason is available from sidereon_last_error_message.
/// Failed calls may initialize or clear output arguments before returning; no
/// newly owned handle is transferred unless the status is SIDEREON_STATUS_OK.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonStatus {
    /// Success.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// An argument was structurally invalid (e.g. a buffer too small to hold
    /// the fixed-size output, or a non-finite count).
    InvalidArgument = 2,
    /// A C string argument was not valid UTF-8, or a satellite token did not
    /// parse into a known constellation/PRN.
    InvalidToken = 3,
    /// SP3 parsing failed.
    Sp3Parse = 4,
    /// The solve failed.
    Solve = 5,
    /// An internal panic reached the FFI boundary and was contained.
    Panic = 6,
}

/// Observability and covariance-validation tier for an estimation geometry.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonObservabilityTier {
    /// The design rank is below the parameter count; at least one parameter is
    /// not observable.
    RankDeficient = 0,
    /// The design is full rank but has no residual degrees of freedom. Snapshot
    /// solves report unvalidated covariance bounds at this tier.
    ZeroRedundancy = 1,
    /// The design is full rank with residual degrees of freedom, but exceeds a
    /// condition-number or GDOP cutoff. Bounds are reported unclamped.
    Weak = 2,
    /// The design is full rank and within the configured cutoffs.
    Nominal = 3,
}

/// Geometry observability and covariance-validation diagnostics.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeometryQuality {
    /// Observability and validation tier.
    pub tier: SidereonObservabilityTier,
    /// Observation redundancy, defined as number of observations minus number
    /// of estimated parameters.
    pub redundancy: i32,
    /// Rank of the design matrix used by the solve.
    pub rank: usize,
    /// Singular-value condition number of the design matrix.
    pub condition_number: f64,
    /// Geometric dilution of precision for the solved state.
    pub gdop: f64,
    /// Whether residual-based RAIM can test the solve.
    pub raim_checkable: bool,
    /// Whether residuals or a valid propagated prior validate the covariance
    /// bound.
    pub covariance_validated: bool,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

const MAX_SATELLITE_TOKEN_BYTES: usize = 16;
const SATELLITE_TOKEN_C_BYTES: usize = MAX_SATELLITE_TOKEN_BYTES + 1;
const MAX_RTK_ID_BYTES: usize = 64;
const RTK_ID_C_BYTES: usize = MAX_RTK_ID_BYTES + 1;
const MAX_PPP_ID_BYTES: usize = 64;
const PPP_ID_C_BYTES: usize = MAX_PPP_ID_BYTES + 1;
const MAX_PPP_ANTENNA_FREQ_LABEL_BYTES: usize = 32;
const MAX_TLE_LINE_BYTES: usize = 128;
const TLE_LINE_C_BYTES: usize = MAX_TLE_LINE_BYTES + 1;
/// Usable byte length of a visible-satellite id (excluding the terminator). The
/// caller-supplied id in a SidereonVisibleSatellite is stored null-terminated in
/// a fixed buffer; the input is rejected when longer (see
/// sidereon_visible_from_satellites), so it always fits.
const MAX_VISIBLE_ID_BYTES: usize = 64;
const VISIBLE_ID_C_BYTES: usize = MAX_VISIBLE_ID_BYTES + 1;

/// Record the most recent error message for the current thread.
fn set_last_error(message: impl Into<String>) {
    // A NUL inside an engine diagnostic would be unusual; if it ever occurs,
    // truncate at it rather than dropping the message entirely.
    let cstring = CString::new(message.into()).unwrap_or_else(|err| {
        let nul = err.nul_position();
        CString::new(&err.into_vec()[..nul]).unwrap()
    });
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(cstring));
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

/// Run an exported C entry point behind a panic boundary.
fn ffi_boundary<T>(fn_name: &str, panic_value: T, body: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            set_last_error(format!(
                "{fn_name}: panic: {}",
                panic_message(payload.as_ref())
            ));
            panic_value
        }
    }
}

macro_rules! c_try {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(status) => return status,
        }
    };
}

macro_rules! sel_try {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(status) => return marshal_status_to_selection(status),
        }
    };
}

macro_rules! fb_try {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(status) => return marshal_status_to_fallback(status),
        }
    };
}

mod almanac;
mod antenna;
mod antex;
mod araim;
mod atmosphere;
mod bias;
mod broadcast;
mod cdm;
mod clock;
mod combination;
mod constellation;
mod covariance;
mod coverage;
mod decay;
mod dgnss;
mod dop;
mod doppler;
mod drag;
mod dted;
mod ephemeris;
mod error_metrics;
mod estimation;
mod fde;
mod force;
mod frame;
mod frame_catalog;
mod fusion;
mod geodesic;
mod geodetic_time_series;
mod geofence;
mod geoid;
mod geometry;
mod glonass;
mod ils;
mod iod;
mod ionex;
mod mmap;
mod moving;
mod nmea;
mod ntrip;
mod observables;
mod observation;
mod oem;
mod omm;
mod opm;
mod orbit;
mod orbit_fit;
mod ppp;
mod precise;
mod precise_artifact;
mod raim;
mod range;
mod reduced;
mod reliability;
mod rf;
mod rinex;
mod rtcm;
mod rtk;
mod satellite;
mod sbas;
mod sbas_pl;
mod scenario;
mod sgp4;
mod sidereal;
mod signal;
mod solve;
mod source_localization;
mod sourced;
mod sp3;
mod space_weather;
mod spk;
mod spp;
mod ssr;
mod staleness;
mod static_positioning;
mod tca;
mod tdm;
mod terrain;
mod time;
mod tle;
mod trls;
mod tropo;
mod velocity;
pub use almanac::*;
pub use antenna::*;
pub use antex::*;
pub use araim::*;
pub use atmosphere::*;
pub use bias::*;
pub use broadcast::*;
pub use cdm::*;
pub use clock::*;
pub use combination::*;
pub use constellation::*;
pub use covariance::*;
pub use coverage::*;
pub use decay::*;
pub use dgnss::*;
pub use dop::*;
pub use doppler::*;
pub use drag::*;
pub use dted::*;
pub use ephemeris::*;
pub use error_metrics::*;
pub use estimation::*;
pub use fde::*;
pub use force::*;
pub use frame::*;
pub use frame_catalog::*;
pub use fusion::*;
pub use geodesic::*;
pub use geodetic_time_series::*;
pub use geofence::*;
pub use geoid::*;
pub use geometry::*;
pub use glonass::*;
pub use ils::*;
pub use iod::*;
pub use ionex::*;
pub use mmap::*;
pub use moving::*;
pub use nmea::*;
pub use ntrip::*;
pub use observables::*;
pub use observation::*;
pub use oem::*;
pub use omm::*;
pub use opm::*;
pub use orbit::*;
pub use orbit_fit::*;
pub use ppp::*;
pub use precise::*;
pub use precise_artifact::*;
pub use raim::*;
pub use range::*;
pub use reduced::*;
pub use reliability::*;
pub use rf::*;
pub use rinex::*;
pub use rtcm::*;
pub use rtk::*;
pub use satellite::*;
pub use sbas::*;
pub use sbas_pl::*;
pub use scenario::*;
pub use sgp4::*;
pub use sidereal::*;
pub use signal::*;
pub use solve::*;
pub use source_localization::*;
pub use sourced::*;
pub use sp3::*;
pub use space_weather::*;
pub use spk::*;
pub use spp::*;
pub use ssr::*;
pub use staleness::*;
pub use static_positioning::*;
pub use tca::*;
pub use tdm::*;
pub use terrain::*;
pub use time::*;
pub use tle::*;
pub use trls::*;
pub use tropo::*;
pub use velocity::*;

/// Run a fallible body, mapping any `sidereon` error to a status code while
/// recording its `Display` message verbatim.
fn guard<T>(
    status_on_err: SidereonStatus,
    body: impl FnOnce() -> sidereon::Result<T>,
) -> Result<T, SidereonStatus> {
    match body() {
        Ok(value) => Ok(value),
        Err(err) => {
            set_last_error(err.to_string());
            Err(status_on_err)
        }
    }
}

fn guard_core<T>(
    body: impl FnOnce() -> sidereon_core::Result<T>,
    map_err: impl FnOnce(CoreError) -> SidereonStatus,
) -> Result<T, SidereonStatus> {
    match body() {
        Ok(value) => Ok(value),
        Err(err) => Err(map_err(err)),
    }
}

fn validate_element_count<T>(
    fn_name: &str,
    arg_name: &str,
    count: usize,
) -> Result<(), SidereonStatus> {
    let element_size = size_of::<T>().max(1);
    if count > isize::MAX as usize / element_size {
        set_last_error(format!("{fn_name}: {arg_name} is too large"));
        Err(SidereonStatus::InvalidArgument)
    } else {
        Ok(())
    }
}

unsafe fn require_ref<'a, T>(
    ptr: *const T,
    fn_name: &str,
    arg_name: &str,
) -> Result<&'a T, SidereonStatus> {
    ptr.as_ref().ok_or_else(|| {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        SidereonStatus::NullPointer
    })
}

unsafe fn require_mut<'a, T>(
    ptr: *mut T,
    fn_name: &str,
    arg_name: &str,
) -> Result<&'a mut T, SidereonStatus> {
    ptr.as_mut().ok_or_else(|| {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        SidereonStatus::NullPointer
    })
}

unsafe fn require_out<'a, T>(
    ptr: *mut T,
    fn_name: &str,
    arg_name: &str,
) -> Result<&'a mut T, SidereonStatus> {
    ptr.as_mut().ok_or_else(|| {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        SidereonStatus::NullPointer
    })
}

unsafe fn require_slice<'a, T>(
    ptr: *const T,
    count: usize,
    fn_name: &str,
    arg_name: &str,
) -> Result<&'a [T], SidereonStatus> {
    if count == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    validate_element_count::<T>(fn_name, arg_name, count)?;
    Ok(slice::from_raw_parts(ptr, count))
}

unsafe fn require_out_array<T>(
    ptr: *mut T,
    count: usize,
    fn_name: &str,
    arg_name: &str,
) -> Result<(), SidereonStatus> {
    if count == 0 {
        return Ok(());
    }
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    validate_element_count::<T>(fn_name, arg_name, count)
}

fn insert_unique_string_key<T>(
    fn_name: &str,
    arg_name: &str,
    idx: usize,
    out: &mut BTreeMap<String, T>,
    id: String,
    value: T,
) -> Result<(), SidereonStatus> {
    match out.entry(id) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) => {
            set_last_error(format!(
                "{fn_name}: duplicate {arg_name} key '{}' at index {idx}",
                entry.key()
            ));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn write_boxed_handle<T>(out: &mut *mut T, value: T) {
    *out = Box::into_raw(Box::new(value));
}

unsafe fn free_boxed<T>(ptr: *mut T) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

unsafe fn zero_f64_prefix(out: *mut f64, len: usize, required: usize) {
    for idx in 0..len.min(required) {
        *out.add(idx) = 0.0;
    }
}

unsafe fn copy_exact_f64s(
    fn_name: &str,
    arg_name: &str,
    out: *mut f64,
    len: usize,
    values: &[f64],
) -> Result<(), SidereonStatus> {
    if out.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    zero_f64_prefix(out, len, values.len());
    validate_element_count::<f64>(fn_name, "len", len)?;
    if len < values.len() {
        set_last_error(format!(
            "{fn_name}: {arg_name} needs room for {} doubles",
            values.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
    Ok(())
}

unsafe fn copy_prefix_to_c<T: Copy>(
    fn_name: &str,
    out_name: &str,
    values: &[T],
    out: *mut T,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let out_written = require_out(out_written, fn_name, "out_written")?;
    *out_written = 0;
    let out_required = require_out(out_required, fn_name, "out_required")?;
    *out_required = 0;
    validate_element_count::<T>(fn_name, "required", values.len())?;
    *out_required = values.len();
    validate_element_count::<T>(fn_name, "len", len)?;
    if out.is_null() {
        if len == 0 {
            return Ok(());
        }
        set_last_error(format!("{fn_name}: null {out_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    if len < values.len() {
        set_last_error(format!(
            "{fn_name}: {out_name} needs room for {} entries",
            values.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    if !values.is_empty() {
        ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
    }
    *out_written = values.len();
    Ok(())
}

fn checked_flattened_count<T>(
    fn_name: &str,
    row_name: &str,
    row_lengths: impl IntoIterator<Item = usize>,
) -> Result<usize, SidereonStatus> {
    let mut required = 0usize;
    for row_len in row_lengths {
        required = required.checked_add(row_len).ok_or_else(|| {
            set_last_error(format!("{fn_name}: {row_name} is too large"));
            SidereonStatus::InvalidArgument
        })?;
        validate_element_count::<T>(fn_name, row_name, required)?;
    }
    Ok(required)
}

unsafe fn copy_flattened_rows_to_c<Row, T: Copy>(
    fn_name: &str,
    rows: &[Vec<Row>],
    map: impl Fn(&Row) -> T,
    out: *mut T,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let out_written = require_out(out_written, fn_name, "out_written")?;
    *out_written = 0;
    let out_required = require_out(out_required, fn_name, "out_required")?;
    *out_required = 0;
    let required = checked_flattened_count::<T>(fn_name, "required", rows.iter().map(Vec::len))?;
    *out_required = required;
    validate_element_count::<T>(fn_name, "len", len)?;
    if out.is_null() {
        if len == 0 {
            return Ok(());
        }
        set_last_error(format!("{fn_name}: null out"));
        return Err(SidereonStatus::NullPointer);
    }
    if len < required {
        set_last_error(format!("{fn_name}: out needs room for {required} entries"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let mut offset = 0usize;
    for row in rows {
        for value in row {
            *out.add(offset) = map(value);
            offset += 1;
        }
    }
    *out_written = required;
    Ok(())
}

unsafe fn init_copy_counts(
    fn_name: &str,
    out_written: *mut usize,
    out_required: *mut usize,
) -> Result<(), SidereonStatus> {
    let out_written = require_out(out_written, fn_name, "out_written")?;
    *out_written = 0;
    let out_required = require_out(out_required, fn_name, "out_required")?;
    *out_required = 0;
    Ok(())
}

unsafe fn parse_satellite_token(
    fn_name: &str,
    sat_id: *const c_char,
) -> Result<GnssSatelliteId, SidereonStatus> {
    if sat_id.is_null() {
        set_last_error(format!("{fn_name}: null satellite token"));
        return Err(SidereonStatus::NullPointer);
    }

    let mut token_len = None;
    for idx in 0..=MAX_SATELLITE_TOKEN_BYTES {
        if *sat_id.add(idx) == 0 {
            token_len = Some(idx);
            break;
        }
    }

    let Some(token_len) = token_len else {
        set_last_error(format!(
            "{fn_name}: satellite token is not null-terminated within {MAX_SATELLITE_TOKEN_BYTES} bytes"
        ));
        return Err(SidereonStatus::InvalidArgument);
    };

    let bytes = slice::from_raw_parts(sat_id.cast::<u8>(), token_len);
    let token = match str::from_utf8(bytes) {
        Ok(token) => token,
        Err(_) => {
            set_last_error(format!("{fn_name}: satellite token is not valid UTF-8"));
            return Err(SidereonStatus::InvalidToken);
        }
    };
    GnssSatelliteId::from_str(token).map_err(|_| {
        set_last_error(format!("{fn_name}: invalid satellite token: {token}"));
        SidereonStatus::InvalidToken
    })
}

unsafe fn parse_bounded_c_string(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
    max_len: usize,
) -> Result<String, SidereonStatus> {
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }

    let mut token_len = None;
    for idx in 0..=max_len {
        if *ptr.add(idx) == 0 {
            token_len = Some(idx);
            break;
        }
    }

    let Some(token_len) = token_len else {
        set_last_error(format!(
            "{fn_name}: {arg_name} is not null-terminated within {max_len} bytes"
        ));
        return Err(SidereonStatus::InvalidArgument);
    };
    if token_len == 0 {
        set_last_error(format!("{fn_name}: {arg_name} is empty"));
        return Err(SidereonStatus::InvalidArgument);
    }

    let bytes = slice::from_raw_parts(ptr.cast::<u8>(), token_len);
    match str::from_utf8(bytes) {
        Ok(token) => Ok(token.to_owned()),
        Err(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            Err(SidereonStatus::InvalidToken)
        }
    }
}

unsafe fn parse_c_string(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
) -> Result<String, SidereonStatus> {
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(value) if !value.is_empty() => Ok(value.to_owned()),
        Ok(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is empty"));
            Err(SidereonStatus::InvalidArgument)
        }
        Err(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            Err(SidereonStatus::InvalidToken)
        }
    }
}

fn satellite_token(sat_id: GnssSatelliteId) -> SidereonSatelliteToken {
    let text = sat_id.to_string();
    satellite_token_from_text(&text)
}

fn satellite_token_from_text(text: &str) -> SidereonSatelliteToken {
    debug_assert!(text.len() < SATELLITE_TOKEN_C_BYTES);
    let mut token = SidereonSatelliteToken {
        bytes: [0; SATELLITE_TOKEN_C_BYTES],
    };
    for (idx, byte) in text.bytes().take(MAX_SATELLITE_TOKEN_BYTES).enumerate() {
        token.bytes[idx] = byte as c_char;
    }
    token
}

fn rtk_id_token(text: &str) -> SidereonRtkId {
    debug_assert!(text.len() < RTK_ID_C_BYTES);
    let mut token = SidereonRtkId {
        bytes: [0; RTK_ID_C_BYTES],
    };
    for (idx, byte) in text.bytes().take(MAX_RTK_ID_BYTES).enumerate() {
        token.bytes[idx] = byte as c_char;
    }
    token
}

fn ppp_id_token(text: &str) -> SidereonPppId {
    debug_assert!(text.len() < PPP_ID_C_BYTES);
    let mut token = SidereonPppId {
        bytes: [0; PPP_ID_C_BYTES],
    };
    for (idx, byte) in text.bytes().take(MAX_PPP_ID_BYTES).enumerate() {
        token.bytes[idx] = byte as c_char;
    }
    token
}

fn gnss_system_to_c(system: GnssSystem) -> SidereonGnssSystem {
    match system {
        GnssSystem::Gps => SidereonGnssSystem::Gps,
        GnssSystem::Glonass => SidereonGnssSystem::Glonass,
        GnssSystem::Galileo => SidereonGnssSystem::Galileo,
        GnssSystem::BeiDou => SidereonGnssSystem::BeiDou,
        GnssSystem::Qzss => SidereonGnssSystem::Qzss,
        GnssSystem::Navic => SidereonGnssSystem::Navic,
        GnssSystem::Sbas => SidereonGnssSystem::Sbas,
    }
}

fn gnss_system_to_letter(system: GnssSystem) -> String {
    system.letter().to_string()
}

fn gnss_system_from_c_code(
    fn_name: &str,
    arg_name: &str,
    system: u32,
) -> Result<GnssSystem, SidereonStatus> {
    match system {
        value if value == SidereonGnssSystem::Gps as u32 => Ok(GnssSystem::Gps),
        value if value == SidereonGnssSystem::Glonass as u32 => Ok(GnssSystem::Glonass),
        value if value == SidereonGnssSystem::Galileo as u32 => Ok(GnssSystem::Galileo),
        value if value == SidereonGnssSystem::BeiDou as u32 => Ok(GnssSystem::BeiDou),
        value if value == SidereonGnssSystem::Qzss as u32 => Ok(GnssSystem::Qzss),
        value if value == SidereonGnssSystem::Navic as u32 => Ok(GnssSystem::Navic),
        value if value == SidereonGnssSystem::Sbas as u32 => Ok(GnssSystem::Sbas),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} GNSS system"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn observability_tier_to_c(tier: CoreObservabilityTier) -> SidereonObservabilityTier {
    match tier {
        CoreObservabilityTier::RankDeficient => SidereonObservabilityTier::RankDeficient,
        CoreObservabilityTier::ZeroRedundancy => SidereonObservabilityTier::ZeroRedundancy,
        CoreObservabilityTier::Weak => SidereonObservabilityTier::Weak,
        CoreObservabilityTier::Nominal => SidereonObservabilityTier::Nominal,
    }
}

fn geometry_quality_to_c(quality: &CoreGeometryQuality) -> SidereonGeometryQuality {
    SidereonGeometryQuality {
        tier: observability_tier_to_c(quality.tier),
        redundancy: quality.redundancy,
        rank: quality.rank,
        condition_number: quality.condition_number,
        gdop: quality.gdop,
        raim_checkable: quality.raim_checkable,
        covariance_validated: quality.covariance_validated,
    }
}

fn empty_geometry_quality() -> SidereonGeometryQuality {
    SidereonGeometryQuality {
        tier: SidereonObservabilityTier::RankDeficient,
        redundancy: 0,
        rank: 0,
        condition_number: 0.0,
        gdop: 0.0,
        raim_checkable: false,
        covariance_validated: false,
    }
}

fn time_scale_from_c_code(
    fn_name: &str,
    arg_name: &str,
    scale: u32,
) -> Result<TimeScale, SidereonStatus> {
    match scale {
        value if value == SidereonTimeScale::Utc as u32 => Ok(TimeScale::Utc),
        value if value == SidereonTimeScale::Tai as u32 => Ok(TimeScale::Tai),
        value if value == SidereonTimeScale::Tt as u32 => Ok(TimeScale::Tt),
        value if value == SidereonTimeScale::Tdb as u32 => Ok(TimeScale::Tdb),
        value if value == SidereonTimeScale::Gpst as u32 => Ok(TimeScale::Gpst),
        value if value == SidereonTimeScale::Gst as u32 => Ok(TimeScale::Gst),
        value if value == SidereonTimeScale::Bdt as u32 => Ok(TimeScale::Bdt),
        value if value == SidereonTimeScale::Glonasst as u32 => Ok(TimeScale::Glonasst),
        value if value == SidereonTimeScale::Qzsst as u32 => Ok(TimeScale::Qzsst),
        value if value == SidereonTimeScale::Tcg as u32 => Ok(TimeScale::Tcg),
        value if value == SidereonTimeScale::Tcb as u32 => Ok(TimeScale::Tcb),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} time scale {scale}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn time_scale_to_c_code(scale: TimeScale) -> u32 {
    match scale {
        TimeScale::Utc => SidereonTimeScale::Utc as u32,
        TimeScale::Tai => SidereonTimeScale::Tai as u32,
        TimeScale::Tt => SidereonTimeScale::Tt as u32,
        TimeScale::Tdb => SidereonTimeScale::Tdb as u32,
        TimeScale::Gpst => SidereonTimeScale::Gpst as u32,
        TimeScale::Gst => SidereonTimeScale::Gst as u32,
        TimeScale::Bdt => SidereonTimeScale::Bdt as u32,
        TimeScale::Glonasst => SidereonTimeScale::Glonasst as u32,
        TimeScale::Qzsst => SidereonTimeScale::Qzsst as u32,
        TimeScale::Tcg => SidereonTimeScale::Tcg as u32,
        TimeScale::Tcb => SidereonTimeScale::Tcb as u32,
    }
}

fn instant_to_j2000_seconds(epoch: &Instant) -> Option<f64> {
    match epoch.repr {
        InstantRepr::JulianDate(jd) => Some(j2000_seconds_from_split(jd.jd_whole, jd.fraction)),
        InstantRepr::Nanos(_) => None,
    }
}

/// Marshal a caller `SidereonGeodetic` into the engine's `Wgs84Geodetic`,
/// applying the engine's own finite/range validation.
fn geodetic_to_wgs84(
    fn_name: &str,
    arg_name: &str,
    geodetic: SidereonGeodetic,
) -> Result<Wgs84Geodetic, SidereonStatus> {
    Wgs84Geodetic::new(geodetic.lat_rad, geodetic.lon_rad, geodetic.height_m).map_err(|err| {
        set_last_error(format!("{fn_name}: {arg_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn empty_geodetic() -> SidereonGeodetic {
    SidereonGeodetic {
        lat_rad: 0.0,
        lon_rad: 0.0,
        height_m: 0.0,
    }
}

fn default_robust_config() -> SidereonSppRobustConfig {
    let robust = RobustConfig::default();
    SidereonSppRobustConfig {
        huber_k: robust.huber_k,
        scale_floor_m: robust.scale_floor_m,
        max_outer: robust.max_outer,
        outer_tol_m: robust.outer_tol_m,
    }
}

fn default_validation_options() -> SidereonSppValidationOptions {
    let validation = SolutionValidationOptions::default();
    SidereonSppValidationOptions {
        max_pdop_enabled: validation.max_pdop.is_some(),
        max_pdop: validation.max_pdop.unwrap_or(0.0),
        min_plausible_radius_m: validation.min_plausible_radius_m,
        max_plausible_radius_m: validation.max_plausible_radius_m,
        max_converged_residual_rms_m: validation.max_converged_residual_rms_m,
    }
}

fn default_solve_policy() -> SidereonSppSolvePolicy {
    SidereonSppSolvePolicy {
        use_validation_options: true,
        validation: default_validation_options(),
        coarse_search_enabled: false,
        coarse_search_seeds: 0,
    }
}

fn default_spp_inputs_v2() -> SidereonSppInputsV2 {
    SidereonSppInputsV2 {
        base: SidereonSppInputs {
            observations: ptr::null(),
            observation_count: 0,
            t_rx_j2000_s: 0.0,
            t_rx_second_of_day_s: 0.0,
            day_of_year: 0.0,
            initial_guess: [0.0; 4],
            ionosphere: false,
            troposphere: false,
            klobuchar_alpha: [0.0; 4],
            klobuchar_beta: [0.0; 4],
            pressure_hpa: 0.0,
            temperature_k: 0.0,
            relative_humidity: 0.0,
            with_geodetic: false,
        },
        beidou_klobuchar_enabled: false,
        beidou_klobuchar_alpha: [0.0; 4],
        beidou_klobuchar_beta: [0.0; 4],
        robust_enabled: false,
        robust: default_robust_config(),
        policy: default_solve_policy(),
        glonass_channels: ptr::null(),
        glonass_channel_count: 0,
    }
}

unsafe fn build_spp_solve_inputs(
    fn_name: &str,
    inputs: &SidereonSppInputs,
    beidou_klobuchar: Option<KlobucharCoeffs>,
    robust: Option<RobustConfig>,
    glonass_channels: BTreeMap<u8, i8>,
) -> Result<SolveInputs, SidereonStatus> {
    let raw_observations = require_slice(
        inputs.observations,
        inputs.observation_count,
        fn_name,
        "observations",
    )?;
    validate_element_count::<Observation>(fn_name, "observation_count", raw_observations.len())?;
    let mut observations = Vec::with_capacity(raw_observations.len());
    for obs in raw_observations {
        let satellite_id = parse_satellite_token(fn_name, obs.sat_id)?;
        observations.push(Observation {
            satellite_id,
            pseudorange_m: obs.pseudorange_m,
        });
    }

    Ok(SolveInputs {
        observations,
        t_rx_j2000_s: inputs.t_rx_j2000_s,
        t_rx_second_of_day_s: inputs.t_rx_second_of_day_s,
        day_of_year: inputs.day_of_year,
        initial_guess: inputs.initial_guess,
        corrections: Corrections {
            ionosphere: inputs.ionosphere,
            troposphere: inputs.troposphere,
        },
        klobuchar: KlobucharCoeffs {
            alpha: inputs.klobuchar_alpha,
            beta: inputs.klobuchar_beta,
        },
        beidou_klobuchar,
        // The C surface does not yet expose broadcast Galileo NeQuick-G
        // coefficients, so Galileo keeps the Klobuchar fallback and existing
        // zero-Galileo goldens stay bit-identical.
        galileo_nequick: None,
        sbas_iono: None,
        glonass_channels,
        met: SurfaceMet {
            pressure_hpa: inputs.pressure_hpa,
            temperature_k: inputs.temperature_k,
            relative_humidity: inputs.relative_humidity,
        },
        robust,
    })
}

/// Convert the V2 GLONASS channel array into the engine's slot -> channel map.
/// A null/zero-length array yields an empty map (no GLONASS channels), keeping
/// every non-GLONASS solve bit-identical. Duplicate slots are rejected so the
/// result never depends on array order.
unsafe fn glonass_channels_from_c(
    fn_name: &str,
    inputs: &SidereonSppInputsV2,
) -> Result<BTreeMap<u8, i8>, SidereonStatus> {
    let rows = require_slice(
        inputs.glonass_channels,
        inputs.glonass_channel_count,
        fn_name,
        "glonass_channels",
    )?;
    let mut channels = BTreeMap::new();
    for (idx, row) in rows.iter().enumerate() {
        if channels.insert(row.slot, row.channel).is_some() {
            set_last_error(format!(
                "{fn_name}: duplicate glonass_channels slot {} at index {idx}",
                row.slot
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
    }
    Ok(channels)
}

fn robust_config_from_c(inputs: &SidereonSppInputsV2) -> Option<RobustConfig> {
    inputs.robust_enabled.then_some(RobustConfig {
        huber_k: inputs.robust.huber_k,
        scale_floor_m: inputs.robust.scale_floor_m,
        max_outer: inputs.robust.max_outer,
        outer_tol_m: inputs.robust.outer_tol_m,
    })
}

fn beidou_klobuchar_from_c(inputs: &SidereonSppInputsV2) -> Option<KlobucharCoeffs> {
    inputs.beidou_klobuchar_enabled.then_some(KlobucharCoeffs {
        alpha: inputs.beidou_klobuchar_alpha,
        beta: inputs.beidou_klobuchar_beta,
    })
}

/// Convert the C validation-options view into the engine's
/// [`SolutionValidationOptions`]. When `use_validation_options` is false the
/// engine default gates are used, exactly as the V2 solve policy does; shared by
/// the SPP solve policy and the FDE per-iteration solve so both honor the same
/// gates.
fn validation_options_from_c(
    use_validation_options: bool,
    validation: &SidereonSppValidationOptions,
) -> SolutionValidationOptions {
    if use_validation_options {
        SolutionValidationOptions {
            max_pdop: validation.max_pdop_enabled.then_some(validation.max_pdop),
            min_plausible_radius_m: validation.min_plausible_radius_m,
            max_plausible_radius_m: validation.max_plausible_radius_m,
            max_converged_residual_rms_m: validation.max_converged_residual_rms_m,
        }
    } else {
        SolutionValidationOptions::default()
    }
}

unsafe fn rtk_sat_measurement_from_c(
    fn_name: &str,
    row: &SidereonRtkSatMeasurement,
) -> Result<SatMeas, SidereonStatus> {
    let sat = parse_satellite_token(fn_name, row.sat_id)?.to_string();
    let sd_ambiguity_id = parse_bounded_c_string(
        fn_name,
        "sd_ambiguity_id",
        row.sd_ambiguity_id,
        MAX_RTK_ID_BYTES,
    )?;
    Ok(SatMeas {
        sat,
        sd_ambiguity_id,
        base_code_m: row.base_code_m,
        base_phase_m: row.base_phase_m,
        rover_code_m: row.rover_code_m,
        rover_phase_m: row.rover_phase_m,
        base_tx_pos: row.base_tx_pos,
        rover_tx_pos: row.rover_tx_pos,
        pos: row.pos,
    })
}

unsafe fn rtk_f64_map_from_c(
    fn_name: &str,
    values: *const SidereonRtkFloatMapEntry,
    count: usize,
    arg_name: &str,
) -> Result<BTreeMap<String, f64>, SidereonStatus> {
    let raw_values = require_slice(values, count, fn_name, arg_name)?;
    validate_element_count::<SidereonRtkFloatMapEntry>(fn_name, arg_name, raw_values.len())?;
    let mut out = BTreeMap::new();
    for (idx, value) in raw_values.iter().enumerate() {
        let id = parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}[{idx}].id"),
            value.id,
            MAX_RTK_ID_BYTES,
        )?;
        insert_unique_string_key(fn_name, arg_name, idx, &mut out, id, value.value)?;
    }
    Ok(out)
}

unsafe fn rtk_float_only_systems_from_c(
    fn_name: &str,
    values: *const u32,
    count: usize,
) -> Result<Vec<String>, SidereonStatus> {
    validate_element_count::<String>(fn_name, "float_only_system_count", count)?;
    let raw_values = require_slice(values, count, fn_name, "float_only_systems")?;
    let mut out = Vec::with_capacity(raw_values.len());
    for (idx, value) in raw_values.iter().copied().enumerate() {
        let system =
            gnss_system_from_c_code(fn_name, &format!("float_only_systems[{idx}]"), value)?;
        out.push(gnss_system_to_letter(system).to_owned());
    }
    Ok(out)
}

fn rtk_model_from_c(
    fn_name: &str,
    model: &SidereonRtkMeasurementModel,
) -> Result<MeasModel, SidereonStatus> {
    let stochastic = match model.stochastic {
        value if value == SidereonRtkStochasticModel::Simple as u32 => StochasticModel::Simple {
            elevation_weighting: model.elevation_weighting,
        },
        value if value == SidereonRtkStochasticModel::Rtklib as u32 => StochasticModel::Rtklib,
        _ => {
            set_last_error(format!("{fn_name}: invalid RTK stochastic model"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(MeasModel {
        code_sigma_m: model.code_sigma_m,
        phase_sigma_m: model.phase_sigma_m,
        sagnac: model.sagnac,
        stochastic,
    })
}

fn rtk_solve_status_to_c(status: FloatSolveStatus) -> SidereonRtkSolveStatus {
    match status {
        FloatSolveStatus::StateTolerance => SidereonRtkSolveStatus::StateTolerance,
        FloatSolveStatus::MaxIterations => SidereonRtkSolveStatus::MaxIterations,
    }
}

fn rtk_integer_status_to_c(status: RtkIntegerStatus) -> SidereonRtkIntegerStatus {
    match status {
        RtkIntegerStatus::Fixed => SidereonRtkIntegerStatus::Fixed,
        RtkIntegerStatus::NotFixed => SidereonRtkIntegerStatus::NotFixed,
    }
}

fn rtk_float_metadata(solution: &FloatBaselineSolution) -> SidereonRtkFloatMetadata {
    SidereonRtkFloatMetadata {
        iterations: solution.iterations,
        converged: solution.converged,
        status: rtk_solve_status_to_c(solution.status),
        code_rms_m: solution.code_rms_m,
        phase_rms_m: solution.phase_rms_m,
        weighted_rms_m: solution.weighted_rms_m,
        n_observations: solution.n_observations,
        ambiguity_count: solution.ambiguities_m.len(),
        residual_count: solution.residuals.len(),
        used_sat_count: rtk_used_satellite_tokens(&solution.residuals).len(),
        geometry_quality: geometry_quality_to_c(&solution.geometry_quality),
    }
}

fn rtk_fixed_metadata_from_solution(
    fixed: &FixedBaselineSolution,
    geometry_quality: &CoreGeometryQuality,
) -> SidereonRtkFixedMetadata {
    SidereonRtkFixedMetadata {
        iterations: fixed.iterations,
        converged: fixed.converged,
        status: rtk_solve_status_to_c(fixed.status),
        code_rms_m: fixed.code_rms_m,
        phase_rms_m: fixed.phase_rms_m,
        weighted_rms_m: fixed.weighted_rms_m,
        n_observations: fixed.n_observations,
        free_ambiguity_count: fixed.free_ambiguities_m.len(),
        fixed_ambiguity_count: fixed.fixed_ambiguities_cycles.len(),
        residual_count: fixed.residuals.len(),
        used_sat_count: rtk_used_satellite_tokens(&fixed.residuals).len(),
        integer_status: rtk_integer_status_to_c(fixed.search.integer_status),
        has_integer_ratio: fixed.search.integer_ratio.is_some(),
        integer_ratio: fixed.search.integer_ratio.unwrap_or(0.0),
        has_integer_best_score: fixed.search.integer_best_score.is_some(),
        integer_best_score: fixed.search.integer_best_score.unwrap_or(0.0),
        has_integer_second_best_score: fixed.search.integer_second_best_score.is_some(),
        integer_second_best_score: fixed.search.integer_second_best_score.unwrap_or(0.0),
        integer_candidates: fixed.search.integer_candidates,
        geometry_quality: geometry_quality_to_c(geometry_quality),
    }
}

fn missing_fixed_ambiguity_meter(fn_name: &str, id: &str) -> SidereonStatus {
    set_last_error(format!(
        "{fn_name}: fixed ambiguity '{id}' has cycles but no meter value"
    ));
    SidereonStatus::InvalidArgument
}

fn rtk_fixed_ambiguity_rows_to_c(
    fn_name: &str,
    cycles: &[(String, i64)],
    meters: &BTreeMap<&str, f64>,
) -> Result<Vec<SidereonRtkFixedAmbiguity>, SidereonStatus> {
    cycles
        .iter()
        .map(|(id, cycles)| {
            let value_m = meters
                .get(id.as_str())
                .copied()
                .ok_or_else(|| missing_fixed_ambiguity_meter(fn_name, id))?;
            Ok(SidereonRtkFixedAmbiguity {
                id: rtk_id_token(id),
                cycles: *cycles,
                value_m,
            })
        })
        .collect()
}

fn rtk_used_satellite_tokens(residuals: &[FloatResidual]) -> Vec<SidereonSatelliteToken> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for residual in residuals {
        if seen.insert(residual.satellite_id.as_str()) {
            out.push(satellite_token_from_text(&residual.satellite_id));
        }
    }
    out
}

// Project an ECEF baseline onto the geocentric local East-North-Up frame at the
// base station. The basis is built entirely by core
// (`sidereon_core::frame::geocentric_neu_basis`, itself `geocentric_up` ->
// `geocentric_east` -> north), which is the shared byte-exact recipe the RTK
// baseline/PPP goldens were captured against; this binding only marshals the
// dot products of the supplied baseline against that basis.

fn ppp_tropo_mapping_from_c(
    fn_name: &str,
    tropo: &SidereonPppTroposphereOptions,
) -> Result<PppTropoMapping, SidereonStatus> {
    match tropo.mapping {
        value if value == SidereonPppTropoMapping::Niell as u32 => Ok(PppTropoMapping::Niell),
        value if value == SidereonPppTropoMapping::Vmf1 as u32 => {
            let count = tropo.vmf_sample_count;
            if count == 0 || count > SIDEREON_PPP_VMF_SITE_MAX_SAMPLES {
                set_last_error(format!(
                    "{fn_name}: tropo.vmf_sample_count {count} out of range 1..={SIDEREON_PPP_VMF_SITE_MAX_SAMPLES}"
                ));
                return Err(SidereonStatus::InvalidArgument);
            }
            let samples: Vec<PppVmfSiteSample> = tropo.vmf_samples[..count]
                .iter()
                .map(|s| PppVmfSiteSample {
                    mjd: s.mjd,
                    ah: s.ah,
                    aw: s.aw,
                })
                .collect();
            // The core validates ascending mjd and positive, finite coefficients.
            let series = PppVmfSiteSeries::new(&samples).map_err(|err| {
                set_last_error(format!("{fn_name}: tropo.vmf_samples: {err}"));
                SidereonStatus::InvalidArgument
            })?;
            Ok(PppTropoMapping::Vmf1(series))
        }
        other => {
            set_last_error(format!("{fn_name}: invalid tropo.mapping {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn reject_unsupported_ppp_correction(
    fn_name: &str,
    enabled: bool,
    field_name: &str,
    reason: &str,
) -> Result<(), SidereonStatus> {
    if enabled {
        set_last_error(format!(
            "{fn_name}: corrections.{field_name} is not supported: {reason}"
        ));
        Err(SidereonStatus::InvalidArgument)
    } else {
        Ok(())
    }
}

unsafe fn ppp_receiver_antenna_pcv_samples_from_c(
    fn_name: &str,
    arg_name: &str,
    calibration: &SidereonReceiverAntennaCalibration,
) -> Result<Vec<PppPcvSample>, SidereonStatus> {
    let (noazi_samples, azimuth_samples) =
        receiver_antenna_pcv_sample_slices_from_c(fn_name, arg_name, calibration)?;
    let sample_count = noazi_samples
        .len()
        .checked_add(azimuth_samples.len())
        .ok_or_else(|| {
            set_last_error(format!(
                "{fn_name}: {arg_name}.pcv_sample_count is too large"
            ));
            SidereonStatus::InvalidArgument
        })?;
    validate_element_count::<PppPcvSample>(
        fn_name,
        &format!("{arg_name}.pcv_sample_count"),
        sample_count,
    )?;
    let mut samples = Vec::with_capacity(sample_count);
    samples.extend(noazi_samples.iter().map(|sample| PppPcvSample {
        azimuth_deg: None,
        zenith_deg: sample.zenith_deg,
        value_m: sample.value_m,
    }));
    samples.extend(azimuth_samples.iter().map(|sample| PppPcvSample {
        azimuth_deg: Some(sample.azimuth_deg),
        zenith_deg: sample.zenith_deg,
        value_m: sample.value_m,
    }));
    Ok(samples)
}

unsafe fn receiver_antenna_pcv_sample_slices_from_c<'a>(
    fn_name: &str,
    arg_name: &str,
    calibration: &'a SidereonReceiverAntennaCalibration,
) -> Result<
    (
        &'a [SidereonReceiverAntennaNoaziPcvSample],
        &'a [SidereonReceiverAntennaAzimuthPcvSample],
    ),
    SidereonStatus,
> {
    let noazi_samples = require_slice(
        calibration.noazi_pcv_m,
        calibration.noazi_pcv_count,
        fn_name,
        &format!("{arg_name}.noazi_pcv_m"),
    )?;
    let azimuth_samples = require_slice(
        calibration.azimuth_pcv_m,
        calibration.azimuth_pcv_count,
        fn_name,
        &format!("{arg_name}.azimuth_pcv_m"),
    )?;
    Ok((noazi_samples, azimuth_samples))
}

unsafe fn rtk_receiver_antenna_calibration_from_c(
    fn_name: &str,
    arg_name: &str,
    calibration: &SidereonReceiverAntennaCalibration,
) -> Result<RtkReceiverAntennaCalibrationInner, SidereonStatus> {
    let (noazi_samples, azimuth_samples) =
        receiver_antenna_pcv_sample_slices_from_c(fn_name, arg_name, calibration)?;
    Ok(RtkReceiverAntennaCalibrationInner {
        pco_neu_m: calibration.pco_neu_m,
        noazi_pcv_m: noazi_samples
            .iter()
            .map(|sample| (sample.zenith_deg, sample.value_m))
            .collect(),
        azi_pcv_m: azimuth_samples
            .iter()
            .map(|sample| (sample.azimuth_deg, sample.zenith_deg, sample.value_m))
            .collect(),
    })
}

unsafe fn rtk_receiver_antenna_from_c(
    fn_name: &str,
    corrections: *const SidereonRtkReceiverAntennaCorrections,
) -> Result<Option<RtkReceiverAntennaCorrectionsInner>, SidereonStatus> {
    let Some(corrections) = corrections.as_ref() else {
        return Ok(None);
    };
    Ok(Some(RtkReceiverAntennaCorrectionsInner {
        base: rtk_receiver_antenna_calibration_from_c(
            fn_name,
            "receiver_antenna.base",
            &corrections.base,
        )?,
        rover: rtk_receiver_antenna_calibration_from_c(
            fn_name,
            "receiver_antenna.rover",
            &corrections.rover,
        )?,
    }))
}

unsafe fn ppp_receiver_antenna_frequency_from_c(
    fn_name: &str,
    arg_name: &str,
    label: *const c_char,
    calibration: &SidereonReceiverAntennaCalibration,
) -> Result<PppReceiverAntennaFrequencyInner, SidereonStatus> {
    Ok(PppReceiverAntennaFrequencyInner {
        label: parse_bounded_c_string(
            fn_name,
            &format!("{arg_name}.label"),
            label,
            MAX_PPP_ANTENNA_FREQ_LABEL_BYTES,
        )?,
        pco_m: calibration.pco_neu_m,
        pcv_samples: ppp_receiver_antenna_pcv_samples_from_c(fn_name, arg_name, calibration)?,
    })
}

unsafe fn ppp_receiver_antenna_from_c(
    fn_name: &str,
    options: *const SidereonPppReceiverAntennaOptions,
) -> Result<Option<PppReceiverAntennaOptionsInner>, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(None);
    };
    let freq1 = ppp_receiver_antenna_frequency_from_c(
        fn_name,
        "corrections.receiver_antenna.freq1",
        options.freq1_label,
        &options.freq1,
    )?;
    let freq2 = ppp_receiver_antenna_frequency_from_c(
        fn_name,
        "corrections.receiver_antenna.freq2",
        options.freq2_label,
        &options.freq2,
    )?;
    Ok(Some(PppReceiverAntennaOptionsInner {
        freq1_label: freq1.label.clone(),
        freq1_hz: options.freq1_hz,
        freq2_label: freq2.label.clone(),
        freq2_hz: options.freq2_hz,
        frequencies: vec![freq1, freq2],
    }))
}

unsafe fn ppp_satellite_clock_from_c(
    fn_name: &str,
    records: *const SidereonPppSatelliteClockRecord,
    record_count: usize,
) -> Result<Option<SatelliteClockCorrections>, SidereonStatus> {
    let raw_records = require_slice(
        records,
        record_count,
        fn_name,
        "corrections.satellite_clock_records",
    )?;
    if raw_records.is_empty() {
        return Ok(None);
    }
    validate_element_count::<(f64, f64)>(
        fn_name,
        "corrections.satellite_clock_record_count",
        raw_records.len(),
    )?;
    let mut series: BTreeMap<GnssSatelliteId, Vec<(f64, f64)>> = BTreeMap::new();
    for record in raw_records {
        let sat = parse_satellite_token(fn_name, record.sat_id)?;
        series
            .entry(sat)
            .or_default()
            .push((record.gps_seconds, record.clock_s));
    }
    for records in series.values_mut() {
        records.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    Ok(Some(SatelliteClockCorrections { series }))
}

fn ppp_fixed_ambiguity_rows_to_c(
    fn_name: &str,
    cycles: &BTreeMap<String, i64>,
    meters: &BTreeMap<String, f64>,
) -> Result<Vec<SidereonPppFixedAmbiguity>, SidereonStatus> {
    cycles
        .iter()
        .map(|(id, cycles)| {
            let value_m = meters
                .get(id)
                .copied()
                .ok_or_else(|| missing_fixed_ambiguity_meter(fn_name, id))?;
            Ok(SidereonPppFixedAmbiguity {
                id: ppp_id_token(id),
                cycles: *cycles,
                value_m,
            })
        })
        .collect()
}

fn fixed_c_chars<const N: usize>(text: &str) -> [c_char; N] {
    let mut out = [0; N];
    if N == 0 {
        return out;
    }
    for (idx, byte) in text.bytes().take(N - 1).enumerate() {
        out[idx] = byte as c_char;
    }
    out
}

fn tle_line_from_c(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
) -> Result<String, SidereonStatus> {
    unsafe { parse_bounded_c_string(fn_name, arg_name, ptr, MAX_TLE_LINE_BYTES) }
}

fn tle_ops_mode_from_c(fn_name: &str, opsmode: u32) -> Result<OpsMode, SidereonStatus> {
    match opsmode {
        value if value == SidereonTleOpsMode::Afspc as u32 => Ok(OpsMode::Afspc),
        value if value == SidereonTleOpsMode::Improved as u32 => Ok(OpsMode::Improved),
        _ => {
            set_last_error(format!("{fn_name}: invalid TLE opsmode selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn propagation_force_model_from_c(
    fn_name: &str,
    force_model: u32,
) -> Result<PropagationForceModel, SidereonStatus> {
    match force_model {
        value if value == SidereonPropagationForceModel::TwoBody as u32 => {
            Ok(PropagationForceModel::TwoBody)
        }
        value if value == SidereonPropagationForceModel::TwoBodyJ2 as u32 => {
            Ok(PropagationForceModel::TwoBodyJ2)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid force_model selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn propagation_force_model_kind_from_c(
    fn_name: &str,
    config: &SidereonStatePropagationConfig,
) -> Result<ForceModelKind, SidereonStatus> {
    match config.force_model {
        value if value == SidereonPropagationForceModel::TwoBody as u32 => {
            Ok(ForceModelKind::TwoBody {
                mu_km3_s2: if config.mu_km3_s2_enabled {
                    config.mu_km3_s2
                } else {
                    MU_EARTH
                },
            })
        }
        value if value == SidereonPropagationForceModel::TwoBodyJ2 as u32 => {
            Ok(ForceModelKind::TwoBodyJ2 {
                mu_km3_s2: if config.mu_km3_s2_enabled {
                    config.mu_km3_s2
                } else {
                    MU_EARTH
                },
                re_km: sidereon_core::astro::constants::RE_EARTH,
                j2: sidereon_core::astro::constants::J2_EARTH,
            })
        }
        value if value == SidereonPropagationForceModel::Composite as u32 => {
            composite_force_model_from_c(fn_name, config)
        }
        value if value == SidereonPropagationForceModel::EarthPhaseA as u32 => {
            let srp = if config.force_components.has_solar_radiation_pressure {
                Some(solar_radiation_pressure_from_c(
                    fn_name,
                    config.force_components.solar_radiation_pressure,
                )?)
            } else {
                None
            };
            Ok(ForceModelKind::earth_phase_a(srp))
        }
        value if value == SidereonPropagationForceModel::EarthPhaseB as u32 => {
            let srp = if config.force_components.has_solar_radiation_pressure {
                Some(solar_radiation_pressure_from_c(
                    fn_name,
                    config.force_components.solar_radiation_pressure,
                )?)
            } else {
                None
            };
            let max_degree = u16::try_from(config.force_components.spherical_harmonic_max_degree)
                .map_err(|_| {
                set_last_error(format!(
                    "{fn_name}: spherical_harmonic_max_degree is out of range"
                ));
                SidereonStatus::InvalidArgument
            })?;
            let max_order = u16::try_from(config.force_components.spherical_harmonic_max_order)
                .map_err(|_| {
                    set_last_error(format!(
                        "{fn_name}: spherical_harmonic_max_order is out of range"
                    ));
                    SidereonStatus::InvalidArgument
                })?;
            ForceModelKind::earth_phase_b(max_degree, max_order, srp).map_err(|err| {
                set_last_error(format!("{fn_name}: {err}"));
                SidereonStatus::InvalidArgument
            })
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid force_model selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn propagation_integrator_from_c(
    fn_name: &str,
    integrator: u32,
) -> Result<IntegratorKind, SidereonStatus> {
    match integrator {
        value if value == SidereonPropagationIntegrator::Dp54 as u32 => Ok(IntegratorKind::Dp54),
        value if value == SidereonPropagationIntegrator::Rk4 as u32 => Ok(IntegratorKind::Rk4),
        _ => {
            set_last_error(format!("{fn_name}: invalid integrator selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn space_weather_to_c(value: SpaceWeather) -> SidereonSpaceWeather {
    SidereonSpaceWeather {
        f107: value.f107,
        f107a: value.f107a,
        ap: value.ap,
    }
}

fn space_weather_from_c(value: SidereonSpaceWeather) -> SpaceWeather {
    SpaceWeather {
        f107: value.f107,
        f107a: value.f107a,
        ap: value.ap,
    }
}

fn drag_parameters_to_c(value: DragParameters) -> SidereonDragParameters {
    SidereonDragParameters {
        bc_factor_m2_kg: value.bc_factor_m2_kg(),
        space_weather: space_weather_to_c(value.space_weather()),
        cutoff_altitude_km: value.cutoff_altitude_km(),
    }
}

fn drag_parameters_from_c(
    fn_name: &str,
    value: SidereonDragParameters,
) -> Result<DragParameters, SidereonStatus> {
    DragParameters::from_bc_factor_m2_kg(
        value.bc_factor_m2_kg,
        space_weather_from_c(value.space_weather),
        value.cutoff_altitude_km,
    )
    .map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn state_propagator_from_c(
    fn_name: &str,
    config: &SidereonStatePropagationConfig,
) -> Result<StatePropagator, SidereonStatus> {
    if config.initial_step_s <= 0.0 {
        set_last_error(format!("{fn_name}: initial_step_s must be positive"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let force_model = propagation_force_model_kind_from_c(fn_name, config)?;
    let integrator = propagation_integrator_from_c(fn_name, config.integrator)?;
    let options = IntegratorOptions {
        abs_tol: config.abs_tol,
        rel_tol: config.rel_tol,
        initial_step: config.initial_step_s,
        min_step: config.min_step_s,
        max_step: config.max_step_s,
        max_steps: config.max_steps,
        dense_output: false,
    };
    let drag = if config.has_drag {
        Some(drag_parameters_from_c(fn_name, config.drag)?)
    } else {
        None
    };
    Ok(StatePropagator {
        initial: CartesianState::new(config.epoch_s, config.position_km, config.velocity_km_s),
        force_model,
        integrator,
        options,
        drag,
        space_weather: None,
    })
}

fn propagation_context_from_c(config: &SidereonStatePropagationConfig) -> PropagationContext {
    if propagation_force_model_needs_body_fixed_frame(config) {
        PropagationContext::new()
            .with_body_fixed_frame_provider(Arc::new(TdbEarthOrientationProvider::new()))
    } else {
        PropagationContext::default()
    }
}

fn propagation_force_model_needs_body_fixed_frame(config: &SidereonStatePropagationConfig) -> bool {
    config.force_model == SidereonPropagationForceModel::EarthPhaseB as u32
        || (config.force_model == SidereonPropagationForceModel::Composite as u32
            && (config.force_components.has_spherical_harmonic
                || config.force_components.has_solid_earth_tide
                || config.force_components.has_solid_earth_pole_tide))
}

fn composite_force_model_from_c(
    fn_name: &str,
    config: &SidereonStatePropagationConfig,
) -> Result<ForceModelKind, SidereonStatus> {
    let components = config.force_components;
    let effective_mu = if config.mu_km3_s2_enabled {
        config.mu_km3_s2
    } else {
        MU_EARTH
    };
    let two_body_mu = if components.two_body_mu_km3_s2_enabled {
        components.two_body_mu_km3_s2
    } else {
        effective_mu
    };

    if components.has_two_body
        && components.has_zonal
        && components.zonal_max_degree == 2
        && !components.has_spherical_harmonic
        && !components.has_solid_earth_tide
        && !components.has_solid_earth_pole_tide
        && !components.has_third_body
        && !components.has_solar_radiation_pressure
        && !components.has_relativity
    {
        return Ok(ForceModelKind::TwoBodyJ2 {
            mu_km3_s2: two_body_mu,
            re_km: sidereon_core::astro::constants::RE_EARTH,
            j2: sidereon_core::astro::constants::J2_EARTH,
        });
    }

    let mut force_components = ForceModelComponents::default();
    if components.has_two_body {
        force_components = force_components.with_two_body_mu(two_body_mu);
    }
    if components.has_zonal {
        let max_degree = u8::try_from(components.zonal_max_degree).map_err(|_| {
            set_last_error(format!("{fn_name}: zonal_max_degree must be in 2..=6"));
            SidereonStatus::InvalidArgument
        })?;
        let degrees = ZonalDegrees::through(max_degree).map_err(|err| {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::InvalidArgument
        })?;
        force_components = force_components.with_zonal(ZonalGravity::new(
            effective_mu,
            sidereon_core::astro::constants::RE_EARTH,
            degrees,
            ZonalCoefficients::default(),
        ));
    }
    if components.has_spherical_harmonic {
        if components.has_zonal {
            set_last_error(format!(
                "{fn_name}: zonal and spherical_harmonic gravity cannot both be enabled"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        let max_degree = u16::try_from(components.spherical_harmonic_max_degree).map_err(|_| {
            set_last_error(format!(
                "{fn_name}: spherical_harmonic_max_degree is out of range"
            ));
            SidereonStatus::InvalidArgument
        })?;
        let max_order = u16::try_from(components.spherical_harmonic_max_order).map_err(|_| {
            set_last_error(format!(
                "{fn_name}: spherical_harmonic_max_order is out of range"
            ));
            SidereonStatus::InvalidArgument
        })?;
        let spherical_harmonic = SphericalHarmonicGravityConfig::earth(max_degree, max_order)
            .map_err(|err| {
                set_last_error(format!("{fn_name}: {err}"));
                SidereonStatus::InvalidArgument
            })?;
        force_components = force_components.with_spherical_harmonic(spherical_harmonic);
    }
    if components.has_solid_earth_tide {
        force_components = force_components.with_solid_earth_tide(SolidEarthTideGravity::default());
    }
    if components.has_solid_earth_pole_tide {
        force_components =
            force_components.with_solid_earth_pole_tide(SolidEarthPoleTideGravity::default());
    }
    if components.has_third_body {
        force_components = force_components.with_third_body(ThirdBodyGravity {
            bodies: ThirdBodyBodies {
                sun: components.third_body_sun,
                moon: components.third_body_moon,
            },
            ..ThirdBodyGravity::default()
        });
    }
    if components.has_solar_radiation_pressure {
        force_components = force_components.with_solar_radiation_pressure(
            solar_radiation_pressure_from_c(fn_name, components.solar_radiation_pressure)?,
        );
    }
    if components.has_relativity {
        force_components = force_components.with_relativity(SchwarzschildRelativity::default());
    }

    Ok(ForceModelKind::composite(force_components))
}

fn solar_radiation_pressure_from_c(
    fn_name: &str,
    value: SidereonSolarRadiationPressure,
) -> Result<SolarRadiationPressure, SidereonStatus> {
    SolarRadiationPressure::new(value.cr, value.area_to_mass_m2_kg).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn default_pass_finder_options() -> SidereonPassFinderOptions {
    let options = PassFinderOptions::default();
    SidereonPassFinderOptions {
        elevation_mask_deg: options.elevation_mask_deg,
        step_seconds: options.coarse_step_seconds,
        time_tolerance_s: options.time_tolerance_seconds,
    }
}

fn pass_finder_options_from_c(
    fn_name: &str,
    options: *const SidereonPassFinderOptions,
) -> Result<PassFinderOptions, SidereonStatus> {
    let options = match unsafe { options.as_ref() } {
        Some(options) => *options,
        None => default_pass_finder_options(),
    };
    if !options.elevation_mask_deg.is_finite() {
        set_last_error(format!("{fn_name}: elevation_mask_deg must be finite"));
        return Err(SidereonStatus::InvalidArgument);
    }
    if !options.step_seconds.is_finite() || options.step_seconds <= 0.0 {
        set_last_error(format!(
            "{fn_name}: step_seconds must be positive and finite"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    if !options.time_tolerance_s.is_finite() || options.time_tolerance_s <= 0.0 {
        set_last_error(format!(
            "{fn_name}: time_tolerance_s must be positive and finite"
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    Ok(PassFinderOptions {
        elevation_mask_deg: options.elevation_mask_deg,
        coarse_step_seconds: options.step_seconds,
        time_tolerance_seconds: options.time_tolerance_s,
    })
}

fn ground_station_from_c(station: &SidereonGroundStation) -> GroundStation {
    GroundStation {
        latitude_deg: station.latitude_deg,
        longitude_deg: station.longitude_deg,
        altitude_m: station.altitude_m,
    }
}

unsafe fn unix_instants_from_c(
    fn_name: &str,
    epochs_unix_us: *const i64,
    epoch_count: usize,
) -> Result<Vec<UtcInstant>, SidereonStatus> {
    let raw_epochs = require_slice(epochs_unix_us, epoch_count, fn_name, "epochs_unix_us")?;
    validate_element_count::<UtcInstant>(fn_name, "epoch_count", raw_epochs.len())?;
    Ok(raw_epochs
        .iter()
        .copied()
        .map(UtcInstant::from_unix_microseconds)
        .collect())
}

unsafe fn times_from_c<'a>(
    fn_name: &str,
    times_s: *const f64,
    time_count: usize,
) -> Result<&'a [f64], SidereonStatus> {
    let times = require_slice(times_s, time_count, fn_name, "times_s")?;
    Ok(times)
}

fn map_pass_error(fn_name: &str, err: PassError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        PassError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
    }
}

fn map_frame_transform_error(fn_name: &str, err: FrameTransformError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        FrameTransformError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
    }
}

fn look_angle_to_c(look: &LookAngle) -> SidereonLookAngle {
    SidereonLookAngle {
        azimuth_deg: look.azimuth_deg,
        elevation_deg: look.elevation_deg,
        range_km: look.range_km,
    }
}

fn satellite_pass_to_c(pass: &sidereon::passes::SatellitePass) -> SidereonSatellitePass {
    let aos_unix_us = pass.aos.unix_microseconds();
    let los_unix_us = pass.los.unix_microseconds();
    SidereonSatellitePass {
        aos_unix_us,
        los_unix_us,
        culmination_unix_us: pass.culmination.unix_microseconds(),
        max_elevation_deg: pass.max_elevation_deg,
        duration_s: (los_unix_us - aos_unix_us) as f64 / MICROSECONDS_PER_SECOND,
    }
}

fn geodetic_to_c(geodetic: &Wgs84Geodetic) -> SidereonGeodetic {
    SidereonGeodetic {
        lat_rad: geodetic.lat_rad,
        lon_rad: geodetic.lon_rad,
        height_m: geodetic.height_m,
    }
}

fn cartesian_state_to_c(state: &CartesianState) -> SidereonCartesianState {
    SidereonCartesianState {
        epoch_s: state.epoch_tdb_seconds,
        position_km: state.position_array(),
        velocity_km_s: state.velocity_array(),
    }
}

fn cartesian_state_from_c(state: &SidereonCartesianState) -> CartesianState {
    CartesianState::new(state.epoch_s, state.position_km, state.velocity_km_s)
}

fn unwrap_prediction_batch(
    fn_name: &str,
    results: Vec<Result<Vec<Prediction>, Sgp4Error>>,
) -> Result<Vec<Vec<Prediction>>, SidereonStatus> {
    results
        .into_iter()
        .enumerate()
        .map(|(idx, arc)| {
            arc.map_err(|err| {
                set_last_error(format!("{fn_name}: satellite {idx}: {err}"));
                SidereonStatus::Solve
            })
        })
        .collect()
}

fn sp3_merge_flag_slice<'a>(
    fn_name: &str,
    report: &'a MergeReport,
    kind: u32,
) -> Result<&'a [MergeFlag], SidereonStatus> {
    match kind {
        value if value == SidereonSp3MergeFlagKind::Quarantined as u32 => Ok(&report.quarantined),
        value if value == SidereonSp3MergeFlagKind::SingleSource as u32 => {
            Ok(&report.single_source)
        }
        value if value == SidereonSp3MergeFlagKind::PositionOutlier as u32 => {
            Ok(&report.position_outliers)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid merge report flag kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// One record from a parsed multi-record TLE file: the name line (empty for a
/// bare 2-line set) and the initialized TLE handle for that element set.
struct SidereonTleFileRecord {
    name: String,
    tle: SidereonTle,
}

/// Fixed-size null-terminated satellite token storage. The token is valid up
/// to the first null byte; for example G08. Values returned by Sidereon are
/// always null-terminated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteToken {
    /// Null-terminated token bytes.
    pub bytes: [c_char; 17],
}

/// One ECI Cartesian state from numerical propagation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonCartesianState {
    /// Output epoch in absolute TDB seconds.
    pub epoch_s: f64,
    /// ECI position in kilometers.
    pub position_km: [f64; 3],
    /// ECI velocity in kilometers per second.
    pub velocity_km_s: [f64; 3],
}

/// GNSS constellation using the standard RINEX single-letter systems.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonGnssSystem {
    /// GPS.
    Gps = 0,
    /// GLONASS.
    Glonass = 1,
    /// Galileo.
    Galileo = 2,
    /// BeiDou.
    BeiDou = 3,
    /// QZSS.
    Qzss = 4,
    /// NavIC / IRNSS.
    Navic = 5,
    /// SBAS.
    Sbas = 6,
}

/// A time scale, tagging the system a time reading is expressed in. Pass as a
/// uint32_t to the inter-system offset helpers.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonTimeScale {
    /// Coordinated Universal Time.
    Utc = 0,
    /// International Atomic Time.
    Tai = 1,
    /// Terrestrial Time.
    Tt = 2,
    /// Barycentric Dynamical Time.
    Tdb = 3,
    /// GPS time.
    Gpst = 4,
    /// Galileo System Time.
    Gst = 5,
    /// BeiDou Time.
    Bdt = 6,
    /// GLONASS system time (UTC(SU)-based; leap-second dependent).
    Glonasst = 7,
    /// QZSS system time (steered to GPST).
    Qzsst = 8,
    /// Geocentric Coordinate Time.
    Tcg = 9,
    /// Barycentric Coordinate Time.
    Tcb = 10,
}

const _: () = assert!(
    SIDEREON_PPP_VMF_SITE_MAX_SAMPLES == PPP_VMF_SITE_MAX_SAMPLES,
    "SIDEREON_PPP_VMF_SITE_MAX_SAMPLES must match the engine VMF_SITE_MAX_SAMPLES"
);

/// Geodetic receiver position in radians and meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeodetic {
    /// Geodetic latitude in radians.
    pub lat_rad: f64,
    /// Geodetic longitude in radians.
    pub lon_rad: f64,
    /// Ellipsoidal height above WGS84 in meters.
    pub height_m: f64,
}

/// A position in the ITRF / IGS-realization ECEF frame, in meters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonItrfPosition {
    /// ECEF X coordinate in meters.
    pub x_m: f64,
    /// ECEF Y coordinate in meters.
    pub y_m: f64,
    /// ECEF Z coordinate in meters.
    pub z_m: f64,
}

/// Dilution-of-precision scalars from the converged geometry: the SPP solution's
/// geometry-covariance summary. Mirrors the engine DOP result.
#[repr(C)]
pub struct SidereonDop {
    /// Geometric DOP.
    pub gdop: f64,
    /// Position DOP.
    pub pdop: f64,
    /// Horizontal DOP.
    pub hdop: f64,
    /// Vertical DOP.
    pub vdop: f64,
    /// Time (clock) DOP.
    pub tdop: f64,
}

/// Copy the current thread's last error message into buf as a null-terminated
/// C string. Returns the number of bytes (excluding the terminator) the full
/// message needs; if that is greater than or equal to len, the message was
/// truncated, but the buffer is still null-terminated when len is greater than
/// zero. Pass len 0 to query the length.
///
/// Safety: if len is nonzero, buf must point to at least len writable bytes.
/// When len is zero, buf may be NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_error_message(buf: *mut c_char, len: usize) -> usize {
    ffi_boundary("sidereon_last_error_message", 0, || {
        if !buf.is_null() && len > 0 {
            *buf = 0;
        }
        LAST_ERROR.with(|slot| {
            let borrow = slot.borrow();
            let bytes = borrow.as_ref().map(|s| s.as_bytes()).unwrap_or(b"");
            let needed = bytes.len();
            if len == 0 || buf.is_null() {
                return needed;
            }
            // Leave room for the terminator.
            let copy = needed.min(len - 1);
            ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buf, copy);
            *buf.add(copy) = 0;
            needed
        })
    })
}

/// Return a static, null-terminated, human-readable name for a status code.
///
/// The pointer refers to static storage and is valid for the lifetime of the
/// program; do not free it. Unlike sidereon_last_error_message, which gives the
/// specific reason for the most recent failure, this maps the status enum itself
/// to a fixed string and never depends on thread-local state. An unrecognized
/// value yields "unknown status".
#[no_mangle]
pub extern "C" fn sidereon_status_message(status: SidereonStatus) -> *const c_char {
    let text: &CStr = match status {
        SidereonStatus::Ok => c"ok",
        SidereonStatus::NullPointer => c"null pointer argument",
        SidereonStatus::InvalidArgument => c"invalid argument",
        SidereonStatus::InvalidToken => c"invalid UTF-8 or satellite token",
        SidereonStatus::Sp3Parse => c"SP3 parse error",
        SidereonStatus::Solve => c"solve failed",
        SidereonStatus::Panic => c"internal panic contained at the FFI boundary",
    };
    text.as_ptr()
}

/// Write the binding's semantic version components to the out-parameters. Any of
/// out_major, out_minor, or out_patch may be NULL to skip that component. These
/// match the SIDEREON_VERSION_* compile-time macros in this header.
///
/// Safety: each non-null out pointer must point to a writable uint32_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_version(
    out_major: *mut u32,
    out_minor: *mut u32,
    out_patch: *mut u32,
) {
    if !out_major.is_null() {
        *out_major = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0);
    }
    if !out_minor.is_null() {
        *out_minor = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0);
    }
    if !out_patch.is_null() {
        *out_patch = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0);
    }
}

/// Return the binding's version as a static "MAJOR.MINOR.PATCH" string. The
/// pointer refers to static storage and must not be freed.
#[no_mangle]
pub extern "C" fn sidereon_version_string() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

fn empty_dop() -> SidereonDop {
    SidereonDop {
        gdop: 0.0,
        pdop: 0.0,
        hdop: 0.0,
        vdop: 0.0,
        tdop: 0.0,
    }
}

fn dop_to_c(dop: Dop) -> SidereonDop {
    SidereonDop {
        gdop: dop.gdop,
        pdop: dop.pdop,
        hdop: dop.hdop,
        vdop: dop.vdop,
        tdop: dop.tdop,
    }
}

/// Map a DOP error to a status code. Malformed inputs report
/// SIDEREON_STATUS_INVALID_ARGUMENT; a rank-deficient or singular geometry
/// (well-formed inputs the engine simply cannot turn into a finite DOP) reports
/// SIDEREON_STATUS_SOLVE.
fn map_dop_error(fn_name: &str, err: DopError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        DopError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
        DopError::TooFewSatellites | DopError::Singular => SidereonStatus::Solve,
    }
}

// === ANTEX antenna PCO/PCV ==================================================

/// Maximum byte length accepted for an ANTEX `TYPE / SERIAL` antenna id.
const MAX_ANTEX_ID_BYTES: usize = 256;
/// Maximum byte length accepted for an ANTEX frequency code (e.g. `G01`).
const MAX_ANTEX_FREQUENCY_BYTES: usize = 32;

/// Map an ANTEX parse or lookup error to a status code. Every ANTEX error is a
/// malformed-input condition, so they all report SIDEREON_STATUS_INVALID_ARGUMENT
/// with the detail available via sidereon_last_error_message.
fn map_antex_error(fn_name: &str, err: AntexError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn velocity_observable_from_c(
    fn_name: &str,
    arg_name: &str,
    observable: u32,
) -> Result<VelocityObservable, SidereonStatus> {
    match observable {
        value if value == SidereonVelocityObservable::RangeRate as u32 => {
            Ok(VelocityObservable::RangeRate)
        }
        value if value == SidereonVelocityObservable::Doppler as u32 => {
            Ok(VelocityObservable::Doppler)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} velocity observable"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

// === Ionosphere: standalone Klobuchar + IONEX =============================

/// Degrees-to-radians as a single rounded constant pi/180, so the boundary
/// conversion is one multiply and one rounding. This matches the goldens'
/// `math.radians` exactly (and the Python/Elixir bindings), keeping the IONEX
/// slant delay bit-exact when callers pass degrees.
const IONO_DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

fn rinex_epoch_time_to_c(
    epoch: sidereon_core::rinex::observations::ObsEpochTime,
) -> SidereonCalendarEpoch {
    SidereonCalendarEpoch {
        year: epoch.year,
        month: i32::from(epoch.month),
        day: i32::from(epoch.day),
        hour: i32::from(epoch.hour),
        minute: i32::from(epoch.minute),
        second: epoch.second,
    }
}

enum AllanSeriesStorage {
    PhaseSeconds(Vec<f64>),
    FractionalFrequency(Vec<f64>),
    PhaseSecondsWithGaps(Vec<Option<f64>>),
    FractionalFrequencyWithGaps(Vec<Option<f64>>),
}

impl AllanSeriesStorage {
    fn as_series(&self) -> AllanSeries<'_> {
        match self {
            Self::PhaseSeconds(values) => AllanSeries::PhaseSeconds(values),
            Self::FractionalFrequency(values) => AllanSeries::FractionalFrequency(values),
            Self::PhaseSecondsWithGaps(values) => AllanSeries::PhaseSecondsWithGaps(values),
            Self::FractionalFrequencyWithGaps(values) => {
                AllanSeries::FractionalFrequencyWithGaps(values)
            }
        }
    }
}

fn allan_series_kind_from_c(
    fn_name: &str,
    kind: u32,
) -> Result<SidereonAllanSeriesKind, SidereonStatus> {
    match kind {
        value if value == SidereonAllanSeriesKind::PhaseSeconds as u32 => {
            Ok(SidereonAllanSeriesKind::PhaseSeconds)
        }
        value if value == SidereonAllanSeriesKind::FractionalFrequency as u32 => {
            Ok(SidereonAllanSeriesKind::FractionalFrequency)
        }
        value if value == SidereonAllanSeriesKind::PhaseSecondsWithGaps as u32 => {
            Ok(SidereonAllanSeriesKind::PhaseSecondsWithGaps)
        }
        value if value == SidereonAllanSeriesKind::FractionalFrequencyWithGaps as u32 => {
            Ok(SidereonAllanSeriesKind::FractionalFrequencyWithGaps)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid Allan series kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

struct ByteCopyOut {
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
}

fn const_record_to_c(record: &ConstRecord) -> SidereonConstellationRecord {
    SidereonConstellationRecord {
        system: gnss_system_to_c(record.system),
        prn: record.prn,
        svn_present: record.svn.is_some(),
        svn: record.svn.unwrap_or(0),
        norad_id: record.norad_id,
        fdma_channel_present: record.fdma_channel.is_some(),
        fdma_channel: record.fdma_channel.unwrap_or(0),
        active: record.active,
        usable: record.usable,
    }
}

// === Lenient OMM catalog (CelesTrak combined feed) =========================
//
// Wraps sidereon_core::constellation::from_celestrak_omm_lenient: build a
// single-system identity catalog from a raw combined CelesTrak OMM/JSON feed
// without aborting on entries that do not resolve to the requested system.
// Unlike sidereon_constellation_build (which fails on the first unresolvable
// name), this keeps both the resolved Records and the skipped (object_name,
// norad_id) identities so the caller can triage a mixed `gnss` feed. The catalog
// is an opaque handle; records are read back as SidereonConstellationRecord and
// skipped entries through the accessors below.

fn degradation_kind_to_c(kind: DegradationKind) -> SidereonDegradationKind {
    match kind {
        DegradationKind::Exact => SidereonDegradationKind::Exact,
        DegradationKind::NearestPrior => SidereonDegradationKind::NearestPrior,
        DegradationKind::DiurnalShift => SidereonDegradationKind::DiurnalShift,
    }
}

fn staleness_metadata_to_c(metadata: StalenessMetadata) -> SidereonStalenessMetadata {
    SidereonStalenessMetadata {
        kind: degradation_kind_to_c(metadata.kind),
        requested_epoch_j2000_s: metadata.requested_epoch_j2000_s,
        source_epoch_j2000_s: metadata.source_epoch_j2000_s,
        staleness_s: metadata.staleness_s,
        staleness_days: metadata.staleness_days,
    }
}

fn empty_staleness_metadata() -> SidereonStalenessMetadata {
    SidereonStalenessMetadata {
        kind: SidereonDegradationKind::Exact,
        requested_epoch_j2000_s: 0.0,
        source_epoch_j2000_s: 0.0,
        staleness_s: 0.0,
        staleness_days: 0.0,
    }
}

/// Map a typed selection error to its C status without touching last_error.
/// Use this when the selection error is provenance to be surfaced on a
/// successful readout, not the cause of a failing return.
fn selection_error_to_status(err: &SelectionError) -> SidereonSelectionStatus {
    match err {
        SelectionError::EmptyProductSet => SidereonSelectionStatus::EmptyProductSet,
        SelectionError::InvalidRange { .. } => SidereonSelectionStatus::InvalidRange,
        SelectionError::NoPriorProduct { .. } => SidereonSelectionStatus::NoPriorProduct,
        SelectionError::BeyondStalenessCap { .. } => SidereonSelectionStatus::BeyondStalenessCap,
        SelectionError::InvalidProduct(_) => SidereonSelectionStatus::InvalidProduct,
        SelectionError::InvalidPolicy { .. } => SidereonSelectionStatus::InvalidPolicy,
        SelectionError::Overflow { .. } => SidereonSelectionStatus::Overflow,
    }
}

/// Map a typed selection error to its C status on a failing return, recording
/// its Display (which carries the structured detail) for
/// sidereon_last_error_message.
fn map_selection_error(fn_name: &str, err: &SelectionError) -> SidereonSelectionStatus {
    set_last_error(format!("{fn_name}: {err}"));
    selection_error_to_status(err)
}

/// Map a marshaling status onto the selection status surface.
fn marshal_status_to_selection(status: SidereonStatus) -> SidereonSelectionStatus {
    match status {
        SidereonStatus::Ok => SidereonSelectionStatus::Ok,
        SidereonStatus::NullPointer => SidereonSelectionStatus::NullPointer,
        SidereonStatus::InvalidToken => SidereonSelectionStatus::InvalidToken,
        SidereonStatus::Panic => SidereonSelectionStatus::Panic,
        SidereonStatus::InvalidArgument | SidereonStatus::Sp3Parse | SidereonStatus::Solve => {
            SidereonSelectionStatus::InvalidArgument
        }
    }
}

/// Map a marshaling status onto the fallback status surface.
fn marshal_status_to_fallback(status: SidereonStatus) -> SidereonFallbackStatus {
    match status {
        SidereonStatus::Ok => SidereonFallbackStatus::Ok,
        SidereonStatus::NullPointer => SidereonFallbackStatus::NullPointer,
        SidereonStatus::InvalidToken => SidereonFallbackStatus::InvalidToken,
        SidereonStatus::Panic => SidereonFallbackStatus::Panic,
        SidereonStatus::InvalidArgument | SidereonStatus::Sp3Parse | SidereonStatus::Solve => {
            SidereonFallbackStatus::InvalidArgument
        }
    }
}

/// Collect an array of SP3 handle pointers into an owned product slice the engine
/// can select over. A zero count yields an empty set (so the engine reports
/// EmptyProductSet), matching the core contract.
unsafe fn sp3_products_from_c(
    fn_name: &str,
    products: *const *const SidereonSp3,
    product_count: usize,
) -> Result<Vec<Sp3>, SidereonStatus> {
    let raw = require_slice(products, product_count, fn_name, "products")?;
    let mut set = Vec::with_capacity(raw.len());
    for (idx, &handle_ptr) in raw.iter().enumerate() {
        let handle = require_ref(handle_ptr, fn_name, &format!("products[{idx}]"))?;
        set.push(handle.inner.clone());
    }
    Ok(set)
}

/// Decode a UTF-8 byte buffer to text, recording a thread-local error and
/// returning the InvalidToken status on failure. Shared by the OEM/OPM readers.
unsafe fn ndm_text_from_utf8<'a>(
    data: *const u8,
    len: usize,
    fn_name: &str,
) -> Result<&'a str, SidereonStatus> {
    let bytes = require_slice(data, len, fn_name, "data")?;
    match str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(_) => {
            set_last_error(format!("{fn_name}: data is not valid UTF-8"));
            Err(SidereonStatus::InvalidToken)
        }
    }
}

fn guard_dop<T>(
    fn_name: &str,
    body: impl FnOnce() -> Result<T, DopError>,
) -> Result<T, SidereonStatus> {
    body().map_err(|err| map_dop_error(fn_name, err))
}

fn map_observables_error(fn_name: &str, err: ObservablesError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        ObservablesError::InvalidInput { .. } | ObservablesError::Media(_) => {
            SidereonStatus::InvalidArgument
        }
        ObservablesError::NoEphemeris | ObservablesError::Ephemeris(_) => SidereonStatus::Solve,
    }
}

fn predicted_observables_to_c(obs: &PredictedObservables) -> SidereonPredictedObservables {
    SidereonPredictedObservables {
        geometric_range_m: obs.geometric_range_m,
        range_rate_m_s: obs.range_rate_m_s,
        doppler_hz: obs.doppler_hz,
        has_sat_clock_s: obs.sat_clock_s.is_some(),
        sat_clock_s: obs.sat_clock_s.unwrap_or(0.0),
        elevation_deg: obs.elevation_deg,
        azimuth_deg: obs.azimuth_deg,
        transmit_offset_us: obs.transmit_offset_us,
        transmit_time_j2000_s: obs.transmit_time_j2000_s,
        los_unit: obs.los_unit,
        sat_pos_ecef_m: obs.sat_pos_ecef_m,
        sat_velocity_m_s: obs.sat_velocity_m_s,
    }
}

unsafe fn predict_options_from_c(
    fn_name: &str,
    options: *const SidereonObservablesOptions,
) -> Result<PredictOptions, SidereonStatus> {
    if options.is_null() {
        return Ok(PredictOptions::default());
    }
    let options = require_ref(options, fn_name, "options")?;
    Ok(PredictOptions {
        carrier_hz: options.carrier_hz,
        light_time: options.light_time,
        sagnac: options.sagnac,
    })
}

fn calendar_epoch_to_c(epoch: CalendarEpoch) -> SidereonCalendarEpoch {
    SidereonCalendarEpoch {
        year: epoch.year,
        month: epoch.month,
        day: epoch.day,
        hour: epoch.hour,
        minute: epoch.minute,
        second: epoch.second,
    }
}

fn calendar_epoch_from_c(epoch: &SidereonCalendarEpoch) -> CalendarEpoch {
    CalendarEpoch::new(
        epoch.year,
        epoch.month,
        epoch.day,
        epoch.hour,
        epoch.minute,
        epoch.second,
    )
}

fn reduced_orbit_model_to_c(model: ReducedOrbitModelInner) -> u32 {
    match model {
        ReducedOrbitModelInner::CircularSecular => {
            SidereonReducedOrbitModel::CircularSecular as u32
        }
        ReducedOrbitModelInner::EccentricSecular => {
            SidereonReducedOrbitModel::EccentricSecular as u32
        }
    }
}
// ============================================================================

/// Record `err` against `fn_name` and report it as an invalid-argument status.
/// Used for the input-validation error enums the math kernels return.
fn extra_invalid_arg(fn_name: &str, err: impl std::fmt::Display) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn gnss_week_tow_from_c(
    fn_name: &str,
    value: &SidereonGnssWeekTow,
) -> Result<GnssWeekTow, SidereonStatus> {
    let system = time_scale_from_c_code(fn_name, "value.system", value.system)?;
    GnssWeekTow::new(system, value.week, value.tow_s).map_err(|err| {
        set_last_error(format!("{fn_name}: {err}"));
        SidereonStatus::InvalidArgument
    })
}

fn pseudorange_variance_options_from_c(
    fn_name: &str,
    options: &SidereonPseudorangeVarianceOptions,
) -> Result<sidereon_core::quality::PseudorangeVarianceOptions, SidereonStatus> {
    let model = match options.model {
        v if v == SidereonPseudorangeVarianceModel::Elevation as u32 => {
            sidereon_core::quality::PseudorangeVarianceModel::Elevation
        }
        v if v == SidereonPseudorangeVarianceModel::ElevationCn0 as u32 => {
            sidereon_core::quality::PseudorangeVarianceModel::ElevationCn0
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid variance model"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    Ok(sidereon_core::quality::PseudorangeVarianceOptions {
        a_m: options.a_m,
        b_m: options.b_m,
        model,
        cn0_dbhz: options.has_cn0.then_some(options.cn0_dbhz),
        cn0_scale_m2: options.cn0_scale_m2,
    })
}

unsafe fn read_vec3(
    fn_name: &str,
    arg_name: &str,
    ptr: *const f64,
) -> Result<[f64; 3], SidereonStatus> {
    let slice = require_slice(ptr, 3, fn_name, arg_name)?;
    Ok([slice[0], slice[1], slice[2]])
}

struct SunMoonEpochBatchArgs {
    epochs_unix_us: *const i64,
    count: usize,
    out_sun_m: *mut f64,
    sun_len: usize,
    out_moon_m: *mut f64,
    moon_len: usize,
}

unsafe fn observable_state_common(
    fn_name: &str,
    source: &dyn sidereon_core::observables::ObservableEphemerisSource,
    satellite_id: *const c_char,
    t_j2000_s: f64,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
    out_has_clock: *mut bool,
) -> SidereonStatus {
    let out_clock_s = match require_out(out_clock_s, fn_name, "out_clock_s") {
        Ok(p) => p,
        Err(status) => return status,
    };
    *out_clock_s = 0.0;
    let out_has_clock = match require_out(out_has_clock, fn_name, "out_has_clock") {
        Ok(p) => p,
        Err(status) => return status,
    };
    *out_has_clock = false;
    if let Err(status) = copy_exact_f64s(
        fn_name,
        "out_position_ecef_m",
        out_position_ecef_m,
        3,
        &[0.0, 0.0, 0.0],
    ) {
        return status;
    }
    let sat = match parse_satellite_token(fn_name, satellite_id) {
        Ok(s) => s,
        Err(status) => return status,
    };
    match source.observable_state_at_j2000_s(sat, t_j2000_s) {
        Ok(state) => {
            if let Err(status) = copy_exact_f64s(
                fn_name,
                "out_position_ecef_m",
                out_position_ecef_m,
                3,
                &state.position_ecef_m,
            ) {
                return status;
            }
            if let Some(clock) = state.clock_s {
                *out_clock_s = clock;
                *out_has_clock = true;
            }
            SidereonStatus::Ok
        }
        Err(err) => {
            set_last_error(format!("{fn_name}: {err}"));
            SidereonStatus::Solve
        }
    }
}

fn none_to_nan(value: Option<f64>) -> f64 {
    value.unwrap_or(f64::NAN)
}

const _: () = assert!(SATELLITE_TOKEN_C_BYTES == 17);

unsafe fn write_c_token(
    fn_name: &str,
    out_buf: *mut c_char,
    buf_len: usize,
    token: &str,
) -> Result<(), SidereonStatus> {
    if out_buf.is_null() {
        set_last_error(format!("{fn_name}: null out_sat_id"));
        return Err(SidereonStatus::NullPointer);
    }
    let bytes = token.as_bytes();
    if bytes.len() + 1 > buf_len {
        set_last_error(format!(
            "{fn_name}: out_sat_id needs room for {} bytes",
            bytes.len() + 1
        ));
        return Err(SidereonStatus::InvalidArgument);
    }
    for (i, b) in bytes.iter().enumerate() {
        *out_buf.add(i) = *b as c_char;
    }
    *out_buf.add(bytes.len()) = 0;
    Ok(())
}
// ============================================================================

// --- High-accuracy frame transforms (sidereon_core::astro::frames) ----------

use sidereon_core::astro::frames::transforms as ft;
use sidereon_core::astro::frames::{nutation as ft_nutation, precession as ft_precession};
use sidereon_core::astro::time::scales::TimeScales as CoreTimeScales;

fn flatten_mat3(m: [[f64; 3]; 3]) -> [f64; 9] {
    [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    ]
}

unsafe fn read_mat3(
    fn_name: &str,
    arg_name: &str,
    ptr: *const f64,
) -> Result<[[f64; 3]; 3], SidereonStatus> {
    let s = require_slice(ptr, 9, fn_name, arg_name)?;
    Ok([[s[0], s[1], s[2]], [s[3], s[4], s[5]], [s[6], s[7], s[8]]])
}

unsafe fn copy_flat9(out: *mut f64, m: [[f64; 3]; 3]) {
    let flat = flatten_mat3(m);
    for (idx, value) in flat.iter().enumerate() {
        *out.add(idx) = *value;
    }
}

unsafe fn copy_vec3(out: *mut f64, v: [f64; 3]) {
    for (idx, value) in v.iter().enumerate() {
        *out.add(idx) = *value;
    }
}

// --- Broadcast orbit/clock from Keplerian elements (sidereon_core::ephemeris) -

use sidereon_core::ephemeris::{
    ClockOffset as CoreClockOffset, ClockPolynomial as CoreClockPolynomial,
    ConstellationConstants as CoreConstellationConstants,
    KeplerianElements as CoreKeplerianElements, OrbitState as CoreOrbitState,
    SatelliteState as CoreSatelliteState,
};

fn bias_epoch_from_c(fn_name: &str, epoch: SidereonBiasEpoch) -> Result<BiasEpoch, SidereonStatus> {
    BiasEpoch::new(epoch.year, epoch.day_of_year, epoch.second_of_day).map_err(|err| {
        set_last_error(format!("{fn_name}: invalid bias epoch: {err}"));
        SidereonStatus::InvalidArgument
    })
}

// --- PPP static correction precompute (sidereon_core::ppp_corrections) -------

use sidereon_core::ppp_corrections::{
    OceanLoadingBlq as PppOceanLoadingBlq, PoleTideOptions as PppPoleTideOptions,
    PppCorrectionEpoch, PppCorrectionObservation, PppCorrections as PppCorrectionsInner,
    PppCorrectionsOptions, SatelliteAntenna as PppSatelliteAntenna,
    SatelliteAntennaFrequency as PppSatelliteAntennaFrequency,
    SatelliteAntennaOptions as PppSatelliteAntennaOptions,
    NUM_OCEAN_CONSTITUENTS as PPP_NUM_OCEAN_CONSTITUENTS,
};
const _: () = assert!(SIDEREON_PPP_OCEAN_CONSTITUENTS == PPP_NUM_OCEAN_CONSTITUENTS);

const MAX_PPP_SAT_ANTENNA_LABEL_BYTES: usize = 32;

unsafe fn ppp_satellite_antenna_frequencies_from_c(
    fn_name: &str,
    frequencies: *const SidereonSatelliteAntennaFrequency,
    count: usize,
) -> Result<Vec<PppSatelliteAntennaFrequency>, SidereonStatus> {
    let rows = require_slice(frequencies, count, fn_name, "satellite_antenna.frequencies")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let label = parse_bounded_c_string(
            fn_name,
            "satellite_antenna.frequency.label",
            row.label,
            MAX_PPP_SAT_ANTENNA_LABEL_BYTES,
        )?;
        let samples = require_slice(
            row.noazi_pcv,
            row.noazi_count,
            fn_name,
            "satellite_antenna.frequency.noazi_pcv",
        )?;
        let noazi_pcv_m = samples.iter().map(|s| (s.a, s.b)).collect();
        out.push(PppSatelliteAntennaFrequency {
            label,
            pco_m: row.pco_m,
            noazi_pcv_m,
        });
    }
    Ok(out)
}

unsafe fn ppp_satellite_antennas_from_c(
    fn_name: &str,
    antennas: *const SidereonSatelliteAntenna,
    count: usize,
) -> Result<Vec<PppSatelliteAntenna>, SidereonStatus> {
    let rows = require_slice(antennas, count, fn_name, "satellite_antenna.antennas")?;
    let mut out = Vec::with_capacity(count);
    for row in rows {
        let sat = parse_satellite_token(fn_name, row.sat_id)?;
        let frequencies = ppp_satellite_antenna_frequencies_from_c(
            fn_name,
            row.frequencies,
            row.frequency_count,
        )?;
        out.push(PppSatelliteAntenna {
            sat,
            valid_from: row.has_valid_from.then(|| row.valid_from.to_core()),
            valid_until: row.has_valid_until.then(|| row.valid_until.to_core()),
            frequencies,
        });
    }
    Ok(out)
}

// --- Time of closest approach (sidereon_core::astro::tca) --------------------

use sidereon_core::astro::conjunction::PcMethod;
use sidereon_core::astro::covariance::Covariance6;
use sidereon_core::astro::sgp4::JulianDate as TcaJulianDate;
use sidereon_core::astro::tca as core_tca;

// --- Ionosphere-free paired pseudoranges (sidereon_core::combinations) --------

use sidereon_core::combinations::PseudorangeDropReason;

impl TryFrom<u32> for SidereonRetrogradeFactor {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonRetrogradeFactor::Prograde as u32 => Ok(Self::Prograde),
            v if v == SidereonRetrogradeFactor::Retrograde as u32 => Ok(Self::Retrograde),
            _ => Err(()),
        }
    }
}

// --- General angular geometry (sidereon_core::astro::angles) ----------------

unsafe fn angle_scalar_vec3_2(
    fn_name: &str,
    a: *const f64,
    b: *const f64,
    out: *mut f64,
    f: fn([f64; 3], [f64; 3]) -> Result<f64, sidereon_core::astro::angles::AngleError>,
) -> SidereonStatus {
    let out = c_try!(require_out(out, fn_name, "out"));
    *out = 0.0;
    let a = c_try!(read_vec3(fn_name, "a", a));
    let b = c_try!(read_vec3(fn_name, "b", b));
    match f(a, b) {
        Ok(value) => {
            *out = value;
            SidereonStatus::Ok
        }
        Err(err) => extra_invalid_arg(fn_name, err),
    }
}

impl TryFrom<u32> for SidereonRelativeFrame {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonRelativeFrame::Rsw as u32 => Ok(Self::Rsw),
            v if v == SidereonRelativeFrame::Rtn as u32 => Ok(Self::Rtn),
            v if v == SidereonRelativeFrame::Ric as u32 => Ok(Self::Ric),
            v if v == SidereonRelativeFrame::Lvlh as u32 => Ok(Self::Lvlh),
            _ => Err(()),
        }
    }
}

impl TryFrom<u32> for SidereonObserveTargetKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonObserveTargetKind::Sun as u32 => Ok(Self::Sun),
            v if v == SidereonObserveTargetKind::Moon as u32 => Ok(Self::Moon),
            v if v == SidereonObserveTargetKind::Spk as u32 => Ok(Self::Spk),
            v if v == SidereonObserveTargetKind::BarycentricState as u32 => {
                Ok(Self::BarycentricState)
            }
            _ => Err(()),
        }
    }
}

impl TryFrom<u32> for SidereonPlanet {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonPlanet::Mercury as u32 => Ok(Self::Mercury),
            v if v == SidereonPlanet::Venus as u32 => Ok(Self::Venus),
            v if v == SidereonPlanet::Mars as u32 => Ok(Self::Mars),
            v if v == SidereonPlanet::Jupiter as u32 => Ok(Self::Jupiter),
            v if v == SidereonPlanet::Saturn as u32 => Ok(Self::Saturn),
            v if v == SidereonPlanet::Uranus as u32 => Ok(Self::Uranus),
            v if v == SidereonPlanet::Neptune as u32 => Ok(Self::Neptune),
            _ => Err(()),
        }
    }
}

impl TryFrom<u32> for SidereonPlanetaryEventKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonPlanetaryEventKind::Conjunction as u32 => Ok(Self::Conjunction),
            v if v == SidereonPlanetaryEventKind::Opposition as u32 => Ok(Self::Opposition),
            _ => Err(()),
        }
    }
}

impl TryFrom<u32> for SidereonTransitBodyKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            v if v == SidereonTransitBodyKind::Sun as u32 => Ok(Self::Sun),
            v if v == SidereonTransitBodyKind::Moon as u32 => Ok(Self::Moon),
            v if v == SidereonTransitBodyKind::Planet as u32 => Ok(Self::Planet),
            _ => Err(()),
        }
    }
}

unsafe fn almanac_source_from_c<'a>(
    fn_name: &str,
    spk: *const SidereonSpk,
) -> Result<sidereon_core::astro::almanac::EphemerisSource<'a>, SidereonStatus> {
    if spk.is_null() {
        Ok(sidereon_core::astro::almanac::EphemerisSource::Analytic)
    } else {
        let spk = require_ref(spk, fn_name, "spk")?;
        Ok(sidereon_core::astro::almanac::EphemerisSource::Spk(
            &spk.inner,
        ))
    }
}

fn map_observation_error(fn_name: &str, err: ObservationError) -> SidereonStatus {
    extra_invalid_arg(fn_name, err)
}

fn dted_interpolation_from_c(
    fn_name: &str,
    interpolation: u32,
) -> Result<DtedInterpolation, SidereonStatus> {
    match interpolation {
        value if value == SidereonDtedInterpolation::NearestPosting as u32 => {
            Ok(DtedInterpolation::NearestPosting)
        }
        value if value == SidereonDtedInterpolation::Bilinear as u32 => {
            Ok(DtedInterpolation::Bilinear)
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid DTED interpolation selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn dted_options_from_c(
    fn_name: &str,
    options: &SidereonDtedLookupOptions,
) -> Result<DtedLookupOptions, SidereonStatus> {
    Ok(DtedLookupOptions {
        interpolation: dted_interpolation_from_c(fn_name, options.interpolation)?,
    })
}

thread_local! {
    static LAST_TERRAIN_STORE_ERROR: RefCell<Option<SidereonTerrainStoreError>> =
        const { RefCell::new(None) };
    static LAST_TERRAIN_DATUM_ERROR: RefCell<Option<SidereonTerrainDatumError>> =
        const { RefCell::new(None) };
}

fn empty_terrain_store_error() -> SidereonTerrainStoreError {
    SidereonTerrainStoreError {
        kind: SidereonTerrainStoreErrorKind::None as u32,
        path: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
        message: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
        reason: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
        version: 0,
        tag: 0,
        lat_index: 0,
        lon_index: 0,
        expected_checksum64: 0,
        found_checksum64: 0,
    }
}

fn empty_terrain_datum_error() -> SidereonTerrainDatumError {
    SidereonTerrainDatumError {
        kind: SidereonTerrainDatumErrorKind::None as u32,
        path: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
        message: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
        remediation: [0; SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES],
    }
}

fn terrain_store_error_to_c(err: &TerrainStoreError) -> SidereonTerrainStoreError {
    let mut out = empty_terrain_store_error();
    match err {
        TerrainStoreError::Io { path, message } => {
            out.kind = SidereonTerrainStoreErrorKind::Io as u32;
            out.path = fixed_c_chars(&path.display().to_string());
            out.message = fixed_c_chars(message);
        }
        TerrainStoreError::Parse { reason } => {
            out.kind = SidereonTerrainStoreErrorKind::Parse as u32;
            out.reason = fixed_c_chars(reason);
        }
        TerrainStoreError::UnsupportedVersion { version } => {
            out.kind = SidereonTerrainStoreErrorKind::UnsupportedVersion as u32;
            out.version = *version;
        }
        TerrainStoreError::UnsupportedDatum { tag } => {
            out.kind = SidereonTerrainStoreErrorKind::UnsupportedDatum as u32;
            out.tag = *tag;
        }
        TerrainStoreError::DuplicateTile {
            lat_index,
            lon_index,
        } => {
            out.kind = SidereonTerrainStoreErrorKind::DuplicateTile as u32;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
        }
        TerrainStoreError::TileIdMismatch {
            path,
            expected,
            found,
        } => {
            out.kind = SidereonTerrainStoreErrorKind::TileIdMismatch as u32;
            out.path = fixed_c_chars(&path.display().to_string());
            out.message = fixed_c_chars(&format!("expected tile {expected:?}, parsed {found:?}"));
        }
        TerrainStoreError::Checksum {
            lat_index,
            lon_index,
            expected,
            found,
        } => {
            out.kind = SidereonTerrainStoreErrorKind::Checksum as u32;
            out.lat_index = *lat_index;
            out.lon_index = *lon_index;
            out.expected_checksum64 = *expected;
            out.found_checksum64 = *found;
        }
    }
    out
}

fn terrain_datum_error_to_c(err: &TerrainDatumError) -> SidereonTerrainDatumError {
    let mut out = empty_terrain_datum_error();
    match err {
        TerrainDatumError::Terrain(err) => {
            out.kind = SidereonTerrainDatumErrorKind::Terrain as u32;
            out.message = fixed_c_chars(&err.to_string());
        }
        TerrainDatumError::Geoid(err) => {
            out.kind = SidereonTerrainDatumErrorKind::Geoid as u32;
            out.message = fixed_c_chars(&err.to_string());
        }
        TerrainDatumError::Io { path, message } => {
            out.kind = SidereonTerrainDatumErrorKind::Io as u32;
            out.path = fixed_c_chars(&path.display().to_string());
            out.message = fixed_c_chars(message);
        }
        TerrainDatumError::MissingEgm96Dac { path, remediation } => {
            out.kind = SidereonTerrainDatumErrorKind::MissingEgm96Dac as u32;
            out.path = fixed_c_chars(&path.display().to_string());
            out.remediation = fixed_c_chars(remediation);
        }
    }
    out
}

fn map_terrain_store_error(fn_name: &str, err: TerrainStoreError) -> SidereonStatus {
    let typed = terrain_store_error_to_c(&err);
    LAST_TERRAIN_STORE_ERROR.with(|slot| *slot.borrow_mut() = Some(typed));
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_terrain_datum_error(fn_name: &str, err: TerrainDatumError) -> SidereonStatus {
    let typed = terrain_datum_error_to_c(&err);
    LAST_TERRAIN_DATUM_ERROR.with(|slot| *slot.borrow_mut() = Some(typed));
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

unsafe fn terrain_geoid_model_from_c<'a>(
    fn_name: &str,
    model: u32,
    geoid: *const SidereonEgm96FifteenMinuteGeoid,
) -> Result<CoreTerrainGeoidModel<'a>, SidereonStatus> {
    match model {
        value if value == SidereonTerrainGeoidModel::Egm96OneDegree as u32 => {
            Ok(CoreTerrainGeoidModel::Egm96OneDegree)
        }
        value if value == SidereonTerrainGeoidModel::Egm96FifteenMinute as u32 => {
            let geoid = require_ref(geoid, fn_name, "geoid")?;
            Ok(CoreTerrainGeoidModel::Egm96FifteenMinute(&geoid.inner))
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid terrain geoid model selector"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

struct RtcmFrameRecord {
    body: Vec<u8>,
    frame_len: usize,
}

unsafe fn rtcm_message_at<'a>(
    fn_name: &str,
    messages: *const SidereonRtcmMessages,
    index: usize,
) -> Result<&'a RtcmMessage, SidereonStatus> {
    let handle = require_ref(messages, fn_name, "messages")?;
    handle.messages.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: index {index} out of range ({} messages)",
            handle.messages.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

unsafe fn rtk_arc_positions_from_c(
    fn_name: &str,
    ptr: *const SidereonRtkArcPositionEntry,
    count: usize,
    arg_name: &str,
) -> Result<BTreeMap<String, [f64; 3]>, SidereonStatus> {
    let raw = require_slice(ptr, count, fn_name, arg_name)?;
    validate_element_count::<SidereonRtkArcPositionEntry>(fn_name, arg_name, raw.len())?;
    let mut out = BTreeMap::new();
    for (idx, entry) in raw.iter().enumerate() {
        let id = parse_satellite_token(fn_name, entry.id)?.to_string();
        insert_unique_string_key(fn_name, arg_name, idx, &mut out, id, entry.pos)?;
    }
    Ok(out)
}

fn rtk_cycle_slip_policy_from_c(
    fn_name: &str,
    value: u32,
) -> Result<CycleSlipPolicy, SidereonStatus> {
    match value {
        v if v == SidereonRtkCycleSlipPolicy::Error as u32 => Ok(CycleSlipPolicy::Error),
        v if v == SidereonRtkCycleSlipPolicy::DropSatellite as u32 => {
            Ok(CycleSlipPolicy::DropSatellite)
        }
        v if v == SidereonRtkCycleSlipPolicy::SplitArc as u32 => Ok(CycleSlipPolicy::SplitArc),
        _ => {
            set_last_error(format!("{fn_name}: invalid preprocessing.cycle_slip"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn rtk_arc_preprocessing_from_c(
    fn_name: &str,
    preprocessing: &SidereonRtkArcPreprocessing,
) -> Result<RtkArcPreprocessing, SidereonStatus> {
    Ok(RtkArcPreprocessing {
        cycle_slip: if preprocessing.has_cycle_slip {
            Some(rtk_cycle_slip_policy_from_c(
                fn_name,
                preprocessing.cycle_slip,
            )?)
        } else {
            None
        },
        hatch_window_cap: preprocessing
            .has_hatch_window_cap
            .then_some(preprocessing.hatch_window_cap),
        elevation_mask_deg: preprocessing
            .has_elevation_mask_deg
            .then_some(preprocessing.elevation_mask_deg),
    })
}

unsafe fn rtk_reference_selection_from_c(
    fn_name: &str,
    reference_mode: u32,
    reference_satellite: *const c_char,
    reference_per_system: *const SidereonRtkArcReferenceEntry,
    reference_per_system_count: usize,
) -> Result<BaselineReferenceSelection, SidereonStatus> {
    match reference_mode {
        value if value == SidereonRtkArcReferenceMode::Auto as u32 => {
            Ok(BaselineReferenceSelection::Auto)
        }
        value if value == SidereonRtkArcReferenceMode::Satellite as u32 => {
            let sat = parse_bounded_c_string(
                fn_name,
                "reference_satellite",
                reference_satellite,
                MAX_RTK_ID_BYTES,
            )?;
            Ok(BaselineReferenceSelection::Satellite(sat))
        }
        value if value == SidereonRtkArcReferenceMode::PerSystem as u32 => {
            let raw = require_slice(
                reference_per_system,
                reference_per_system_count,
                fn_name,
                "reference_per_system",
            )?;
            let mut refs = BTreeMap::new();
            for (idx, entry) in raw.iter().enumerate() {
                let system = gnss_system_to_letter(gnss_system_from_c_code(
                    fn_name,
                    "reference_per_system.system",
                    entry.system as u32,
                )?)
                .to_owned();
                let sat = parse_satellite_token(fn_name, entry.sat_id)?.to_string();
                insert_unique_string_key(
                    fn_name,
                    "reference_per_system",
                    idx,
                    &mut refs,
                    system,
                    sat,
                )?;
            }
            Ok(BaselineReferenceSelection::PerSystem(refs))
        }
        _ => {
            set_last_error(format!("{fn_name}: invalid reference_mode"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn rtk_arc_reference_from_c(
    fn_name: &str,
    config: &SidereonRtkArcConfig,
) -> Result<BaselineReferenceSelection, SidereonStatus> {
    rtk_reference_selection_from_c(
        fn_name,
        config.reference_mode,
        config.reference_satellite,
        config.reference_per_system,
        config.reference_per_system_count,
    )
}

unsafe fn rtk_arc_update_opts_from_c(
    fn_name: &str,
    options: &SidereonRtkArcUpdateOptions,
) -> Result<UpdateOpts, SidereonStatus> {
    let float_only_systems = rtk_float_only_systems_from_c(
        fn_name,
        options.float_only_systems,
        options.float_only_system_count,
    )?;
    let receiver_antenna_corrections =
        rtk_receiver_antenna_from_c(fn_name, options.receiver_antenna)?;
    Ok(UpdateOpts {
        hold_sigma_m: options.hold_sigma_m,
        position_tol_m: options.position_tol_m,
        ambiguity_tol_m: options.ambiguity_tol_m,
        max_iterations: options.max_iterations,
        process_noise_baseline_sigma_m: options.process_noise_baseline_sigma_m,
        dynamics_model: if options.dynamics_velocity_propagated {
            DynamicsModel::VelocityPropagated
        } else {
            DynamicsModel::ConstantPosition
        },
        float_only_systems,
        report_residuals: options.report_residuals,
        receiver_antenna_corrections,
        ar_arming_sigma_m: options
            .has_ar_arming_sigma_m
            .then_some(options.ar_arming_sigma_m),
        search: SearchOpts {
            ratio_threshold: options.ratio_threshold,
        },
    })
}

fn cycle_slip_options_from_c(
    options: &SidereonCycleSlipOptions,
) -> sidereon_core::carrier_phase::CycleSlipOptions {
    sidereon_core::carrier_phase::CycleSlipOptions {
        gf_threshold_m: options.gf_threshold_m,
        mw_threshold_cycles: options.mw_threshold_cycles,
        min_arc_gap_s: options.min_arc_gap_s,
    }
}

unsafe fn rtk_dual_frequency_observation_from_c(
    fn_name: &str,
    arg_name: &str,
    observation: &SidereonRtkDualFrequencyObservation,
) -> Result<RtkDualFrequencyObservation, SidereonStatus> {
    let ambiguity_id = parse_bounded_c_string(
        fn_name,
        &format!("{arg_name}.ambiguity_id"),
        observation.ambiguity_id,
        MAX_RTK_ID_BYTES,
    )?;
    Ok(RtkDualFrequencyObservation {
        ambiguity_id,
        p1_m: observation.p1_m,
        p2_m: observation.p2_m,
        phi1_cycles: observation.phi1_cycles,
        phi2_cycles: observation.phi2_cycles,
        f1_hz: observation.f1_hz,
        f2_hz: observation.f2_hz,
        lli1: observation.has_lli1.then_some(observation.lli1),
        lli2: observation.has_lli2.then_some(observation.lli2),
    })
}

fn fixed_c_token_to_string<const N: usize>(
    fn_name: &str,
    arg_name: &str,
    bytes: &[c_char; N],
) -> Result<String, SidereonStatus> {
    let len = bytes.iter().position(|byte| *byte == 0).unwrap_or(N);
    if len == 0 {
        set_last_error(format!("{fn_name}: {arg_name} is empty"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let raw = bytes[..len]
        .iter()
        .map(|byte| *byte as u8)
        .collect::<Vec<_>>();
    match str::from_utf8(&raw) {
        Ok(text) => Ok(text.to_owned()),
        Err(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            Err(SidereonStatus::InvalidToken)
        }
    }
}

unsafe fn rtk_arc_epoch_at<'a>(
    fn_name: &str,
    solution: *const SidereonRtkArcSolution,
    index: usize,
) -> Result<&'a RtkArcEpochSolution, SidereonStatus> {
    let handle = require_ref(solution, fn_name, "solution")?;
    handle.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: epoch index {index} out of range ({} epochs)",
            handle.inner.epochs.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

unsafe fn rtk_ionosphere_free_epoch_at<'a>(
    fn_name: &str,
    solution: *const SidereonRtkIonosphereFreeArcSolution,
    index: usize,
) -> Result<&'a RtkArcEpoch, SidereonStatus> {
    let handle = require_ref(solution, fn_name, "solution")?;
    handle.inner.epochs.get(index).ok_or_else(|| {
        set_last_error(format!(
            "{fn_name}: epoch index {index} out of range ({} epochs)",
            handle.inner.epochs.len()
        ));
        SidereonStatus::InvalidArgument
    })
}

struct RtkVariableOut<T> {
    out: *mut T,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
}

/// Maximum RTCM 3 length-prefixed equipment string (8-bit character count).
const MAX_RTCM_STRING_BYTES: usize = 255;

unsafe fn optional_bounded_c_string(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
    max_len: usize,
) -> Result<Option<String>, SidereonStatus> {
    if ptr.is_null() {
        Ok(None)
    } else {
        Ok(Some(parse_bounded_c_string(
            fn_name, arg_name, ptr, max_len,
        )?))
    }
}
const _: () =
    assert!(SIDEREON_LNAV_SUBFRAME_LENGTH == sidereon_core::navigation::lnav::SUBFRAME_LENGTH);
// ===========================================================================

use nalgebra::DMatrix;
use sidereon_core::astro::bodies::{
    find_moon_elevation_crossings as core_find_moon_elevation_crossings,
    find_moon_transits as core_find_moon_transits, moon_az_el as core_moon_az_el,
    moon_illumination as core_moon_illumination, sun_az_el as core_sun_az_el, BodyObservationError,
    MoonElevationCrossingKind, MoonElevationOptions as CoreMoonElevationOptions, MoonTransitKind,
};
use sidereon_core::astro::events::EventFinderError;
use sidereon_core::astro::frames::transforms::GeodeticStationKm;
use sidereon_core::astro::math::least_squares::{
    covariance_from_jacobian as core_covariance_from_jacobian, hessian_trace as core_hessian_trace,
    normal_covariance as core_normal_covariance, SolveError as LsqSolveError,
};
use sidereon_core::astro::time::scales::{
    gps_utc_offset_s as core_gps_utc_offset_s, tai_utc_offset_s as core_tai_utc_offset_s,
};
use sidereon_core::geoid::{
    egm96_ellipsoidal_height_m as core_egm96_ellipsoidal_height_m,
    egm96_orthometric_height_m as core_egm96_orthometric_height_m,
    egm96_undulation as core_egm96_undulation,
};
use sidereon_core::geometry::{
    dop_with_convention as core_dop_with_convention, error_ellipse_2x2 as core_error_ellipse_2x2,
    EnuConvention,
};
use sidereon_core::observables::{predict_batch as core_predict_batch, PredictRequest};
use sidereon_core::quality::normality::{
    jarque_bera as core_jarque_bera, kurtosis as core_kurtosis, moments as core_moments,
    shapiro_wilk as core_shapiro_wilk, skewness as core_skewness, NormalityError,
};
use trust_region_least_squares::batch::{
    solve_data_problem_drop_one as core_solve_drop_one,
    solve_data_problem_drop_one_with as core_solve_drop_one_with, DropOneReport,
};
use trust_region_least_squares::data::{
    solve_data_problem as core_solve_data_problem,
    solve_data_problem_with as core_solve_data_problem_with, BuiltinResidual, DataProblem,
};
use trust_region_least_squares::hostlapack::LapackSvd;
use trust_region_least_squares::loss::Loss as TrlsLoss;
use trust_region_least_squares::trf::{TrfError, TrfResult, XScale};

fn trls_kind_from_c(
    fn_name: &str,
    arg_name: &str,
    kind: u32,
) -> Result<SidereonTrlsKind, SidereonStatus> {
    match kind {
        value if value == SidereonTrlsKind::Linear as u32 => Ok(SidereonTrlsKind::Linear),
        value if value == SidereonTrlsKind::Polynomial as u32 => Ok(SidereonTrlsKind::Polynomial),
        value if value == SidereonTrlsKind::Exponential as u32 => Ok(SidereonTrlsKind::Exponential),
        _ => {
            set_last_error(format!("{fn_name}: invalid {arg_name} TRLS residual kind"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// Map a least-squares geometry error to a status code. A rank-deficient
/// Jacobian reports SIDEREON_STATUS_SOLVE; malformed input reports
/// SIDEREON_STATUS_INVALID_ARGUMENT.
fn map_lsq_error(fn_name: &str, err: LsqSolveError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        LsqSolveError::SingularJacobian => SidereonStatus::Solve,
        LsqSolveError::InvalidInput { .. } => SidereonStatus::InvalidArgument,
    }
}

fn zero_predicted_observables() -> SidereonPredictedObservables {
    SidereonPredictedObservables {
        geometric_range_m: 0.0,
        range_rate_m_s: 0.0,
        doppler_hz: 0.0,
        has_sat_clock_s: false,
        sat_clock_s: 0.0,
        elevation_deg: 0.0,
        azimuth_deg: 0.0,
        transmit_offset_us: 0,
        transmit_time_j2000_s: 0.0,
        los_unit: [0.0; 3],
        sat_pos_ecef_m: [0.0; 3],
        sat_velocity_m_s: [0.0; 3],
    }
}

unsafe fn predict_requests_from_c(
    fn_name: &str,
    requests: &[SidereonPredictRequest],
) -> Result<Vec<PredictRequest>, SidereonStatus> {
    let mut parsed = Vec::with_capacity(requests.len());
    for request in requests {
        let sat = parse_satellite_token(fn_name, request.sat_id)?;
        parsed.push((sat, request.receiver_ecef_m, request.t_rx_j2000_s));
    }
    Ok(parsed)
}

unsafe fn write_predict_batch_results(
    results: &[Result<PredictedObservables, ObservablesError>],
    out: *mut SidereonPredictedObservables,
    out_ok: *mut bool,
) {
    for (idx, result) in results.iter().enumerate() {
        match result {
            Ok(obs) => {
                *out.add(idx) = predicted_observables_to_c(obs);
                *out_ok.add(idx) = true;
            }
            Err(_) => {
                *out.add(idx) = zero_predicted_observables();
                *out_ok.add(idx) = false;
            }
        }
    }
}

/// Reconstruct an [`Instant`] from seconds since J2000 in a given scale, using
/// the same split the SP3/IONEX readers carry: the whole-second count fixes the
/// integer JD boundary and the residual within-day seconds become the fraction
/// (carrying any sub-second part). The floored node axis the sample source builds
/// therefore matches the SP3 path exactly, while the unfloored query stays
/// faithful.
fn instant_from_j2000_seconds(
    fn_name: &str,
    arg_name: &str,
    scale: TimeScale,
    j2000_s: f64,
) -> Result<Instant, SidereonStatus> {
    if !j2000_s.is_finite() {
        set_last_error(format!("{fn_name}: {arg_name} epoch is not finite"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let whole_s = j2000_s.floor();
    // `whole_s as i64` saturates for finite magnitudes outside i64's range,
    // which would silently clamp an absurd epoch instead of rejecting it. Guard
    // the range before the cast. `i64::MAX as f64` rounds up to 2^63, so `>=`
    // there also rejects the single saturating positive value 2^63; `i64::MIN as
    // f64` is exactly -2^63 and stays valid.
    if whole_s < i64::MIN as f64 || whole_s >= i64::MAX as f64 {
        set_last_error(format!("{fn_name}: {arg_name} epoch is out of range"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let (jd_whole, day_fraction) = split_julian_date_from_j2000_seconds(whole_s as i64);
    let fraction = day_fraction + (j2000_s - whole_s) / SECONDS_PER_DAY;
    let split = JulianDateSplit::new(jd_whole, fraction).map_err(|err| {
        set_last_error(format!("{fn_name}: {arg_name} epoch: {err}"));
        SidereonStatus::InvalidArgument
    })?;
    Ok(Instant::from_julian_date(scale, split))
}

fn range_prediction_to_c(pred: &RangePrediction) -> SidereonRangePrediction {
    SidereonRangePrediction {
        geometric_range_m: pred.geometric_range_m,
        has_sat_clock_s: pred.sat_clock_s.is_some(),
        sat_clock_s: pred.sat_clock_s.unwrap_or(0.0),
        transmit_time_j2000_s: pred.transmit_time_j2000_s,
        sat_pos_ecef_m: pred.sat_pos_ecef_m,
    }
}

fn zero_range_prediction() -> SidereonRangePrediction {
    SidereonRangePrediction {
        geometric_range_m: 0.0,
        has_sat_clock_s: false,
        sat_clock_s: 0.0,
        transmit_time_j2000_s: 0.0,
        sat_pos_ecef_m: [0.0; 3],
    }
}

fn zero_range_prediction_core() -> RangePrediction {
    RangePrediction {
        geometric_range_m: 0.0,
        sat_clock_s: None,
        transmit_time_j2000_s: 0.0,
        sat_pos_ecef_m: [0.0; 3],
    }
}

fn ephemeris_sample_row_to_c(row: &EphemerisSampleRow) -> SidereonEphemerisSampleRow {
    SidereonEphemerisSampleRow {
        sat_id: satellite_token(row.sat),
        epoch_j2000_s: row.epoch_j2000_s,
        status: match row.status {
            EphemerisSampleStatus::Valid => SidereonEphemerisSampleStatus::Valid,
            EphemerisSampleStatus::Gap => SidereonEphemerisSampleStatus::Gap,
        },
        has_position_ecef_m: row.position_ecef_m.is_some(),
        position_ecef_m: row.position_ecef_m.unwrap_or([0.0; 3]),
        has_clock_s: row.clock_s.is_some(),
        clock_s: row.clock_s.unwrap_or(0.0),
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn ephemeris_sample_common(
    fn_name: &str,
    source: &dyn ObservableEphemerisSource,
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
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    let sat_ptrs = c_try!(require_slice(
        satellites,
        satellite_count,
        fn_name,
        "satellites"
    ));
    let mut sats = Vec::with_capacity(satellite_count);
    for ptr in sat_ptrs {
        sats.push(c_try!(parse_satellite_token(fn_name, *ptr)));
    }
    let rows = match ephemeris_sample(source, &sats, start_j2000_s, stop_j2000_s, step_s) {
        Ok(rows) => rows,
        Err(err) => return map_observables_error(fn_name, err),
    };
    let mapped: Vec<SidereonEphemerisSampleRow> =
        rows.iter().map(ephemeris_sample_row_to_c).collect();
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        &mapped,
        out,
        len,
        out_written,
        out_required,
    ));
    SidereonStatus::Ok
}

unsafe fn range_prediction_requests_from_c(
    fn_name: &str,
    requests: &[SidereonRangePredictionRequest],
) -> Result<Vec<RangePredictionRequest>, SidereonStatus> {
    let mut parsed = Vec::with_capacity(requests.len());
    for request in requests {
        let sat = parse_satellite_token(fn_name, request.sat_id)?;
        parsed.push(RangePredictionRequest {
            sat,
            receiver_ecef_m: request.receiver_ecef_m,
            t_rx_j2000_s: request.t_rx_j2000_s,
        });
    }
    Ok(parsed)
}

/// Shared batch range-prediction body over any observable source. Delegates to
/// sidereon_core::observables::predict_ranges, so out[i] is exactly the geometry
/// for requests[i]. A per-request failure (invalid input or missing ephemeris)
/// aborts the batch, records the error, and returns its status; out is then the
/// pre-zeroed prefix. options may be NULL for the engine defaults.
unsafe fn predict_ranges_into(
    fn_name: &str,
    source: &dyn ObservableEphemerisSource,
    requests: *const SidereonRangePredictionRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonRangePrediction,
) -> SidereonStatus {
    let raw = c_try!(require_slice(requests, count, fn_name, "requests"));
    // Validate the caller-owned output pointer directly. The C contract only
    // guarantees `out` is writable, not readable or initialized, so we must not
    // form a `&[T]`/`&mut [T]` over it (that would assert initialized elements
    // and is UB). Mirror require_slice's checks by hand: non-null when count >
    // 0, and no element-count/size overflow. Every write below goes through the
    // raw pointer.
    if count > 0 && out.is_null() {
        set_last_error(format!("{fn_name}: null out"));
        return SidereonStatus::NullPointer;
    }
    c_try!(validate_element_count::<SidereonRangePrediction>(
        fn_name, "out", count
    ));
    // Pre-zero the output via raw writes so an aborted batch leaves defined
    // values, never reading the uninitialized destination.
    for idx in 0..count {
        out.add(idx).write(zero_range_prediction());
    }
    let opts = c_try!(predict_options_from_c(fn_name, options));
    let parsed = c_try!(range_prediction_requests_from_c(fn_name, raw));
    let mut results = vec![zero_range_prediction_core(); count];
    match observables_predict_ranges(source, &parsed, opts, &mut results) {
        Ok(()) => {}
        Err(err) => return map_observables_error(fn_name, err),
    }
    for (idx, pred) in results.iter().enumerate() {
        out.add(idx).write(range_prediction_to_c(pred));
    }
    SidereonStatus::Ok
}

fn observable_state_element_status_to_c(
    status: CoreObservableStateElementStatus,
) -> SidereonObservableStateElementStatus {
    match status {
        CoreObservableStateElementStatus::Valid => SidereonObservableStateElementStatus::Valid,
        CoreObservableStateElementStatus::Gap => SidereonObservableStateElementStatus::Gap,
        CoreObservableStateElementStatus::Error => SidereonObservableStateElementStatus::Error,
    }
}

fn observable_state_result_status(result: &Result<(), ObservablesError>) -> SidereonStatus {
    match result {
        Ok(()) => SidereonStatus::Ok,
        Err(ObservablesError::InvalidInput { .. })
        | Err(ObservablesError::Media(_))
        | Err(ObservablesError::Ephemeris(CoreError::InvalidInput(_))) => {
            SidereonStatus::InvalidArgument
        }
        Err(ObservablesError::NoEphemeris | ObservablesError::Ephemeris(_)) => {
            SidereonStatus::Solve
        }
    }
}

fn checked_position_output_count(fn_name: &str, count: usize) -> Result<usize, SidereonStatus> {
    count.checked_mul(3).ok_or_else(|| {
        set_last_error(format!("{fn_name}: output position count is too large"));
        SidereonStatus::InvalidArgument
    })
}

unsafe fn initialize_observable_state_outputs(
    fn_name: &str,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> Result<(), SidereonStatus> {
    let position_values = checked_position_output_count(fn_name, count)?;
    require_out_array(
        out_positions_ecef_m,
        position_values,
        fn_name,
        "out_positions_ecef_m",
    )?;
    require_out_array(out_clocks_s, count, fn_name, "out_clocks_s")?;
    require_out_array(out_has_clocks_s, count, fn_name, "out_has_clocks_s")?;
    require_out_array(out_element_statuses, count, fn_name, "out_element_statuses")?;
    require_out_array(out_result_statuses, count, fn_name, "out_result_statuses")?;

    for idx in 0..count {
        let base = idx * 3;
        for (axis, value) in OBSERVABLE_STATE_MISSING_POSITION_ECEF_M.iter().enumerate() {
            out_positions_ecef_m.add(base + axis).write(*value);
        }
        out_clocks_s.add(idx).write(0.0);
        out_has_clocks_s.add(idx).write(false);
        out_element_statuses
            .add(idx)
            .write(SidereonObservableStateElementStatus::Error);
        out_result_statuses
            .add(idx)
            .write(SidereonStatus::InvalidArgument);
    }
    Ok(())
}

unsafe fn satellites_from_c_tokens(
    fn_name: &str,
    satellites: *const *const c_char,
    count: usize,
) -> Result<Vec<GnssSatelliteId>, SidereonStatus> {
    let raw = require_slice(satellites, count, fn_name, "satellites")?;
    let mut parsed = Vec::with_capacity(raw.len());
    for ptr in raw {
        parsed.push(parse_satellite_token(fn_name, *ptr)?);
    }
    Ok(parsed)
}

#[allow(clippy::too_many_arguments)]
unsafe fn write_observable_state_batch(
    fn_name: &str,
    batch: &ObservableStateBatch,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    if batch.len() != count {
        set_last_error(format!(
            "{fn_name}: core returned {} observable states for {count} inputs",
            batch.len()
        ));
        return SidereonStatus::Solve;
    }
    for idx in 0..count {
        let position = batch.positions_ecef_m[idx];
        let base = idx * 3;
        for (axis, value) in position.iter().enumerate() {
            out_positions_ecef_m.add(base + axis).write(*value);
        }
        let clock_s = batch.clocks_s[idx];
        out_has_clocks_s.add(idx).write(clock_s.is_some());
        out_clocks_s.add(idx).write(clock_s.unwrap_or(0.0));
        let element_status = batch
            .element_status(idx)
            .unwrap_or(CoreObservableStateElementStatus::Error);
        out_element_statuses
            .add(idx)
            .write(observable_state_element_status_to_c(element_status));
        out_result_statuses
            .add(idx)
            .write(observable_state_result_status(&batch.element_results[idx]));
    }
    SidereonStatus::Ok
}

#[allow(clippy::too_many_arguments)]
unsafe fn observable_states_at_j2000_s_common(
    fn_name: &str,
    source: &dyn ObservableEphemerisSource,
    satellites: *const *const c_char,
    epochs_j2000_s: *const f64,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    c_try!(initialize_observable_state_outputs(
        fn_name,
        count,
        out_positions_ecef_m,
        out_clocks_s,
        out_has_clocks_s,
        out_element_statuses,
        out_result_statuses,
    ));
    let sats = c_try!(satellites_from_c_tokens(fn_name, satellites, count));
    let epochs = c_try!(require_slice(
        epochs_j2000_s,
        count,
        fn_name,
        "epochs_j2000_s"
    ));
    let batch = match source.observable_states_at_j2000_s(&sats, epochs) {
        Ok(batch) => batch,
        Err(err) => return map_observables_error(fn_name, err),
    };
    write_observable_state_batch(
        fn_name,
        &batch,
        count,
        out_positions_ecef_m,
        out_clocks_s,
        out_has_clocks_s,
        out_element_statuses,
        out_result_statuses,
    )
}

#[allow(clippy::too_many_arguments)]
unsafe fn observable_states_at_shared_j2000_s_common(
    fn_name: &str,
    source: &dyn ObservableEphemerisSource,
    satellites: *const *const c_char,
    satellite_count: usize,
    epoch_j2000_s: f64,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    c_try!(initialize_observable_state_outputs(
        fn_name,
        satellite_count,
        out_positions_ecef_m,
        out_clocks_s,
        out_has_clocks_s,
        out_element_statuses,
        out_result_statuses,
    ));
    let sats = c_try!(satellites_from_c_tokens(
        fn_name,
        satellites,
        satellite_count
    ));
    let batch = source.observable_states_at_shared_j2000_s(&sats, epoch_j2000_s);
    write_observable_state_batch(
        fn_name,
        &batch,
        satellite_count,
        out_positions_ecef_m,
        out_clocks_s,
        out_has_clocks_s,
        out_element_statuses,
        out_result_statuses,
    )
}

// --- 0.13 estimation and detection primitives ------------------------------

use sidereon_core::estimation as core_estimation;

// --- 0.13 source localization ----------------------------------------------

use sidereon_core::source_localization::{
    chan_ho_initial_guess as core_chan_ho_initial_guess, locate_source as core_locate_source,
    source_crlb as core_source_crlb, source_dop as core_source_dop, Loss as SourceLossInner,
    Sensor as CoreSourceSensor, SourceCovariance as CoreSourceCovariance,
    SourceCrlb as CoreSourceCrlb, SourceInitialGuess as CoreSourceInitialGuess,
    SourceLocalizationError as CoreSourceLocalizationError,
    SourceLocateOptions as CoreSourceLocateOptions, SourceResidual as CoreSourceResidual,
    SourceSensorInfluence as CoreSourceSensorInfluence, SourceSolution as CoreSourceSolution,
    SourceSolveMode as CoreSourceSolveMode,
};

fn zero_vec3_from_slice(values: &[f64]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (idx, value) in values.iter().take(3).enumerate() {
        out[idx] = *value;
    }
    out
}

fn empty_cnav_parameters() -> SidereonCnavParameters {
    SidereonCnavParameters {
        present: false,
        adot_m_s: 0.0,
        delta_n0_dot_rad_s2: 0.0,
        top_week: 0,
        top_tow_s: 0.0,
        ura_ed_index: 0,
        ura_ned0_index: 0,
        ura_ned1_index: 0,
        ura_ned2_index: 0,
        transmission_time_sow: 0.0,
        has_flags: false,
        flags: 0,
    }
}

fn fixed_c_array_to_string(
    fn_name: &str,
    field: &str,
    bytes: &[c_char],
) -> Result<String, SidereonStatus> {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let raw: Vec<u8> = bytes[..len].iter().map(|&b| b as u8).collect();
    str::from_utf8(&raw).map(str::to_owned).map_err(|_| {
        set_last_error(format!("{fn_name}: {field} is not valid UTF-8"));
        SidereonStatus::InvalidToken
    })
}

fn observation_qc_signal_empty_sat() -> SidereonSatelliteToken {
    SidereonSatelliteToken {
        bytes: [0; SATELLITE_TOKEN_C_BYTES],
    }
}

unsafe fn text_bytes_from_c<'a>(
    fn_name: &str,
    data: *const u8,
    len: usize,
) -> Result<&'a str, SidereonStatus> {
    let bytes = require_slice(data, len, fn_name, "data")?;
    str::from_utf8(bytes).map_err(|_| {
        set_last_error(format!("{fn_name}: data is not valid UTF-8"));
        SidereonStatus::InvalidToken
    })
}

unsafe fn parse_c_string_allow_empty(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
) -> Result<String, SidereonStatus> {
    if ptr.is_null() {
        set_last_error(format!("{fn_name}: null {arg_name}"));
        return Err(SidereonStatus::NullPointer);
    }
    match CStr::from_ptr(ptr).to_str() {
        Ok(value) => Ok(value.to_owned()),
        Err(_) => {
            set_last_error(format!("{fn_name}: {arg_name} is not valid UTF-8"));
            Err(SidereonStatus::InvalidToken)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_error() -> String {
        LAST_ERROR.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|message| message.as_c_str().to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }

    fn assert_invalid_argument<T>(result: Result<Vec<T>, SidereonStatus>) {
        match result {
            Ok(_) => panic!("expected InvalidArgument"),
            Err(status) => assert_eq!(status, SidereonStatus::InvalidArgument),
        }
    }

    fn pass_rows_match(a: &[SidereonSatellitePass], b: &[SidereonSatellitePass]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(left, right)| {
                left.aos_unix_us == right.aos_unix_us
                    && left.los_unix_us == right.los_unix_us
                    && left.culmination_unix_us == right.culmination_unix_us
                    && left.max_elevation_deg.to_bits() == right.max_elevation_deg.to_bits()
                    && left.duration_s.to_bits() == right.duration_s.to_bits()
            })
    }

    #[test]
    fn batch_flattened_count_rejects_oversized_required_count() {
        let too_many = isize::MAX as usize / size_of::<SidereonTemeState>() + 1;
        let err = checked_flattened_count::<SidereonTemeState>(
            "test_batch_overflow",
            "required",
            [too_many],
        )
        .expect_err("oversized batch result must be rejected");

        assert_eq!(err, SidereonStatus::InvalidArgument);
        let message = last_error();
        assert!(message.contains("test_batch_overflow"));
        assert!(message.contains("required is too large"));
    }

    #[test]
    fn tle_find_passes_uses_loaded_opsmode() {
        const LINE1: &str = "1 23599U 95029B   06171.76535463  .00085586  12891-6  12956-2 0  2905";
        const LINE2: &str =
            "2 23599   6.9327   0.2849 5782022 274.4436  25.2425  4.47796565123555      0.0       720.0         20.00";
        const START_UNIX_US: i64 = 1_150_914_126_640_038;
        const END_UNIX_US: i64 = 1_150_957_326_640_038;

        let station = SidereonGroundStation {
            latitude_deg: 40.7128,
            longitude_deg: -74.0060,
            altitude_m: 10.0,
        };
        let c_options = SidereonPassFinderOptions {
            elevation_mask_deg: 0.0,
            step_seconds: 300.0,
            time_tolerance_s: 0.001,
        };
        let core_options = PassFinderOptions {
            elevation_mask_deg: c_options.elevation_mask_deg,
            coarse_step_seconds: c_options.step_seconds,
            time_tolerance_seconds: c_options.time_tolerance_s,
        };
        let start = UtcInstant::from_unix_microseconds(START_UNIX_US);
        let end = UtcInstant::from_unix_microseconds(END_UNIX_US);
        let ground_station = ground_station_from_c(&station);

        let improved = Satellite::from_tle_with_opsmode(LINE1, LINE2, OpsMode::Improved).unwrap();
        let afspc = Satellite::from_tle_with_opsmode(LINE1, LINE2, OpsMode::Afspc).unwrap();
        let expected_improved: Vec<SidereonSatellitePass> =
            find_passes_for_satellite(&improved, ground_station, start, end, core_options)
                .unwrap()
                .iter()
                .map(satellite_pass_to_c)
                .collect();
        let afspc_reference: Vec<SidereonSatellitePass> =
            find_passes_for_satellite(&afspc, ground_station, start, end, core_options)
                .unwrap()
                .iter()
                .map(satellite_pass_to_c)
                .collect();
        assert!(!pass_rows_match(&expected_improved, &afspc_reference));

        let line1 = CString::new(LINE1).unwrap();
        let line2 = CString::new(LINE2).unwrap();
        let mut tle: *mut SidereonTle = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_load(
                line1.as_ptr(),
                line2.as_ptr(),
                SidereonTleOpsMode::Improved as u32,
                &mut tle,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!tle.is_null());

        let mut passes: *mut SidereonPassList = ptr::null_mut();
        let status = unsafe {
            sidereon_tle_find_passes(
                tle,
                &station,
                START_UNIX_US,
                END_UNIX_US,
                &c_options,
                &mut passes,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert!(!passes.is_null());

        let mut count = usize::MAX;
        assert_eq!(
            unsafe { sidereon_pass_list_count(passes, &mut count) },
            SidereonStatus::Ok
        );
        let mut actual = vec![
            SidereonSatellitePass {
                aos_unix_us: 0,
                los_unix_us: 0,
                culmination_unix_us: 0,
                max_elevation_deg: 0.0,
                duration_s: 0.0,
            };
            count
        ];
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_pass_list_values(
                    passes,
                    actual.as_mut_ptr(),
                    actual.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        actual.truncate(written);
        assert_eq!(required, expected_improved.len());

        assert!(pass_rows_match(&actual, &expected_improved));
        assert!(!pass_rows_match(&actual, &afspc_reference));

        unsafe {
            sidereon_pass_list_free(passes);
            sidereon_tle_free(tle);
        }
    }

    #[test]
    fn rtk_fixed_ambiguities_reject_missing_meter_value() {
        let cycles = vec![("G02".to_owned(), 4), ("G03".to_owned(), -7)];
        let meters = BTreeMap::from([("G02", 0.1)]);

        assert_invalid_argument(rtk_fixed_ambiguity_rows_to_c(
            "test_rtk_fixed_ambiguities",
            &cycles,
            &meters,
        ));

        let message = last_error();
        assert!(message.contains("test_rtk_fixed_ambiguities"));
        assert!(message.contains("G03"));
        assert!(message.contains("no meter value"));
    }

    #[test]
    fn ppp_fixed_ambiguities_reject_missing_meter_value() {
        let cycles = BTreeMap::from([("G02#arc0".to_owned(), 4), ("G03#arc0".to_owned(), -7)]);
        let meters = BTreeMap::from([("G02#arc0".to_owned(), 0.1)]);

        assert_invalid_argument(ppp_fixed_ambiguity_rows_to_c(
            "test_ppp_fixed_ambiguities",
            &cycles,
            &meters,
        ));

        let message = last_error();
        assert!(message.contains("test_ppp_fixed_ambiguities"));
        assert!(message.contains("G03#arc0"));
        assert!(message.contains("no meter value"));
    }

    // A V2 input whose GLONASS channel array is the given slice; every other
    // field is the engine default. The returned struct borrows `channels`, so
    // the caller must keep that slice alive for as long as the struct is used.
    fn spp_inputs_v2_with_glonass_channels(
        channels: &[SidereonGlonassChannel],
    ) -> SidereonSppInputsV2 {
        let mut inputs = default_spp_inputs_v2();
        inputs.glonass_channels = channels.as_ptr();
        inputs.glonass_channel_count = channels.len();
        inputs
    }

    #[test]
    fn glonass_channels_from_c_empty_is_no_channels() {
        // The default V2 inputs carry a null pointer and a zero count; that must
        // marshal to an empty map (no GLONASS channels), which is what keeps
        // every non-GLONASS solve bit-identical.
        let inputs = default_spp_inputs_v2();
        let channels =
            unsafe { glonass_channels_from_c("test_glonass_empty", &inputs) }.expect("empty ok");
        assert!(channels.is_empty());

        // An explicit zero-length array (non-null pointer would also be fine) is
        // equivalent.
        let inputs = spp_inputs_v2_with_glonass_channels(&[]);
        let channels =
            unsafe { glonass_channels_from_c("test_glonass_empty2", &inputs) }.expect("empty ok");
        assert!(channels.is_empty());
    }

    #[test]
    fn glonass_channels_from_c_marshals_slots_to_map() {
        let rows = [
            SidereonGlonassChannel {
                slot: 1,
                channel: 1,
            },
            SidereonGlonassChannel {
                slot: 2,
                channel: -4,
            },
            SidereonGlonassChannel {
                slot: 24,
                channel: 2,
            },
        ];
        let inputs = spp_inputs_v2_with_glonass_channels(&rows);
        let channels = unsafe { glonass_channels_from_c("test_glonass_map", &inputs) }.expect("ok");
        assert_eq!(channels, BTreeMap::from([(1u8, 1i8), (2, -4), (24, 2)]));
    }

    #[test]
    fn glonass_channels_from_c_rejects_duplicate_slot() {
        let rows = [
            SidereonGlonassChannel {
                slot: 3,
                channel: 5,
            },
            SidereonGlonassChannel {
                slot: 3,
                channel: 6,
            },
        ];
        let inputs = spp_inputs_v2_with_glonass_channels(&rows);
        let err = unsafe { glonass_channels_from_c("test_glonass_dup", &inputs) }
            .expect_err("duplicate slot must be rejected");
        assert_eq!(err, SidereonStatus::InvalidArgument);
        let message = last_error();
        assert!(message.contains("test_glonass_dup"));
        assert!(message.contains("duplicate glonass_channels slot 3"));
    }

    #[test]
    fn build_spp_solve_inputs_threads_glonass_channels_to_engine() {
        // One GLONASS observation so the resulting SolveInputs is realistic; the
        // pseudorange value is irrelevant to this marshalling test.
        let sat = CString::new("R01").unwrap();
        let observation = SidereonObservation {
            sat_id: sat.as_ptr(),
            pseudorange_m: 0.0,
        };
        let mut base = default_spp_inputs_v2().base;
        base.observations = &observation;
        base.observation_count = 1;

        let map = BTreeMap::from([(1u8, 1i8)]);
        let solve_inputs = unsafe {
            build_spp_solve_inputs("test_glonass_thread", &base, None, None, map.clone())
        }
        .expect("ok");
        // The channel map the C caller supplied must arrive verbatim on the
        // engine SolveInputs that the solver consumes.
        assert_eq!(solve_inputs.glonass_channels, map);

        // The legacy path supplies no channels, leaving the map empty.
        let solve_inputs = unsafe {
            build_spp_solve_inputs(
                "test_glonass_thread_empty",
                &base,
                None,
                None,
                BTreeMap::new(),
            )
        }
        .expect("ok");
        assert!(solve_inputs.glonass_channels.is_empty());
    }

    #[test]
    fn velocity_observable_from_c_maps_known_and_rejects_unknown() {
        assert!(matches!(
            velocity_observable_from_c(
                "test_vel",
                "observable",
                SidereonVelocityObservable::RangeRate as u32
            ),
            Ok(VelocityObservable::RangeRate)
        ));
        assert!(matches!(
            velocity_observable_from_c(
                "test_vel",
                "observable",
                SidereonVelocityObservable::Doppler as u32
            ),
            Ok(VelocityObservable::Doppler)
        ));
        let err = velocity_observable_from_c("test_vel", "observable", 9)
            .expect_err("unknown discriminant must be rejected");
        assert_eq!(err, SidereonStatus::InvalidArgument);
        let message = last_error();
        assert!(message.contains("test_vel"));
        assert!(message.contains("invalid observable velocity observable"));
    }

    #[test]
    fn staleness_policy_constructors_match_engine() {
        assert_eq!(
            sidereon_staleness_policy_default().max_staleness_s,
            StalenessPolicy::default().max_staleness_s
        );
        assert_eq!(
            sidereon_staleness_policy_days(2.0).max_staleness_s,
            2.0 * 86_400.0
        );
        assert_eq!(
            sidereon_staleness_policy_seconds(42.0).max_staleness_s,
            42.0
        );
    }

    #[test]
    fn degradation_kind_round_trips_every_variant() {
        assert_eq!(
            degradation_kind_to_c(DegradationKind::Exact),
            SidereonDegradationKind::Exact
        );
        assert_eq!(
            degradation_kind_to_c(DegradationKind::NearestPrior),
            SidereonDegradationKind::NearestPrior
        );
        assert_eq!(
            degradation_kind_to_c(DegradationKind::DiurnalShift),
            SidereonDegradationKind::DiurnalShift
        );
    }

    #[test]
    fn staleness_metadata_to_c_copies_fields() {
        let metadata = StalenessMetadata {
            kind: DegradationKind::NearestPrior,
            requested_epoch_j2000_s: 1000.0,
            source_epoch_j2000_s: 400.0,
            staleness_s: 600.0,
            staleness_days: 600.0 / 86_400.0,
        };
        let c = staleness_metadata_to_c(metadata);
        assert_eq!(c.kind, SidereonDegradationKind::NearestPrior);
        assert_eq!(c.requested_epoch_j2000_s, 1000.0);
        assert_eq!(c.source_epoch_j2000_s, 400.0);
        assert_eq!(c.staleness_s, 600.0);
        assert_eq!(c.staleness_days, 600.0 / 86_400.0);
    }

    #[test]
    fn map_selection_error_covers_every_variant() {
        let cases = [
            (
                SelectionError::EmptyProductSet,
                SidereonSelectionStatus::EmptyProductSet,
            ),
            (
                SelectionError::InvalidRange {
                    start_epoch_j2000_s: 1.0,
                    end_epoch_j2000_s: 0.0,
                },
                SidereonSelectionStatus::InvalidRange,
            ),
            (
                SelectionError::NoPriorProduct {
                    requested_epoch_j2000_s: 5.0,
                },
                SidereonSelectionStatus::NoPriorProduct,
            ),
            (
                SelectionError::BeyondStalenessCap {
                    requested_epoch_j2000_s: 5.0,
                    source_epoch_j2000_s: 1.0,
                    staleness_s: 4.0,
                    max_staleness_s: 1.0,
                },
                SidereonSelectionStatus::BeyondStalenessCap,
            ),
            (
                SelectionError::InvalidProduct("bad".to_owned()),
                SidereonSelectionStatus::InvalidProduct,
            ),
            (
                SelectionError::InvalidPolicy {
                    max_staleness_s: f64::NAN,
                },
                SidereonSelectionStatus::InvalidPolicy,
            ),
            (
                SelectionError::Overflow {
                    context: "end - hi",
                },
                SidereonSelectionStatus::Overflow,
            ),
        ];
        for (err, expected) in cases {
            // The pure mapping matches and leaves last_error untouched, so it is
            // safe to surface on a successful provenance readout.
            set_last_error("sentinel");
            assert_eq!(selection_error_to_status(&err), expected);
            assert_eq!(last_error(), "sentinel");
            // The failing-return mapping records the typed detail.
            assert_eq!(map_selection_error("test_sel", &err), expected);
            assert!(last_error().contains("test_sel"));
        }
    }

    #[test]
    fn marshal_status_maps_onto_typed_surfaces() {
        assert_eq!(
            marshal_status_to_selection(SidereonStatus::NullPointer),
            SidereonSelectionStatus::NullPointer
        );
        assert_eq!(
            marshal_status_to_selection(SidereonStatus::Solve),
            SidereonSelectionStatus::InvalidArgument
        );
        assert_eq!(
            marshal_status_to_fallback(SidereonStatus::NullPointer),
            SidereonFallbackStatus::NullPointer
        );
        assert_eq!(
            marshal_status_to_fallback(SidereonStatus::InvalidToken),
            SidereonFallbackStatus::InvalidToken
        );
    }
}
