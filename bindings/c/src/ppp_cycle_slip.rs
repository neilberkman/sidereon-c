use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use serde_json::{json, Value};
use sidereon_core::carrier_phase::SlipReason;
use sidereon_core::precise_positioning as core;

/// PPP dual-frequency cycle-slip detector configuration. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppCycleSlipConfig {
    /// Absolute Melbourne-Wubbena threshold, in wide-lane cycles.
    pub melbourne_wubbena_threshold_cycles: f64,
    /// Geometry-free phase-step threshold, in metres.
    pub geometry_free_threshold_m: f64,
    /// Minimum usable sample count for an initialized arc.
    pub minimum_arc_length: usize,
    /// Running-statistic sigma multiplier.
    pub running_statistic_k_factor: f64,
    /// Maximum continuous-arc gap, in seconds.
    pub maximum_gap_s: f64,
}

/// Raw dual-frequency PPP observation used by cycle-slip detection.
#[repr(C)]
pub struct SidereonPppCycleSlipObservation {
    /// Null-terminated UTF-8 satellite identifier.
    pub satellite_id: *const c_char,
    /// Null-terminated UTF-8 ambiguity identifier.
    pub ambiguity_id: *const c_char,
    /// First-band code pseudorange, in metres.
    pub p1_m: f64,
    /// Second-band code pseudorange, in metres.
    pub p2_m: f64,
    /// First-band carrier phase, in cycles.
    pub phi1_cyc: f64,
    /// Second-band carrier phase, in cycles.
    pub phi2_cyc: f64,
    /// First carrier frequency, in hertz.
    pub f1_hz: f64,
    /// Second carrier frequency, in hertz.
    pub f2_hz: f64,
    /// Whether `lli1` is present.
    pub has_lli1: bool,
    /// First-band loss-of-lock indicator.
    pub lli1: i64,
    /// Whether `lli2` is present.
    pub has_lli2: bool,
    /// Second-band loss-of-lock indicator.
    pub lli2: i64,
}

/// One dual-frequency PPP epoch used by cycle-slip detection.
#[repr(C)]
pub struct SidereonPppCycleSlipEpoch {
    /// Whether `gap_time_s` is present.
    pub has_gap_time_s: bool,
    /// Comparable epoch coordinate, in seconds.
    pub gap_time_s: f64,
    /// Optional exact epoch handle; borrowed for the call.
    pub gap_epoch: *const SidereonExactEpoch,
    /// Observation array.
    pub observations: *const SidereonPppCycleSlipObservation,
    /// Number of observations.
    pub observation_count: usize,
}

/// Snapshot of a running mean and variance.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppRunningMeanVarianceInfo {
    /// Accumulated sample count.
    pub sample_count: usize,
    /// Sample mean.
    pub mean: f64,
    /// Sample variance.
    pub variance: f64,
    /// Whether a sample standard deviation is available.
    pub has_standard_deviation: bool,
    /// Sample standard deviation when available.
    pub standard_deviation: f64,
}

/// Mutable engine running-statistic state.
pub struct SidereonPppRunningMeanVariance {
    inner: core::RunningMeanVariance,
}

/// Snapshot of one satellite cycle-slip state.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppSatelliteCycleSlipStateInfo {
    /// Whether a previous seconds coordinate is present.
    pub has_previous_epoch_time_s: bool,
    /// Previous seconds coordinate.
    pub previous_epoch_time_s: f64,
    /// Whether an exact previous epoch is present.
    pub has_previous_epoch: bool,
    /// Whether a previous Melbourne-Wubbena value is present.
    pub has_previous_melbourne_wubbena_cycles: bool,
    /// Previous Melbourne-Wubbena value, in cycles.
    pub previous_melbourne_wubbena_cycles: f64,
    /// Embedded running-statistic snapshot.
    pub melbourne_wubbena: SidereonPppRunningMeanVarianceInfo,
    /// Whether a previous geometry-free value is present.
    pub has_previous_geometry_free_m: bool,
    /// Previous geometry-free value, in metres.
    pub previous_geometry_free_m: f64,
}

/// Mutable per-satellite engine cycle-slip state.
pub struct SidereonPppSatelliteCycleSlipState {
    inner: core::SatelliteCycleSlipState,
}

