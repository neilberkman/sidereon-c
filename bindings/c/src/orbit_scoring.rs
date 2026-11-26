use super::*;
use crate::engine_error::{
    core_error_value, engine_error_operation_boundary, record_engine_error,
    SidereonEngineErrorFamily,
};
use crate::rinex_clock::{clock_epoch_from_c, instant_to_clock_epoch};
use serde_json::json;
use sidereon_core::ephemeris::Sp3FrameReconciliationOptions;
use sidereon_core::orbit_scoring::{
    self as scoring, BroadcastOrbit, ClockDatum, FrameAgreement, GapPolicy, OrbitFrame,
    OrbitReferencePoint, OrbitRegime, OrbitScoreError, OrbitScoreOptions, OrbitScoreReport,
    OrbitSource, OrbitSourceState, OrbitTrajectory, PredictedSp3, RtnComponents, ScoreEpochs,
    SisreWeightSelection, SisreWeights, TrajectorySample, TriadVelocity,
};

fn score_error_value(error: &OrbitScoreError) -> serde_json::Value {
    use OrbitScoreError as E;
    let (kind, fields) = match error {
        E::InvalidInput { field, reason } => {
            ("invalid_input", json!({"field":field,"reason":reason}))
        }
        E::TimeSystemMismatch {
            orbit_source,
            reference,
        } => (
            "time_system_mismatch",
            json!({"orbit_source":orbit_source.abbrev(),"reference":reference.abbrev()}),
        ),
        E::TimeSystemLabelMismatch {
            orbit_source,
            reference,
        } => (
            "time_system_label_mismatch",
            json!({"orbit_source":orbit_source.label(),"reference":reference.label()}),
        ),
        E::InstantTimeSystemMismatch {
            field,
            index,
            found,
            expected,
        } => (
            "instant_time_system_mismatch",
            json!({"field":field,"index":index,"found":found.abbrev(),"expected":expected.abbrev()}),
        ),
        E::FrameMismatch {
            orbit_source,
            reference,
        } => (
            "frame_mismatch",
            json!({"orbit_source":orbit_source,"reference":reference}),
        ),
        E::FrameTransform(cause) => (
            "frame_transform",
            json!({"cause":crate::frame_catalog::frame_catalog_error_value(cause)}),
        ),
        E::MissingSatellite { product, satellite } => (
            "missing_satellite",
            json!({"product":scored_product_code(*product),"satellite":satellite.to_string()}),
        ),
        E::MissingEpoch {
            product,
            satellite,
            epoch_index,
            error,
        } => (
            "missing_epoch",
            json!({"product":scored_product_code(*product),"satellite":satellite.to_string(),"epoch_index":epoch_index,"cause":core_error_value(error)}),
        ),
        E::NoPredictionOrigin { satellite } => (
            "no_prediction_origin",
            json!({"satellite":satellite.to_string()}),
        ),
        E::NoSisreWeights { satellite, regime } => (
            "no_sisre_weights",
            json!({"satellite":satellite.to_string(),"regime":regime.to_string()}),
        ),
        E::UndeterminedOrbitRegime { satellite } => (
            "undetermined_orbit_regime",
            json!({"satellite":satellite.to_string()}),
        ),
        E::NonFiniteSourceState {
            satellite,
            epoch_index,
        } => (
            "non_finite_source_state",
            json!({"satellite":satellite.to_string(),"epoch_index":epoch_index}),
        ),
        E::DegenerateReferenceState {
            satellite,
            epoch_index,
        } => (
            "degenerate_reference_state",
            json!({"satellite":satellite.to_string(),"epoch_index":epoch_index}),
        ),
        E::NonFiniteResult { satellite } => (
            "non_finite_result",
            json!({"satellite":satellite.to_string()}),
        ),
        _ => ("unknown", json!({})),
    };
    json!({"kind":kind,"fields":fields})
}

pub(crate) fn map_score_error(operation: &str, error: OrbitScoreError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::OrbitScore,
        operation,
        score_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

fn map_source_error(operation: &str, error: sidereon_core::Error) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::OrbitScore,
        operation,
        core_error_value(&error),
    );
    extra_invalid_arg(operation, error)
}

/// One of the two products in an orbit score.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonScoredProduct {
    /// Orbit source being scored.
    Source = 0,
    /// Reference SP3.
    Reference = 1,
}

fn scored_product_code(product: scoring::ScoredProduct) -> u32 {
    match product {
        scoring::ScoredProduct::Source => SidereonScoredProduct::Source as u32,
        scoring::ScoredProduct::Reference => SidereonScoredProduct::Reference as u32,
    }
}

/// Cartesian velocity used to construct the orbit triad.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonTriadVelocity {
    /// Inertial velocity in Earth-fixed axes.
    Inertial = 0,
    /// Earth-fixed velocity.
    EarthFixed = 1,
}

/// Policy for orbit scoring gaps.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonGapPolicy {
    /// Count gaps and continue.
    Count = 0,
    /// Refuse the first gap.
    Refuse = 1,
}

/// Treatment of the clock datum when computing errors.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonClockDatum {
    /// Remove the median per epoch and constellation.
    EpochSystemMedian = 0,
    /// Use the clock differences as supplied.
    Unadjusted = 1,
}

/// Spacecraft reference point.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonOrbitReferencePoint {
    /// Centre of mass.
    CenterOfMass = 0,
    /// Antenna phase centre.
    AntennaPhaseCenter = 1,
}

fn reference_point_to_c(value: OrbitReferencePoint) -> SidereonOrbitReferencePoint {
    match value {
        OrbitReferencePoint::CenterOfMass => SidereonOrbitReferencePoint::CenterOfMass,
        OrbitReferencePoint::AntennaPhaseCenter => SidereonOrbitReferencePoint::AntennaPhaseCenter,
    }
}

/// Orbit class used by the SISRE weight table.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonOrbitRegime {
    /// Medium Earth orbit.
    Meo = 0,
    /// Inclined geosynchronous orbit.
    Igso = 1,
    /// Geostationary orbit.
    Geo = 2,
}

fn regime_to_c(value: OrbitRegime) -> SidereonOrbitRegime {
    match value {
        OrbitRegime::Meo => SidereonOrbitRegime::Meo,
        OrbitRegime::Igso => SidereonOrbitRegime::Igso,
        OrbitRegime::Geo => SidereonOrbitRegime::Geo,
    }
}

fn regime_from_c(operation: &str, value: u32) -> Result<OrbitRegime, SidereonStatus> {
    match value {
        0 => Ok(OrbitRegime::Meo),
        1 => Ok(OrbitRegime::Igso),
        2 => Ok(OrbitRegime::Geo),
        _ => Err(extra_invalid_arg(
            operation,
            format_args!("invalid orbit regime {value}"),
        )),
    }
}

