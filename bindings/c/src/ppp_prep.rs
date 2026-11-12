use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use crate::ppp_cycle_slip::{SidereonPppCycleSlipEpoch, SidereonPppCycleSlipObservation};
use serde_json::{json, Value};
use sidereon_core::carrier_phase::{CarrierPhaseError, CycleSlipOptions, SlipReason};
use sidereon_core::combinations::IonosphereFreeError;
use sidereon_core::precise_positioning as core;

/// Wide-lane preparation controls. This plain value has field equality.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppWideLanePrepOptions {
    /// Minimum samples required for an ambiguity arc.
    pub min_epochs: usize,
    /// Maximum distance from the nearest integer, in cycles.
    pub tolerance_cycles: f64,
}

/// Cycle-slip policy for PPP ambiguity preparation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppPrepCycleSlipPolicy {
    /// Refuse a detected slip.
    Error = 0,
    /// Drop the affected satellite.
    DropSatellite = 1,
    /// Split the ambiguity arc.
    SplitArc = 2,
}

/// One float observation with optional raw dual-frequency detector input.
#[repr(C)]
pub struct SidereonPppFloatCycleSlipObservation {
    /// Null-terminated UTF-8 satellite identifier.
    pub satellite_id: *const c_char,
    /// Null-terminated UTF-8 input ambiguity identifier.
    pub ambiguity_id: *const c_char,
    /// Optional borrowed raw dual-frequency observation.
    pub raw: *const SidereonPppCycleSlipObservation,
}

/// One float epoch for ambiguity tagging.
#[repr(C)]
pub struct SidereonPppFloatCycleSlipEpoch {
    /// Whether `gap_time_s` is present.
    pub has_gap_time_s: bool,
    /// Comparable seconds coordinate.
    pub gap_time_s: f64,
    /// Optional borrowed exact epoch.
    pub gap_epoch: *const SidereonExactEpoch,
    /// Float observation array.
    pub observations: *const SidereonPppFloatCycleSlipObservation,
    /// Number of float observations.
    pub observation_count: usize,
}

/// Fixed fields for one prepared float epoch.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppPreparedFloatEpochInfo {
    /// Original input epoch index.
    pub epoch_index: usize,
    /// Number of prepared observations.
    pub observation_count: usize,
}

/// Numeric fields for one prepared float observation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct SidereonPppPreparedFloatObservationInfo {
    /// Ionosphere-free code, in metres.
    pub code_m: f64,
    /// Ionosphere-free phase, in metres.
    pub phase_m: f64,
}

/// Fixed fields for one split ambiguity arc.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppSplitArcInfo {
    /// Original first epoch index.
    pub start_epoch_index: usize,
    /// Original final epoch index.
    pub end_epoch_index: usize,
    /// Retained sample count.
    pub n_epochs: usize,
}

/// Summary counts for a wide-lane preparation result.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppWideLanePrepResultInfo {
    /// Number of prepared epochs.
    pub epoch_count: usize,
    /// Number of wavelength entries.
    pub wavelength_count: usize,
    /// Number of offset entries.
    pub offset_count: usize,
    /// Number of fixed wide-lane integers.
    pub wide_lane_count: usize,
    /// Number of dropped satellites.
    pub dropped_satellite_count: usize,
    /// Number of retained split arcs.
    pub split_arc_count: usize,
}

/// Owned wide-lane and narrow-lane preparation result.
pub struct SidereonPppWideLanePrepResult {
    inner: core::WideLanePrepResult,
}

/// Summary for tagged float epochs.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SidereonPppFloatCycleSlipTaggedEpochInfo {
    /// Number of tagged observations.
    pub observation_count: usize,
}

/// Owned float ambiguity-tagging result.
pub struct SidereonPppFloatCycleSlipTaggedResult {
    inner: Vec<core::FloatCycleSlipTaggedEpoch>,
}