/// Melbourne-Wubbena state-update result. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppMelbourneWubbenaUpdate {
    /// Melbourne-Wubbena combination, in wide-lane cycles.
    pub melbourne_wubbena_cycles: f64,
    /// Whether the detector classified a slip.
    pub slip: bool,
}

/// Geometry-free state-update result. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppGeometryFreeUpdate {
    /// Geometry-free phase combination, in metres.
    pub geometry_free_m: f64,
    /// Whether the detector classified a slip.
    pub slip: bool,
    /// Whether a long gap reset the arc.
    pub reset: bool,
}

/// Cycle-slip reason.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPppCycleSlipReason {
    /// Loss-of-lock bit 0 was set.
    Lli = 0,
    /// A data gap reset the arc.
    DataGap = 1,
    /// Geometry-free phase changed beyond its threshold.
    GeometryFree = 2,
    /// Melbourne-Wubbena changed beyond its threshold.
    MelbourneWubbena = 3,
}

/// Fixed fields for one flagged output epoch.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppCycleSlipFlagEpochInfo {
    /// Whether `gap_time_s` is present.
    pub has_gap_time_s: bool,
    /// Comparable seconds coordinate.
    pub gap_time_s: f64,
    /// Whether an exact epoch is present.
    pub has_gap_epoch: bool,
    /// Number of observation flags.
    pub observation_count: usize,
}

/// Fixed fields for one flagged observation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppCycleSlipFlagObservationInfo {
    /// Whether any detector classified a slip.
    pub slip: bool,
    /// Number of reasons.
    pub reason_count: usize,
}

/// Owned cycle-slip detection result.
pub struct SidereonPppCycleSlipDetection {
    inner: Vec<core::CycleSlipFlagEpoch>,
}

/// Cycle-slip configuration refusal kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
pub enum SidereonPppCycleSlipConfigErrorKind {
    /// Invalid Melbourne-Wubbena threshold.
    InvalidMelbourneWubbenaThreshold = 0,
    /// Invalid geometry-free threshold.
    InvalidGeometryFreeThreshold = 1,
    /// Zero minimum arc length.
    InvalidMinimumArcLength = 2,
    /// Invalid running-statistic multiplier.
    InvalidRunningStatisticKFactor = 3,
    /// Invalid maximum gap.
    InvalidMaximumGap = 4,
}

/// Cycle-slip operation refusal kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppCycleSlipErrorKind {
    /// Invalid detector configuration.
    InvalidConfig = 0,
    /// Non-finite observation scalar.
    NonFiniteObservation = 1,
    /// Non-finite seconds coordinate.
    NonFiniteEpochTime = 2,
    /// Epochs were not ordered.
    EpochsNotOrdered = 3,
    /// Invalid carrier frequency.
    InvalidFrequency = 4,
    /// Carrier frequencies were effectively equal.
    EqualFrequencies = 5,
}

/// Carrier-phase error input for the core cycle-slip error conversion.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppCycleSlipCarrierPhaseErrorKind {
    /// Frequencies were effectively equal.
    EqualFrequencies = 0,
    /// A frequency was invalid.
    InvalidFrequency = 1,
    /// An observation was invalid.
    InvalidObservation = 2,
    /// A threshold was invalid.
    InvalidThreshold = 3,
}

/// Lossless result of converting another core error to a cycle-slip error.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppCycleSlipConvertedError {
    /// Converted cycle-slip error kind.
    pub kind: SidereonPppCycleSlipErrorKind,
    /// Whether an invalid-config cause is present.
    pub has_config_cause: bool,
    /// Invalid-config cause when present.
    pub config_cause: SidereonPppCycleSlipConfigErrorKind,
}

fn config_to_core(value: SidereonPppCycleSlipConfig) -> core::CycleSlipConfig {
    let mut out = core::CycleSlipConfig::default();
    out.melbourne_wubbena_threshold_cycles = value.melbourne_wubbena_threshold_cycles;
    out.geometry_free_threshold_m = value.geometry_free_threshold_m;
    out.minimum_arc_length = value.minimum_arc_length;
    out.running_statistic_k_factor = value.running_statistic_k_factor;
    out.maximum_gap_s = value.maximum_gap_s;
    out
}