/// Optional scalar result from an orbit operation. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonOptionalOrbitScalar {
    /// Whether a value is present.
    pub present: bool,
    /// Value when present.
    pub value: f64,
}

impl From<Option<f64>> for SidereonOptionalOrbitScalar {
    fn from(value: Option<f64>) -> Self {
        Self {
            present: value.is_some(),
            value: value.unwrap_or_default(),
        }
    }
}

/// Radial, transverse and normal components of a vector. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonRtnComponents {
    /// Radial component in metres.
    pub radial_m: f64,
    /// Transverse component in metres.
    pub transverse_m: f64,
    /// Normal component in metres.
    pub normal_m: f64,
}

impl From<SidereonRtnComponents> for RtnComponents {
    fn from(value: SidereonRtnComponents) -> Self {
        Self {
            radial_m: value.radial_m,
            transverse_m: value.transverse_m,
            normal_m: value.normal_m,
        }
    }
}

/// SISRE radial weight and squared transverse/normal weight. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonSisreWeights {
    /// Radial weight.
    pub radial: f64,
    /// Squared transverse and normal weight.
    pub transverse_normal_squared: f64,
}

impl From<SisreWeights> for SidereonSisreWeights {
    fn from(value: SisreWeights) -> Self {
        Self {
            radial: value.radial(),
            transverse_normal_squared: value.transverse_normal_squared(),
        }
    }
}

fn weights_from_c(
    operation: &str,
    weights: SidereonSisreWeights,
) -> Result<SisreWeights, SidereonStatus> {
    SisreWeights::new(weights.radial, weights.transverse_normal_squared)
        .map_err(|err| map_score_error(operation, err))
}

unsafe fn write_score_value<T: Copy>(
    operation: &str,
    out: *mut T,
    evaluate: impl FnOnce() -> Result<T, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        out.write(c_try!(evaluate()));
        SidereonStatus::Ok
    })
}

/// Project a difference into the radial, transverse and normal frame.
///
/// Safety: out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_rtn_components(
    difference_m: SidereonVector3,
    position_m: SidereonVector3,
    velocity_m_s: SidereonVector3,
    out: *mut SidereonRtnComponents,
) -> SidereonStatus {
    const OP: &str = "sidereon_rtn_components";
    write_score_value(OP, out, || {
        let result = scoring::rtn_components(difference_m.xyz, position_m.xyz, velocity_m_s.xyz)
            .map_err(|err| map_score_error(OP, err))?;
        Ok(SidereonRtnComponents {
            radial_m: result.radial_m,
            transverse_m: result.transverse_m,
            normal_m: result.normal_m,
        })
    })
}

/// Convert Earth-fixed velocity to inertial velocity in Earth-fixed axes.
///
/// Safety: out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_inertial_velocity_from_earth_fixed(
    position_ecef_m: SidereonVector3,
    velocity_ecef_m_s: SidereonVector3,
    out: *mut SidereonVector3,
) -> SidereonStatus {
    const OP: &str = "sidereon_inertial_velocity_from_earth_fixed";
    write_score_value(OP, out, || {
        scoring::inertial_velocity_from_earth_fixed(position_ecef_m.xyz, velocity_ecef_m_s.xyz)
            .map(|xyz| SidereonVector3 { xyz })
            .map_err(|err| map_score_error(OP, err))
    })
}

/// Identify the SISRE orbit regime of a bound inertial state. Writes false
/// to present when the state has no orbit regime.
///
/// Safety: present and out are writable disjoint outputs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_regime_from_inertial_state(
    position_m: SidereonVector3,
    inertial_velocity_m_s: SidereonVector3,
    present: *mut bool,
    out: *mut SidereonOrbitRegime,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_regime_from_inertial_state";
    write_score_value(OP, out, || {
        let present = require_out(present, OP, "present")?;
        let regime = OrbitRegime::from_inertial_state(position_m.xyz, inertial_velocity_m_s.xyz);
        present.write(regime.is_some());
        Ok(regime.map_or(SidereonOrbitRegime::Meo, regime_to_c))
    })
}

/// Look up published SISRE weights for a constellation and orbit regime.
///
/// Safety: present and out are writable disjoint outputs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sisre_weights_montenbruck_2018(
    system: u32,
    regime: u32,
    present: *mut bool,
    out: *mut SidereonSisreWeights,
) -> SidereonStatus {
    const OP: &str = "sidereon_sisre_weights_montenbruck_2018";
    write_score_value(OP, out, || {
        let system = gnss_system_from_c_code(OP, "system", system)?;
        let regime = regime_from_c(OP, regime)?;
        let present = require_out(present, OP, "present")?;
        let weights = SisreWeights::montenbruck_2018(system, regime);
        present.write(weights.is_some());
        Ok(weights.map(Into::into).unwrap_or_default())
    })
}

/// Default half step for the reference velocity estimate, seconds.
#[no_mangle]
pub extern "C" fn sidereon_default_velocity_half_step_s() -> f64 {
    scoring::DEFAULT_VELOCITY_HALF_STEP_S
}

/// Minimum reference velocity half step, seconds.
#[no_mangle]
pub extern "C" fn sidereon_min_velocity_half_step_s() -> f64 {
    scoring::MIN_VELOCITY_HALF_STEP_S
}

enum Source {
    Predicted {
        value: PredictedSp3<'static>,
        _product: Arc<Sp3>,
    },
    Broadcast {
        value: BroadcastOrbit<'static>,
        _ephemeris: Arc<BroadcastEphemeris>,
    },
    Trajectory(OrbitTrajectory),
}

impl Source {
    fn predicted(
        product: Arc<Sp3>,
        build: impl for<'a> FnOnce(&'a Sp3) -> Result<PredictedSp3<'a>, OrbitScoreError>,
    ) -> Result<Self, OrbitScoreError> {
        let value = build(&product)?;
        // The Arc keeps the immutable product at a stable address until after
        // value is dropped. The borrowed orbit is only exposed through &self.
        let value =
            unsafe { std::mem::transmute::<PredictedSp3<'_>, PredictedSp3<'static>>(value) };
        Ok(Self::Predicted {
            value,
            _product: product,
        })
    }
}

pub(crate) struct OwnedOrbitSource(Source);

impl OwnedOrbitSource {
    pub(crate) fn source(&self) -> &dyn OrbitSource {
        match &self.0 {
            Source::Predicted { value, .. } => value,
            Source::Broadcast { value, .. } => value,
            Source::Trajectory(value) => value,
        }
    }
}

/// Owned orbit-source handle. Backing products are retained independently.
pub struct SidereonOrbitSource {
    pub(crate) inner: Arc<OwnedOrbitSource>,
}

impl SidereonOrbitSource {
    fn source(&self) -> &dyn OrbitSource {
        self.inner.source()
    }
}