/// Wide-lane preparation refusal kind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SidereonPppWideLanePrepErrorKind {
    /// A cycle slip was rejected by policy.
    CycleSlipDetected = 0,
    /// A wide-lane sample could not be formed.
    WideLaneFailed = 1,
    /// Too few wide-lane samples were available.
    TooFewWideLaneEpochs = 2,
    /// The sample mean was not near an integer.
    WideLaneNotInteger = 3,
    /// A prepared ambiguity lacked a wide-lane integer.
    MissingWideLaneAmbiguity = 4,
    /// One ambiguity used inconsistent frequencies.
    InconsistentFrequencies = 5,
    /// Narrow-lane or ionosphere-free preparation failed.
    IonosphereFreeFailed = 6,
}

fn options_to_core(value: SidereonPppWideLanePrepOptions) -> core::WideLanePrepOptions {
    core::WideLanePrepOptions::new(value.min_epochs, value.tolerance_cycles)
}

fn cycle_options_to_core(value: SidereonCycleSlipOptions) -> CycleSlipOptions {
    let mut out = CycleSlipOptions::default();
    out.gf_threshold_m = value.gf_threshold_m;
    out.mw_threshold_cycles = value.mw_threshold_cycles;
    out.min_arc_gap_s = value.min_arc_gap_s;
    out
}