fn config_from_core(value: core::CycleSlipConfig) -> SidereonPppCycleSlipConfig {
    SidereonPppCycleSlipConfig {
        melbourne_wubbena_threshold_cycles: value.melbourne_wubbena_threshold_cycles,
        geometry_free_threshold_m: value.geometry_free_threshold_m,
        minimum_arc_length: value.minimum_arc_length,
        running_statistic_k_factor: value.running_statistic_k_factor,
        maximum_gap_s: value.maximum_gap_s,
    }
}

pub(crate) unsafe fn observation_from_c(
    fn_name: &str,
    value: &SidereonPppCycleSlipObservation,
) -> Result<core::DualFrequencyObservation, SidereonStatus> {
    Ok(core::DualFrequencyObservation {
        satellite_id: parse_c_string(fn_name, "observation.satellite_id", value.satellite_id)?,
        ambiguity_id: parse_c_string(fn_name, "observation.ambiguity_id", value.ambiguity_id)?,
        p1_m: value.p1_m,
        p2_m: value.p2_m,
        phi1_cyc: value.phi1_cyc,
        phi2_cyc: value.phi2_cyc,
        f1_hz: value.f1_hz,
        f2_hz: value.f2_hz,
        lli1: value.has_lli1.then_some(value.lli1),
        lli2: value.has_lli2.then_some(value.lli2),
    })
}

pub(crate) unsafe fn epochs_from_c(
    fn_name: &str,
    values: *const SidereonPppCycleSlipEpoch,
    count: usize,
) -> Result<Vec<core::DualFrequencyEpoch>, SidereonStatus> {
    require_slice(values, count, fn_name, "epochs")?
        .iter()
        .map(|epoch| {
            let observations = require_slice(
                epoch.observations,
                epoch.observation_count,
                fn_name,
                "epoch.observations",
            )?
            .iter()
            .map(|value| observation_from_c(fn_name, value))
            .collect::<Result<Vec<_>, _>>()?;
            Ok(core::DualFrequencyEpoch {
                gap_time_s: epoch.has_gap_time_s.then_some(epoch.gap_time_s),
                gap_epoch: epoch.gap_epoch.as_ref().map(|value| value.inner),
                observations,
            })
        })
        .collect()
}

fn config_error_value(error: core::CycleSlipConfigError) -> Value {
    let kind = match error {
        core::CycleSlipConfigError::InvalidMelbourneWubbenaThreshold => {
            "invalid_melbourne_wubbena_threshold"
        }
        core::CycleSlipConfigError::InvalidGeometryFreeThreshold => {
            "invalid_geometry_free_threshold"
        }
        core::CycleSlipConfigError::InvalidMinimumArcLength => "invalid_minimum_arc_length",
        core::CycleSlipConfigError::InvalidRunningStatisticKFactor => {
            "invalid_running_statistic_k_factor"
        }
        core::CycleSlipConfigError::InvalidMaximumGap => "invalid_maximum_gap",
    };
    json!({"kind": kind, "message": error.to_string(), "fields": {}})
}

fn error_value(error: core::CycleSlipError) -> Value {
    let kind = match error {
        core::CycleSlipError::InvalidConfig(_) => "invalid_config",
        core::CycleSlipError::NonFiniteObservation => "non_finite_observation",
        core::CycleSlipError::NonFiniteEpochTime => "non_finite_epoch_time",
        core::CycleSlipError::EpochsNotOrdered => "epochs_not_ordered",
        core::CycleSlipError::InvalidFrequency => "invalid_frequency",
        core::CycleSlipError::EqualFrequencies => "equal_frequencies",
    };
    let fields = match error {
        core::CycleSlipError::InvalidConfig(cause) => json!({"cause": config_error_value(cause)}),
        _ => json!({}),
    };
    json!({"kind": kind, "message": error.to_string(), "fields": fields})
}

fn map_error(fn_name: &str, error: core::CycleSlipError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PppCycleSlip,
        fn_name,
        error_value(error),
    );
    set_last_error(format!("{fn_name}: {error}"));
    SidereonStatus::InvalidArgument
}