unsafe fn write_orbit_source<H>(
    operation: &str,
    out: *mut *mut SidereonOrbitSource,
    input: *const H,
    input_name: &str,
    build: impl FnOnce() -> Result<Source, SidereonStatus>,
) -> SidereonStatus {
    engine_error_operation_boundary(operation, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, operation, "out"));
        c_try!(reject_output_overlaps_handle(
            operation, out, "out", input, input_name
        ));
        out.write(ptr::null_mut());
        let inner = Arc::new(OwnedOrbitSource(c_try!(build())));
        write_boxed_handle(out, SidereonOrbitSource { inner });
        SidereonStatus::Ok
    })
}

/// Release an orbit source.
///
/// Safety: source is NULL or a live owned source.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_free(source: *mut SidereonOrbitSource) {
    ffi_boundary("sidereon_orbit_source_free", (), || free_boxed(source));
}

/// One Earth-fixed state from an orbit source. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SidereonOrbitSourceState {
    /// Earth-fixed position in metres.
    pub position_ecef_m: [f64; 3],
    /// Clock offset in seconds, when available.
    pub clock_s: SidereonOptionalOrbitScalar,
}

impl From<OrbitSourceState> for SidereonOrbitSourceState {
    fn from(value: OrbitSourceState) -> Self {
        Self {
            position_ecef_m: value.position_ecef_m,
            clock_s: value.clock_s.into(),
        }
    }
}

/// One observation of an Earth-fixed orbit trajectory.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonTrajectorySample {
    /// Satellite token.
    pub satellite: SidereonSatelliteToken,
    /// Sample epoch in its own time scale.
    pub epoch: SidereonClockEpoch,
    /// Position in the trajectory's declared frame, in metres.
    pub position_ecef_m: [f64; 3],
    /// Whether the clock offset is supplied.
    pub has_clock_s: bool,
    /// Clock offset in seconds when supplied.
    pub clock_s: f64,
}

unsafe fn trajectory_sample_from_c(
    operation: &str,
    sample: SidereonTrajectorySample,
) -> Result<TrajectorySample, SidereonStatus> {
    Ok(TrajectorySample {
        satellite: parse_satellite_token(operation, sample.satellite.bytes.as_ptr())?,
        epoch: clock_epoch_from_c(operation, "epoch", &sample.epoch)?,
        position_ecef_m: sample.position_ecef_m,
        clock_s: sample.has_clock_s.then_some(sample.clock_s),
    })
}

/// Compare two trajectory samples by value, including their optional clocks.
///
/// Safety: out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_trajectory_sample_equal(
    left: SidereonTrajectorySample,
    right: SidereonTrajectorySample,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_trajectory_sample_equal";
    write_score_value(OP, out, || {
        Ok(trajectory_sample_from_c(OP, left)? == trajectory_sample_from_c(OP, right)?)
    })
}

/// Create a sampled orbit trajectory with a single prediction origin.
///
/// Safety: frame_label points to frame_len bytes, samples to count readable
/// entries, and out to disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_trajectory_new(
    frame_label: *const u8,
    frame_len: usize,
    prediction_origin: SidereonClockEpoch,
    samples: *const SidereonTrajectorySample,
    count: usize,
    out: *mut *mut SidereonOrbitSource,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_trajectory_new";
    write_orbit_source(OP, out, ptr::null::<()>(), "input", || {
        let label = text_bytes_from_c(OP, frame_label, frame_len)?;
        let origin = clock_epoch_from_c(OP, "prediction_origin", &prediction_origin)?;
        let samples = require_slice(samples, count, OP, "samples")?;
        let samples: Vec<_> = samples
            .iter()
            .map(|&sample| trajectory_sample_from_c(OP, sample))
            .collect::<Result<_, _>>()?;
        OrbitTrajectory::new(label, origin, samples)
            .map(Source::Trajectory)
            .map_err(|err| map_score_error(OP, err))
    })
}

/// Build a broadcast orbit scored from the supplied prediction origin.
///
/// Safety: ephemeris is live during the call and out is disjoint writable
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_broadcast_orbit_new(
    ephemeris: *const SidereonBroadcastEphemeris,
    prediction_origin: SidereonClockEpoch,
    out: *mut *mut SidereonOrbitSource,
) -> SidereonStatus {
    const OP: &str = "sidereon_broadcast_orbit_new";
    write_orbit_source(OP, out, ephemeris, "ephemeris", || {
        let ephemeris = require_ref(ephemeris, OP, "ephemeris")?;
        let origin = clock_epoch_from_c(OP, "prediction_origin", &prediction_origin)?;
        let ephemeris = Arc::clone(&ephemeris.inner);
        let value =
            BroadcastOrbit::new(&ephemeris, origin).map_err(|err| map_score_error(OP, err))?;
        // The Arc outlives value, which is only exposed through &Source.
        let value = std::mem::transmute::<BroadcastOrbit<'_>, BroadcastOrbit<'static>>(value);
        Ok(Source::Broadcast {
            value,
            _ephemeris: ephemeris,
        })
    })
}

/// Use the product’s prediction flags to establish each satellite’s origin.
///
/// Safety: product is live during the call and out is disjoint writable
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_predicted_sp3_from_prediction_flags(
    product: *const SidereonSp3,
    out: *mut *mut SidereonOrbitSource,
) -> SidereonStatus {
    const OP: &str = "sidereon_predicted_sp3_from_prediction_flags";
    write_orbit_source(OP, out, product, "product", || {
        let product = require_ref(product, OP, "product")?;
        Source::predicted(Arc::new(product.inner.clone()), |product| {
            PredictedSp3::from_prediction_flags(product)
        })
        .map_err(|err| map_score_error(OP, err))
    })
}

/// Use the same cutoff as the prediction origin for every satellite.
///
/// Safety: product is live during the call and out is disjoint writable
/// handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_predicted_sp3_with_cutoff(
    product: *const SidereonSp3,
    cutoff: SidereonClockEpoch,
    out: *mut *mut SidereonOrbitSource,
) -> SidereonStatus {
    const OP: &str = "sidereon_predicted_sp3_with_cutoff";
    write_orbit_source(OP, out, product, "product", || {
        let product = require_ref(product, OP, "product")?;
        let cutoff = clock_epoch_from_c(OP, "cutoff", &cutoff)?;
        Source::predicted(Arc::new(product.inner.clone()), |product| {
            PredictedSp3::with_cutoff(product, cutoff)
        })
        .map_err(|err| map_score_error(OP, err))
    })
}

/// Compute the orbit-only SISRE in metres.
///
/// Safety: out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sisre_weights_orbit_only_m(
    weights: SidereonSisreWeights,
    components: SidereonRtnComponents,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_sisre_weights_orbit_only_m";
    write_score_value(OP, out, || {
        weights_from_c(OP, weights)?
            .orbit_only_m(components.into())
            .map_err(|err| map_score_error(OP, err))
    })
}