fn policy_from_c(fn_name: &str, value: u32) -> Result<core::CycleSlipPolicy, SidereonStatus> {
    match value {
        0 => Ok(core::CycleSlipPolicy::Error),
        1 => Ok(core::CycleSlipPolicy::DropSatellite),
        2 => Ok(core::CycleSlipPolicy::SplitArc),
        _ => {
            set_last_error(format!("{fn_name}: invalid cycle_slip_policy"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn float_epochs_from_c(
    fn_name: &str,
    values: *const SidereonPppFloatCycleSlipEpoch,
    count: usize,
) -> Result<Vec<core::FloatCycleSlipEpoch>, SidereonStatus> {
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
            .map(|observation| {
                Ok(core::FloatCycleSlipObservation {
                    satellite_id: parse_c_string(
                        fn_name,
                        "observation.satellite_id",
                        observation.satellite_id,
                    )?,
                    ambiguity_id: parse_c_string(
                        fn_name,
                        "observation.ambiguity_id",
                        observation.ambiguity_id,
                    )?,
                    raw: observation
                        .raw
                        .as_ref()
                        .map(|raw| crate::ppp_cycle_slip::observation_from_c(fn_name, raw))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, SidereonStatus>>()?;
            Ok(core::FloatCycleSlipEpoch {
                gap_time_s: epoch.has_gap_time_s.then_some(epoch.gap_time_s),
                gap_epoch: epoch.gap_epoch.as_ref().map(|value| value.inner),
                observations,
            })
        })
        .collect()
}

fn slip_reason_name(value: SlipReason) -> &'static str {
    match value {
        SlipReason::Lli => "lli",
        SlipReason::DataGap => "data_gap",
        SlipReason::GeometryFree => "geometry_free",
        SlipReason::MelbourneWubbena => "melbourne_wubbena",
    }
}

fn carrier_error_name(value: CarrierPhaseError) -> &'static str {
    match value {
        CarrierPhaseError::EqualFrequencies => "equal_frequencies",
        CarrierPhaseError::InvalidFrequency => "invalid_frequency",
        CarrierPhaseError::InvalidObservation => "invalid_observation",
        CarrierPhaseError::InvalidThreshold => "invalid_threshold",
    }
}

fn ionosphere_free_error_value(value: &IonosphereFreeError) -> Value {
    match value {
        IonosphereFreeError::UnknownSystem(system) => {
            json!({"kind": "unknown_system", "system": system.to_string()})
        }
        IonosphereFreeError::UnknownBand { system, band } => {
            json!({"kind": "unknown_band", "system": system.to_string(), "band": band})
        }
        IonosphereFreeError::EqualFrequencies => json!({"kind": "equal_frequencies"}),
        IonosphereFreeError::InvalidFrequency => json!({"kind": "invalid_frequency"}),
        IonosphereFreeError::InvalidObservation => json!({"kind": "invalid_observation"}),
    }
}

fn error_value(error: &core::WideLanePrepError) -> Value {
    match error {
        core::WideLanePrepError::CycleSlipDetected {
            satellite_id,
            epoch_index,
            reasons,
        } => json!({
            "kind": "cycle_slip_detected", "fields": {"satellite_id": satellite_id, "epoch_index": epoch_index, "reasons": reasons.iter().copied().map(slip_reason_name).collect::<Vec<_>>()}
        }),
        core::WideLanePrepError::WideLaneFailed {
            ambiguity_id,
            reason,
        } => json!({
            "kind": "wide_lane_failed", "fields": {"ambiguity_id": ambiguity_id, "reason": carrier_error_name(*reason)}
        }),
        core::WideLanePrepError::TooFewWideLaneEpochs {
            ambiguity_id,
            count,
            minimum,
        } => json!({
            "kind": "too_few_wide_lane_epochs", "fields": {"ambiguity_id": ambiguity_id, "count": count, "minimum": minimum}
        }),
        core::WideLanePrepError::WideLaneNotInteger {
            ambiguity_id,
            mean_cycles,
            fixed_cycles,
        } => json!({
            "kind": "wide_lane_not_integer", "fields": {"ambiguity_id": ambiguity_id, "mean_cycles": mean_cycles, "fixed_cycles": fixed_cycles}
        }),
        core::WideLanePrepError::MissingWideLaneAmbiguity(id) => json!({
            "kind": "missing_wide_lane_ambiguity", "fields": {"ambiguity_id": id}
        }),
        core::WideLanePrepError::InconsistentFrequencies(id) => json!({
            "kind": "inconsistent_frequencies", "fields": {"ambiguity_id": id}
        }),
        core::WideLanePrepError::IonosphereFreeFailed {
            satellite_id,
            reason,
        } => json!({
            "kind": "ionosphere_free_failed", "fields": {"satellite_id": satellite_id, "reason": ionosphere_free_error_value(reason)}
        }),
    }
}

fn map_error(fn_name: &str, error: core::WideLanePrepError) -> SidereonStatus {
    // WideLanePrepError has no Display implementation in the pinned core. Keep
    // the public JSON kind and fields explicitly mapped above; the legacy text
    // is intentionally generic until the core provides display text.
    record_engine_error(
        SidereonEngineErrorFamily::PppPrep,
        fn_name,
        error_value(&error),
    );
    set_last_error(format!("{fn_name}: wide-lane preparation failed"));
    SidereonStatus::InvalidArgument
}

unsafe fn copy_text(
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

/// Construct wide-lane preparation options through the core constructor.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_options_new(
    min_epochs: usize,
    tolerance_cycles: f64,
    out: *mut SidereonPppWideLanePrepOptions,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_options_new";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let value = core::WideLanePrepOptions::new(min_epochs, tolerance_cycles);
        *c_try!(require_out(out, fn_name, "out")) = SidereonPppWideLanePrepOptions {
            min_epochs: value.min_epochs,
            tolerance_cycles: value.tolerance_cycles,
        };
        SidereonStatus::Ok
    })
}

/// Compare two dual-frequency observations using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_dual_frequency_observation_equal(
    left: *const SidereonPppCycleSlipObservation,
    right: *const SidereonPppCycleSlipObservation,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_dual_frequency_observation_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(crate::ppp_cycle_slip::observation_from_c(
            fn_name,
            c_try!(require_ref(left, fn_name, "left"))
        ));
        let right = c_try!(crate::ppp_cycle_slip::observation_from_c(
            fn_name,
            c_try!(require_ref(right, fn_name, "right"))
        ));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left == right;
        SidereonStatus::Ok
    })
}