fn statistic_info(value: core::RunningMeanVariance) -> SidereonPppRunningMeanVarianceInfo {
    let standard_deviation = value.standard_deviation();
    SidereonPppRunningMeanVarianceInfo {
        sample_count: value.sample_count,
        mean: value.mean,
        variance: value.variance,
        has_standard_deviation: standard_deviation.is_some(),
        standard_deviation: standard_deviation.unwrap_or(0.0),
    }
}

fn reason_from_core(value: SlipReason) -> SidereonPppCycleSlipReason {
    match value {
        SlipReason::Lli => SidereonPppCycleSlipReason::Lli,
        SlipReason::DataGap => SidereonPppCycleSlipReason::DataGap,
        SlipReason::GeometryFree => SidereonPppCycleSlipReason::GeometryFree,
        SlipReason::MelbourneWubbena => SidereonPppCycleSlipReason::MelbourneWubbena,
    }
}

fn converted_error(value: core::CycleSlipError) -> SidereonPppCycleSlipConvertedError {
    match value {
        core::CycleSlipError::InvalidConfig(cause) => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::InvalidConfig,
            has_config_cause: true,
            config_cause: match cause {
                core::CycleSlipConfigError::InvalidMelbourneWubbenaThreshold => {
                    SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold
                }
                core::CycleSlipConfigError::InvalidGeometryFreeThreshold => {
                    SidereonPppCycleSlipConfigErrorKind::InvalidGeometryFreeThreshold
                }
                core::CycleSlipConfigError::InvalidMinimumArcLength => {
                    SidereonPppCycleSlipConfigErrorKind::InvalidMinimumArcLength
                }
                core::CycleSlipConfigError::InvalidRunningStatisticKFactor => {
                    SidereonPppCycleSlipConfigErrorKind::InvalidRunningStatisticKFactor
                }
                core::CycleSlipConfigError::InvalidMaximumGap => {
                    SidereonPppCycleSlipConfigErrorKind::InvalidMaximumGap
                }
            },
        },
        core::CycleSlipError::NonFiniteObservation => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::NonFiniteObservation,
            has_config_cause: false,
            config_cause: SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold,
        },
        core::CycleSlipError::NonFiniteEpochTime => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::NonFiniteEpochTime,
            has_config_cause: false,
            config_cause: SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold,
        },
        core::CycleSlipError::EpochsNotOrdered => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::EpochsNotOrdered,
            has_config_cause: false,
            config_cause: SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold,
        },
        core::CycleSlipError::InvalidFrequency => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::InvalidFrequency,
            has_config_cause: false,
            config_cause: SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold,
        },
        core::CycleSlipError::EqualFrequencies => SidereonPppCycleSlipConvertedError {
            kind: SidereonPppCycleSlipErrorKind::EqualFrequencies,
            has_config_cause: false,
            config_cause: SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold,
        },
    }
}

unsafe fn copy_cycle_slip_text(
    fn_name: &str,
    text: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    match copy_prefix_to_c(
        fn_name,
        "out",
        text.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    ) {
        Ok(()) => SidereonStatus::Ok,
        Err(status) => status,
    }
}

/// Return the engine default cycle-slip configuration.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_config_default(
    out: *mut SidereonPppCycleSlipConfig,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_config_default";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        *c_try!(require_out(out, fn_name, "out")) =
            config_from_core(core::CycleSlipConfig::default());
        SidereonStatus::Ok
    })
}

/// Validate cycle-slip configuration through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_config_validate(
    config: SidereonPppCycleSlipConfig,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_config_validate";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        match config_to_core(config).validate() {
            Ok(()) => SidereonStatus::Ok,
            Err(error) => map_error(fn_name, error.into()),
        }
    })
}