/// Compute SISRE including clock error in metres.
///
/// Safety: out points to writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sisre_weights_with_clock_m(
    weights: SidereonSisreWeights,
    components: SidereonRtnComponents,
    clock_m: f64,
    out: *mut f64,
) -> SidereonStatus {
    const OP: &str = "sidereon_sisre_weights_with_clock_m";
    write_score_value(OP, out, || {
        weights_from_c(OP, weights)?
            .with_clock_m(components.into(), clock_m)
            .map_err(|err| map_score_error(OP, err))
    })
}

fn trajectory_from_source<'a>(
    operation: &str,
    source: &'a SidereonOrbitSource,
) -> Result<&'a OrbitTrajectory, SidereonStatus> {
    match &source.inner.0 {
        Source::Trajectory(value) => Ok(value),
        _ => Err(extra_invalid_arg(
            operation,
            "source is not a sampled trajectory",
        )),
    }
}

/// Copy a sampled trajectory with the clock relativity flag set.
///
/// Safety: source is live and out points to disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_trajectory_with_clock_relativity_included(
    source: *const SidereonOrbitSource,
    included: bool,
    out: *mut *mut SidereonOrbitSource,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_trajectory_with_clock_relativity_included";
    write_orbit_source(OP, out, source, "source", || {
        let source = require_ref(source, OP, "source")?;
        Ok(Source::Trajectory(
            trajectory_from_source(OP, source)?
                .clone()
                .with_clock_relativity_included(included),
        ))
    })
}

/// Compare two sampled trajectories by value.
///
/// Safety: both sources are live and out is disjoint writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_trajectory_equal(
    left: *const SidereonOrbitSource,
    right: *const SidereonOrbitSource,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_trajectory_equal";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", left, "left")?;
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        let left = trajectory_from_source(OP, require_ref(left, OP, "left")?)?;
        let right = trajectory_from_source(OP, require_ref(right, OP, "right")?)?;
        Ok(left == right)
    })
}

/// Kind of frame declared by an orbit source.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonOrbitFrame {
    /// Terrestrial frame named by a label.
    Label = 0,
    /// Constellation-specific broadcast datums.
    BroadcastDatums = 1,
}

/// Orbit source metadata. The frame and SP3 time system labels are separate text outputs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidereonOrbitSourceInfo {
    /// Time scale as SidereonTimeScale.
    pub time_scale: u32,
    /// Whether the source specifically declares an SP3 time system label.
    pub has_sp3_time_system: bool,
    /// Coordinate frame kind.
    pub frame: SidereonOrbitFrame,
    /// Spacecraft reference point.
    pub reference_point: SidereonOrbitReferencePoint,
    /// Whether the clock includes periodic relativity.
    pub clock_includes_relativity: bool,
}

/// Read orbit source time, frame, reference point and clock information.
///
/// Safety: source is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_info(
    source: *const SidereonOrbitSource,
    out: *mut SidereonOrbitSourceInfo,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_source_info";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", source, "source")?;
        let source = require_ref(source, OP, "source")?.source();
        let frame = match source.frame() {
            OrbitFrame::Label(_) => SidereonOrbitFrame::Label,
            OrbitFrame::BroadcastDatums => SidereonOrbitFrame::BroadcastDatums,
        };
        Ok(SidereonOrbitSourceInfo {
            time_scale: time_scale_to_c_code(source.time_scale()),
            has_sp3_time_system: source.sp3_time_system().is_some(),
            frame,
            reference_point: reference_point_to_c(source.reference_point()),
            clock_includes_relativity: source.clock_includes_relativity(),
        })
    })
}

/// Evaluate the Earth-fixed state of a satellite at an epoch in the source time scale.
///
/// Safety: source is live, satellite is a NUL-terminated token and out is
/// disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_state_at_j2000_s(
    source: *const SidereonOrbitSource,
    satellite: *const c_char,
    epoch_j2000_s: f64,
    out: *mut SidereonOrbitSourceState,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_source_state_at_j2000_s";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", source, "source")?;
        let source = require_ref(source, OP, "source")?.source();
        let satellite = parse_satellite_token(OP, satellite)?;
        source
            .state_at_j2000_s(satellite, epoch_j2000_s)
            .map(Into::into)
            .map_err(|err| map_source_error(OP, err))
    })
}

/// Return a satellite's prediction origin in the source time scale.
///
/// Safety: source is live, satellite is a NUL-terminated token and out is
/// disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_prediction_origin_j2000_s(
    source: *const SidereonOrbitSource,
    satellite: *const c_char,
    out: *mut SidereonOptionalOrbitScalar,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_source_prediction_origin_j2000_s";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", source, "source")?;
        let source = require_ref(source, OP, "source")?.source();
        let satellite = parse_satellite_token(OP, satellite)?;
        Ok(source.prediction_origin_j2000_s(satellite).into())
    })
}

/// One complete UTF-8 frame label.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFrameLabel {
    /// Label bytes.
    pub data: *const u8,
    /// Number of bytes.
    pub len: usize,
}

/// A set of labels asserted to refer to the same terrestrial frame.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFrameLabelSet {
    /// Label records.
    pub labels: *const SidereonFrameLabel,
    /// Number of labels.
    pub label_count: usize,
}

/// Caller-asserted frame equivalences and optional catalog Helmert transforms.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3FrameReconciliationOptions {
    /// Sets of labels that identify equivalent frames.
    pub asserted_equivalent_label_sets: *const SidereonFrameLabelSet,
    /// Number of label sets.
    pub label_set_count: usize,
    /// Whether to apply catalog Helmert transforms.
    pub helmert: bool,
}

/// Epoch selection for an orbit score.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScoreEpochs {
    /// Evaluate at explicit instants when true, or reference record epochs otherwise.
    pub explicit_instants: bool,
    /// Ordered explicit instants in the reference time scale.
    pub instants: *const SidereonClockEpoch,
    /// Number of explicit instants.
    pub count: usize,
}

/// Orbit scoring configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitScoreOptions {
    /// Evaluation epochs.
    pub epochs: SidereonScoreEpochs,
    /// Horizon bin edges in seconds.
    pub horizon_bin_edges_s: *const f64,
    /// Number of horizon bin edges.
    pub horizon_bin_edge_count: usize,
    /// Orbit triad velocity selector as SidereonTriadVelocity.
    pub triad: u32,
    /// Half step of the reference velocity difference in seconds.
    pub velocity_half_step_s: f64,
    /// Optional fixed weights. NULL selects published constellation and regime weights.
    pub sisre_weights: *const SidereonSisreWeights,
    /// Clock datum selector as SidereonClockDatum.
    pub clock_datum: u32,
    /// Missing data policy as SidereonGapPolicy.
    pub gaps: u32,
    /// Coordinate frame reconciliation controls.
    pub frame_reconciliation: SidereonSp3FrameReconciliationOptions,
}