/// Compare two dual-frequency epochs using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_dual_frequency_epoch_equal(
    left: *const SidereonPppCycleSlipEpoch,
    right: *const SidereonPppCycleSlipEpoch,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_dual_frequency_epoch_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(crate::ppp_cycle_slip::epochs_from_c(fn_name, left, 1));
        let right = c_try!(crate::ppp_cycle_slip::epochs_from_c(fn_name, right, 1));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left[0] == right[0];
        SidereonStatus::Ok
    })
}

/// Compare two float cycle-slip observations using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_observation_equal(
    left: *const SidereonPppFloatCycleSlipObservation,
    right: *const SidereonPppFloatCycleSlipObservation,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_observation_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let left_epoch = SidereonPppFloatCycleSlipEpoch {
            has_gap_time_s: false,
            gap_time_s: 0.0,
            gap_epoch: ptr::null(),
            observations: left,
            observation_count: 1,
        };
        let right_epoch = SidereonPppFloatCycleSlipEpoch {
            has_gap_time_s: false,
            gap_time_s: 0.0,
            gap_epoch: ptr::null(),
            observations: right,
            observation_count: 1,
        };
        let left = c_try!(float_epochs_from_c(fn_name, &left_epoch, 1));
        let right = c_try!(float_epochs_from_c(fn_name, &right_epoch, 1));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) =
            left[0].observations[0] == right[0].observations[0];
        SidereonStatus::Ok
    })
}

/// Compare two float cycle-slip epochs using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_epoch_equal(
    left: *const SidereonPppFloatCycleSlipEpoch,
    right: *const SidereonPppFloatCycleSlipEpoch,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_epoch_equal";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let left = c_try!(float_epochs_from_c(fn_name, left, 1));
        let right = c_try!(float_epochs_from_c(fn_name, right, 1));
        *c_try!(require_out(out_equal, fn_name, "out_equal")) = left[0] == right[0];
        SidereonStatus::Ok
    })
}

/// Prepare dual-frequency epochs for fixed wide-lane/narrow-lane PPP.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_prepare_widelane_fixed_epochs(
    epochs: *const SidereonPppCycleSlipEpoch,
    epoch_count: usize,
    wide_lane: SidereonPppWideLanePrepOptions,
    cycle_slip_policy: u32,
    cycle_slip_options: SidereonCycleSlipOptions,
    out: *mut *mut SidereonPppWideLanePrepResult,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_prepare_widelane_fixed_epochs";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        let policy = c_try!(policy_from_c(fn_name, cycle_slip_policy));
        let epochs = c_try!(crate::ppp_cycle_slip::epochs_from_c(
            fn_name,
            epochs,
            epoch_count
        ));
        match core::prepare_widelane_fixed_epochs(
            &epochs,
            options_to_core(wide_lane),
            policy,
            cycle_options_to_core(cycle_slip_options),
        ) {
            Ok(inner) => {
                write_boxed_handle(out, SidereonPppWideLanePrepResult { inner });
                SidereonStatus::Ok
            }
            Err(error) => map_error(fn_name, error),
        }
    })
}

/// Read summary counts from a wide-lane preparation result.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_info(
    result: *const SidereonPppWideLanePrepResult,
    out: *mut SidereonPppWideLanePrepResultInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = &c_try!(require_ref(result, fn_name, "result")).inner;
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppWideLanePrepResultInfo {
            epoch_count: value.epochs.len(),
            wavelength_count: value.wavelengths_m.len(),
            offset_count: value.offsets_m.len(),
            wide_lane_count: value.wide_lane_cycles.len(),
            dropped_satellite_count: value.dropped_sats.len(),
            split_arc_count: value.split_arcs.len(),
        });
        SidereonStatus::Ok
    })
}

unsafe fn prep_epoch<'a>(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
) -> Result<&'a core::PreparedFloatEpoch, SidereonStatus> {
    require_ref(result, fn_name, "result")?
        .inner
        .epochs
        .get(index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: epoch_index out of range"));
            SidereonStatus::InvalidArgument
        })
}
unsafe fn prep_observation<'a>(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    epoch_index: usize,
    observation_index: usize,
) -> Result<&'a core::PreparedFloatObservation, SidereonStatus> {
    prep_epoch(fn_name, result, epoch_index)?
        .observations
        .get(observation_index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: observation_index out of range"));
            SidereonStatus::InvalidArgument
        })
}