/// Return core display text for a configuration error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_config_error_text(
    kind: SidereonPppCycleSlipConfigErrorKind,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let error = match kind {
        SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold => {
            core::CycleSlipConfigError::InvalidMelbourneWubbenaThreshold
        }
        SidereonPppCycleSlipConfigErrorKind::InvalidGeometryFreeThreshold => {
            core::CycleSlipConfigError::InvalidGeometryFreeThreshold
        }
        SidereonPppCycleSlipConfigErrorKind::InvalidMinimumArcLength => {
            core::CycleSlipConfigError::InvalidMinimumArcLength
        }
        SidereonPppCycleSlipConfigErrorKind::InvalidRunningStatisticKFactor => {
            core::CycleSlipConfigError::InvalidRunningStatisticKFactor
        }
        SidereonPppCycleSlipConfigErrorKind::InvalidMaximumGap => {
            core::CycleSlipConfigError::InvalidMaximumGap
        }
    };
    copy_cycle_slip_text(
        "sidereon_ppp_cycle_slip_config_error_text",
        &error.to_string(),
        out,
        out_len,
        out_written,
        out_required,
    )
}

/// Return core display text for a cycle-slip error kind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_error_text(
    kind: SidereonPppCycleSlipErrorKind,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let error = match kind {
        SidereonPppCycleSlipErrorKind::InvalidConfig => {
            core::CycleSlipError::InvalidConfig(core::CycleSlipConfigError::InvalidMinimumArcLength)
        }
        SidereonPppCycleSlipErrorKind::NonFiniteObservation => {
            core::CycleSlipError::NonFiniteObservation
        }
        SidereonPppCycleSlipErrorKind::NonFiniteEpochTime => {
            core::CycleSlipError::NonFiniteEpochTime
        }
        SidereonPppCycleSlipErrorKind::EpochsNotOrdered => core::CycleSlipError::EpochsNotOrdered,
        SidereonPppCycleSlipErrorKind::InvalidFrequency => core::CycleSlipError::InvalidFrequency,
        SidereonPppCycleSlipErrorKind::EqualFrequencies => core::CycleSlipError::EqualFrequencies,
    };
    copy_cycle_slip_text(
        "sidereon_ppp_cycle_slip_error_text",
        &error.to_string(),
        out,
        out_len,
        out_written,
        out_required,
    )
}

/// Convert a core configuration error to its cycle-slip error through `From`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_error_from_config(
    kind: SidereonPppCycleSlipConfigErrorKind,
    out: *mut SidereonPppCycleSlipConvertedError,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_error_from_config";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let cause = match kind {
            SidereonPppCycleSlipConfigErrorKind::InvalidMelbourneWubbenaThreshold => {
                core::CycleSlipConfigError::InvalidMelbourneWubbenaThreshold
            }
            SidereonPppCycleSlipConfigErrorKind::InvalidGeometryFreeThreshold => {
                core::CycleSlipConfigError::InvalidGeometryFreeThreshold
            }
            SidereonPppCycleSlipConfigErrorKind::InvalidMinimumArcLength => {
                core::CycleSlipConfigError::InvalidMinimumArcLength
            }
            SidereonPppCycleSlipConfigErrorKind::InvalidRunningStatisticKFactor => {
                core::CycleSlipConfigError::InvalidRunningStatisticKFactor
            }
            SidereonPppCycleSlipConfigErrorKind::InvalidMaximumGap => {
                core::CycleSlipConfigError::InvalidMaximumGap
            }
        };
        *c_try!(require_out(out, fn_name, "out")) = converted_error(cause.into());
        SidereonStatus::Ok
    })
}

/// Convert a core carrier-phase error to its cycle-slip error through `From`.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_error_from_carrier_phase(
    kind: SidereonPppCycleSlipCarrierPhaseErrorKind,
    out: *mut SidereonPppCycleSlipConvertedError,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_error_from_carrier_phase";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let cause = match kind {
            SidereonPppCycleSlipCarrierPhaseErrorKind::EqualFrequencies => {
                sidereon_core::carrier_phase::CarrierPhaseError::EqualFrequencies
            }
            SidereonPppCycleSlipCarrierPhaseErrorKind::InvalidFrequency => {
                sidereon_core::carrier_phase::CarrierPhaseError::InvalidFrequency
            }
            SidereonPppCycleSlipCarrierPhaseErrorKind::InvalidObservation => {
                sidereon_core::carrier_phase::CarrierPhaseError::InvalidObservation
            }
            SidereonPppCycleSlipCarrierPhaseErrorKind::InvalidThreshold => {
                sidereon_core::carrier_phase::CarrierPhaseError::InvalidThreshold
            }
        };
        *c_try!(require_out(out, fn_name, "out")) = converted_error(cause.into());
        SidereonStatus::Ok
    })
}