/// Default scoring controls for a caller-provided horizon edge array.
///
/// Safety: horizon_bin_edges_s has count entries and remains live through scoring.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_options_default(
    horizon_bin_edges_s: *const f64,
    count: usize,
) -> SidereonOrbitScoreOptions {
    SidereonOrbitScoreOptions {
        epochs: SidereonScoreEpochs {
            explicit_instants: false,
            instants: ptr::null(),
            count: 0,
        },
        horizon_bin_edges_s,
        horizon_bin_edge_count: count,
        triad: SidereonTriadVelocity::Inertial as u32,
        velocity_half_step_s: scoring::DEFAULT_VELOCITY_HALF_STEP_S,
        sisre_weights: ptr::null(),
        clock_datum: SidereonClockDatum::EpochSystemMedian as u32,
        gaps: SidereonGapPolicy::Count as u32,
        frame_reconciliation: SidereonSp3FrameReconciliationOptions {
            asserted_equivalent_label_sets: ptr::null(),
            label_set_count: 0,
            helmert: false,
        },
    }
}

unsafe fn score_epochs_from_c(
    operation: &str,
    input: SidereonScoreEpochs,
) -> Result<ScoreEpochs, SidereonStatus> {
    if input.explicit_instants {
        let instants = require_slice(input.instants, input.count, operation, "instants")?;
        Ok(ScoreEpochs::Instants(
            instants
                .iter()
                .map(|instant| clock_epoch_from_c(operation, "instant", instant))
                .collect::<Result<_, _>>()?,
        ))
    } else if input.count != 0 {
        Err(extra_invalid_arg(
            operation,
            "epoch count requires explicit instants",
        ))
    } else {
        Ok(ScoreEpochs::ReferenceRecords)
    }
}

unsafe fn frame_reconciliation_options_from_c(
    operation: &str,
    input: SidereonSp3FrameReconciliationOptions,
) -> Result<Sp3FrameReconciliationOptions, SidereonStatus> {
    let mut options = Sp3FrameReconciliationOptions::disabled();
    options.helmert = input.helmert;
    let sets = require_slice(
        input.asserted_equivalent_label_sets,
        input.label_set_count,
        operation,
        "asserted_equivalent_label_sets",
    )?;
    options.asserted_equivalent_label_sets = sets
        .iter()
        .map(|set| {
            let labels = require_slice(set.labels, set.label_count, operation, "frame labels")?;
            let labels = labels
                .iter()
                .map(|label| text_bytes_from_c(operation, label.data, label.len))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(sidereon_core::ephemeris::Sp3FrameLabelSet::new(labels))
        })
        .collect::<Result<_, SidereonStatus>>()?;
    Ok(options)
}

/// Compare epoch selections by value.
///
/// Safety: the explicit epoch arrays are readable for their declared lengths
/// and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_score_epochs_equal(
    left: SidereonScoreEpochs,
    right: SidereonScoreEpochs,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_score_epochs_equal";
    write_score_value(OP, out, || {
        Ok(score_epochs_from_c(OP, left)? == score_epochs_from_c(OP, right)?)
    })
}

/// Compare SP3 frame reconciliation controls by value.
///
/// Safety: the label arrays and UTF-8 bytes are readable for their declared
/// lengths and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_frame_reconciliation_options_equal(
    left: SidereonSp3FrameReconciliationOptions,
    right: SidereonSp3FrameReconciliationOptions,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_sp3_frame_reconciliation_options_equal";
    write_score_value(OP, out, || {
        Ok(frame_reconciliation_options_from_c(OP, left)?
            == frame_reconciliation_options_from_c(OP, right)?)
    })
}

/// Compare orbit scoring controls by value.
///
/// Safety: input arrays and any fixed weights are readable, and out is
/// disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_options_equal(
    left: SidereonOrbitScoreOptions,
    right: SidereonOrbitScoreOptions,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_options_equal";
    write_score_value(OP, out, || {
        Ok(score_options_from_c(OP, &left)? == score_options_from_c(OP, &right)?)
    })
}

unsafe fn score_options_from_c(
    operation: &str,
    input: &SidereonOrbitScoreOptions,
) -> Result<OrbitScoreOptions, SidereonStatus> {
    let edges = require_slice(
        input.horizon_bin_edges_s,
        input.horizon_bin_edge_count,
        operation,
        "horizon_bin_edges_s",
    )?;
    let mut options = OrbitScoreOptions::new(edges.to_vec());
    options.epochs = score_epochs_from_c(operation, input.epochs)?;
    options.triad = match input.triad {
        0 => TriadVelocity::Inertial,
        1 => TriadVelocity::EarthFixed,
        value => {
            return Err(extra_invalid_arg(
                operation,
                format_args!("invalid triad velocity {value}"),
            ))
        }
    };
    options.velocity_half_step_s = input.velocity_half_step_s;
    if let Some(weights) = input.sisre_weights.as_ref() {
        options.sisre_weights = SisreWeightSelection::Fixed(weights_from_c(operation, *weights)?);
    }
    options.clock_datum = match input.clock_datum {
        0 => ClockDatum::EpochSystemMedian,
        1 => ClockDatum::Unadjusted,
        value => {
            return Err(extra_invalid_arg(
                operation,
                format_args!("invalid clock datum {value}"),
            ))
        }
    };
    options.gaps = match input.gaps {
        0 => GapPolicy::Count,
        1 => GapPolicy::Refuse,
        value => {
            return Err(extra_invalid_arg(
                operation,
                format_args!("invalid gap policy {value}"),
            ))
        }
    };
    options.frame_reconciliation =
        frame_reconciliation_options_from_c(operation, input.frame_reconciliation)?;
    Ok(options)
}

/// Owned orbit score report. Release with sidereon_orbit_score_report_free.
pub struct SidereonOrbitScoreReport {
    inner: OrbitScoreReport,
}

/// Score an orbit source against a reference SP3 on the requested satellites.
///
/// Safety: both handles are live. satellites points to count NUL-terminated
/// tokens. options is readable with live input arrays. out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_score_orbit(
    source: *const SidereonOrbitSource,
    reference: *const SidereonSp3,
    satellites: *const *const c_char,
    count: usize,
    options: *const SidereonOrbitScoreOptions,
    out: *mut *mut SidereonOrbitScoreReport,
) -> SidereonStatus {
    const OP: &str = "sidereon_score_orbit";
    engine_error_operation_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", source, "source"
        ));
        c_try!(reject_output_overlaps_handle(
            OP,
            out,
            "out",
            reference,
            "reference"
        ));
        out.write(ptr::null_mut());
        let source = c_try!(require_ref(source, OP, "source"));
        let reference = c_try!(require_ref(reference, OP, "reference"));
        let satellites = c_try!(satellites_from_c_tokens(OP, satellites, count));
        let options = c_try!(score_options_from_c(
            OP,
            c_try!(require_ref(options, OP, "options"))
        ));
        let inner =
            c_try!(
                scoring::score_orbit(source.source(), &reference.inner, &satellites, &options)
                    .map_err(|err| map_score_error(OP, err))
            );
        out.write(Box::into_raw(Box::new(SidereonOrbitScoreReport { inner })));
        SidereonStatus::Ok
    })
}