/// Read one prepared epoch's fixed fields.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_epoch_info(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut SidereonPppPreparedFloatEpochInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_epoch_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(prep_epoch(fn_name, result, index));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppPreparedFloatEpochInfo {
            epoch_index: value.epoch_index,
            observation_count: value.observations.len(),
        });
        SidereonStatus::Ok
    })
}

/// Read one prepared observation's numeric fields.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_observation_info(
    result: *const SidereonPppWideLanePrepResult,
    epoch_index: usize,
    observation_index: usize,
    out: *mut SidereonPppPreparedFloatObservationInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_observation_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(prep_observation(
            fn_name,
            result,
            epoch_index,
            observation_index
        ));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppPreparedFloatObservationInfo {
            code_m: value.code_m,
            phase_m: value.phase_m,
        });
        SidereonStatus::Ok
    })
}

#[allow(clippy::too_many_arguments)]
unsafe fn observation_text(
    result: *const SidereonPppWideLanePrepResult,
    epoch_index: usize,
    observation_index: usize,
    ambiguity: bool,
    fn_name: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let value = c_try!(prep_observation(
        fn_name,
        result,
        epoch_index,
        observation_index
    ));
    copy_text(
        fn_name,
        if ambiguity {
            &value.ambiguity_id
        } else {
            &value.satellite_id
        },
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy one prepared observation's satellite identifier losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_observation_satellite_id(
    result: *const SidereonPppWideLanePrepResult,
    epoch_index: usize,
    observation_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_observation_satellite_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        observation_text(
            result,
            epoch_index,
            observation_index,
            false,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy one prepared observation's ambiguity identifier losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_observation_ambiguity_id(
    result: *const SidereonPppWideLanePrepResult,
    epoch_index: usize,
    observation_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_observation_ambiguity_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        observation_text(
            result,
            epoch_index,
            observation_index,
            true,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

unsafe fn map_f64_entry<'a>(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    which: u8,
    index: usize,
) -> Result<(&'a str, f64), SidereonStatus> {
    let value = &require_ref(result, fn_name, "result")?.inner;
    let entry = if which == 0 {
        value
            .wavelengths_m
            .iter()
            .nth(index)
            .map(|(key, value)| (key.as_str(), *value))
    } else {
        value
            .offsets_m
            .iter()
            .nth(index)
            .map(|(key, value)| (key.as_str(), *value))
    };
    entry.ok_or_else(|| {
        set_last_error(format!("{fn_name}: index out of range"));
        SidereonStatus::InvalidArgument
    })
}
unsafe fn map_i64_entry<'a>(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
) -> Result<(&'a str, i64), SidereonStatus> {
    require_ref(result, fn_name, "result")?
        .inner
        .wide_lane_cycles
        .iter()
        .nth(index)
        .map(|(key, value)| (key.as_str(), *value))
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: index out of range"));
            SidereonStatus::InvalidArgument
        })
}

#[allow(clippy::too_many_arguments)]
unsafe fn f64_map_key(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    which: u8,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let (key, _) = c_try!(map_f64_entry(fn_name, result, which, index));
    copy_text(fn_name, key, out, len, out_written, out_required)
}

unsafe fn f64_map_value(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    which: u8,
    index: usize,
    out: *mut f64,
) -> SidereonStatus {
    c_try!(reject_output_overlaps_handle(
        fn_name, out, "out", result, "result"
    ));
    let out = c_try!(require_out(out, fn_name, "out"));
    out.write(0.0);
    let (_, value) = c_try!(map_f64_entry(fn_name, result, which, index));
    out.write(value);
    SidereonStatus::Ok
}

