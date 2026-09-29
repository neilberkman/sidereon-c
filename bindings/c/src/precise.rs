use super::*;
use crate::engine_error::{
    engine_error_operation_boundary, record_engine_error, SidereonEngineErrorFamily,
};
use crate::sp3::interpolation_options_from_c;

/// A sample-backed precise-ephemeris source, built from canonical
/// position/clock samples rather than parsed SP3 text. Opaque to C. Create with
/// sidereon_precise_ephemeris_samples_from_samples and release with
/// sidereon_precise_ephemeris_samples_free. Interpolates and predicts ranges
/// through the same substrate as a loaded SP3 product.
pub struct SidereonPreciseEphemerisSamples {
    pub(crate) inner: PreciseEphemerisSamples,
    pub(crate) source_samples_v2: Option<Vec<SidereonPreciseEphemerisSampleV2>>,
    pub(crate) source_accuracy_v2: Option<Vec<SidereonPreciseEphemerisAccuracySampleV2>>,
}

/// A build-once precise-ephemeris interpolant with cached per-satellite nodes.
/// Opaque to C. Create with sidereon_precise_ephemeris_interpolant_from_sp3,
/// sidereon_precise_ephemeris_interpolant_from_samples, or
/// sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples and
/// release with sidereon_precise_ephemeris_interpolant_free.
pub struct SidereonPreciseEphemerisInterpolant {
    pub(crate) inner: PreciseEphemerisInterpolant,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SidereonEphemerisSourceState {
    pub has_state: bool,
    pub position_ecef_m: [f64; 3],
    pub clock_s: f64,
    pub has_group_delay: bool,
    pub group_delay_s: f64,
    pub degraded: bool,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonClockRelativityKind {
    NotApplicable = 0,
    Term = 1,
    Unavailable = 2,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_state_at_epoch_queries(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out: *mut SidereonEphemerisSourceState,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_state_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        *out = SidereonEphemerisSourceState::default();
        record_degrade_reason(None);
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let state = c_try!(guard_core(
            || {
                sidereon_core::positioning::EphemerisSource::try_position_clock_group_delay_selected_at_epoch_query(
                &interpolant.inner, satellite, &state_epoch.inner, &selection_epoch.inner,
            )
            },
            |error| precise_source_error_to_status(FN_NAME, error),
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
pub unsafe extern "C" fn sidereon_precise_interpolant_transmit_epoch_clock_at_epoch_queries(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    sat_id: *const c_char,
    transmit_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_has_clock: *mut bool,
    out_clock_s: *mut f64,
    out_degraded: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_transmit_epoch_clock_at_epoch_queries";
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
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let transmit_epoch = c_try!(require_ref(transmit_epoch, FN_NAME, "transmit_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let clock = c_try!(guard_core(
            || sidereon_core::positioning::EphemerisSource::try_transmit_epoch_clock_at_epoch_query(
                &interpolant.inner,
                satellite,
                &transmit_epoch.inner,
                &selection_epoch.inner,
            ),
            |error| precise_source_error_to_status(FN_NAME, error),
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
pub unsafe extern "C" fn sidereon_precise_interpolant_clock_relativity_at_epoch_query(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    sat_id: *const c_char,
    epoch: *const SidereonExactEpochQuery,
    position_ecef_m: *const f64,
    out_kind: *mut SidereonClockRelativityKind,
    out_term_s: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_clock_relativity_at_epoch_query";
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
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
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
            &interpolant.inner,
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
pub unsafe extern "C" fn sidereon_precise_interpolant_ephemeris_variance_at_epoch_queries(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_variance_m2: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_ephemeris_variance_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_variance_m2, FN_NAME, "out_variance_m2"));
        *out = 0.0;
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        *out = sidereon_core::positioning::EphemerisSource::ephemeris_variance_at_epoch_query(
            &interpolant.inner,
            satellite,
            &state_epoch.inner,
            &selection_epoch.inner,
        );
        SidereonStatus::Ok
    })
}

pub(crate) fn precise_source_error_to_status(fn_name: &str, error: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {error} ({error:?})"));
    match error {
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        CoreError::EpochOutOfRange | CoreError::InsufficientPreciseNodes { .. } => {
            SidereonStatus::InvalidArgument
        }
        _ => SidereonStatus::Solve,
    }
}

// --- Batch forward-observable prediction ------------------------------------

/// One batch observable-prediction request: the satellite token, the static
/// receiver ECEF position (meters), and the receive epoch (seconds since J2000).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPredictRequest {
    /// Null-terminated satellite token (e.g. "G01").
    pub sat_id: *const c_char,
    /// Receiver ECEF position, meters.
    pub receiver_ecef_m: [f64; 3],
    /// Receive epoch, seconds since J2000.
    pub t_rx_j2000_s: f64,
}

// --- Precise-ephemeris samples + batch range prediction ---------------------

/// One canonical precise-ephemeris sample: a satellite's ECEF position (and
/// optional clock) at one epoch, in SI units. This is the serialization-
/// independent element behind an SP3 record; sidereon_sp3_precise_ephemeris_samples
/// extracts them and sidereon_precise_ephemeris_samples_from_samples rebuilds an
/// interpolatable source from them.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseEphemerisSample {
    /// Satellite this sample describes, as a null-terminated token (e.g. G01).
    pub sat: SidereonSatelliteToken,
    /// Time scale the epoch is expressed in (a SidereonTimeScale code as
    /// uint32_t). Every sample in one source must share this scale.
    pub time_scale: u32,
    /// Sample epoch, seconds since J2000 in the sample's time scale.
    pub epoch_j2000_s: f64,
    /// Satellite ECEF position in the ITRF/IGS frame, meters.
    pub position_ecef_m: [f64; 3],
    /// Whether clock_s carries a satellite clock estimate.
    pub has_clock_s: bool,
    /// Satellite clock offset, seconds, when has_clock_s is true.
    pub clock_s: f64,
    /// Whether this epoch carries the SP3 E clock-event flag: true splits the
    /// clock interpolation arc here (a clock reset takes effect at this epoch).
    pub clock_event: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseEphemerisAccuracySample {
    pub sat: SidereonSatelliteToken,
    pub time_scale: u32,
    pub epoch_j2000_s: f64,
    pub position_variance_m2: [SidereonSp3AccuracyValue; 3],
    pub clock_variance_m2: SidereonSp3AccuracyValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseEphemerisSampleV2 {
    pub sat: SidereonSatelliteToken,
    pub epoch: SidereonClockEpoch,
    pub position_ecef_m: [f64; 3],
    pub has_clock_s: bool,
    pub clock_s: f64,
    pub clock_event: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseEphemerisAccuracySampleV2 {
    pub sat: SidereonSatelliteToken,
    pub epoch: SidereonClockEpoch,
    pub position_variance_m2: [SidereonSp3AccuracyValue; 3],
    pub clock_variance_m2: SidereonSp3AccuracyValue,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPreciseSamplesErrorKind {
    None = 0,
    Empty = 1,
    SingleSampleSatellite = 2,
    NonMonotonicEpochs = 3,
    MixedTimeScales = 4,
    EpochNotRepresentable = 5,
    NonFiniteSample = 6,
    AccuracySamplesMismatch = 7,
    InvalidAccuracyValue = 8,
    Other = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseSamplesError {
    pub kind: SidereonPreciseSamplesErrorKind,
    pub has_satellite: bool,
    pub satellite: SidereonSatelliteToken,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_precise_ephemeris_accuracy_samples(
    sp3: *const SidereonSp3,
    out: *mut SidereonPreciseEphemerisAccuracySample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_precise_ephemeris_accuracy_samples";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let mut values = Vec::new();
        for sample in sp3.inner.precise_ephemeris_accuracy_samples() {
            let Some(epoch_j2000_s) = instant_to_j2000_seconds(&sample.epoch) else {
                set_last_error(format!(
                    "{FN_NAME}: an epoch is outside the legacy seconds representation; use the lossless v2 accessor"
                ));
                return SidereonStatus::InvalidArgument;
            };
            values.push(SidereonPreciseEphemerisAccuracySample {
                sat: satellite_token(sample.sat),
                time_scale: time_scale_to_c_code(sample.epoch.scale),
                epoch_j2000_s,
                position_variance_m2: sample
                    .position_variance_m2
                    .map(crate::sp3::sp3_accuracy_value_to_c),
                clock_variance_m2: crate::sp3::sp3_accuracy_value_to_c(sample.clock_variance_m2),
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_precise_ephemeris_accuracy_samples_v2(
    sp3: *const SidereonSp3,
    out: *mut SidereonPreciseEphemerisAccuracySampleV2,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_precise_ephemeris_accuracy_samples_v2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let values = sp3
            .inner
            .precise_ephemeris_accuracy_samples()
            .into_iter()
            .map(|sample| SidereonPreciseEphemerisAccuracySampleV2 {
                sat: satellite_token(sample.sat),
                epoch: crate::rinex_clock::instant_to_clock_epoch(&sample.epoch),
                position_variance_m2: sample
                    .position_variance_m2
                    .map(crate::sp3::sp3_accuracy_value_to_c),
                clock_variance_m2: crate::sp3::sp3_accuracy_value_to_c(sample.clock_variance_m2),
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

/// One batch range-prediction request: the satellite token, the static receiver
/// ECEF position (meters), and the receive epoch (seconds since J2000).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangePredictionRequest {
    /// Null-terminated satellite token (e.g. "G01").
    pub sat_id: *const c_char,
    /// Receiver ECEF position, meters.
    pub receiver_ecef_m: [f64; 3],
    /// Receive epoch, seconds since J2000.
    pub t_rx_j2000_s: f64,
}

/// The geometry-only result of one range-prediction request: the transmit-time
/// geometry a range-only consumer needs, without Doppler or topocentric fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonRangePrediction {
    /// Geometric range after optional Sagnac transport, meters.
    pub geometric_range_m: f64,
    /// Whether sat_clock_s is present.
    pub has_sat_clock_s: bool,
    /// Satellite clock offset at transmit time, seconds, when present.
    pub sat_clock_s: f64,
    /// Transmit time, seconds since J2000.
    pub transmit_time_j2000_s: f64,
    /// Sagnac-transported satellite ECEF position, meters.
    pub sat_pos_ecef_m: [f64; 3],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonEphemerisSampleStatus {
    Valid = 0,
    Gap = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonEphemerisSampleRow {
    pub sat_id: SidereonSatelliteToken,
    pub epoch_j2000_s: f64,
    pub status: SidereonEphemerisSampleStatus,
    pub has_position_ecef_m: bool,
    pub position_ecef_m: [f64; 3],
    pub has_clock_s: bool,
    pub clock_s: f64,
}

/// Build a sample-backed precise-ephemeris source from count canonical samples.
/// On success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_samples_free. Validation failures (no samples, a
/// single-sample satellite, non-monotonic epochs, mixed time scales, a
/// non-representable epoch, or a non-finite value) return
/// SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: samples must point to count entries (each with a valid sat token) or
/// be NULL when count is 0; out_handle must point to storage for a
/// SidereonPreciseEphemerisSamples*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
) -> SidereonStatus {
    sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
        samples, count, 0.0, out_handle,
    )
}

/// Build a sample-backed precise-ephemeris source from count canonical samples
/// with an explicit coverage-gap threshold factor. When gap_threshold_factor is
/// <= 0.0, the core default of 1.5 is used.
/// On success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_samples_free. Validation failures (no samples, a
/// single-sample satellite, non-monotonic epochs, mixed time scales, a
/// non-representable epoch, a non-finite value, or an invalid gap threshold
/// factor) return SIDEREON_STATUS_INVALID_ARGUMENT.
///
/// Safety: samples must point to count entries (each with a valid sat token) or
/// be NULL when count is 0; out_handle must point to storage for a
/// SidereonPreciseEphemerisSamples*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
) -> SidereonStatus {
    const FN_NAME: &str =
        "sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let raw = c_try!(require_slice(samples, count, FN_NAME, "samples"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let mut parsed = Vec::with_capacity(raw.len());
        for sample in raw {
            parsed.push(c_try!(precise_sample_from_c(FN_NAME, sample)));
        }
        let inner = match PreciseEphemerisSamples::from_samples(parsed) {
            Ok(inner) => inner.with_interpolation_options(options),
            Err(err) => return map_precise_samples_error(FN_NAME, err),
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisSamples {
                inner,
                source_samples_v2: None,
                source_accuracy_v2: None,
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples_with_accuracy(
    samples: *const SidereonPreciseEphemerisSample,
    accuracy: *const SidereonPreciseEphemerisAccuracySample,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
    out_error: *mut SidereonPreciseSamplesError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_from_samples_with_accuracy";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let out_error = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out_error = empty_precise_samples_error();
        let source = match precise_samples_with_accuracy_from_c(
            FN_NAME,
            samples,
            accuracy,
            count,
            gap_threshold_factor,
            out_error,
        ) {
            Ok(source) => source,
            Err(status) => return status,
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisSamples {
                inner: source,
                source_samples_v2: None,
                source_accuracy_v2: None,
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples_with_accuracy(
    samples: *const SidereonPreciseEphemerisSample,
    accuracy: *const SidereonPreciseEphemerisAccuracySample,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
    out_error: *mut SidereonPreciseSamplesError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_interpolant_from_samples_with_accuracy";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let out_error = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out_error = empty_precise_samples_error();
        let source = match precise_samples_with_accuracy_from_c(
            FN_NAME,
            samples,
            accuracy,
            count,
            gap_threshold_factor,
            out_error,
        ) {
            Ok(source) => source,
            Err(status) => return status,
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisInterpolant {
                inner: PreciseEphemerisInterpolant::from_precise_ephemeris_samples(&source),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples_with_accuracy_v2(
    samples: *const SidereonPreciseEphemerisSampleV2,
    accuracy: *const SidereonPreciseEphemerisAccuracySampleV2,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
    out_error: *mut SidereonPreciseSamplesError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_from_samples_with_accuracy_v2";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let out_error = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out_error = empty_precise_samples_error();
        let source = match precise_samples_with_accuracy_from_c_v2(
            FN_NAME,
            samples,
            accuracy,
            count,
            gap_threshold_factor,
            out_error,
        ) {
            Ok(source) => source,
            Err(status) => return status,
        };
        let source_samples_v2 = c_try!(require_slice(samples, count, FN_NAME, "samples")).to_vec();
        let source_accuracy_v2 =
            c_try!(require_slice(accuracy, count, FN_NAME, "accuracy")).to_vec();
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisSamples {
                inner: source,
                source_samples_v2: Some(source_samples_v2),
                source_accuracy_v2: Some(source_accuracy_v2),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_from_samples_v2(
    samples: *const SidereonPreciseEphemerisSampleV2,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisSamples,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_from_samples_v2";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let raw = c_try!(require_slice(samples, count, FN_NAME, "samples"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let mut parsed = Vec::with_capacity(count);
        for sample in raw {
            parsed.push(c_try!(precise_sample_from_c_v2(FN_NAME, sample)));
        }
        let inner = match PreciseEphemerisSamples::from_samples(parsed) {
            Ok(inner) => inner.with_interpolation_options(options),
            Err(error) => return map_precise_samples_error(FN_NAME, error),
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisSamples {
                inner,
                source_samples_v2: Some(raw.to_vec()),
                source_accuracy_v2: None,
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples_v2(
    samples: *const SidereonPreciseEphemerisSampleV2,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_interpolant_from_samples_v2";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let raw = c_try!(require_slice(samples, count, FN_NAME, "samples"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let mut parsed = Vec::with_capacity(count);
        for sample in raw {
            parsed.push(c_try!(precise_sample_from_c_v2(FN_NAME, sample)));
        }
        let source = match PreciseEphemerisSamples::from_samples(parsed) {
            Ok(source) => source.with_interpolation_options(options),
            Err(error) => return map_precise_samples_error(FN_NAME, error),
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisInterpolant {
                inner: PreciseEphemerisInterpolant::from_precise_ephemeris_samples(&source),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_records_v2(
    samples: *const SidereonPreciseEphemerisSamples,
    out: *mut SidereonPreciseEphemerisSampleV2,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_records_v2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let samples = c_try!(require_ref(samples, FN_NAME, "samples"));
        let Some(records) = samples.source_samples_v2.as_ref() else {
            set_last_error(format!(
                "{FN_NAME}: exact source records are unavailable for this handle"
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            records,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_accuracy_records_v2(
    samples: *const SidereonPreciseEphemerisSamples,
    out: *mut SidereonPreciseEphemerisAccuracySampleV2,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_accuracy_records_v2";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let samples = c_try!(require_ref(samples, FN_NAME, "samples"));
        let Some(records) = samples.source_accuracy_v2.as_ref() else {
            set_last_error(format!(
                "{FN_NAME}: exact accuracy records are unavailable for this handle"
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            records,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples_with_accuracy_v2(
    samples: *const SidereonPreciseEphemerisSampleV2,
    accuracy: *const SidereonPreciseEphemerisAccuracySampleV2,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
    out_error: *mut SidereonPreciseSamplesError,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_interpolant_from_samples_with_accuracy_v2";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let out_error = c_try!(require_out(out_error, FN_NAME, "out_error"));
        *out_error = empty_precise_samples_error();
        let source = match precise_samples_with_accuracy_from_c_v2(
            FN_NAME,
            samples,
            accuracy,
            count,
            gap_threshold_factor,
            out_error,
        ) {
            Ok(source) => source,
            Err(status) => return status,
        };
        write_boxed_handle(
            out_handle,
            SidereonPreciseEphemerisInterpolant {
                inner: PreciseEphemerisInterpolant::from_precise_ephemeris_samples(&source),
            },
        );
        SidereonStatus::Ok
    })
}

unsafe fn precise_samples_with_accuracy_from_c_v2(
    fn_name: &str,
    samples: *const SidereonPreciseEphemerisSampleV2,
    accuracy: *const SidereonPreciseEphemerisAccuracySampleV2,
    count: usize,
    gap_threshold_factor: f64,
    out_error: *mut SidereonPreciseSamplesError,
) -> Result<PreciseEphemerisSamples, SidereonStatus> {
    let sample_rows = require_slice(samples, count, fn_name, "samples")?;
    let accuracy_rows = require_slice(accuracy, count, fn_name, "accuracy")?;
    let options = interpolation_options_from_c(fn_name, gap_threshold_factor)?;
    let mut parsed_samples = Vec::with_capacity(count);
    let mut parsed_accuracy = Vec::with_capacity(count);
    for (sample, accuracy) in sample_rows.iter().zip(accuracy_rows) {
        parsed_samples.push(precise_sample_from_c_v2(fn_name, sample)?);
        let accuracy_satellite = parse_satellite_token(fn_name, accuracy.sat.bytes.as_ptr())?;
        let accuracy_epoch =
            crate::rinex_clock::clock_epoch_from_c(fn_name, "accuracy.epoch", &accuracy.epoch)?;
        let Some(position_variance_m2) = accuracy
            .position_variance_m2
            .map(sp3_accuracy_value_from_c)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|values| values.try_into().ok())
        else {
            set_last_error(format!("{fn_name}: unknown position accuracy value kind"));
            return Err(SidereonStatus::InvalidArgument);
        };
        let Some(clock_variance_m2) = sp3_accuracy_value_from_c(accuracy.clock_variance_m2) else {
            set_last_error(format!("{fn_name}: unknown clock accuracy value kind"));
            return Err(SidereonStatus::InvalidArgument);
        };
        parsed_accuracy.push(
            sidereon_core::ephemeris::PreciseEphemerisAccuracySample::new(
                accuracy_satellite,
                accuracy_epoch,
                position_variance_m2,
                clock_variance_m2,
            ),
        );
    }
    match PreciseEphemerisSamples::from_samples_with_accuracy(parsed_samples, parsed_accuracy) {
        Ok(source) => Ok(source.with_interpolation_options(options)),
        Err(error) => {
            *out_error = precise_samples_error_to_c(&error);
            set_last_error(format!("{fn_name}: {error}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

unsafe fn precise_sample_from_c_v2(
    fn_name: &str,
    sample: &SidereonPreciseEphemerisSampleV2,
) -> Result<PreciseEphemerisSample, SidereonStatus> {
    let satellite = parse_satellite_token(fn_name, sample.sat.bytes.as_ptr())?;
    let epoch = crate::rinex_clock::clock_epoch_from_c(fn_name, "sample.epoch", &sample.epoch)?;
    Ok(PreciseEphemerisSample {
        sat: satellite,
        epoch,
        position_ecef_m: sample.position_ecef_m,
        clock_s: sample.has_clock_s.then_some(sample.clock_s),
        clock_event: sample.clock_event,
    })
}

#[cfg(test)]
mod precise_accuracy_epoch_tests {
    use super::*;

    #[test]
    fn lossless_accuracy_epoch_keeps_subsecond_identity_after_rounded_seconds_collapse() {
        let first = Instant::from_nanos(TimeScale::Gpst, 1_000_000_000_000_000_000);
        let second = Instant::from_nanos(TimeScale::Gpst, 1_000_000_000_000_000_001);
        assert_eq!(
            instant_to_j2000_seconds(&first),
            instant_to_j2000_seconds(&second)
        );
        let first_c = crate::rinex_clock::instant_to_clock_epoch(&first);
        let second_c = crate::rinex_clock::instant_to_clock_epoch(&second);
        assert_eq!(
            first_c.representation,
            SidereonRinexClockInstantRepresentation::Nanos as u32
        );
        assert_eq!(
            second_c.representation,
            SidereonRinexClockInstantRepresentation::Nanos as u32
        );
        assert_ne!(
            (first_c.nanos_high, first_c.nanos_low),
            (second_c.nanos_high, second_c.nanos_low)
        );
        assert_eq!(
            crate::rinex_clock::clock_epoch_from_c("test", "epoch", &first_c).unwrap(),
            first
        );
        assert_eq!(
            crate::rinex_clock::clock_epoch_from_c("test", "epoch", &second_c).unwrap(),
            second
        );
    }

    #[test]
    fn no_accuracy_v2_samples_round_trip_the_tagged_epochs() {
        let satellite = satellite_token(
            sidereon_core::GnssSatelliteId::new(sidereon_core::GnssSystem::Gps, 1).unwrap(),
        );
        let first = Instant::from_nanos(TimeScale::Gpst, 1_000_000_000_000_000_000);
        let second = Instant::from_nanos(TimeScale::Gpst, 1_000_000_900_000_000_000);
        let samples = [
            SidereonPreciseEphemerisSampleV2 {
                sat: satellite,
                epoch: crate::rinex_clock::instant_to_clock_epoch(&first),
                position_ecef_m: [1.0, 2.0, 3.0],
                has_clock_s: true,
                clock_s: 4.0e-6,
                clock_event: false,
            },
            SidereonPreciseEphemerisSampleV2 {
                sat: satellite,
                epoch: crate::rinex_clock::instant_to_clock_epoch(&second),
                position_ecef_m: [4.0, 5.0, 6.0],
                has_clock_s: false,
                clock_s: 0.0,
                clock_event: true,
            },
        ];
        let mut handle = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_from_samples_v2(
                    samples.as_ptr(),
                    samples.len(),
                    0.0,
                    &mut handle,
                )
            },
            SidereonStatus::Ok
        );
        let mut output = [samples[0]; 2];
        let mut written = 0;
        let mut required = 0;
        assert_eq!(
            unsafe {
                sidereon_precise_ephemeris_samples_records_v2(
                    handle,
                    output.as_mut_ptr(),
                    output.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!((written, required), (2, 2));
        assert_eq!(
            crate::rinex_clock::clock_epoch_from_c("test", "first", &output[0].epoch).unwrap(),
            first
        );
        assert_eq!(
            crate::rinex_clock::clock_epoch_from_c("test", "second", &output[1].epoch).unwrap(),
            second
        );
        assert_eq!(output[0].position_ecef_m, samples[0].position_ecef_m);
        assert_eq!(output[1].position_ecef_m, samples[1].position_ecef_m);
        unsafe { sidereon_precise_ephemeris_samples_free(handle) };
    }
}

unsafe fn precise_samples_with_accuracy_from_c(
    fn_name: &str,
    samples: *const SidereonPreciseEphemerisSample,
    accuracy: *const SidereonPreciseEphemerisAccuracySample,
    count: usize,
    gap_threshold_factor: f64,
    out_error: *mut SidereonPreciseSamplesError,
) -> Result<PreciseEphemerisSamples, SidereonStatus> {
    let sample_rows = require_slice(samples, count, fn_name, "samples")?;
    let accuracy_rows = require_slice(accuracy, count, fn_name, "accuracy")?;
    let options = interpolation_options_from_c(fn_name, gap_threshold_factor)?;
    let mut parsed_samples = Vec::with_capacity(count);
    let mut parsed_accuracy = Vec::with_capacity(count);
    for (sample, accuracy) in sample_rows.iter().zip(accuracy_rows) {
        parsed_samples.push(precise_sample_from_c(fn_name, sample)?);
        let satellite = parse_satellite_token(fn_name, accuracy.sat.bytes.as_ptr())?;
        let scale = time_scale_from_c_code(fn_name, "accuracy.time_scale", accuracy.time_scale)?;
        let epoch = instant_from_j2000_seconds(fn_name, "accuracy", scale, accuracy.epoch_j2000_s)?;
        let Some(position_variance_m2) = accuracy
            .position_variance_m2
            .map(sp3_accuracy_value_from_c)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|values| values.try_into().ok())
        else {
            set_last_error(format!("{fn_name}: unknown accuracy value kind"));
            return Err(SidereonStatus::InvalidArgument);
        };
        let Some(clock_variance_m2) = sp3_accuracy_value_from_c(accuracy.clock_variance_m2) else {
            set_last_error(format!("{fn_name}: unknown accuracy value kind"));
            return Err(SidereonStatus::InvalidArgument);
        };
        parsed_accuracy.push(
            sidereon_core::ephemeris::PreciseEphemerisAccuracySample::new(
                satellite,
                epoch,
                position_variance_m2,
                clock_variance_m2,
            ),
        );
    }
    let source = match PreciseEphemerisSamples::from_samples_with_accuracy(
        parsed_samples,
        parsed_accuracy,
    ) {
        Ok(source) => source.with_interpolation_options(options),
        Err(error) => {
            *out_error = precise_samples_error_to_c(&error);
            return Err(map_precise_samples_error(fn_name, error));
        }
    };
    Ok(source)
}

fn empty_precise_samples_error() -> SidereonPreciseSamplesError {
    SidereonPreciseSamplesError {
        kind: SidereonPreciseSamplesErrorKind::None,
        has_satellite: false,
        satellite: SidereonSatelliteToken { bytes: [0; 17] },
    }
}

fn precise_samples_error_to_c(error: &PreciseSamplesError) -> SidereonPreciseSamplesError {
    let mut result = empty_precise_samples_error();
    match error {
        PreciseSamplesError::Empty => result.kind = SidereonPreciseSamplesErrorKind::Empty,
        PreciseSamplesError::SingleSampleSatellite(satellite) => {
            result.kind = SidereonPreciseSamplesErrorKind::SingleSampleSatellite;
            result.has_satellite = true;
            result.satellite = satellite_token(*satellite);
        }
        PreciseSamplesError::NonMonotonicEpochs(satellite) => {
            result.kind = SidereonPreciseSamplesErrorKind::NonMonotonicEpochs;
            result.has_satellite = true;
            result.satellite = satellite_token(*satellite);
        }
        PreciseSamplesError::MixedTimeScales => {
            result.kind = SidereonPreciseSamplesErrorKind::MixedTimeScales;
        }
        PreciseSamplesError::EpochNotRepresentable(satellite) => {
            result.kind = SidereonPreciseSamplesErrorKind::EpochNotRepresentable;
            result.has_satellite = true;
            result.satellite = satellite_token(*satellite);
        }
        PreciseSamplesError::NonFiniteSample(satellite) => {
            result.kind = SidereonPreciseSamplesErrorKind::NonFiniteSample;
            result.has_satellite = true;
            result.satellite = satellite_token(*satellite);
        }
        PreciseSamplesError::AccuracySamplesMismatch => {
            result.kind = SidereonPreciseSamplesErrorKind::AccuracySamplesMismatch;
        }
        PreciseSamplesError::InvalidAccuracyValue(satellite) => {
            result.kind = SidereonPreciseSamplesErrorKind::InvalidAccuracyValue;
            result.has_satellite = true;
            result.satellite = satellite_token(*satellite);
        }
        _ => result.kind = SidereonPreciseSamplesErrorKind::Other,
    }
    result
}

fn sp3_accuracy_value_from_c(
    value: SidereonSp3AccuracyValue,
) -> Option<sidereon_core::ephemeris::Sp3AccuracyValue> {
    use sidereon_core::ephemeris::Sp3AccuracyValue as CoreValue;
    match value.kind {
        kind if kind == SidereonSp3AccuracyValueKind::Known as u32 => {
            Some(CoreValue::Known(value.value))
        }
        kind if kind == SidereonSp3AccuracyValueKind::Unknown as u32 => Some(CoreValue::Unknown),
        kind if kind == SidereonSp3AccuracyValueKind::TooLarge as u32 => Some(CoreValue::TooLarge),
        kind if kind == SidereonSp3AccuracyValueKind::InvalidBase as u32 => {
            Some(CoreValue::InvalidBase)
        }
        kind if kind == SidereonSp3AccuracyValueKind::Overflow as u32 => Some(CoreValue::Overflow),
        _ => None,
    }
}

/// Write the SP3 interpolation gap threshold factor carried by this
/// sample-backed source to *out_gap_threshold_factor.
///
/// Safety: samples must be a live handle; out_gap_threshold_factor must point
/// to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_gap_threshold_factor(
    samples: *const SidereonPreciseEphemerisSamples,
    out_gap_threshold_factor: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_samples_gap_threshold_factor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_gap_threshold_factor = c_try!(require_out(
            out_gap_threshold_factor,
            FN_NAME,
            "out_gap_threshold_factor"
        ));
        *out_gap_threshold_factor = 0.0;
        let samples = c_try!(require_ref(samples, FN_NAME, "samples"));
        *out_gap_threshold_factor = samples.inner.interpolation_options().gap_threshold_factor();
        SidereonStatus::Ok
    })
}

/// Release a precise-ephemeris samples handle. Null is a no-op. A non-null handle
/// must come from sidereon_precise_ephemeris_samples_from_samples and must be
/// freed exactly once with this function.
///
/// Safety: samples must be NULL or a live handle from
/// sidereon_precise_ephemeris_samples_from_samples. Passing a handle after it has
/// already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_free(
    samples: *mut SidereonPreciseEphemerisSamples,
) {
    ffi_boundary("sidereon_precise_ephemeris_samples_free", (), || {
        free_boxed(samples);
    });
}

/// Sample a sample-backed precise-ephemeris source over a regular grid.
///
/// Safety: samples must be a live handle; satellites points to satellite_count
/// null-terminated tokens; out points to len SidereonEphemerisSampleRow or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_sample(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_sample",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_sample",
                "samples"
            ));
            ephemeris_sample_common(
                "sidereon_precise_ephemeris_samples_sample",
                &samples.inner,
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
/// sample-backed precise-ephemeris source in one call. Mirror of
/// sidereon_sp3_predict_ranges for the samples source; same per-request out
/// contract. Delegates to sidereon_core::observables::predict_ranges.
///
/// Safety: samples must be a live handle from
/// sidereon_precise_ephemeris_samples_from_samples; requests must point to count
/// entries (each with a valid sat_id); out must point to count writable entries
/// (or be NULL when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_predict_ranges(
    samples: *const SidereonPreciseEphemerisSamples,
    requests: *const SidereonRangePredictionRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonRangePrediction,
) -> SidereonStatus {
    crate::engine_error::observables_error_operation_boundary(
        "sidereon_precise_ephemeris_samples_predict_ranges",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_predict_ranges",
                "samples"
            ));
            predict_ranges_into(
                "sidereon_precise_ephemeris_samples_predict_ranges",
                &samples.inner,
                requests,
                count,
                options,
                out,
            )
        },
    )
}

// --- 0.13 batched observable states and cached interpolants -----------------

/// Per-element state category for a batched observable-state query.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonObservableStateElementStatus {
    /// Position and clock fields hold a usable state.
    Valid = 0,
    /// The source had no usable state for this satellite and epoch.
    Gap = 1,
    /// The scalar evaluator returned a non-gap error.
    Error = 2,
}

/// Build a cached precise-ephemeris interpolant from a loaded SP3 handle. On
/// success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: sp3 must be a live handle; out_handle must point to storage for a
/// SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_sp3(
    sp3: *const SidereonSp3,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_interpolant_from_sp3",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_interpolant_from_sp3",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_precise_ephemeris_interpolant_from_sp3",
                "sp3"
            ));
            write_boxed_handle(
                out_handle,
                SidereonPreciseEphemerisInterpolant {
                    inner: PreciseEphemerisInterpolant::from_sp3(&sp3.inner),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Build a cached precise-ephemeris interpolant from canonical samples. On
/// success writes a newly owned handle to *out_handle; release it with
/// sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: samples must point to count entries or be NULL when count is 0;
/// out_handle must point to storage for a SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
        samples, count, 0.0, out_handle,
    )
}

/// Build a cached precise-ephemeris interpolant from canonical samples with an
/// explicit coverage-gap threshold factor. When gap_threshold_factor is <= 0.0,
/// the core default of 1.5 is used. On success writes a newly owned handle to
/// *out_handle; release it with sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: samples must point to count entries or be NULL when count is 0;
/// out_handle must point to storage for a SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
    samples: *const SidereonPreciseEphemerisSample,
    count: usize,
    gap_threshold_factor: f64,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    const FN_NAME: &str =
        "sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor";
    engine_error_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_handle = c_try!(require_out(out_handle, FN_NAME, "out_handle"));
        *out_handle = ptr::null_mut();
        let raw = c_try!(require_slice(samples, count, FN_NAME, "samples"));
        let options = c_try!(interpolation_options_from_c(FN_NAME, gap_threshold_factor));
        let mut parsed = Vec::with_capacity(raw.len());
        for sample in raw {
            parsed.push(c_try!(precise_sample_from_c(FN_NAME, sample)));
        }
        let inner = match PreciseEphemerisInterpolant::from_samples(parsed) {
            Ok(inner) => inner.with_interpolation_options(options),
            Err(err) => return map_precise_interpolant_error(FN_NAME, err),
        };
        write_boxed_handle(out_handle, SidereonPreciseEphemerisInterpolant { inner });
        SidereonStatus::Ok
    })
}

/// Write the SP3 interpolation gap threshold factor carried by this
/// interpolant to *out_gap_threshold_factor.
///
/// Safety: interpolant must be a live handle; out_gap_threshold_factor must
/// point to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    out_gap_threshold_factor: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_interpolant_gap_threshold_factor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_gap_threshold_factor = c_try!(require_out(
            out_gap_threshold_factor,
            FN_NAME,
            "out_gap_threshold_factor"
        ));
        *out_gap_threshold_factor = 0.0;
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
        *out_gap_threshold_factor = interpolant
            .inner
            .interpolation_options()
            .gap_threshold_factor();
        SidereonStatus::Ok
    })
}

/// Build a cached precise-ephemeris interpolant from an existing sample-backed
/// source handle. On success writes a newly owned handle to *out_handle; release
/// it with sidereon_precise_ephemeris_interpolant_free.
///
/// Safety: samples must be a live handle; out_handle must point to storage for a
/// SidereonPreciseEphemerisInterpolant*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples(
    samples: *const SidereonPreciseEphemerisSamples,
    out_handle: *mut *mut SidereonPreciseEphemerisInterpolant,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
        SidereonStatus::Panic,
        || {
            let out_handle = c_try!(require_out(
                out_handle,
                "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
                "out_handle"
            ));
            *out_handle = ptr::null_mut();
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_interpolant_from_precise_ephemeris_samples",
                "samples"
            ));
            write_boxed_handle(
                out_handle,
                SidereonPreciseEphemerisInterpolant {
                    inner: PreciseEphemerisInterpolant::from_precise_ephemeris_samples(
                        &samples.inner,
                    ),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a cached precise-ephemeris interpolant. Null is a no-op.
///
/// Safety: interpolant must be NULL or a live handle from an interpolant
/// creation function.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_free(
    interpolant: *mut SidereonPreciseEphemerisInterpolant,
) {
    ffi_boundary("sidereon_precise_ephemeris_interpolant_free", (), || {
        free_boxed(interpolant);
    });
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_state_at_epoch_query(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
    sat_id: *const c_char,
    query: *const SidereonExactEpochQuery,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_ephemeris_interpolant_state_at_epoch_query";
    crate::sp3::sp3_operation_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN_NAME, "out_state"));
        *out_state = crate::sp3::empty_sp3_state();
        let interpolant = c_try!(require_ref(interpolant, FN_NAME, "interpolant"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let query = c_try!(require_ref(query, FN_NAME, "query"));
        let state = c_try!(guard_core(
            || interpolant.inner.position_at_epoch_query(sat, &query.inner),
            |error| {
                crate::sp3::map_sp3_interpolation_error(FN_NAME, query.inner.j2000_seconds(), error)
            },
        ));
        *out_state = crate::sp3::sp3_state_to_c(state);
        SidereonStatus::Ok
    })
}

/// Evaluate many sample-backed precise-ephemeris observable states with
/// per-satellite epochs. The output arrays follow
/// sidereon_sp3_observable_states_at_j2000_s.
///
/// Safety: samples is a live handle; all array pointers follow the SP3 batch
/// state contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_observable_states_at_j2000_s(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
                "samples"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_precise_ephemeris_samples_observable_states_at_j2000_s",
                &samples.inner,
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

/// Evaluate many sample-backed precise-ephemeris observable states at one shared
/// epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s(
    samples: *const SidereonPreciseEphemerisSamples,
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
        "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let samples = c_try!(require_ref(
                samples,
                "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
                "samples"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_precise_ephemeris_samples_observable_states_at_shared_j2000_s",
                &samples.inner,
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

/// Evaluate many cached precise-interpolant observable states with
/// per-satellite epochs. The output arrays follow
/// sidereon_sp3_observable_states_at_j2000_s.
///
/// Safety: interpolant is a live handle; all array pointers follow the SP3 batch
/// state contract.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
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
        "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let interpolant = c_try!(require_ref(
                interpolant,
                "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
                "interpolant"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_precise_ephemeris_interpolant_observable_states_at_j2000_s",
                &interpolant.inner,
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

/// Evaluate many cached precise-interpolant observable states at one shared
/// epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s(
    interpolant: *const SidereonPreciseEphemerisInterpolant,
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
        "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let interpolant = c_try!(require_ref(
                interpolant,
                "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
                "interpolant"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_precise_ephemeris_interpolant_observable_states_at_shared_j2000_s",
                &interpolant.inner,
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

fn precise_samples_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

fn gnss_system_name(system: sidereon_core::GnssSystem) -> &'static str {
    use sidereon_core::GnssSystem as S;
    match system {
        S::Gps => "gps",
        S::Glonass => "glonass",
        S::Galileo => "galileo",
        S::BeiDou => "beidou",
        S::Qzss => "qzss",
        S::Navic => "navic",
        S::Sbas => "sbas",
    }
}

fn gnss_satellite_id_value(sat: &sidereon_core::GnssSatelliteId) -> serde_json::Value {
    serde_json::json!({
        "system": gnss_system_name(sat.system),
        "prn": sat.prn,
    })
}

pub(crate) fn precise_samples_error_value(error: &PreciseSamplesError) -> serde_json::Value {
    use PreciseSamplesError as E;
    match error {
        E::Empty => precise_samples_node("empty", serde_json::json!({})),
        E::SingleSampleSatellite(sat) => precise_samples_node(
            "single_sample_satellite",
            serde_json::json!({
                "satellite": gnss_satellite_id_value(sat),
            }),
        ),
        E::NonMonotonicEpochs(sat) => precise_samples_node(
            "non_monotonic_epochs",
            serde_json::json!({
                "satellite": gnss_satellite_id_value(sat),
            }),
        ),
        E::MixedTimeScales => precise_samples_node("mixed_time_scales", serde_json::json!({})),
        E::EpochNotRepresentable(sat) => precise_samples_node(
            "epoch_not_representable",
            serde_json::json!({
                "satellite": gnss_satellite_id_value(sat),
            }),
        ),
        E::NonFiniteSample(sat) => precise_samples_node(
            "non_finite_sample",
            serde_json::json!({
                "satellite": gnss_satellite_id_value(sat),
            }),
        ),
        E::AccuracySamplesMismatch => {
            precise_samples_node("accuracy_samples_mismatch", serde_json::json!({}))
        }
        E::InvalidAccuracyValue(sat) => precise_samples_node(
            "invalid_accuracy_value",
            serde_json::json!({
                "satellite": gnss_satellite_id_value(sat),
            }),
        ),
        _ => precise_samples_node("unknown", serde_json::json!({})),
    }
}

fn map_precise_samples_error_retaining(fn_name: &str, err: &PreciseSamplesError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_precise_samples_error(fn_name: &str, err: PreciseSamplesError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PreciseSamples,
        fn_name,
        precise_samples_error_value(&err),
    );
    map_precise_samples_error_retaining(fn_name, &err)
}

pub(crate) unsafe fn precise_sample_from_c(
    fn_name: &str,
    sample: &SidereonPreciseEphemerisSample,
) -> Result<PreciseEphemerisSample, SidereonStatus> {
    let sat = parse_satellite_token(fn_name, sample.sat.bytes.as_ptr())?;
    let scale = time_scale_from_c_code(fn_name, "sample.time_scale", sample.time_scale)?;
    let epoch = instant_from_j2000_seconds(fn_name, "sample", scale, sample.epoch_j2000_s)?;
    let clock_s = if sample.has_clock_s {
        Some(sample.clock_s)
    } else {
        None
    };
    Ok(PreciseEphemerisSample {
        sat,
        epoch,
        position_ecef_m: sample.position_ecef_m,
        clock_s,
        clock_event: sample.clock_event,
    })
}

fn precise_interpolant_node(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "fields": fields,
    })
}

pub(crate) fn precise_interpolant_error_value(
    error: &PreciseInterpolantError,
) -> serde_json::Value {
    use PreciseInterpolantError as E;
    match error {
        E::Samples(source) => precise_interpolant_node(
            "samples",
            serde_json::json!({
                "cause": precise_samples_error_value(source),
            }),
        ),
    }
}

fn map_precise_interpolant_error_retaining(
    fn_name: &str,
    err: &PreciseInterpolantError,
) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn map_precise_interpolant_error(fn_name: &str, err: PreciseInterpolantError) -> SidereonStatus {
    record_engine_error(
        SidereonEngineErrorFamily::PreciseInterpolant,
        fn_name,
        precise_interpolant_error_value(&err),
    );
    map_precise_interpolant_error_retaining(fn_name, &err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_error::{
        clear_engine_error, sidereon_last_engine_error_info, sidereon_last_engine_error_payload,
        SidereonEngineErrorFamily, SidereonEngineErrorInfo,
    };
    use serde_json::{json, Value};
    use sidereon_core::ephemeris::{PreciseInterpolantError, PreciseSamplesError};
    use sidereon_core::{GnssSatelliteId, GnssSystem};
    use std::ptr;

    #[test]
    fn table_driven_precise_samples_error_mapping() {
        let gps1 = GnssSatelliteId::new(GnssSystem::Gps, 1).unwrap();
        let gal5 = GnssSatelliteId::new(GnssSystem::Galileo, 5).unwrap();
        let glo7 = GnssSatelliteId::new(GnssSystem::Glonass, 7).unwrap();
        let bds14 = GnssSatelliteId::new(GnssSystem::BeiDou, 14).unwrap();
        let sbas20 = GnssSatelliteId::new(GnssSystem::Sbas, 20).unwrap();

        let cases: Vec<(PreciseSamplesError, &'static str, Option<Value>)> = vec![
            (PreciseSamplesError::Empty, "empty", None),
            (
                PreciseSamplesError::SingleSampleSatellite(gps1),
                "single_sample_satellite",
                Some(json!({"system": "gps", "prn": 1})),
            ),
            (
                PreciseSamplesError::NonMonotonicEpochs(gal5),
                "non_monotonic_epochs",
                Some(json!({"system": "galileo", "prn": 5})),
            ),
            (
                PreciseSamplesError::MixedTimeScales,
                "mixed_time_scales",
                None,
            ),
            (
                PreciseSamplesError::EpochNotRepresentable(glo7),
                "epoch_not_representable",
                Some(json!({"system": "glonass", "prn": 7})),
            ),
            (
                PreciseSamplesError::NonFiniteSample(bds14),
                "non_finite_sample",
                Some(json!({"system": "beidou", "prn": 14})),
            ),
            (
                PreciseSamplesError::AccuracySamplesMismatch,
                "accuracy_samples_mismatch",
                None,
            ),
            (
                PreciseSamplesError::InvalidAccuracyValue(sbas20),
                "invalid_accuracy_value",
                Some(json!({"system": "sbas", "prn": 20})),
            ),
        ];

        for (err, kind, expected_sat) in cases {
            let v = precise_samples_error_value(&err);
            assert_eq!(v["kind"], json!(kind));
            if let Some(expected_sat) = expected_sat {
                assert_eq!(v["fields"]["satellite"], expected_sat);
            }
        }
    }

    #[test]
    fn table_driven_precise_interpolant_error_mapping() {
        let gps1 = GnssSatelliteId::new(GnssSystem::Gps, 1).unwrap();
        let inner = PreciseSamplesError::SingleSampleSatellite(gps1);
        let err = PreciseInterpolantError::Samples(inner);

        let v = precise_interpolant_error_value(&err);
        assert_eq!(v["kind"], json!("samples"));
        assert_eq!(
            v["fields"]["cause"]["kind"],
            json!("single_sample_satellite")
        );
        assert_eq!(
            v["fields"]["cause"]["fields"]["satellite"],
            json!({"system": "gps", "prn": 1})
        );
    }

    fn make_valid_sample(epoch_s: f64) -> SidereonPreciseEphemerisSample {
        let sat = GnssSatelliteId::new(GnssSystem::Gps, 11).unwrap();
        SidereonPreciseEphemerisSample {
            sat: satellite_token(sat),
            time_scale: crate::SidereonTimeScale::Gpst as u32,
            epoch_j2000_s: epoch_s,
            position_ecef_m: [7_000_000.0, 0.0, 0.0],
            has_clock_s: false,
            clock_s: 0.0,
            clock_event: false,
        }
    }

    #[test]
    fn precise_samples_producer_real_refusal_valid_control_and_retention() {
        clear_engine_error();

        let samples = [make_valid_sample(0.0), make_valid_sample(600.0)];

        unsafe {
            // Real public refusal: single sample (minimum 2 needed)
            let mut out_handle: *mut SidereonPreciseEphemerisSamples = ptr::null_mut();
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(out_handle.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseSamples);
            assert!(info.payload_len > 0);
            let expected_len = info.payload_len;

            // Two-pass payload retrieval: Pass 1 query with null/0 buffer
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
                SidereonStatus::Ok
            );
            assert_eq!(written, 0);
            assert_eq!(required, expected_len);

            // Short buffer query returns InvalidArgument, 0 written, full required, and retains
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

            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "precise_samples");
            assert_eq!(
                payload["operation"],
                "sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor"
            );
            assert_eq!(payload["error"]["kind"], "single_sample_satellite");
            assert_eq!(payload["error"]["fields"]["satellite"]["system"], "gps");
            assert_eq!(payload["error"]["fields"]["satellite"]["prn"], 11);

            // Reader on live handle retains TLS error
            let mut live_handle: *mut SidereonPreciseEphemerisSamples = ptr::null_mut();
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    &mut live_handle,
                ),
                SidereonStatus::Ok
            );
            assert!(!live_handle.is_null());

            // Re-seed error via 1-sample refusal while holding live handle
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut gap_factor = 0.0;
            assert_eq!(
                sidereon_precise_ephemeris_samples_gap_threshold_factor(
                    live_handle,
                    &mut gap_factor
                ),
                SidereonStatus::Ok
            );
            assert!(gap_factor > 0.0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseSamples);
            assert_eq!(info.payload_len, expected_len);

            // Free retains TLS error
            sidereon_precise_ephemeris_samples_free(live_handle);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseSamples);
            assert_eq!(info.payload_len, expected_len);

            // Valid success control clears TLS error
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::Ok
            );
            assert!(!out_handle.is_null());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
            sidereon_precise_ephemeris_samples_free(out_handle);

            // Re-seed error for early argument reset check
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseSamples);

            // Early argument refusal (null pointer) clears slot before checks!
            assert_eq!(
                sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }

    #[test]
    fn precise_interpolant_producer_real_refusal_valid_control_and_retention() {
        clear_engine_error();

        let samples = [make_valid_sample(0.0), make_valid_sample(600.0)];

        unsafe {
            // Real public refusal: single sample in interpolant builder
            let mut out_handle: *mut SidereonPreciseEphemerisInterpolant = ptr::null_mut();
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(out_handle.is_null());

            let mut info = SidereonEngineErrorInfo {
                family: SidereonEngineErrorFamily::None,
                payload_len: 0,
            };
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseInterpolant);
            assert!(info.payload_len > 0);
            let expected_len = info.payload_len;

            // Two-pass payload retrieval: Pass 1 query with null/0 buffer
            let mut written = 999;
            let mut required = 0;
            assert_eq!(
                sidereon_last_engine_error_payload(ptr::null_mut(), 0, &mut written, &mut required),
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

            let payload: Value = serde_json::from_slice(&buf).expect("valid JSON");
            assert_eq!(payload["schema_version"], 1);
            assert_eq!(payload["family"], "precise_interpolant");
            assert_eq!(
                payload["operation"],
                "sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor"
            );
            assert_eq!(payload["error"]["kind"], "samples");
            assert_eq!(
                payload["error"]["fields"]["cause"]["kind"],
                "single_sample_satellite"
            );
            assert_eq!(
                payload["error"]["fields"]["cause"]["fields"]["satellite"]["system"],
                "gps"
            );
            assert_eq!(
                payload["error"]["fields"]["cause"]["fields"]["satellite"]["prn"],
                11
            );

            // Reader on live handle retains TLS error
            let mut live_handle: *mut SidereonPreciseEphemerisInterpolant = ptr::null_mut();
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    &mut live_handle,
                ),
                SidereonStatus::Ok
            );
            assert!(!live_handle.is_null());

            // Re-seed error via 1-sample refusal
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );

            let mut gap_factor = 0.0;
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_gap_threshold_factor(
                    live_handle,
                    &mut gap_factor
                ),
                SidereonStatus::Ok
            );
            assert!(gap_factor > 0.0);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseInterpolant);
            assert_eq!(info.payload_len, expected_len);

            // Free retains TLS error
            sidereon_precise_ephemeris_interpolant_free(live_handle);
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseInterpolant);
            assert_eq!(info.payload_len, expected_len);

            // Valid success control clears TLS error
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::Ok
            );
            assert!(!out_handle.is_null());
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
            sidereon_precise_ephemeris_interpolant_free(out_handle);

            // Re-seed error for early argument reset check
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    1,
                    0.0,
                    &mut out_handle,
                ),
                SidereonStatus::InvalidArgument
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::PreciseInterpolant);

            // Early argument refusal (null pointer) clears slot before checks!
            assert_eq!(
                sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor(
                    samples.as_ptr(),
                    2,
                    0.0,
                    ptr::null_mut(),
                ),
                SidereonStatus::NullPointer
            );
            assert_eq!(
                sidereon_last_engine_error_info(&mut info),
                SidereonStatus::Ok
            );
            assert_eq!(info.family, SidereonEngineErrorFamily::None);
            assert_eq!(info.payload_len, 0);
        }
    }
}