/// Release an orbit score report.
///
/// Safety: report is NULL or live and owned.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_free(report: *mut SidereonOrbitScoreReport) {
    ffi_boundary("sidereon_orbit_score_report_free", (), || {
        free_boxed(report)
    });
}

/// Counts of requested satellite and epoch pairs. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonScoreCoverage {
    /// Number of requested pairs.
    pub epochs: usize,
    /// Number of pairs scored.
    pub scored: usize,
    /// Number of pairs with clock scores.
    pub clocked: usize,
    /// Number of pairs outside the horizon bins.
    pub outside_horizon_bins: usize,
    /// Number of pairs without a prediction origin.
    pub no_prediction_origin: usize,
    /// Number of pairs whose satellite is absent from the source.
    pub source_satellite_missing: usize,
    /// Number of pairs whose epoch is absent from the source.
    pub source_epoch_missing: usize,
    /// Number of pairs whose satellite is absent from the reference.
    pub reference_satellite_missing: usize,
    /// Number of pairs whose epoch is absent from the reference.
    pub reference_epoch_missing: usize,
}

impl From<scoring::ScoreCoverage> for SidereonScoreCoverage {
    fn from(value: scoring::ScoreCoverage) -> Self {
        Self {
            epochs: value.epochs,
            scored: value.scored,
            clocked: value.clocked,
            outside_horizon_bins: value.outside_horizon_bins,
            no_prediction_origin: value.no_prediction_origin,
            source_satellite_missing: value.source_satellite_missing,
            source_epoch_missing: value.source_epoch_missing,
            reference_satellite_missing: value.reference_satellite_missing,
            reference_epoch_missing: value.reference_epoch_missing,
        }
    }
}

/// Statistics of a range error series. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonErrorStats {
    /// Whether the series contains data.
    pub present: bool,
    /// Root mean square in metres.
    pub rms_m: f64,
    /// Mean error in metres.
    pub mean_m: f64,
    /// Maximum absolute error in metres.
    pub max_abs_m: f64,
}

impl From<Option<scoring::ErrorStats>> for SidereonErrorStats {
    fn from(value: Option<scoring::ErrorStats>) -> Self {
        Self {
            present: value.is_some(),
            rms_m: value.map_or(0.0, |v| v.rms_m),
            mean_m: value.map_or(0.0, |v| v.mean_m),
            max_abs_m: value.map_or(0.0, |v| v.max_abs_m),
        }
    }
}

/// Scores for a prediction horizon bin. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonHorizonBinStats {
    /// Inclusive bin start in seconds.
    pub horizon_start_s: f64,
    /// Bin end in seconds.
    pub horizon_end_s: f64,
    /// Number of orbit scores in the bin.
    pub count: usize,
    /// Radial position errors.
    pub radial: SidereonErrorStats,
    /// Transverse position errors.
    pub transverse: SidereonErrorStats,
    /// Normal position errors.
    pub normal: SidereonErrorStats,
    /// Three-dimensional position errors.
    pub orbit_3d: SidereonErrorStats,
    /// Orbit-only SISRE.
    pub sisre_orbit: SidereonErrorStats,
    /// Clock errors expressed in metres.
    pub clock: SidereonErrorStats,
    /// SISRE including clock offsets.
    pub sisre: SidereonErrorStats,
    /// Number of clock scores in the bin.
    pub clock_count: usize,
}

impl From<&scoring::HorizonBinStats> for SidereonHorizonBinStats {
    fn from(value: &scoring::HorizonBinStats) -> Self {
        Self {
            horizon_start_s: value.horizon_start_s,
            horizon_end_s: value.horizon_end_s,
            count: value.count,
            radial: value.radial.into(),
            transverse: value.transverse.into(),
            normal: value.normal.into(),
            orbit_3d: value.orbit_3d.into(),
            sisre_orbit: value.sisre_orbit.into(),
            clock: value.clock.into(),
            sisre: value.sisre.into(),
            clock_count: value.clock_count,
        }
    }
}

/// Summary for one satellite. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteScore {
    /// Satellite token.
    pub satellite: SidereonSatelliteToken,
    /// Prediction origin in the source time scale.
    pub prediction_origin_j2000_s: SidereonOptionalOrbitScalar,
    /// Whether the orbit regime is known.
    pub has_orbit_regime: bool,
    /// Orbit regime when known.
    pub orbit_regime: SidereonOrbitRegime,
    /// Whether SISRE weights were determined.
    pub has_sisre_weights: bool,
    /// SISRE weights when determined.
    pub sisre_weights: SidereonSisreWeights,
    /// Number of prediction horizon bins.
    pub bin_count: usize,
    /// Coverage for this satellite.
    pub coverage: SidereonScoreCoverage,
}

impl From<&scoring::SatelliteScore> for SidereonSatelliteScore {
    fn from(value: &scoring::SatelliteScore) -> Self {
        Self {
            satellite: satellite_token(value.satellite),
            prediction_origin_j2000_s: value.prediction_origin_j2000_s.into(),
            has_orbit_regime: value.orbit_regime.is_some(),
            orbit_regime: value
                .orbit_regime
                .map_or(SidereonOrbitRegime::Meo, regime_to_c),
            has_sisre_weights: value.sisre_weights.is_some(),
            sisre_weights: value.sisre_weights.map(Into::into).unwrap_or_default(),
            bin_count: value.bins.len(),
            coverage: value.coverage.into(),
        }
    }
}

/// Summary for one constellation. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSystemScore {
    /// Constellation.
    pub system: SidereonGnssSystem,
    /// Number of prediction horizon bins.
    pub bin_count: usize,
    /// Coverage for this constellation.
    pub coverage: SidereonScoreCoverage,
}

impl From<&scoring::SystemScore> for SidereonSystemScore {
    fn from(value: &scoring::SystemScore) -> Self {
        Self {
            system: gnss_system_to_c(value.system),
            bin_count: value.bins.len(),
            coverage: value.coverage.into(),
        }
    }
}

/// How the source and reference terrestrial frames were reconciled.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidereonFrameAgreementKind {
    /// Both products use the same frame label.
    SameLabel = 0,
    /// The caller asserted that the frame labels are equivalent.
    AssertedEquivalent = 1,
    /// A catalog Helmert transformation was used.
    Helmert = 2,
    /// The source uses constellation-specific broadcast datums.
    BroadcastDatums = 3,
}

/// Frame reconciliation applied in a score. Compare fields by value.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFrameAgreement {
    /// Frame agreement kind.
    pub kind: SidereonFrameAgreementKind,
    /// Source frame code for Helmert agreement.
    pub from: u32,
    /// Reference frame code for Helmert agreement.
    pub to: u32,
}