/// Copy one narrow-lane wavelength map key.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_wavelength_key(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_wavelength_key";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        f64_map_key(
            fn_name,
            result,
            0,
            index,
            out,
            len,
            out_written,
            out_required,
        )
    })
}
/// Read one narrow-lane wavelength.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_wavelength_m(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_wavelength_m";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        f64_map_value(fn_name, result, 0, index, out)
    })
}
/// Copy one narrow-lane offset map key.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_offset_key(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_offset_key";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        f64_map_key(
            fn_name,
            result,
            1,
            index,
            out,
            len,
            out_written,
            out_required,
        )
    })
}
/// Read one narrow-lane offset.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_offset_m(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut f64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_offset_m";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        f64_map_value(fn_name, result, 1, index, out)
    })
}

/// Copy one fixed wide-lane map key.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_wide_lane_key(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_wide_lane_key";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let (key, _) = c_try!(map_i64_entry(fn_name, result, index));
        copy_text(fn_name, key, out, len, out_written, out_required)
    })
}
/// Read one fixed wide-lane integer.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_wide_lane_cycles(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut i64,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_wide_lane_cycles";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(0);
        let (_, value) = c_try!(map_i64_entry(fn_name, result, index));
        out.write(value);
        SidereonStatus::Ok
    })
}

/// Copy one dropped satellite identifier losslessly.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_dropped_satellite(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_dropped_satellite";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let value = c_try!(require_ref(result, fn_name, "result"))
            .inner
            .dropped_sats
            .get(index)
            .ok_or_else(|| {
                set_last_error(format!("{fn_name}: index out of range"));
                SidereonStatus::InvalidArgument
            });
        copy_text(fn_name, c_try!(value), out, len, out_written, out_required)
    })
}

unsafe fn split_arc<'a>(
    fn_name: &str,
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
) -> Result<&'a core::PppSplitArc, SidereonStatus> {
    require_ref(result, fn_name, "result")?
        .inner
        .split_arcs
        .get(index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: index out of range"));
            SidereonStatus::InvalidArgument
        })
}
/// Read one split arc's fixed fields.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_split_arc_info(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut SidereonPppSplitArcInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_split_arc_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(split_arc(fn_name, result, index));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppSplitArcInfo {
            start_epoch_index: value.start_epoch_index,
            end_epoch_index: value.end_epoch_index,
            n_epochs: value.n_epochs,
        });
        SidereonStatus::Ok
    })
}
#[allow(clippy::too_many_arguments)]
unsafe fn split_arc_text(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    ambiguity: bool,
    fn_name: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let value = c_try!(split_arc(fn_name, result, index));
    copy_text(
        fn_name,
        if ambiguity {
            &value.ambiguity_id
        } else {
            &value.satellite_id
        },
        out,
        len,
        out_written,
        out_required,
    )
}
/// Copy one split arc's satellite identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_split_arc_satellite_id(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_split_arc_satellite_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        split_arc_text(
            result,
            index,
            false,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}
/// Copy one split arc's ambiguity identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_split_arc_ambiguity_id(
    result: *const SidereonPppWideLanePrepResult,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_split_arc_ambiguity_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        split_arc_text(
            result,
            index,
            true,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Compare wide-lane preparation results using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_equal(
    left: *const SidereonPppWideLanePrepResult,
    right: *const SidereonPppWideLanePrepResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_wide_lane_prep_result_equal";
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
/// Free a wide-lane preparation result. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_wide_lane_prep_result_free(
    result: *mut SidereonPppWideLanePrepResult,
) {
    free_boxed(result);
}

/// Split float ambiguity identifiers at dual-frequency cycle slips.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_split_float_cycle_slip_epochs(
    epochs: *const SidereonPppFloatCycleSlipEpoch,
    epoch_count: usize,
    cycle_slip_options: SidereonCycleSlipOptions,
    out: *mut *mut SidereonPppFloatCycleSlipTaggedResult,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_split_float_cycle_slip_epochs";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, fn_name, "out"));
        out.write(ptr::null_mut());
        let epochs = c_try!(float_epochs_from_c(fn_name, epochs, epoch_count));
        let inner =
            core::split_float_cycle_slip_epochs(&epochs, cycle_options_to_core(cycle_slip_options));
        write_boxed_handle(out, SidereonPppFloatCycleSlipTaggedResult { inner });
        SidereonStatus::Ok
    })
}