/// Allocate an empty engine running statistic.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_running_mean_variance_new(
    out: *mut *mut SidereonPppRunningMeanVariance,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_running_mean_variance_new";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        write_boxed_handle(
            out,
            SidereonPppRunningMeanVariance {
                inner: core::RunningMeanVariance::new(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Push one value into a mutable engine running statistic.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_running_mean_variance_push(
    state: *mut SidereonPppRunningMeanVariance,
    value: f64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_running_mean_variance_push";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(require_mut(state, fn_name, "state"))
            .inner
            .push(value);
        SidereonStatus::Ok
    })
}

/// Read a running-statistic snapshot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_running_mean_variance_info(
    state: *const SidereonPppRunningMeanVariance,
    out: *mut SidereonPppRunningMeanVarianceInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_running_mean_variance_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", state, "state"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(statistic_info(
            c_try!(require_ref(state, fn_name, "state")).inner,
        ));
        SidereonStatus::Ok
    })
}

/// Compare two running-statistic handles by engine value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_running_mean_variance_equal(
    left: *const SidereonPppRunningMeanVariance,
    right: *const SidereonPppRunningMeanVariance,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_running_mean_variance_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) =
            c_try!(require_ref(left, fn_name, "left")).inner
                == c_try!(require_ref(right, fn_name, "right")).inner;
        SidereonStatus::Ok
    })
}

/// Free a running-statistic handle. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_running_mean_variance_free(
    state: *mut SidereonPppRunningMeanVariance,
) {
    free_boxed(state);
}

/// Allocate an empty per-satellite cycle-slip state.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_new(
    out: *mut *mut SidereonPppSatelliteCycleSlipState,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_satellite_cycle_slip_state_new";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        write_boxed_handle(
            out,
            SidereonPppSatelliteCycleSlipState {
                inner: core::SatelliteCycleSlipState::new(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Reset a per-satellite cycle-slip state through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_reset_arc(
    state: *mut SidereonPppSatelliteCycleSlipState,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_satellite_cycle_slip_state_reset_arc";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(require_mut(state, fn_name, "state"))
            .inner
            .reset_arc();
        SidereonStatus::Ok
    })
}

/// Read a per-satellite cycle-slip state snapshot.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_info(
    state: *const SidereonPppSatelliteCycleSlipState,
    out: *mut SidereonPppSatelliteCycleSlipStateInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_satellite_cycle_slip_state_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", state, "state"
        ));
        let value = &c_try!(require_ref(state, fn_name, "state")).inner;
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(SidereonPppSatelliteCycleSlipStateInfo {
            has_previous_epoch_time_s: value.previous_epoch_time_s.is_some(),
            previous_epoch_time_s: value.previous_epoch_time_s.unwrap_or(0.0),
            has_previous_epoch: value.previous_epoch.is_some(),
            has_previous_melbourne_wubbena_cycles: value
                .previous_melbourne_wubbena_cycles
                .is_some(),
            previous_melbourne_wubbena_cycles: value
                .previous_melbourne_wubbena_cycles
                .unwrap_or(0.0),
            melbourne_wubbena: statistic_info(value.melbourne_wubbena),
            has_previous_geometry_free_m: value.previous_geometry_free_m.is_some(),
            previous_geometry_free_m: value.previous_geometry_free_m.unwrap_or(0.0),
        });
        SidereonStatus::Ok
    })
}

/// Clone the previous exact epoch from a cycle-slip state.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_previous_epoch(
    state: *const SidereonPppSatelliteCycleSlipState,
    out: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_satellite_cycle_slip_state_previous_epoch";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", state, "state"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        if let Some(inner) = c_try!(require_ref(state, fn_name, "state"))
            .inner
            .previous_epoch
        {
            write_boxed_handle(out, SidereonExactEpoch { inner });
        }
        SidereonStatus::Ok
    })
}