impl From<&FrameAgreement> for SidereonFrameAgreement {
    fn from(value: &FrameAgreement) -> Self {
        use SidereonFrameAgreementKind as K;
        let (kind, from, to) = match value {
            FrameAgreement::SameLabel => (K::SameLabel, 0, 0),
            FrameAgreement::AssertedEquivalent => (K::AssertedEquivalent, 0, 0),
            FrameAgreement::Helmert { from, to } => (
                K::Helmert,
                crate::sp3::sp3_terrestrial_frame_to_c(*from),
                crate::sp3::sp3_terrestrial_frame_to_c(*to),
            ),
            FrameAgreement::BroadcastDatums => (K::BroadcastDatums, 0, 0),
        };
        Self { kind, from, to }
    }
}

/// Aggregate score report and row counts.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonOrbitScoreSummary {
    /// Frame reconciliation applied.
    pub frame: SidereonFrameAgreement,
    /// Spacecraft reference point of the orbit source.
    pub source_reference_point: SidereonOrbitReferencePoint,
    /// Number of satellite summaries.
    pub satellite_count: usize,
    /// Number of constellation summaries.
    pub system_count: usize,
    /// Coverage over all requested satellites and epochs.
    pub coverage: SidereonScoreCoverage,
}

impl From<&OrbitScoreReport> for SidereonOrbitScoreSummary {
    fn from(value: &OrbitScoreReport) -> Self {
        Self {
            frame: (&value.frame).into(),
            source_reference_point: reference_point_to_c(value.source_reference_point),
            satellite_count: value.per_satellite.len(),
            system_count: value.per_system.len(),
            coverage: value.coverage.into(),
        }
    }
}

/// Read the aggregate orbit score summary.
///
/// Safety: report is live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_summary(
    report: *const SidereonOrbitScoreReport,
    out: *mut SidereonOrbitScoreSummary,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_summary";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", report, "report")?;
        Ok((&require_ref(report, OP, "report")?.inner).into())
    })
}

/// Copy the horizon bin edges in seconds.
/// NULL and zero capacity query the full count.
///
/// Safety: report is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_horizon_bin_edges_s(
    report: *const SidereonOrbitScoreReport,
    out: *mut f64,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_horizon_bin_edges_s";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            report,
            "report"
        ));
        let report = c_try!(require_ref(report, OP, "report"));

        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &report.inner.horizon_bin_edges_s,
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy satellite summaries in request order.
/// NULL and zero capacity query the full count.
///
/// Safety: report is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_per_satellite(
    report: *const SidereonOrbitScoreReport,
    out: *mut SidereonSatelliteScore,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_per_satellite";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            report,
            "report"
        ));
        let report = c_try!(require_ref(report, OP, "report"));

        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &report
                .inner
                .per_satellite
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy constellation summaries in order of first appearance.
/// NULL and zero capacity query the full count.
///
/// Safety: report is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_per_system(
    report: *const SidereonOrbitScoreReport,
    out: *mut SidereonSystemScore,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_per_system";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            report,
            "report"
        ));
        let report = c_try!(require_ref(report, OP, "report"));

        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &report
                .inner
                .per_system
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy a satellite’s prediction horizon bins.
/// NULL and zero capacity query the full count.
///
/// Safety: report is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_satellite_bins(
    report: *const SidereonOrbitScoreReport,
    index: usize,
    out: *mut SidereonHorizonBinStats,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_satellite_bins";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            report,
            "report"
        ));
        let report = c_try!(require_ref(report, OP, "report"));
        let Some(row) = report.inner.per_satellite.get(index) else {
            return extra_invalid_arg(OP, format_args!("satellite index {index} out of range"));
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &row.bins.iter().map(Into::into).collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy a constellation’s prediction horizon bins.
/// NULL and zero capacity query the full count.
///
/// Safety: report is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_system_bins(
    report: *const SidereonOrbitScoreReport,
    index: usize,
    out: *mut SidereonHorizonBinStats,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_system_bins";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            report,
            "report"
        ));
        let report = c_try!(require_ref(report, OP, "report"));
        let Some(row) = report.inner.per_system.get(index) else {
            return extra_invalid_arg(OP, format_args!("system index {index} out of range"));
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &row.bins.iter().map(Into::into).collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the sample epochs in order.
/// NULL and zero capacity query the full count.
///
/// Safety: source is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_trajectory_epochs(
    source: *const SidereonOrbitSource,
    out: *mut SidereonClockEpoch,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_trajectory_epochs";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            source,
            "source"
        ));
        let source = c_try!(require_ref(source, OP, "source"));
        let trajectory = c_try!(trajectory_from_source(OP, source));
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &trajectory
                .epochs()
                .iter()
                .map(instant_to_clock_epoch)
                .collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the sampled satellite tokens.