/// Return the number of tagged float epochs.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_epoch_count(
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    out: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_tagged_result_epoch_count";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        *c_try!(require_out(out, fn_name, "out")) =
            c_try!(require_ref(result, fn_name, "result")).inner.len();
        SidereonStatus::Ok
    })
}
unsafe fn tagged_epoch<'a>(
    fn_name: &str,
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    index: usize,
) -> Result<&'a core::FloatCycleSlipTaggedEpoch, SidereonStatus> {
    require_ref(result, fn_name, "result")?
        .inner
        .get(index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: epoch_index out of range"));
            SidereonStatus::InvalidArgument
        })
}
unsafe fn tagged_observation<'a>(
    fn_name: &str,
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    epoch_index: usize,
    observation_index: usize,
) -> Result<&'a core::FloatCycleSlipTaggedObservation, SidereonStatus> {
    tagged_epoch(fn_name, result, epoch_index)?
        .observations
        .get(observation_index)
        .ok_or_else(|| {
            set_last_error(format!("{fn_name}: observation_index out of range"));
            SidereonStatus::InvalidArgument
        })
}
/// Read one tagged epoch's observation count.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_epoch_info(
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    index: usize,
    out: *mut SidereonPppFloatCycleSlipTaggedEpochInfo,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_tagged_result_epoch_info";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(reject_output_overlaps_handle(
            fn_name, out, "out", result, "result"
        ));
        let value = c_try!(tagged_epoch(fn_name, result, index));
        c_try!(require_out(out, fn_name, "out")).write(SidereonPppFloatCycleSlipTaggedEpochInfo {
            observation_count: value.observations.len(),
        });
        SidereonStatus::Ok
    })
}
#[allow(clippy::too_many_arguments)]
unsafe fn tagged_text(
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    epoch_index: usize,
    observation_index: usize,
    ambiguity: bool,
    fn_name: &str,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let value = c_try!(tagged_observation(
        fn_name,
        result,
        epoch_index,
        observation_index
    ));
    copy_text(
        fn_name,
        if ambiguity {
            &value.ambiguity_id
        } else {
            &value.satellite_id
        },
        out,
        len,
        out_written,
        out_required,
    )
}
/// Copy one tagged satellite identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_satellite_id(
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    epoch_index: usize,
    observation_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_tagged_result_satellite_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        tagged_text(
            result,
            epoch_index,
            observation_index,
            false,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}
/// Copy one tagged ambiguity identifier.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_ambiguity_id(
    result: *const SidereonPppFloatCycleSlipTaggedResult,
    epoch_index: usize,
    observation_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_tagged_result_ambiguity_id";
    engine_error_operation_boundary(fn_name, SidereonStatus::Panic, || {
        tagged_text(
            result,
            epoch_index,
            observation_index,
            true,
            fn_name,
            out,
            len,
            out_written,
            out_required,
        )
    })
}
/// Compare tagged float results using core value equality.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_equal(
    left: *const SidereonPppFloatCycleSlipTaggedResult,
    right: *const SidereonPppFloatCycleSlipTaggedResult,
    out_equal: *mut bool,
) -> SidereonStatus {
    let fn_name = "sidereon_ppp_float_cycle_slip_tagged_result_equal";
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
/// Free a tagged float result. Null is accepted.
#[no_mangle]
pub unsafe extern "C" fn sidereon_ppp_float_cycle_slip_tagged_result_free(
    result: *mut SidereonPppFloatCycleSlipTaggedResult,
) {
    free_boxed(result);
}