/// Compare two per-satellite states by engine value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_equal(
    left: *const SidereonPppSatelliteCycleSlipState,
    right: *const SidereonPppSatelliteCycleSlipState,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_satellite_cycle_slip_state_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) =
            c_try!(require_ref(left, fn_name, "left")).inner
                == c_try!(require_ref(right, fn_name, "right")).inner;
        SidereonStatus::Ok
    })
}

/// Free a per-satellite cycle-slip state. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_satellite_cycle_slip_state_free(
    state: *mut SidereonPppSatelliteCycleSlipState,
) {
    free_boxed(state);
}

/// Compute Melbourne-Wubbena in wide-lane cycles through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_melbourne_wubbena_cycles(
    observation: *const SidereonPppCycleSlipObservation,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_melbourne_wubbena_cycles";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(0.0);
        let observation = c_try!(observation_from_c(
            fn_name,
            c_try!(require_ref(observation, fn_name, "observation"))
        ));
        match core::melbourne_wubbena_cycles(&observation) {
            Ok(value) => {
                out.write(value);
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Update mutable Melbourne-Wubbena detector state through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_update_melbourne_wubbena(
    state: *mut SidereonPppSatelliteCycleSlipState,
    observation: *const SidereonPppCycleSlipObservation,
    config: SidereonPppCycleSlipConfig,
    out: *mut SidereonPppMelbourneWubbenaUpdate,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_update_melbourne_wubbena";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", state, "state"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(SidereonPppMelbourneWubbenaUpdate {
            melbourne_wubbena_cycles: 0.0,
            slip: false,
        });
        let observation = c_try!(observation_from_c(
            fn_name,
            c_try!(require_ref(observation, fn_name, "observation"))
        ));
        match core::update_melbourne_wubbena(
            &mut c_try!(require_mut(state, fn_name, "state")).inner,
            &observation,
            config_to_core(config),
        ) {
            Ok(value) => {
                out.write(SidereonPppMelbourneWubbenaUpdate {
                    melbourne_wubbena_cycles: value.melbourne_wubbena_cycles,
                    slip: value.slip,
                });
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Compute geometry-free phase in metres through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_geometry_free_m(
    observation: *const SidereonPppCycleSlipObservation,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_geometry_free_m";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(0.0);
        let observation = c_try!(observation_from_c(
            fn_name,
            c_try!(require_ref(observation, fn_name, "observation"))
        ));
        match core::geometry_free_m(&observation) {
            Ok(value) => {
                out.write(value);
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Update mutable geometry-free detector state through the engine.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_update_geometry_free(
    state: *mut SidereonPppSatelliteCycleSlipState,
    observation: *const SidereonPppCycleSlipObservation,
    has_epoch_time_s: bool,
    epoch_time_s: f64,
    epoch: *const SidereonExactEpoch,
    config: SidereonPppCycleSlipConfig,
    out: *mut SidereonPppGeometryFreeUpdate,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_update_geometry_free";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", state, "state"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(SidereonPppGeometryFreeUpdate {
            geometry_free_m: 0.0,
            slip: false,
            reset: false,
        });
        let observation = c_try!(observation_from_c(
            fn_name,
            c_try!(require_ref(observation, fn_name, "observation"))
        ));
        let exact = epoch.as_ref().map(|value| value.inner);
        match core::update_geometry_free(
            &mut c_try!(require_mut(state, fn_name, "state")).inner,
            &observation,
            has_epoch_time_s.then_some(epoch_time_s),
            exact,
            config_to_core(config),
        ) {
            Ok(value) => {
                out.write(SidereonPppGeometryFreeUpdate {
                    geometry_free_m: value.geometry_free_m,
                    slip: value.slip,
                    reset: value.reset,
                });
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Detect cycle slips across ordered dual-frequency PPP epochs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_detect_cycle_slips(
    epochs: *const SidereonPppCycleSlipEpoch,
    epoch_count: usize,
    config: SidereonPppCycleSlipConfig,
    out: *mut *mut SidereonPppCycleSlipDetection,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_detect_cycle_slips";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        let epochs = c_try!(epochs_from_c(fn_name, epochs, epoch_count));
        match core::detect_cycle_slips(&epochs, config_to_core(config)) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppCycleSlipDetection { inner });
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Return the number of flagged epochs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_epoch_count(
    result: *const SidereonPppCycleSlipDetection,
    out: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_epoch_count";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        *c_try!(require_out(out, fn_name, "out")) =
            c_try!(require_ref(result, fn_name, "result")).inner.len();
        SidereonStatus::Ok
    })
}

unsafe fn flagged_epoch<'a>(
    fn_name: &str,
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
) -> Result<&'a core::CycleSlipFlagEpoch, SidereonStatus> {
    require_ref(result, fn_name, "result")?
        .inner
        .get(epoch_index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: epoch_index out of range"));
            SidereonStatus::InvalidArgument
        })
}
unsafe fn flagged_observation<'a>(
    fn_name: &str,
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    observation_index: usize,
) -> Result<&'a core::CycleSlipFlagObservation, SidereonStatus> {
    flagged_epoch(fn_name, result, epoch_index)?
        .observations
        .get(observation_index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: observation_index out of range"));
            SidereonStatus::InvalidArgument
        })
}

/// Read fixed fields for one flagged epoch.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_epoch_info(
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    out: *mut SidereonPppCycleSlipFlagEpochInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_epoch_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(flagged_epoch(fn_name, result, epoch_index));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppCycleSlipFlagEpochInfo {
            has_gap_time_s: value.gap_time_s.is_some(),
            gap_time_s: value.gap_time_s.unwrap_or(0.0),
            has_gap_epoch: value.gap_epoch.is_some(),
            observation_count: value.observations.len(),
        });
        SidereonStatus::Ok
    })
}