/// NULL and zero capacity query the full count.
///
/// Safety: source is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_trajectory_satellites(
    source: *const SidereonOrbitSource,
    out: *mut SidereonSatelliteToken,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_trajectory_satellites";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            source,
            "source"
        ));
        let source = c_try!(require_ref(source, OP, "source"));
        let trajectory = c_try!(trajectory_from_source(OP, source));
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            &trajectory
                .satellites()
                .map(satellite_token)
                .collect::<Vec<_>>(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the orbit source frame label as UTF-8 without a terminator.
/// NULL and zero capacity query the full count.
///
/// Safety: source is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_frame_label(
    source: *const SidereonOrbitSource,
    out: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_source_frame_label";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            source,
            "source"
        ));
        let source = c_try!(require_ref(source, OP, "source"));
        let label = match source.source().frame() {
            OrbitFrame::Label(label) => label,
            OrbitFrame::BroadcastDatums => String::new(),
        };
        c_try!(copy_prefix_to_c(
            OP,
            "out",
            label.as_bytes(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the declared SP3 time system label as UTF-8 without a terminator.
/// NULL and zero capacity query the full count.
///
/// Safety: source is live. All outputs are writable and disjoint from each
/// other and the handle. out has capacity entries.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_source_sp3_time_system(
    source: *const SidereonOrbitSource,
    out: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_source_sp3_time_system";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_handle_copy_counts(
            OP,
            out,
            capacity,
            out_written,
            out_required,
            source,
            "source"
        ));
        let source = c_try!(require_ref(source, OP, "source"));

        c_try!(copy_prefix_to_c(
            OP,
            "out",
            source
                .source()
                .sp3_time_system()
                .map_or("", |system| system.label())
                .as_bytes(),
            out,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the SP3 product backing a predicted-SP3 source. Release with sidereon_sp3_free.
///
/// Safety: source is live and out is disjoint writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_predicted_sp3_product(
    source: *const SidereonOrbitSource,
    out: *mut *mut SidereonSp3,
) -> SidereonStatus {
    const OP: &str = "sidereon_predicted_sp3_product";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, OP, "out"));
        c_try!(reject_output_overlaps_handle(
            OP, out, "out", source, "source"
        ));
        out.write(ptr::null_mut());
        let source = c_try!(require_ref(source, OP, "source"));
        let Source::Predicted { value: product, .. } = &source.inner.0 else {
            return extra_invalid_arg(OP, "source is not a predicted SP3");
        };
        write_boxed_handle(
            out,
            SidereonSp3 {
                inner: product.product().clone(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Compare two orbit score reports by value.
///
/// Safety: both reports are live and out is disjoint writable result storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_score_report_equal(
    left: *const SidereonOrbitScoreReport,
    right: *const SidereonOrbitScoreReport,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_score_report_equal";
    write_score_value(OP, out, || {
        reject_output_overlaps_handle(OP, out, "out", left, "left")?;
        reject_output_overlaps_handle(OP, out, "out", right, "right")?;
        Ok(require_ref(left, OP, "left")?.inner == require_ref(right, OP, "right")?.inner)
    })
}

/// Compare two SISRE weight selections by value.
/// A NULL weight pointer selects published constellation and regime weights.
///
/// Safety: each weight pointer is NULL or readable. out is writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sisre_weight_selection_equal(
    left: *const SidereonSisreWeights,
    right: *const SidereonSisreWeights,
    out: *mut bool,
) -> SidereonStatus {
    const OP: &str = "sidereon_sisre_weight_selection_equal";
    write_score_value(OP, out, || {
        let selection = |value: *const SidereonSisreWeights| {
            value
                .as_ref()
                .map_or(Ok(SisreWeightSelection::Montenbruck2018), |weights| {
                    weights_from_c(OP, *weights).map(SisreWeightSelection::Fixed)
                })
        };
        Ok(selection(left)? == selection(right)?)
    })
}

/// Copy the role of a scored product as UTF-8 without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_text has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_scored_product_text(
    value: u32,
    out_text: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_scored_product_text";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let text = c_try!(match value {
            0 => Ok(scoring::ScoredProduct::Source.to_string()),
            1 => Ok(scoring::ScoredProduct::Reference.to_string()),
            _ => Err(extra_invalid_arg(
                OP,
                format_args!("invalid scored product {value}")
            )),
        });
        c_try!(copy_prefix_to_c(
            OP,
            "out_text",
            text.as_bytes(),
            out_text,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

/// Copy the orbit regime as UTF-8 without a terminator.
/// NULL and zero capacity query the full byte count.
///
/// Safety: outputs are writable and disjoint. out_text has capacity bytes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_orbit_regime_text(
    value: u32,
    out_text: *mut u8,
    capacity: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const OP: &str = "sidereon_orbit_regime_text";
    ffi_boundary(OP, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(OP, out_written, out_required));
        let text = c_try!(regime_from_c(OP, value).map(|value| value.to_string()));
        c_try!(copy_prefix_to_c(
            OP,
            "out_text",
            text.as_bytes(),
            out_text,
            capacity,
            out_written,
            out_required
        ));
        SidereonStatus::Ok
    })
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn orbit_score_error_details() {
        use OrbitScoreError as E;
        let cases = [
            (
                E::InvalidInput {
                    field: "sample",
                    reason: "sample",
                },
                "invalid_input",
                json!({"field":"sample", "reason":"sample"}),
            ),
            (
                E::TimeSystemMismatch {
                    orbit_source: TimeScale::Gpst,
                    reference: TimeScale::Gpst,
                },
                "time_system_mismatch",
                json!({"orbit_source":"GPST", "reference":"GPST"}),
            ),
            (
                E::TimeSystemLabelMismatch {
                    orbit_source: sidereon_core::ephemeris::Sp3TimeSystem::Gps,
                    reference: sidereon_core::ephemeris::Sp3TimeSystem::Gps,
                },
                "time_system_label_mismatch",
                json!({"orbit_source":"GPS", "reference":"GPS"}),
            ),
            (
                E::InstantTimeSystemMismatch {
                    field: "sample",
                    index: 3,
                    found: TimeScale::Gpst,
                    expected: TimeScale::Gpst,
                },
                "instant_time_system_mismatch",
                json!({"field":"sample", "index":3, "found":"GPST", "expected":"GPST"}),
            ),
            (
                E::FrameMismatch {
                    orbit_source: String::from("sample"),
                    reference: String::from("sample"),
                },
                "frame_mismatch",
                json!({"orbit_source":"sample", "reference":"sample"}),
            ),
            (
                E::FrameTransform(
                    sidereon_core::frame_catalog::FrameCatalogError::InvalidInput {
                        field: "sample",
                        reason: "sample",
                    },
                ),
                "frame_transform",
                json!({"cause":{"kind":"invalid_input","fields":{"field":"sample","reason":"sample"}}}),
            ),
            (
                E::MissingSatellite {
                    product: scoring::ScoredProduct::Reference,
                    satellite: "G03".parse().unwrap(),
                },
                "missing_satellite",
                json!({"product":1, "satellite":"G03"}),
            ),
            (
                E::MissingEpoch {
                    product: scoring::ScoredProduct::Reference,
                    satellite: "G03".parse().unwrap(),
                    epoch_index: 3,
                    error: sidereon_core::Error::EpochOutOfRange,
                },
                "missing_epoch",
                json!({"product":1, "satellite":"G03", "epoch_index":3, "cause":{"kind":"epoch_out_of_range","fields":{}}}),
            ),
            (
                E::NoPredictionOrigin {
                    satellite: "G03".parse().unwrap(),
                },
                "no_prediction_origin",
                json!({"satellite":"G03"}),
            ),
            (
                E::NoSisreWeights {
                    satellite: "G03".parse().unwrap(),
                    regime: OrbitRegime::Meo,
                },
                "no_sisre_weights",
                json!({"satellite":"G03", "regime":"MEO"}),
            ),
            (
                E::UndeterminedOrbitRegime {
                    satellite: "G03".parse().unwrap(),
                },
                "undetermined_orbit_regime",
                json!({"satellite":"G03"}),
            ),
            (
                E::NonFiniteSourceState {
                    satellite: "G03".parse().unwrap(),
                    epoch_index: 3,
                },
                "non_finite_source_state",
                json!({"satellite":"G03", "epoch_index":3}),
            ),
            (
                E::DegenerateReferenceState {
                    satellite: "G03".parse().unwrap(),
                    epoch_index: 3,
                },
                "degenerate_reference_state",
                json!({"satellite":"G03", "epoch_index":3}),
            ),
            (
                E::NonFiniteResult {
                    satellite: "G03".parse().unwrap(),
                },
                "non_finite_result",
                json!({"satellite":"G03"}),
            ),
        ];
        for (error, kind, fields) in cases {
            assert_eq!(
                score_error_value(&error),
                json!({"kind":kind,"fields":fields})
            );
        }
    }
}