/// Clone an exact epoch from one flagged epoch, or return null when absent.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_epoch_exact_epoch(
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    out: *mut *mut SidereonExactEpoch,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_epoch_exact_epoch";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        if let Some(inner) = c_try!(flagged_epoch(fn_name, result, epoch_index)).gap_epoch {
            write_boxed_handle(out, SidereonExactEpoch { inner });
        }
        SidereonStatus::Ok
    })
}

/// Read fixed fields for one flagged observation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_observation_info(
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    observation_index: usize,
    out: *mut SidereonPppCycleSlipFlagObservationInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_observation_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(flagged_observation(
            fn_name,
            result,
            epoch_index,
            observation_index
        ));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppCycleSlipFlagObservationInfo {
            slip: value.slip,
            reason_count: value.reasons.len(),
        });
        SidereonStatus::Ok
    })
}

/// Copy one flagged observation's satellite identifier losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_observation_satellite_id(
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    observation_index: usize,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_observation_satellite_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(flagged_observation(
            fn_name,
            result,
            epoch_index,
            observation_index
        ));
        copy_cycle_slip_text(
            fn_name,
            &value.satellite_id,
            out,
            out_len,
            out_written,
            out_required,
        )
    })
}

/// Read one deterministic cycle-slip reason.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_observation_reason(
    result: *const SidereonPppCycleSlipDetection,
    epoch_index: usize,
    observation_index: usize,
    reason_index: usize,
    out: *mut SidereonPppCycleSlipReason,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_observation_reason";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(flagged_observation(
            fn_name,
            result,
            epoch_index,
            observation_index
        ));
        let reason = c_try!(value.reasons.get(reason_index).copied().ok_or_else(|| {
            set_last_error(format!("{fn_name}: reason_index out of range"));
            SidereonStatus::InvalidArgument
        }));
        *c_try!(require_out(out, fn_name, "out")) = reason_from_core(reason);
        SidereonStatus::Ok
    })
}

/// Compare two detection results by engine value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_equal(
    left: *const SidereonPppCycleSlipDetection,
    right: *const SidereonPppCycleSlipDetection,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_cycle_slip_detection_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            left,
            "left"
        ));
        c_try!(reject_output_overlaps_handle(
            fn_name,
            out_equal,
            "out_equal",
            right,
            "right"
        ));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) =
            c_try!(require_ref(left, fn_name, "left")).inner
                == c_try!(require_ref(right, fn_name, "right")).inner;
        SidereonStatus::Ok
    })
}

/// Free a detection result. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_cycle_slip_detection_free(
    result: *mut SidereonPppCycleSlipDetection,
) {
    free_boxed(result);
}
